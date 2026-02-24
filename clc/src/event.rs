use serde_json::Value;

/// Agent-agnostic events that clc understands.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
    SessionStart {
        source: String,
    },
    UserPromptSubmit {
        prompt: String,
    },
    PreToolUse {
        tool_name: String,
        tool_input: Value,
    },
    PostToolUse {
        tool_name: String,
        tool_input: Value,
        tool_response: Value,
    },
    PostToolUseFailure {
        tool_name: String,
        error: String,
    },
    Stop,
    Unknown {
        name: String,
    },
}

/// What clc tells the adapter to do in response to an event.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Response {
    /// Allow the action, optionally injecting context.
    Allow { context: Option<String> },
    /// Block the action with a message fed back to the agent.
    Block { message: String },
    /// Pass through without opinion.
    Passthrough,
}
