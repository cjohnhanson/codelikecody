use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Error;

const YAML_ROOT_CONFIG_FILENAME: &str = "clc.yml";
const TOML_CONFIG_FILENAME: &str = "clc.toml";
const YAML_CONFIG_FILENAME: &str = "config.yml";
const USER_CONFIG_FILENAME: &str = "config.yml";
const USER_CONFIG_DIR: &str = ".clc";

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

    #[serde(default)]
    test_command: Option<String>,
}

// --- Workflow policy types ---

/// Permission rules for a workflow phase or review.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PermissionsDef {
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
}

/// A transition target — either a bare phase name or a rich object with review gates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TransitionDef {
    Simple(String),
    Rich {
        target: String,
        /// Reviewer agent names that must approve before this transition.
        /// Each name resolves to `.clc/reviewers/<name>.md`.
        #[serde(default, deserialize_with = "deserialize_review_field")]
        review: Vec<String>,
    },
}

/// Accept `review:` as either a single string or a list of strings.
fn deserialize_review_field<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }
    match OneOrMany::deserialize(deserializer)? {
        OneOrMany::One(s) => Ok(vec![s]),
        OneOrMany::Many(v) => Ok(v),
    }
}

impl TransitionDef {
    /// The target phase name regardless of variant.
    pub fn target(&self) -> &str {
        match self {
            Self::Simple(s) => s,
            Self::Rich { target, .. } => target,
        }
    }

    /// Reviewer agent names required for this transition.
    pub fn reviewers(&self) -> &[String] {
        match self {
            Self::Simple(_) => &[],
            Self::Rich { review, .. } => review,
        }
    }
}

/// A single phase definition in a workflow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhaseDef {
    pub name: String,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub nudge: Option<String>,
    #[serde(default)]
    pub can_stop: bool,
    #[serde(default)]
    pub permissions: Option<PermissionsDef>,
    #[serde(default)]
    pub transitions: Option<Vec<TransitionDef>>,
}

/// Wrapper that allows a phase to be specified as either a bare string (name only)
/// or a full object. Used for backward compatibility with `phases = ["foo", "bar"]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
enum PhaseDefOrString {
    Full(PhaseDef),
    Name(String),
}

impl From<PhaseDefOrString> for PhaseDef {
    fn from(val: PhaseDefOrString) -> Self {
        match val {
            PhaseDefOrString::Full(p) => p,
            PhaseDefOrString::Name(name) => PhaseDef {
                name,
                instructions: None,
                nudge: None,
                can_stop: false,
                permissions: None,
                transitions: None,
            },
        }
    }
}

/// A named workflow: description and phase graph. Review gates live on
/// transitions — each transition's `review` field names the reviewer agents
/// that must approve before the transition is allowed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowDef {
    #[serde(default)]
    pub description: Option<String>,

    #[serde(default, deserialize_with = "deserialize_phases")]
    pub phases: Vec<PhaseDef>,
}

fn deserialize_phases<'de, D>(deserializer: D) -> Result<Vec<PhaseDef>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: Vec<PhaseDefOrString> = Vec::deserialize(deserializer)?;
    Ok(raw.into_iter().map(PhaseDef::from).collect())
}

/// Deserialize an optional phases list (for topology WorkflowSpec where phases
/// are optional — omitted means use the built-in default).
pub fn deserialize_phases_opt<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<PhaseDef>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: Option<Vec<PhaseDefOrString>> = Option::deserialize(deserializer)?;
    Ok(raw.map(|v| v.into_iter().map(PhaseDef::from).collect()))
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

