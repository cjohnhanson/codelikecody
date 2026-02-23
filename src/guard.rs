use crate::event::{Event, Response};
use crate::git::GitState;

const ALLOWED_ON_MAIN: &[&str] = &["Read", "Glob", "Grep"];

/// Evaluate an event against the current git state and return the appropriate response.
pub fn evaluate(event: &Event, git: Option<&GitState>) -> Response {
    match event {
        Event::AboutToUseTool { tool_name, .. } => check_tool_use(tool_name, git),
        Event::SessionStarting { .. } => session_context(git),
        _ => Response::Passthrough,
    }
}

fn check_tool_use(tool_name: &str, git: Option<&GitState>) -> Response {
    let Some(state) = git else {
        return Response::Passthrough;
    };

    if !state.is_main {
        return Response::Passthrough;
    }

    if ALLOWED_ON_MAIN.contains(&tool_name) {
        return Response::Passthrough;
    }

    Response::Block {
        message: format!(
            "Blocked: {tool_name} is not allowed on the main branch.\n\
             Only read operations (Read, Glob, Grep) are permitted on main.\n\
             Create a worktree to make changes: git worktree add .worktrees/<name> -b <branch>"
        ),
    }
}

fn session_context(git: Option<&GitState>) -> Response {
    let Some(state) = git else {
        return Response::Allow {
            context: Some("clc is active. No git repository detected.".to_string()),
        };
    };

    if state.is_main {
        Response::Allow {
            context: Some(format!(
                "clc is active. On branch '{}' (main). \
                 Write operations are blocked. \
                 Pick up a tisket and create a worktree to begin work.",
                state.branch
            )),
        }
    } else {
        Response::Allow {
            context: Some(format!("clc is active. On branch '{}'.", state.branch)),
        }
    }
}
