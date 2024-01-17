use std::{error::Error, fmt::Display};

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    SerdeJson(serde_json::Error),
    Glob(glob::PatternError),
    Recv(std::sync::mpsc::RecvError),
    TryRecv(std::sync::mpsc::TryRecvError),
    AddrParse(std::net::AddrParseError),
}

impl Error for AppError {}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{}", error),
            Self::SerdeJson(error) => write!(f, "{}", error),
            Self::Glob(error) => write!(f, "{}", error),
            Self::Recv(error) => write!(f, "{}", error),
            Self::TryRecv(error) => write!(f, "{}", error),
            Self::AddrParse(error) => write!(f, "{}", error),
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

impl From<std::sync::mpsc::RecvError> for AppError {
    fn from(value: std::sync::mpsc::RecvError) -> Self {
        AppError::Recv(value)
    }
}

impl From<std::sync::mpsc::TryRecvError> for AppError {
    fn from(value: std::sync::mpsc::TryRecvError) -> Self {
        AppError::TryRecv(value)
    }
}

impl From<std::net::AddrParseError> for AppError {
    fn from(value: std::net::AddrParseError) -> Self {
        AppError::AddrParse(value)
    }
}
