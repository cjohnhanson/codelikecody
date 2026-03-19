use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub project: String,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
    #[serde(default)]
    pub depends_on: Option<Vec<String>>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Deserialize)]
pub struct EditIssueRequest {
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub due_date: Option<String>,
    pub title: Option<String>,
    pub priority: Option<u8>,
    pub labels: Option<Vec<String>>,
    pub add_label: Option<String>,
    pub remove_label: Option<String>,
    pub depends_on: Option<Vec<String>>,
    pub body: Option<String>,
    pub append: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct StatusOverride {
    pub status: Option<String>,
}

#[derive(Serialize)]
pub struct CreateIssueResponse {
    pub id: String,
}

#[derive(Deserialize)]
pub struct ListIssuesParams {
    pub project: Option<String>,
    pub status: Option<String>,
    pub label: Option<String>,
    #[serde(default)]
    pub closed: bool,
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub project: Option<String>,
}
