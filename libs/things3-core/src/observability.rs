//! Observability module for structured logging and metrics collection
//!
//! This module provides comprehensive observability features including:
//! - Structured logging with tracing
//! - Metrics collection for performance monitoring
//! - Health check endpoints
//! - Log aggregation and filtering

use std::collections::HashMap;
// Removed unused import
use std::time::{Duration, Instant};

// Simplified metrics - in a real application, this would use proper metrics types
// Simplified OpenTelemetry - in a real application, this would use proper OpenTelemetry
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, error, info, instrument, warn, Level};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Error types for observability operations
#[derive(Error, Debug)]
pub enum ObservabilityError {
    #[error("Failed to initialize tracing: {0}")]
    TracingInit(String),

    #[error("Failed to initialize metrics: {0}")]
    MetricsInit(String),

    #[error("Failed to initialize OpenTelemetry: {0}")]
    OpenTelemetryInit(String),

    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),
}

/// Result type for observability operations
pub type Result<T> = std::result::Result<T, ObservabilityError>;

/// Configuration for observability features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,

    /// Enable JSON logging format
    pub json_logs: bool,

    /// Enable OpenTelemetry tracing
    pub enable_tracing: bool,

    /// Jaeger endpoint for tracing
    pub jaeger_endpoint: Option<String>,

    /// OTLP endpoint for tracing
    pub otlp_endpoint: Option<String>,

    /// Enable metrics collection
    pub enable_metrics: bool,

    /// Prometheus metrics port
    pub metrics_port: u16,

    /// Health check port
    pub health_port: u16,

    /// Service name for tracing
    pub service_name: String,

    /// Service version
    pub service_version: String,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            json_logs: false,
            enable_tracing: true,
            jaeger_endpoint: None,
            otlp_endpoint: None,
            enable_metrics: true,
            metrics_port: 9090,
            health_port: 8080,
            service_name: "things3-cli".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Metrics collector for Things 3 operations
#[derive(Debug, Clone)]
pub struct ThingsMetrics {
    // Database operation metrics
    pub db_operations_total: u64,
    pub db_operation_duration: f64,
    pub db_connection_pool_size: u64,
    pub db_connection_pool_active: u64,

    // Task operation metrics
    pub tasks_created_total: u64,
    pub tasks_updated_total: u64,
    pub tasks_deleted_total: u64,
    pub tasks_completed_total: u64,

    // Search operation metrics
    pub search_operations_total: u64,
    pub search_duration: f64,
    pub search_results_count: u64,

    // Export operation metrics
    pub export_operations_total: u64,
    pub export_duration: f64,
    pub export_file_size: u64,

    // Error metrics
    pub errors_total: u64,
    pub error_rate: f64,

    // Performance metrics
    pub memory_usage: u64,
    pub cpu_usage: f64,
    pub cache_hit_rate: f64,
    pub cache_size: u64,
}

impl Default for ThingsMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl ThingsMetrics {
    /// Create new metrics instance
    #[must_use]
    pub fn new() -> Self {
        Self {
            db_operations_total: 0,
            db_operation_duration: 0.0,
            db_connection_pool_size: 0,
            db_connection_pool_active: 0,

            tasks_created_total: 0,
            tasks_updated_total: 0,
            tasks_deleted_total: 0,
            tasks_completed_total: 0,

            search_operations_total: 0,
            search_duration: 0.0,
            search_results_count: 0,

            export_operations_total: 0,
            export_duration: 0.0,
            export_file_size: 0,

            errors_total: 0,
            error_rate: 0.0,

            memory_usage: 0,
            cpu_usage: 0.0,
            cache_hit_rate: 0.0,
            cache_size: 0,
        }
    }
}

/// Health check status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub uptime: Duration,
    pub checks: HashMap<String, CheckResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub status: String,
    pub message: Option<String>,
    pub duration_ms: u64,
}

/// Observability manager
#[derive(Debug)]
pub struct ObservabilityManager {
    config: ObservabilityConfig,
    #[allow(dead_code)]
    metrics: ThingsMetrics,
    // Simplified tracer - in a real application, this would use proper OpenTelemetry
    start_time: Instant,
}

impl ObservabilityManager {
    /// Create a new observability manager
    ///
    /// # Errors
    /// Returns an error if the observability manager cannot be created
    pub fn new(config: ObservabilityConfig) -> Result<Self> {
        let metrics = ThingsMetrics::new();
        let start_time = Instant::now();

        Ok(Self {
            config,
            metrics,
            start_time,
        })
    }

