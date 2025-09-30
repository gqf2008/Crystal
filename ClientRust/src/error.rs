use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("ui error: {0}")]
    Ui(String),
    #[error("audio error: {0}")]
    Audio(String),
    #[error("runtime error: {0}")]
    Runtime(String),
}

#[allow(dead_code)]
pub type ClientResult<T> = Result<T, ClientError>;
