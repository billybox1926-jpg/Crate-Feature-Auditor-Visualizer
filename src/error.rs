use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Metadata error: {0}")]
    Metadata(String),
    #[error("Resolution error: {0}")]
    Resolution(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Suggestions error: {0}")]
    Suggestions(String),
    #[error("CLI error: {0}")]
    Cli(String),
    #[error("Remote analysis error: {0}")]
    Remote(String),
    #[error("{0}")]
    Other(String),
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Other(s.to_string())
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Error::Parse(format!("invalid UTF-8: {e}"))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
