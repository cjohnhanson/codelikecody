use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Issue {
    pub id: String,
    pub project: String,
    pub frontmatter: IssueFrontmatter,
    pub body: String,
    pub scratch: String,
    pub closed: bool,
    pub diverges: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IssueFrontmatter {
    pub title: String,
    pub status: String,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub due_date: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Project {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchResult {
    pub issue: Issue,
    pub matched_fields: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateIssueResponse {
    pub id: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct EditIssueRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}
