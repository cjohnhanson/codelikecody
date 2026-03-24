use crate::error::{Error, Result};

/// Resolve a `ref+keyring://SERVICE/ACCOUNT` reference.
///
/// The path format is `SERVICE/ACCOUNT` where SERVICE maps to the keyring
/// service name and ACCOUNT maps to the user/account name.
pub fn resolve(path: &str) -> Result<String> {
    let (service, account) = path
        .split_once('/')
        .ok_or_else(|| Error::InvalidRefUri(format!("ref+keyring://{path} — expected SERVICE/ACCOUNT")))?;

    if service.is_empty() || account.is_empty() {
        return Err(Error::InvalidRefUri(format!(
            "ref+keyring://{path} — service and account must not be empty"
        )));
    }

    let entry = keyring::Entry::new(service, account)
        .map_err(|e| Error::KeyringError(format!("{service}/{account}: {e}")))?;

    entry
        .get_password()
        .map_err(|e| Error::KeyringError(format!("{service}/{account}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_slash_fails() {
        let err = resolve("no-slash").unwrap_err();
        assert!(matches!(err, Error::InvalidRefUri(_)));
    }

    #[test]
    fn empty_service_fails() {
        let err = resolve("/account").unwrap_err();
        assert!(matches!(err, Error::InvalidRefUri(_)));
    }

    #[test]
    fn empty_account_fails() {
        let err = resolve("service/").unwrap_err();
        assert!(matches!(err, Error::InvalidRefUri(_)));
    }

    #[test]
    fn nonexistent_entry_returns_error() {
        // This should fail because no such keyring entry exists.
        let result = resolve("belmont-test-nonexistent/does-not-exist");
        assert!(result.is_err());
    }
}
