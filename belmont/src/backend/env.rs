use crate::error::{Error, Result};

use super::Backend;

/// Backend that reads secrets from environment variables.
pub struct EnvBackend;

impl Backend for EnvBackend {
    fn resolve(&self, var_name: &str) -> Result<String> {
        std::env::var(var_name).map_err(|_| Error::EnvNotSet(var_name.to_string()))
    }

    fn set(&self, _path: &str, _value: &str) -> Result<()> {
        Err(Error::ReadOnlyBackend("env".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_set_variable() {
        unsafe { std::env::set_var("BELMONT_TEST_SECRET", "hunter2") };
        let backend = EnvBackend;
        let value = backend.resolve("BELMONT_TEST_SECRET").unwrap();
        assert_eq!(value, "hunter2");
        unsafe { std::env::remove_var("BELMONT_TEST_SECRET") };
    }

    #[test]
    fn missing_variable_returns_error() {
        let backend = EnvBackend;
        let err = backend.resolve("BELMONT_DEFINITELY_NOT_SET_12345").unwrap_err();
        assert!(matches!(err, Error::EnvNotSet(_)));
    }

    #[test]
    fn set_returns_read_only() {
        let backend = EnvBackend;
        let err = backend.set("VAR", "val").unwrap_err();
        assert!(matches!(err, Error::ReadOnlyBackend(_)));
    }
}