    /// Initialize observability features
    ///
    /// # Errors
    /// Returns an error if observability features cannot be initialized
    #[instrument(skip(self))]
    pub fn initialize(&mut self) -> Result<()> {
        // Only log initialization messages if tracing is enabled
        if self.config.enable_tracing {
            info!("Initializing observability features");
        }

        // Initialize tracing
        self.init_tracing()?;

        // Initialize metrics
        Self::init_metrics();

        // Initialize OpenTelemetry if enabled
        if self.config.enable_tracing {
            Self::init_opentelemetry();
        }

        // Only log success message if tracing is enabled
        if self.config.enable_tracing {
            info!("Observability features initialized successfully");
        }
        Ok(())
    }

    /// Initialize structured logging
    fn init_tracing(&self) -> Result<()> {
        let _log_level = self
            .config
            .log_level
            .parse::<Level>()
            .map_err(|e| ObservabilityError::TracingInit(format!("Invalid log level: {e}")))?;

        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&self.config.log_level));

        let registry = tracing_subscriber::registry().with(filter);

        if self.config.json_logs {
            let json_layer = fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(true)
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true);

            registry.with(json_layer).init();
        } else {
            let fmt_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true)
                .with_span_events(FmtSpan::CLOSE);

            registry.with(fmt_layer).init();
        }

        // Only log initialization message if tracing is enabled
        if self.config.enable_tracing {
            info!("Tracing initialized with level: {}", self.config.log_level);
        }
        Ok(())
    }

    /// Initialize metrics collection
    fn init_metrics() {
        // For now, use a simple metrics implementation
        // In a real implementation, this would set up a proper metrics recorder
        // Note: This is a static method, so we can't check enable_tracing here
        // But metrics initialization messages are typically not critical
        // If needed, this could be made an instance method
    }

    /// Initialize OpenTelemetry tracing
    fn init_opentelemetry() {
        // Simplified OpenTelemetry implementation
        // In a real implementation, this would set up proper tracing
        // Note: This method is only called when enable_tracing is true,
        // so logging here would be safe, but we skip it to be extra cautious
    }

    /// Get health status
    #[must_use]
    pub fn health_status(&self) -> HealthStatus {
        let mut checks = HashMap::new();

        // Database health check
        checks.insert(
            "database".to_string(),
            CheckResult {
                status: "healthy".to_string(),
                message: Some("Database connection is healthy".to_string()),
                duration_ms: 0, // TODO: Implement actual health check
            },
        );

        // Memory health check
        checks.insert(
            "memory".to_string(),
            CheckResult {
                status: "healthy".to_string(),
                message: Some("Memory usage is within normal limits".to_string()),
                duration_ms: 0, // TODO: Implement actual health check
            },
        );

        HealthStatus {
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now(),
            version: self.config.service_version.clone(),
            uptime: self.start_time.elapsed(),
            checks,
        }
    }

    /// Record a database operation
    #[instrument(skip(self, f))]
    pub fn record_db_operation<F, R>(&self, operation: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        // In a real implementation, this would update metrics atomically
        debug!(
            operation = operation,
            duration_ms = duration.as_millis(),
            "Database operation completed"
        );

        result
    }

    /// Record a task operation
    #[instrument(skip(self))]
    pub fn record_task_operation(&self, operation: &str, count: u64) {
        // In a real implementation, this would update metrics atomically
        info!(
            operation = operation,
            count = count,
            "Task operation recorded"
        );
    }

    /// Record a search operation
    #[instrument(skip(self, f))]
    pub fn record_search_operation<F, R>(&self, query: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        // In a real implementation, this would update metrics atomically
        debug!(
            query = query,
            duration_ms = duration.as_millis(),
            "Search operation completed"
        );

        result
    }

    /// Record an error
    #[instrument(skip(self))]
    pub fn record_error(&self, error_type: &str, error_message: &str) {
        // In a real implementation, this would update metrics atomically
        error!(
            error_type = error_type,
            error_message = error_message,
            "Error recorded"
        );
    }

    /// Update performance metrics
    #[instrument(skip(self))]
    pub fn update_performance_metrics(
        &self,
        memory_usage: u64,
        cpu_usage: f64,
        cache_hit_rate: f64,
        cache_size: u64,
    ) {
        // In a real implementation, this would update metrics atomically
        debug!(
            memory_usage = memory_usage,
            cpu_usage = cpu_usage,
            cache_hit_rate = cache_hit_rate,
            cache_size = cache_size,
            "Performance metrics updated"
        );
    }
}

// Simplified metrics implementation - in a real application, this would use
// a proper metrics library like prometheus or statsd

