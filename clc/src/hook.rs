use std::fmt::Write;
use std::io::Read;
use std::path::Path;

use camino::Utf8Path;
use serde_json::Value;

use crate::adapter::Adapter;
use crate::adapter::claude_code::ClaudeCodeAdapter;
use crate::belmont;
use crate::config::{self, Config};
use crate::error::Error;
use crate::event::{Event, Response};
use crate::git;
use crate::guard;
use crate::missouri;
use crate::phase;
use crate::skills;
use crate::tisket;
use crate::workflow::Workflow;
use crate::zettel;

use clc_sdk::ClcTool;

/// Run the hook: read JSON from stdin, process event, write response to stdout.
/// Returns the exit code to use.
pub fn run() -> Result<i32, Error> {
    let input = read_stdin()?;
    let json: Value = serde_json::from_str(&input)
        .map_err(|e| Error::NonBlocking(format!("invalid JSON on stdin: {e}")))?;

    let adapter = ClaudeCodeAdapter;
    let event = adapter.parse_event(&json)?;

    let cwd = json
        .get("cwd")
        .and_then(Value::as_str)
        .map_or_else(|| Path::new("."), Path::new);
    let mut cfg = config::load(cwd).unwrap_or_default();
    let user_cfg = config::load_user_config().unwrap_or(None);
    if let Some(ref uc) = user_cfg {
        config::merge_user_config(&mut cfg, uc);
    }
    let git_state = git::detect(cwd, &cfg.main_branch, &cfg.admin_branch);

    // Load phase name (string, not enum) and resolve workflow.
    let phase_name = if std::env::var("CLC_API_URL").is_ok() {
        load_phase_name_from_api(git_state.as_ref())
    } else {
        phase::load_name(cwd).unwrap_or(None)
    };

    let workflow = resolve_workflow(cwd, &cfg);

    // Phase bootstrap: auto-set initial phase on unphased feature branches
    // with a matching tisket. Skip when using API — the supervisor manages phase.
    let phase_name = if matches!(event, Event::SessionStart { .. })
        && std::env::var("CLC_API_URL").is_err()
    {
        maybe_bootstrap_phase(cwd, git_state.as_ref(), phase_name, &workflow)
    } else {
        phase_name
    };

    // Detect reviewer sessions via env var.
    let review_type = std::env::var("CLC_REVIEW_TYPE").ok();
    let is_reviewer = review_type.is_some();

    let response = match event {
        Event::SessionStart { .. } => {
            let prime = if is_reviewer {
                assemble_reviewer_prime(
                    cwd,
                    git_state.as_ref(),
                    review_type.as_deref().unwrap_or(""),
                    &workflow,
                    &cfg,
                )
            } else {
                assemble_prime(cwd, git_state.as_ref(), phase_name.as_deref(), &workflow, &cfg)
            };
            Response::Allow {
                context: Some(prime),
            }
        }
        Event::UserPromptSubmit { .. } => {
            let reinforcement = if is_reviewer {
                format!("reviewer: type={}", review_type.as_deref().unwrap_or("?"))
            } else {
                assemble_reinforcement(cwd, git_state.as_ref(), phase_name.as_deref(), &cfg)
            };
            Response::Allow {
                context: Some(reinforcement),
            }
        }
        Event::PostToolUse { ref tool_name, .. } if !is_reviewer => {
            let nudge = post_tool_nudge_workflow(tool_name, phase_name.as_deref(), &workflow);
            nudge.map_or(Response::Passthrough, |text| Response::Allow {
                context: Some(text),
            })
        }
        Event::PreToolUse { ref tool_name, ref tool_input }
            if std::env::var("CLC_API_URL").is_ok() =>
        {
            // Check local phase/permission guards first. Only escalate to the
            // supervisor API if the local guard doesn't explicitly allow it.
            let local = guard::evaluate_with_workflow(&event, git_state.as_ref(), phase_name.as_deref(), &workflow, cwd);
            match local {
                Response::Passthrough => {
                    // Local guard says passthrough — tool is allowed locally.
                    // No need to call the API for tools that are already permitted.
                    Response::Passthrough
                }
                Response::Block { .. } => {
                    // Local guard blocked it — check if the API has a grant.
                    let api_result = check_tool_via_api(tool_name, tool_input, git_state.as_ref());
                    if matches!(api_result, Response::Passthrough) {
                        api_result
                    } else {
                        // Both local and API denied — use local message (more specific).
                        local
                    }
                }
                other => other,
            }
        }
        Event::Stop if std::env::var("CLC_API_URL").is_ok() => {
            // Allow stopping when a permission escalation is pending —
            // the worker should not burn tokens waiting for coordinator.
            let guard_result = guard::evaluate_with_workflow(
                &event, git_state.as_ref(), phase_name.as_deref(), &workflow, cwd,
            );
            if matches!(guard_result, Response::Block { .. }) {
                if has_pending_escalation(git_state.as_ref()) {
                    Response::Passthrough
                } else {
                    guard_result
                }
            } else {
                guard_result
            }
        }
        _ => guard::evaluate_with_workflow(&event, git_state.as_ref(), phase_name.as_deref(), &workflow, cwd),
    };

    let (output, exit_code) = adapter.format_response(&event, &response);

    if let Some(json_output) = output {
        let formatted = serde_json::to_string(&json_output)
            .map_err(|e| Error::NonBlocking(format!("failed to serialize response: {e}")))?;
        println!("{formatted}");
    }

    if let Response::Block { ref message } = response {
        eprintln!("{message}");
    }

    Ok(exit_code)
}

