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
    /// Workflow definitions from the topology config. Used for phase
    /// transition validation without loading config files from disk.
    pub workflows: std::collections::HashMap<String, crate::config::WorkflowDef>,
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
        .route("/git/sync/{branch}", get(get_git_sync))
        // Worker stdin (write a message to the worker's stdin pipe)
        .route("/agents/{id}/stdin", post(write_stdin))
        // Fetch workspace pack for landing
        .route("/agents/{id}/pack", get(fetch_workspace_pack))
        // Tool check (PreToolUse hook calls this)
        .route("/agents/{id}/tool-check", post(tool_check))
        // Permission grants
        .route("/agents/{id}/grants", post(create_grant).get(list_grants))
        // Dispatch (coordinator requests supervisor to create a worker)
        .route("/dispatch", post(dispatch_worker))
        // Pickable tiskets (coordinator asks supervisor for fresh list)
        .route("/pickable", get(pickable_tiskets))
        // Escalations
        .route("/escalations", get(list_escalations))
        // Health
        .route("/health", get(health))
        .with_state(state);

    // Bind to all interfaces so Docker containers can reach the API.
    // mTLS ensures only clients with valid certs can connect.
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    if let Some(tls) = tls_config {
        // mTLS server via axum-server + rustls.
        let rustls_config = axum_server::tls_rustls::RustlsConfig::from_config(tls);
        let bound_addr = addr;

        tokio::spawn(async move {
            eprintln!("API server: starting TLS accept loop");
            match axum_server::bind_rustls(addr, rustls_config)
                .serve(app.into_make_service())
                .await
            {
                Ok(()) => eprintln!("API server: serve() returned Ok — server stopped"),
                Err(e) => eprintln!("API server: serve() returned Err — {e}"),
            }
            eprintln!("API server: accept loop exited!");
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
    #[serde(default)]
    workflow: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<String>,
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
            parent_id: None,
        })
        .collect();

    Ok(Json(response))
}

