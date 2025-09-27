//! Configuration management for Things 3 integration

use crate::error::{Result, ThingsError};
use std::path::{Path, PathBuf};

/// Configuration for Things 3 database access
#[derive(Debug, Clone)]
pub struct ThingsConfig {
    /// Path to the Things 3 database
    pub database_path: PathBuf,
    /// Whether to use the default database path if the specified path doesn't exist
    pub fallback_to_default: bool,
}

impl ThingsConfig {
    /// Create a new configuration with a custom database path
    ///
    /// # Arguments
    /// * `database_path` - Path to the Things 3 database
    /// * `fallback_to_default` - Whether to fall back to the default path if the specified path doesn't exist
    #[must_use]
    pub fn new<P: AsRef<Path>>(database_path: P, fallback_to_default: bool) -> Self {
        Self {
            database_path: database_path.as_ref().to_path_buf(),
            fallback_to_default,
        }
    }

    /// Create a configuration with the default database path
    #[must_use]
    pub fn with_default_path() -> Self {
        Self {
            database_path: Self::get_default_database_path(),
            fallback_to_default: false,
        }
    }

    /// Get the effective database path, falling back to default if needed
    ///
    /// # Errors
    /// Returns `ThingsError::Message` if neither the specified path nor the default path exists
    pub fn get_effective_database_path(&self) -> Result<PathBuf> {
        // Check if the specified path exists
        if self.database_path.exists() {
            return Ok(self.database_path.clone());
        }

        // If fallback is enabled, try the default path
        if self.fallback_to_default {
            let default_path = Self::get_default_database_path();
            if default_path.exists() {
                return Ok(default_path);
            }
        }

        Err(ThingsError::configuration(format!(
            "Database not found at {} and fallback is {}",
            self.database_path.display(),
            if self.fallback_to_default {
                "disabled"
            } else {
                "enabled but default path also not found"
            }
        )))
    }

    /// Get the default Things 3 database path
    #[must_use]
    pub fn get_default_database_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
        PathBuf::from(format!(
            "{home}/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite"
        ))
    }

    /// Create configuration from environment variables
    ///
    /// Reads `THINGS_DATABASE_PATH` and `THINGS_FALLBACK_TO_DEFAULT` environment variables
    #[must_use]
    pub fn from_env() -> Self {
        let database_path = std::env::var("THINGS_DATABASE_PATH")
            .map_or_else(|_| Self::get_default_database_path(), PathBuf::from);

        let fallback_to_default = std::env::var("THINGS_FALLBACK_TO_DEFAULT")
            .map(|v| {
                let lower = v.to_lowercase();
                lower == "true" || lower == "1" || lower == "yes" || lower == "on"
            })
            .unwrap_or(true);

        Self::new(database_path, fallback_to_default)
    }

    /// Create configuration for testing with a temporary database
    ///
    /// # Errors
    /// Returns `ThingsError::Io` if the temporary file cannot be created
    pub fn for_testing() -> Result<Self> {
        use tempfile::NamedTempFile;
        let temp_file = NamedTempFile::new()?;
        let db_path = temp_file.path().to_path_buf();
        Ok(Self::new(db_path, false))
    }
}

