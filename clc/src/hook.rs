use std::fmt::Write;
use std::io::Read;
use std::path::Path;

use serde_json::Value;

use crate::adapter::Adapter;
use crate::adapter::claude_code::ClaudeCodeAdapter;
use crate::config;
use crate::error::Error;
use crate::event::{Event, Response};
use crate::git;
use crate::guard;
use crate::missouri;
use crate::phase;
use crate::tisket;

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
    let cfg = config::load(cwd).unwrap_or_default();
    let git_state = git::detect(cwd, &cfg.main_branch);
    let current_phase = phase::load(cwd).unwrap_or(None);

    // Phase bootstrap: auto-set tests-unwritten on unphased feature branches
    // with a matching tisket. This catches worktrees created outside `clc pickup`.
    let current_phase = if matches!(event, Event::SessionStart { .. }) {
        maybe_bootstrap_phase(cwd, git_state.as_ref(), current_phase)
    } else {
        current_phase
    };

    let response = match event {
        Event::SessionStart { .. } => {
            let prime = assemble_prime(cwd, git_state.as_ref(), current_phase);
            Response::Allow {
                context: Some(prime),
            }
        }
        Event::UserPromptSubmit { .. } => {
            let reinforcement = assemble_reinforcement(cwd, git_state.as_ref(), current_phase);
            Response::Allow {
                context: Some(reinforcement),
            }
        }
        Event::PostToolUse { ref tool_name, .. } => {
            let nudge = post_tool_nudge(tool_name, current_phase);
            nudge.map_or(Response::Passthrough, |text| Response::Allow {
                context: Some(text),
            })
        }
        _ => guard::evaluate(&event, git_state.as_ref(), current_phase),
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

/// Assemble the full prime text from clc header + tisket + missouri.
#[allow(clippy::too_many_lines)]
fn assemble_prime(cwd: &Path, git: Option<&git::GitState>, phase: Option<phase::Phase>) -> String {
    let ctx = clc_sdk::PrimeContext {
        phase: phase.map(|p| p.to_string()),
    };

    let mut out = String::new();

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
             Trunk is read-only for file modifications. Edit, Write, and NotebookEdit\n\
             tools are blocked. Bash, Read, Glob, Grep, and other tools work normally.\n\
             Use trunk for triage, planning, and picking up work.\n\n\
             To begin work, pick up a tisket:\n\n\
             \x20   clc pickup <issue-id>\n\n\
             This creates a worktree on a dedicated branch and sets the initial phase.\n\
             All implementation happens in worktrees, never on trunk.\n\n",
        );
    }

    // --- workflow loop ---
    out.push_str(
        "## The workflow loop\n\n\
         1. `clc pickup <id>` — creates a worktree, checks out a branch, sets phase\n\
         2. Write tests first — phase gates prevent implementation until tests exist\n\
         3. Implement — phase advances to `implementing`, all edits unlocked\n\
         4. Get green — run tests, reach `green` phase\n\
         5. `clc done` — finalize the work\n\n\
         Phases constrain what you can edit and whether you can stop. The hooks\n\
         enforce this automatically. Run `clc status` to see where you are.\n\n",
    );

    // --- TDD mandate ---
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

    // --- working memory ---
    out.push_str(
        "## Working memory\n\n\
         Your active tisket contains a `## Scratch Notes` section — working memory\n\
         that persists across sessions.\n\n\
         Write to it as you work: decisions made, approaches tried, files consulted,\n\
         what turned out irrelevant, next steps. On session start, read the scratch\n\
         notes to recover context.\n\n\
         To find the file: `tisket issue path <id>`\n\n\
         The scratch section is tracked separately from the issue body and is not\n\
         shown by `tisket issue show`. It is internal working state, not task\n\
         description.\n\n",
    );

    // --- commit discipline ---
    out.push_str(
        "## Commit discipline\n\n\
         Commit frequently. Commits are checkpoints, not milestones. Good pre-commit\n\
         hooks mean every commit is a validated state. Don't accumulate large\n\
         uncommitted diffs — stage and commit as you go.\n\n",
    );

    // --- capturing discovered work ---
    out.push_str(
        "## Capturing discovered work\n\n\
         If you discover work that needs doing — a bug, a missing feature, a refactor —\n\
         don't go off on a tangent. Create a tisket for it:\n\n\
         \x20   tisket issue create -t \"title\" -b \"description\"\n\n\
         Tisket is scratch paper for future work. Capture it and move on.\n\n",
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

    out
}

/// Build and return the prime text for CLI output.
pub fn prime_text() -> Result<String, Error> {
    let cwd = std::env::current_dir()?;
    let cfg = config::load(&cwd).unwrap_or_default();
    let git_state = git::detect(&cwd, &cfg.main_branch);
    let current_phase = phase::load(&cwd).unwrap_or(None);
    Ok(assemble_prime(&cwd, git_state.as_ref(), current_phase))
}

/// Assemble lean status reinforcement for `UserPromptSubmit`.
fn assemble_reinforcement(
    cwd: &Path,
    git: Option<&git::GitState>,
    phase: Option<phase::Phase>,
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

    if let Some(p) = phase {
        let _ = writeln!(out, "phase: {p}");
    }

    out
}

/// File-modifying tools that trigger post-tool nudges.
const WRITE_TOOLS: &[&str] = &["Edit", "Write", "NotebookEdit"];

/// Return a phase-aware nudge after a tool use, if applicable.
fn post_tool_nudge(tool_name: &str, phase: Option<phase::Phase>) -> Option<String> {
    if !WRITE_TOOLS.contains(&tool_name) {
        return None;
    }

    match phase {
        Some(phase::Phase::Implementing) => {
            Some("phase: implementing — run tests before advancing".to_string())
        }
        _ => None,
    }
}

/// If on a feature branch with no phase and a matching tisket, auto-set
/// the initial phase to `tests-unwritten`. Returns the (possibly new) phase.
fn maybe_bootstrap_phase(
    cwd: &Path,
    git: Option<&git::GitState>,
    current_phase: Option<phase::Phase>,
) -> Option<phase::Phase> {
    // Already has a phase — nothing to do.
    if current_phase.is_some() {
        return current_phase;
    }

    let state = git?;

    // Don't bootstrap on main.
    if state.is_main {
        return None;
    }

    // Check for a matching tisket.
    let branch = Some(state.branch.as_str());
    let tisket_state = tisket::detect(cwd, branch).ok()?;
    tisket_state.current_issue.as_ref()?;

    // Bootstrap: set phase to tests-unwritten.
    if phase::set(cwd, "tests-unwritten", 1).is_ok() {
        Some(phase::Phase::TestsUnwritten)
    } else {
        None
    }
}

fn read_stdin() -> Result<String, Error> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| Error::NonBlocking(format!("failed to read stdin: {e}")))?;
    Ok(buf)
}