/// Assemble the full prime text from clc header + tisket + missouri + skills.
#[allow(clippy::too_many_lines)]
fn assemble_prime(
    cwd: &Path,
    git: Option<&git::GitState>,
    phase_name: Option<&str>,
    workflow: &Workflow,
    cfg: &Config,
) -> String {
    let ctx = clc_sdk::PrimeContext {
        phase: phase_name.map(|p| p.to_string()),
    };

    let mut out = String::new();

    // --- clean output directive for untracked mode ---
    if crate::is_untracked(cwd) {
        out.push_str(
            "## CRITICAL: Clean output mode\n\n\
             This project uses clc in untracked mode. Workflow tooling should not\n\
             leak into project artifacts.\n\n\
             Do not reference clc, tisket, missouri, zettel, almanac, belmont, or\n\
             any commands specific to these tools in commit messages, PR descriptions,\n\
             code comments, or documentation. These are workflow implementation\n\
             details — they belong in your process, not in the project's history.\n\n\
             Commit messages should describe what changed and why.\n\
             PR descriptions should describe motivation and impact.\n\
             Code comments should explain the code, not the workflow that produced it.\n\n\
             Internal tool use is unaffected — use clc commands, load skills, track\n\
             issues normally. Just keep the artifacts clean.\n\n",
        );
    }

    // --- clc header: what the agent is inside of ---
    out.push_str("# clc workflow engine\n\n");
    out.push_str(
        "This session is governed by clc via Claude Code hooks.\n\n\
         Hooks fire on every event in this session:\n\
         - **SessionStart**: injects this context\n\
         - **PreToolUse**: blocks file writes on trunk, enforces phase constraints on branches\n\
         - **PostToolUse**: reminds you of phase obligations after edits\n\
         - **UserPromptSubmit**: reinforces current state on every prompt\n\
         - **Stop**: prevents you from stopping if work is incomplete\n\n\
         These are not suggestions. The hooks will reject tool calls that violate\n\
         workflow constraints. Work with them, not around them.\n\n",
    );

    // --- current state ---
    out.push_str("## Current state\n\n");
    if let Some(state) = git {
        let _ = write!(out, "Branch: `{}`", state.branch);
        if state.is_main {
            out.push_str(" (trunk)");
        }
        if state.is_admin {
            out.push_str(" (admin)");
        }
        if state.is_worktree {
            out.push_str(" [worktree]");
        }
        out.push('\n');
    } else {
        out.push_str("No git repository detected.\n");
    }
    if let Some(ref p) = ctx.phase {
        let _ = writeln!(out, "Phase: `{p}`");
    }
    out.push('\n');

    // --- trunk directives ---
    if let Some(state) = git
        && state.is_main
    {
        out.push_str(
            "## What to do on trunk\n\n\
             Trunk is read-only. Edit, Write, and NotebookEdit are blocked. Read, Glob,\n\
             and Grep always pass. Bash is restricted to an allowlist of safe commands:\n\
             git, cargo test/check/build/clippy/fmt, clc, missouri, tisket issue list/show/path,\n\
             tisket search, ls, pwd, which, cat, head, tail, wc, find, tree. Other Bash\n\
             commands are blocked — false positives are better than accidental writes on trunk.\n\n\
             To begin work on a tisket:\n\n\
             \x20   clc pickup <issue-id>\n\n\
             For admin work without a specific tisket (triage, planning, reviewing):\n\n\
             \x20   clc admin\n\n\
             Both commands create a dedicated worktree. All implementation and file\n\
             modifications happen in worktrees, never on trunk.\n\n",
        );
    }

    // --- workflow description ---
    out.push_str("## The workflow loop\n\n");
    out.push_str("1. `clc pickup <id>` — creates a worktree, checks out a branch, sets phase\n");
    if let Some(desc) = workflow.description() {
        let _ = writeln!(out, "2. {desc}");
    }
    out.push_str("\nPhases: ");
    let names: Vec<&str> = workflow.phase_names().collect();
    out.push_str(&names.join(" → "));
    out.push_str("\n\n");
    out.push_str(
        "Phases constrain what you can edit and whether you can stop. The hooks\n\
         enforce this automatically. Run `clc status` to see where you are.\n\n",
    );

    // --- phase instructions ---
    if let Some(name) = phase_name {
        if let Some(instructions) = workflow.phase_instructions(name) {
            let _ = writeln!(out, "**Current phase `{name}`:** {instructions}\n");
        }
    }

    // --- TDD mandate (only for TDD-like workflows with test phases) ---
    if workflow.phase_names().any(|n| n.contains("test")) {
        out.push_str(
            "## Test-driven development\n\n\
             Every implementation change starts with a test. This is not a phase system\n\
             rule — it is the development methodology for all work in this project.\n\n\
             1. Write a failing test that specifies the desired behavior\n\
             2. Verify the test fails (red)\n\
             3. Write the minimum code to make it pass (green)\n\
             4. Refactor if needed, keeping tests green\n\n\
             Do not write implementation code without a corresponding test. If you find\n\
             yourself editing source files without having written or updated tests first,\n\
             stop and write the test. Phase gates enforce this mechanically, but TDD\n\
             discipline should hold even when phase gates are absent or permissive.\n\n",
        );
    }

    // --- working memory ---
    out.push_str(
        "## Working memory\n\n\
         Your tisket file has a `## Scratch Notes` section. Find it with\n\
         `tisket issue path <id>`. Read it at session start. Write to it as you work.\n\n\
         This is your only persistent state. Context compaction and session\n\
         boundaries destroy everything else. If it's not in scratch notes, it's gone.\n\n\
         Write: decisions, files consulted (and which were irrelevant), approaches\n\
         tried, current status, next steps. Update proactively throughout the session,\n\
         not at the end.\n\n",
    );

    // --- commit discipline ---
    out.push_str(
        "## Commit discipline\n\n\
         Commit frequently. Commits are checkpoints, not milestones. Good pre-commit\n\
         hooks mean every commit is a validated state. Don't accumulate large\n\
         uncommitted diffs — stage and commit as you go.\n\n\
         Stage and commit all implementation changes before running `clc done` to\n\
         finalize. The done command requires a clean working tree — uncommitted\n\
         changes will cause it to fail.\n\n",
    );

    // --- capturing discovered work ---
    out.push_str(
        "## Capturing discovered work\n\n\
         If you discover work that needs doing — a bug, a missing feature, a refactor —\n\
         don't go off on a tangent. Create a tisket for it:\n\n\
         \x20   tisket issue create -p v0.1.0 \"title\"\n\n\
         Tisket is scratch paper for future work. Capture it and move on.\n\n",
    );

    // --- tisket status lifecycle ---
    out.push_str(
        "## Tisket status lifecycle\n\n\
         Issues start in `discovery` — an idea captured, not yet scoped. Do not\n\
         create issues directly in `todo`. The progression is:\n\n\
         \x20   discovery → todo → in_progress → done\n\n\
         Moving from discovery to todo is a deliberate scoping decision: the issue\n\
         has a clear title, a body that defines what done looks like, and enough\n\
         detail for a worker to pick it up without guessing. When creating tiskets\n\
         to capture discovered work, always use the default status (discovery).\n\
         Only the admin session or a human promotes issues to todo after scoping.\n\n",
    );

    // --- tisket section ---
    let branch = git.map(|s| s.branch.as_str());
    match tisket::detect(cwd, branch) {
        Ok(tisket_state) => {
            let section = tisket_state.prime(&ctx);
            if !section.is_empty() {
                out.push_str(&section);
                out.push('\n');
            }
        }
        Err(e) => {
            let _ = write!(out, "## Tisket\n\ntisket error: {e}\n\n");
        }
    }

    // --- missouri section ---
    match missouri::detect(cwd) {
        Ok(missouri_state) => {
            let section = missouri_state.prime(&ctx);
            if !section.is_empty() {
                out.push_str(&section);
                out.push('\n');
            }
        }
        Err(e) => {
            let _ = write!(out, "## Missouri\n\nmissouri error: {e}\n\n");
        }
    }

    // --- zettel section ---
    match zettel::detect(cwd) {
        Ok(zettel_state) if zettel_state.has_repo => {
            let section = zettel_state.prime(&ctx);
            if !section.is_empty() {
                out.push_str(&section);
                out.push('\n');
            }
        }
        Ok(_) => {} // not initialized — omit silently
        Err(e) => {
            let _ = write!(out, "## Zettel\n\nzettel error: {e}\n\n");
        }
    }

    // --- belmont section ---
    match belmont::detect(cwd) {
        Ok(belmont_state) if belmont_state.initialized => {
            let section = belmont_state.prime(&ctx);
            if !section.is_empty() {
                out.push_str(&section);
                out.push('\n');
            }
        }
        Ok(_) => {} // not initialized — omit silently
        Err(e) => {
            let _ = write!(out, "## Belmont\n\nbelmont error: {e}\n\n");
        }
    }

    // --- almanac (skills) section ---
    let almanac_state = skills::detect(cwd, &cfg.skills);
    let section = almanac_state.prime(&ctx);
    if !section.is_empty() {
        out.push_str(&section);
    }

    // --- clc remind section ---
    out.push_str(
        "## Reminders (`clc remind`)\n\n\
         `clc remind` is a background timer. It sleeps, then prints a message.\n\
         Run it in the background as insurance against stopping mid-loop.\n\n\
         ### Why this exists\n\n\
         Context compaction, model decisions, and session limits can stop you\n\
         mid-work. If you're in an iterative loop — making changes, dispatching\n\
         reviews, resolving feedback, repeating until clean — a stop means the\n\
         loop dies silently. `clc remind` prevents that: if you stop, the\n\
         reminder fires and tells you to resume.\n\n\
         ### Usage\n\n\
         \x20 clc remind <seconds> <message>              One-shot reminder\n\
         \x20 clc remind <seconds> <message> --repeat N   Fires N+1 times total\n\n\
         When `--repeat` is set, the output includes a command to re-run with\n\
         the decremented count. Run that command in the background to continue.\n\n\
         ### When to set a reminder\n\n\
         Any time you're told to loop or iterate without stopping until a\n\
         condition is met. The pattern:\n\n\
         1. Start the work loop\n\
         2. Set a background reminder with enough time for one iteration\n\
         3. Do the work\n\
         4. If the reminder fires before you're done, you stopped — resume\n\
            the loop and set the reminder again\n\
         5. If you finish before the reminder fires, the reminder is harmless\n\n\
         ### Example\n\n\
         User says: \"Iterate on this code. Dispatch review agents, resolve\n\
         their feedback, repeat until reviews come back clean. Don't stop.\"\n\n\
         ```bash\n\
         # Set a 5-minute reminder with 10 repeats as insurance\n\
         clc remind 300 'You were iterating: make changes, dispatch review \\\n\
         agents, resolve feedback, repeat until clean. Resume the loop.' \\\n\
         --repeat 10 &\n\
         ```\n\n\
         The message should describe what you were doing so you can resume\n\
         with context if the reminder fires in a new session.\n\n",
    );

    out
}

