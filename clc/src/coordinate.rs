//! Coordinator: spawn a coordinator agent that dispatches and monitors workers.
//!
//! The coordinator runs on trunk as a background claude --print process with
//! the same pipe infrastructure as workers. The user talks to it via
//! `clc worker coordinator send/check`.

use std::path::Path;

use camino::Utf8Path;
use serde::Deserialize;

use crate::config::CoordinatorConfig;
use crate::dispatch;
use crate::error::Error;
use crate::git;
use crate::permissions;
use crate::worker;

/// Filter options for coordinator tisket selection.
pub struct CoordinateFilters<'a> {
    pub tisket: Option<&'a str>,
    pub label: Option<&'a str>,
    pub exclude_label: Option<&'a str>,
    pub project: Option<&'a str>,
    pub depends_on: Option<&'a str>,
    pub dry_run: bool,
    pub coordinator_id: Option<&'a str>,
    pub auto_grant: &'a [String],
    pub escalate_all: bool,
    pub grant_config: Option<&'a str>,
}

/// Resolved permission policy for the coordinator.
#[derive(Debug, Default)]
pub struct PermissionPolicy {
    pub auto_grant: Vec<String>,
    pub always_escalate: Vec<String>,
    pub escalate_all: bool,
}

/// External grant-config file format.
#[derive(Debug, Deserialize)]
struct GrantConfigFile {
    #[serde(default)]
    auto_grant: Vec<String>,
    #[serde(default)]
    always_escalate: Vec<String>,
}

pub fn coordinate(
    project_dir: &Path,
    main_branch: &str,
    model: &str,
    filters: &CoordinateFilters<'_>,
    extra_allow: &[String],
    coordinator_config: &CoordinatorConfig,
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

    // Resolve the permission policy early so errors surface before side effects.
    let policy = resolve_policy(coordinator_config, filters)?;

    // Find pickable tiskets.
    let pickable = find_pickable_tiskets(
        project_dir,
        filters.tisket,
        filters.label,
        filters.exclude_label,
        filters.project,
        filters.depends_on,
    )?;

    if pickable.is_empty() {
        eprintln!("no pickable tiskets found");
        return Ok(());
    }

    // Dry-run: print pickable list and exit.
    if filters.dry_run {
        for id in &pickable {
            println!("{id}");
        }
        return Ok(());
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

    // Seed baseline permissions so coordinator can function without
    // --dangerously-skip-permissions.
    permissions::seed_baseline(project_dir, extra_allow)?;

    // Build the initial prompt with pickable tiskets.
    let initial_prompt =
        build_coordinator_prompt(&pickable, filters.tisket, filters.coordinator_id);
    let system_prompt = build_coordinator_system_prompt(&policy);

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

/// Resolve the permission policy from config, CLI flags, and optional external file.
///
/// Merging order: config → grant-config file → CLI flags. Duplicates are preserved
/// (the coordinator prompt consumes them as pattern lists, not sets).
fn resolve_policy(
    config: &CoordinatorConfig,
    filters: &CoordinateFilters<'_>,
) -> Result<PermissionPolicy, Error> {
    let mut policy = PermissionPolicy {
        auto_grant: config.auto_grant.clone(),
        always_escalate: config.always_escalate.clone(),
        escalate_all: filters.escalate_all,
    };

    // Merge from external grant-config file if provided.
    if let Some(path) = filters.grant_config {
        let grant_config = load_grant_config(path)?;
        policy.auto_grant.extend(grant_config.auto_grant);
        policy.always_escalate.extend(grant_config.always_escalate);
    }

    // Merge CLI --auto-grant flags.
    policy.auto_grant.extend(filters.auto_grant.iter().cloned());

    Ok(policy)
}

/// Load and parse an external grant-config YAML file.
fn load_grant_config(path: &str) -> Result<GrantConfigFile, Error> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| Error::NonBlocking(format!("failed to read grant-config '{path}': {e}")))?;

    serde_yml::from_str(&contents)
        .map_err(|e| Error::NonBlocking(format!("invalid grant-config '{path}': {e}")))
}

