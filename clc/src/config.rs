use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Error;

const CONFIG_FILENAME: &str = "config.yml";

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
    pub coordinator: CoordinatorConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            main_branch: default_main_branch(),
            required_attempts: default_required_attempts(),
            permissions: PermissionsConfig::default(),
            coordinator: CoordinatorConfig::default(),
        }
    }
}

const fn default_required_attempts() -> u32 {
    1
}

fn default_main_branch() -> String {
    "main".to_string()
}

/// Load config from `.clc/config.yml` in the given project directory.
/// Returns defaults if the file doesn't exist. Returns an error if
/// the file exists but is invalid.
pub fn load(project_dir: &Path) -> Result<Config, Error> {
    let config_path = project_dir.join(".clc").join(CONFIG_FILENAME);

    if !config_path.exists() {
        return Ok(Config::default());
    }

    let contents = std::fs::read_to_string(&config_path).map_err(|e| {
        Error::NonBlocking(format!(
            "failed to read config {}: {e}",
            config_path.display()
        ))
    })?;

    serde_yml::from_str(&contents)
        .map_err(|e| Error::NonBlocking(format!("invalid config {}: {e}", config_path.display())))
}

/// Print the effective config as YAML.
pub fn show(project_dir: &Path) -> Result<(), Error> {
    let config = load(project_dir)?;
    let yaml = serde_yml::to_string(&config)
        .map_err(|e| Error::NonBlocking(format!("failed to serialize config: {e}")))?;
    print!("{yaml}");
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
        let yaml = serde_yml::to_string(&config).unwrap();
        assert!(yaml.contains("coordinator"));
        assert!(yaml.contains("auto_grant"));
        assert!(yaml.contains("always_escalate"));
        assert!(yaml.contains("Bash(cargo *)"));
        assert!(yaml.contains("Bash(rm *)"));
    }
}