/// Assemble prime text for a reviewer session.
fn assemble_reviewer_prime(
    cwd: &Path,
    git: Option<&git::GitState>,
    review_type: &str,
    workflow: &Workflow,
    _cfg: &Config,
) -> String {
    use std::fmt::Write;

    let mut out = String::new();

    out.push_str("# clc reviewer session\n\n");
    let _ = writeln!(out, "You are a **reviewer agent** performing a **{review_type}** review.");
    out.push_str(
        "Your role is to examine the work in this worktree and render a verdict.\n\
         You are NOT the worker — do not make changes to the code.\n\n",
    );

    // Branch context
    if let Some(state) = git {
        let _ = writeln!(out, "Branch: `{}`", state.branch);
    }
    let _ = writeln!(out, "Review type: `{review_type}`\n");

    // Review instructions from workflow config
    if let Some(review_def) = workflow.review_def(review_type) {
        if let Some(instructions) = &review_def.instructions {
            out.push_str("## Review instructions\n\n");
            out.push_str(instructions);
            out.push_str("\n\n");
        }
    }

    // Verdict commands
    out.push_str("## Rendering your verdict\n\n");
    out.push_str("When your review is complete, render exactly one verdict:\n\n");
    out.push_str("- **Approve:** `clc review approve \"optional comments\"`\n");
    out.push_str("- **Request changes:** `clc review request-changes \"description of what needs to change\"`\n\n");
    out.push_str(
        "You must render a verdict before stopping. The worker is waiting on your review.\n\n",
    );

    // Tisket context (reviewer sees the same tisket as the worker)
    let ctx = clc_sdk::PrimeContext { phase: None };
    let branch = git.map(|s| s.branch.as_str());
    if let Ok(tisket_state) = tisket::detect(cwd, branch) {
        let section = tisket_state.prime(&ctx);
        if !section.is_empty() {
            out.push_str(&section);
            out.push('\n');
        }
    }

    out
}

