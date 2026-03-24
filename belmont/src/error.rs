#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not initialized (belmont.yml not found)")]
    NotInitialized,

    #[error("already initialized (belmont.yml exists)")]
    AlreadyInitialized,

    #[error("unknown backend scheme '{0}'")]
    UnknownBackend(String),

    #[error("secret '{0}' could not be resolved")]
    UnresolvableSecret(String),

    #[error("invalid ref URI '{0}'")]
    InvalidRefUri(String),

    #[error("backend config cycle detected: {0}")]
    ConfigCycle(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Yaml(#[from] serde_yml::Error),

    #[error("env: variable '{0}' not set")]
    EnvNotSet(String),

    #[error("keyring: {0}")]
    KeyringError(String),

    #[error("backend '{0}' is read-only")]
    ReadOnlyBackend(String),
}

pub type Result<T> = std::result::Result<T, Error>;
