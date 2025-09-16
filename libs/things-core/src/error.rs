//! Error types for the Things Core library

use thiserror::Error;

/// Result type alias for Things operations
pub type Result<T> = std::result::Result<T, ThingsError>;

/// Main error type for Things operations
#[derive(Error, Debug)]
pub enum ThingsError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database not found: {path}")]
    DatabaseNotFound { path: String },

    #[error("Invalid UUID: {uuid}")]
    InvalidUuid { uuid: String },

    #[error("Invalid date: {date}")]
    InvalidDate { date: String },

    #[error("Task not found: {uuid}")]
    TaskNotFound { uuid: String },

    #[error("Project not found: {uuid}")]
    ProjectNotFound { uuid: String },

    #[error("Area not found: {uuid}")]
    AreaNotFound { uuid: String },

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Unknown error: {message}")]
    Unknown { message: String },
}

impl ThingsError {
    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Create an unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::Unknown {
            message: message.into(),
        }
    }
}
