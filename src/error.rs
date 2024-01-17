use std::{error::Error, fmt::Display};

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    SerdeJson(serde_json::Error),
    Glob(glob::PatternError),
}

impl Error for AppError {}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{}", error),
            Self::SerdeJson(error) => write!(f, "{}", error),
            Self::Glob(error) => write!(f, "{}", error),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::Io(value)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        AppError::SerdeJson(value)
    }
}

impl From<glob::PatternError> for AppError {
    fn from(value: glob::PatternError) -> Self {
        AppError::Glob(value)
    }
}
