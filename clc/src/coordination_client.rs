//! HTTP client for the supervisor API.
//!
//! When `CLC_API_URL` is set, coordination operations go through HTTP
//! to the supervisor API instead of opening SQLite directly. This is
//! the client side of the workspace communication architecture.

use clc_sdk::coordination::{
    AgentId, AgentStatus, CoordinationError, Cursor, Message, MessageId, MessageKind,
};

/// HTTP-backed coordination client. Supports mTLS when cert env vars are set.
pub struct ApiClient {
    base_url: String,
    client: reqwest::Client,
    rt: tokio::runtime::Runtime,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Result<Self, CoordinationError> {
        // Multi-thread runtime so reqwest's connection pool background tasks
        // (HTTP/2 keep-alive, flow control) run between block_on() calls.
        // A current_thread runtime only polls during block_on(), causing
        // connections to go stale between sequential API calls.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|e| CoordinationError::Storage(format!("tokio: {e}")))?;

        // Build reqwest client with mTLS if cert env vars are set.
        let client = build_api_client()
            .map_err(|e| CoordinationError::Storage(format!("http client: {e:#}")))?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
            rt,
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    pub fn register_agent(
        &self,
        id: &str,
        parent_id: Option<&str>,
    ) -> Result<(), CoordinationError> {
        let body = serde_json::json!({
            "id": id,
            "parent_id": parent_id,
        });

        let status = self.rt.block_on(async {
            let resp = self.client.clone()
                .post(self.url("/agents"))
                .json(&body)
                .send()
                .await
                .map_err(|e| CoordinationError::Storage(format!("http: {e}")))?;
            Ok::<_, CoordinationError>(resp.status())
        })?;

        if status.is_success() || status.as_u16() == 409 {
            Ok(())
        } else {
            Err(CoordinationError::Storage(format!(
                "register_agent: HTTP {status}"
            )))
        }
    }

    /// Register an agent and return the bearer token for API authentication.
    pub fn register_agent_with_token(
        &self,
        id: &str,
        parent_id: Option<&str>,
    ) -> Result<String, CoordinationError> {
        let body = serde_json::json!({
            "id": id,
            "parent_id": parent_id,
        });

        let resp: serde_json::Value = self.rt.block_on(async {
            let resp = self.client.clone()
                .post(self.url("/agents"))
                .json(&body)
                .send()
                .await
                .map_err(|e| CoordinationError::Storage(format!("http: {e}")))?;

            if !resp.status().is_success() {
                return Err(CoordinationError::Storage(format!(
                    "register_agent: HTTP {}",
                    resp.status()
                )));
            }

            resp.json()
                .await
                .map_err(|e| CoordinationError::Storage(format!("parse response: {e}")))
        })?;

        resp["token"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| CoordinationError::Storage("no token in registration response".into()))
    }

    pub fn set_status(
        &self,
        agent_id: &str,
        status: AgentStatus,
    ) -> Result<(), CoordinationError> {
        let status_str = match status {
            AgentStatus::Pending => "pending",
            AgentStatus::Running => "running",
            AgentStatus::Completed => "completed",
            AgentStatus::Failed => "failed",
            AgentStatus::Stopped => "stopped",
        };

        let body = serde_json::json!({ "status": status_str });

        let resp_status = self.rt.block_on(async {
            let resp = self.client.clone()
                .patch(self.url(&format!("/agents/{agent_id}")))
                .json(&body)
                .send()
                .await
                .map_err(|e| CoordinationError::Storage(format!("http: {e}")))?;
            Ok::<_, CoordinationError>(resp.status())
        })?;

        if resp_status.is_success() {
            Ok(())
        } else {
            Err(CoordinationError::NotFound(agent_id.to_string()))
        }
    }

    pub fn get_status(
        &self,
        agent_id: &str,
    ) -> Result<AgentStatus, CoordinationError> {
        let body: serde_json::Value = self.rt.block_on(async {
            let resp = self.client.clone()
                .get(self.url(&format!("/agents/{agent_id}")))
                .send()
                .await
                .map_err(|e| CoordinationError::Storage(format!("http: {e}")))?;

            if resp.status().as_u16() == 404 {
                return Err(CoordinationError::NotFound(agent_id.to_string()));
            }

            resp.json()
                .await
                .map_err(|e| CoordinationError::Storage(format!("json: {e}")))
        })?;

        let status_str = body["status"].as_str().unwrap_or("pending");
        match status_str {
            "Pending" | "pending" => Ok(AgentStatus::Pending),
            "Running" | "running" => Ok(AgentStatus::Running),
            "Completed" | "completed" => Ok(AgentStatus::Completed),
            "Failed" | "failed" => Ok(AgentStatus::Failed),
            "Stopped" | "stopped" => Ok(AgentStatus::Stopped),
            other => Err(CoordinationError::Storage(format!(
                "unknown status: {other}"
            ))),
        }
    }

    pub fn send(&self, msg: Message) -> Result<MessageId, CoordinationError> {
        let (kind_str, payload) = kind_to_json(&msg.kind);

        let body = serde_json::json!({
            "from": msg.from,
            "kind": kind_str,
            "payload": payload,
        });

        let resp_body: serde_json::Value = self.rt.block_on(async {
            let resp = self.client.clone()
                .post(self.url(&format!("/agents/{}/messages", msg.to)))
                .json(&body)
                .send()
                .await
                .map_err(|e| CoordinationError::Storage(format!("http: {e}")))?;

            resp.json()
                .await
                .map_err(|e| CoordinationError::Storage(format!("json: {e}")))
        })?;

        Ok(resp_body["id"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    pub fn recv(
        &self,
        agent_id: &str,
        cursor: &Cursor,
    ) -> Result<(Vec<Message>, Cursor), CoordinationError> {
        let body: serde_json::Value = self.rt.block_on(async {
            let resp = self.client.clone()
                .get(self.url(&format!(
                    "/agents/{agent_id}/messages?after={}",
                    cursor.0
                )))
                .send()
                .await
                .map_err(|e| CoordinationError::Storage(format!("http: {e}")))?;

            resp.json()
                .await
                .map_err(|e| CoordinationError::Storage(format!("json: {e}")))
        })?;

        let new_cursor = Cursor(body["cursor"].as_i64().unwrap_or(cursor.0));
        let messages: Vec<Message> = body["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| json_to_message(m))
                    .collect()
            })
            .unwrap_or_default();

        Ok((messages, new_cursor))
    }

    pub fn pending_permissions(
        &self,
        grantor_id: &str,
    ) -> Result<Vec<Message>, CoordinationError> {
        let body: serde_json::Value = self.rt.block_on(async {
            let resp = self.client.clone()
                .get(self.url(&format!("/agents/{grantor_id}/permissions")))
                .send()
                .await
                .map_err(|e| CoordinationError::Storage(format!("http: {e}")))?;

            resp.json()
                .await
                .map_err(|e| CoordinationError::Storage(format!("json: {e}")))
        })?;

        let messages: Vec<Message> = body
            .as_array()
            .map(|arr| arr.iter().filter_map(json_to_message).collect())
            .unwrap_or_default();

        Ok(messages)
    }

    pub fn list_agents(
        &self,
        parent_id: Option<&str>,
    ) -> Result<Vec<(AgentId, AgentStatus)>, CoordinationError> {
        let url = if let Some(pid) = parent_id {
            self.url(&format!("/agents?parent_id={pid}"))
        } else {
            self.url("/agents")
        };

        let body: serde_json::Value = self.rt.block_on(async {
            let resp = self.client.clone()
                .get(&url)
                .send()
                .await
                .map_err(|e| CoordinationError::Storage(format!("http: {e}")))?;

            resp.json()
                .await
                .map_err(|e| CoordinationError::Storage(format!("json: {e}")))
        })?;

        let agents: Vec<(AgentId, AgentStatus)> = body
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| {
                        let id = a["id"].as_str()?.to_string();
                        let status = match a["status"].as_str()? {
                            "Pending" | "pending" => AgentStatus::Pending,
                            "Running" | "running" => AgentStatus::Running,
                            "Completed" | "completed" => AgentStatus::Completed,
                            "Failed" | "failed" => AgentStatus::Failed,
                            "Stopped" | "stopped" => AgentStatus::Stopped,
                            _ => return None,
                        };
                        Some((id, status))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(agents)
    }
}

fn kind_to_json(kind: &MessageKind) -> (&'static str, serde_json::Value) {
    match kind {
        MessageKind::Text(t) => ("text", serde_json::json!({ "text": t })),
        MessageKind::Output(t) => ("output", serde_json::json!({ "output": t })),
        MessageKind::PermissionRequest { tool_name, reason } => (
            "permission_request",
            serde_json::json!({ "tool_name": tool_name, "reason": reason }),
        ),
        MessageKind::PermissionGrant { request_id, scope } => (
            "permission_grant",
            serde_json::json!({ "request_id": request_id, "scope": scope }),
        ),
        MessageKind::PermissionDenied { request_id, reason } => (
            "permission_denied",
            serde_json::json!({ "request_id": request_id, "reason": reason }),
        ),
        MessageKind::ReviewRequest { review_type, branch, summary } => (
            "review_request",
            serde_json::json!({ "review_type": review_type, "branch": branch, "summary": summary }),
        ),
        MessageKind::ReviewResult {
            request_id,
            review_type,
            verdict,
            comments,
        } => (
            "review_result",
            serde_json::json!({
                "request_id": request_id,
                "review_type": review_type,
                "verdict": format!("{verdict:?}"),
                "comments": comments
            }),
        ),
        MessageKind::StatusUpdate { phase, detail } => (
            "status_update",
            serde_json::json!({ "phase": phase, "detail": detail }),
        ),
    }
}

fn json_to_message(v: &serde_json::Value) -> Option<Message> {
    let id = v["id"].as_str()?.to_string();
    let from = v["from"].as_str()?.to_string();
    let to = v["to"].as_str()?.to_string();
    let kind_str = v["kind"].as_str()?;
    let payload = &v["payload"];

    let kind = match kind_str {
        "text" => MessageKind::Text(payload["text"].as_str().unwrap_or_default().to_string()),
        "output" => {
            MessageKind::Output(payload["output"].as_str().unwrap_or_default().to_string())
        }
        "permission_request" => MessageKind::PermissionRequest {
            tool_name: payload["tool_name"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            reason: payload["reason"].as_str().unwrap_or_default().to_string(),
        },
        "permission_grant" => MessageKind::PermissionGrant {
            request_id: payload["request_id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            scope: payload["scope"].as_str().unwrap_or_default().to_string(),
        },
        "permission_denied" => MessageKind::PermissionDenied {
            request_id: payload["request_id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            reason: payload["reason"].as_str().unwrap_or_default().to_string(),
        },
        "status_update" => MessageKind::StatusUpdate {
            phase: payload["phase"].as_str().unwrap_or_default().to_string(),
            detail: payload["detail"].as_str().unwrap_or_default().to_string(),
        },
        _ => return None,
    };

    Some(Message {
        id,
        from,
        to,
        kind,
        timestamp: std::time::SystemTime::now(),
    })
}

/// Build a reqwest client with mTLS if CLC_API_CERT, CLC_API_KEY, and
/// CLC_API_CA env vars are set. Otherwise returns a plain client.
pub fn build_api_client() -> Result<reqwest::Client, Box<dyn std::error::Error>> {
    let cert_path = std::env::var("CLC_API_CERT").ok();
    let key_path = std::env::var("CLC_API_KEY").ok();
    let ca_path = std::env::var("CLC_API_CA").ok();

    // Support both file paths and inline PEM content.
    // If the value starts with "-----BEGIN", treat it as inline PEM.
    let read_pem = |val: &str, label: &str| -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if val.starts_with("-----BEGIN") {
            Ok(val.as_bytes().to_vec())
        } else {
            std::fs::read(val).map_err(|e| format!("read {label} {val}: {e}").into())
        }
    };

    // If CLC_AGENT_TOKEN is set, include it as a default Authorization header.
    let mut default_headers = reqwest::header::HeaderMap::new();
    if let Ok(token) = std::env::var("CLC_AGENT_TOKEN") {
        if let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}")) {
            default_headers.insert(reqwest::header::AUTHORIZATION, val);
        }
    }

    match (cert_path, key_path, ca_path) {
        (Some(cert), Some(key), Some(ca)) => {
            let cert_pem = read_pem(&cert, "cert")?;
            let key_pem = read_pem(&key, "key")?;
            let ca_pem = read_pem(&ca, "CA")?;

            let identity = reqwest::Identity::from_pem(&[cert_pem, key_pem].concat())
                .map_err(|e| format!("parse identity: {e}"))?;
            let ca_cert = reqwest::Certificate::from_pem(&ca_pem)
                .map_err(|e| format!("parse CA cert: {e}"))?;

            reqwest::Client::builder()
                .default_headers(default_headers)
                .identity(identity)
                .add_root_certificate(ca_cert)
                .danger_accept_invalid_certs(false)
                .build()
                .map_err(|e| format!("build client: {e}").into())
        }
        _ => reqwest::Client::builder()
            .default_headers(default_headers)
            .build()
            .map_err(|e| format!("build client: {e}").into()),
    }
}
