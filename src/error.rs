use std::{fmt, fmt::Display, io};
use thiserror::Error as ThisError;

#[derive(Debug)]
pub enum HeadlessError {
    String(String),
    Wrapped(Box<dyn std::error::Error + Send + Sync>),
}

impl Display for HeadlessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            Self::Wrapped(e) => write!(f, "{}", e),
        }
    }
}

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Error making HTTP request: {0}")]
    Http(#[from] ureq::Error),

    #[error("Error in headless browser: {0}")]
    Headless(HeadlessError),

    #[error("Error while doing IO: {0}")]
    Io(#[from] io::Error),
}
