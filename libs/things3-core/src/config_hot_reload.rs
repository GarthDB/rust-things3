//! Configuration Hot Reloading
//!
//! This module provides functionality for hot-reloading configuration files
//! without restarting the server.

use crate::error::{Result, ThingsError};
use crate::mcp_config::McpServerConfig;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info};

/// Configuration hot reloader
pub struct ConfigHotReloader {
    /// Current configuration
    config: Arc<RwLock<McpServerConfig>>,
    /// Configuration file path
    config_path: PathBuf,
    /// Reload interval
    reload_interval: Duration,
    /// Whether hot reloading is enabled
    enabled: bool,
    /// Broadcast channel for configuration change notifications
    change_tx: broadcast::Sender<McpServerConfig>,
    /// Last modification time of the config file
    last_modified: Option<std::time::SystemTime>,
}

impl ConfigHotReloader {
    /// Create a new configuration hot reloader
    ///
    /// # Arguments
    /// * `config` - Initial configuration
    /// * `config_path` - Path to the configuration file to watch
    /// * `reload_interval` - How often to check for changes
    ///
    /// # Errors
    /// Returns an error if the configuration file cannot be accessed
    pub fn new(
        config: McpServerConfig,
        config_path: PathBuf,
        reload_interval: Duration,
    ) -> Result<Self> {
        // Validate that the config file exists and is readable
        if !config_path.exists() {
            return Err(ThingsError::configuration(format!(
                "Configuration file does not exist: {}",
                config_path.display()
            )));
        }

        let (change_tx, _) = broadcast::channel(16);
        let last_modified = Self::get_file_modified_time(&config_path)?;

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
            reload_interval,
            enabled: true,
            change_tx,
            last_modified: Some(last_modified),
        })
    }

    /// Create a hot reloader with default settings
    ///
    /// # Arguments
    /// * `config_path` - Path to the configuration file to watch
    ///
    /// # Errors
    /// Returns an error if the configuration file cannot be accessed
    pub fn with_default_settings(config_path: PathBuf) -> Result<Self> {
        let config = McpServerConfig::default();
        Self::new(config, config_path, Duration::from_secs(5))
    }

    /// Get the current configuration
    #[must_use]
    pub async fn get_config(&self) -> McpServerConfig {
        self.config.read().await.clone()
    }

    /// Update the configuration
    ///
    /// # Arguments
    /// * `new_config` - New configuration to set
    ///
    /// # Errors
    /// Returns an error if the configuration is invalid
    pub async fn update_config(&self, new_config: McpServerConfig) -> Result<()> {
        new_config.validate()?;

        let mut config = self.config.write().await;
        *config = new_config.clone();

        // Broadcast the change
        let _ = self.change_tx.send(new_config);

        info!("Configuration updated successfully");
        Ok(())
    }

    /// Get a receiver for configuration change notifications
    #[must_use]
    pub fn subscribe_to_changes(&self) -> broadcast::Receiver<McpServerConfig> {
        self.change_tx.subscribe()
    }

    /// Enable or disable hot reloading
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            info!("Configuration hot reloading enabled");
        } else {
            info!("Configuration hot reloading disabled");
        }
    }

    /// Check if hot reloading is enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Start the hot reloader task
    ///
    /// This will spawn a background task that periodically checks for configuration changes
    /// and reloads the configuration if changes are detected.
    pub async fn start(&self) -> Result<()> {
        if !self.enabled {
            debug!("Hot reloading is disabled, not starting reloader task");
            return Ok(());
        }

        let config = Arc::clone(&self.config);
        let config_path = self.config_path.clone();
        let change_tx = self.change_tx.clone();
        let mut interval = interval(self.reload_interval);
        let mut last_modified = self.last_modified;

        info!(
            "Starting configuration hot reloader for: {}",
            config_path.display()
        );

        tokio::spawn(async move {
            loop {
                interval.tick().await;

                match Self::check_and_reload_config(
                    &config_path,
                    &config,
                    &change_tx,
                    &mut last_modified,
                )
                .await
                {
                    Ok(reloaded) => {
                        if reloaded {
                            debug!(
                                "Configuration reloaded from file: {}",
                                config_path.display()
                            );
                        }
                    }
                    Err(e) => {
                        error!("Failed to check/reload configuration: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Check for configuration changes and reload if necessary
    async fn check_and_reload_config(
        config_path: &PathBuf,
        config: &Arc<RwLock<McpServerConfig>>,
        change_tx: &broadcast::Sender<McpServerConfig>,
        last_modified: &mut Option<std::time::SystemTime>,
    ) -> Result<bool> {
        // Check if the file has been modified
        let current_modified = Self::get_file_modified_time(config_path)?;

        if let Some(last) = *last_modified {
            if current_modified <= last {
                return Ok(false); // No changes
            }
        }

        // File has been modified, try to reload
        debug!("Configuration file modified, attempting to reload");

        match McpServerConfig::from_file(config_path) {
            Ok(new_config) => {
                // Validate the new configuration
                new_config.validate()?;

                // Update the configuration
                {
                    let mut current_config = config.write().await;
                    *current_config = new_config.clone();
                }

                // Broadcast the change
                let _ = change_tx.send(new_config);

                // Update the last modified time
                *last_modified = Some(current_modified);

                info!(
                    "Configuration successfully reloaded from: {}",
                    config_path.display()
                );
                Ok(true)
            }
            Err(e) => {
                error!(
                    "Failed to reload configuration from {}: {}",
                    config_path.display(),
                    e
                );
                Err(e)
            }
        }
    }

    /// Get the last modification time of a file
    fn get_file_modified_time(path: &PathBuf) -> Result<std::time::SystemTime> {
        let metadata = std::fs::metadata(path).map_err(|e| {
            ThingsError::Io(std::io::Error::other(format!(
                "Failed to get file metadata for {}: {}",
                path.display(),
                e
            )))
        })?;

        metadata.modified().map_err(|e| {
            ThingsError::Io(std::io::Error::other(format!(
                "Failed to get modification time for {}: {}",
                path.display(),
                e
            )))
        })
    }

    /// Manually trigger a configuration reload
    ///
    /// # Errors
    /// Returns an error if the configuration cannot be reloaded
    pub async fn reload_now(&self) -> Result<bool> {
        let mut last_modified = self.last_modified;
        Self::check_and_reload_config(
            &self.config_path,
            &self.config,
            &self.change_tx,
            &mut last_modified,
        )
        .await
    }

    /// Get the configuration file path being watched
    #[must_use]
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// Get the reload interval
    #[must_use]
    pub fn reload_interval(&self) -> Duration {
        self.reload_interval
    }

    /// Set the reload interval
    pub fn set_reload_interval(&mut self, interval: Duration) {
        self.reload_interval = interval;
        debug!("Configuration reload interval set to: {:?}", interval);
    }
}

/// Configuration change handler trait
#[async_trait::async_trait]
pub trait ConfigChangeHandler: Send + Sync {
    /// Handle a configuration change
    ///
    /// # Arguments
    /// * `old_config` - The previous configuration
    /// * `new_config` - The new configuration
    async fn handle_config_change(
        &self,
        old_config: &McpServerConfig,
        new_config: &McpServerConfig,
    );
}

/// Default configuration change handler that logs changes
pub struct DefaultConfigChangeHandler;

#[async_trait::async_trait]
impl ConfigChangeHandler for DefaultConfigChangeHandler {
    async fn handle_config_change(
        &self,
        old_config: &McpServerConfig,
        new_config: &McpServerConfig,
    ) {
        info!("Configuration changed:");

        if old_config.server.name != new_config.server.name {
            info!(
                "  Server name: {} -> {}",
                old_config.server.name, new_config.server.name
            );
        }
        if old_config.logging.level != new_config.logging.level {
            info!(
                "  Log level: {} -> {}",
                old_config.logging.level, new_config.logging.level
            );
        }
        if old_config.cache.enabled != new_config.cache.enabled {
            info!(
                "  Cache enabled: {} -> {}",
                old_config.cache.enabled, new_config.cache.enabled
            );
        }
        if old_config.performance.enabled != new_config.performance.enabled {
            info!(
                "  Performance monitoring: {} -> {}",
                old_config.performance.enabled, new_config.performance.enabled
            );
        }
        if old_config.security.authentication.enabled != new_config.security.authentication.enabled
        {
            info!(
                "  Authentication: {} -> {}",
                old_config.security.authentication.enabled,
                new_config.security.authentication.enabled
            );
        }
    }
}

/// Configuration hot reloader with change handler
pub struct ConfigHotReloaderWithHandler {
    /// The base hot reloader
    reloader: ConfigHotReloader,
    /// Change handler
    handler: Arc<dyn ConfigChangeHandler>,
}

impl ConfigHotReloaderWithHandler {
    /// Create a new hot reloader with a change handler
    ///
    /// # Arguments
    /// * `config` - Initial configuration
    /// * `config_path` - Path to the configuration file to watch
    /// * `reload_interval` - How often to check for changes
    /// * `handler` - Handler for configuration changes
    ///
    /// # Errors
    /// Returns an error if the configuration file cannot be accessed
    pub fn new(
        config: McpServerConfig,
        config_path: PathBuf,
        reload_interval: Duration,
        handler: Arc<dyn ConfigChangeHandler>,
    ) -> Result<Self> {
        let reloader = ConfigHotReloader::new(config, config_path, reload_interval)?;

        Ok(Self { reloader, handler })
    }

    /// Start the hot reloader with change handling
    ///
    /// # Errors
    /// Returns an error if the hot reloader cannot be started
    pub async fn start_with_handler(&self) -> Result<()> {
        // Start the base reloader
        self.reloader.start().await?;

        // Start the change handler task
        let mut change_rx = self.reloader.subscribe_to_changes();
        let handler = Arc::clone(&self.handler);
        let config = Arc::clone(&self.reloader.config);

        tokio::spawn(async move {
            let mut old_config = config.read().await.clone();

            while let Ok(new_config) = change_rx.recv().await {
                handler.handle_config_change(&old_config, &new_config).await;
                old_config = new_config;
            }
        });

        Ok(())
    }

    /// Get the underlying hot reloader
    #[must_use]
    pub fn reloader(&self) -> &ConfigHotReloader {
        &self.reloader
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_config_hot_reloader_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().with_extension("json");

        let config = McpServerConfig::default();
        config.to_file(&config_path, "json").unwrap();

        let reloader = ConfigHotReloader::new(config, config_path, Duration::from_secs(1)).unwrap();
        assert!(reloader.is_enabled());
    }

    #[tokio::test]
    async fn test_config_hot_reloader_with_default_settings() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().with_extension("json");

        let config = McpServerConfig::default();
        config.to_file(&config_path, "json").unwrap();

        let reloader = ConfigHotReloader::with_default_settings(config_path).unwrap();
        assert!(reloader.is_enabled());
    }

    #[tokio::test]
    async fn test_config_hot_reloader_enable_disable() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().with_extension("json");

        let config = McpServerConfig::default();
        config.to_file(&config_path, "json").unwrap();

        let mut reloader =
            ConfigHotReloader::new(config, config_path, Duration::from_secs(1)).unwrap();
        assert!(reloader.is_enabled());

        reloader.set_enabled(false);
        assert!(!reloader.is_enabled());

        reloader.set_enabled(true);
        assert!(reloader.is_enabled());
    }

    #[tokio::test]
    async fn test_config_hot_reloader_get_config() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().with_extension("json");

        let mut config = McpServerConfig::default();
        config.server.name = "test-server".to_string();
        config.to_file(&config_path, "json").unwrap();

        let reloader = ConfigHotReloader::new(config, config_path, Duration::from_secs(1)).unwrap();
        let loaded_config = reloader.get_config().await;
        assert_eq!(loaded_config.server.name, "test-server");
    }

    #[tokio::test]
    async fn test_config_hot_reloader_update_config() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().with_extension("json");

        let config = McpServerConfig::default();
        config.to_file(&config_path, "json").unwrap();

        let reloader = ConfigHotReloader::new(config, config_path, Duration::from_secs(1)).unwrap();

        let mut new_config = McpServerConfig::default();
        new_config.server.name = "updated-server".to_string();

        reloader.update_config(new_config).await.unwrap();

        let loaded_config = reloader.get_config().await;
        assert_eq!(loaded_config.server.name, "updated-server");
    }

    #[tokio::test]
    async fn test_config_hot_reloader_subscribe_to_changes() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().with_extension("json");

        let config = McpServerConfig::default();
        config.to_file(&config_path, "json").unwrap();

        let reloader = ConfigHotReloader::new(config, config_path, Duration::from_secs(1)).unwrap();
        let mut change_rx = reloader.subscribe_to_changes();

        let mut new_config = McpServerConfig::default();
        new_config.server.name = "changed-server".to_string();

        reloader.update_config(new_config).await.unwrap();

        let received_config = change_rx.recv().await.unwrap();
        assert_eq!(received_config.server.name, "changed-server");
    }

    #[tokio::test]
    async fn test_config_hot_reloader_with_handler() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().with_extension("json");

        let config = McpServerConfig::default();
        config.to_file(&config_path, "json").unwrap();

        let handler = Arc::new(DefaultConfigChangeHandler);
        let reloader =
            ConfigHotReloaderWithHandler::new(config, config_path, Duration::from_secs(1), handler)
                .unwrap();

        assert!(reloader.reloader().is_enabled());
    }
}
