use std::collections::BTreeMap;

use crate::backend;
use crate::config::BelmontConfig;

/// Resolved secret: name, value, and whether resolution succeeded.
#[derive(Debug)]
pub struct ResolvedSecret {
    pub name: String,
    pub value: Option<String>,
    pub error: Option<String>,
}

/// Orchestrates secret resolution from config.
pub struct SecretRegistry {
    resolved: Vec<ResolvedSecret>,
}

impl SecretRegistry {
    /// Resolve all secrets declared in the config.
    pub fn resolve(config: &BelmontConfig) -> Self {
        let mut resolved = Vec::new();
        for (name, uri) in &config.secrets {
            match backend::resolve(uri) {
                Ok(value) => resolved.push(ResolvedSecret {
                    name: name.clone(),
                    value: Some(value),
                    error: None,
                }),
                Err(e) => resolved.push(ResolvedSecret {
                    name: name.clone(),
                    value: None,
                    error: Some(e.to_string()),
                }),
            }
        }
        SecretRegistry { resolved }
    }

    /// Names of secrets that could not be resolved.
    pub fn missing(&self) -> Vec<&str> {
        self.resolved
            .iter()
            .filter(|s| s.value.is_none())
            .map(|s| s.name.as_str())
            .collect()
    }

    /// Successfully resolved name/value pairs.
    pub fn resolved_pairs(&self) -> Vec<(String, String)> {
        self.resolved
            .iter()
            .filter_map(|s| {
                s.value
                    .as_ref()
                    .map(|v| (s.name.clone(), v.clone()))
            })
            .collect()
    }

    /// All declared secret names.
    pub fn names(&self) -> Vec<&str> {
        self.resolved.iter().map(|s| s.name.as_str()).collect()
    }

    /// All resolved secrets with their status.
    pub fn all(&self) -> &[ResolvedSecret] {
        &self.resolved
    }

    /// Returns true if all secrets were resolved successfully.
    pub fn all_resolved(&self) -> bool {
        self.resolved.iter().all(|s| s.value.is_some())
    }

    /// Build an environment variable map from resolved secrets.
    pub fn env_map(&self) -> BTreeMap<String, String> {
        self.resolved_pairs().into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_secrets(secrets: Vec<(&str, &str)>) -> BelmontConfig {
        BelmontConfig {
            backends: BTreeMap::new(),
            secrets: secrets
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn resolves_env_secrets() {
        unsafe { std::env::set_var("BELMONT_REG_TEST", "value123") };
        let config = config_with_secrets(vec![("MY_SECRET", "ref+env://BELMONT_REG_TEST")]);
        let reg = SecretRegistry::resolve(&config);
        assert!(reg.all_resolved());
        assert_eq!(reg.resolved_pairs(), vec![("MY_SECRET".to_string(), "value123".to_string())]);
        unsafe { std::env::remove_var("BELMONT_REG_TEST") };
    }

    #[test]
    fn missing_env_secret_tracked() {
        let config =
            config_with_secrets(vec![("GONE", "ref+env://BELMONT_DEFINITELY_MISSING_999")]);
        let reg = SecretRegistry::resolve(&config);
        assert!(!reg.all_resolved());
        assert_eq!(reg.missing(), vec!["GONE"]);
    }

    #[test]
    fn mixed_resolved_and_missing() {
        unsafe { std::env::set_var("BELMONT_MIX_OK", "found") };
        let config = config_with_secrets(vec![
            ("GOOD", "ref+env://BELMONT_MIX_OK"),
            ("BAD", "ref+env://BELMONT_MIX_MISSING"),
        ]);
        let reg = SecretRegistry::resolve(&config);
        assert!(!reg.all_resolved());
        assert_eq!(reg.missing(), vec!["BAD"]);
        assert_eq!(reg.resolved_pairs(), vec![("GOOD".to_string(), "found".to_string())]);
        unsafe { std::env::remove_var("BELMONT_MIX_OK") };
    }

    #[test]
    fn invalid_uri_tracked_as_missing() {
        let config = config_with_secrets(vec![("BROKEN", "not-a-ref-uri")]);
        let reg = SecretRegistry::resolve(&config);
        assert!(!reg.all_resolved());
        assert_eq!(reg.missing(), vec!["BROKEN"]);
    }

    #[test]
    fn env_map_only_includes_resolved() {
        unsafe { std::env::set_var("BELMONT_MAP_TEST", "mapval") };
        let config = config_with_secrets(vec![
            ("OK", "ref+env://BELMONT_MAP_TEST"),
            ("MISSING", "ref+env://BELMONT_MAP_NOPE"),
        ]);
        let reg = SecretRegistry::resolve(&config);
        let map = reg.env_map();
        assert_eq!(map.len(), 1);
        assert_eq!(map["OK"], "mapval");
        unsafe { std::env::remove_var("BELMONT_MAP_TEST") };
    }
}
