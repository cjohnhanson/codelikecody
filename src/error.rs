use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum Error {
    #[error("{0}")]
    Block(String),

    #[error("{0}")]
    NonBlocking(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::Block(_) => 2,
            _ => 1,
        }
    }
}
