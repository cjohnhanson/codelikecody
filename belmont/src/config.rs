use std::collections::BTreeMap;

use camino::Utf8Path;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BelmontConfig {
    #[serde(default)]
    pub backends: BTreeMap<String, BackendConfig>,
    #[serde(default)]
    pub secrets: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackendConfig {
    #[serde(flatten)]
    pub settings: BTreeMap<String, String>,
}

impl BelmontConfig {
    pub fn load(project_dir: &Utf8Path) -> Result<Self> {
        let path = project_dir.join("belmont.yml");
        if !path.exists() {
            return Err(Error::NotInitialized);
        }
        let contents = std::fs::read_to_string(&path)?;
        let config: BelmontConfig = serde_yml::from_str(&contents)?;
        Ok(config)
    }

    pub fn empty() -> Self {
        BelmontConfig {
            backends: BTreeMap::new(),
            secrets: BTreeMap::new(),
        }
    }

    pub fn init(project_dir: &Utf8Path) -> Result<()> {
        let path = project_dir.join("belmont.yml");
        if path.exists() {
            return Err(Error::AlreadyInitialized);
        }
        let config = Self::empty();
        let contents = serde_yml::to_string(&config)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_config() {
        let yaml = "backends: {}\nsecrets: {}\n";
        let config: BelmontConfig = serde_yml::from_str(yaml).unwrap();
        assert!(config.backends.is_empty());
        assert!(config.secrets.is_empty());
    }

    #[test]
    fn parse_config_with_secrets() {
        let yaml = r#"
backends: {}
secrets:
  DATABASE_URL: "ref+env://DATABASE_URL"
  API_KEY: "ref+keyring://belmont/API_KEY"
"#;
        let config: BelmontConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.secrets.len(), 2);
        assert_eq!(
            config.secrets["DATABASE_URL"],
            "ref+env://DATABASE_URL"
        );
        assert_eq!(
            config.secrets["API_KEY"],
            "ref+keyring://belmont/API_KEY"
        );
    }

    #[test]
    fn parse_config_with_backend_settings() {
        let yaml = r#"
backends:
  age:
    identity: "ref+keyring://belmont/age-identity"
secrets:
  SECRET: "ref+age://secrets.age#/SECRET"
"#;
        let config: BelmontConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.backends.len(), 1);
        assert_eq!(
            config.backends["age"].settings["identity"],
            "ref+keyring://belmont/age-identity"
        );
    }

    #[test]
    fn load_missing_file_returns_not_initialized() {
        let dir = Utf8Path::new("/tmp/belmont-test-nonexistent");
        let err = BelmontConfig::load(dir).unwrap_err();
        assert!(matches!(err, Error::NotInitialized));
    }

    #[test]
    fn init_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = Utf8Path::from_path(dir.path()).unwrap();
        BelmontConfig::init(dir_path).unwrap();
        let config = BelmontConfig::load(dir_path).unwrap();
        assert!(config.secrets.is_empty());
    }

    #[test]
    fn init_twice_fails() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = Utf8Path::from_path(dir.path()).unwrap();
        BelmontConfig::init(dir_path).unwrap();
        let err = BelmontConfig::init(dir_path).unwrap_err();
        assert!(matches!(err, Error::AlreadyInitialized));
    }
}
