//! Task search example
//!
//! Run with: cargo run --example search_tasks -- "meeting"

use std::env;
use things3_core::{ThingsConfig, ThingsDatabase};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get search query from command line
    let query = env::args()
        .nth(1)
        .unwrap_or_else(|| "meeting".to_string());

    println!("Searching for: '{}'\n", query);

    // Connect to database
    let config = ThingsConfig::from_env();
    let db = ThingsDatabase::new(&config.database_path).await?;

    // Search tasks
    let results = db.search_tasks(&query).await?;

    println!("Found {} matching tasks:\n", results.len());

    for task in results {
        println!("Title: {}", task.title);
        println!("UUID: {}", task.uuid);
        
        if let Some(notes) = &task.notes {
            println!("Notes: {}", notes);
        }
        
        if let Some(project) = &task.project {
            println!("Project: {}", project);
        }
        
        if let Some(area) = &task.area {
            println!("Area: {}", area);
        }
        
        println!();
    }

    Ok(())
}

