//! Coordinator: dispatch pickable tiskets to autonomous worker agents.
//!
//! The coordinator runs on trunk (read-only). For each pickable tisket it:
//! 1. Runs `pickup` (creates worktree, sets status, inits clc hooks)
//! 2. Launches a Claude Code worker in stream-json mode
//! 3. Monitors the worker until completion or failure
//! 4. Merges completed work back to trunk via `merge`

use std::path::Path;

use camino::Utf8Path;

use clc_sdk::stream::OutputMessage;
use clc_sdk::workspace::{Workspace, WorkspaceConfig, WorkspaceStatus};

use crate::error::Error;
use crate::workspace::WorktreeWorkspace;
use crate::{git, merge, pickup};

enum WorkerOutcome {
    Completed,
    Failed(String),
}

pub fn coordinate(
    project_dir: &Path,
    main_branch: &str,
    budget: f64,
    model: &str,
    tisket_filter: Option<&str>,
) -> Result<(), Error> {
    // Must be on trunk.
    let git_state = git::detect(project_dir, main_branch)
        .ok_or_else(|| Error::NonBlocking("not inside a git repository".into()))?;

    if !git_state.is_main {
        return Err(Error::NonBlocking(format!(
            "coordinator must run from the main branch (currently on '{}')",
            git_state.branch
        )));
    }

    // Find pickable tiskets, optionally filtered to a single one.
    let pickable = if let Some(id) = tisket_filter {
        // Verify the requested tisket exists and is pickable.
        let all = find_pickable_tiskets(project_dir)?;
        if !all.iter().any(|t| t == id) {
            return Err(Error::NonBlocking(format!(
                "tisket '{id}' is not pickable (not found or dependencies unresolved)"
            )));
        }
        vec![id.to_string()]
    } else {
        find_pickable_tiskets(project_dir)?
    };

    if pickable.is_empty() {
        eprintln!("no pickable tiskets found");
        return Ok(());
    }

    eprintln!("found {} pickable tisket(s)", pickable.len());

    // Process each tisket sequentially.
    for tisket_id in &pickable {
        eprintln!("--- picking up: {tisket_id} ---");

        // Pickup: creates worktree, sets status, inits clc hooks.
        pickup::pickup(project_dir, tisket_id, main_branch)?;

        // Build prompts from the tisket body.
        let initial_prompt = build_worker_prompt(project_dir, tisket_id)?;
        let system_prompt = build_system_prompt(tisket_id);

        // Create and start workspace.
        let config = WorkspaceConfig {
            tisket_id: tisket_id.clone(),
            project_dir: project_dir.to_path_buf(),
            main_branch: main_branch.to_string(),
            initial_prompt,
            system_prompt: Some(system_prompt),
            max_budget_usd: Some(budget),
            model: Some(model.to_string()),
        };

        let mut ws = WorktreeWorkspace::new(config);
        ws.start()
            .map_err(|e| Error::NonBlocking(format!("failed to start workspace: {e}")))?;

        eprintln!("--- {tisket_id}: worker launched ---");

        // Monitor until completion or failure.
        let outcome = monitor_worker(&mut ws)?;

        // Always stop the workspace process.
        let _ = ws.stop();

        match outcome {
            WorkerOutcome::Completed => {
                eprintln!("--- {tisket_id}: completed, merging ---");

                // Check for permission denials.
                let denials = ws.permission_denials();
                if !denials.is_empty() {
                    eprintln!("--- {tisket_id}: permission denials: ---");
                    for d in denials {
                        eprintln!("  {}: {}", d.tool_name, d.message);
                    }
                }

                match merge::merge(project_dir, tisket_id, main_branch) {
                    Ok(()) => eprintln!("--- {tisket_id}: merged ---"),
                    Err(e) => {
                        eprintln!("--- {tisket_id}: merge failed: {e} ---");
                        // Leave worktree intact for inspection.
                    }
                }
            }
            WorkerOutcome::Failed(reason) => {
                eprintln!("--- {tisket_id}: failed: {reason} ---");
                // Leave worktree intact for debugging.
            }
        }
    }

    Ok(())
}

fn monitor_worker(ws: &mut WorktreeWorkspace) -> Result<WorkerOutcome, Error> {
    loop {
        let messages = ws
            .recv_output()
            .map_err(|e| Error::NonBlocking(format!("recv_output: {e}")))?;

        for msg in &messages {
            match msg {
                OutputMessage::Result(result) => {
                    eprintln!(
                        "  result: cost=${:.4}, duration={:.1}s, error={}",
                        result.cost_usd, result.duration_secs, result.is_error
                    );
                }
                OutputMessage::Assistant(assistant) => {
                    // Log tool uses for observability.
                    for block in &assistant.message.content {
                        if let clc_sdk::stream::ContentBlock::ToolUse { name, .. } = block {
                            eprintln!("  tool: {name}");
                        }
                    }
                }
                _ => {}
            }
        }

        match ws.status() {
            WorkspaceStatus::Completed => return Ok(WorkerOutcome::Completed),
            WorkspaceStatus::Failed => {
                return Ok(WorkerOutcome::Failed(
                    "worker process exited with error".into(),
                ));
            }
            WorkspaceStatus::Running => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            WorkspaceStatus::NotStarted => unreachable!(),
        }
    }
}

fn find_pickable_tiskets(project_dir: &Path) -> Result<Vec<String>, Error> {
    let utf8_dir = Utf8Path::new(
        project_dir
            .to_str()
            .ok_or_else(|| Error::NonBlocking("non-UTF8 project directory".into()))?,
    );

    let repo =
        tisket::Repo::open(utf8_dir).map_err(|e| Error::NonBlocking(format!("tisket: {e}")))?;

    let issues = repo
        .list_issues(None, None, false)
        .map_err(|e| Error::NonBlocking(format!("tisket: {e}")))?;

    let mut pickable: Vec<String> = issues
        .into_iter()
        .filter(|i| i.frontmatter.status.is_pickable())
        .filter(|i| {
            // Dependencies must all be closed.
            i.frontmatter.depends_on.iter().all(|dep_id| {
                repo.find_issue(dep_id)
                    .map(|dep| dep.closed)
                    .unwrap_or(false)
            })
        })
        .map(|i| i.id)
        .collect();

    pickable.sort();
    Ok(pickable)
}

fn build_worker_prompt(project_dir: &Path, tisket_id: &str) -> Result<String, Error> {
    let utf8_dir = Utf8Path::new(
        project_dir
            .to_str()
            .ok_or_else(|| Error::NonBlocking("non-UTF8 project directory".into()))?,
    );

    let repo =
        tisket::Repo::open(utf8_dir).map_err(|e| Error::NonBlocking(format!("tisket: {e}")))?;

    let issue = repo
        .find_issue(tisket_id)
        .map_err(|e| Error::NonBlocking(format!("tisket '{tisket_id}': {e}")))?;

    Ok(format!(
        "You are working on tisket '{tisket_id}': {}\n\n\
         {}\n\n\
         Follow the clc workflow: write tests, implement, get green, run `clc done`.\n\
         The hooks will guide you through each phase.",
        issue.frontmatter.title, issue.body,
    ))
}

fn build_system_prompt(tisket_id: &str) -> String {
    format!(
        "You are an autonomous worker agent managed by clc. \
         Your task is defined by tisket '{tisket_id}'. \
         Follow the phase system: tests-unwritten -> tests-written -> red -> implementing -> green -> done. \
         When all tests pass and work is complete, run `clc done` to finalize. \
         Do not stop before reaching the 'done' phase."
    )
}
