//! Coordinator process: runs inside a workspace, polls the coordination DB,
//! handles mechanical operations directly, invokes Claude for judgment calls.
//!
//! Started by the supervisor via `clc coordinator-run`. Communicates with
//! workers and the supervisor entirely through the coordination DB.

use std::path::Path;
use std::thread;
use std::time::Duration;

use camino::Utf8Path;

use crate::config::CoordinatorScope;
use crate::coordination::Coordination;
use crate::error::Error;
use crate::git;

/// Run the coordinator loop. Blocks until all work is done or the process is killed.
pub fn run(
    project_dir: &Path,
    main_branch: &str,
    admin_branch: &str,
    scope: &CoordinatorScope,
    worker_perm_defaults: &[String],
    worker_perm_deny: &[String],
    poll_interval: Duration,
) -> Result<(), Error> {
    let git_state = git::detect(project_dir, main_branch, admin_branch)
        .ok_or_else(|| Error::NonBlocking("not inside a git repository".into()))?;

    if !git_state.is_main {
        return Err(Error::NonBlocking(format!(
            "coordinator must run from the main branch (currently on '{}')",
            git_state.branch
        )));
    }

    let coord = Coordination::open(project_dir)
        .map_err(|e| Error::NonBlocking(format!("coordination DB: {e}")))?;

    // Register this coordinator.
    let _ = coord.register_agent(&scope.id, Some("supervisor"));
    let _ = coord.set_status(&scope.id, clc_sdk::coordination::AgentStatus::Running);

    eprintln!("coordinator '{}' started (poll every {poll_interval:?})", scope.id);

    loop {
        match tick(project_dir, main_branch, admin_branch, scope, worker_perm_defaults, worker_perm_deny, &coord) {
            Ok(TickResult::Continue) => {}
            Ok(TickResult::AllDone) => {
                eprintln!("coordinator '{}': all work completed", scope.id);
                let _ = coord.set_status(&scope.id, clc_sdk::coordination::AgentStatus::Completed);
                return Ok(());
            }
            Err(e) => {
                eprintln!("coordinator '{}' tick error: {e}", scope.id);
            }
        }

        thread::sleep(poll_interval);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum TickResult {
    Continue,
    AllDone,
}

fn tick(
    project_dir: &Path,
    main_branch: &str,
    admin_branch: &str,
    scope: &CoordinatorScope,
    worker_perm_defaults: &[String],
    worker_perm_deny: &[String],
    coord: &Coordination,
) -> Result<TickResult, Error> {
    // 1. Dispatch pickable tiskets up to max_workers.
    let running = coord
        .list_agents(Some(&scope.id))
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, s)| *s == clc_sdk::coordination::AgentStatus::Running)
        .count();

    if running < scope.max_workers {
        let pickable = find_undispatched(project_dir, scope, coord)?;
        let slots = scope.max_workers - running;

        for id in pickable.iter().take(slots) {
            eprintln!("coordinator '{}': dispatching '{id}'", scope.id);
            match crate::dispatch::dispatch(
                project_dir,
                id,
                main_branch,
                admin_branch,
                &scope.model,
                worker_perm_defaults,
                worker_perm_deny,
                Some(&scope.id),
            ) {
                Ok(()) => {}
                Err(e) => eprintln!("coordinator '{}': dispatch failed for '{id}': {e}", scope.id),
            }
        }
    }

    // 2. Land completed workers.
    let agents = coord.list_agents(Some(&scope.id)).unwrap_or_default();
    for (id, status) in &agents {
        if *status == clc_sdk::coordination::AgentStatus::Completed {
            eprintln!("coordinator '{}': landing '{id}'", scope.id);
            match crate::merge::merge(project_dir, id, main_branch, admin_branch) {
                Ok(()) => eprintln!("coordinator '{}': landed '{id}'", scope.id),
                Err(e) => {
                    // TODO: merge conflict → invoke Claude for resolution
                    eprintln!("coordinator '{}': land failed for '{id}': {e}", scope.id);
                }
            }
        }
    }

    // 3. Resume stopped workers (unless they have a pending permission request).
    for (id, status) in &agents {
        if *status == clc_sdk::coordination::AgentStatus::Stopped
            || *status == clc_sdk::coordination::AgentStatus::Failed
        {
            if crate::permissions::pending_request(project_dir, id).is_some() {
                continue;
            }
            eprintln!("coordinator '{}': resuming '{id}'", scope.id);
            match crate::worker::resume(project_dir, id) {
                Ok(()) => {}
                Err(e) => eprintln!("coordinator '{}': resume failed for '{id}': {e}", scope.id),
            }
        }
    }

    // 4. Handle permission requests.
    let pending = coord.pending_permissions(&scope.id).unwrap_or_default();
    for msg in &pending {
        if let clc_sdk::coordination::MessageKind::PermissionRequest {
            ref tool_name,
            ref reason,
        } = msg.kind
        {
            let worker_id = &msg.from;

            // Mechanical: auto-grant if pattern matches.
            if scope.auto_grant.iter().any(|p| tool_name.contains(p)) {
                eprintln!("coordinator '{}': auto-granting '{tool_name}' for '{worker_id}'", scope.id);
                let _ = crate::permissions::grant(project_dir, worker_id, tool_name);
                continue;
            }

            // Mechanical: escalate if pattern matches.
            if scope.always_escalate.iter().any(|p| tool_name.contains(p)) {
                eprintln!("coordinator '{}': escalating '{tool_name}' for '{worker_id}'", scope.id);
                let _ = crate::permissions::escalate(project_dir, worker_id, reason);
                continue;
            }

            // Judgment call: neither pattern matches → escalate to admin for now.
            // TODO: invoke coordinator's Claude session for judgment
            eprintln!(
                "coordinator '{}': escalating unknown permission '{tool_name}' for '{worker_id}'",
                scope.id
            );
            let _ = crate::permissions::escalate(project_dir, worker_id, reason);
        }
    }

    // 5. Check if all work is done.
    let all_agents = coord.list_agents(Some(&scope.id)).unwrap_or_default();
    let any_active = all_agents.iter().any(|(_, s)| {
        *s == clc_sdk::coordination::AgentStatus::Running
            || *s == clc_sdk::coordination::AgentStatus::Pending
            || *s == clc_sdk::coordination::AgentStatus::Stopped
            || *s == clc_sdk::coordination::AgentStatus::Failed
    });

    let pickable = find_undispatched(project_dir, scope, coord)?;

    if !any_active && pickable.is_empty() {
        return Ok(TickResult::AllDone);
    }

    Ok(TickResult::Continue)
}

fn find_undispatched(
    project_dir: &Path,
    scope: &CoordinatorScope,
    coord: &Coordination,
) -> Result<Vec<String>, Error> {
    let utf8_dir = Utf8Path::new(
        project_dir
            .to_str()
            .ok_or_else(|| Error::NonBlocking("non-UTF8 project directory".into()))?,
    );

    let repo =
        tisket::Repo::open(utf8_dir).map_err(|e| Error::NonBlocking(format!("tisket: {e}")))?;

    let issues = repo
        .list_issues(scope.project.as_deref(), None, None, false, &[])
        .map_err(|e| Error::NonBlocking(format!("tisket: {e}")))?;

    let dispatched: Vec<String> = coord
        .list_agents(Some(&scope.id))
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
            scope
                .label
                .as_deref()
                .is_none_or(|l| i.frontmatter.labels.iter().any(|il| il == l))
        })
        .filter(|i| {
            scope
                .exclude_label
                .as_deref()
                .is_none_or(|l| !i.frontmatter.labels.iter().any(|il| il == l))
        })
        .map(|i| i.id)
        .filter(|id| !dispatched.contains(id))
        .collect();

    Ok(pickable)
}
