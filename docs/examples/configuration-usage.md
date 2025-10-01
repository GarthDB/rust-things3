# Configuration Management Usage Examples

This document provides comprehensive examples of how to use the new MCP server configuration management system.

## Overview

The configuration management system provides:
- **McpServerConfig**: Comprehensive configuration struct with all server options
- **Environment Variable Support**: Load configuration from environment variables
- **Configuration File Support**: Load from JSON/YAML files
- **Configuration Validation**: Ensure configuration is valid
- **Hot Reloading**: Reload configuration without restarting the server
- **Configuration Loader**: Smart loading with precedence and fallbacks

## Basic Usage

### Loading Configuration

```rust
use things3_core::{load_config, McpServerConfig, ConfigLoader};

// Load configuration with default settings (tries files, then environment)
let config = load_config()?;

// Load configuration from specific files
let config = load_config_with_paths(vec!["/path/to/config.json"])?;

// Load configuration from environment variables only
let config = load_config_from_env()?;

// Custom configuration loading
let config = ConfigLoader::new()
    .with_config_paths(vec!["custom-config.yaml"])
    .with_env_loading(true)
    .with_validation(true)
    .load()?;
```

### Creating Configuration Files

```rust
use things3_core::{McpServerConfig, ConfigLoader};

// Create sample configuration files
ConfigLoader::create_all_sample_configs()?;

// Create a specific configuration file
let config = McpServerConfig::default();
config.to_file("my-config.json", "json")?;
config.to_file("my-config.yaml", "yaml")?;
```

## Configuration Structure

### Server Configuration

```json
{
  "server": {
    "name": "things3-mcp-server",
    "version": "1.0.0",
    "description": "Things 3 MCP Server",
    "max_connections": 100,
    "connection_timeout": 30,
    "request_timeout": 60,
    "graceful_shutdown": true,
    "shutdown_timeout": 30
  }
}
```

### Database Configuration

```json
{
  "database": {
    "path": "/path/to/things3/database.sqlite",
    "fallback_to_default": true,
    "pool_size": 10,
    "connection_timeout": 30,
    "query_timeout": 60,
    "enable_query_logging": false,
    "enable_query_metrics": true
  }
}
```

### Logging Configuration

```json
{
  "logging": {
    "level": "info",
    "json_logs": false,
    "log_file": "/var/log/things3-mcp/server.log",
    "console_logs": true,
    "structured_logs": true,
    "rotation": {
      "enabled": true,
      "max_file_size_mb": 100,
      "max_files": 5,
      "compress": true
    }
  }
}
```

### Performance Configuration

```json
{
  "performance": {
    "enabled": true,
    "slow_request_threshold_ms": 1000,
    "enable_profiling": false,
    "memory_monitoring": {
      "enabled": true,
      "threshold_percentage": 80.0,
      "check_interval": 60
    },
    "cpu_monitoring": {
      "enabled": true,
      "threshold_percentage": 80.0,
      "check_interval": 60
    }
  }
}
```

### Security Configuration

```json
{
  "security": {
    "authentication": {
      "enabled": false,
      "require_auth": false,
      "jwt_secret": "your-secret-key",
      "jwt_expiration": 3600,
      "api_keys": [],
      "oauth": null
    },
    "rate_limiting": {
      "enabled": true,
      "requests_per_minute": 60,
      "burst_limit": 10,
      "custom_limits": {
        "admin": 300,
        "api": 100
      }
    },
    "cors": {
      "enabled": true,
      "allowed_origins": ["*"],
      "allowed_methods": ["GET", "POST", "PUT", "DELETE"],
      "allowed_headers": ["*"],
      "exposed_headers": [],
      "allow_credentials": false,
      "max_age": 86400
    },
    "validation": {
      "enabled": true,
      "strict_mode": false,
      "max_request_size": 1048576,
      "max_field_length": 1000
    }
  }
}
```

### Cache Configuration

```json
{
  "cache": {
    "enabled": true,
    "cache_type": "memory",
    "max_size_mb": 100,
    "ttl_seconds": 3600,
    "compression": true,
    "eviction_policy": "lru"
  }
}
```

### Monitoring Configuration

```json
{
  "monitoring": {
    "enabled": true,
    "metrics_port": 9090,
    "health_port": 8080,
    "health_checks": true,
    "metrics_collection": true,
    "metrics_path": "/metrics",
    "health_path": "/health"
  }
}
```

### Feature Flags

```json
{
  "features": {
    "real_time_updates": true,
    "websocket_server": true,
    "dashboard": true,
    "bulk_operations": true,
    "data_export": true,
    "backup": true,
    "hot_reloading": false
  }
}
```

## Environment Variables

The system supports loading configuration from environment variables with the `MCP_` prefix:

```bash
# Server configuration
export MCP_SERVER_NAME="my-server"
export MCP_SERVER_VERSION="1.0.0"
export MCP_MAX_CONNECTIONS="200"

# Database configuration
export MCP_DATABASE_PATH="/path/to/database.sqlite"
export MCP_DATABASE_POOL_SIZE="20"

# Logging configuration
export MCP_LOG_LEVEL="debug"
export MCP_JSON_LOGS="true"
export MCP_LOG_FILE="/var/log/server.log"

# Performance configuration
export MCP_PERFORMANCE_ENABLED="true"
export MCP_SLOW_REQUEST_THRESHOLD="500"

# Security configuration
export MCP_AUTH_ENABLED="true"
export MCP_JWT_SECRET="your-secret-key"
export MCP_RATE_LIMIT_ENABLED="true"
export MCP_REQUESTS_PER_MINUTE="120"

# Cache configuration
export MCP_CACHE_ENABLED="true"
export MCP_CACHE_TYPE="hybrid"
export MCP_CACHE_MAX_SIZE_MB="500"

# Monitoring configuration
export MCP_MONITORING_ENABLED="true"
export MCP_METRICS_PORT="9090"
export MCP_HEALTH_PORT="8080"

# Feature flags
export MCP_REAL_TIME_UPDATES="true"
export MCP_WEBSOCKET_SERVER="true"
export MCP_DASHBOARD="true"
export MCP_BULK_OPERATIONS="true"
export MCP_DATA_EXPORT="true"
export MCP_BACKUP="true"
export MCP_HOT_RELOADING="true"
```

