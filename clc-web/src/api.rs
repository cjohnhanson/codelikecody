use gloo_net::http::Request;

use crate::types::{CreateIssueRequest, CreateIssueResponse, EditIssueRequest, Issue, Project, SearchResult};

#[derive(Debug)]
pub struct ApiError(pub String);

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<gloo_net::Error> for ApiError {
    fn from(err: gloo_net::Error) -> Self {
        Self(err.to_string())
    }
}

type Result<T> = std::result::Result<T, ApiError>;

pub async fn list_projects() -> Result<Vec<Project>> {
    let resp = Request::get("/api/projects").send().await?;
    Ok(resp.json().await?)
}

pub async fn list_issues(
    project: Option<&str>,
    status: Option<&str>,
    closed: bool,
) -> Result<Vec<Issue>> {
    let mut url = String::from("/api/issues?");
    if let Some(p) = project {
        url.push_str(&format!("project={p}&"));
    }
    if let Some(s) = status {
        url.push_str(&format!("status={s}&"));
    }
    if closed {
        url.push_str("closed=true&");
    }
    let resp = Request::get(&url).send().await?;
    Ok(resp.json().await?)
}

pub async fn get_issue(id: &str) -> Result<Issue> {
    let resp = Request::get(&format!("/api/issues/{id}")).send().await?;
    if resp.status() == 404 {
        return Err(ApiError(format!("issue '{id}' not found")));
    }
    Ok(resp.json().await?)
}

pub async fn create_issue(req: &CreateIssueRequest) -> Result<CreateIssueResponse> {
    let resp = Request::post("/api/issues")
        .json(req)
        .map_err(|e| ApiError(e.to_string()))?
        .send()
        .await?;
    Ok(resp.json().await?)
}

pub async fn edit_issue(id: &str, req: &EditIssueRequest) -> Result<()> {
    let _resp = Request::patch(&format!("/api/issues/{id}"))
        .json(req)
        .map_err(|e| ApiError(e.to_string()))?
        .send()
        .await?;
    Ok(())
}

pub async fn close_issue(id: &str) -> Result<()> {
    let _resp = Request::post(&format!("/api/issues/{id}/close"))
        .send()
        .await?;
    Ok(())
}

pub async fn reopen_issue(id: &str) -> Result<()> {
    let _resp = Request::post(&format!("/api/issues/{id}/reopen"))
        .send()
        .await?;
    Ok(())
}

pub async fn search(q: &str) -> Result<Vec<SearchResult>> {
    let resp = Request::get(&format!("/api/search?q={q}"))
        .send()
        .await?;
    Ok(resp.json().await?)
}