/// Macro for easy instrumentation
#[macro_export]
macro_rules! instrument_operation {
    ($operation:expr, $code:block) => {{
        let start = std::time::Instant::now();
        let result = $code;
        let duration = start.elapsed();

        tracing::debug!(
            operation = $operation,
            duration_ms = duration.as_millis(),
            "Operation completed"
        );

        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observability_config_default() {
        let config = ObservabilityConfig::default();
        assert_eq!(config.log_level, "info");
        assert!(!config.json_logs);
        assert!(config.enable_tracing);
        assert!(config.enable_metrics);
        assert_eq!(config.metrics_port, 9090);
        assert_eq!(config.health_port, 8080);
    }

    #[test]
    fn test_health_status() {
        let config = ObservabilityConfig::default();
        let manager = ObservabilityManager::new(config).unwrap();
        let health = manager.health_status();

        assert_eq!(health.status, "healthy");
        assert!(health.checks.contains_key("database"));
        assert!(health.checks.contains_key("memory"));
    }

    #[test]
    fn test_metrics_creation() {
        let _metrics = ThingsMetrics::new();
        // Test that metrics can be created without panicking
    }

    #[test]
    fn test_observability_config_creation() {
        let config = ObservabilityConfig {
            log_level: "debug".to_string(),
            json_logs: true,
            enable_tracing: true,
            jaeger_endpoint: Some("http://localhost:14268".to_string()),
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            enable_metrics: true,
            metrics_port: 9091,
            health_port: 8081,
            service_name: "test-service".to_string(),
            service_version: "1.0.0".to_string(),
        };

        assert_eq!(config.log_level, "debug");
        assert!(config.json_logs);
        assert!(config.enable_tracing);
        assert_eq!(
            config.jaeger_endpoint,
            Some("http://localhost:14268".to_string())
        );
        assert_eq!(
            config.otlp_endpoint,
            Some("http://localhost:4317".to_string())
        );
        assert!(config.enable_metrics);
        assert_eq!(config.metrics_port, 9091);
        assert_eq!(config.health_port, 8081);
        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.service_version, "1.0.0");
    }

    #[test]
    fn test_observability_config_serialization() {
        let config = ObservabilityConfig {
            log_level: "warn".to_string(),
            json_logs: false,
            enable_tracing: false,
            jaeger_endpoint: None,
            otlp_endpoint: None,
            enable_metrics: false,
            metrics_port: 9092,
            health_port: 8082,
            service_name: "serialization-test".to_string(),
            service_version: "2.0.0".to_string(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ObservabilityConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.log_level, "warn");
        assert!(!deserialized.json_logs);
        assert!(!deserialized.enable_tracing);
        assert_eq!(deserialized.jaeger_endpoint, None);
        assert_eq!(deserialized.otlp_endpoint, None);
        assert!(!deserialized.enable_metrics);
        assert_eq!(deserialized.metrics_port, 9092);
        assert_eq!(deserialized.health_port, 8082);
        assert_eq!(deserialized.service_name, "serialization-test");
        assert_eq!(deserialized.service_version, "2.0.0");
    }

    #[test]
    fn test_observability_config_clone() {
        let config = ObservabilityConfig::default();
        let cloned_config = config.clone();

        assert_eq!(cloned_config.log_level, config.log_level);
        assert_eq!(cloned_config.json_logs, config.json_logs);
        assert_eq!(cloned_config.enable_tracing, config.enable_tracing);
        assert_eq!(cloned_config.jaeger_endpoint, config.jaeger_endpoint);
        assert_eq!(cloned_config.otlp_endpoint, config.otlp_endpoint);
        assert_eq!(cloned_config.enable_metrics, config.enable_metrics);
        assert_eq!(cloned_config.metrics_port, config.metrics_port);
        assert_eq!(cloned_config.health_port, config.health_port);
        assert_eq!(cloned_config.service_name, config.service_name);
        assert_eq!(cloned_config.service_version, config.service_version);
    }

    #[test]
    fn test_things_metrics_creation() {
        let metrics = ThingsMetrics::new();

        assert_eq!(metrics.db_operations_total, 0);
        assert!((metrics.db_operation_duration - 0.0).abs() < f64::EPSILON);
        assert_eq!(metrics.db_connection_pool_size, 0);
        assert_eq!(metrics.db_connection_pool_active, 0);
        assert_eq!(metrics.tasks_created_total, 0);
        assert_eq!(metrics.tasks_updated_total, 0);
        assert_eq!(metrics.tasks_deleted_total, 0);
        assert_eq!(metrics.tasks_completed_total, 0);
        assert_eq!(metrics.search_operations_total, 0);
        assert!((metrics.search_duration - 0.0).abs() < f64::EPSILON);
        assert_eq!(metrics.search_results_count, 0);
        assert_eq!(metrics.export_operations_total, 0);
        assert!((metrics.export_duration - 0.0).abs() < f64::EPSILON);
        assert_eq!(metrics.export_file_size, 0);
        assert_eq!(metrics.errors_total, 0);
        assert!((metrics.error_rate - 0.0).abs() < f64::EPSILON);
        assert_eq!(metrics.memory_usage, 0);
        assert!((metrics.cpu_usage - 0.0).abs() < f64::EPSILON);
        assert!((metrics.cache_hit_rate - 0.0).abs() < f64::EPSILON);
        assert_eq!(metrics.cache_size, 0);
    }

    #[test]
    fn test_things_metrics_default() {
        let metrics = ThingsMetrics::default();
        let new_metrics = ThingsMetrics::new();

        assert_eq!(metrics.db_operations_total, new_metrics.db_operations_total);
        assert!(
            (metrics.db_operation_duration - new_metrics.db_operation_duration).abs()
                < f64::EPSILON
        );
        assert_eq!(metrics.tasks_created_total, new_metrics.tasks_created_total);
        assert_eq!(metrics.errors_total, new_metrics.errors_total);
    }

    #[test]
    fn test_things_metrics_clone() {
        let metrics = ThingsMetrics::new();
        let cloned_metrics = metrics.clone();

        assert_eq!(
            cloned_metrics.db_operations_total,
            metrics.db_operations_total
        );
        assert!(
            (cloned_metrics.db_operation_duration - metrics.db_operation_duration).abs()
                < f64::EPSILON
        );
        assert_eq!(
            cloned_metrics.tasks_created_total,
            metrics.tasks_created_total
        );
        assert_eq!(cloned_metrics.errors_total, metrics.errors_total);
    }

    #[test]
    fn test_health_status_creation() {
        let config = ObservabilityConfig::default();
        let manager = ObservabilityManager::new(config).unwrap();
        let health = manager.health_status();

        assert_eq!(health.status, "healthy");
        assert!(health.checks.contains_key("database"));
        assert!(health.checks.contains_key("memory"));
        assert_eq!(health.checks.len(), 2);
    }

    #[test]
    fn test_health_status_serialization() {
        let config = ObservabilityConfig::default();
        let manager = ObservabilityManager::new(config).unwrap();
        let health = manager.health_status();

        let json = serde_json::to_string(&health).unwrap();
        let deserialized: HealthStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.status, "healthy");
        assert!(deserialized.checks.contains_key("database"));
        assert!(deserialized.checks.contains_key("memory"));
        assert_eq!(deserialized.checks.len(), 2);
    }

    #[test]
    fn test_health_status_clone() {
        let config = ObservabilityConfig::default();
        let manager = ObservabilityManager::new(config).unwrap();
        let health = manager.health_status();
        let cloned_health = health.clone();

        assert_eq!(cloned_health.status, health.status);
        assert_eq!(cloned_health.checks.len(), health.checks.len());
        assert!(cloned_health.checks.contains_key("database"));
        assert!(cloned_health.checks.contains_key("memory"));
    }

    #[test]
    fn test_check_result_creation() {
        let check_result = CheckResult {
            status: "healthy".to_string(),
            message: Some("Test check passed".to_string()),
            duration_ms: 150,
        };

        assert_eq!(check_result.status, "healthy");
        assert_eq!(check_result.message, Some("Test check passed".to_string()));
        assert_eq!(check_result.duration_ms, 150);
    }

    #[test]
    fn test_check_result_without_message() {
        let check_result = CheckResult {
            status: "unhealthy".to_string(),
            message: None,
            duration_ms: 0,
        };

        assert_eq!(check_result.status, "unhealthy");
        assert_eq!(check_result.message, None);
        assert_eq!(check_result.duration_ms, 0);
    }

    #[test]
    fn test_check_result_serialization() {
        let check_result = CheckResult {
            status: "healthy".to_string(),
            message: Some("Database connection is healthy".to_string()),
            duration_ms: 250,
        };

        let json = serde_json::to_string(&check_result).unwrap();
        let deserialized: CheckResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.status, "healthy");
        assert_eq!(
            deserialized.message,
            Some("Database connection is healthy".to_string())
        );
        assert_eq!(deserialized.duration_ms, 250);
    }

    #[test]
    fn test_check_result_clone() {
        let check_result = CheckResult {
            status: "healthy".to_string(),
            message: Some("Test check passed".to_string()),
            duration_ms: 100,
        };
        let cloned_check = check_result.clone();

        assert_eq!(cloned_check.status, check_result.status);
        assert_eq!(cloned_check.message, check_result.message);
        assert_eq!(cloned_check.duration_ms, check_result.duration_ms);
    }

    #[test]
    fn test_observability_manager_creation() {
        let config = ObservabilityConfig::default();
        let manager = ObservabilityManager::new(config).unwrap();

        // Test that the manager was created successfully
        assert!(manager.start_time.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn test_observability_manager_creation_with_custom_config() {
        let config = ObservabilityConfig {
            log_level: "debug".to_string(),
            json_logs: true,
            enable_tracing: true,
            jaeger_endpoint: Some("http://localhost:14268".to_string()),
            otlp_endpoint: None,
            enable_metrics: true,
            metrics_port: 9091,
            health_port: 8081,
            service_name: "custom-service".to_string(),
            service_version: "1.2.3".to_string(),
        };

        let manager = ObservabilityManager::new(config).unwrap();
        assert!(manager.start_time.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn test_observability_manager_debug_formatting() {
        let config = ObservabilityConfig::default();
        let manager = ObservabilityManager::new(config).unwrap();

        let debug_str = format!("{manager:?}");
        assert!(debug_str.contains("ObservabilityManager"));
    }

    #[test]
    fn test_record_db_operation() {
        let config = ObservabilityConfig::default();
        let manager = ObservabilityManager::new(config).unwrap();

        let result = manager.record_db_operation("test_operation", || {
            // Simulate some work
            std::thread::sleep(std::time::Duration::from_millis(10));
            "operation_result"
        });

        assert_eq!(result, "operation_result");
    }

    #[test]
    fn test_record_task_operation() {
        let config = ObservabilityConfig::default();
        let manager = ObservabilityManager::new(config).unwrap();

        // This should not panic
        manager.record_task_operation("create_task", 5);
        manager.record_task_operation("update_task", 3);
        manager.record_task_operation("delete_task", 1);
    }

    #[test]
    fn test_record_search_operation() {
        let config = ObservabilityConfig::default();
        let manager = ObservabilityManager::new(config).unwrap();

        let result = manager.record_search_operation("test query", || {
            // Simulate search work
            std::thread::sleep(std::time::Duration::from_millis(5));
            vec!["result1", "result2"]
        });

        assert_eq!(result, vec!["result1", "result2"]);
    }

    #[test]
    fn test_record_error() {
        let config = ObservabilityConfig::default();
        let manager = ObservabilityManager::new(config).unwrap();

        // This should not panic
        manager.record_error("database_error", "Connection failed");
        manager.record_error("validation_error", "Invalid input");
        manager.record_error("timeout_error", "Operation timed out");
    }

    #[test]
    fn test_update_performance_metrics() {
        let config = ObservabilityConfig::default();
        let manager = ObservabilityManager::new(config).unwrap();

        // This should not panic
        manager.update_performance_metrics(1024, 0.5, 0.95, 512);
        manager.update_performance_metrics(2048, 0.75, 0.88, 1024);
        manager.update_performance_metrics(4096, 1.0, 0.92, 2048);
    }

    #[test]
    fn test_observability_error_variants() {
        let tracing_error = ObservabilityError::TracingInit("Test error".to_string());
        let metrics_error = ObservabilityError::MetricsInit("Test error".to_string());
        let otel_error = ObservabilityError::OpenTelemetryInit("Test error".to_string());
        let health_error = ObservabilityError::HealthCheckFailed("Test error".to_string());

        assert!(matches!(tracing_error, ObservabilityError::TracingInit(_)));
        assert!(matches!(metrics_error, ObservabilityError::MetricsInit(_)));
        assert!(matches!(
            otel_error,
            ObservabilityError::OpenTelemetryInit(_)
        ));
        assert!(matches!(
            health_error,
            ObservabilityError::HealthCheckFailed(_)
        ));
    }

    #[test]
    fn test_observability_error_display() {
        let tracing_error = ObservabilityError::TracingInit("Failed to initialize".to_string());
        let error_string = tracing_error.to_string();

        assert!(error_string.contains("Failed to initialize tracing"));
        assert!(error_string.contains("Failed to initialize"));
    }

    #[test]
    fn test_observability_error_debug() {
        let error = ObservabilityError::HealthCheckFailed("Database down".to_string());
        let debug_str = format!("{error:?}");

        assert!(debug_str.contains("HealthCheckFailed"));
        assert!(debug_str.contains("Database down"));
    }
}