## Hot Reloading

### Basic Hot Reloading

```rust
use things3_core::{ConfigHotReloader, McpServerConfig};
use std::path::PathBuf;
use std::time::Duration;

// Create a hot reloader
let config = McpServerConfig::default();
let config_path = PathBuf::from("config.yaml");
let mut reloader = ConfigHotReloader::new(config, config_path, Duration::from_secs(5))?;

// Start the hot reloader
reloader.start().await?;

// Get current configuration
let current_config = reloader.get_config().await;

// Subscribe to configuration changes
let mut change_rx = reloader.subscribe_to_changes();
while let Ok(new_config) = change_rx.recv().await {
    println!("Configuration changed: {:?}", new_config);
}
```

### Hot Reloading with Change Handler

```rust
use things3_core::{ConfigHotReloaderWithHandler, DefaultConfigChangeHandler};
use std::sync::Arc;

// Create a custom change handler
struct MyConfigChangeHandler;

#[async_trait::async_trait]
impl ConfigChangeHandler for MyConfigChangeHandler {
    async fn handle_config_change(&self, old_config: &McpServerConfig, new_config: &McpServerConfig) {
        println!("Configuration changed from {:?} to {:?}", old_config, new_config);
        // Handle the configuration change
    }
}

// Create hot reloader with handler
let config = McpServerConfig::default();
let config_path = PathBuf::from("config.yaml");
let handler = Arc::new(MyConfigChangeHandler);
let reloader = ConfigHotReloaderWithHandler::new(
    config,
    config_path,
    Duration::from_secs(5),
    handler,
)?;

// Start with change handling
reloader.start_with_handler().await?;
```

## Configuration Precedence

The configuration system uses the following precedence order (highest to lowest):

1. **Environment Variables** - Highest precedence
2. **Configuration Files** - Loaded in order, later files override earlier ones
3. **Default Configuration** - Lowest precedence

### Configuration File Search Order

1. `mcp-config.json` (current directory)
2. `mcp-config.yaml` (current directory)
3. `mcp-config.yml` (current directory)
4. `~/.config/things3-mcp/mcp-config.json`
5. `~/.config/things3-mcp/mcp-config.yaml`
6. `~/.config/things3-mcp/mcp-config.yml`
7. `/etc/things3-mcp/mcp-config.json` (system-wide)
8. `/etc/things3-mcp/mcp-config.yaml` (system-wide)
9. `/etc/things3-mcp/mcp-config.yml` (system-wide)

## Validation

The configuration system includes comprehensive validation:

```rust
use things3_core::McpServerConfig;

let config = McpServerConfig::default();

// Validate the configuration
match config.validate() {
    Ok(()) => println!("Configuration is valid"),
    Err(e) => println!("Configuration validation failed: {}", e),
}
```

### Validation Rules

- Server name and version cannot be empty
- Database pool size must be greater than 0
- Log level must be one of: trace, debug, info, warn, error
- Performance monitoring requires valid thresholds
- Authentication requires JWT secret when enabled
- Cache requires valid size when enabled
- Monitoring requires valid ports when enabled

## Integration with MCP Server

### Using with MCP Server

```rust
use things3_core::{load_config, start_mcp_server_with_config};
use things3_cli::mcp::start_mcp_server_with_config;

// Load configuration
let mcp_config = load_config()?;

// Start MCP server with configuration
start_mcp_server_with_config(db, mcp_config)?;
```

### CLI Integration

The CLI automatically tries to load comprehensive configuration:

```bash
# This will try to load configuration files and environment variables
things3-cli mcp

# If comprehensive configuration is not found, it falls back to basic configuration
```

## Best Practices

1. **Use Configuration Files**: Store complex configurations in files rather than environment variables
2. **Environment-Specific Files**: Use different configuration files for different environments (dev, staging, prod)
3. **Validation**: Always validate configuration before using it
4. **Hot Reloading**: Use hot reloading for development, disable for production
5. **Security**: Store sensitive data (like JWT secrets) in environment variables
6. **Documentation**: Document your configuration options and their effects
7. **Testing**: Test configuration loading and validation in your tests

## Troubleshooting

### Common Issues

1. **Configuration Not Found**: Ensure configuration files exist in the expected locations
2. **Validation Errors**: Check that all required fields are set and valid
3. **Permission Errors**: Ensure the application has read access to configuration files
4. **Hot Reloading Not Working**: Check that the configuration file path is correct and accessible

### Debug Configuration Loading

```rust
use things3_core::ConfigLoader;
use tracing::Level;

// Enable debug logging
tracing::subscriber::set_global_default(
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish(),
)?;

// Load configuration with debug output
let config = ConfigLoader::new()
    .with_validation(true)
    .load()?;
```

This will provide detailed logging about the configuration loading process.
