//! MCP Server Configuration Management
//!
//! This module provides comprehensive configuration management for the MCP server,
//! including support for environment variables, configuration files, and validation.

use crate::error::{Result, ThingsError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Comprehensive configuration for the MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Performance configuration
    pub performance: PerformanceConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Cache configuration
    pub cache: CacheConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    /// Feature flags
    pub features: FeatureFlags,
}

/// Server-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
    /// Server description
    pub description: String,
    /// Maximum concurrent connections
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Request timeout in seconds
    pub request_timeout: u64,
    /// Enable graceful shutdown
    pub graceful_shutdown: bool,
    /// Shutdown timeout in seconds
    pub shutdown_timeout: u64,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database path
    pub path: PathBuf,
    /// Fallback to default path if specified path doesn't exist
    pub fallback_to_default: bool,
    /// Connection pool size
    pub pool_size: u32,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Query timeout in seconds
    pub query_timeout: u64,
    /// Enable query logging
    pub enable_query_logging: bool,
    /// Enable query metrics
    pub enable_query_metrics: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Enable JSON logging
    pub json_logs: bool,
    /// Log file path (optional)
    pub log_file: Option<PathBuf>,
    /// Enable console logging
    pub console_logs: bool,
    /// Enable structured logging
    pub structured_logs: bool,
    /// Log rotation configuration
    pub rotation: LogRotationConfig,
}

/// Log rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    /// Enable log rotation
    pub enabled: bool,
    /// Maximum file size in MB
    pub max_file_size_mb: u64,
    /// Maximum number of files to keep
    pub max_files: u32,
    /// Compression enabled
    pub compress: bool,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable performance monitoring
    pub enabled: bool,
    /// Slow request threshold in milliseconds
    pub slow_request_threshold_ms: u64,
    /// Enable request profiling
    pub enable_profiling: bool,
    /// Memory usage monitoring
    pub memory_monitoring: MemoryMonitoringConfig,
    /// CPU usage monitoring
    pub cpu_monitoring: CpuMonitoringConfig,
}

/// Memory monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMonitoringConfig {
    /// Enable memory monitoring
    pub enabled: bool,
    /// Memory usage threshold percentage
    pub threshold_percentage: f64,
    /// Check interval in seconds
    pub check_interval: u64,
}

/// CPU monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMonitoringConfig {
    /// Enable CPU monitoring
    pub enabled: bool,
    /// CPU usage threshold percentage
    pub threshold_percentage: f64,
    /// Check interval in seconds
    pub check_interval: u64,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Authentication configuration
    pub authentication: AuthenticationConfig,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitingConfig,
    /// CORS configuration
    pub cors: CorsConfig,
    /// Input validation configuration
    pub validation: ValidationConfig,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// Enable authentication
    pub enabled: bool,
    /// Require authentication for all requests
    pub require_auth: bool,
    /// JWT secret key
    pub jwt_secret: String,
    /// JWT expiration time in seconds
    pub jwt_expiration: u64,
    /// API keys configuration
    pub api_keys: Vec<ApiKeyConfig>,
    /// OAuth 2.0 configuration
    pub oauth: Option<OAuth2Config>,
}

/// API key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// API key value
    pub key: String,
    /// Key identifier
    pub key_id: String,
    /// Permissions for this key
    pub permissions: Vec<String>,
    /// Optional expiration date
    pub expires_at: Option<String>,
}

/// OAuth 2.0 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Config {
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret
    pub client_secret: String,
    /// Token endpoint URL
    pub token_endpoint: String,
    /// Required scopes
    pub scopes: Vec<String>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Requests per minute limit
    pub requests_per_minute: u32,
    /// Burst limit for short bursts
    pub burst_limit: u32,
    /// Custom limits per client type
    pub custom_limits: Option<HashMap<String, u32>>,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Enable CORS
    pub enabled: bool,
    /// Allowed origins
    pub allowed_origins: Vec<String>,
    /// Allowed methods
    pub allowed_methods: Vec<String>,
    /// Allowed headers
    pub allowed_headers: Vec<String>,
    /// Exposed headers
    pub exposed_headers: Vec<String>,
    /// Allow credentials
    pub allow_credentials: bool,
    /// Max age in seconds
    pub max_age: u64,
}

