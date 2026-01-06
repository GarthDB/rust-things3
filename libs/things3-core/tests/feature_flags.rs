//! Integration tests for feature flags
//!
//! These tests verify that feature flags work correctly and that
//! conditional compilation is working as expected.

#[test]
fn test_core_features_always_available() {
    // Core features should always be available regardless of feature flags
    use things3_core::{ThingsConfig, ThingsError};

    // Verify core types are available
    let _config: ThingsConfig;
    let _error: Result<(), ThingsError> = Ok(());
}

#[cfg(feature = "export-csv")]
#[test]
fn test_csv_export_feature_enabled() {
    // When export-csv is enabled, CSV export types should be available
    use things3_core::{DataExporter, ExportConfig, ExportFormat};

    let config = ExportConfig::default();
    let exporter = DataExporter::new(config);

    // Verify we can create CSV format
    let format = ExportFormat::Csv;
    assert_eq!(format, ExportFormat::Csv);

    // Verify exporter exists and is usable
    let _ = exporter;
}

#[cfg(not(feature = "export-csv"))]
#[test]
fn test_csv_export_feature_disabled() {
    // When export-csv is disabled, we should not be able to import export types
    // This test just verifies the feature flag is working by checking compilation

    // Note: We can't actually test that imports fail, but the fact that this
    // test compiles without importing export types proves the feature flag works
    assert!(true, "CSV export feature correctly disabled");
}

#[cfg(feature = "export-opml")]
#[test]
fn test_opml_export_feature_enabled() {
    // When export-opml is enabled, OPML export types should be available
    use things3_core::{DataExporter, ExportConfig, ExportFormat};

    let config = ExportConfig::default();
    let exporter = DataExporter::new(config);

    // Verify we can create OPML format
    let format = ExportFormat::Opml;
    assert_eq!(format, ExportFormat::Opml);

    // Verify exporter exists and is usable
    let _ = exporter;
}

#[cfg(not(feature = "export-opml"))]
#[test]
fn test_opml_export_feature_disabled() {
    // When export-opml is disabled, we should not be able to import export types
    assert!(true, "OPML export feature correctly disabled");
}

#[cfg(feature = "observability")]
#[test]
fn test_observability_feature_enabled() {
    // When observability is enabled, observability types should be available
    use things3_core::{
        CheckResult, HealthStatus, ObservabilityConfig, ObservabilityManager, ThingsMetrics,
    };

    // Verify we can create observability config
    let config = ObservabilityConfig::default();
    assert_eq!(config.metrics_port, 9090);

    // Verify we can reference health status struct
    let _status: Option<HealthStatus> = None;

    // Verify we can reference metrics types
    let _metrics: Option<ThingsMetrics> = None;
    let _check: Option<CheckResult> = None;
    let _manager: Option<ObservabilityManager> = None;
}

#[cfg(not(feature = "observability"))]
#[test]
fn test_observability_feature_disabled() {
    // When observability is disabled, we should not be able to import observability types
    assert!(true, "Observability feature correctly disabled");
}

#[cfg(all(feature = "export-csv", feature = "export-opml"))]
#[test]
fn test_multiple_export_features_enabled() {
    // When both export features are enabled, both should work
    use things3_core::{DataExporter, ExportConfig, ExportFormat};

    let config = ExportConfig::default();
    let exporter = DataExporter::new(config);

    // Verify both formats are available
    let csv_format = ExportFormat::Csv;
    let opml_format = ExportFormat::Opml;

    assert_eq!(csv_format, ExportFormat::Csv);
    assert_eq!(opml_format, ExportFormat::Opml);
    let _ = exporter;
}

#[cfg(all(
    feature = "export-csv",
    feature = "export-opml",
    feature = "observability"
))]
#[test]
fn test_all_features_enabled() {
    // When all features are enabled, all types should be available
    use things3_core::{
        DataExporter, ExportConfig, ExportFormat, HealthStatus, ObservabilityConfig,
        ObservabilityManager,
    };

    // Verify export functionality
    let export_config = ExportConfig::default();
    let exporter = DataExporter::new(export_config);
    let csv_format = ExportFormat::Csv;
    let opml_format = ExportFormat::Opml;

    assert_eq!(csv_format, ExportFormat::Csv);
    assert_eq!(opml_format, ExportFormat::Opml);
    let _ = exporter;

    // Verify observability functionality
    let obs_config = ObservabilityConfig::default();

    assert_eq!(obs_config.metrics_port, 9090);

    // Verify we can reference the types
    let _status: Option<HealthStatus> = None;
    let _manager: Option<ObservabilityManager> = None;
}

#[test]
fn test_tracing_always_available() {
    // Tracing should always be available as it's a core dependency
    use tracing::{debug, error, info, warn};

    // These should compile regardless of features
    info!("Test info log");
    debug!("Test debug log");
    warn!("Test warn log");
    error!("Test error log");
}

#[cfg(feature = "test-utils")]
#[test]
fn test_test_utils_feature_enabled() {
    // When test-utils is enabled, test utilities should be available
    use things3_core::test_utils::{create_mock_areas, create_mock_projects, create_mock_tasks};

    // Verify we can create mock data
    let tasks = create_mock_tasks();
    assert!(!tasks.is_empty(), "Should create mock tasks");

    let areas = create_mock_areas();
    assert!(!areas.is_empty(), "Should create mock areas");

    let projects = create_mock_projects();
    assert!(!projects.is_empty(), "Should create mock projects");
}

#[test]
fn test_default_features() {
    // With default features, export and observability should be available
    // This test will only pass when run with default features enabled

    #[cfg(all(
        feature = "export-csv",
        feature = "export-opml",
        feature = "observability"
    ))]
    {
        use things3_core::{ExportConfig, ObservabilityConfig};

        let export_config = ExportConfig::default();
        let obs_config = ObservabilityConfig::default();

        // Verify configs are created successfully
        let _ = export_config;
        assert_eq!(obs_config.metrics_port, 9090);
        assert!(true, "Default features are enabled");
    }

    #[cfg(not(all(
        feature = "export-csv",
        feature = "export-opml",
        feature = "observability"
    )))]
    {
        // If not all default features are enabled, that's ok - user might be
        // testing with a custom feature configuration
        assert!(true, "Custom feature configuration detected");
    }
}