/// Build and return the prime text for CLI output.
pub fn prime_text() -> Result<String, Error> {
    let cwd = std::env::current_dir()?;
    let mut cfg = config::load(&cwd).unwrap_or_default();
    if let Some(ref uc) = config::load_user_config().unwrap_or(None) {
        config::merge_user_config(&mut cfg, uc);
    }
    let git_state = git::detect(&cwd, &cfg.main_branch, &cfg.admin_branch);
    let phase_name = phase::load_name(&cwd).unwrap_or(None);
    let workflow = resolve_workflow(&cwd, &cfg);
    Ok(assemble_prime(&cwd, git_state.as_ref(), phase_name.as_deref(), &workflow, &cfg))
}

/// Assemble lean status reinforcement for `UserPromptSubmit`.
fn assemble_reinforcement(
    cwd: &Path,
    git: Option<&git::GitState>,
    phase_name: Option<&str>,
    cfg: &Config,
) -> String {
    let mut out = String::new();

    let branch = git.map(|s| s.branch.as_str());
    if let Ok(tisket_state) = tisket::detect(cwd, branch) {
        let line = tisket_state.status_basic();
        if !line.is_empty() {
            out.push_str(&line);
            out.push('\n');
        }
    }

    if let Ok(missouri_state) = missouri::detect(cwd) {
        let line = missouri_state.status_basic();
        if !line.is_empty() {
            out.push_str(&line);
            out.push('\n');
        }
    }

    if let Ok(zettel_state) = zettel::detect(cwd) {
        if zettel_state.has_repo {
            let line = zettel_state.status_basic();
            out.push_str(&line);
            out.push('\n');
        }
    }

    if let Ok(belmont_state) = belmont::detect(cwd) {
        if belmont_state.initialized {
            let line = belmont_state.status_basic();
            if !line.is_empty() {
                out.push_str(&line);
                out.push('\n');
            }
        }
    }

    if !cfg.skills.is_empty() {
        let almanac_state = skills::detect(cwd, &cfg.skills);
        let line = almanac_state.status_basic();
        if !line.is_empty() {
            out.push_str(&line);
            out.push('\n');
        }
    }

    if let Some(p) = phase_name {
        let _ = writeln!(out, "phase: {p}");
    }

    out
}