/// Input validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Enable input validation
    pub enabled: bool,
    /// Use strict validation mode
    pub strict_mode: bool,
    /// Maximum request size in bytes
    pub max_request_size: u64,
    /// Maximum field length
    pub max_field_length: usize,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    pub enabled: bool,
    /// Cache type (memory, disk, hybrid)
    pub cache_type: String,
    /// Maximum cache size in MB
    pub max_size_mb: u64,
    /// Cache TTL in seconds
    pub ttl_seconds: u64,
    /// Enable cache compression
    pub compression: bool,
    /// Cache eviction policy
    pub eviction_policy: String,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable monitoring
    pub enabled: bool,
    /// Metrics port
    pub metrics_port: u16,
    /// Health check port
    pub health_port: u16,
    /// Enable health checks
    pub health_checks: bool,
    /// Enable metrics collection
    pub metrics_collection: bool,
    /// Metrics endpoint path
    pub metrics_path: String,
    /// Health endpoint path
    pub health_path: String,
}

/// Feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable real-time updates
    pub real_time_updates: bool,
    /// Enable WebSocket server
    pub websocket_server: bool,
    /// Enable dashboard
    pub dashboard: bool,
    /// Enable bulk operations
    pub bulk_operations: bool,
    /// Enable data export
    pub data_export: bool,
    /// Enable backup functionality
    pub backup: bool,
    /// Enable hot reloading
    pub hot_reloading: bool,
}

impl McpServerConfig {
    /// Create a new MCP server configuration with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create configuration from environment variables
    ///
    /// # Errors
    /// Returns an error if environment variables contain invalid values
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        // Server configuration
        if let Ok(name) = std::env::var("MCP_SERVER_NAME") {
            config.server.name = name;
        }
        if let Ok(version) = std::env::var("MCP_SERVER_VERSION") {
            config.server.version = version;
        }
        if let Ok(description) = std::env::var("MCP_SERVER_DESCRIPTION") {
            config.server.description = description;
        }
        if let Ok(max_connections) = std::env::var("MCP_MAX_CONNECTIONS") {
            config.server.max_connections = max_connections
                .parse()
                .map_err(|_| ThingsError::configuration("Invalid MCP_MAX_CONNECTIONS value"))?;
        }
        if let Ok(connection_timeout) = std::env::var("MCP_CONNECTION_TIMEOUT") {
            config.server.connection_timeout = connection_timeout
                .parse()
                .map_err(|_| ThingsError::configuration("Invalid MCP_CONNECTION_TIMEOUT value"))?;
        }
        if let Ok(request_timeout) = std::env::var("MCP_REQUEST_TIMEOUT") {
            config.server.request_timeout = request_timeout
                .parse()
                .map_err(|_| ThingsError::configuration("Invalid MCP_REQUEST_TIMEOUT value"))?;
        }

        // Database configuration
        if let Ok(db_path) = std::env::var("MCP_DATABASE_PATH") {
            config.database.path = PathBuf::from(db_path);
        }
        if let Ok(fallback) = std::env::var("MCP_DATABASE_FALLBACK") {
            config.database.fallback_to_default = parse_bool(&fallback);
        }
        if let Ok(pool_size) = std::env::var("MCP_DATABASE_POOL_SIZE") {
            config.database.pool_size = pool_size
                .parse()
                .map_err(|_| ThingsError::configuration("Invalid MCP_DATABASE_POOL_SIZE value"))?;
        }

        // Logging configuration
        if let Ok(level) = std::env::var("MCP_LOG_LEVEL") {
            config.logging.level = level;
        }
        if let Ok(json_logs) = std::env::var("MCP_JSON_LOGS") {
            config.logging.json_logs = parse_bool(&json_logs);
        }
        if let Ok(log_file) = std::env::var("MCP_LOG_FILE") {
            config.logging.log_file = Some(PathBuf::from(log_file));
        }
        if let Ok(console_logs) = std::env::var("MCP_CONSOLE_LOGS") {
            config.logging.console_logs = parse_bool(&console_logs);
        }

        // Performance configuration
        if let Ok(enabled) = std::env::var("MCP_PERFORMANCE_ENABLED") {
            config.performance.enabled = parse_bool(&enabled);
        }
        if let Ok(threshold) = std::env::var("MCP_SLOW_REQUEST_THRESHOLD") {
            config.performance.slow_request_threshold_ms = threshold.parse().map_err(|_| {
                ThingsError::configuration("Invalid MCP_SLOW_REQUEST_THRESHOLD value")
            })?;
        }

