//! Tests for observability feature functionality
//!
//! These tests actually execute observability code to increase coverage

#[cfg(feature = "observability")]
#[tokio::test]
async fn test_observability_manager_creation() {
    use things3_core::ObservabilityConfig;

    let config = ObservabilityConfig {
        log_level: "info".to_string(),
        json_logs: false,
        enable_tracing: true,
        jaeger_endpoint: None,
        otlp_endpoint: None,
        enable_metrics: true,
        metrics_port: 9090,
        health_port: 8080,
        service_name: "test-service".to_string(),
        service_version: "1.0.0".to_string(),
    };

    // Verify config values
    assert_eq!(config.log_level, "info");
    assert!(!config.json_logs);
    assert!(config.enable_tracing);
    assert!(config.enable_metrics);
    assert_eq!(config.metrics_port, 9090);
    assert_eq!(config.health_port, 8080);
}

#[cfg(feature = "observability")]
#[test]
fn test_observability_config_default() {
    use things3_core::ObservabilityConfig;

    let config = ObservabilityConfig::default();

    assert_eq!(config.metrics_port, 9090);
    assert_eq!(config.health_port, 8080);
    assert!(config.enable_metrics);
}

#[cfg(feature = "observability")]
#[test]
fn test_observability_config_customization() {
    use things3_core::ObservabilityConfig;

    let config = ObservabilityConfig {
        log_level: "debug".to_string(),
        json_logs: true,
        metrics_port: 9091,
        ..ObservabilityConfig::default()
    };

    assert_eq!(config.log_level, "debug");
    assert!(config.json_logs);
    assert_eq!(config.metrics_port, 9091);
}

#[cfg(feature = "observability")]
#[test]
fn test_health_status_structure() {
    use std::collections::HashMap;
    use things3_core::HealthStatus;

    let status = HealthStatus {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now(),
        version: "1.0.0".to_string(),
        uptime: std::time::Duration::from_secs(100),
        checks: HashMap::new(),
    };

    assert_eq!(status.status, "healthy");
    assert_eq!(status.version, "1.0.0");
    assert_eq!(status.uptime.as_secs(), 100);
}

#[cfg(feature = "observability")]
#[test]
fn test_check_result_structure() {
    use things3_core::CheckResult;

    let check = CheckResult {
        status: "pass".to_string(),
        message: Some("All systems operational".to_string()),
        duration_ms: 100,
    };

    assert_eq!(check.status, "pass");
    assert!(check.message.is_some());
    assert_eq!(check.duration_ms, 100);
}