/// File-modifying tools that trigger post-tool nudges.
const WRITE_TOOLS: &[&str] = &["Edit", "Write", "NotebookEdit"];

/// Return a phase-aware nudge using the workflow's per-phase nudge config.
fn post_tool_nudge_workflow(tool_name: &str, phase_name: Option<&str>, workflow: &Workflow) -> Option<String> {
    if !WRITE_TOOLS.contains(&tool_name) {
        return None;
    }

    let name = phase_name?;
    let nudge = workflow.phase_nudge(name)?;
    Some(format!("phase: {name} — {nudge}"))
}


/// If on a feature branch with no phase and a matching tisket, auto-set
/// the initial phase from the workflow. Returns the (possibly new) phase name.
fn maybe_bootstrap_phase(
    cwd: &Path,
    git: Option<&git::GitState>,
    current_phase: Option<String>,
    workflow: &Workflow,
) -> Option<String> {
    // Already has a phase — nothing to do.
    if current_phase.is_some() {
        return current_phase;
    }

    let state = git?;

    // Don't bootstrap on main or admin.
    if state.is_main || state.is_admin {
        return None;
    }

    // Check for a matching tisket.
    let branch = Some(state.branch.as_str());
    let tisket_state = tisket::detect(cwd, branch).ok()?;
    let issue = tisket_state.current_issue.as_ref()?;
    let issue_id = issue.id.clone();

    // Bootstrap: set phase to the workflow's initial phase.
    let initial = workflow.initial_phase();
    if phase::set_with_workflow(cwd, initial, 1, workflow).is_ok() {
        // Advance the matching tisket to in_progress.
        if let Some(s) = cwd.to_str()
            && let Ok(repo) = ::tisket::Repo::open(Utf8Path::new(s))
        {
            let _ = repo.edit_issue(
                &issue_id,
                ::tisket::EditIssueOptions {
                    status: Some("in_progress"),
                    ..Default::default()
                },
            );
        }
        Some(initial.to_string())
    } else {
        None
    }
}

