//! Coordinator loop: poll-based coordinator that drives the worker lifecycle.
//!
//! Replaces the Claude-process-as-coordinator model with a deterministic loop
//! that queries the coordination DB, makes decisions, and only invokes the LLM
//! when judgment is needed.
//!
//! The loop:
//! 1. Find pickable tiskets → dispatch workers
//! 2. Check for completed workers → land them
//! 3. Check for failed/stopped workers → resume or flag
//! 4. Check for pending permission requests → auto-grant or escalate
//! 5. Sleep → repeat

use std::path::Path;
use std::thread;
use std::time::Duration;

use camino::Utf8Path;

use crate::config::CoordinatorConfig;
use crate::coordination::Coordination;
use crate::coordinate::{CoordinateFilters, PermissionPolicy};
use crate::error::Error;
use crate::git;

/// Configuration for the coordinator loop.
pub struct LoopConfig<'a> {
    pub project_dir: &'a Path,
    pub main_branch: &'a str,
    pub admin_branch: &'a str,
    pub model: &'a str,
    pub filters: &'a CoordinateFilters<'a>,
    pub worker_perm_defaults: &'a [String],
    pub worker_perm_deny: &'a [String],
    pub coordinator_config: &'a CoordinatorConfig,
    pub poll_interval: Duration,
    pub max_workers: usize,
}

/// Run the coordinator loop. Blocks until interrupted or all work is done.
pub fn run(cfg: &LoopConfig<'_>) -> Result<(), Error> {
    // Must be on trunk.
    let git_state = git::detect(cfg.project_dir, cfg.main_branch, cfg.admin_branch)
        .ok_or_else(|| Error::NonBlocking("not inside a git repository".into()))?;

    if !git_state.is_main {
        return Err(Error::NonBlocking(format!(
            "coordinator must run from the main branch (currently on '{}')",
            git_state.branch
        )));
    }

    // Resolve permission policy.
    let policy = crate::coordinate::resolve_policy_pub(cfg.coordinator_config, cfg.filters)?;

    // Open coordination DB.
    let coord = Coordination::open(cfg.project_dir)
        .map_err(|e| Error::NonBlocking(format!("coordination DB: {e}")))?;

    // Register the coordinator agent.
    let _ = coord.register_agent("coordinator", None);
    let _ = coord.set_status("coordinator", clc_sdk::coordination::AgentStatus::Running);

    eprintln!("coordinator loop started (poll every {:?})", cfg.poll_interval);

    loop {
        let action = tick(cfg, &coord, &policy)?;

        if action == TickAction::AllDone {
            eprintln!("all work completed");
            let _ = coord.set_status(
                "coordinator",
                clc_sdk::coordination::AgentStatus::Completed,
            );
            return Ok(());
        }

        thread::sleep(cfg.poll_interval);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum TickAction {
    Continue,
    AllDone,
}

/// One iteration of the coordinator loop.
fn tick(
    cfg: &LoopConfig<'_>,
    coord: &Coordination,
    policy: &PermissionPolicy,
) -> Result<TickAction, Error> {
    // 1. Find pickable tiskets and dispatch workers (up to max_workers).
    let running = coord
        .list_agents(None)
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, s)| *s == clc_sdk::coordination::AgentStatus::Running)
        .count();

    if running < cfg.max_workers {
        let pickable = find_undispatched_tiskets(cfg, coord)?;
        let slots = cfg.max_workers - running;

        for id in pickable.iter().take(slots) {
            eprintln!("dispatching worker for '{id}'");
            match crate::dispatch::dispatch(
                cfg.project_dir,
                id,
                cfg.main_branch,
                cfg.admin_branch,
                cfg.model,
                cfg.worker_perm_defaults,
                cfg.worker_perm_deny,
                Some("coordinator"),
            ) {
                Ok(()) => {}
                Err(e) => eprintln!("dispatch failed for '{id}': {e}"),
            }
        }
    }

    // 2. Check for completed workers → land them.
    let agents = coord.list_agents(Some("coordinator")).unwrap_or_default();
    for (id, status) in &agents {
        if *status == clc_sdk::coordination::AgentStatus::Completed {
            eprintln!("landing completed worker '{id}'");
            match crate::merge::merge(cfg.project_dir, id, cfg.main_branch, cfg.admin_branch) {
                Ok(()) => {
                    eprintln!("landed '{id}'");
                }
                Err(e) => eprintln!("land failed for '{id}': {e}"),
            }
        }
    }

    // 3. Check for stopped/failed workers → resume.
    for (id, status) in &agents {
        if *status == clc_sdk::coordination::AgentStatus::Stopped
            || *status == clc_sdk::coordination::AgentStatus::Failed
        {
            // Don't resume if there's a pending permission request.
            if crate::permissions::pending_request(cfg.project_dir, id).is_some() {
                continue;
            }

            eprintln!("resuming stopped worker '{id}'");
            match crate::worker::resume(cfg.project_dir, id) {
                Ok(()) => {}
                Err(e) => eprintln!("resume failed for '{id}': {e}"),
            }
        }
    }

    // 4. Handle pending permission requests.
    handle_permissions(cfg, coord, policy)?;

    // 5. Check if all work is done.
    let all_agents = coord.list_agents(Some("coordinator")).unwrap_or_default();
    let any_active = all_agents
        .iter()
        .any(|(_, s)| {
            *s == clc_sdk::coordination::AgentStatus::Running
                || *s == clc_sdk::coordination::AgentStatus::Pending
                || *s == clc_sdk::coordination::AgentStatus::Stopped
                || *s == clc_sdk::coordination::AgentStatus::Failed
        });

    let pickable = find_undispatched_tiskets(cfg, coord)?;

    if !any_active && pickable.is_empty() {
        return Ok(TickAction::AllDone);
    }

    Ok(TickAction::Continue)
}

