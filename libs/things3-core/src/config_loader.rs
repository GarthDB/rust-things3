//! Configuration Loader
//!
//! This module provides utilities for loading configuration from multiple sources
//! with proper precedence and validation.

use crate::error::{Result, ThingsError};
use crate::mcp_config::McpServerConfig;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Configuration loader that handles multiple sources with precedence
pub struct ConfigLoader {
    /// Base configuration
    base_config: McpServerConfig,
    /// Configuration file paths to try in order
    config_paths: Vec<PathBuf>,
    /// Whether to load from environment variables
    load_from_env: bool,
    /// Whether to validate the final configuration
    validate: bool,
}

impl ConfigLoader {
    /// Create a new configuration loader
    #[must_use]
    pub fn new() -> Self {
        Self {
            base_config: McpServerConfig::default(),
            config_paths: Self::get_default_config_paths(),
            load_from_env: true,
            validate: true,
        }
    }

    /// Set the base configuration
    #[must_use]
    pub fn with_base_config(mut self, config: McpServerConfig) -> Self {
        self.base_config = config;
        self
    }

    /// Add a configuration file path
    #[must_use]
    pub fn add_config_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config_paths.push(path.as_ref().to_path_buf());
        self
    }

    /// Set configuration file paths
    #[must_use]
    pub fn with_config_paths<P: AsRef<Path>>(mut self, paths: Vec<P>) -> Self {
        self.config_paths = paths
            .into_iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();
        self
    }

    /// Disable loading from environment variables
    #[must_use]
    pub fn without_env_loading(mut self) -> Self {
        self.load_from_env = false;
        self
    }

    /// Enable or disable loading from environment variables
    #[must_use]
    pub fn with_env_loading(mut self, enabled: bool) -> Self {
        self.load_from_env = enabled;
        self
    }

    /// Enable or disable configuration validation
    #[must_use]
    pub fn with_validation(mut self, enabled: bool) -> Self {
        self.validate = enabled;
        self
    }

    /// Load configuration from all sources
    ///
    /// # Errors
    /// Returns an error if configuration cannot be loaded or is invalid
    pub fn load(&self) -> Result<McpServerConfig> {
        let mut config = self.base_config.clone();
        info!("Starting configuration loading process");

        // Load from configuration files in order
        for path in &self.config_paths {
            if path.exists() {
                debug!("Loading configuration from file: {}", path.display());
                match McpServerConfig::from_file(path) {
                    Ok(file_config) => {
                        config.merge_with(&file_config);
                        info!("Successfully loaded configuration from: {}", path.display());
                    }
                    Err(e) => {
                        warn!(
                            "Failed to load configuration from {}: {}",
                            path.display(),
                            e
                        );
                        // Continue with other sources
                    }
                }
            } else {
                debug!("Configuration file not found: {}", path.display());
            }
        }

        // Load from environment variables (highest precedence)
        if self.load_from_env {
            debug!("Loading configuration from environment variables");
            match McpServerConfig::from_env() {
                Ok(env_config) => {
                    config.merge_with(&env_config);
                    info!("Successfully loaded configuration from environment variables");
                }
                Err(e) => {
                    warn!(
                        "Failed to load configuration from environment variables: {}",
                        e
                    );
                    // Continue with current config
                }
            }
        }

        // Validate the final configuration
        if self.validate {
            debug!("Validating final configuration");
            config.validate()?;
            info!("Configuration validation passed");
        }

        info!("Configuration loading completed successfully");
        Ok(config)
    }

    /// Get the default configuration file paths to try
    #[must_use]
    pub fn get_default_config_paths() -> Vec<PathBuf> {
        vec![
            // Current directory
            PathBuf::from("mcp-config.json"),
            PathBuf::from("mcp-config.yaml"),
            PathBuf::from("mcp-config.yml"),
            // User config directory
            Self::get_user_config_dir().join("mcp-config.json"),
            Self::get_user_config_dir().join("mcp-config.yaml"),
            Self::get_user_config_dir().join("mcp-config.yml"),
            // System config directory
            Self::get_system_config_dir().join("mcp-config.json"),
            Self::get_system_config_dir().join("mcp-config.yaml"),
            Self::get_system_config_dir().join("mcp-config.yml"),
        ]
    }

    /// Get the user configuration directory
    #[must_use]
    pub fn get_user_config_dir() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config").join("things3-mcp")
        } else if let Ok(userprofile) = std::env::var("USERPROFILE") {
            // Windows
            PathBuf::from(userprofile)
                .join("AppData")
                .join("Roaming")
                .join("things3-mcp")
        } else {
            // Fallback
            PathBuf::from("~/.config/things3-mcp")
        }
    }

    /// Get the system configuration directory
    #[must_use]
    pub fn get_system_config_dir() -> PathBuf {
        if cfg!(target_os = "macos") {
            PathBuf::from("/Library/Application Support/things3-mcp")
        } else if cfg!(target_os = "windows") {
            PathBuf::from("C:\\ProgramData\\things3-mcp")
        } else {
            // Linux and others
            PathBuf::from("/etc/things3-mcp")
        }
    }

    /// Create a sample configuration file
    ///
    /// # Arguments
    /// * `path` - Path to create the sample configuration file
    /// * `format` - Format to use ("json" or "yaml")
    ///
    /// # Errors
    /// Returns an error if the file cannot be created
    pub fn create_sample_config<P: AsRef<Path>>(path: P, format: &str) -> Result<()> {
        let config = McpServerConfig::default();
        config.to_file(path, format)?;
        Ok(())
    }

    /// Create all default configuration files with sample content
    ///
    /// # Errors
    /// Returns an error if any file cannot be created
    pub fn create_all_sample_configs() -> Result<()> {
        let config = McpServerConfig::default();

        // Create user config directory
        let user_config_dir = Self::get_user_config_dir();
        std::fs::create_dir_all(&user_config_dir).map_err(|e| {
            ThingsError::Io(std::io::Error::other(format!(
                "Failed to create user config directory: {e}"
            )))
        })?;

        // Create sample files
        let sample_files = vec![
            (user_config_dir.join("mcp-config.json"), "json"),
            (user_config_dir.join("mcp-config.yaml"), "yaml"),
            (PathBuf::from("mcp-config.json"), "json"),
            (PathBuf::from("mcp-config.yaml"), "yaml"),
        ];

        for (path, format) in sample_files {
            config.to_file(&path, format)?;
            info!("Created sample configuration file: {}", path.display());
        }

        Ok(())
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick configuration loader that uses sensible defaults
///
/// # Errors
/// Returns an error if configuration cannot be loaded
pub fn load_config() -> Result<McpServerConfig> {
    ConfigLoader::new().load()
}

/// Load configuration with custom paths
///
/// # Arguments
/// * `config_paths` - Paths to configuration files to try
///
/// # Errors
/// Returns an error if configuration cannot be loaded
pub fn load_config_with_paths<P: AsRef<Path>>(config_paths: Vec<P>) -> Result<McpServerConfig> {
    ConfigLoader::new().with_config_paths(config_paths).load()
}

/// Load configuration from environment variables only
///
/// # Errors
/// Returns an error if configuration cannot be loaded
pub fn load_config_from_env() -> Result<McpServerConfig> {
    ConfigLoader::new()
        .with_config_paths::<String>(vec![])
        .load()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_loader_default() {
        let loader = ConfigLoader::new();
        assert!(loader.load_from_env);
        assert!(loader.validate);
        assert!(!loader.config_paths.is_empty());
    }

    #[test]
    fn test_config_loader_with_base_config() {
        let mut base_config = McpServerConfig::default();
        base_config.server.name = "test-server".to_string();

        let loader = ConfigLoader::new()
            .with_base_config(base_config.clone())
            .without_env_loading();
        let loaded_config = loader.load().unwrap();
        assert_eq!(loaded_config.server.name, "test-server");
    }

    #[test]
    fn test_config_loader_with_custom_paths() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test-config.json");

        // Create a test configuration file
        let mut test_config = McpServerConfig::default();
        test_config.server.name = "file-server".to_string();
        test_config.to_file(&config_file, "json").unwrap();

        let loader = ConfigLoader::new()
            .with_config_paths(vec![&config_file])
            .with_env_loading(false);

        let loaded_config = loader.load().unwrap();
        assert_eq!(loaded_config.server.name, "file-server");
    }

    #[test]
    fn test_config_loader_precedence() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test-config.json");

        // Create a test configuration file
        let mut file_config = McpServerConfig::default();
        file_config.server.name = "file-server".to_string();
        file_config.to_file(&config_file, "json").unwrap();

        // Set environment variable
        std::env::set_var("MCP_SERVER_NAME", "env-server");

        let loader = ConfigLoader::new().with_config_paths(vec![&config_file]);

        let loaded_config = loader.load().unwrap();
        // Environment should take precedence
        assert_eq!(loaded_config.server.name, "env-server");

        // Clean up
        std::env::remove_var("MCP_SERVER_NAME");
    }

    #[test]
    fn test_get_default_config_paths() {
        let paths = ConfigLoader::get_default_config_paths();
        assert!(!paths.is_empty());
        assert!(paths
            .iter()
            .any(|p| p.file_name().unwrap() == "mcp-config.json"));
        assert!(paths
            .iter()
            .any(|p| p.file_name().unwrap() == "mcp-config.yaml"));
    }

    #[test]
    fn test_get_user_config_dir() {
        let user_dir = ConfigLoader::get_user_config_dir();
        assert!(user_dir.to_string_lossy().contains("things3-mcp"));
    }

    #[test]
    fn test_get_system_config_dir() {
        let system_dir = ConfigLoader::get_system_config_dir();
        assert!(system_dir.to_string_lossy().contains("things3-mcp"));
    }

    #[test]
    fn test_create_sample_config() {
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("sample.json");
        let yaml_file = temp_dir.path().join("sample.yaml");

        ConfigLoader::create_sample_config(&json_file, "json").unwrap();
        ConfigLoader::create_sample_config(&yaml_file, "yaml").unwrap();

        assert!(json_file.exists());
        assert!(yaml_file.exists());
    }

    #[test]
    fn test_load_config() {
        let config = load_config().unwrap();
        assert!(!config.server.name.is_empty());
    }

    #[test]
    fn test_load_config_from_env() {
        std::env::set_var("MCP_SERVER_NAME", "env-test");
        let config = load_config_from_env().unwrap();
        assert_eq!(config.server.name, "env-test");
        std::env::remove_var("MCP_SERVER_NAME");
    }
}