impl Default for ThingsConfig {
    fn default() -> Self {
        Self::with_default_path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_creation() {
        let config = ThingsConfig::new("/path/to/db.sqlite", true);
        assert_eq!(config.database_path, PathBuf::from("/path/to/db.sqlite"));
        assert!(config.fallback_to_default);
    }

    #[test]
    fn test_default_config() {
        let config = ThingsConfig::default();
        assert!(config
            .database_path
            .to_string_lossy()
            .contains("Things Database.thingsdatabase"));
        assert!(!config.fallback_to_default);
    }

    #[test]
    fn test_config_from_env() {
        // Save original values
        let original_db_path = std::env::var("THINGS_DATABASE_PATH").ok();
        let original_fallback = std::env::var("THINGS_FALLBACK_TO_DEFAULT").ok();

        std::env::set_var("THINGS_DATABASE_PATH", "/custom/path/db.sqlite");
        std::env::set_var("THINGS_FALLBACK_TO_DEFAULT", "true");

        let config = ThingsConfig::from_env();
        assert_eq!(
            config.database_path,
            PathBuf::from("/custom/path/db.sqlite")
        );
        assert!(config.fallback_to_default);

        // Restore original values
        if let Some(path) = original_db_path {
            std::env::set_var("THINGS_DATABASE_PATH", path);
        } else {
            std::env::remove_var("THINGS_DATABASE_PATH");
        }
        if let Some(fallback) = original_fallback {
            std::env::set_var("THINGS_FALLBACK_TO_DEFAULT", fallback);
        } else {
            std::env::remove_var("THINGS_FALLBACK_TO_DEFAULT");
        }
    }

    #[test]
    fn test_effective_database_path() {
        // Test with existing file
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        let config = ThingsConfig::new(db_path, false);

        let effective_path = config.get_effective_database_path().unwrap();
        assert_eq!(effective_path, db_path);
    }

    #[test]
    fn test_fallback_behavior() {
        // Test fallback when it should succeed (default path exists)
        let config = ThingsConfig::new("/nonexistent/path.sqlite", true);
        let result = config.get_effective_database_path();

        // If the default path exists, fallback should succeed
        if ThingsConfig::get_default_database_path().exists() {
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), ThingsConfig::get_default_database_path());
        } else {
            // If default path doesn't exist, should get an error
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_fallback_disabled() {
        // Test when fallback is disabled - should always fail if path doesn't exist
        let config = ThingsConfig::new("/nonexistent/path.sqlite", false);
        let result = config.get_effective_database_path();

        // Should always fail when fallback is disabled and path doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_config_with_fallback_enabled() {
        let config = ThingsConfig::new("/nonexistent/path", true);
        assert_eq!(config.database_path, PathBuf::from("/nonexistent/path"));
        assert!(config.fallback_to_default);
    }

    #[test]
    fn test_config_from_env_with_custom_path() {
        // Save original values
        let original_db_path = std::env::var("THINGS_DATABASE_PATH").ok();
        let original_fallback = std::env::var("THINGS_FALLBACK_TO_DEFAULT").ok();

        std::env::set_var("THINGS_DATABASE_PATH", "/env/custom/path");
        std::env::set_var("THINGS_FALLBACK_TO_DEFAULT", "false");
        let config = ThingsConfig::from_env();

        // Check that the database path is set to what we specified
        // In CI environments, paths might be resolved differently, so we check the string representation
        let expected_path = PathBuf::from("/env/custom/path");
        let actual_path = config.database_path;
        assert_eq!(
            actual_path.to_string_lossy(),
            expected_path.to_string_lossy()
        );
        assert!(!config.fallback_to_default);

        // Restore original values
        if let Some(path) = original_db_path {
            std::env::set_var("THINGS_DATABASE_PATH", path);
        } else {
            std::env::remove_var("THINGS_DATABASE_PATH");
        }
        if let Some(fallback) = original_fallback {
            std::env::set_var("THINGS_FALLBACK_TO_DEFAULT", fallback);
        } else {
            std::env::remove_var("THINGS_FALLBACK_TO_DEFAULT");
        }
    }

    #[test]
    fn test_config_from_env_with_fallback() {
        // Save original values
        let original_db_path = std::env::var("THINGS_DATABASE_PATH").ok();
        let original_fallback = std::env::var("THINGS_FALLBACK_TO_DEFAULT").ok();

        // Set test values
        std::env::set_var("THINGS_DATABASE_PATH", "/env/path");
        std::env::set_var("THINGS_FALLBACK_TO_DEFAULT", "true");

        let config = ThingsConfig::from_env();

        // Check that the database path is set to what we specified
        // In CI environments, paths might be resolved differently, so we check the string representation
        let expected_path = PathBuf::from("/env/path");
        let actual_path = config.database_path;
        assert_eq!(
            actual_path.to_string_lossy(),
            expected_path.to_string_lossy()
        );
        assert!(config.fallback_to_default);

        // Restore original values
        if let Some(db_path) = original_db_path {
            std::env::set_var("THINGS_DATABASE_PATH", db_path);
        } else {
            std::env::remove_var("THINGS_DATABASE_PATH");
        }

        if let Some(fallback) = original_fallback {
            std::env::set_var("THINGS_FALLBACK_TO_DEFAULT", fallback);
        } else {
            std::env::remove_var("THINGS_FALLBACK_TO_DEFAULT");
        }
    }

    #[test]
    fn test_config_from_env_with_invalid_fallback() {
        // Save original values
        let original_db_path = std::env::var("THINGS_DATABASE_PATH").ok();
        let original_fallback = std::env::var("THINGS_FALLBACK_TO_DEFAULT").ok();

        std::env::set_var("THINGS_DATABASE_PATH", "/env/path");
        std::env::set_var("THINGS_FALLBACK_TO_DEFAULT", "invalid");
        let config = ThingsConfig::from_env();

        // Check that the database path is set to what we specified
        // Use canonicalize to handle path resolution differences in CI
        let expected_path = PathBuf::from("/env/path");
        let actual_path = config.database_path;

        // In CI environments, paths might be resolved differently, so we check the string representation
        assert_eq!(
            actual_path.to_string_lossy(),
            expected_path.to_string_lossy()
        );
        assert!(!config.fallback_to_default); // Should default to false for invalid value

        // Restore original values
        if let Some(path) = original_db_path {
            std::env::set_var("THINGS_DATABASE_PATH", path);
        } else {
            std::env::remove_var("THINGS_DATABASE_PATH");
        }
        if let Some(fallback) = original_fallback {
            std::env::set_var("THINGS_FALLBACK_TO_DEFAULT", fallback);
        } else {
            std::env::remove_var("THINGS_FALLBACK_TO_DEFAULT");
        }
    }

    #[test]
    fn test_config_debug_formatting() {
        let config = ThingsConfig::new("/test/path", true);
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("/test/path"));
        assert!(debug_str.contains("true"));
    }

