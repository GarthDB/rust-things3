//! Data export example
//!
//! Run with: cargo run --example export_data

use things3_core::{DataExporter, ExportFormat, ThingsConfig, ThingsDatabase};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Things 3 Data Export Example\n");

    // Connect to database
    let config = ThingsConfig::from_env();
    let db = ThingsDatabase::new(&config.database_path).await?;

    // Get data
    println!("Fetching data...");
    let tasks = db.get_inbox(None).await?;
    let projects = db.get_projects(None).await?;
    let areas = db.get_areas().await?;

    println!("  Tasks: {}", tasks.len());
    println!("  Projects: {}", projects.len());
    println!("  Areas: {}", areas.len());
    println!();

    // Create exporter
    let exporter = DataExporter::new_default();

    // Export to JSON
    println!("Exporting to JSON...");
    let json = exporter.export_json(&tasks, &projects, &areas).await?;
    std::fs::write("export.json", &json)?;
    println!("  ✓ Saved to export.json ({} bytes)", json.len());

    // Export to CSV
    println!("Exporting to CSV...");
    let csv = exporter.export_csv(&tasks, &projects, &areas).await?;
    std::fs::write("export.csv", &csv)?;
    println!("  ✓ Saved to export.csv ({} bytes)", csv.len());

    // Export to Markdown
    println!("Exporting to Markdown...");
    let markdown = exporter
        .export_markdown(&tasks, &projects, &areas)
        .await?;
    std::fs::write("export.md", &markdown)?;
    println!("  ✓ Saved to export.md ({} bytes)", markdown.len());

    println!("\n✓ Export completed successfully");
    Ok(())
}