/// Resolve the active workflow from config + state.
fn resolve_workflow(cwd: &Path, cfg: &Config) -> Workflow {
    // Try to read the workflow name from the state file.
    let wf_name = phase::load_workflow_name(cwd).unwrap_or(None);

    if let Some(name) = &wf_name {
        if let Some(def) = cfg.workflows.get(name) {
            if let Ok(wf) = Workflow::new(def) {
                return wf;
            }
        }
    }

    // No stored workflow or it's invalid — check if any workflows are configured.
    // If so, we can't resolve without issue labels, so fall back to default.
    Workflow::default_tdd()
}

/// Load phase name from the supervisor API (string, not enum).
fn load_phase_name_from_api(git: Option<&git::GitState>) -> Option<String> {
    let api_url = std::env::var("CLC_API_URL").ok()?;
    let agent_id = git.map(|s| s.branch.as_str()).unwrap_or("unknown");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;

    let result: Result<serde_json::Value, String> = rt.block_on(async {
        let client = crate::coordination_client::build_api_client()
            .map_err(|e| format!("{e}"))?;
        let resp = client
            .get(format!("{api_url}/agents/{agent_id}/phase"))
            .send()
            .await
            .map_err(|e| format!("{e}"))?;
        resp.json().await.map_err(|e| format!("{e}"))
    });

    match result {
        Ok(resp) => resp["phase"].as_str().map(|s| s.to_string()),
        Err(_) => None,
    }
}


