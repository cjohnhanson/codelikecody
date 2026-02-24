pub mod claude_code;

use serde_json::Value;

use crate::error::Error;
use crate::event::{Event, Response};

/// Translates between an agent's hook protocol and clc events.
pub trait Adapter {
    /// Parse raw hook input (JSON from stdin) into a clc event.
    fn parse_event(&self, input: &Value) -> Result<Event, Error>;

    /// Translate a clc response back into the agent's expected output format.
    /// Returns (`json_output`, `exit_code`).
    fn format_response(&self, event: &Event, response: &Response) -> (Option<Value>, i32);
}
