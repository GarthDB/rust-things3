//! Tests for when features are disabled
//!
//! These tests verify error handling when optional features are not enabled

// These tests only make sense when at least one export feature is enabled
#[cfg(any(feature = "export-csv", feature = "export-opml"))]
mod export_tests {
    fn create_test_export_data() -> things3_core::ExportData {
        things3_core::ExportData::new(vec![], vec![], vec![])
    }

    #[cfg(not(feature = "export-csv"))]
    #[test]
    fn test_csv_export_disabled_error() {
        use things3_core::{DataExporter, ExportFormat};

        let exporter = DataExporter::new_default();
        let export_data = create_test_export_data();

        // Attempting to export to CSV without the feature should return an error
        let result = exporter.export(&export_data, ExportFormat::Csv);
        assert!(
            result.is_err(),
            "CSV export should fail when feature is disabled"
        );

        let error = result.unwrap_err().to_string();
        assert!(
            error.contains("CSV export is not enabled"),
            "Error message should indicate CSV feature is disabled"
        );
    }

    #[cfg(not(feature = "export-opml"))]
    #[test]
    fn test_opml_export_disabled_error() {
        use things3_core::{DataExporter, ExportFormat};

        let exporter = DataExporter::new_default();
        let export_data = create_test_export_data();

        // Attempting to export to OPML without the feature should return an error
        let result = exporter.export(&export_data, ExportFormat::Opml);
        assert!(
            result.is_err(),
            "OPML export should fail when feature is disabled"
        );

        let error = result.unwrap_err().to_string();
        assert!(
            error.contains("OPML export is not enabled"),
            "Error message should indicate OPML feature is disabled"
        );
    }

    #[test]
    fn test_json_export_always_available() {
        use things3_core::{DataExporter, ExportFormat};

        let exporter = DataExporter::new_default();
        let export_data = create_test_export_data();

        // JSON export should always work regardless of features
        let result = exporter.export(&export_data, ExportFormat::Json);
        assert!(result.is_ok(), "JSON export should always be available");
    }
}

#[cfg(not(any(feature = "export-csv", feature = "export-opml")))]
#[test]
fn test_no_export_features_enabled() {
    // When no export features are enabled, export types aren't even available
    // This test just verifies the conditional compilation works
}