        // Security configuration
        if let Ok(auth_enabled) = std::env::var("MCP_AUTH_ENABLED") {
            config.security.authentication.enabled = parse_bool(&auth_enabled);
        }
        if let Ok(jwt_secret) = std::env::var("MCP_JWT_SECRET") {
            config.security.authentication.jwt_secret = jwt_secret;
        }
        if let Ok(rate_limit_enabled) = std::env::var("MCP_RATE_LIMIT_ENABLED") {
            config.security.rate_limiting.enabled = parse_bool(&rate_limit_enabled);
        }
        if let Ok(requests_per_minute) = std::env::var("MCP_REQUESTS_PER_MINUTE") {
            config.security.rate_limiting.requests_per_minute = requests_per_minute
                .parse()
                .map_err(|_| ThingsError::configuration("Invalid MCP_REQUESTS_PER_MINUTE value"))?;
        }

        // Cache configuration
        if let Ok(cache_enabled) = std::env::var("MCP_CACHE_ENABLED") {
            config.cache.enabled = parse_bool(&cache_enabled);
        }
        if let Ok(cache_type) = std::env::var("MCP_CACHE_TYPE") {
            config.cache.cache_type = cache_type;
        }
        if let Ok(max_size) = std::env::var("MCP_CACHE_MAX_SIZE_MB") {
            config.cache.max_size_mb = max_size
                .parse()
                .map_err(|_| ThingsError::configuration("Invalid MCP_CACHE_MAX_SIZE_MB value"))?;
        }

        // Monitoring configuration
        if let Ok(monitoring_enabled) = std::env::var("MCP_MONITORING_ENABLED") {
            config.monitoring.enabled = parse_bool(&monitoring_enabled);
        }
        if let Ok(metrics_port) = std::env::var("MCP_METRICS_PORT") {
            config.monitoring.metrics_port = metrics_port
                .parse()
                .map_err(|_| ThingsError::configuration("Invalid MCP_METRICS_PORT value"))?;
        }
        if let Ok(health_port) = std::env::var("MCP_HEALTH_PORT") {
            config.monitoring.health_port = health_port
                .parse()
                .map_err(|_| ThingsError::configuration("Invalid MCP_HEALTH_PORT value"))?;
        }

        // Feature flags
        if let Ok(real_time) = std::env::var("MCP_REAL_TIME_UPDATES") {
            config.features.real_time_updates = parse_bool(&real_time);
        }
        if let Ok(websocket) = std::env::var("MCP_WEBSOCKET_SERVER") {
            config.features.websocket_server = parse_bool(&websocket);
        }
        if let Ok(dashboard) = std::env::var("MCP_DASHBOARD") {
            config.features.dashboard = parse_bool(&dashboard);
        }
        if let Ok(bulk_ops) = std::env::var("MCP_BULK_OPERATIONS") {
            config.features.bulk_operations = parse_bool(&bulk_ops);
        }
        if let Ok(data_export) = std::env::var("MCP_DATA_EXPORT") {
            config.features.data_export = parse_bool(&data_export);
        }
        if let Ok(backup) = std::env::var("MCP_BACKUP") {
            config.features.backup = parse_bool(&backup);
        }
        if let Ok(hot_reload) = std::env::var("MCP_HOT_RELOADING") {
            config.features.hot_reloading = parse_bool(&hot_reload);
        }

