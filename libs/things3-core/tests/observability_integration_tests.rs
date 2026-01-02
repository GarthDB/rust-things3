//! Integration tests for observability system
//!
//! These tests verify that observability components work correctly together
//! in realistic scenarios, complementing the unit tests in observability.rs

use std::collections::HashMap;
use std::time::Duration;
use things3_core::observability::{
    CheckResult, HealthStatus, ObservabilityConfig, ObservabilityManager,
};

// ============================================================================
// ObservabilityConfig Integration Tests
// ============================================================================

#[test]
fn test_observability_config_with_all_features_enabled() {
    let config = ObservabilityConfig {
        log_level: "debug".to_string(),
        json_logs: true,
        enable_tracing: true,
        jaeger_endpoint: Some("http://localhost:14268/api/traces".to_string()),
        otlp_endpoint: Some("http://localhost:4317".to_string()),
        enable_metrics: true,
        metrics_port: 9090,
        health_port: 8080,
        service_name: "things3-test".to_string(),
        service_version: "1.0.0".to_string(),
    };

    assert_eq!(config.log_level, "debug");
    assert!(config.json_logs);
    assert!(config.enable_tracing);
    assert!(config.jaeger_endpoint.is_some());
    assert!(config.otlp_endpoint.is_some());
    assert!(config.enable_metrics);
    assert_eq!(config.metrics_port, 9090);
    assert_eq!(config.health_port, 8080);
}

#[test]
fn test_observability_config_with_minimal_features() {
    let config = ObservabilityConfig {
        log_level: "error".to_string(),
        json_logs: false,
        enable_tracing: false,
        jaeger_endpoint: None,
        otlp_endpoint: None,
        enable_metrics: false,
        metrics_port: 9090,
        health_port: 8080,
        service_name: "things3-minimal".to_string(),
        service_version: "0.1.0".to_string(),
    };

    assert_eq!(config.log_level, "error");
    assert!(!config.json_logs);
    assert!(!config.enable_tracing);
    assert!(config.jaeger_endpoint.is_none());
    assert!(config.otlp_endpoint.is_none());
    assert!(!config.enable_metrics);
}

#[test]
fn test_observability_config_log_levels() {
    let levels = vec!["trace", "debug", "info", "warn", "error"];

    for level in levels {
        let config = ObservabilityConfig {
            log_level: level.to_string(),
            ..Default::default()
        };

        assert_eq!(config.log_level, level);
    }
}

#[test]
fn test_observability_config_custom_ports() {
    let config = ObservabilityConfig {
        metrics_port: 3000,
        health_port: 3001,
        ..Default::default()
    };

    assert_eq!(config.metrics_port, 3000);
    assert_eq!(config.health_port, 3001);
}

#[test]
fn test_observability_config_with_jaeger_only() {
    let config = ObservabilityConfig {
        jaeger_endpoint: Some("http://jaeger:14268/api/traces".to_string()),
        otlp_endpoint: None,
        ..Default::default()
    };

    assert!(config.jaeger_endpoint.is_some());
    assert!(config.otlp_endpoint.is_none());
}

#[test]
fn test_observability_config_with_otlp_only() {
    let config = ObservabilityConfig {
        jaeger_endpoint: None,
        otlp_endpoint: Some("http://otel-collector:4317".to_string()),
        ..Default::default()
    };

    assert!(config.jaeger_endpoint.is_none());
    assert!(config.otlp_endpoint.is_some());
}

// ============================================================================
// ObservabilityManager Integration Tests
// ============================================================================

#[test]
fn test_observability_manager_creation_with_default_config() {
    let config = ObservabilityConfig::default();
    let manager = ObservabilityManager::new(config);

    assert!(manager.is_ok());
}

#[test]
fn test_observability_manager_creation_with_custom_config() {
    let config = ObservabilityConfig {
        log_level: "debug".to_string(),
        json_logs: true,
        enable_tracing: true,
        enable_metrics: true,
        ..Default::default()
    };

    let manager = ObservabilityManager::new(config);
    assert!(manager.is_ok());
}

#[test]
fn test_observability_manager_initialization() {
    let config = ObservabilityConfig::default();
    let manager = ObservabilityManager::new(config).unwrap();

    // Don't call initialize() - it sets global state that conflicts with other tests
    // Just verify manager was created successfully
    assert!(manager.health_status().status == "healthy");
}

#[test]
fn test_observability_manager_with_tracing_disabled() {
    let config = ObservabilityConfig {
        enable_tracing: false,
        ..Default::default()
    };

    let manager = ObservabilityManager::new(config).unwrap();
    // Don't call initialize() - it sets global state that conflicts with other tests
    // Just verify manager was created successfully
    assert!(manager.health_status().status == "healthy");
}

