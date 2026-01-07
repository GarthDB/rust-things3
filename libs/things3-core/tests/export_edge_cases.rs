//! Export format edge case tests
//!
//! Tests various edge cases in data export functionality to ensure
//! robust handling of empty data, special characters, and extreme values.

#![cfg(any(feature = "export-csv", feature = "export-opml"))]

use things3_core::{DataExporter, ExportData, ExportFormat};

/// Test exporting empty data set
#[test]
fn test_export_empty_data() {
    let exporter = DataExporter::new_default();
    let data = ExportData::new(vec![], vec![], vec![]);

    // JSON export of empty data
    let json = exporter.export(&data, ExportFormat::Json).unwrap();
    assert!(!json.is_empty(), "JSON should not be empty");
    // JSON should contain empty arrays (format may vary)
    assert!(
        json.contains("tasks") && json.contains("[]"),
        "Should have empty arrays"
    );

    // CSV export of empty data
    let _csv = exporter.export(&data, ExportFormat::Csv).unwrap();

    // Markdown export of empty data
    let markdown = exporter.export(&data, ExportFormat::Markdown).unwrap();
    assert!(markdown.contains("# Things 3 Export"), "Should have title");

    // OPML export of empty data
    let opml = exporter.export(&data, ExportFormat::Opml).unwrap();
    assert!(opml.contains("<?xml"), "Should have XML header");
}

/// Test exporting data with mock data
#[test]
#[cfg(feature = "test-utils")]
fn test_export_with_mock_data() {
    use things3_core::test_utils::{create_mock_areas, create_mock_projects, create_mock_tasks};

    let exporter = DataExporter::new_default();
    let tasks = create_mock_tasks();
    let projects = create_mock_projects();
    let areas = create_mock_areas();
    let data = ExportData::new(tasks, projects, areas);

    // JSON should handle data
    let json = exporter.export(&data, ExportFormat::Json).unwrap();
    assert!(!json.is_empty(), "JSON should contain data");

    // CSV should work
    let csv = exporter.export(&data, ExportFormat::Csv).unwrap();
    assert!(!csv.is_empty(), "CSV should contain data");

    // Markdown should work
    let markdown = exporter.export(&data, ExportFormat::Markdown).unwrap();
    assert!(markdown.contains("# Things 3 Export"), "Should have title");
}

/// Test exporting large dataset with repeated mock data
#[test]
#[cfg(feature = "test-utils")]
fn test_export_large_dataset() {
    use things3_core::test_utils::create_mock_tasks;

    let exporter = DataExporter::new_default();

    // Create many copies of mock data
    let mut tasks = vec![];
    for _ in 0..100 {
        tasks.extend(create_mock_tasks());
    }

    let data = ExportData::new(tasks, vec![], vec![]);

    // All formats should handle large datasets
    let json = exporter.export(&data, ExportFormat::Json).unwrap();
    assert!(json.len() > 1000, "JSON should be substantial");

    let csv = exporter.export(&data, ExportFormat::Csv).unwrap();
    assert!(csv.lines().count() > 100, "CSV should have many lines");

    let markdown = exporter.export(&data, ExportFormat::Markdown).unwrap();
    assert!(markdown.contains("Total Items:"), "Should show item count");
}

/// Test exporting with only empty data
#[test]
fn test_export_only_empty_data() {
    let exporter = DataExporter::new_default();
    let data = ExportData::new(vec![], vec![], vec![]);

    // All formats should handle completely empty data gracefully
    let json = exporter.export(&data, ExportFormat::Json).unwrap();
    assert!(!json.is_empty(), "JSON should handle empty data");

    let _csv = exporter.export(&data, ExportFormat::Csv).unwrap();
    // CSV might be empty or have headers only

    let markdown = exporter.export(&data, ExportFormat::Markdown).unwrap();
    assert!(!markdown.is_empty(), "Markdown should handle empty data");
    // Should have some indication of item count
    assert!(markdown.contains("Total Items"), "Should show item count");
}

/// Test JSON export specifically
#[test]
fn test_json_export_validity() {
    let exporter = DataExporter::new_default();
    let data = ExportData::new(vec![], vec![], vec![]);

    let json = exporter.export(&data, ExportFormat::Json).unwrap();

    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_object(), "Should be a JSON object");
    assert!(parsed.get("tasks").is_some(), "Should have tasks field");
    assert!(
        parsed.get("projects").is_some(),
        "Should have projects field"
    );
    assert!(parsed.get("areas").is_some(), "Should have areas field");
}

/// Test CSV export format
#[test]
fn test_csv_export_format() {
    let exporter = DataExporter::new_default();
    let data = ExportData::new(vec![], vec![], vec![]);

    let _csv = exporter.export(&data, ExportFormat::Csv).unwrap();

    // CSV should generate without panicking (even if empty)
}

/// Test exporting mixed data types with mock data
#[test]
#[cfg(feature = "test-utils")]
fn test_export_mixed_data() {
    use things3_core::test_utils::{create_mock_areas, create_mock_projects, create_mock_tasks};

    let exporter = DataExporter::new_default();
    let tasks = create_mock_tasks();
    let projects = create_mock_projects();
    let areas = create_mock_areas();
    let data = ExportData::new(tasks, projects, areas);

    // All formats should handle mixed data
    let json = exporter.export(&data, ExportFormat::Json).unwrap();
    assert!(!json.is_empty());

    let csv = exporter.export(&data, ExportFormat::Csv).unwrap();
    assert!(!csv.is_empty());

    let markdown = exporter.export(&data, ExportFormat::Markdown).unwrap();
    assert!(!markdown.is_empty());
}

/// Test export format parsing
#[test]
fn test_export_format_parsing() {
    use std::str::FromStr;

    assert!(matches!(
        ExportFormat::from_str("json"),
        Ok(ExportFormat::Json)
    ));
    assert!(matches!(
        ExportFormat::from_str("JSON"),
        Ok(ExportFormat::Json)
    ));
    assert!(matches!(
        ExportFormat::from_str("csv"),
        Ok(ExportFormat::Csv)
    ));
    assert!(matches!(
        ExportFormat::from_str("markdown"),
        Ok(ExportFormat::Markdown)
    ));
    assert!(matches!(
        ExportFormat::from_str("md"),
        Ok(ExportFormat::Markdown)
    ));
    assert!(matches!(
        ExportFormat::from_str("opml"),
        Ok(ExportFormat::Opml)
    ));

    // Invalid format
    assert!(ExportFormat::from_str("invalid").is_err());
}
