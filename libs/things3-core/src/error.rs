//! Error types for the Things Core library

use thiserror::Error;

/// Result type alias for Things operations
pub type Result<T> = std::result::Result<T, ThingsError>;

/// Main error type for Things operations
#[derive(Error, Debug)]
pub enum ThingsError {
    #[error("Database error: {0}")]
    Database(String),

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_database_error_from_rusqlite() {
        // Skip this test since rusqlite is not available in this build
        // This test would verify rusqlite error conversion if the dependency was available
    }

    #[test]
    fn test_serialization_error_from_serde() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let things_error: ThingsError = json_error.into();

        match things_error {
            ThingsError::Serialization(_) => (),
            _ => panic!("Expected Serialization error"),
        }
    }

    #[test]
    fn test_io_error_from_std() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let things_error: ThingsError = io_error.into();

        match things_error {
            ThingsError::Io(_) => (),
            _ => panic!("Expected Io error"),
        }
    }

    #[test]
    fn test_database_not_found_error() {
        let error = ThingsError::DatabaseNotFound {
            path: "/path/to/db".to_string(),
        };

        assert!(error.to_string().contains("Database not found"));
        assert!(error.to_string().contains("/path/to/db"));
    }

    #[test]
    fn test_invalid_uuid_error() {
        let error = ThingsError::InvalidUuid {
            uuid: "invalid-uuid".to_string(),
        };

        assert!(error.to_string().contains("Invalid UUID"));
        assert!(error.to_string().contains("invalid-uuid"));
    }

    #[test]
    fn test_invalid_date_error() {
        let error = ThingsError::InvalidDate {
            date: "2023-13-45".to_string(),
        };

        assert!(error.to_string().contains("Invalid date"));
        assert!(error.to_string().contains("2023-13-45"));
    }

    #[test]
    fn test_task_not_found_error() {
        let error = ThingsError::TaskNotFound {
            uuid: "task-uuid-123".to_string(),
        };

        assert!(error.to_string().contains("Task not found"));
        assert!(error.to_string().contains("task-uuid-123"));
    }

    #[test]
    fn test_project_not_found_error() {
        let error = ThingsError::ProjectNotFound {
            uuid: "project-uuid-456".to_string(),
        };

        assert!(error.to_string().contains("Project not found"));
        assert!(error.to_string().contains("project-uuid-456"));
    }

    #[test]
    fn test_area_not_found_error() {
        let error = ThingsError::AreaNotFound {
            uuid: "area-uuid-789".to_string(),
        };

        assert!(error.to_string().contains("Area not found"));
        assert!(error.to_string().contains("area-uuid-789"));
    }

    #[test]
    fn test_validation_error() {
        let error = ThingsError::Validation {
            message: "Invalid input data".to_string(),
        };

        assert!(error.to_string().contains("Validation error"));
        assert!(error.to_string().contains("Invalid input data"));
    }

    #[test]
    fn test_configuration_error() {
        let error = ThingsError::Configuration {
            message: "Missing required config".to_string(),
        };

        assert!(error.to_string().contains("Configuration error"));
        assert!(error.to_string().contains("Missing required config"));
    }

    #[test]
    fn test_unknown_error() {
        let error = ThingsError::Unknown {
            message: "Something went wrong".to_string(),
        };

        assert!(error.to_string().contains("Unknown error"));
        assert!(error.to_string().contains("Something went wrong"));
    }

    #[test]
    fn test_validation_helper() {
        let error = ThingsError::validation("Test validation message");

        match error {
            ThingsError::Validation { message } => {
                assert_eq!(message, "Test validation message");
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn test_validation_helper_with_string() {
        let message = "Test validation message".to_string();
        let error = ThingsError::validation(message);

        match error {
            ThingsError::Validation { message } => {
                assert_eq!(message, "Test validation message");
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn test_configuration_helper() {
        let error = ThingsError::configuration("Test config message");

        match error {
            ThingsError::Configuration { message } => {
                assert_eq!(message, "Test config message");
            }
            _ => panic!("Expected Configuration error"),
        }
    }

    #[test]
    fn test_configuration_helper_with_string() {
        let message = "Test config message".to_string();
        let error = ThingsError::configuration(message);

        match error {
            ThingsError::Configuration { message } => {
                assert_eq!(message, "Test config message");
            }
            _ => panic!("Expected Configuration error"),
        }
    }

    #[test]
    fn test_unknown_helper() {
        let error = ThingsError::unknown("Test unknown message");

        match error {
            ThingsError::Unknown { message } => {
                assert_eq!(message, "Test unknown message");
            }
            _ => panic!("Expected Unknown error"),
        }
    }

    #[test]
    fn test_unknown_helper_with_string() {
        let message = "Test unknown message".to_string();
        let error = ThingsError::unknown(message);

        match error {
            ThingsError::Unknown { message } => {
                assert_eq!(message, "Test unknown message");
            }
            _ => panic!("Expected Unknown error"),
        }
    }

    #[test]
    fn test_error_display_formatting() {
        let errors = vec![
            ThingsError::DatabaseNotFound {
                path: "test.db".to_string(),
            },
            ThingsError::InvalidUuid {
                uuid: "bad-uuid".to_string(),
            },
            ThingsError::InvalidDate {
                date: "bad-date".to_string(),
            },
            ThingsError::TaskNotFound {
                uuid: "task-123".to_string(),
            },
            ThingsError::ProjectNotFound {
                uuid: "project-456".to_string(),
            },
            ThingsError::AreaNotFound {
                uuid: "area-789".to_string(),
            },
            ThingsError::Validation {
                message: "validation failed".to_string(),
            },
            ThingsError::Configuration {
                message: "config error".to_string(),
            },
            ThingsError::Unknown {
                message: "unknown error".to_string(),
            },
        ];

        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty());
            assert!(error_string.len() > 10); // Should have meaningful content
        }
    }

    #[test]
    fn test_error_debug_formatting() {
        let error = ThingsError::Validation {
            message: "test message".to_string(),
        };

        let debug_string = format!("{error:?}");
        assert!(debug_string.contains("Validation"));
        assert!(debug_string.contains("test message"));
    }

    #[test]
    fn test_result_type_alias() {
        // Test that the Result type alias works correctly
        fn returns_result() -> String {
            "success".to_string()
        }

        fn returns_error() -> Result<String> {
            Err(ThingsError::validation("test error"))
        }

        assert_eq!(returns_result(), "success");
        assert!(returns_error().is_err());

        match returns_error() {
            Err(ThingsError::Validation { message }) => {
                assert_eq!(message, "test error");
            }
            _ => panic!("Expected Validation error"),
        }
    }
}
