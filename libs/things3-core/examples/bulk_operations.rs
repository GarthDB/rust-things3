//! Bulk operations example
//!
//! This example demonstrates bulk operations for efficient task management:
//! - Bulk move tasks to a project
//! - Bulk update dates
//! - Bulk complete tasks
//! - Bulk delete tasks

use chrono::NaiveDate;
use things3_core::{
    BulkCompleteRequest, BulkDeleteRequest, BulkMoveRequest, BulkUpdateDatesRequest,
    ThingsDatabase, ThingsError,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), ThingsError> {
    let db_path = things3_core::get_default_database_path();
    let db = ThingsDatabase::new(&db_path).await?;

    // Get some task UUIDs for demonstration
    let tasks = db.get_inbox(Some(10)).await?;
    if tasks.len() < 3 {
        println!("Need at least 3 tasks for this example");
        return Ok(());
    }

    let task_uuids: Vec<Uuid> = tasks.iter().take(3).map(|t| t.uuid).collect();

    println!("=== Bulk Operations Example ===");
    println!("Using {} tasks for demonstration", task_uuids.len());

    // Example 1: Bulk move tasks to a project
    println!("\n1. Bulk Move Tasks");
    // Note: In a real scenario, you'd have a project UUID
    // For this example, we'll skip if no projects exist
    let projects = db.get_projects(None).await?;
    if let Some(project) = projects.first() {
        let move_request = BulkMoveRequest {
            task_uuids: task_uuids.clone(),
            project_uuid: Some(project.uuid),
            area_uuid: None,
        };

        let result = db.bulk_move(move_request).await?;
        println!("Bulk move result: {}", result.message);
        println!("Processed {} tasks", result.processed_count);
    } else {
        println!("No projects found, skipping bulk move example");
    }

    // Example 2: Bulk update dates
    println!("\n2. Bulk Update Dates");
    let update_dates_request = BulkUpdateDatesRequest {
        task_uuids: task_uuids.clone(),
        start_date: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
        deadline: Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
        clear_start_date: false,
        clear_deadline: false,
    };

    let result = db.bulk_update_dates(update_dates_request).await?;
    println!("Bulk update dates result: {}", result.message);
    println!("Processed {} tasks", result.processed_count);

    // Example 3: Bulk complete tasks
    println!("\n3. Bulk Complete Tasks");
    let complete_request = BulkCompleteRequest {
        task_uuids: task_uuids.clone(),
    };

    let result = db.bulk_complete(complete_request).await?;
    println!("Bulk complete result: {}", result.message);
    println!("Processed {} tasks", result.processed_count);

    // Example 4: Bulk delete (soft delete)
    println!("\n4. Bulk Delete Tasks");
    // Note: We'll use different tasks for deletion to avoid deleting
    // tasks we just completed
    let more_tasks = db.get_inbox(Some(3)).await?;
    if !more_tasks.is_empty() {
        let delete_uuids: Vec<Uuid> = more_tasks.iter().take(2).map(|t| t.uuid).collect();
        let delete_request = BulkDeleteRequest {
            task_uuids: delete_uuids,
        };

        let result = db.bulk_delete(delete_request).await?;
        println!("Bulk delete result: {}", result.message);
        println!("Processed {} tasks", result.processed_count);
    } else {
        println!("No tasks available for deletion example");
    }

    println!("\n=== Bulk Operations Complete ===");
    println!("All operations are transactional - either all succeed or all fail");

    Ok(())
}
