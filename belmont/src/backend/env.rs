use crate::error::{Error, Result};

/// Resolve a `ref+env://VAR_NAME` reference by reading the environment variable.
pub fn resolve(var_name: &str) -> Result<String> {
    std::env::var(var_name).map_err(|_| Error::EnvNotSet(var_name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_set_variable() {
        unsafe { std::env::set_var("BELMONT_TEST_SECRET", "hunter2") };
        let value = resolve("BELMONT_TEST_SECRET").unwrap();
        assert_eq!(value, "hunter2");
        unsafe { std::env::remove_var("BELMONT_TEST_SECRET") };
    }

    #[test]
    fn missing_variable_returns_error() {
        let err = resolve("BELMONT_DEFINITELY_NOT_SET_12345").unwrap_err();
        assert!(matches!(err, Error::EnvNotSet(_)));
    }
}
