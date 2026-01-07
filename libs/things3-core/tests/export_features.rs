//! Tests for export feature functionality
//!
//! These tests actually execute export code to increase coverage

#![cfg(any(feature = "export-csv", feature = "export-opml"))]

fn create_test_export_data() -> things3_core::ExportData {
    things3_core::ExportData::new(vec![], vec![], vec![])
}

#[cfg(feature = "export-csv")]
#[test]
fn test_csv_export_functionality() {
    use things3_core::{DataExporter, ExportFormat};

    // Create test data
    let export_data = create_test_export_data();

    // Create exporter
    let exporter = DataExporter::new_default();

    // Export to CSV should work
    let result = exporter.export(&export_data, ExportFormat::Csv);
    assert!(result.is_ok(), "CSV export should succeed");
}

#[cfg(feature = "export-opml")]
#[test]
fn test_opml_export_functionality() {
    use things3_core::{DataExporter, ExportFormat};

    // Create test data
    let export_data = create_test_export_data();

    // Create exporter
    let exporter = DataExporter::new_default();

    // Export to OPML should work
    let result = exporter.export(&export_data, ExportFormat::Opml);
    assert!(result.is_ok(), "OPML export should succeed");
}

#[test]
fn test_json_export_functionality() {
    use things3_core::{DataExporter, ExportFormat};

    // Create test data (JSON is always available)
    let export_data = create_test_export_data();

    // Create exporter
    let exporter = DataExporter::new_default();

    // Export to JSON should work
    let result = exporter.export(&export_data, ExportFormat::Json);
    assert!(result.is_ok(), "JSON export should succeed");
}

#[cfg(all(feature = "export-csv", feature = "export-opml"))]
#[test]
fn test_multiple_export_formats() {
    use things3_core::{DataExporter, ExportFormat};

    let export_data = create_test_export_data();
    let exporter = DataExporter::new_default();

    // Test all formats
    assert!(exporter.export(&export_data, ExportFormat::Json).is_ok());
    assert!(exporter.export(&export_data, ExportFormat::Csv).is_ok());
    assert!(exporter.export(&export_data, ExportFormat::Opml).is_ok());
}

#[test]
fn test_export_config_customization() {
    use things3_core::ExportConfig;

    let config = ExportConfig {
        include_metadata: false,
        include_notes: true,
        include_tags: false,
        date_format: "%Y-%m-%d".to_string(),
        timezone: "America/New_York".to_string(),
    };

    assert!(!config.include_metadata);
    assert!(config.include_notes);
    assert!(!config.include_tags);
    assert_eq!(config.date_format, "%Y-%m-%d");
    assert_eq!(config.timezone, "America/New_York");
}

#[test]
fn test_export_config_default() {
    use things3_core::ExportConfig;

    let config = ExportConfig::default();

    assert!(config.include_metadata);
    assert!(config.include_notes);
    assert!(config.include_tags);
    assert_eq!(config.date_format, "%Y-%m-%d %H:%M:%S");
    assert_eq!(config.timezone, "UTC");
}
