use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Error;

const YAML_ROOT_CONFIG_FILENAME: &str = "clc.yml";
const TOML_CONFIG_FILENAME: &str = "clc.toml";
const YAML_CONFIG_FILENAME: &str = "config.yml";

// --- TOML deserialization types (match the clc.toml file structure) ---

#[derive(Debug, Default, Deserialize)]
struct ProjectSection {
    #[serde(default = "default_main_branch")]
    main_branch: String,

    #[serde(default = "default_admin_branch")]
    admin_branch: String,

    #[serde(default = "default_required_attempts")]
    required_attempts: u32,
}

#[derive(Debug, Default, Deserialize)]
struct TomlFile {
    #[serde(default)]
    project: ProjectSection,

    #[serde(default)]
    worker: WorkerConfig,

    #[serde(default)]
    coordinator: CoordinatorConfig,

    #[serde(default)]
    supervisor: SupervisorConfig,

    #[serde(default)]
    workflows: HashMap<String, WorkflowDef>,

    #[serde(default)]
    rules: Vec<PolicyRule>,

    #[serde(default)]
    skills: Vec<SkillSource>,
}

// --- Workflow policy types ---

/// A named sequence of TDD phases for a workflow.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowDef {
    #[serde(default)]
    pub phases: Vec<String>,
}

/// Match criteria for a workflow policy rule.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleMatch {
    pub label: Option<String>,
    pub project: Option<String>,
    pub status: Option<String>,
}

/// A single workflow policy rule: if the match criteria are met, use this workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub workflow: String,
    #[serde(rename = "match")]
    pub criteria: RuleMatch,
}

