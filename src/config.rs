use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Error;

const CONFIG_FILENAME: &str = "config.yml";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_main_branch")]
    pub main_branch: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            main_branch: default_main_branch(),
        }
    }
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