        Ok(config)
    }

    /// Load configuration from a file
    ///
    /// # Arguments
    /// * `path` - Path to the configuration file
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            ThingsError::Io(std::io::Error::other(format!(
                "Failed to read config file {}: {}",
                path.display(),
                e
            )))
        })?;

        let config = if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            serde_yaml::from_str(&content).map_err(|e| {
                ThingsError::configuration(format!("Failed to parse YAML config: {}", e))
            })?
        } else {
            serde_json::from_str(&content).map_err(|e| {
                ThingsError::configuration(format!("Failed to parse JSON config: {}", e))
            })?
        };

        Ok(config)
    }

    /// Save configuration to a file
    ///
    /// # Arguments
    /// * `path` - Path to save the configuration file
    /// * `format` - Format to save as ("json" or "yaml")
    ///
    /// # Errors
    /// Returns an error if the file cannot be written
    pub fn to_file<P: AsRef<Path>>(&self, path: P, format: &str) -> Result<()> {
        let path = path.as_ref();
        let content = match format {
            "yaml" | "yml" => serde_yaml::to_string(self).map_err(|e| {
                ThingsError::configuration(format!("Failed to serialize YAML: {}", e))
            })?,
            "json" => serde_json::to_string_pretty(self).map_err(|e| {
                ThingsError::configuration(format!("Failed to serialize JSON: {}", e))
            })?,
            _ => {
                return Err(ThingsError::configuration(format!(
                    "Unsupported format: {}",
                    format
                )))
            }
        };

        std::fs::write(path, content).map_err(|e| {
            ThingsError::Io(std::io::Error::other(format!(
                "Failed to write config file {}: {}",
                path.display(),
                e
            )))
        })?;

        Ok(())
    }

    /// Validate the configuration
    ///
    /// # Errors
    /// Returns an error if the configuration is invalid
    pub fn validate(&self) -> Result<()> {
        // Validate server configuration
        if self.server.name.is_empty() {
            return Err(ThingsError::configuration("Server name cannot be empty"));
        }
        if self.server.version.is_empty() {
            return Err(ThingsError::configuration("Server version cannot be empty"));
        }
        if self.server.max_connections == 0 {
            return Err(ThingsError::configuration(
                "Max connections must be greater than 0",
            ));
        }

        // Validate database configuration
        if self.database.pool_size == 0 {
            return Err(ThingsError::configuration(
                "Database pool size must be greater than 0",
            ));
        }

        // Validate logging configuration
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(ThingsError::configuration(format!(
                "Invalid log level: {}. Must be one of: {}",
                self.logging.level,
                valid_levels.join(", ")
            )));
        }

        // Validate performance configuration
        if self.performance.enabled && self.performance.slow_request_threshold_ms == 0 {
            return Err(ThingsError::configuration("Slow request threshold must be greater than 0 when performance monitoring is enabled"));
        }

        // Validate security configuration
        if self.security.authentication.enabled
            && self.security.authentication.jwt_secret.is_empty()
        {
            return Err(ThingsError::configuration(
                "JWT secret cannot be empty when authentication is enabled",
            ));
        }

        // Validate cache configuration
        if self.cache.enabled && self.cache.max_size_mb == 0 {
            return Err(ThingsError::configuration(
                "Cache max size must be greater than 0 when caching is enabled",
            ));
        }

        // Validate monitoring configuration
        if self.monitoring.enabled && self.monitoring.metrics_port == 0 {
            return Err(ThingsError::configuration(
                "Metrics port must be greater than 0 when monitoring is enabled",
            ));
        }
        if self.monitoring.enabled && self.monitoring.health_port == 0 {
            return Err(ThingsError::configuration(
                "Health port must be greater than 0 when monitoring is enabled",
            ));
        }

        Ok(())
    }

    /// Merge with another configuration, with the other config taking precedence
    pub fn merge_with(&mut self, other: &McpServerConfig) {
        // Merge server config
        if !other.server.name.is_empty() {
            self.server.name = other.server.name.clone();
        }
        if !other.server.version.is_empty() {
            self.server.version = other.server.version.clone();
        }
        if !other.server.description.is_empty() {
            self.server.description = other.server.description.clone();
        }
        if other.server.max_connections > 0 {
            self.server.max_connections = other.server.max_connections;
        }
        if other.server.connection_timeout > 0 {
            self.server.connection_timeout = other.server.connection_timeout;
        }
        if other.server.request_timeout > 0 {
            self.server.request_timeout = other.server.request_timeout;
        }

        // Merge database config
        if other.database.path != PathBuf::new() {
            self.database.path = other.database.path.clone();
        }
        if other.database.pool_size > 0 {
            self.database.pool_size = other.database.pool_size;
        }

        // Merge logging config
        if !other.logging.level.is_empty() {
            self.logging.level = other.logging.level.clone();
        }
        if other.logging.log_file.is_some() {
            self.logging.log_file = other.logging.log_file.clone();
        }

        // Merge performance config
        if other.performance.enabled {
            self.performance.enabled = other.performance.enabled;
        }
        if other.performance.slow_request_threshold_ms > 0 {
            self.performance.slow_request_threshold_ms =
                other.performance.slow_request_threshold_ms;
        }

        // Merge security config
        if other.security.authentication.enabled {
            self.security.authentication.enabled = other.security.authentication.enabled;
        }
        if !other.security.authentication.jwt_secret.is_empty() {
            self.security.authentication.jwt_secret =
                other.security.authentication.jwt_secret.clone();
        }
        if other.security.rate_limiting.enabled {
            self.security.rate_limiting.enabled = other.security.rate_limiting.enabled;
        }
        if other.security.rate_limiting.requests_per_minute > 0 {
            self.security.rate_limiting.requests_per_minute =
                other.security.rate_limiting.requests_per_minute;
        }

        // Merge cache config
        if other.cache.enabled {
            self.cache.enabled = other.cache.enabled;
        }
        if other.cache.max_size_mb > 0 {
            self.cache.max_size_mb = other.cache.max_size_mb;
        }

        // Merge monitoring config
        if other.monitoring.enabled {
            self.monitoring.enabled = other.monitoring.enabled;
        }
        if other.monitoring.metrics_port > 0 {
            self.monitoring.metrics_port = other.monitoring.metrics_port;
        }
        if other.monitoring.health_port > 0 {
            self.monitoring.health_port = other.monitoring.health_port;
        }

        // Merge feature flags
        if other.features.real_time_updates {
            self.features.real_time_updates = other.features.real_time_updates;
        }
        if other.features.websocket_server {
            self.features.websocket_server = other.features.websocket_server;
        }
        if other.features.dashboard {
            self.features.dashboard = other.features.dashboard;
        }
        if other.features.bulk_operations {
            self.features.bulk_operations = other.features.bulk_operations;
        }
        if other.features.data_export {
            self.features.data_export = other.features.data_export;
        }
        if other.features.backup {
            self.features.backup = other.features.backup;
        }
        if other.features.hot_reloading {
            self.features.hot_reloading = other.features.hot_reloading;
        }
    }

    /// Get the effective database path, falling back to default if needed
    ///
    /// # Errors
    /// Returns an error if neither the specified path nor the default path exists
    pub fn get_effective_database_path(&self) -> Result<PathBuf> {
        // Check if the specified path exists
        if self.database.path.exists() {
            return Ok(self.database.path.clone());
        }

        // If fallback is enabled, try the default path
        if self.database.fallback_to_default {
            let default_path = Self::get_default_database_path();
            if default_path.exists() {
                return Ok(default_path);
            }
        }

        Err(ThingsError::configuration(format!(
            "Database not found at {} and fallback is {}",
            self.database.path.display(),
            if self.database.fallback_to_default {
                "enabled but default path also not found"
            } else {
                "disabled"
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
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                name: "things3-mcp-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: "Things 3 MCP Server".to_string(),
                max_connections: 100,
                connection_timeout: 30,
                request_timeout: 60,
                graceful_shutdown: true,
                shutdown_timeout: 30,
            },
            database: DatabaseConfig {
                path: Self::get_default_database_path(),
                fallback_to_default: true,
                pool_size: 10,
                connection_timeout: 30,
                query_timeout: 60,
                enable_query_logging: false,
                enable_query_metrics: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                json_logs: false,
                log_file: None,
                console_logs: true,
                structured_logs: true,
                rotation: LogRotationConfig {
                    enabled: true,
                    max_file_size_mb: 100,
                    max_files: 5,
                    compress: true,
                },
            },
            performance: PerformanceConfig {
                enabled: true,
                slow_request_threshold_ms: 1000,
                enable_profiling: false,
                memory_monitoring: MemoryMonitoringConfig {
                    enabled: true,
                    threshold_percentage: 80.0,
                    check_interval: 60,
                },
                cpu_monitoring: CpuMonitoringConfig {
                    enabled: true,
                    threshold_percentage: 80.0,
                    check_interval: 60,
                },
            },
            security: SecurityConfig {
                authentication: AuthenticationConfig {
                    enabled: false,
                    require_auth: false,
                    jwt_secret: "your-secret-key-change-this-in-production".to_string(),
                    jwt_expiration: 3600,
                    api_keys: vec![],
                    oauth: None,
                },
                rate_limiting: RateLimitingConfig {
                    enabled: true,
                    requests_per_minute: 60,
                    burst_limit: 10,
                    custom_limits: None,
                },
                cors: CorsConfig {
                    enabled: true,
                    allowed_origins: vec!["*".to_string()],
                    allowed_methods: vec![
                        "GET".to_string(),
                        "POST".to_string(),
                        "PUT".to_string(),
                        "DELETE".to_string(),
                    ],
                    allowed_headers: vec!["*".to_string()],
                    exposed_headers: vec![],
                    allow_credentials: false,
                    max_age: 86400,
                },
                validation: ValidationConfig {
                    enabled: true,
                    strict_mode: false,
                    max_request_size: 1024 * 1024, // 1MB
                    max_field_length: 1000,
                },
            },
            cache: CacheConfig {
                enabled: true,
                cache_type: "memory".to_string(),
                max_size_mb: 100,
                ttl_seconds: 3600,
                compression: true,
                eviction_policy: "lru".to_string(),
            },
            monitoring: MonitoringConfig {
                enabled: true,
                metrics_port: 9090,
                health_port: 8080,
                health_checks: true,
                metrics_collection: true,
                metrics_path: "/metrics".to_string(),
                health_path: "/health".to_string(),
            },
            features: FeatureFlags {
                real_time_updates: true,
                websocket_server: true,
                dashboard: true,
                bulk_operations: true,
                data_export: true,
                backup: true,
                hot_reloading: false,
            },
        }
    }
}

/// Parse a boolean value from a string
fn parse_bool(value: &str) -> bool {
    let lower = value.to_lowercase();
    matches!(lower.as_str(), "true" | "1" | "yes" | "on")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = McpServerConfig::default();
        assert_eq!(config.server.name, "things3-mcp-server");
        assert!(config.database.fallback_to_default);
        assert_eq!(config.logging.level, "info");
        assert!(config.performance.enabled);
        assert!(!config.security.authentication.enabled);
        assert!(config.cache.enabled);
        assert!(config.monitoring.enabled);
    }

    #[test]
    fn test_config_validation() {
        let config = McpServerConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_server_name() {
        let mut config = McpServerConfig::default();
        config.server.name = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_log_level() {
        let mut config = McpServerConfig::default();
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_from_env() {
        std::env::set_var("MCP_SERVER_NAME", "test-server");
        std::env::set_var("MCP_LOG_LEVEL", "debug");
        std::env::set_var("MCP_CACHE_ENABLED", "false");

        let config = McpServerConfig::from_env().unwrap();
        assert_eq!(config.server.name, "test-server");
        assert_eq!(config.logging.level, "debug");
        assert!(!config.cache.enabled);

        // Clean up
        std::env::remove_var("MCP_SERVER_NAME");
        std::env::remove_var("MCP_LOG_LEVEL");
        std::env::remove_var("MCP_CACHE_ENABLED");
    }

    #[test]
    fn test_config_to_and_from_file_json() {
        let config = McpServerConfig::default();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().with_extension("json");

        config.to_file(&path, "json").unwrap();
        let loaded_config = McpServerConfig::from_file(&path).unwrap();

        assert_eq!(config.server.name, loaded_config.server.name);
        assert_eq!(config.logging.level, loaded_config.logging.level);
    }

    #[test]
    fn test_config_merge() {
        let mut config1 = McpServerConfig::default();
        let mut config2 = McpServerConfig::default();
        config2.server.name = "merged-server".to_string();
        config2.logging.level = "debug".to_string();
        config2.cache.enabled = false;

        config1.merge_with(&config2);
        assert_eq!(config1.server.name, "merged-server");
        assert_eq!(config1.logging.level, "debug");
        assert!(!config1.cache.enabled);
    }

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("true"));
        assert!(parse_bool("TRUE"));
        assert!(parse_bool("1"));
        assert!(parse_bool("yes"));
        assert!(parse_bool("on"));
        assert!(!parse_bool("false"));
        assert!(!parse_bool("0"));
        assert!(!parse_bool("no"));
        assert!(!parse_bool("off"));
        assert!(!parse_bool("invalid"));
    }

    #[test]
    fn test_effective_database_path() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        let mut config = McpServerConfig::default();
        config.database.path = db_path.to_path_buf();
        config.database.fallback_to_default = false;

        let effective_path = config.get_effective_database_path().unwrap();
        assert_eq!(effective_path, db_path);
    }

    #[test]
    fn test_effective_database_path_fallback() {
        let mut config = McpServerConfig::default();
        config.database.path = PathBuf::from("/nonexistent/path");
        config.database.fallback_to_default = true;

        // This will succeed if the default path exists, fail otherwise
        let _ = config.get_effective_database_path();
    }
}
