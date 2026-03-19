use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use camino::Utf8Path;
use tisket::Repo;

use crate::error::ApiError;
use crate::types::{
    CreateIssueRequest, CreateIssueResponse, EditIssueRequest, ListIssuesParams, SearchParams,
    StatusOverride,
};
use crate::AppState;

fn open_repo(root: &Utf8Path) -> Result<Repo, ApiError> {
    Ok(Repo::open(root)?)
}

pub async fn health() -> &'static str {
    "ok"
}

pub async fn list_projects(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, ApiError> {
    let repo = open_repo(&state.root)?;
    let projects = repo.list_projects()?;
    let result: Vec<serde_json::Value> = projects
        .into_iter()
        .map(|name| serde_json::json!({ "name": name }))
        .collect();
    Ok(Json(result))
}

pub async fn list_issues(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListIssuesParams>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = open_repo(&state.root)?;
    let issues = repo.list_issues(
        params.project.as_deref(),
        params.status.as_deref(),
        params.label.as_deref(),
        params.closed,
    )?;
    Ok(Json(issues))
}

pub async fn get_issue(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = open_repo(&state.root)?;
    let issue = repo.find_issue(&id)?;
    Ok(Json(issue))
}

pub async fn create_issue(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateIssueRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = open_repo(&state.root)?;

    let status = req
        .status
        .as_deref()
        .map(|s| s.parse())
        .transpose()
        .map_err(|e: tisket::Error| ApiError::from(e))?;

    let opts = tisket::CreateIssueOptions {
        priority: req.priority,
        assignee: req.assignee,
        due_date: req.due_date,
        labels: req.labels.map(|v| v.join(", ")),
        depends_on: req.depends_on.map(|v| v.join(", ")),
        status,
        body: req.body,
    };

    let id = repo.create_issue(&req.title, &req.project, opts)?;
    Ok((StatusCode::CREATED, Json(CreateIssueResponse { id })))
}

pub async fn edit_issue(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<EditIssueRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = open_repo(&state.root)?;

    let labels_str = req.labels.as_ref().map(|v| v.join(", "));
    let depends_str = req.depends_on.as_ref().map(|v| v.join(", "));

    let opts = tisket::EditIssueOptions {
        status: req.status.as_deref(),
        assignee: req.assignee.as_deref(),
        due_date: req.due_date.as_deref(),
        title: req.title.as_deref(),
        priority: req.priority,
        labels: labels_str.as_deref(),
        add_label: req.add_label.as_deref(),
        remove_label: req.remove_label.as_deref(),
        depends_on: depends_str.as_deref(),
        body: req.body.as_deref(),
        append: req.append.as_deref(),
    };

    repo.edit_issue(&id, opts)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn close_issue(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: Option<Json<StatusOverride>>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = open_repo(&state.root)?;
    let status = body.and_then(|b| b.0.status);
    repo.close_issue(&id, status.as_deref())?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reopen_issue(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: Option<Json<StatusOverride>>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = open_repo(&state.root)?;
    let status = body.and_then(|b| b.0.status);
    repo.reopen_issue(&id, status.as_deref())?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = open_repo(&state.root)?;
    let results = repo.search(&params.q, params.project.as_deref())?;
    Ok(Json(results))
}
