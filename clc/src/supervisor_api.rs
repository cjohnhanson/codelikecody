//! Supervisor API: mTLS-authenticated HTTP API for workspace communication.
//!
//! Every `clc` command inside a workspace hits this API instead of
//! opening SQLite directly. The supervisor validates operations,
//! enforces role-based access, and writes to the DB.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};

use clc_sdk::coordination::CoordinationBackend;
use clc_sdk::coordination_db::DbBackend;

/// Shared state for the API server.
pub struct ApiState {
    pub db: Arc<DbBackend>,
    pub project_dir: PathBuf,
}

/// Start the supervisor API server. Returns the bound address.
/// If `tls_config` is provided, the server uses mTLS. Otherwise plain HTTP.
pub async fn start(
    state: Arc<ApiState>,
    port: u16,
    tls_config: Option<Arc<rustls::ServerConfig>>,
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let app = Router::new()
        // Agents
        .route("/agents", get(list_agents).post(register_agent))
        .route("/agents/{id}", get(get_agent).patch(update_agent))
        // Messages
        .route(
            "/agents/{id}/messages",
            get(recv_messages).post(send_message),
        )
        // Permissions
        .route("/agents/{id}/permissions", get(pending_permissions))
        // Phase
        .route("/agents/{id}/phase", get(get_phase).put(set_phase))
        // Worker output (raw NDJSON — supervisor reads from workspace)
        .route("/agents/{id}/output", get(get_output))
        // Git pack for a branch (supervisor creates from local repo)
        .route("/git/pack/{branch}", get(get_git_pack))
        // Worker stdin (write a message to the worker's stdin pipe)
        .route("/agents/{id}/stdin", post(write_stdin))
        // Escalations
        .route("/escalations", get(list_escalations))
        // Health
        .route("/health", get(health))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    if let Some(tls) = tls_config {
        // mTLS server via axum-server + rustls.
        let rustls_config = axum_server::tls_rustls::RustlsConfig::from_config(tls);
        let bound_addr = addr;

        tokio::spawn(async move {
            axum_server::bind_rustls(addr, rustls_config)
                .serve(app.into_make_service())
                .await
                .ok();
        });

        Ok(bound_addr)
    } else {
        // Plain HTTP (for local worktree mode or testing).
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let bound_addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });

        Ok(bound_addr)
    }
}

// --- Request/Response types ---

#[derive(Deserialize)]
struct RegisterAgentRequest {
    id: String,
    parent_id: Option<String>,
}

#[derive(Deserialize)]
struct UpdateAgentRequest {
    status: Option<String>,
    pid: Option<i32>,
}

#[derive(Deserialize)]
struct SendMessageRequest {
    from: String,
    kind: String,
    payload: serde_json::Value,
}

#[derive(Deserialize)]
struct SetPhaseRequest {
    phase: String,
}

#[derive(Deserialize)]
struct ListAgentsQuery {
    parent_id: Option<String>,
}

#[derive(Deserialize)]
struct RecvMessagesQuery {
    after: Option<i64>,
}

#[derive(Serialize)]
struct AgentResponse {
    id: String,
    status: String,
    pid: Option<i32>,
}

#[derive(Serialize)]
struct MessageResponse {
    id: String,
    from: String,
    to: String,
    kind: String,
    payload: serde_json::Value,
}

#[derive(Serialize)]
struct CursorResponse<T> {
    data: Vec<T>,
    cursor: i64,
}

// --- Handlers ---

async fn health() -> &'static str {
    "ok"
}

