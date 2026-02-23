use serde_json::{Value, json};

use crate::error::Error;
use crate::event::{Event, Response};

use super::Adapter;

pub struct ClaudeCodeAdapter;

impl Adapter for ClaudeCodeAdapter {
    fn parse_event(&self, input: &Value) -> Result<Event, Error> {
        let event_name = input
            .get("hook_event_name")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::NonBlocking("missing hook_event_name in input".to_string()))?;

        match event_name {
            "SessionStart" => {
                let source = input
                    .get("source")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                Ok(Event::SessionStarting { source })
            }
            "UserPromptSubmit" => {
                let prompt = input
                    .get("prompt")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                Ok(Event::PromptSubmitted { prompt })
            }
            "PreToolUse" => {
                let tool_name = input
                    .get("tool_name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let tool_input = input.get("tool_input").cloned().unwrap_or(Value::Null);
                Ok(Event::AboutToUseTool {
                    tool_name,
                    tool_input,
                })
            }
            "PostToolUse" => {
                let tool_name = input
                    .get("tool_name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let tool_input = input.get("tool_input").cloned().unwrap_or(Value::Null);
                let tool_response = input.get("tool_response").cloned().unwrap_or(Value::Null);
                Ok(Event::AfterToolUse {
                    tool_name,
                    tool_input,
                    tool_response,
                })
            }
            "PostToolUseFailure" => {
                let tool_name = input
                    .get("tool_name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let error = input
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                Ok(Event::AfterToolFailure { tool_name, error })
            }
            "Stop" | "SubagentStop" => Ok(Event::AgentStopping),
            _ => Ok(Event::Unknown {
                name: event_name.to_string(),
            }),
        }
    }

    fn format_response(&self, event: &Event, response: &Response) -> (Option<Value>, i32) {
        match response {
            Response::Block { .. } => (None, 2),
            Response::Passthrough => (None, 0),
            Response::Allow { context } => format_allow(event, context.as_deref()),
        }
    }
}

fn format_allow(event: &Event, context: Option<&str>) -> (Option<Value>, i32) {
    match event {
        Event::SessionStarting { .. } => {
            let ctx = context.unwrap_or("clc is active.");
            (
                Some(json!({
                    "hookSpecificOutput": {
                        "hookEventName": "SessionStart",
                        "additionalContext": ctx
                    }
                })),
                0,
            )
        }
        Event::AboutToUseTool { .. } => context.map_or((None, 0), |ctx| {
            (
                Some(json!({
                    "hookSpecificOutput": {
                        "hookEventName": "PreToolUse",
                        "permissionDecision": "allow",
                        "additionalContext": ctx
                    }
                })),
                0,
            )
        }),
        Event::PromptSubmitted { .. } => context.map_or((None, 0), |ctx| {
            (
                Some(json!({
                    "hookSpecificOutput": {
                        "hookEventName": "UserPromptSubmit",
                        "additionalContext": ctx
                    }
                })),
                0,
            )
        }),
        _ => (None, 0),
    }
}