fn default_workspace() -> String {
    "worktree".to_string()
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

    /// Workspace isolation type. Opaque string — the supervisor uses this
    /// to select the workspace implementation, not to branch on known values.
    #[serde(default = "default_workspace")]
    pub workspace: String,

    /// Container image for SSH-based workspaces.
    #[serde(default)]
    pub image: Option<String>,

    #[serde(default)]
    pub auto_grant: Vec<String>,

    #[serde(default)]
    pub always_escalate: Vec<String>,

    /// Named workflow. Determines phase graph, permissions, and which
    /// agents run at review gates.
    #[serde(default)]
    pub workflow: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SupervisorConfig {
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u64,

    #[serde(default = "default_api_port")]
    pub api_port: u16,

    #[serde(default = "default_tunnel_base_port")]
    pub tunnel_base_port: u16,

    #[serde(default)]
    pub coordinators: Vec<CoordinatorScope>,

    /// Workflow name → full definition (phase graph + review agents).
    /// Source of truth for phase transition validation and review gate enforcement.
    #[serde(default)]
    pub workflows: std::collections::HashMap<String, WorkflowDef>,
}

const fn default_api_port() -> u16 {
    19100
}

const fn default_tunnel_base_port() -> u16 {
    19200
}

/// Default model for reviewer agents when not specified in the reviewer file.
pub const DEFAULT_REVIEWER_MODEL: &str = "sonnet";

/// Generate a random hex token (16 bytes / 32 hex chars).
pub fn generate_token() -> String {
    use std::fmt::Write;
    let mut bytes = [0u8; 16];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut bytes);
    }
    let mut buf = String::with_capacity(32);
    for b in &bytes {
        let _ = write!(buf, "{b:02x}");
    }
    buf
}

/// Baseline tool grants seeded at dispatch. The phase guard (workflow
/// permissions) constrains what edits are allowed in each phase — these
/// grants cover the mechanical tools every worker needs.
pub const BASELINE_TOOL_GRANTS: &[&str] = &[
    "Read",
    "Write",
    "Edit",
    "MultiEdit",
    "Grep",
    "Glob",
    "WebFetch",
    "WebSearch",
    "Bash",
    "Bash(*)",
    "Agent",
    "Skill",
    "ToolSearch",
    "NotebookEdit",
];

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

    /// Command to run to verify tests pass (e.g. "cargo test --workspace").
    /// Used by phase transitions to/from "green" and by `clc done`.
    #[serde(default)]
    pub test_command: Option<String>,

    /// Additional bash command prefixes allowed on trunk. These are appended
    /// to the built-in allowlist (git, cargo, clc, etc). Use for project-specific
    /// tools that agents should be able to run on trunk.
    #[serde(default)]
    pub trunk_bash_allow: Vec<String>,
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

static DEFAULT_WORKFLOW_DEF: std::sync::LazyLock<WorkflowDef> =
    std::sync::LazyLock::new(WorkflowDef::default);

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
            test_command: None,
            trunk_bash_allow: Vec::new(),
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
            test_command: toml.test_command,
            trunk_bash_allow: Vec::new(),
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
    let mut cfg = if yaml_root_path.exists() {
        load_yaml(&yaml_root_path)?
    } else {
        let toml_path = project_dir.join(TOML_CONFIG_FILENAME);
        if toml_path.exists() {
            load_toml(&toml_path)?
        } else {
            let yaml_path = project_dir.join(".clc").join(YAML_CONFIG_FILENAME);
            if yaml_path.exists() {
                load_yaml(&yaml_path)?
            } else {
                Config::default()
            }
        }
    };

    // Merge workflow definitions from topology (clc.yaml) if present.
    // The topology is the authoritative source for workflows — its definitions
    // take precedence over anything in clc.yml.
    if let Ok(Some(topo)) = crate::topology::load(project_dir) {
        for (name, spec) in &topo.workflows {
            if let Some(phases) = &spec.phases {
                cfg.workflows.insert(
                    name.clone(),
                    WorkflowDef {
                        description: spec.description.clone(),
                        phases: phases.clone(),
                    },
                );
            }
        }
    }

    Ok(cfg)
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

