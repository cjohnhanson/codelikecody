use crate::error::{Error, Result};

use super::Backend;

/// Backend that reads and writes secrets via the OS credential store
/// (macOS Keychain, Windows Credential Manager, Linux secret-service).
pub struct KeyringBackend;

/// Parse a `SERVICE/ACCOUNT` path, returning (service, account).
fn parse_path(path: &str) -> Result<(&str, &str)> {
    let (service, account) = path
        .split_once('/')
        .ok_or_else(|| Error::InvalidRefUri(format!("ref+keyring://{path} — expected SERVICE/ACCOUNT")))?;

    if service.is_empty() || account.is_empty() {
        return Err(Error::InvalidRefUri(format!(
            "ref+keyring://{path} — service and account must not be empty"
        )));
    }

    Ok((service, account))
}

impl Backend for KeyringBackend {
    fn resolve(&self, path: &str) -> Result<String> {
        let (service, account) = parse_path(path)?;

        let entry = keyring::Entry::new(service, account)
            .map_err(|e| Error::KeyringError(format!("{service}/{account}: {e}")))?;

        entry
            .get_password()
            .map_err(|e| Error::KeyringError(format!("{service}/{account}: {e}")))
    }

    fn set(&self, path: &str, value: &str) -> Result<()> {
        let (service, account) = parse_path(path)?;

        let entry = keyring::Entry::new(service, account)
            .map_err(|e| Error::KeyringError(format!("{service}/{account}: {e}")))?;

        entry
            .set_password(value)
            .map_err(|e| Error::KeyringError(format!("{service}/{account}: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_slash_fails() {
        let backend = KeyringBackend;
        let err = backend.resolve("no-slash").unwrap_err();
        assert!(matches!(err, Error::InvalidRefUri(_)));
    }

    #[test]
    fn empty_service_fails() {
        let backend = KeyringBackend;
        let err = backend.resolve("/account").unwrap_err();
        assert!(matches!(err, Error::InvalidRefUri(_)));
    }

    #[test]
    fn empty_account_fails() {
        let backend = KeyringBackend;
        let err = backend.resolve("service/").unwrap_err();
        assert!(matches!(err, Error::InvalidRefUri(_)));
    }

    #[test]
    fn nonexistent_entry_returns_error() {
        let backend = KeyringBackend;
        let result = backend.resolve("belmont-test-nonexistent/does-not-exist");
        assert!(result.is_err());
    }

    #[test]
    fn roundtrip_set_then_resolve() {
        let backend = KeyringBackend;
        let path = "belmont-test/roundtrip-test";

        // Skip if no keychain is available (e.g. nix sandbox).
        if let Err(e) = backend.set(path, "test-value-12345") {
            eprintln!("skipping roundtrip test: {e}");
            return;
        }

        // Resolve
        let value = backend.resolve(path).unwrap();
        assert_eq!(value, "test-value-12345");

        // Cleanup
        let (service, account) = parse_path(path).unwrap();
        let entry = keyring::Entry::new(service, account).unwrap();
        let _ = entry.delete_credential();
    }
}
