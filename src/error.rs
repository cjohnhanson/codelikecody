use thiserror::Error;

#[derive(Debug, Error)]
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
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::Block(_) => 2,
            _ => 1,
        }
    }
}