/// Check a tool use via the supervisor API.
/// Returns Allow if granted, Block if denied (with escalation message).
fn check_tool_via_api(
    tool_name: &str,
    tool_input: &serde_json::Value,
    git: Option<&git::GitState>,
) -> Response {
    let api_url = match std::env::var("CLC_API_URL") {
        Ok(url) => url,
        Err(_) => return Response::Passthrough,
    };

    let agent_id = git
        .map(|s| s.branch.clone())
        .unwrap_or_else(|| "unknown".to_string());

    // Build a tool description for the check.
    let tool_desc = if tool_name == "Bash" {
        let cmd = tool_input
            .get("command")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        format!("Bash({})", cmd.split_whitespace().next().unwrap_or(cmd))
    } else {
        tool_name.to_string()
    };

    let body = serde_json::json!({
        "tool_name": tool_desc,
        "reason": format!("{tool_name} tool call"),
    });

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return Response::Passthrough,
    };

    let result: Result<serde_json::Value, String> = rt.block_on(async {
        let client = crate::coordination_client::build_api_client()
            .map_err(|e| format!("{e}"))?;
        let resp = client
            .post(format!("{api_url}/agents/{agent_id}/tool-check"))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("{e}"))?;
        resp.json().await.map_err(|e| format!("{e}"))
    });

    match result {
        Ok(resp) => {
            let allowed = resp["allowed"].as_bool().unwrap_or(false);
            if allowed {
                Response::Passthrough
            } else {
                let message = resp["message"]
                    .as_str()
                    .unwrap_or("Permission denied by supervisor")
                    .to_string();
                Response::Block { message }
            }
        }
        Err(_) => {
            // API unreachable — fall through to local guard.
            Response::Passthrough
        }
    }
}

/// Check if this agent has a pending permission escalation via the API.
/// Used by the Stop hook to allow stopping when waiting for coordinator approval.
fn has_pending_escalation(git: Option<&git::GitState>) -> bool {
    let api_url = match std::env::var("CLC_API_URL") {
        Ok(url) => url,
        Err(_) => return false,
    };
    let agent_id = git
        .map(|s| s.branch.clone())
        .unwrap_or_default();

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return false,
    };

    rt.block_on(async {
        let client = crate::coordination_client::build_api_client().ok()?;
        let resp = client
            .get(format!("{api_url}/agents/{agent_id}/permissions"))
            .send()
            .await
            .ok()?;
        let body: serde_json::Value = resp.json().await.ok()?;
        let pending = body.as_array().map(|a| !a.is_empty()).unwrap_or(false);
        Some(pending)
    })
    .unwrap_or(false)
}

fn read_stdin() -> Result<String, Error> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| Error::NonBlocking(format!("failed to read stdin: {e}")))?;
    Ok(buf)
}