/// Find tiskets that are pickable but haven't been dispatched yet.
fn find_undispatched_tiskets(
    cfg: &LoopConfig<'_>,
    coord: &Coordination,
) -> Result<Vec<String>, Error> {
    let utf8_dir = Utf8Path::new(
        cfg.project_dir
            .to_str()
            .ok_or_else(|| Error::NonBlocking("non-UTF8 project directory".into()))?,
    );

    let repo =
        tisket::Repo::open(utf8_dir).map_err(|e| Error::NonBlocking(format!("tisket: {e}")))?;

    let issues = repo
        .list_issues(cfg.filters.project, None, None, false, &[])
        .map_err(|e| Error::NonBlocking(format!("tisket: {e}")))?;

    // Get already-dispatched agent IDs.
    let dispatched: Vec<String> = coord
        .list_agents(Some("coordinator"))
        .unwrap_or_default()
        .into_iter()
        .map(|(id, _)| id)
        .collect();

    let pickable: Vec<String> = issues
        .into_iter()
        .filter(|i| i.frontmatter.status.is_pickable())
        .filter(|i| {
            i.frontmatter.depends_on.iter().all(|dep_id| {
                repo.find_issue(dep_id)
                    .map(|dep| dep.closed)
                    .unwrap_or(false)
            })
        })
        .filter(|i| {
            cfg.filters
                .label
                .is_none_or(|l| i.frontmatter.labels.iter().any(|il| il == l))
        })
        .filter(|i| {
            cfg.filters
                .exclude_label
                .is_none_or(|l| !i.frontmatter.labels.iter().any(|il| il == l))
        })
        .map(|i| i.id)
        .filter(|id| !dispatched.contains(id))
        .collect();

    Ok(pickable)
}

/// Handle pending permission requests according to the policy.
fn handle_permissions(
    cfg: &LoopConfig<'_>,
    coord: &Coordination,
    policy: &PermissionPolicy,
) -> Result<(), Error> {
    let pending = coord
        .pending_permissions("coordinator")
        .unwrap_or_default();

    for msg in &pending {
        if let clc_sdk::coordination::MessageKind::PermissionRequest {
            ref tool_name,
            ref reason,
        } = msg.kind
        {
            let worker_id = &msg.from;

            // Check auto-grant patterns.
            if !policy.escalate_all
                && policy
                    .auto_grant
                    .iter()
                    .any(|pattern| tool_name.contains(pattern))
            {
                eprintln!("auto-granting '{tool_name}' for worker '{worker_id}'");
                match crate::permissions::grant(cfg.project_dir, worker_id, tool_name) {
                    Ok(()) => {}
                    Err(e) => eprintln!("grant failed: {e}"),
                }
                continue;
            }

            // Check always-escalate patterns or escalate-all.
            if policy.escalate_all
                || policy
                    .always_escalate
                    .iter()
                    .any(|pattern| tool_name.contains(pattern))
            {
                eprintln!("escalating '{tool_name}' for worker '{worker_id}': {reason}");
                match crate::permissions::escalate(cfg.project_dir, worker_id, reason) {
                    Ok(()) => {}
                    Err(e) => eprintln!("escalation failed: {e}"),
                }
                continue;
            }

            // Default: escalate unknown permissions.
            eprintln!("escalating unknown permission '{tool_name}' for worker '{worker_id}'");
            match crate::permissions::escalate(cfg.project_dir, worker_id, reason) {
                Ok(()) => {}
                Err(e) => eprintln!("escalation failed: {e}"),
            }
        }
    }

    Ok(())
}
