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
fn assemble_prime(cwd: &Path, git: Option<&git::GitState>, phase: Option<phase::Phase>) -> String {
    let ctx = clc_sdk::PrimeContext {
        phase: phase.map(|p| p.to_string()),
    };

    let mut out = String::new();

    // clc header
    out.push_str("# clc — codelikecody workflow engine\n\n");
    if let Some(state) = git {
        let _ = write!(out, "Branch: `{}`", state.branch);
        if state.is_main {
            out.push_str(" (main)");
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

    // Branch-specific directives
    if let Some(state) = git
        && state.is_main
    {
        out.push_str(
            "Write operations are blocked on the main branch.\n\
             Pick up a tisket to begin work: `clc pickup <issue-id>`\n\n",
        );
    }

    // Tisket section
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
            let _ = write!(out, "# Tisket\n\ntisket error: {e}\n\n");
        }
    }

    // Missouri section
    match missouri::detect(cwd) {
        Ok(missouri_state) => {
            let section = missouri_state.prime(&ctx);
            if !section.is_empty() {
                out.push_str(&section);
                out.push('\n');
            }
        }
        Err(e) => {
            let _ = write!(out, "# Missouri\n\nmissouri error: {e}\n\n");
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

fn read_stdin() -> Result<String, Error> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| Error::NonBlocking(format!("failed to read stdin: {e}")))?;
    Ok(buf)
}