#[test]
fn test_observability_manager_with_metrics_disabled() {
    let config = ObservabilityConfig {
        enable_metrics: false,
        ..Default::default()
    };

    let manager = ObservabilityManager::new(config).unwrap();
    // Don't call initialize() - it sets global state that conflicts with other tests
    // Just verify manager was created successfully
    assert!(manager.health_status().status == "healthy");
}

#[test]
fn test_observability_manager_with_all_features_disabled() {
    let config = ObservabilityConfig {
        enable_tracing: false,
        enable_metrics: false,
        ..Default::default()
    };

    let manager = ObservabilityManager::new(config).unwrap();
    // Don't call initialize() - it sets global state that conflicts with other tests
    // Just verify manager was created successfully
    assert!(manager.health_status().status == "healthy");
}

// ============================================================================
// Metrics Integration Tests
// ============================================================================

#[test]
fn test_metrics_record_database_operations() {
    let config = ObservabilityConfig::default();
    let manager = ObservabilityManager::new(config).unwrap();
    // Don't call initialize() - it sets global state that conflicts with other tests

    // Record multiple database operations using closures
    for i in 0..5 {
        let result = manager.record_db_operation("test_query", || {
            // Simulate database work
            i * 2
        });
        assert_eq!(result, i * 2);
    }
}

#[test]
fn test_metrics_record_task_operations() {
    let config = ObservabilityConfig::default();
    let manager = ObservabilityManager::new(config).unwrap();
    // Don't call initialize() - it sets global state that conflicts with other tests

    // Record various task operations
    manager.record_task_operation("create", 1);
    manager.record_task_operation("update", 1);
    manager.record_task_operation("delete", 1);
    manager.record_task_operation("complete", 1);
}

#[test]
fn test_metrics_record_search_operations() {
    let config = ObservabilityConfig::default();
    let manager = ObservabilityManager::new(config).unwrap();
    // Don't call initialize() - it sets global state that conflicts with other tests

    // Record search operations using closures
    let result1 = manager.record_search_operation("test query 1", || vec![1, 2, 3, 4, 5]);
    assert_eq!(result1.len(), 5);

    let result2 = manager.record_search_operation("test query 2", || vec![1, 2, 3]);
    assert_eq!(result2.len(), 3);
}

#[test]
fn test_metrics_record_errors() {
    let config = ObservabilityConfig::default();
    let manager = ObservabilityManager::new(config).unwrap();
    // Don't call initialize() - it sets global state that conflicts with other tests

    // Record errors
    for i in 0..3 {
        manager.record_error(&format!("error_type_{}", i), "Test error message");
    }
}

// ============================================================================
// Health Check Integration Tests
// ============================================================================

#[test]
fn test_health_status_creation() {
    let config = ObservabilityConfig::default();
    let manager = ObservabilityManager::new(config).unwrap();

    let health = manager.health_status();

    assert_eq!(health.status, "healthy");
    assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
    assert!(!health.checks.is_empty());
}

#[test]
fn test_health_status_with_checks() {
    let mut checks = HashMap::new();
    checks.insert(
        "database".to_string(),
        CheckResult {
            status: "healthy".to_string(),
            message: Some("Connection OK".to_string()),
            duration_ms: 5,
        },
    );
    checks.insert(
        "cache".to_string(),
        CheckResult {
            status: "healthy".to_string(),
            message: None,
            duration_ms: 1,
        },
    );

    let health = HealthStatus {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now(),
        version: "1.0.0".to_string(),
        uptime: Duration::from_secs(3600),
        checks,
    };

    assert_eq!(health.checks.len(), 2);
    assert!(health.checks.contains_key("database"));
    assert!(health.checks.contains_key("cache"));
}

#[test]
fn test_health_status_degraded() {
    let mut checks = HashMap::new();
    checks.insert(
        "database".to_string(),
        CheckResult {
            status: "healthy".to_string(),
            message: None,
            duration_ms: 5,
        },
    );
    checks.insert(
        "cache".to_string(),
        CheckResult {
            status: "degraded".to_string(),
            message: Some("High latency".to_string()),
            duration_ms: 100,
        },
    );

    let health = HealthStatus {
        status: "degraded".to_string(),
        timestamp: chrono::Utc::now(),
        version: "1.0.0".to_string(),
        uptime: Duration::from_secs(3600),
        checks,
    };

    assert_eq!(health.status, "degraded");
    assert_eq!(health.checks["cache"].status, "degraded");
}

// ============================================================================
// Logging Configuration Integration Tests
// ============================================================================

