use std::sync::PoisonError;
use thiserror::Error;
pub type Result<T> = anyhow::Result<T, KvsError>;

#[derive(Debug, Error)]
pub enum KvsError {
    #[error("")]
    EmptyResponse,

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("BufReader Error: {0}")]
    BufReaderError(String, std::io::Error),

    #[error("Error: {0}")]
    GeneralError(String),

    #[error("Can't glob given pattern")]
    GlobPatternError(#[from] glob::PatternError),

    #[error("Key not found")]
    KeyNotFound,

    #[error("Can't create or detect log index ID")]
    LogIndexIDError,

    #[error("Can't parse log index ID")]
    LogIndexParseError(#[from] std::num::ParseIntError),

    #[error("PoisonError: {0}")]
    PoisonError(String),

    #[error("Can't serialize data")]
    SerializationError(#[from] bincode::Error),

    #[error("Server Not Initialized")]
    ServerNotInitialized,

    #[error("Sled DB Error")]
    SledDB(#[from] sled::Error),

    #[error("Thread Error: {0}")]
    ThreadError(String),

    #[error("UTF8 Error")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("Wrong engine selected")]
    WrongEngine,
}

impl<T> From<PoisonError<T>> for KvsError {
    fn from(error: PoisonError<T>) -> Self {
        KvsError::PoisonError(error.to_string())
    }
}