fn find_pickable_tiskets(
    project_dir: &Path,
    tisket_filter: Option<&str>,
    label: Option<&str>,
    exclude_label: Option<&str>,
    project: Option<&str>,
    depends_on: Option<&str>,
) -> Result<Vec<String>, Error> {
    let utf8_dir = Utf8Path::new(
        project_dir
            .to_str()
            .ok_or_else(|| Error::NonBlocking("non-UTF8 project directory".into()))?,
    );

    let repo =
        tisket::Repo::open(utf8_dir).map_err(|e| Error::NonBlocking(format!("tisket: {e}")))?;

    // Build the dependency chain scope if --depends-on is specified.
    let chain_scope = if let Some(root_id) = depends_on {
        // Verify the root tisket exists.
        repo.find_issue(root_id)
            .map_err(|_| Error::NonBlocking(format!("tisket '{root_id}' not found")))?;
        Some(build_dependency_chain(&repo, root_id)?)
    } else {
        None
    };

    let issues = repo
        .list_issues(project, None, None, false)
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
        // Label filter: only tiskets with this label.
        .filter(|i| label.is_none_or(|l| i.frontmatter.labels.iter().any(|il| il == l)))
        // Exclude-label filter: skip tiskets with this label.
        .filter(|i| exclude_label.is_none_or(|l| !i.frontmatter.labels.iter().any(|il| il == l)))
        // Depends-on filter: only tiskets in the dependency chain scope.
        .filter(|i| {
            chain_scope
                .as_ref()
                .is_none_or(|scope| scope.contains(&i.id))
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

/// Build the set of tisket IDs in the dependency chain rooted at `root_id`.
///
/// The chain includes the root itself plus all tiskets that transitively
/// depend on it (i.e., have it in their `depends_on`, directly or indirectly).
fn build_dependency_chain(repo: &tisket::Repo, root_id: &str) -> Result<Vec<String>, Error> {
    // Collect all issues across all projects to scan for dependents.
    let all_issues = repo
        .list_issues(None, None, None, false)
        .map_err(|e| Error::NonBlocking(format!("tisket: {e}")))?;

    // Also include closed issues to find the full chain.
    let closed_issues = repo
        .list_issues(None, None, None, true)
        .map_err(|e| Error::NonBlocking(format!("tisket: {e}")))?;

    // Build a map of id → depends_on for all issues.
    let mut deps_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for issue in all_issues.iter().chain(closed_issues.iter()) {
        deps_map
            .entry(issue.id.clone())
            .or_default()
            .clone_from(&issue.frontmatter.depends_on);
    }

    // Walk: start with root, find all IDs that transitively depend on it.
    let mut chain = vec![root_id.to_string()];
    let mut frontier = vec![root_id.to_string()];

    while let Some(current) = frontier.pop() {
        for (id, dep_list) in &deps_map {
            if dep_list.iter().any(|d| d == &current) && !chain.contains(id) {
                chain.push(id.clone());
                frontier.push(id.clone());
            }
        }
    }

    Ok(chain)
}

fn build_coordinator_prompt(
    pickable: &[String],
    tisket_filter: Option<&str>,
    coordinator_id: Option<&str>,
) -> String {
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

    let dispatch_cmd = coordinator_id.map_or_else(
        || "clc dispatch <id>".to_string(),
        |cid| format!("clc dispatch <id> --coordinator-id {cid}"),
    );

    format!(
        "{scope}\n\n\
         Pickable tiskets:\n{tisket_list}\n\n\
         Read each tisket before dispatching to understand the task:\n\
         \x20 tisket issue show <id>\n\n\
         Dispatch a worker for each tisket:\n\
         \x20 {dispatch_cmd}\n\n\
         Monitor worker progress:\n\
         \x20 clc workers              # list all workers and their status\n\
         \x20 clc worker <id> check    # see recent output from a worker\n\
         \x20 clc worker <id> send \"<message>\"  # send a message to a worker\n\n\
         Land completed work (worker must be in `done` phase):\n\
         \x20 clc land <id>\n\n\
         When all dispatched workers have completed and been landed, you are done."
    )
}

fn build_coordinator_system_prompt(policy: &PermissionPolicy) -> String {
    // NOTE: This is prompt content. Approved by tisket spec for this feature.
    let base = String::from(
        "You are the coordinator agent. You do not write code — you manage worker agents \
         that do. You run on the main branch. Workers run in worktrees.\n\n\
         Two CLI tools are available: `clc` (workflow engine) and `tisket` (issue tracker). \
         Both support `--help` on every subcommand — use it to discover usage and flags.\n\n\
         Your job is to keep workers moving. When you start, and any time you finish \
         landing or have no active workers to monitor:\n\n\
         1. Check for todo tiskets: `tisket issue list`\n\
         2. For each todo tisket, read it (`tisket issue show <id>`) and dispatch \
         a worker (`clc dispatch <id>`). Do this immediately — do not ask for \
         permission to dispatch. Dispatching is your primary function.\n\
         3. Monitor active workers: `clc workers` (overview), `clc worker <id> check` (detail)\n\
         4. Intervene if needed: `clc worker <id> send \"<message>\"`\n\
         5. Land completed work: `clc land <id>` (worker must be in `done` phase)\n\
         6. After landing, go back to step 1.\n\n\
         You are autonomous. Do not ask the user \"should I dispatch?\" or \"want me to \
         pick up more work?\" — just do it. The only time to pause and consult the \
         user is when a tisket explicitly says it requires human judgment or approval.\n\n\
         Workers have limited permissions. When a worker needs a tool that isn't \
         pre-approved, it files a request with `clc permissions request` and stops. \
         Check for pending requests with `clc permissions list` or `clc worker <id> check`. \
         Grant with `clc permissions grant <id> \"<permission rule>\"`, then resume the worker. \
         If the request seems inappropriate or dangerous, escalate to the user with \
         `clc permissions escalate <id> \"<description>\"` instead of granting directly. \
         The user can review pending escalations with `clc permissions inbox`.",
    );

    let policy_section = format_policy_section(policy);

    let rest = String::from(
        "Missouri is the project's state-graph test framework. Tests live in `tests/missouri/` \
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
         LANDING PLAYBOOK\n\n\
         Before landing a worker:\n\
         1. Confirm the worker reached done phase: `clc worker <id> check` — look for phase: done\n\
         2. If the worker is dead (process not running) but not in done phase, it failed. \
         Re-dispatch with `clc dispatch <id>` — stale worktree cleanup runs automatically, \
         then a fresh worker is dispatched.\n\
         3. If the worker is alive but stuck, send it guidance: \
         `clc worker <id> send \"<message>\"`\n\n\
         When `clc land <id>` fails, read the error:\n\
         - \"not a descendant of HEAD\" — main advanced since the worker branched. \
         This resolves automatically: `clc land` rebases the branch onto HEAD before merging.\n\
         - \"phase is not done\" — worker stopped before finishing. Resume it: \
         `clc worker <id> resume`, then send instructions to complete.\n\
         - \"working tree has uncommitted changes\" — something is dirty on trunk. \
         Run `git status` to investigate before retrying.\n\n\
         After a successful landing, immediately check for more todo tiskets and dispatch. \
         Do not pause or ask the user — landing one worker and dispatching the next is a \
         single continuous action.\n\n\
         The user communicates with you via messages on stdin. When you receive a user \
         message, respond to it and act on their instructions. Between user messages, \
         continue monitoring active workers and landing completed ones.",
    );

    format!("{base}\n\n{policy_section}\n\n{rest}")
}

/// Format the permission policy as a prompt section for the coordinator.
fn format_policy_section(policy: &PermissionPolicy) -> String {
    use std::fmt::Write;

    // NOTE: This is prompt content. Required by tisket spec:
    // "The coordinator's system prompt includes its policy."
    if policy.escalate_all {
        return String::from(
            "PERMISSION POLICY\n\n\
             All permission requests must be escalated to the user. Do not grant any \
             permission directly — always use `clc permissions escalate <id> \"<description>\"`.",
        );
    }

    let has_auto_grant = !policy.auto_grant.is_empty();
    let has_always_escalate = !policy.always_escalate.is_empty();

    if !has_auto_grant && !has_always_escalate {
        return String::from(
            "PERMISSION POLICY\n\n\
             No specific permission policy is configured. When a worker requests a permission, \
             use your judgment. Bias toward escalation — grant only when the request is clearly \
             safe and necessary for the worker's task. When in doubt, escalate.",
        );
    }

    let mut section = String::from(
        "PERMISSION POLICY\n\n\
         When a worker requests a permission via `clc permissions request`:\n",
    );

    if has_auto_grant {
        section.push_str(
            "\nAuto-grant these patterns immediately \
             (grant with `clc permissions grant` and resume without asking the user):\n",
        );
        for pattern in &policy.auto_grant {
            let _ = writeln!(section, "- {pattern}");
        }
    }

    if has_always_escalate {
        section.push_str(
            "\nAlways escalate these patterns to the user \
             (never grant directly — use `clc permissions escalate`):\n",
        );
        for pattern in &policy.always_escalate {
            let _ = writeln!(section, "- {pattern}");
        }
    }

    section.push_str(
        "\nFor requests that don't match either list, use your judgment with a bias \
         toward escalation. Grant only when the request is clearly safe and necessary.",
    );

    section
}

fn read_pid(worker_dir: &Path) -> Option<i32> {
    std::fs::read_to_string(worker_dir.join("pid"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn is_pid_alive(pid: i32) -> bool {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- resolve_policy tests ---

    fn empty_filters() -> CoordinateFilters<'static> {
        CoordinateFilters {
            tisket: None,
            label: None,
            exclude_label: None,
            project: None,
            depends_on: None,
            dry_run: false,
            coordinator_id: None,
            auto_grant: &[],
            escalate_all: false,
            grant_config: None,
        }
    }

    #[test]
    fn resolve_policy_empty_config_and_flags() {
        let config = CoordinatorConfig::default();
        let filters = empty_filters();
        let policy = resolve_policy(&config, &filters).unwrap();
        assert!(policy.auto_grant.is_empty());
        assert!(policy.always_escalate.is_empty());
        assert!(!policy.escalate_all);
    }

    #[test]
    fn resolve_policy_from_config_only() {
        let config = CoordinatorConfig {
            auto_grant: vec!["Bash(cargo *)".into()],
            always_escalate: vec!["Bash(rm *)".into()],
        };
        let filters = empty_filters();
        let policy = resolve_policy(&config, &filters).unwrap();
        assert_eq!(policy.auto_grant, vec!["Bash(cargo *)"]);
        assert_eq!(policy.always_escalate, vec!["Bash(rm *)"]);
    }

    #[test]
    fn resolve_policy_from_cli_flags_only() {
        let config = CoordinatorConfig::default();
        let cli_grants = vec!["Bash(npm *)".into(), "Bash(make *)".into()];
        let mut filters = empty_filters();
        filters.auto_grant = &cli_grants;
        let policy = resolve_policy(&config, &filters).unwrap();
        assert_eq!(policy.auto_grant, vec!["Bash(npm *)", "Bash(make *)"]);
    }

    #[test]
    fn resolve_policy_merges_config_and_cli() {
        let config = CoordinatorConfig {
            auto_grant: vec!["Bash(cargo *)".into()],
            always_escalate: vec!["Bash(rm *)".into()],
        };
        let cli_grants = vec!["Bash(npm *)".into()];
        let mut filters = empty_filters();
        filters.auto_grant = &cli_grants;
        let policy = resolve_policy(&config, &filters).unwrap();
        assert_eq!(policy.auto_grant, vec!["Bash(cargo *)", "Bash(npm *)"]);
        assert_eq!(policy.always_escalate, vec!["Bash(rm *)"]);
    }

    #[test]
    fn resolve_policy_escalate_all_flag() {
        let config = CoordinatorConfig {
            auto_grant: vec!["Bash(cargo *)".into()],
            ..CoordinatorConfig::default()
        };
        let mut filters = empty_filters();
        filters.escalate_all = true;
        let policy = resolve_policy(&config, &filters).unwrap();
        assert!(policy.escalate_all);
        // auto_grant is still populated (but escalate_all overrides in prompt)
        assert_eq!(policy.auto_grant, vec!["Bash(cargo *)"]);
    }

    #[test]
    fn resolve_policy_grant_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("policy.yml");
        std::fs::write(
            &path,
            "auto_grant:\n  - \"Bash(docker *)\"\nalways_escalate:\n  - \"Bash(sudo *)\"\n",
        )
        .unwrap();

        let config = CoordinatorConfig::default();
        let path_str = path.to_str().unwrap().to_string();
        let mut filters = empty_filters();
        filters.grant_config = Some(&path_str);
        let policy = resolve_policy(&config, &filters).unwrap();
        assert_eq!(policy.auto_grant, vec!["Bash(docker *)"]);
        assert_eq!(policy.always_escalate, vec!["Bash(sudo *)"]);
    }

    #[test]
    fn resolve_policy_grant_config_merges_with_config_and_cli() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("policy.yml");
        std::fs::write(&path, "auto_grant:\n  - \"Bash(docker *)\"\n").unwrap();

        let config = CoordinatorConfig {
            auto_grant: vec!["Bash(cargo *)".into()],
            ..CoordinatorConfig::default()
        };
        let cli_grants = vec!["Bash(npm *)".into()];
        let path_str = path.to_str().unwrap().to_string();
        let mut filters = empty_filters();
        filters.auto_grant = &cli_grants;
        filters.grant_config = Some(&path_str);
        let policy = resolve_policy(&config, &filters).unwrap();
        // Order: config → grant-config → CLI
        assert_eq!(
            policy.auto_grant,
            vec!["Bash(cargo *)", "Bash(docker *)", "Bash(npm *)"]
        );
    }

    #[test]
    fn resolve_policy_grant_config_nonexistent_file_errors() {
        let config = CoordinatorConfig::default();
        let path = "/tmp/no-such-policy-file-12345.yml".to_string();
        let mut filters = empty_filters();
        filters.grant_config = Some(&path);
        assert!(resolve_policy(&config, &filters).is_err());
    }

    #[test]
    fn resolve_policy_grant_config_invalid_yaml_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.yml");
        std::fs::write(&path, "][not valid yaml{{{").unwrap();

        let config = CoordinatorConfig::default();
        let path_str = path.to_str().unwrap().to_string();
        let mut filters = empty_filters();
        filters.grant_config = Some(&path_str);
        assert!(resolve_policy(&config, &filters).is_err());
    }

    // --- format_policy_section tests ---

    #[test]
    fn policy_section_empty_policy_mentions_judgment() {
        let policy = PermissionPolicy::default();
        let section = format_policy_section(&policy);
        assert!(section.contains("PERMISSION POLICY"));
        assert!(section.contains("judgment"));
    }

    #[test]
    fn policy_section_escalate_all_mentions_escalate() {
        let policy = PermissionPolicy {
            escalate_all: true,
            ..PermissionPolicy::default()
        };
        let section = format_policy_section(&policy);
        assert!(section.contains("PERMISSION POLICY"));
        assert!(section.contains("escalated to the user"));
        assert!(!section.contains("Auto-grant"));
    }

    #[test]
    fn policy_section_with_auto_grant_lists_patterns() {
        let policy = PermissionPolicy {
            auto_grant: vec!["Bash(cargo *)".into(), "Bash(npm *)".into()],
            ..PermissionPolicy::default()
        };
        let section = format_policy_section(&policy);
        assert!(section.contains("Auto-grant"));
        assert!(section.contains("Bash(cargo *)"));
        assert!(section.contains("Bash(npm *)"));
    }

    #[test]
    fn policy_section_with_always_escalate_lists_patterns() {
        let policy = PermissionPolicy {
            always_escalate: vec!["Bash(rm *)".into(), "Bash(git push *)".into()],
            ..PermissionPolicy::default()
        };
        let section = format_policy_section(&policy);
        assert!(section.contains("escalate"));
        assert!(section.contains("Bash(rm *)"));
        assert!(section.contains("Bash(git push *)"));
    }

    #[test]
    fn policy_section_with_both_lists_has_both_sections() {
        let policy = PermissionPolicy {
            auto_grant: vec!["Bash(cargo *)".into()],
            always_escalate: vec!["Bash(rm *)".into()],
            escalate_all: false,
        };
        let section = format_policy_section(&policy);
        assert!(section.contains("Auto-grant"));
        assert!(section.contains("Bash(cargo *)"));
        assert!(section.contains("escalate"));
        assert!(section.contains("Bash(rm *)"));
    }

    #[test]
    fn system_prompt_includes_policy_section() {
        let policy = PermissionPolicy {
            auto_grant: vec!["Bash(cargo *)".into()],
            ..PermissionPolicy::default()
        };
        let prompt = build_coordinator_system_prompt(&policy);
        assert!(prompt.contains("PERMISSION POLICY"));
        assert!(prompt.contains("Bash(cargo *)"));
        // Also still contains the base coordinator instructions.
        assert!(prompt.contains("coordinator agent"));
        assert!(prompt.contains("LANDING PLAYBOOK"));
    }
}
