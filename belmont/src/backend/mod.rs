pub mod env;
pub mod keyring;

use crate::error::{Error, Result};

/// A parsed `ref+SCHEME://path` URI.
#[derive(Debug, Clone, PartialEq)]
pub struct RefUri {
    pub scheme: String,
    pub path: String,
}

/// Backend trait for resolving and storing secrets.
pub trait Backend {
    /// Resolve a secret value from this backend.
    fn resolve(&self, path: &str) -> Result<String>;

    /// Store a secret value in this backend.
    /// Returns an error if the backend is read-only.
    fn set(&self, path: &str, value: &str) -> Result<()>;
}

/// Parse a `ref+SCHEME://path` string into its components.
pub fn parse_ref_uri(uri: &str) -> Result<RefUri> {
    let rest = uri
        .strip_prefix("ref+")
        .ok_or_else(|| Error::InvalidRefUri(uri.to_string()))?;
    let (scheme, path) = rest
        .split_once("://")
        .ok_or_else(|| Error::InvalidRefUri(uri.to_string()))?;
    if scheme.is_empty() || path.is_empty() {
        return Err(Error::InvalidRefUri(uri.to_string()));
    }
    Ok(RefUri {
        scheme: scheme.to_string(),
        path: path.to_string(),
    })
}

/// Get the backend implementation for a given scheme.
fn backend_for(scheme: &str) -> Result<Box<dyn Backend>> {
    match scheme {
        "env" => Ok(Box::new(env::EnvBackend)),
        "keyring" => Ok(Box::new(keyring::KeyringBackend)),
        other => Err(Error::UnknownBackend(other.to_string())),
    }
}

/// Resolve a ref URI to its secret value using the appropriate backend.
pub fn resolve(uri: &str) -> Result<String> {
    let parsed = parse_ref_uri(uri)?;
    backend_for(&parsed.scheme)?.resolve(&parsed.path)
}

/// Store a value for a ref URI using the appropriate backend.
pub fn set(uri: &str, value: &str) -> Result<()> {
    let parsed = parse_ref_uri(uri)?;
    backend_for(&parsed.scheme)?.set(&parsed.path, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_env_ref() {
        let uri = parse_ref_uri("ref+env://DATABASE_URL").unwrap();
        assert_eq!(uri.scheme, "env");
        assert_eq!(uri.path, "DATABASE_URL");
    }

    #[test]
    fn parse_keyring_ref() {
        let uri = parse_ref_uri("ref+keyring://belmont/API_KEY").unwrap();
        assert_eq!(uri.scheme, "keyring");
        assert_eq!(uri.path, "belmont/API_KEY");
    }

    #[test]
    fn parse_age_ref_with_fragment() {
        let uri = parse_ref_uri("ref+age://secrets.age#/SECRET").unwrap();
        assert_eq!(uri.scheme, "age");
        assert_eq!(uri.path, "secrets.age#/SECRET");
    }

    #[test]
    fn missing_ref_prefix_fails() {
        let err = parse_ref_uri("env://DATABASE_URL").unwrap_err();
        assert!(matches!(err, Error::InvalidRefUri(_)));
    }

    #[test]
    fn missing_scheme_separator_fails() {
        let err = parse_ref_uri("ref+env:DATABASE_URL").unwrap_err();
        assert!(matches!(err, Error::InvalidRefUri(_)));
    }

    #[test]
    fn empty_scheme_fails() {
        let err = parse_ref_uri("ref+://something").unwrap_err();
        assert!(matches!(err, Error::InvalidRefUri(_)));
    }

    #[test]
    fn empty_path_fails() {
        let err = parse_ref_uri("ref+env://").unwrap_err();
        assert!(matches!(err, Error::InvalidRefUri(_)));
    }

    #[test]
    fn unknown_backend_fails() {
        let err = resolve("ref+vault://secret/path").unwrap_err();
        assert!(matches!(err, Error::UnknownBackend(_)));
    }

    #[test]
    fn env_set_is_read_only() {
        let err = set("ref+env://SOME_VAR", "value").unwrap_err();
        assert!(matches!(err, Error::ReadOnlyBackend(_)));
    }
}
