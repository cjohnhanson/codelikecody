use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config;
use crate::error::Error;

const TOPOLOGY_FILENAME: &str = "clc.yaml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceType {
    Worker,
    Reviewer,
}

fn default_isolation() -> String {
    "worktree".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSpec {
    #[serde(rename = "type")]
    pub workspace_type: WorkspaceType,
    pub agent: String,
    /// Isolation type (e.g. "worktree", "docker", "podman"). Opaque string.
    #[serde(default = "default_isolation")]
    pub isolation: String,
    /// Container image for SSH-based workspaces.
    #[serde(default)]
    pub image: Option<String>,
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
    #[serde(default = "default_max_workers")]
    pub max_workers: usize,
    #[serde(default)]
    pub auto_grant: Vec<String>,
    #[serde(default)]
    pub always_escalate: Vec<String>,
    /// Named workflow from the `workflows` map. Determines phase graph,
    /// permissions, and which agents run at review gates.
    #[serde(default)]
    pub workflow: Option<String>,
}

fn default_max_workers() -> usize {
    3
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

fn default_api_port() -> u16 {
    19100
}

fn default_tunnel_base_port() -> u16 {
    19200
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorSpec {
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u64,

    #[serde(default = "default_api_port")]
    pub api_port: u16,

    #[serde(default = "default_tunnel_base_port")]
    pub tunnel_base_port: u16,
}

fn default_poll_interval() -> u64 {
    10
}

impl Default for SupervisorSpec {
    fn default() -> Self {
        Self {
            poll_interval: default_poll_interval(),
            api_port: default_api_port(),
            tunnel_base_port: default_tunnel_base_port(),
        }
    }
}

/// A workflow definition in the topology. Review gates live on transitions
/// within the phase graph — each transition's `review` field names the
/// reviewer agents that must approve.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowSpec {
    /// Phase graph. When omitted, the built-in default_tdd graph is used.
    #[serde(default, deserialize_with = "crate::config::deserialize_phases_opt")]
    pub phases: Option<Vec<crate::config::PhaseDef>>,

    /// Human-readable description injected into prime text.
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TopologyConfig {
    #[serde(default)]
    pub workspaces: HashMap<String, WorkspaceSpec>,
    #[serde(default)]
    pub coordinators: HashMap<String, CoordinatorSpec>,
    #[serde(default)]
    pub workflows: HashMap<String, WorkflowSpec>,
    #[serde(default)]
    pub inboxes: HashMap<String, InboxSpec>,
    #[serde(default)]
    pub outboxes: HashMap<String, OutboxSpec>,
    pub admin: Option<AdminConfig>,
    #[serde(default)]
    pub supervisor: SupervisorSpec,
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
            if let Some(ref wf) = coordinator.workflow {
                if !self.workflows.contains_key(wf) {
                    return Err(Error::NonBlocking(format!(
                        "coordinator '{name}' references unknown workflow '{wf}'"
                    )));
                }
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

    /// Convert topology into the supervisor config the runtime consumes.
    /// Each coordinator becomes a `CoordinatorScope` with fields resolved
    /// from its referenced workspace.
    pub fn to_supervisor_config(&self) -> config::SupervisorConfig {
        let mut coordinators = Vec::new();

        // Sort by name for deterministic ordering.
        let mut names: Vec<&String> = self.coordinators.keys().collect();
        names.sort();

        for name in names {
            let coord = &self.coordinators[name];
            let ws = self.workspaces.get(&coord.workspace);

            let (model, isolation, image) = match ws {
                Some(ws) => {
                    (ws.agent.clone(), ws.isolation.clone(), ws.image.clone())
                }
                None => (
                    "opus".to_string(),
                    "worktree".to_string(),
                    None,
                ),
            };

            coordinators.push(config::CoordinatorScope {
                id: name.clone(),
                project: coord.selector.project.clone(),
                label: coord.selector.label.clone(),
                exclude_label: coord.selector.exclude_label.clone(),
                max_workers: coord.max_workers,
                model,
                workspace: isolation,
                image,
                auto_grant: coord.auto_grant.clone(),
                always_escalate: coord.always_escalate.clone(),
                workflow: coord.workflow.clone(),
            });
        }

        let mut workflows = HashMap::new();
        for (name, spec) in &self.workflows {
            match spec.phases.clone() {
                Some(phases) => {
                    workflows.insert(name.clone(), config::WorkflowDef {
                        description: spec.description.clone(),
                        phases,
                    });
                }
                None => {
                    eprintln!(
                        "topology: workflow '{name}' has no phases — skipping \
                         (add a 'phases:' list to include it)"
                    );
                }
            }
        }

        config::SupervisorConfig {
            poll_interval: self.supervisor.poll_interval,
            api_port: self.supervisor.api_port,
            tunnel_base_port: self.supervisor.tunnel_base_port,
            coordinators,
            workflows,
        }
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

    // --- Conversion tests ---

    #[test]
    fn to_supervisor_config_maps_coordinator_and_workspace() {
        let yaml = "
workspaces:
  docker-worker:
    type: worker
    agent: opus
    isolation: docker
    image: clc-worker:latest
coordinators:
  dev:
    workspace: docker-worker
    max_workers: 2
    selector:
      label: backend
      project: v0.1.0
supervisor:
  poll_interval: 5
";
        let topo = parse(yaml);
        let sup = topo.to_supervisor_config();

        assert_eq!(sup.poll_interval, 5);
        assert_eq!(sup.coordinators.len(), 1);

        let c = &sup.coordinators[0];
        assert_eq!(c.id, "dev");
        assert_eq!(c.model, "opus");
        assert_eq!(c.max_workers, 2);
        assert_eq!(c.label.as_deref(), Some("backend"));
        assert_eq!(c.project.as_deref(), Some("v0.1.0"));
        assert_eq!(c.workspace, "docker");
        assert_eq!(c.image.as_deref(), Some("clc-worker:latest"));
    }

    #[test]
    fn to_supervisor_config_worktree_default() {
        let yaml = "
workspaces:
  local:
    type: worker
    agent: sonnet
coordinators:
  main:
    workspace: local
";
        let topo = parse(yaml);
        let sup = topo.to_supervisor_config();
        let c = &sup.coordinators[0];

        assert_eq!(c.model, "sonnet");
        assert_eq!(c.workspace, "worktree");
        assert!(c.image.is_none());
        assert_eq!(c.max_workers, 3); // default
    }

    #[test]
    fn to_supervisor_config_multiple_coordinators_sorted() {
        let yaml = "
workspaces:
  w:
    type: worker
    agent: haiku
coordinators:
  zebra:
    workspace: w
  alpha:
    workspace: w
";
        let topo = parse(yaml);
        let sup = topo.to_supervisor_config();

        assert_eq!(sup.coordinators[0].id, "alpha");
        assert_eq!(sup.coordinators[1].id, "zebra");
    }

    #[test]
    fn to_supervisor_config_default_poll_interval() {
        let yaml = "
workspaces:
  w:
    type: worker
    agent: opus
coordinators:
  c:
    workspace: w
";
        let topo = parse(yaml);
        let sup = topo.to_supervisor_config();
        assert_eq!(sup.poll_interval, 10);
    }

    #[test]
    fn to_supervisor_config_passes_through_workflow() {
        let yaml = "
workspaces:
  w:
    type: worker
    agent: opus
workflows:
  standard:
    agents:
      - scope-check
      - code-quality
coordinators:
  dev:
    workspace: w
    workflow: standard
";
        let topo = parse(yaml);
        let sup = topo.to_supervisor_config();
        assert_eq!(sup.coordinators[0].workflow.as_deref(), Some("standard"));
    }

    #[test]
    fn to_supervisor_config_no_workflow_default() {
        let yaml = "
workspaces:
  w:
    type: worker
    agent: opus
coordinators:
  c:
    workspace: w
";
        let topo = parse(yaml);
        let sup = topo.to_supervisor_config();
        assert!(sup.coordinators[0].workflow.is_none());
    }

    #[test]
    fn validate_rejects_unknown_workflow() {
        let yaml = "
workspaces:
  w:
    type: worker
    agent: opus
coordinators:
  c:
    workspace: w
    workflow: nonexistent
";
        let topo = parse(yaml);
        let result = topo.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("nonexistent"), "expected workflow name in error: {err}");
    }

    #[test]
    fn validate_accepts_valid_workflow_reference() {
        let yaml = "
workspaces:
  w:
    type: worker
    agent: opus
workflows:
  my-flow:
    phases:
      - draft
      - done
coordinators:
  c:
    workspace: w
    workflow: my-flow
";
        let topo = parse(yaml);
        assert!(topo.validate().is_ok());
    }

    #[test]
    fn to_supervisor_config_excludes_workflow_without_phases() {
        let yaml = "
workspaces:
  w:
    type: worker
    agent: opus
workflows:
  has-phases:
    description: This one has phases
    phases:
      - draft
      - done
  no-phases:
    description: This one has no phases
coordinators:
  c:
    workspace: w
";
        let topo = parse(yaml);
        let sup = topo.to_supervisor_config();

        // The workflow with phases should be included.
        assert!(sup.workflows.contains_key("has-phases"),
            "workflow with phases should be in config");

        // The workflow without phases should be excluded.
        assert!(!sup.workflows.contains_key("no-phases"),
            "workflow without phases should NOT be in config");
    }

    #[test]
    fn to_supervisor_config_includes_workflow_phases() {
        let yaml = "
workspaces:
  w:
    type: worker
    agent: opus
workflows:
  docs:
    description: Documentation
    phases:
      - name: outline
        transitions:
          - target: draft
            review: docs-review
      - name: draft
        transitions: [done]
      - done
coordinators:
  c:
    workspace: w
    workflow: docs
";
        let topo = parse(yaml);
        let sup = topo.to_supervisor_config();

        // Workflow should be in the supervisor config.
        let wf = sup.workflows.get("docs").expect("docs workflow missing");
        assert_eq!(wf.phases.len(), 3);
        assert_eq!(wf.phases[0].name, "outline");
        assert_eq!(wf.description.as_deref(), Some("Documentation"));

        // The review gate should be preserved.
        let t = &wf.phases[0].transitions.as_ref().unwrap()[0];
        assert_eq!(t.reviewers(), &["docs-review"]);
    }
}