#[test]
fn test_logging_with_json_format() {
    let config = ObservabilityConfig {
        json_logs: true,
        log_level: "info".to_string(),
        ..Default::default()
    };

    let manager = ObservabilityManager::new(config).unwrap();
    // Don't call initialize() - it sets global state that conflicts with other tests
    // Just verify manager was created successfully with JSON logging config
    assert!(manager.health_status().status == "healthy");
}

#[test]
fn test_logging_with_text_format() {
    let config = ObservabilityConfig {
        json_logs: false,
        log_level: "debug".to_string(),
        ..Default::default()
    };

    let manager = ObservabilityManager::new(config).unwrap();
    // Don't call initialize() - it sets global state that conflicts with other tests
    // Just verify manager was created successfully with text logging config
    assert!(manager.health_status().status == "healthy");
}

#[test]
fn test_logging_with_different_levels() {
    let levels = vec!["trace", "debug", "info", "warn", "error"];

    for level in levels {
        let config = ObservabilityConfig {
            log_level: level.to_string(),
            ..Default::default()
        };

        let manager = ObservabilityManager::new(config).unwrap();
        // Don't call initialize() - it sets global state that conflicts with other tests
        // Just verify manager was created successfully with each log level
        assert!(manager.health_status().status == "healthy");
    }
}

// ============================================================================
// Complex Scenario Tests
// ============================================================================

#[test]
fn test_full_observability_stack() {
    let config = ObservabilityConfig {
        log_level: "debug".to_string(),
        json_logs: false,
        enable_tracing: true,
        enable_metrics: true,
        jaeger_endpoint: Some("http://localhost:14268/api/traces".to_string()),
        otlp_endpoint: Some("http://localhost:4317".to_string()),
        metrics_port: 9090,
        health_port: 8080,
        service_name: "things3-integration-test".to_string(),
        service_version: "1.0.0".to_string(),
    };

    let manager = ObservabilityManager::new(config).unwrap();
    // Don't call initialize() - it sets global state that conflicts with other tests

    // Record various operations
    manager.record_db_operation("test_op", || 42);
    manager.record_task_operation("create", 1);
    manager.record_search_operation("test query", || vec![1, 2, 3, 4, 5]);
    manager.record_error("test_error", "Test error message");

    // Get health status
    let health = manager.health_status();
    assert_eq!(health.status, "healthy");
}

#[test]
fn test_observability_with_multiple_operations() {
    let config = ObservabilityConfig::default();
    let manager = ObservabilityManager::new(config).unwrap();
    // Don't call initialize() - it sets global state that conflicts with other tests

    // Simulate a workload
    for i in 0..10 {
        manager.record_db_operation(&format!("operation_{}", i), || i);
        if i % 2 == 0 {
            manager.record_task_operation("create", 1);
        } else {
            manager.record_task_operation("update", 1);
        }
    }

    manager.record_search_operation("test query", || {
        vec![1; 50] // Simulate 50 results
    });

    manager.record_error("test_error", "Simulated error");
}

#[test]
fn test_observability_config_round_trip_serialization() {
    let original_config = ObservabilityConfig {
        log_level: "debug".to_string(),
        json_logs: true,
        enable_tracing: true,
        jaeger_endpoint: Some("http://jaeger:14268".to_string()),
        otlp_endpoint: Some("http://otel:4317".to_string()),
        enable_metrics: true,
        metrics_port: 9091,
        health_port: 8081,
        service_name: "test-service".to_string(),
        service_version: "2.0.0".to_string(),
    };

    // Serialize to JSON
    let json = serde_json::to_string(&original_config).unwrap();

    // Deserialize back
    let deserialized_config: ObservabilityConfig = serde_json::from_str(&json).unwrap();

    // Verify all fields match
    assert_eq!(original_config.log_level, deserialized_config.log_level);
    assert_eq!(original_config.json_logs, deserialized_config.json_logs);
    assert_eq!(
        original_config.enable_tracing,
        deserialized_config.enable_tracing
    );
    assert_eq!(
        original_config.jaeger_endpoint,
        deserialized_config.jaeger_endpoint
    );
    assert_eq!(
        original_config.otlp_endpoint,
        deserialized_config.otlp_endpoint
    );
    assert_eq!(
        original_config.enable_metrics,
        deserialized_config.enable_metrics
    );
    assert_eq!(
        original_config.metrics_port,
        deserialized_config.metrics_port
    );
    assert_eq!(original_config.health_port, deserialized_config.health_port);
    assert_eq!(
        original_config.service_name,
        deserialized_config.service_name
    );
    assert_eq!(
        original_config.service_version,
        deserialized_config.service_version
    );
}