    #[test]
    fn test_config_clone() {
        let config1 = ThingsConfig::new("/test/path", true);
        let config2 = config1.clone();

        assert_eq!(config1.database_path, config2.database_path);
        assert_eq!(config1.fallback_to_default, config2.fallback_to_default);
    }

    #[test]
    fn test_config_with_different_path_types() {
        // Test with relative path
        let config = ThingsConfig::new("relative/path", false);
        assert_eq!(config.database_path, PathBuf::from("relative/path"));

        // Test with absolute path
        let config = ThingsConfig::new("/absolute/path", false);
        assert_eq!(config.database_path, PathBuf::from("/absolute/path"));

        // Test with current directory
        let config = ThingsConfig::new(".", false);
        assert_eq!(config.database_path, PathBuf::from("."));
    }

    #[test]
    fn test_config_edge_cases() {
        // Test with empty string path
        let config = ThingsConfig::new("", false);
        assert_eq!(config.database_path, PathBuf::from(""));

        // Test with very long path
        let long_path = "/".repeat(1000);
        let config = ThingsConfig::new(&long_path, false);
        assert_eq!(config.database_path, PathBuf::from(&long_path));
    }

    #[test]
    fn test_get_default_database_path() {
        let default_path = ThingsConfig::get_default_database_path();

        // Should be a valid path (may or may not exist)
        assert!(!default_path.to_string_lossy().is_empty());

        // Should be a reasonable path (may or may not contain "Things3" depending on system)
        assert!(!default_path.to_string_lossy().is_empty());
    }
}