/// User-level config loaded from `~/.clc/config.yml`.
/// Contains only fields that make sense at the user level —
/// no workflow/phase enforcement, no main_branch overrides.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UserConfig {
    #[serde(default)]
    pub skills: Vec<SkillSource>,

    #[serde(default)]
    pub tisket: Option<UserTisketConfig>,

    #[serde(default)]
    pub zettel: Option<UserZettelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTisketConfig {
    pub root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserZettelConfig {
    pub root: PathBuf,
}

/// Load user-level config from `$HOME/.clc/config.yml`.
/// Returns `None` if the file doesn't exist or HOME isn't set.
/// Returns an error if the file exists but is invalid.
pub fn load_user_config() -> Result<Option<UserConfig>, Error> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Ok(None),
    };
    let config_path = home.join(USER_CONFIG_DIR).join(USER_CONFIG_FILENAME);
    if !config_path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(&config_path).map_err(|e| {
        Error::NonBlocking(format!(
            "failed to read user config {}: {e}",
            config_path.display()
        ))
    })?;
    let mut user_config: UserConfig = serde_yml::from_str(&contents).map_err(|e| {
        Error::NonBlocking(format!(
            "invalid user config {}: {e}",
            config_path.display()
        ))
    })?;

    // Resolve relative skill paths against the config file's parent directory.
    let config_dir = config_path.parent().unwrap_or(&home);
    resolve_skill_paths(&mut user_config.skills, config_dir);

    // Resolve relative tisket/zettel roots against the config file's parent directory.
    if let Some(ref mut t) = user_config.tisket {
        if t.root.is_relative() {
            t.root = config_dir.join(&t.root);
        }
    }
    if let Some(ref mut z) = user_config.zettel {
        if z.root.is_relative() {
            z.root = config_dir.join(&z.root);
        }
    }

    Ok(Some(user_config))
}

/// Resolve relative skill paths against a base directory.
fn resolve_skill_paths(skills: &mut [SkillSource], base: &Path) {
    for skill in skills.iter_mut() {
        if let SkillSource::Path { path } = skill {
            let p = PathBuf::from(path.as_str());
            if p.is_relative() {
                *path = base.join(&p).to_string_lossy().to_string();
            }
        }
    }
}

