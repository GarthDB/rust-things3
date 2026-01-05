//! Basic usage example for things3-core
//!
//! This example demonstrates basic database operations including:
//! - Connecting to the Things 3 database
//! - Retrieving tasks, projects, and areas
//! - Searching for tasks
//! - Creating and updating tasks

use chrono::NaiveDate;
use things3_core::{CreateTaskRequest, ThingsDatabase, ThingsError, UpdateTaskRequest};

#[tokio::main]
async fn main() -> Result<(), ThingsError> {
    // Get the default database path or use a custom path
    let db_path = things3_core::get_default_database_path();

    // Connect to the database
    println!("Connecting to database at: {}", db_path.display());
    let db = ThingsDatabase::new(&db_path).await?;

    // Get inbox tasks
    println!("\n=== Inbox Tasks ===");
    let inbox_tasks = db.get_inbox(Some(5)).await?;
    println!("Found {} tasks in inbox", inbox_tasks.len());
    for task in &inbox_tasks {
        println!("  - {} ({:?})", task.title, task.status);
    }

    // Get today's tasks
    println!("\n=== Today's Tasks ===");
    let today_tasks = db.get_today(Some(5)).await?;
    println!("Found {} tasks scheduled for today", today_tasks.len());
    for task in &today_tasks {
        println!("  - {}", task.title);
    }

    // Get all projects
    println!("\n=== Projects ===");
    let projects = db.get_projects(None).await?;
    println!("Found {} projects", projects.len());
    for project in &projects {
        println!("  - {} ({:?})", project.title, project.status);
    }

    // Get all areas
    println!("\n=== Areas ===");
    let areas = db.get_areas().await?;
    println!("Found {} areas", areas.len());
    for area in &areas {
        println!("  - {}", area.title);
    }

    // Search for tasks
    println!("\n=== Search Results ===");
    let search_results = db.search_tasks("meeting").await?;
    println!("Found {} tasks matching 'meeting'", search_results.len());
    for task in &search_results {
        println!("  - {}", task.title);
    }

    // Create a new task
    println!("\n=== Creating Task ===");
    let create_request = CreateTaskRequest {
        title: "Example task from Rust".to_string(),
        notes: Some("Created using things3-core library".to_string()),
        deadline: Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
        start_date: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        task_type: None,
        status: None,
    };

    let task_uuid = db.create_task(create_request).await?;
    println!("Created task with UUID: {}", task_uuid);

    // Update the task
    println!("\n=== Updating Task ===");
    let update_request = UpdateTaskRequest {
        uuid: task_uuid,
        title: Some("Updated example task".to_string()),
        notes: Some("This task was updated".to_string()),
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        tags: None,
        status: None,
    };

    db.update_task(update_request).await?;
    println!("Updated task successfully");

    // Get database statistics
    println!("\n=== Database Statistics ===");
    let stats = db.get_stats().await?;
    println!("Total tasks: {}", stats.task_count);
    println!("Total projects: {}", stats.project_count);
    println!("Total areas: {}", stats.area_count);

    Ok(())
}
