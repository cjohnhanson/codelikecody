use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Error;

const TOML_CONFIG_FILENAME: &str = "clc.toml";
const YAML_CONFIG_FILENAME: &str = "config.yml";

// --- TOML deserialization types (match the clc.toml file structure) ---

#[derive(Debug, Default, Deserialize)]
struct ProjectSection {
    #[serde(default = "default_main_branch")]
    main_branch: String,

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
pub struct PermissionsConfig {
    #[serde(default)]
    pub allow: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    #[serde(default)]
    pub auto_grant: Vec<String>,

    #[serde(default)]
    pub always_escalate: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_main_branch")]
    pub main_branch: String,

    #[serde(default = "default_required_attempts")]
    pub required_attempts: u32,

    #[serde(default)]
    pub permissions: PermissionsConfig,

    #[serde(default)]
    pub worker: WorkerConfig,

    #[serde(default)]
    pub coordinator: CoordinatorConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            main_branch: default_main_branch(),
            required_attempts: default_required_attempts(),
            permissions: PermissionsConfig::default(),
            worker: WorkerConfig::default(),
            coordinator: CoordinatorConfig::default(),
        }
    }
}

impl From<TomlFile> for Config {
    fn from(toml: TomlFile) -> Self {
        Self {
            main_branch: toml.project.main_branch,
            required_attempts: toml.project.required_attempts,
            permissions: PermissionsConfig::default(),
            worker: toml.worker,
            coordinator: toml.coordinator,
        }
    }
}

const fn default_required_attempts() -> u32 {
    1
}

fn default_main_branch() -> String {
    "main".to_string()
}

/// Load config from `clc.toml` at the project root, falling back to
/// `.clc/config.yml` for backward compatibility. Returns defaults if
/// neither file exists. Returns an error if a file exists but is invalid.
pub fn load(project_dir: &Path) -> Result<Config, Error> {
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

/// Print the effective config as TOML.
pub fn show(project_dir: &Path) -> Result<(), Error> {
    let config = load(project_dir)?;
    let toml_str = toml::to_string_pretty(&config)
        .map_err(|e| Error::NonBlocking(format!("failed to serialize config: {e}")))?;
    print!("{toml_str}");
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
    fn parse_config_coordinator_coexists_with_permissions() {
        let yaml = "\
            permissions:\n\
            \x20 allow:\n\
            \x20   - \"Bash(just *)\"\n\
            coordinator:\n\
            \x20 auto_grant:\n\
            \x20   - \"Bash(cargo *)\"\n";
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.permissions.allow, vec!["Bash(just *)"]);
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
        assert!(config.coordinator.auto_grant.is_empty());
        assert!(config.coordinator.always_escalate.is_empty());
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
