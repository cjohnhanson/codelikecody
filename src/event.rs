use serde_json::Value;

/// Agent-agnostic events that clc understands.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
    SessionStarting {
        source: String,
    },
    PromptSubmitted {
        prompt: String,
    },
    AboutToUseTool {
        tool_name: String,
        tool_input: Value,
    },
    AfterToolUse {
        tool_name: String,
        tool_input: Value,
        tool_response: Value,
    },
    AfterToolFailure {
        tool_name: String,
        error: String,
    },
    AgentStopping,
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