// --- Public config types ---

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorkerPermissionsConfig {
    #[serde(default)]
    pub default: Vec<String>,

    #[serde(default)]
    pub deny: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorkerConfig {
    #[serde(default)]
    pub permissions: WorkerPermissionsConfig,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    #[serde(default)]
    pub auto_grant: Vec<String>,

    #[serde(default)]
    pub always_escalate: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceType {
    #[default]
    Worktree,
    Docker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorScope {
    pub id: String,

    #[serde(default)]
    pub project: Option<String>,

    #[serde(default)]
    pub label: Option<String>,

    #[serde(default)]
    pub exclude_label: Option<String>,

    #[serde(default = "default_max_workers")]
    pub max_workers: usize,

    #[serde(default = "default_coordinator_model")]
    pub model: String,

    #[serde(default)]
    pub workspace: WorkspaceType,

    /// Docker image to use for docker workspaces.
    #[serde(default)]
    pub docker_image: Option<String>,

    #[serde(default)]
    pub auto_grant: Vec<String>,

    #[serde(default)]
    pub always_escalate: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SupervisorConfig {
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u64,

    #[serde(default)]
    pub coordinators: Vec<CoordinatorScope>,
}

const fn default_poll_interval() -> u64 {
    10
}

const fn default_max_workers() -> usize {
    3
}

fn default_coordinator_model() -> String {
    "opus".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_main_branch")]
    pub main_branch: String,

    #[serde(default = "default_admin_branch")]
    pub admin_branch: String,

    #[serde(default = "default_required_attempts")]
    pub required_attempts: u32,

    #[serde(default)]
    pub worker: WorkerConfig,

    #[serde(default)]
    pub coordinator: CoordinatorConfig,

    #[serde(default)]
    pub supervisor: SupervisorConfig,

    #[serde(default)]
    pub workflows: HashMap<String, WorkflowDef>,

    #[serde(default)]
    pub rules: Vec<PolicyRule>,

    #[serde(default)]
    pub skills: Vec<SkillSource>,
}

pub use almanac::SkillSource;

impl Config {
    /// Resolve which workflow applies to an issue by evaluating rules in order.
    /// Returns the matching `WorkflowDef`, or the "default" workflow, or an
    /// empty workflow if neither exists.
    pub fn resolve_workflow<'a>(&'a self, labels: &[String], project: &str) -> &'a WorkflowDef {
        for rule in &self.rules {
            let label_match = rule
                .criteria
                .label
                .as_deref()
                .map_or(true, |l| labels.iter().any(|il| il == l));
            let project_match = rule
                .criteria
                .project
                .as_deref()
                .map_or(true, |p| p == project);
            if label_match && project_match {
                if let Some(wf) = self.workflows.get(&rule.workflow) {
                    return wf;
                }
            }
        }
        self.workflows
            .get("default")
            .unwrap_or(&DEFAULT_WORKFLOW_DEF)
    }
}

static DEFAULT_WORKFLOW_DEF: WorkflowDef = WorkflowDef { phases: Vec::new() };

impl Default for Config {
    fn default() -> Self {
        Self {
            main_branch: default_main_branch(),
            admin_branch: default_admin_branch(),
            required_attempts: default_required_attempts(),
            worker: WorkerConfig::default(),
            coordinator: CoordinatorConfig::default(),
            supervisor: SupervisorConfig::default(),
            workflows: HashMap::new(),
            rules: Vec::new(),
            skills: Vec::new(),
        }
    }
}

impl From<TomlFile> for Config {
    fn from(toml: TomlFile) -> Self {
        Self {
            main_branch: toml.project.main_branch,
            admin_branch: toml.project.admin_branch,
            required_attempts: toml.project.required_attempts,
            worker: toml.worker,
            coordinator: toml.coordinator,
            supervisor: toml.supervisor,
            workflows: toml.workflows,
            rules: toml.rules,
            skills: toml.skills,
        }
    }
}

const fn default_required_attempts() -> u32 {
    1
}

fn default_main_branch() -> String {
    "main".to_string()
}

fn default_admin_branch() -> String {
    "clc-admin".to_string()
}

/// Load config from `clc.yml` at the project root (primary), falling back to
/// `clc.toml`, then `.clc/config.yml` for backward compatibility. Returns
/// defaults if no file exists. Returns an error if a file exists but is invalid.
pub fn load(project_dir: &Path) -> Result<Config, Error> {
    let yaml_root_path = project_dir.join(YAML_ROOT_CONFIG_FILENAME);
    if yaml_root_path.exists() {
        return load_yaml(&yaml_root_path);
    }

    let toml_path = project_dir.join(TOML_CONFIG_FILENAME);
    if toml_path.exists() {
        return load_toml(&toml_path);
    }

    let yaml_path = project_dir.join(".clc").join(YAML_CONFIG_FILENAME);
    if yaml_path.exists() {
        return load_yaml(&yaml_path);
    }

    Ok(Config::default())
}

fn load_toml(path: &Path) -> Result<Config, Error> {
    let contents = std::fs::read_to_string(path).map_err(|e| {
        Error::NonBlocking(format!("failed to read config {}: {e}", path.display()))
    })?;

    let toml_file: TomlFile = toml::from_str(&contents)
        .map_err(|e| Error::NonBlocking(format!("invalid config {}: {e}", path.display())))?;

    Ok(Config::from(toml_file))
}

fn load_yaml(path: &Path) -> Result<Config, Error> {
    let contents = std::fs::read_to_string(path).map_err(|e| {
        Error::NonBlocking(format!("failed to read config {}: {e}", path.display()))
    })?;

    serde_yml::from_str(&contents)
        .map_err(|e| Error::NonBlocking(format!("invalid config {}: {e}", path.display())))
}

/// Print the effective config as YAML.
pub fn show(project_dir: &Path) -> Result<(), Error> {
    let config = load(project_dir)?;
    let yaml_str = serde_yml::to_string(&config)
        .map_err(|e| Error::NonBlocking(format!("failed to serialize config: {e}")))?;
    print!("{yaml_str}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_empty_coordinator() {
        let config = Config::default();
        assert!(config.coordinator.auto_grant.is_empty());
        assert!(config.coordinator.always_escalate.is_empty());
    }

    #[test]
    fn parse_config_without_coordinator_section() {
        let yaml = "main_branch: main\n";
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert!(config.coordinator.auto_grant.is_empty());
        assert!(config.coordinator.always_escalate.is_empty());
    }

    #[test]
    fn parse_config_with_auto_grant() {
        let yaml = "coordinator:\n  auto_grant:\n    - \"Bash(cargo *)\"\n    - \"Bash(npm *)\"\n";
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert_eq!(
            config.coordinator.auto_grant,
            vec!["Bash(cargo *)", "Bash(npm *)"]
        );
        assert!(config.coordinator.always_escalate.is_empty());
    }

    #[test]
    fn parse_config_with_always_escalate() {
        let yaml =
            "coordinator:\n  always_escalate:\n    - \"Bash(rm *)\"\n    - \"Bash(git push *)\"\n";
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert!(config.coordinator.auto_grant.is_empty());
        assert_eq!(
            config.coordinator.always_escalate,
            vec!["Bash(rm *)", "Bash(git push *)"]
        );
    }

    #[test]
    fn parse_config_with_full_coordinator_policy() {
        let yaml = "\
            coordinator:\n\
            \x20 auto_grant:\n\
            \x20   - \"Bash(cargo *)\"\n\
            \x20   - \"Bash(npm *)\"\n\
            \x20 always_escalate:\n\
            \x20   - \"Bash(rm *)\"\n\
            \x20   - \"Bash(git push *)\"\n";
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.coordinator.auto_grant.len(), 2);
        assert_eq!(config.coordinator.always_escalate.len(), 2);
    }

    #[test]
    fn parse_config_coordinator_coexists_with_worker_permissions() {
        let yaml = "\
            worker:\n\
            \x20 permissions:\n\
            \x20   default:\n\
            \x20     - \"Bash(just *)\"\n\
            coordinator:\n\
            \x20 auto_grant:\n\
            \x20   - \"Bash(cargo *)\"\n";
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.worker.permissions.default, vec!["Bash(just *)"]);
        assert_eq!(config.coordinator.auto_grant, vec!["Bash(cargo *)"]);
    }

    #[test]
    fn load_config_with_coordinator_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(
            clc_dir.join("config.yml"),
            "coordinator:\n  auto_grant:\n    - \"Bash(cargo *)\"\n  always_escalate:\n    - \"Bash(rm *)\"\n",
        )
        .unwrap();

        let config = load(dir.path()).unwrap();
        assert_eq!(config.coordinator.auto_grant, vec!["Bash(cargo *)"]);
        assert_eq!(config.coordinator.always_escalate, vec!["Bash(rm *)"]);
    }

    #[test]
    fn serialized_config_includes_coordinator() {
        let config = Config {
            coordinator: CoordinatorConfig {
                auto_grant: vec!["Bash(cargo *)".into()],
                always_escalate: vec!["Bash(rm *)".into()],
            },
            ..Config::default()
        };
        let output = toml::to_string_pretty(&config).unwrap();
        assert!(output.contains("coordinator"));
        assert!(output.contains("auto_grant"));
        assert!(output.contains("always_escalate"));
        assert!(output.contains("Bash(cargo *)"));
        assert!(output.contains("Bash(rm *)"));
    }

    // --- TOML config tests (new behavior after migration) ---

    #[test]
    fn load_toml_config_from_project_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("clc.toml"),
            "[project]\nmain_branch = \"trunk\"\n",
        )
        .unwrap();

        let config = load(dir.path()).unwrap();
        assert_eq!(config.main_branch, "trunk");
    }

    #[test]
    fn load_toml_config_defaults_when_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = load(dir.path()).unwrap();
        assert_eq!(config.main_branch, "main");
        assert_eq!(config.admin_branch, "clc-admin");
        assert!(config.coordinator.auto_grant.is_empty());
        assert!(config.coordinator.always_escalate.is_empty());
    }

    #[test]
    fn load_toml_config_with_custom_admin_branch() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("clc.toml"),
            "[project]\nadmin_branch = \"admin\"\n",
        )
        .unwrap();

        let config = load(dir.path()).unwrap();
        assert_eq!(config.admin_branch, "admin");
        assert_eq!(config.main_branch, "main"); // default preserved
    }

    #[test]
    fn load_toml_config_error_on_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("clc.toml"), "][not valid toml{{{\n").unwrap();

        let result = load(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn load_toml_config_with_coordinator_section() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("clc.toml"),
            "[project]\nmain_branch = \"main\"\n\n\
             [coordinator]\nauto_grant = [\"Bash(cargo *)\"]\nalways_escalate = [\"Bash(rm *)\"]\n",
        )
        .unwrap();

        let config = load(dir.path()).unwrap();
        assert_eq!(config.coordinator.auto_grant, vec!["Bash(cargo *)"]);
        assert_eq!(config.coordinator.always_escalate, vec!["Bash(rm *)"]);
    }

    #[test]
    fn load_toml_config_with_worker_permissions_default() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("clc.toml"),
            "[project]\nmain_branch = \"main\"\n\n\
             [worker.permissions]\n\
             default = [\"Read\", \"Grep\", \"Write({worktree}/**)\", \"Edit({worktree}/**)\"]\n",
        )
        .unwrap();

        let config = load(dir.path()).unwrap();
        assert_eq!(config.worker.permissions.default.len(), 4);
        assert!(config.worker.permissions.default.contains(&"Read".to_string()));
        assert!(config
            .worker
            .permissions
            .default
            .contains(&"Write({worktree}/**)".to_string()));
    }

    #[test]
    fn load_toml_config_with_worker_permissions_deny() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("clc.toml"),
            "[project]\nmain_branch = \"main\"\n\n\
             [worker.permissions]\n\
             default = [\"Read\"]\n\
             deny = [\"Write({worktree}/.clc/**)\", \"Edit({worktree}/.clc/**)\"]\n",
        )
        .unwrap();

        let config = load(dir.path()).unwrap();
        assert_eq!(config.worker.permissions.deny.len(), 2);
        assert!(config
            .worker
            .permissions
            .deny
            .contains(&"Write({worktree}/.clc/**)".to_string()));
        assert!(config
            .worker
            .permissions
            .deny
            .contains(&"Edit({worktree}/.clc/**)".to_string()));
    }

    #[test]
    fn load_toml_config_worker_permissions_empty_by_default() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("clc.toml"),
            "[project]\nmain_branch = \"main\"\n",
        )
        .unwrap();

        let config = load(dir.path()).unwrap();
        assert!(config.worker.permissions.default.is_empty());
        assert!(config.worker.permissions.deny.is_empty());
    }

    #[test]
    fn load_toml_config_prefers_clc_toml_over_yaml() {
        // When both clc.toml and .clc/config.yml exist, clc.toml wins.
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("config.yml"), "main_branch: yaml-branch\n").unwrap();
        std::fs::write(
            dir.path().join("clc.toml"),
            "[project]\nmain_branch = \"toml-branch\"\n",
        )
        .unwrap();

        let config = load(dir.path()).unwrap();
        assert_eq!(config.main_branch, "toml-branch");
    }

    // --- YAML root config tests (new behavior: clc.yml at project root) ---

    #[test]
    fn load_yaml_root_config_from_project_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("clc.yml"), "main_branch: trunk\n").unwrap();

        let config = load(dir.path()).unwrap();
        assert_eq!(config.main_branch, "trunk");
    }

    #[test]
    fn load_yaml_root_config_prefers_over_toml() {
        // When both clc.yml and clc.toml exist, clc.yml wins.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("clc.yml"), "main_branch: yaml-branch\n").unwrap();
        std::fs::write(
            dir.path().join("clc.toml"),
            "[project]\nmain_branch = \"toml-branch\"\n",
        )
        .unwrap();

        let config = load(dir.path()).unwrap();
        assert_eq!(config.main_branch, "yaml-branch");
    }

    #[test]
    fn load_yaml_root_config_prefers_over_clc_dir_yaml() {
        // clc.yml at root takes priority over .clc/config.yml.
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("config.yml"), "main_branch: legacy-branch\n").unwrap();
        std::fs::write(dir.path().join("clc.yml"), "main_branch: root-branch\n").unwrap();

        let config = load(dir.path()).unwrap();
        assert_eq!(config.main_branch, "root-branch");
    }

    #[test]
    fn load_yaml_root_config_error_on_invalid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("clc.yml"), "}{not valid yaml\n").unwrap();

        let result = load(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn load_yaml_root_config_with_coordinator_section() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("clc.yml"),
            "coordinator:\n  auto_grant:\n    - \"Bash(cargo *)\"\n  always_escalate:\n    - \"Bash(rm *)\"\n",
        )
        .unwrap();

        let config = load(dir.path()).unwrap();
        assert_eq!(config.coordinator.auto_grant, vec!["Bash(cargo *)"]);
        assert_eq!(config.coordinator.always_escalate, vec!["Bash(rm *)"]);
    }

    #[test]
    fn show_outputs_yaml_format() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("clc.yml"), "main_branch: trunk\n").unwrap();

        // show() produces YAML: keys use `: ` not ` = `
        // We test by serializing directly as YAML and checking the format.
        let config = load(dir.path()).unwrap();
        let yaml_str = serde_yml::to_string(&config).unwrap();
        assert!(yaml_str.contains("main_branch: trunk"), "expected YAML format but got: {yaml_str}");
        assert!(!yaml_str.contains("main_branch = "), "expected YAML not TOML but got: {yaml_str}");
    }

    // --- Skills config tests ---

    #[test]
    fn parse_yaml_config_with_skill_sources() {
        let yaml = "\
            skills:\n\
            \x20 - path: ~/Projects/co.d/skills/\n\
            \x20 - path: ./skills/\n\
            \x20 - git: git@github.com:cjohnhanson/skills.git\n";
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.skills.len(), 3);
        assert_eq!(
            config.skills[0],
            SkillSource::Path {
                path: "~/Projects/co.d/skills/".into()
            }
        );
        assert_eq!(
            config.skills[1],
            SkillSource::Path {
                path: "./skills/".into()
            }
        );
        assert_eq!(
            config.skills[2],
            SkillSource::Git {
                git: "git@github.com:cjohnhanson/skills.git".into()
            }
        );
    }

    #[test]
    fn parse_yaml_config_skills_empty_by_default() {
        let yaml = "main_branch: main\n";
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert!(config.skills.is_empty());
    }

    #[test]
    fn parse_yaml_config_skills_coexists_with_other_fields() {
        let yaml = "\
            main_branch: trunk\n\
            skills:\n\
            \x20 - path: ./skills/\n\
            coordinator:\n\
            \x20 auto_grant:\n\
            \x20   - \"Bash(cargo *)\"\n";
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.main_branch, "trunk");
        assert_eq!(config.skills.len(), 1);
        assert_eq!(config.coordinator.auto_grant, vec!["Bash(cargo *)"]);
    }

    #[test]
    fn load_yaml_root_config_with_skills() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("clc.yml"),
            "skills:\n  - path: ./skills/\n  - git: git@github.com:example/skills.git\n",
        )
        .unwrap();

        let config = load(dir.path()).unwrap();
        assert_eq!(config.skills.len(), 2);
    }

    #[test]
    fn load_toml_config_with_skills() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("clc.toml"),
            "[[skills]]\npath = \"./skills/\"\n\n[[skills]]\ngit = \"git@github.com:example/skills.git\"\n",
        )
        .unwrap();

        let config = load(dir.path()).unwrap();
        assert_eq!(config.skills.len(), 2);
        assert_eq!(
            config.skills[0],
            SkillSource::Path {
                path: "./skills/".into()
            }
        );
        assert_eq!(
            config.skills[1],
            SkillSource::Git {
                git: "git@github.com:example/skills.git".into()
            }
        );
    }

    #[test]
    fn load_toml_config_full_shape() {
        let toml = r#"
[project]
main_branch = "develop"

[worker.permissions]
default = [
    "Read",
    "Grep",
    "Glob",
    "Write({worktree}/**)",
    "Edit({worktree}/**)",
    "Bash(clc *)",
]
deny = [
    "Write({worktree}/.clc/**)",
    "Edit({worktree}/.clc/**)",
]

[coordinator]
auto_grant = ["Bash(cargo *)"]
always_escalate = ["Bash(rm *)"]
"#;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("clc.toml"), toml).unwrap();

        let config = load(dir.path()).unwrap();
        assert_eq!(config.main_branch, "develop");
        assert_eq!(config.worker.permissions.default.len(), 6);
        assert_eq!(config.worker.permissions.deny.len(), 2);
        assert_eq!(config.coordinator.auto_grant, vec!["Bash(cargo *)"]);
        assert_eq!(config.coordinator.always_escalate, vec!["Bash(rm *)"]);
    }
}
