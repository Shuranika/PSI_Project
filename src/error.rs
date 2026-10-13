use std::{fmt, io};

#[derive(Debug)]
pub enum AppError {
    MissingFile(String),
    InvalidWorkbook(String),
    InvalidDocument(String),
    Read(String),
    Save(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFile(path) => write!(formatter, "Required file is missing: {path}"),
            Self::InvalidWorkbook(message) => write!(formatter, "Invalid workbook: {message}"),
            Self::InvalidDocument(message) => {
                write!(formatter, "Invalid tickets document: {message}")
            }
            Self::Read(message) => write!(formatter, "Could not read input: {message}"),
            Self::Save(message) => write!(formatter, "Could not save results.xlsx: {message}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
        Self::Read(error.to_string())
    }
}