/// Merge user-level config into a repo-level config.
/// Skills are unioned. Other repo-level settings are preserved as-is.
pub fn merge_user_config(repo: &mut Config, user: &UserConfig) {
    // Union skills: user skills come first, then repo skills.
    let mut merged_skills = user.skills.clone();
    merged_skills.extend(repo.skills.drain(..));
    repo.skills = merged_skills;
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

    // --- Workflow config tests ---

    #[test]
    fn parse_workflow_with_full_phase_objects() {
        let yaml = r#"
workflows:
  tdd:
    description: "Test-driven development"
    phases:
      - name: tests-unwritten
        instructions: "Write failing tests."
        transitions: [tests-written]
        permissions:
          allow: ["Edit(tests/**)"]
          deny: ["Edit", "Write"]
      - name: tests-written
        transitions: [implementing]
      - name: implementing
        nudge: "Run tests."
        transitions: [green]
      - name: green
        can_stop: true
        transitions:
          - implementing
          - target: done
            review: [code]
      - name: done
"#;
        let config: Config = serde_yml::from_str(yaml).unwrap();
        let wf = &config.workflows["tdd"];
        assert_eq!(wf.description.as_deref(), Some("Test-driven development"));
        assert_eq!(wf.phases.len(), 5);
        assert_eq!(wf.phases[0].name, "tests-unwritten");
        assert_eq!(
            wf.phases[0].instructions.as_deref(),
            Some("Write failing tests.")
        );
        assert!(wf.phases[0].permissions.is_some());
        let perms = wf.phases[0].permissions.as_ref().unwrap();
        assert_eq!(perms.allow, vec!["Edit(tests/**)"]);
        assert_eq!(perms.deny, vec!["Edit", "Write"]);

        // Simple transition
        let t = wf.phases[0].transitions.as_ref().unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].target(), "tests-written");
        assert!(t[0].reviewers().is_empty());

        // Rich transition with review agents
        let green_t = wf.phases[3].transitions.as_ref().unwrap();
        assert_eq!(green_t.len(), 2);
        assert_eq!(green_t[0].target(), "implementing");
        assert_eq!(green_t[1].target(), "done");
        assert_eq!(green_t[1].reviewers(), &["code".to_string()]);

        // Terminal phase (no transitions)
        assert!(wf.phases[4].transitions.is_none());

        // can_stop
        assert!(wf.phases[3].can_stop);
        assert!(!wf.phases[0].can_stop);

        // nudge
        assert_eq!(wf.phases[2].nudge.as_deref(), Some("Run tests."));
    }

    #[test]
    fn parse_workflow_with_plain_string_phases() {
        let toml = r#"
[workflows.default]
phases = ["tests-unwritten", "tests-written", "red", "implementing", "green"]
"#;
        let config: Config = toml::from_str::<TomlFile>(toml).unwrap().into();
        let wf = &config.workflows["default"];
        assert_eq!(wf.phases.len(), 5);
        assert_eq!(wf.phases[0].name, "tests-unwritten");
        assert_eq!(wf.phases[4].name, "green");
        // All fields default
        assert!(wf.phases[0].instructions.is_none());
        assert!(wf.phases[0].transitions.is_none());
        assert!(!wf.phases[0].can_stop);
    }

    #[test]
    fn parse_workflow_empty_phases_default() {
        let yaml = "workflows:\n  empty: {}\n";
        let config: Config = serde_yml::from_str(yaml).unwrap();
        let wf = &config.workflows["empty"];
        assert!(wf.phases.is_empty());
        assert!(wf.description.is_none());
    }

    #[test]
    fn resolve_workflow_matches_label() {
        let yaml = r#"
workflows:
  docs:
    description: "docs workflow"
    phases:
      - name: draft
        transitions: [done]
      - name: done
  default:
    phases:
      - name: working
        transitions: [done]
      - name: done
rules:
  - workflow: docs
    match:
      label: docs
"#;
        let config: Config = serde_yml::from_str(yaml).unwrap();
        let wf = config.resolve_workflow(&["docs".into()], "v0.1.0");
        assert_eq!(wf.description.as_deref(), Some("docs workflow"));

        let wf_default = config.resolve_workflow(&["feature".into()], "v0.1.0");
        assert_eq!(wf_default.phases[0].name, "working");
    }

    #[test]
    fn transition_def_target_and_requires() {
        let simple = TransitionDef::Simple("foo".into());
        assert_eq!(simple.target(), "foo");
        assert!(simple.reviewers().is_empty());

        let rich = TransitionDef::Rich {
            target: "bar".into(),
            review: vec!["code-review".into(), "security-review".into()],
        };
        assert_eq!(rich.target(), "bar");
        assert_eq!(rich.reviewers().len(), 2);
    }

    #[test]
    fn review_field_deserializes_single_string() {
        let yaml = r#"
workflows:
  test:
    phases:
      - name: writing
        transitions:
          - target: done
            review: docs-review
      - done
"#;
        let config: Config = serde_yml::from_str(yaml).unwrap();
        let wf = &config.workflows["test"];
        let t = &wf.phases[0].transitions.as_ref().unwrap()[0];
        assert_eq!(t.reviewers(), &["docs-review"]);
    }

    #[test]
    fn review_field_deserializes_list() {
        let yaml = r#"
workflows:
  test:
    phases:
      - name: writing
        transitions:
          - target: done
            review: [scope-check, code-review]
      - done
"#;
        let config: Config = serde_yml::from_str(yaml).unwrap();
        let wf = &config.workflows["test"];
        let t = &wf.phases[0].transitions.as_ref().unwrap()[0];
        assert_eq!(t.reviewers(), &["scope-check", "code-review"]);
    }

    #[test]
    fn review_field_absent_means_empty() {
        let yaml = r#"
workflows:
  test:
    phases:
      - name: writing
        transitions:
          - target: done
      - done
"#;
        let config: Config = serde_yml::from_str(yaml).unwrap();
        let wf = &config.workflows["test"];
        let t = &wf.phases[0].transitions.as_ref().unwrap()[0];
        assert!(t.reviewers().is_empty());
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
