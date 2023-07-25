use thiserror::Error;

#[derive(Debug, Error)]
pub enum KvsErrors {
    #[error("")]
    EmptyResponse,

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("BufReader Error")]
    BufReaderError(String, std::io::Error),

    #[error("Can't glob given pattern")]
    GlobPatternError(#[from] glob::PatternError),

    #[error("Key not found")]
    KeyNotFound,

    #[error("Can't create or detect log index ID")]
    LogIndexIDError,

    #[error("Can't parse log index ID")]
    LogIndexParseError(#[from] std::num::ParseIntError),

    #[error("Can't serialize data")]
    SerializationError(#[from] bincode::Error),
}
