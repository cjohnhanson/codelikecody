//! Types for Claude Code's `--output-format stream-json` NDJSON protocol.
//!
//! Output is typed NDJSON: one JSON object per line, discriminated by `"type"`.
//! All types use `#[serde(flatten)]` with catch-all maps so new fields from
//! future Claude Code versions get captured instead of breaking deserialization.

use serde::{Deserialize, Serialize};

/// A single line of NDJSON output from Claude Code in stream-json mode.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum OutputMessage {
    /// Session initialization — model, tools, session info.
    #[serde(rename = "system")]
    System(SystemMessage),

    /// Assistant turn content — thinking, text, `tool_use`.
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),

    /// Tool result fed back to the model.
    #[serde(rename = "user")]
    User(UserMessage),

    /// Final result with cost, duration, permission denials.
    #[serde(rename = "result")]
    Result(ResultMessage),
}

#[derive(Debug, Clone, Deserialize)]
pub struct SystemMessage {
    #[serde(default)]
    pub subtype: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssistantMessage {
    #[serde(default)]
    pub message: AssistantContent,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AssistantContent {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: serde_json::Value,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserMessage {
    #[serde(default)]
    pub message: serde_json::Value,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResultMessage {
    #[serde(default)]
    pub is_error: bool,
    #[serde(default)]
    pub cost_usd: f64,
    #[serde(default)]
    pub duration_secs: f64,
    #[serde(default)]
    pub duration_api_calls: Option<String>,
    #[serde(default)]
    pub permission_denials: Vec<PermissionDenialMsg>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PermissionDenialMsg {
    #[serde(default)]
    pub tool_name: String,
    #[serde(default)]
    pub message: String,
}

// --- Input types (coordinator → worker stdin) ---

/// Input message for sending follow-up prompts via stdin.
#[derive(Debug, Clone, Serialize)]
pub struct InputMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message: InputContent,
}

#[derive(Debug, Clone, Serialize)]
pub struct InputContent {
    pub role: String,
    pub content: String,
}

impl InputMessage {
    #[must_use]
    pub fn user(content: &str) -> Self {
        Self {
            msg_type: "user".into(),
            message: InputContent {
                role: "user".into(),
                content: content.into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_system_init() {
        let line =
            r#"{"type":"system","subtype":"init","session_id":"abc123","tools":[],"model":"opus"}"#;
        let msg: OutputMessage = serde_json::from_str(line).unwrap();
        match msg {
            OutputMessage::System(sys) => {
                assert_eq!(sys.subtype, "init");
                assert_eq!(sys.session_id.as_deref(), Some("abc123"));
                assert_eq!(sys.model.as_deref(), Some("opus"));
            }
            _ => panic!("expected System"),
        }
    }

    #[test]
    fn deserialize_result_message() {
        let line = r#"{"type":"result","is_error":false,"cost_usd":0.05,"duration_secs":12.3,"result":"done"}"#;
        let msg: OutputMessage = serde_json::from_str(line).unwrap();
        match msg {
            OutputMessage::Result(r) => {
                assert!(!r.is_error);
                assert!((r.cost_usd - 0.05).abs() < f64::EPSILON);
                assert_eq!(r.result.as_deref(), Some("done"));
                assert!(r.permission_denials.is_empty());
            }
            _ => panic!("expected Result"),
        }
    }

    #[test]
    fn deserialize_result_with_denials() {
        let line = r#"{"type":"result","is_error":false,"cost_usd":0.01,"duration_secs":1.0,"permission_denials":[{"tool_name":"Bash","message":"not allowed"}]}"#;
        let msg: OutputMessage = serde_json::from_str(line).unwrap();
        match msg {
            OutputMessage::Result(r) => {
                assert_eq!(r.permission_denials.len(), 1);
                assert_eq!(r.permission_denials[0].tool_name, "Bash");
            }
            _ => panic!("expected Result"),
        }
    }

    #[test]
    fn deserialize_assistant_text() {
        let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"hello"}]}}"#;
        let msg: OutputMessage = serde_json::from_str(line).unwrap();
        match msg {
            OutputMessage::Assistant(a) => {
                assert_eq!(a.message.role, "assistant");
                assert_eq!(a.message.content.len(), 1);
                match &a.message.content[0] {
                    ContentBlock::Text { text } => assert_eq!(text, "hello"),
                    _ => panic!("expected Text"),
                }
            }
            _ => panic!("expected Assistant"),
        }
    }

    #[test]
    fn deserialize_unknown_fields_preserved() {
        let line = r#"{"type":"result","is_error":false,"cost_usd":0.0,"duration_secs":0.0,"new_future_field":"surprise"}"#;
        let msg: OutputMessage = serde_json::from_str(line).unwrap();
        match msg {
            OutputMessage::Result(r) => {
                assert!(r.extra.contains_key("new_future_field"));
            }
            _ => panic!("expected Result"),
        }
    }

    #[test]
    fn serialize_input_message() {
        let msg = InputMessage::user("do the thing");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"user""#));
        assert!(json.contains(r#""role":"user""#));
        assert!(json.contains("do the thing"));
    }
}
