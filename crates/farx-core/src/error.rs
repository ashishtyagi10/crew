use thiserror::Error;

#[derive(Error, Debug)]
pub enum FarxError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Filesystem error: {0}")]
    Fs(String),

    #[error("UI error: {0}")]
    Ui(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("AI error: {0}")]
    Ai(String),

    #[error("{0}")]
    Other(String),
}
