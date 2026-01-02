//! Basic usage example for things3-core
//!
//! Run with: cargo run --example basic_usage

use things3_core::{ThingsConfig, ThingsDatabase};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Things 3 Basic Usage Example\n");

    // Connect to database
    let config = ThingsConfig::from_env();
    println!("Connecting to database: {}", config.database_path.display());
    
    let db = ThingsDatabase::new(&config.database_path).await?;
    println!("✓ Connected successfully\n");

    // Get inbox tasks
    println!("=== Inbox Tasks ===");
    let inbox = db.get_inbox(Some(5)).await?;
    println!("Found {} inbox tasks:", inbox.len());
    for task in &inbox {
        println!("  - {} ({})", task.title, task.uuid);
    }
    println!();

    // Get today's tasks
    println!("=== Today's Tasks ===");
    let today = db.get_today(Some(5)).await?;
    println!("Found {} tasks for today:", today.len());
    for task in &today {
        println!("  - {} ({})", task.title, task.uuid);
    }
    println!();

    // Get projects
    println!("=== Projects ===");
    let projects = db.get_projects(Some(5)).await?;
    println!("Found {} projects:", projects.len());
    for project in &projects {
        println!("  - {} ({})", project.title, project.uuid);
    }
    println!();

    // Get areas
    println!("=== Areas ===");
    let areas = db.get_areas().await?;
    println!("Found {} areas:", areas.len());
    for area in &areas {
        println!("  - {} ({})", area.title, area.uuid);
    }
    println!();

    println!("✓ Example completed successfully");
    Ok(())
}