async fn register_agent(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<RegisterAgentRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state
        .db
        .register_agent(&req.id, req.parent_id.as_deref())
        .await
        .map_err(|_| StatusCode::CONFLICT)?;

    // Generate and store a bearer token for this agent.
    let token = crate::config::generate_token();
    state
        .db
        .set_token(&req.id, &token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({ "id": req.id, "token": token })))
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
    let parent_id = state.db.get_parent_id(&id).await.ok().flatten();

    Ok(Json(AgentResponse {
        id,
        status: format!("{status:?}"),
        pid,
        parent_id,
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
    headers: axum::http::HeaderMap,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let kind = parse_message_kind(&req.kind, &req.payload).ok_or(StatusCode::BAD_REQUEST)?;

    // For ReviewResult messages, validate that the caller's identity matches
    // the `from` field. This prevents workers from impersonating reviewers.
    if matches!(kind, clc_sdk::coordination::MessageKind::ReviewResult { .. }) {
        validate_sender_identity(&state, &headers, &req.from).await?;
    }

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

/// Validate that the bearer token in the request matches the claimed sender.
async fn validate_sender_identity(
    state: &ApiState,
    headers: &axum::http::HeaderMap,
    claimed_from: &str,
) -> Result<(), StatusCode> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let actual_agent_id = state
        .db
        .get_agent_id_by_token(token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if actual_agent_id != claimed_from {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(())
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

/// Dispatch a worker on behalf of a coordinator.
/// The coordinator (possibly in Docker) asks the supervisor to create
/// and start the workspace. Supervisor registers the worker, seeds
/// baseline tool grants, and sets the initial phase.
async fn dispatch_worker(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<DispatchRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    // Register or re-activate. If the agent exists from a prior run (status
    // Stopped/Failed), reset it to Pending instead of failing.
    if let Err(_) = state
        .db
        .register_agent(&req.tisket_id, Some(&req.coordinator_id))
        .await
    {
        state
            .db
            .set_status(
                &req.tisket_id,
                clc_sdk::coordination::AgentStatus::Pending,
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Seed baseline tool grants so the worker can function.
    for pattern in crate::config::BASELINE_TOOL_GRANTS {
        let _ = state
            .db
            .grant_permission(
                "dispatch",
                &req.tisket_id,
                pattern,
                "supervisor",
                "baseline grant at dispatch",
            )
            .await;
    }

    // Set initial phase from the workflow definition. The first phase in
    // the workflow is the starting point — no hardcoded phase names.
    let initial_phase = req.workflow.as_deref()
        .and_then(|name| state.workflows.get(name))
        .and_then(|def| crate::workflow::Workflow::new(def).ok())
        .map(|wf| wf.initial_phase().to_string())
        .unwrap_or_else(|| crate::workflow::Workflow::default_tdd().initial_phase().to_string());

    let _ = state
        .db
        .set_phase(&req.tisket_id, &initial_phase, 0, req.workflow.as_deref())
        .await;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "worker_id": req.tisket_id,
            "model": req.model,
            "coordinator_id": req.coordinator_id,
            "status": "pending",
        })),
    ))
}


#[derive(Deserialize)]
struct PickableQuery {
    label: Option<String>,
    exclude_label: Option<String>,
    project: Option<String>,
    coordinator_id: Option<String>,
}

/// Return pickable tisket IDs from the supervisor's always-current trunk.
/// Coordinators call this instead of reading stale tisket files locally.
async fn pickable_tiskets(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<PickableQuery>,
) -> Result<Json<Vec<String>>, StatusCode> {
    // Currently-active agents for this coordinator. Only Running/Pending
    // agents are excluded — Stopped/Completed/Failed agents from prior runs
    // must be eligible for re-dispatch.
    let dispatched: Vec<String> = if let Some(ref cid) = query.coordinator_id {
        state
            .db
            .list_agents(Some(cid))
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|(_, s)| matches!(s,
                clc_sdk::coordination::AgentStatus::Running
                | clc_sdk::coordination::AgentStatus::Pending
            ))
            .map(|(id, _)| id)
            .collect()
    } else {
        Vec::new()
    };

    // Tisket repo operations are synchronous — run on a blocking thread.
    let project_dir = state.project_dir.clone();
    let pickable = tokio::task::spawn_blocking(move || {
        let utf8_dir = camino::Utf8Path::new(
            project_dir
                .to_str()
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
        );

        let repo =
            tisket::Repo::open(utf8_dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let issues = repo
            .list_issues(query.project.as_deref(), None, None, false, &[])
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let result: Vec<String> = crate::coordinator_loop::filter_pickable_ids(
                &issues, &repo, query.label.as_deref(), query.exclude_label.as_deref(),
            )
            .into_iter()
            .filter(|id| !dispatched.contains(id))
            .collect();

        Ok::<_, StatusCode>(result)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|e| e)?;

    Ok(Json(pickable))
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

/// Check if a tool use is allowed for an agent.
/// Returns: { "allowed": true/false, "message": "..." }
/// If not allowed, auto-escalates to the coordinator.
async fn tool_check(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tool_name = req["tool_name"]
        .as_str()
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Check if this tool is already granted.
    let allowed = state
        .db
        .check_permission(&id, tool_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if allowed {
        return Ok(Json(serde_json::json!({
            "allowed": true,
        })));
    }

    // Not granted — escalate to the agent's parent (coordinator).
    // The agent's parent_id is set at registration time by the coordinator.
    let coordinator_id = state
        .db
        .get_parent_id(&id)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "coordinator".to_string());

    let reason = req["reason"]
        .as_str()
        .unwrap_or("tool check auto-escalation");

    let msg = clc_sdk::coordination::Message {
        id: format!(
            "tool-check-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ),
        from: id.clone(),
        to: coordinator_id,
        kind: clc_sdk::coordination::MessageKind::PermissionRequest {
            tool_name: tool_name.to_string(),
            reason: reason.to_string(),
        },
        timestamp: std::time::SystemTime::now(),
    };

    let _ = state.db.send(msg).await;

    Ok(Json(serde_json::json!({
        "allowed": false,
        "message": format!("Permission for '{tool_name}' escalated to coordinator. Retry after approval."),
    })))
}

#[derive(Deserialize)]
struct DispatchRequest {
    tisket_id: String,
    model: String,
    coordinator_id: String,
    #[serde(default)]
    workflow: Option<String>,
}

#[derive(Deserialize)]
struct CreateGrantRequest {
    tool_pattern: String,
    granted_by: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    session_id: String,
}

/// Store a permission grant for an agent.
async fn create_grant(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateGrantRequest>,
) -> Result<StatusCode, StatusCode> {
    state
        .db
        .grant_permission(
            &req.session_id,
            &id,
            &req.tool_pattern,
            &req.granted_by,
            &req.reason,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}

/// List permission grants for an agent.
async fn list_grants(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // For now, use the check_permission path to verify grants exist.
    // A full listing would need a new DB method.
    Ok(Json(serde_json::json!({
        "agent_id": id,
        "note": "use tool-check endpoint to verify specific permissions"
    })))
}

/// Get phase from the DB (not filesystem).
async fn get_phase(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let entry = state
        .db
        .get_phase(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some((phase, attempts, workflow)) = entry else {
        return Err(StatusCode::NOT_FOUND);
    };

    Ok(Json(serde_json::json!({
        "agent_id": id,
        "phase": phase,
        "attempts": attempts,
        "workflow": workflow,
    })))
}

/// Set phase in the DB with server-side transition validation.
/// First write (no existing entry) is always accepted — this is the
/// initial phase set by dispatch or init_phase_via_api. Subsequent
/// writes are validated against the workflow graph.
async fn set_phase(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<SetPhaseRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let existing = state
        .db
        .get_phase(&id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "db"}))))?;

    // Validate transitions for existing entries only.
    if let Some((current_phase, _, workflow_name)) = &existing {
        if current_phase != &req.phase {
            let wf = workflow_name
                .as_deref()
                .and_then(|name| state.workflows.get(name))
                .and_then(|def| crate::workflow::Workflow::new(def).ok())
                .unwrap_or_else(crate::workflow::Workflow::default_tdd);

            if !wf.has_phase(&req.phase) {
                return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
                    "error": format!("unknown phase '{}'", req.phase),
                }))));
            }
            if !wf.is_valid_transition(current_phase, &req.phase) {
                return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
                    "error": format!("invalid transition from '{}' to '{}'", current_phase, req.phase),
                }))));
            }

            // Review gate: check if this transition requires reviewer approval.
            if let Some(reviewers) = wf.transition_reviewers(current_phase, &req.phase) {
                // Check coordination DB for approval messages.
                let msgs = state
                    .db
                    .recv(&id, &clc_sdk::coordination::Cursor::default())
                    .await
                    .map(|(m, _)| m)
                    .unwrap_or_default();

                let missing: Vec<&str> = reviewers
                    .iter()
                    .filter(|reviewer_name| {
                        !msgs.iter().any(|m| matches!(
                            &m.kind,
                            clc_sdk::coordination::MessageKind::ReviewResult {
                                review_type,
                                verdict: clc_sdk::coordination::ReviewVerdict::Approved,
                                ..
                            } if review_type == *reviewer_name
                        ))
                    })
                    .map(|s| s.as_str())
                    .collect();

                if !missing.is_empty() {
                    return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
                        "error": format!(
                            "review required: {}. Reviewers will be dispatched — wait for approval, then retry.",
                            missing.join(", ")
                        ),
                    }))));
                }
            }
        }
    }

    let wf_to_store = req.workflow.as_deref()
        .or(existing.as_ref().and_then(|(_, _, wf)| wf.as_deref()));

    state
        .db
        .set_phase(&id, &req.phase, 0, wf_to_store)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "db"}))))?;

    Ok(StatusCode::OK)
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