async fn list_agents(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<ListAgentsQuery>,
) -> Result<Json<Vec<AgentResponse>>, StatusCode> {
    let agents = state
        .db
        .list_agents(query.parent_id.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response: Vec<AgentResponse> = agents
        .into_iter()
        .map(|(id, status)| AgentResponse {
            id,
            status: format!("{status:?}"),
            pid: None,
        })
        .collect();

    Ok(Json(response))
}

async fn register_agent(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<RegisterAgentRequest>,
) -> Result<StatusCode, StatusCode> {
    state
        .db
        .register_agent(&req.id, req.parent_id.as_deref())
        .await
        .map_err(|_| StatusCode::CONFLICT)?;

    Ok(StatusCode::CREATED)
}

async fn get_agent(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<Json<AgentResponse>, StatusCode> {
    let status = state
        .db
        .get_status(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let pid = state.db.get_pid(&id).await.ok().flatten();

    Ok(Json(AgentResponse {
        id,
        status: format!("{status:?}"),
        pid,
    }))
}

async fn update_agent(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAgentRequest>,
) -> Result<StatusCode, StatusCode> {
    if let Some(ref status_str) = req.status {
        let status = parse_status(status_str).ok_or(StatusCode::BAD_REQUEST)?;
        state
            .db
            .set_status(&id, status)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;
    }

    if let Some(pid) = req.pid {
        state
            .db
            .set_pid(&id, Some(pid))
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;
    }

    Ok(StatusCode::OK)
}

async fn send_message(
    State(state): State<Arc<ApiState>>,
    Path(to): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let kind = parse_message_kind(&req.kind, &req.payload).ok_or(StatusCode::BAD_REQUEST)?;

    let msg = clc_sdk::coordination::Message {
        id: format!(
            "api-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ),
        from: req.from,
        to,
        kind,
        timestamp: std::time::SystemTime::now(),
    };

    let msg_id = state
        .db
        .send(msg)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({ "id": msg_id })))
}

async fn recv_messages(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Query(query): Query<RecvMessagesQuery>,
) -> Result<Json<CursorResponse<MessageResponse>>, StatusCode> {
    let cursor = clc_sdk::coordination::Cursor(query.after.unwrap_or(0));

    let (messages, new_cursor) = state
        .db
        .recv(&id, &cursor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let data: Vec<MessageResponse> = messages
        .into_iter()
        .map(|m| MessageResponse {
            id: m.id,
            from: m.from,
            to: m.to,
            kind: kind_to_string(&m.kind),
            payload: kind_to_payload(&m.kind),
        })
        .collect();

    Ok(Json(CursorResponse {
        data,
        cursor: new_cursor.0,
    }))
}

async fn pending_permissions(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<MessageResponse>>, StatusCode> {
    let messages = state
        .db
        .pending_permissions(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let data: Vec<MessageResponse> = messages
        .into_iter()
        .map(|m| MessageResponse {
            id: m.id,
            from: m.from,
            to: m.to,
            kind: kind_to_string(&m.kind),
            payload: kind_to_payload(&m.kind),
        })
        .collect();

    Ok(Json(data))
}

async fn get_phase(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Phase is stored in the workspace's .clc/state file.
    // For SSH workspaces, the supervisor would need to SSH in to read it.
    // For now, check coordination DB for the latest StatusUpdate message.
    let (messages, _) = state
        .db
        .recv(&id, &clc_sdk::coordination::Cursor(0))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Find the latest StatusUpdate.
    let phase = messages.iter().rev().find_map(|m| {
        if let clc_sdk::coordination::MessageKind::StatusUpdate { ref phase, .. } = m.kind {
            Some(phase.clone())
        } else {
            None
        }
    });

    Ok(Json(
        serde_json::json!({ "agent_id": id, "phase": phase }),
    ))
}

async fn set_phase(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<SetPhaseRequest>,
) -> Result<StatusCode, StatusCode> {
    // Record as a StatusUpdate message.
    let msg = clc_sdk::coordination::Message {
        id: format!(
            "phase-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ),
        from: id.clone(),
        to: "supervisor".into(),
        kind: clc_sdk::coordination::MessageKind::StatusUpdate {
            phase: req.phase.clone(),
            detail: format!("set to {}", req.phase),
        },
        timestamp: std::time::SystemTime::now(),
    };

    state
        .db
        .send(msg)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

async fn list_escalations(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<Vec<MessageResponse>>, StatusCode> {
    let messages = state
        .db
        .pending_permissions("admin")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let data: Vec<MessageResponse> = messages
        .into_iter()
        .map(|m| MessageResponse {
            id: m.id,
            from: m.from,
            to: m.to,
            kind: kind_to_string(&m.kind),
            payload: kind_to_payload(&m.kind),
        })
        .collect();

    Ok(Json(data))
}

/// Get raw NDJSON output for a worker. Cursor-based via ?after= (line count).
/// The supervisor reads the output from the workspace — currently from the
/// local filesystem, but will be over SSH for remote workspaces.
async fn get_output(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Query(query): Query<RecvMessagesQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let cursor = query.after.unwrap_or(0) as usize;

    // Read stdout.jsonl from the workspace's worker dir.
    let stdout_path = state
        .project_dir
        .join(".worktrees")
        .join(&id)
        .join(".clc")
        .join("worker")
        .join("stdout.jsonl");

    let content = tokio::fs::read_to_string(&stdout_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let lines: Vec<&str> = content.lines().collect();
    let new_lines: Vec<&str> = lines.iter().skip(cursor).copied().collect();

    Ok(Json(serde_json::json!({
        "agent_id": id,
        "lines": new_lines,
        "cursor": lines.len(),
    })))
}

/// Serve a git pack for a branch. The pack contains all objects
/// reachable from the branch tip, plus the refs. Returned as JSON
/// with base64-encoded pack data.
async fn get_git_pack(
    State(state): State<Arc<ApiState>>,
    Path(branch): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pack_data = tokio::task::spawn_blocking({
        let project_dir = state.project_dir.clone();
        let branch = branch.clone();
        move || crate::git_pack::create_pack(&project_dir, &branch)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::NOT_FOUND)?;

    // Base64 encode the pack.
    let b64 = crate::ssh_workspace::base64_encode(&pack_data.pack);

    let refs: Vec<serde_json::Value> = pack_data
        .refs
        .iter()
        .map(|(oid, name)| serde_json::json!([oid, name]))
        .collect();

    Ok(Json(serde_json::json!({
        "pack": b64,
        "refs": refs,
        "branch": branch,
    })))
}

/// Write a message to a worker's stdin pipe.
async fn write_stdin(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    let message = req["message"]
        .as_str()
        .ok_or(StatusCode::BAD_REQUEST)?;

    let pipe_path = state
        .project_dir
        .join(".worktrees")
        .join(&id)
        .join(".clc")
        .join("worker")
        .join("stdin.pipe");

    // Write the message as stream-json input.
    let input = claude_code::protocol::InputMessage::user(message);
    let json = serde_json::to_string(&input)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tokio::fs::write(&pipe_path, format!("{json}\n"))
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(StatusCode::OK)
}

// --- Helpers ---

fn parse_status(s: &str) -> Option<clc_sdk::coordination::AgentStatus> {
    match s.to_lowercase().as_str() {
        "pending" => Some(clc_sdk::coordination::AgentStatus::Pending),
        "running" => Some(clc_sdk::coordination::AgentStatus::Running),
        "completed" => Some(clc_sdk::coordination::AgentStatus::Completed),
        "failed" => Some(clc_sdk::coordination::AgentStatus::Failed),
        "stopped" => Some(clc_sdk::coordination::AgentStatus::Stopped),
        _ => None,
    }
}

fn parse_message_kind(
    kind: &str,
    payload: &serde_json::Value,
) -> Option<clc_sdk::coordination::MessageKind> {
    match kind {
        "text" => Some(clc_sdk::coordination::MessageKind::Text(
            payload["text"].as_str().unwrap_or_default().to_string(),
        )),
        "output" => Some(clc_sdk::coordination::MessageKind::Output(
            payload["output"].as_str().unwrap_or_default().to_string(),
        )),
        "permission_request" => Some(clc_sdk::coordination::MessageKind::PermissionRequest {
            tool_name: payload["tool_name"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            reason: payload["reason"].as_str().unwrap_or_default().to_string(),
        }),
        "permission_grant" => Some(clc_sdk::coordination::MessageKind::PermissionGrant {
            request_id: payload["request_id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            scope: payload["scope"].as_str().unwrap_or_default().to_string(),
        }),
        "permission_denied" => Some(clc_sdk::coordination::MessageKind::PermissionDenied {
            request_id: payload["request_id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            reason: payload["reason"].as_str().unwrap_or_default().to_string(),
        }),
        "status_update" => Some(clc_sdk::coordination::MessageKind::StatusUpdate {
            phase: payload["phase"].as_str().unwrap_or_default().to_string(),
            detail: payload["detail"].as_str().unwrap_or_default().to_string(),
        }),
        _ => None,
    }
}

fn kind_to_string(kind: &clc_sdk::coordination::MessageKind) -> String {
    match kind {
        clc_sdk::coordination::MessageKind::Text(_) => "text",
        clc_sdk::coordination::MessageKind::Output(_) => "output",
        clc_sdk::coordination::MessageKind::PermissionRequest { .. } => "permission_request",
        clc_sdk::coordination::MessageKind::PermissionGrant { .. } => "permission_grant",
        clc_sdk::coordination::MessageKind::PermissionDenied { .. } => "permission_denied",
        clc_sdk::coordination::MessageKind::ReviewRequest { .. } => "review_request",
        clc_sdk::coordination::MessageKind::ReviewResult { .. } => "review_result",
        clc_sdk::coordination::MessageKind::StatusUpdate { .. } => "status_update",
    }
    .to_string()
}

fn kind_to_payload(kind: &clc_sdk::coordination::MessageKind) -> serde_json::Value {
    match kind {
        clc_sdk::coordination::MessageKind::Text(t) => serde_json::json!({ "text": t }),
        clc_sdk::coordination::MessageKind::Output(t) => serde_json::json!({ "output": t }),
        clc_sdk::coordination::MessageKind::PermissionRequest { tool_name, reason } => {
            serde_json::json!({ "tool_name": tool_name, "reason": reason })
        }
        clc_sdk::coordination::MessageKind::PermissionGrant { request_id, scope } => {
            serde_json::json!({ "request_id": request_id, "scope": scope })
        }
        clc_sdk::coordination::MessageKind::PermissionDenied { request_id, reason } => {
            serde_json::json!({ "request_id": request_id, "reason": reason })
        }
        clc_sdk::coordination::MessageKind::ReviewRequest { branch, summary } => {
            serde_json::json!({ "branch": branch, "summary": summary })
        }
        clc_sdk::coordination::MessageKind::ReviewResult {
            request_id,
            verdict,
            comments,
        } => serde_json::json!({
            "request_id": request_id,
            "verdict": format!("{verdict:?}"),
            "comments": comments
        }),
        clc_sdk::coordination::MessageKind::StatusUpdate { phase, detail } => {
            serde_json::json!({ "phase": phase, "detail": detail })
        }
    }
}
