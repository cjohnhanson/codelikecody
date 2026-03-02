//! Coordinator: spawn a coordinator agent that dispatches and monitors workers.
//!
//! The coordinator runs on trunk as a background claude --print process with
//! the same pipe infrastructure as workers. The user talks to it via
//! `clc worker coordinator send/check`.

use std::path::Path;

use camino::Utf8Path;

use crate::dispatch;
use crate::error::Error;
use crate::git;
use crate::permissions;
use crate::worker;

pub fn coordinate(
    project_dir: &Path,
    main_branch: &str,
    model: &str,
    tisket_filter: Option<&str>,
    extra_allow: &[String],
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

    // Check if coordinator is already running.
    let coord_worker_dir = worker::worker_dir_for(project_dir, worker::COORDINATOR_ID);
    if coord_worker_dir.is_dir()
        && let Some(pid) = read_pid(&coord_worker_dir)
        && is_pid_alive(pid)
    {
        return Err(Error::NonBlocking(format!(
            "coordinator already running (pid {pid}) — use `clc worker coordinator send` to talk to it"
        )));
    }

    // Find pickable tiskets.
    let pickable = find_pickable_tiskets(project_dir, tisket_filter)?;

    if pickable.is_empty() {
        eprintln!("no pickable tiskets found");
        return Ok(());
    }

    // Seed baseline permissions so coordinator can function without
    // --dangerously-skip-permissions.
    permissions::seed_baseline(project_dir, extra_allow)?;

    // Build the initial prompt with pickable tiskets.
    let initial_prompt = build_coordinator_prompt(&pickable, tisket_filter);
    let system_prompt = build_coordinator_system_prompt();

    // Spawn coordinator as a worker on trunk.
    let pid = dispatch::spawn_worker_process(
        project_dir,
        &coord_worker_dir,
        model,
        &system_prompt,
        &initial_prompt,
        &[],
    )?;

    eprintln!("coordinator launched (pid {pid})");
    eprintln!("talk to it: clc worker coordinator send \"<message>\"");
    eprintln!("check output: clc worker coordinator check");

    Ok(())
}

fn find_pickable_tiskets(
    project_dir: &Path,
    tisket_filter: Option<&str>,
) -> Result<Vec<String>, Error> {
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
            i.frontmatter.depends_on.iter().all(|dep_id| {
                repo.find_issue(dep_id)
                    .map(|dep| dep.closed)
                    .unwrap_or(false)
            })
        })
        .map(|i| i.id)
        .collect();

    pickable.sort();

    if let Some(filter) = tisket_filter {
        if !pickable.iter().any(|t| t == filter) {
            return Err(Error::NonBlocking(format!(
                "tisket '{filter}' is not pickable (not found or dependencies unresolved)"
            )));
        }
        return Ok(vec![filter.to_string()]);
    }

    Ok(pickable)
}

fn build_coordinator_prompt(pickable: &[String], tisket_filter: Option<&str>) -> String {
    let tisket_list = pickable
        .iter()
        .map(|id| format!("- {id}"))
        .collect::<Vec<_>>()
        .join("\n");

    let scope = if tisket_filter.is_some() {
        "You have been asked to work on a specific tisket."
    } else {
        "Dispatch and monitor these tiskets. Work through them sequentially or in parallel as appropriate."
    };

    format!(
        "{scope}\n\n\
         Pickable tiskets:\n{tisket_list}\n\n\
         Read each tisket before dispatching to understand the task:\n\
         \x20 tisket issue show <id>\n\n\
         Dispatch a worker for each tisket:\n\
         \x20 clc dispatch <id>\n\n\
         Monitor worker progress:\n\
         \x20 clc workers              # list all workers and their status\n\
         \x20 clc worker <id> check    # see recent output from a worker\n\
         \x20 clc worker <id> send \"<message>\"  # send a message to a worker\n\n\
         Land completed work (worker must be in `done` phase):\n\
         \x20 clc land <id>\n\n\
         When all dispatched workers have completed and been landed, you are done."
    )
}

fn build_coordinator_system_prompt() -> String {
    // NOTE: This is prompt content. Approved in this commit.
    String::from(
        "You are the coordinator agent. You do not write code — you manage worker agents \
         that do. You run on the main branch. Workers run in worktrees.\n\n\
         Two CLI tools are available: `clc` (workflow engine) and `tisket` (issue tracker). \
         Both support `--help` on every subcommand — use it to discover usage and flags.\n\n\
         Your lifecycle:\n\
         1. Read each tisket to understand the task: `tisket issue show <id>`\n\
         2. Dispatch a worker: `clc dispatch <id>`\n\
         3. Monitor progress: `clc workers` (overview), `clc worker <id> check` (detail)\n\
         4. Intervene if needed: `clc worker <id> send \"<message>\"`\n\
         5. Land completed work: `clc land <id>` (worker must be in `done` phase)\n\
         6. Repeat until all tiskets are landed.\n\n\
         Workers have limited permissions. When a worker needs a tool that isn't \
         pre-approved, it files a request with `clc permissions request` and stops. \
         Check for pending requests with `clc permissions list` or `clc worker <id> check`. \
         Grant with `clc permissions grant <id> \"<permission rule>\"`, then resume the worker. \
         If the request seems inappropriate or dangerous, escalate to the user with \
         `clc permissions escalate <id> \"<description>\"` instead of granting directly. \
         The user can review pending escalations with `clc permissions inbox`.\n\n\
         Missouri is the project's state-graph test framework. Tests live in `tests/missouri/` \
         as a directed graph of states. Each state is a directory with fixture files and a \
         `.missouri/missouri.yml` that defines assertions (invariants that must hold in that \
         state) and transitions (commands that move to the next state, with file comparators). \
         States chain together to form end-to-end paths through the system. Run \
         `clc missouri run` to execute all paths.\n\n\
         Before landing a worker, consider whether the work should have Missouri test coverage. \
         Not everything needs it — but changes to CLI commands, workflow behavior, file formats, \
         or state transitions are good candidates. If the worker's diff touches areas that \
         existing Missouri tests cover, check that the tests still pass. If the change adds \
         new behavior that fits the state graph, ask the worker to add a new state or extend \
         an existing path before landing.\n\n\
         The user communicates with you via messages on stdin. When you receive a user \
         message, respond to it and act on their instructions. Between user messages, \
         continue monitoring active workers and landing completed ones.",
    )
}

fn read_pid(worker_dir: &Path) -> Option<i32> {
    std::fs::read_to_string(worker_dir.join("pid"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn is_pid_alive(pid: i32) -> bool {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None).is_ok()
}