/// Incremental git sync: only objects new since the client's HEAD.
async fn get_git_sync(
    State(state): State<Arc<ApiState>>,
    Path(branch): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let have = query.get("have").cloned().unwrap_or_default();
    let project_dir = state.project_dir.clone();
    let result = tokio::task::spawn_blocking({
        let branch = branch.clone();
        move || crate::git_pack::create_incremental_pack(&project_dir, &branch, &have)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match result {
        None => Ok(Json(serde_json::json!({"up_to_date": true, "branch": branch}))),
        Some(pack_data) => {
            let b64 = crate::ssh_workspace::base64_encode(&pack_data.pack);
            let refs: Vec<serde_json::Value> = pack_data.refs.iter()
                .map(|(oid, name)| serde_json::json!([oid, name])).collect();
            Ok(Json(serde_json::json!({"up_to_date": false, "pack": b64, "refs": refs, "branch": branch})))
        }
    }
}

/// Fetch a git pack from a workspace for landing.
/// The supervisor SSH's into the workspace and runs
/// `clc workspace export --branch <name>` which creates a pack
/// of the worker's commits.
async fn fetch_workspace_pack(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // For now, create pack from local worktree (same machine Docker).
    // For remote workspaces, this would SSH in and run clc workspace export.
    let worktree = state
        .project_dir
        .join(".worktrees")
        .join(&id);

    let pack_data = tokio::task::spawn_blocking({
        let worktree = worktree.clone();
        let branch = id.clone();
        move || crate::git_pack::create_pack(&worktree, &branch)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::NOT_FOUND)?;

    let b64 = crate::ssh_workspace::base64_encode(&pack_data.pack);
    let refs: Vec<serde_json::Value> = pack_data
        .refs
        .iter()
        .map(|(oid, name)| serde_json::json!([oid, name]))
        .collect();

    Ok(Json(serde_json::json!({
        "pack": b64,
        "refs": refs,
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
        clc_sdk::coordination::MessageKind::ReviewRequest { review_type, branch, summary } => {
            serde_json::json!({ "review_type": review_type, "branch": branch, "summary": summary })
        }
        clc_sdk::coordination::MessageKind::ReviewResult {
            request_id,
            review_type,
            verdict,
            comments,
        } => serde_json::json!({
            "request_id": request_id,
            "review_type": review_type,
            "verdict": format!("{verdict:?}"),
            "comments": comments
        }),
        clc_sdk::coordination::MessageKind::StatusUpdate { phase, detail } => {
            serde_json::json!({ "phase": phase, "detail": detail })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Start a plain-HTTP test API server backed by in-memory SQLite.
    fn start_test_api() -> (String, std::thread::JoinHandle<()>) {
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let db = clc_sdk::coordination_db::DbBackend::connect("sqlite::memory:")
                    .await
                    .unwrap();
                db.create_tables().await.unwrap();
                let state = Arc::new(ApiState {
                    db: Arc::new(db),
                    project_dir: std::path::PathBuf::from("/tmp"),
                    workflows: Default::default(),
                });
                let addr = start(state, 0, None).await.unwrap();
                tx.send(addr.port()).unwrap();
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                }
            });
        });
        let port = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("API server did not start");
        std::thread::sleep(std::time::Duration::from_millis(50));
        (format!("http://127.0.0.1:{port}"), handle)
    }

    fn blocking_post(base_url: &str, path: &str, body: &serde_json::Value) -> (u16, serde_json::Value) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let resp = reqwest::Client::new()
                .post(format!("{base_url}{path}"))
                .json(body)
                .send()
                .await
                .unwrap();
            let status = resp.status().as_u16();
            let body = resp.json().await.unwrap_or(serde_json::json!({}));
            (status, body)
        })
    }

    #[test]
    fn dispatch_endpoint_creates_agent_and_returns_id() {
        let (base_url, _handle) = start_test_api();

        // Register a coordinator first.
        blocking_post(
            &base_url,
            "/agents",
            &serde_json::json!({ "id": "coord-test" }),
        );

        let (status, body) = blocking_post(
            &base_url,
            "/dispatch",
            &serde_json::json!({
                "tisket_id": "feat-123",
                "model": "haiku",
                "coordinator_id": "coord-test"
            }),
        );

        assert_eq!(status, 201, "dispatch should return 201 Created");
        assert_eq!(body["worker_id"].as_str(), Some("feat-123"));
    }

    #[test]
    fn dispatch_endpoint_registers_worker_as_pending() {
        let (base_url, _handle) = start_test_api();

        blocking_post(
            &base_url,
            "/agents",
            &serde_json::json!({ "id": "coord-test" }),
        );

        blocking_post(
            &base_url,
            "/dispatch",
            &serde_json::json!({
                "tisket_id": "feat-456",
                "model": "opus",
                "coordinator_id": "coord-test"
            }),
        );

        // Worker should be registered in the DB as pending.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let body: serde_json::Value = rt.block_on(async {
            reqwest::Client::new()
                .get(format!("{base_url}/agents/feat-456"))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap()
        });

        assert_eq!(body["id"].as_str(), Some("feat-456"));
        assert_eq!(body["status"].as_str(), Some("Pending"));
        assert_eq!(body["parent_id"].as_str(), Some("coord-test"));
    }

    #[test]
    fn dispatch_seeds_baseline_grants() {
        let (base_url, _handle) = start_test_api();

        blocking_post(
            &base_url,
            "/agents",
            &serde_json::json!({ "id": "coord-test" }),
        );

        blocking_post(
            &base_url,
            "/dispatch",
            &serde_json::json!({
                "tisket_id": "grant-test",
                "model": "opus",
                "coordinator_id": "coord-test"
            }),
        );

        // Verify Read is granted via tool-check.
        let (status, body) = blocking_post(
            &base_url,
            "/agents/grant-test/tool-check",
            &serde_json::json!({ "tool_name": "Read" }),
        );
        assert_eq!(status, 200);
        assert_eq!(body["allowed"].as_bool(), Some(true), "Read should be granted: {body}");
    }

    #[test]
    fn dispatch_sets_initial_phase() {
        let (base_url, _handle) = start_test_api();

        blocking_post(
            &base_url,
            "/agents",
            &serde_json::json!({ "id": "coord-test" }),
        );

        blocking_post(
            &base_url,
            "/dispatch",
            &serde_json::json!({
                "tisket_id": "phase-test",
                "model": "opus",
                "coordinator_id": "coord-test"
            }),
        );

        // Verify phase was set.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let body: serde_json::Value = rt.block_on(async {
            reqwest::Client::new()
                .get(format!("{base_url}/agents/phase-test/phase"))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap()
        });

        assert_eq!(body["phase"].as_str(), Some("tests-unwritten"));
    }

    // --- Helpers for workflow-aware tests ---

    fn start_test_api_with_workflows(
        workflows: std::collections::HashMap<String, crate::config::WorkflowDef>,
    ) -> (String, Arc<DbBackend>, std::thread::JoinHandle<()>, tempfile::TempDir) {
        let tmp_dir = tempfile::tempdir().unwrap();
        let db_path = tmp_dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
        let db_url2 = db_url.clone();

        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let db = clc_sdk::coordination_db::DbBackend::connect(&db_url)
                    .await
                    .unwrap();
                db.create_tables().await.unwrap();
                let state = Arc::new(ApiState {
                    db: Arc::new(db),
                    project_dir: std::path::PathBuf::from("/tmp"),
                    workflows,
                });
                let addr = start(state, 0, None).await.unwrap();
                tx.send(addr.port()).unwrap();
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                }
            });
        });
        let port = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("API server did not start");
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Open a second connection to the same file-based DB for test helpers.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let db = rt.block_on(async {
            Arc::new(clc_sdk::coordination_db::DbBackend::connect(&db_url2).await.unwrap())
        });

        (format!("http://127.0.0.1:{port}"), db, handle, tmp_dir)
    }

    fn blocking_put(base_url: &str, path: &str, body: &serde_json::Value) -> (u16, serde_json::Value) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let resp = reqwest::Client::new()
                .put(format!("{base_url}{path}"))
                .json(body)
                .send()
                .await
                .unwrap();
            let status = resp.status().as_u16();
            let body = resp.json().await.unwrap_or(serde_json::json!({}));
            (status, body)
        })
    }

    fn blocking_get(base_url: &str, path: &str) -> (u16, serde_json::Value) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let resp = reqwest::Client::new()
                .get(format!("{base_url}{path}"))
                .send()
                .await
                .unwrap();
            let status = resp.status().as_u16();
            let body = resp.json().await.unwrap_or(serde_json::json!({}));
            (status, body)
        })
    }

    fn docs_workflow_def() -> crate::config::WorkflowDef {
        use crate::config::{PhaseDef, TransitionDef};
        crate::config::WorkflowDef {
            description: Some("Docs workflow for testing".into()),
            phases: vec![
                PhaseDef {
                    name: "outline".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false,
                    permissions: None,
                    transitions: Some(vec![TransitionDef::Rich {
                        target: "draft".into(),
                        review: vec!["docs-review".into()],
                    }]),
                },
                PhaseDef {
                    name: "draft".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false,
                    permissions: None,
                    transitions: Some(vec![
                        TransitionDef::Simple("outline".into()),
                        TransitionDef::Simple("done".into()),
                    ]),
                },
                PhaseDef {
                    name: "done".into(),
                    instructions: None,
                    nudge: None,
                    can_stop: false,
                    permissions: None,
                    transitions: None,
                },
            ],
        }
    }

    fn send_approval(db: &Arc<DbBackend>, worker_id: &str, reviewer_name: &str) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            use clc_sdk::coordination::CoordinationBackend;
            db.send(clc_sdk::coordination::Message {
                id: format!("test-approval-{reviewer_name}-{}", std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
                from: format!("{worker_id}-reviewer-{reviewer_name}"),
                to: worker_id.to_string(),
                kind: clc_sdk::coordination::MessageKind::ReviewResult {
                    request_id: String::new(),
                    review_type: reviewer_name.to_string(),
                    verdict: clc_sdk::coordination::ReviewVerdict::Approved,
                    comments: "approved in test".to_string(),
                },
                timestamp: std::time::SystemTime::now(),
            }).await.unwrap();
        });
    }

    fn dispatch_with_workflow(base_url: &str, tisket_id: &str, workflow: &str) {
        blocking_post(
            base_url,
            "/agents",
            &serde_json::json!({ "id": "coord-test" }),
        );
        blocking_post(
            base_url,
            "/dispatch",
            &serde_json::json!({
                "tisket_id": tisket_id,
                "model": "opus",
                "coordinator_id": "coord-test",
                "workflow": workflow,
            }),
        );
    }

    // --- Phase transition validation tests ---

    #[test]
    fn api_set_phase_rejects_invalid_transition() {
        let workflows = std::collections::HashMap::from([
            ("docs".into(), docs_workflow_def()),
        ]);
        let (base_url, _db, _handle, _tmp) = start_test_api_with_workflows(workflows);
        dispatch_with_workflow(&base_url, "worker-1", "docs");

        // outline → done is not a valid transition (must go through draft)
        let (status, body) = blocking_put(
            &base_url,
            "/agents/worker-1/phase",
            &serde_json::json!({ "phase": "done" }),
        );
        assert_eq!(status, 400, "skip-forward should be rejected: {body}");
        assert!(
            body["error"].as_str().unwrap_or("").contains("invalid transition"),
            "error should mention invalid transition: {body}"
        );
    }

    #[test]
    fn api_set_phase_rejects_unknown_phase() {
        let workflows = std::collections::HashMap::from([
            ("docs".into(), docs_workflow_def()),
        ]);
        let (base_url, _db, _handle, _tmp) = start_test_api_with_workflows(workflows);
        dispatch_with_workflow(&base_url, "worker-1", "docs");

        let (status, body) = blocking_put(
            &base_url,
            "/agents/worker-1/phase",
            &serde_json::json!({ "phase": "nonexistent" }),
        );
        assert_eq!(status, 400, "unknown phase should be rejected: {body}");
        assert!(
            body["error"].as_str().unwrap_or("").contains("unknown phase"),
            "error should mention unknown phase: {body}"
        );
    }

    #[test]
    fn api_set_phase_blocks_review_gated_transition() {
        let workflows = std::collections::HashMap::from([
            ("docs".into(), docs_workflow_def()),
        ]);
        let (base_url, _db, _handle, _tmp) = start_test_api_with_workflows(workflows);
        dispatch_with_workflow(&base_url, "worker-1", "docs");

        // outline → draft requires docs-review approval
        let (status, body) = blocking_put(
            &base_url,
            "/agents/worker-1/phase",
            &serde_json::json!({ "phase": "draft" }),
        );
        assert_eq!(status, 400, "review-gated transition should be rejected without approval: {body}");
        assert!(
            body["error"].as_str().unwrap_or("").contains("review required"),
            "error should mention review required: {body}"
        );
    }

    #[test]
    fn api_set_phase_allows_review_gated_transition_after_approval() {
        let workflows = std::collections::HashMap::from([
            ("docs".into(), docs_workflow_def()),
        ]);
        let (base_url, db, _handle, _tmp) = start_test_api_with_workflows(workflows);
        dispatch_with_workflow(&base_url, "worker-1", "docs");

        // Send an approval for docs-review
        send_approval(&db, "worker-1", "docs-review");

        // Now the transition should succeed
        let (status, _body) = blocking_put(
            &base_url,
            "/agents/worker-1/phase",
            &serde_json::json!({ "phase": "draft" }),
        );
        assert_eq!(status, 200, "review-gated transition should succeed after approval");

        // Verify the phase was actually set
        let (_, phase_body) = blocking_get(&base_url, "/agents/worker-1/phase");
        assert_eq!(phase_body["phase"].as_str(), Some("draft"));
    }

    #[test]
    fn api_set_phase_allows_valid_forward_transitions() {
        let workflows = std::collections::HashMap::from([
            ("docs".into(), docs_workflow_def()),
        ]);
        let (base_url, db, _handle, _tmp) = start_test_api_with_workflows(workflows);
        dispatch_with_workflow(&base_url, "worker-1", "docs");

        // Send approval for the review gate
        send_approval(&db, "worker-1", "docs-review");

        // outline → draft (with approval)
        let (status, _) = blocking_put(
            &base_url,
            "/agents/worker-1/phase",
            &serde_json::json!({ "phase": "draft" }),
        );
        assert_eq!(status, 200, "outline → draft should succeed");

        // draft → done (no review gate)
        let (status, _) = blocking_put(
            &base_url,
            "/agents/worker-1/phase",
            &serde_json::json!({ "phase": "done" }),
        );
        assert_eq!(status, 200, "draft → done should succeed");
    }

    #[test]
    fn api_set_phase_allows_backward_transition() {
        let workflows = std::collections::HashMap::from([
            ("docs".into(), docs_workflow_def()),
        ]);
        let (base_url, db, _handle, _tmp) = start_test_api_with_workflows(workflows);
        dispatch_with_workflow(&base_url, "worker-1", "docs");

        // Advance to draft (with approval)
        send_approval(&db, "worker-1", "docs-review");
        blocking_put(
            &base_url,
            "/agents/worker-1/phase",
            &serde_json::json!({ "phase": "draft" }),
        );

        // draft → outline (backward, should be allowed)
        let (status, _) = blocking_put(
            &base_url,
            "/agents/worker-1/phase",
            &serde_json::json!({ "phase": "outline" }),
        );
        assert_eq!(status, 200, "backward transition draft → outline should succeed");
    }

    #[test]
    fn dispatch_with_workflow_sets_correct_initial_phase() {
        let workflows = std::collections::HashMap::from([
            ("docs".into(), docs_workflow_def()),
        ]);
        let (base_url, _db, _handle, _tmp) = start_test_api_with_workflows(workflows);
        dispatch_with_workflow(&base_url, "worker-1", "docs");

        let (_, body) = blocking_get(&base_url, "/agents/worker-1/phase");
        assert_eq!(
            body["phase"].as_str(),
            Some("outline"),
            "docs workflow should start at 'outline', not 'tests-unwritten': {body}"
        );
    }
}
