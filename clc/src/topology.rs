use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Error;

const TOPOLOGY_FILENAME: &str = "clc.yaml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceType {
    Worker,
    Reviewer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSpec {
    #[serde(rename = "type")]
    pub workspace_type: WorkspaceType,
    pub agent: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SelectorSpec {
    pub label: Option<String>,
    pub exclude_label: Option<String>,
    pub project: Option<String>,
    pub depends_on: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorSpec {
    pub workspace: String,
    #[serde(default)]
    pub selector: SelectorSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InboxSpec {
    FolderWatch { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutboxSpec {
    FolderWrite { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    pub prompt: String,
    pub inboxes: Vec<String>,
    pub outboxes: Vec<String>,
    pub coordinators: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TopologyConfig {
    #[serde(default)]
    pub workspaces: HashMap<String, WorkspaceSpec>,
    #[serde(default)]
    pub coordinators: HashMap<String, CoordinatorSpec>,
    #[serde(default)]
    pub inboxes: HashMap<String, InboxSpec>,
    #[serde(default)]
    pub outboxes: HashMap<String, OutboxSpec>,
    pub admin: Option<AdminConfig>,
}

impl TopologyConfig {
    /// Validate that all cross-references are consistent.
    pub fn validate(&self) -> Result<(), Error> {
        for (name, coordinator) in &self.coordinators {
            if !self.workspaces.contains_key(&coordinator.workspace) {
                return Err(Error::NonBlocking(format!(
                    "coordinator '{name}' references unknown workspace '{}'",
                    coordinator.workspace
                )));
            }
        }

        if let Some(ref admin) = self.admin {
            for inbox in &admin.inboxes {
                if !self.inboxes.contains_key(inbox) {
                    return Err(Error::NonBlocking(format!(
                        "admin references unknown inbox '{inbox}'"
                    )));
                }
            }
            for outbox in &admin.outboxes {
                if !self.outboxes.contains_key(outbox) {
                    return Err(Error::NonBlocking(format!(
                        "admin references unknown outbox '{outbox}'"
                    )));
                }
            }
            for coordinator in &admin.coordinators {
                if !self.coordinators.contains_key(coordinator) {
                    return Err(Error::NonBlocking(format!(
                        "admin references unknown coordinator '{coordinator}'"
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Load topology from `clc.yaml` at the project root.
/// Returns `Ok(None)` if the file does not exist.
/// Returns an error if the file exists but is invalid.
pub fn load(project_dir: &Path) -> Result<Option<TopologyConfig>, Error> {
    let path = project_dir.join(TOPOLOGY_FILENAME);

    if !path.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&path)
        .map_err(|e| Error::NonBlocking(format!("failed to read {}: {e}", path.display())))?;

    let config: TopologyConfig = serde_yml::from_str(&contents)
        .map_err(|e| Error::NonBlocking(format!("invalid {}: {e}", path.display())))?;

    config.validate()?;

    Ok(Some(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> TopologyConfig {
        serde_yml::from_str(yaml).expect("valid yaml")
    }

    // --- Parsing tests ---

    #[test]
    fn parse_empty_topology() {
        let config = parse("{}");
        assert!(config.workspaces.is_empty());
        assert!(config.coordinators.is_empty());
        assert!(config.inboxes.is_empty());
        assert!(config.outboxes.is_empty());
        assert!(config.admin.is_none());
    }

    #[test]
    fn parse_workspace_worker_type() {
        let yaml = "
workspaces:
  primary:
    type: worker
    agent: claude-sonnet-4-6
";
        let config = parse(yaml);
        let ws = config.workspaces.get("primary").unwrap();
        assert_eq!(ws.workspace_type, WorkspaceType::Worker);
        assert_eq!(ws.agent, "claude-sonnet-4-6");
    }

    #[test]
    fn parse_workspace_reviewer_type() {
        let yaml = "
workspaces:
  reviewer:
    type: reviewer
    agent: claude-opus-4-6
";
        let config = parse(yaml);
        let ws = config.workspaces.get("reviewer").unwrap();
        assert_eq!(ws.workspace_type, WorkspaceType::Reviewer);
        assert_eq!(ws.agent, "claude-opus-4-6");
    }

    #[test]
    fn parse_coordinator_with_selector() {
        let yaml = "
workspaces:
  worker:
    type: worker
    agent: claude-sonnet-4-6
coordinators:
  backend:
    workspace: worker
    selector:
      label: backend
      exclude_label: blocked
";
        let config = parse(yaml);
        let coord = config.coordinators.get("backend").unwrap();
        assert_eq!(coord.workspace, "worker");
        assert_eq!(coord.selector.label.as_deref(), Some("backend"));
        assert_eq!(coord.selector.exclude_label.as_deref(), Some("blocked"));
        assert!(coord.selector.project.is_none());
        assert!(coord.selector.depends_on.is_none());
    }

    #[test]
    fn parse_coordinator_without_selector() {
        let yaml = "
workspaces:
  worker:
    type: worker
    agent: claude-sonnet-4-6
coordinators:
  main:
    workspace: worker
";
        let config = parse(yaml);
        let coord = config.coordinators.get("main").unwrap();
        assert_eq!(coord.workspace, "worker");
        assert!(coord.selector.label.is_none());
    }

    #[test]
    fn parse_inboxes_and_outboxes() {
        let yaml = "
inboxes:
  user-inbox:
    type: folder_watch
    path: .clc/inbox/user
outboxes:
  worker-outbox:
    type: folder_write
    path: .clc/outbox/worker
";
        let config = parse(yaml);
        match config.inboxes.get("user-inbox").unwrap() {
            InboxSpec::FolderWatch { path } => assert_eq!(path, ".clc/inbox/user"),
        }
        match config.outboxes.get("worker-outbox").unwrap() {
            OutboxSpec::FolderWrite { path } => assert_eq!(path, ".clc/outbox/worker"),
        }
    }

    #[test]
    fn parse_admin_config() {
        let yaml = "
workspaces:
  worker:
    type: worker
    agent: claude-sonnet-4-6
coordinators:
  main:
    workspace: worker
inboxes:
  user-inbox:
    type: folder_watch
    path: .clc/inbox/user
outboxes:
  worker-outbox:
    type: folder_write
    path: .clc/outbox/worker
admin:
  prompt: You are the admin agent.
  inboxes: [user-inbox]
  outboxes: [worker-outbox]
  coordinators: [main]
";
        let config = parse(yaml);
        let admin = config.admin.as_ref().unwrap();
        assert_eq!(admin.prompt, "You are the admin agent.");
        assert_eq!(admin.inboxes, vec!["user-inbox"]);
        assert_eq!(admin.outboxes, vec!["worker-outbox"]);
        assert_eq!(admin.coordinators, vec!["main"]);
    }

    #[test]
    fn parse_multiple_workspaces_and_coordinators() {
        let yaml = "
workspaces:
  worker:
    type: worker
    agent: claude-sonnet-4-6
  reviewer:
    type: reviewer
    agent: claude-opus-4-6
coordinators:
  backend:
    workspace: worker
    selector:
      label: backend
  frontend:
    workspace: worker
    selector:
      label: frontend
";
        let config = parse(yaml);
        assert_eq!(config.workspaces.len(), 2);
        assert_eq!(config.coordinators.len(), 2);
    }

    #[test]
    fn parse_selector_with_project_and_depends_on() {
        let yaml = "
workspaces:
  worker:
    type: worker
    agent: claude-sonnet-4-6
coordinators:
  main:
    workspace: worker
    selector:
      project: v0.1.0
      depends_on: abc123
";
        let config = parse(yaml);
        let coord = config.coordinators.get("main").unwrap();
        assert_eq!(coord.selector.project.as_deref(), Some("v0.1.0"));
        assert_eq!(coord.selector.depends_on.as_deref(), Some("abc123"));
    }

    // --- Validation tests ---

    #[test]
    fn validate_valid_topology_succeeds() {
        let yaml = "
workspaces:
  worker:
    type: worker
    agent: claude-sonnet-4-6
coordinators:
  main:
    workspace: worker
inboxes:
  user-inbox:
    type: folder_watch
    path: .clc/inbox/user
outboxes:
  worker-outbox:
    type: folder_write
    path: .clc/outbox/worker
admin:
  prompt: You are the admin.
  inboxes: [user-inbox]
  outboxes: [worker-outbox]
  coordinators: [main]
";
        let config = parse(yaml);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_empty_topology_succeeds() {
        let config = TopologyConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_coordinator_unknown_workspace_fails() {
        let yaml = "
workspaces:
  worker:
    type: worker
    agent: claude-sonnet-4-6
coordinators:
  main:
    workspace: nonexistent
";
        let config = parse(yaml);
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("main"), "expected 'main' in: {msg}");
        assert!(
            msg.contains("nonexistent"),
            "expected 'nonexistent' in: {msg}"
        );
    }

    #[test]
    fn validate_admin_unknown_inbox_fails() {
        let yaml = "
workspaces:
  worker:
    type: worker
    agent: claude-sonnet-4-6
coordinators:
  main:
    workspace: worker
inboxes: {}
outboxes:
  worker-outbox:
    type: folder_write
    path: .clc/outbox/worker
admin:
  prompt: Admin
  inboxes: [missing-inbox]
  outboxes: [worker-outbox]
  coordinators: [main]
";
        let config = parse(yaml);
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("missing-inbox"),
            "expected 'missing-inbox' in: {msg}"
        );
    }

    #[test]
    fn validate_admin_unknown_outbox_fails() {
        let yaml = "
workspaces:
  worker:
    type: worker
    agent: claude-sonnet-4-6
coordinators:
  main:
    workspace: worker
inboxes:
  user-inbox:
    type: folder_watch
    path: .clc/inbox/user
outboxes: {}
admin:
  prompt: Admin
  inboxes: [user-inbox]
  outboxes: [missing-outbox]
  coordinators: [main]
";
        let config = parse(yaml);
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("missing-outbox"),
            "expected 'missing-outbox' in: {msg}"
        );
    }

    #[test]
    fn validate_admin_unknown_coordinator_fails() {
        let yaml = "
workspaces:
  worker:
    type: worker
    agent: claude-sonnet-4-6
coordinators: {}
inboxes:
  user-inbox:
    type: folder_watch
    path: .clc/inbox/user
outboxes:
  worker-outbox:
    type: folder_write
    path: .clc/outbox/worker
admin:
  prompt: Admin
  inboxes: [user-inbox]
  outboxes: [worker-outbox]
  coordinators: [missing-coordinator]
";
        let config = parse(yaml);
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("missing-coordinator"),
            "expected 'missing-coordinator' in: {msg}"
        );
    }

    // --- File loading tests ---

    #[test]
    fn load_returns_none_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let result = load(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn load_parses_valid_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("clc.yaml"),
            "workspaces:\n  worker:\n    type: worker\n    agent: claude-sonnet-4-6\n",
        )
        .unwrap();

        let result = load(dir.path()).unwrap();
        assert!(result.is_some());
        let config = result.unwrap();
        assert!(config.workspaces.contains_key("worker"));
    }

    #[test]
    fn load_returns_error_for_invalid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        // workspaces expects a map but gets a list — invalid structure
        std::fs::write(dir.path().join("clc.yaml"), "workspaces:\n  - not-a-map\n").unwrap();

        let result = load(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn load_returns_error_for_invalid_references() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("clc.yaml"),
            "coordinators:\n  main:\n    workspace: nonexistent\n",
        )
        .unwrap();

        let result = load(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn load_parses_full_topology() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = "\
workspaces:
  worker:
    type: worker
    agent: claude-sonnet-4-6
coordinators:
  main:
    workspace: worker
    selector:
      label: backend
inboxes:
  user-inbox:
    type: folder_watch
    path: .clc/inbox/user
outboxes:
  worker-outbox:
    type: folder_write
    path: .clc/outbox/worker
admin:
  prompt: You are the admin agent.
  inboxes: [user-inbox]
  outboxes: [worker-outbox]
  coordinators: [main]
";
        std::fs::write(dir.path().join("clc.yaml"), yaml).unwrap();

        let result = load(dir.path()).unwrap().unwrap();
        assert_eq!(result.workspaces.len(), 1);
        assert_eq!(result.coordinators.len(), 1);
        assert_eq!(result.inboxes.len(), 1);
        assert_eq!(result.outboxes.len(), 1);
        assert!(result.admin.is_some());
    }
}
