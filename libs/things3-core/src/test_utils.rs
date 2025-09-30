//! Test utilities and mock data for Things 3 integration

use crate::models::{Area, Project, Task, TaskStatus, TaskType};
use chrono::Utc;
use std::path::Path;
use uuid::Uuid;

/// Create a test database with mock data
///
/// # Errors
/// Returns `ThingsError::Database` if the database cannot be created
pub async fn create_test_database<P: AsRef<Path>>(db_path: P) -> crate::Result<()> {
    use sqlx::SqlitePool;

    let database_url = format!("sqlite:{}", db_path.as_ref().display());
    let pool = SqlitePool::connect(&database_url)
        .await
        .map_err(|e| crate::ThingsError::Database(format!("Failed to connect to database: {e}")))?;

    // Create the Things 3 schema
    sqlx::query(
        r"
        -- TMTask table (main tasks table) - matches real Things 3 schema
        CREATE TABLE IF NOT EXISTS TMTask (
            uuid TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            type INTEGER NOT NULL DEFAULT 0,
            status INTEGER NOT NULL DEFAULT 0,
            notes TEXT,
            start_date TEXT,
            due_date TEXT,
            created TEXT NOT NULL,
            modified TEXT NOT NULL,
            project_uuid TEXT,
            area_uuid TEXT,
            parent_uuid TEXT,
            tags TEXT DEFAULT '[]'
        )
        ",
    )
    .execute(&pool)
    .await
    .map_err(|e| crate::ThingsError::Database(format!("Failed to create TMTask table: {e}")))?;

    sqlx::query(
        r"
        -- TMProject table (projects table)
        CREATE TABLE IF NOT EXISTS TMProject (
            uuid TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            type INTEGER NOT NULL DEFAULT 1,
            status INTEGER NOT NULL DEFAULT 0,
            notes TEXT,
            start_date TEXT,
            due_date TEXT,
            created TEXT NOT NULL,
            modified TEXT NOT NULL,
            area_uuid TEXT,
            parent_uuid TEXT,
            tags TEXT DEFAULT '[]'
        )
        ",
    )
    .execute(&pool)
    .await
    .map_err(|e| crate::ThingsError::Database(format!("Failed to create TMProject table: {e}")))?;

    sqlx::query(
        r"
        -- TMArea table (areas table)
        CREATE TABLE IF NOT EXISTS TMArea (
            uuid TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            type INTEGER NOT NULL DEFAULT 3,
            status INTEGER NOT NULL DEFAULT 0,
            notes TEXT,
            start_date TEXT,
            due_date TEXT,
            created TEXT NOT NULL,
            modified TEXT NOT NULL,
            parent_uuid TEXT,
            tags TEXT DEFAULT '[]'
        )
        ",
    )
    .execute(&pool)
    .await
    .map_err(|e| crate::ThingsError::Database(format!("Failed to create TMArea table: {e}")))?;

    // Insert test data
    insert_test_data(&pool).await?;

    pool.close().await;
    Ok(())
}

async fn insert_test_data(pool: &sqlx::SqlitePool) -> crate::Result<()> {
    let now = Utc::now().to_rfc3339();

    // Generate valid UUIDs for test data
    let area_uuid = Uuid::new_v4().to_string();
    let project_uuid = Uuid::new_v4().to_string();
    let task_uuid = Uuid::new_v4().to_string();

    // Insert test areas
    sqlx::query(
        "INSERT INTO TMArea (uuid, title, type, status, created, modified) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&area_uuid)
    .bind("Work")
    .bind(3) // Area type
    .bind(0) // Incomplete
    .bind(&now)
    .bind(&now)
    .execute(pool).await
    .map_err(|e| crate::ThingsError::Database(format!("Failed to insert test area: {e}")))?;

    // Insert test projects
    sqlx::query(
        "INSERT INTO TMProject (uuid, title, type, status, area_uuid, created, modified) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&project_uuid)
    .bind("Website Redesign")
    .bind(1) // Project type
    .bind(0) // Incomplete
    .bind(&area_uuid)
    .bind(&now)
    .bind(&now)
    .execute(pool).await
    .map_err(|e| crate::ThingsError::Database(format!("Failed to insert test project: {e}")))?;

    // Insert test tasks - one in inbox (no project), one in project
    let inbox_task_uuid = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO TMTask (uuid, title, type, status, project_uuid, created, modified) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&inbox_task_uuid)
    .bind("Inbox Task")
    .bind(0) // Todo type
    .bind(0) // Incomplete
    .bind::<Option<String>>(None) // No project (inbox) - use NULL instead of empty string
    .bind(&now)
    .bind(&now)
    .execute(pool).await
    .map_err(|e| crate::ThingsError::Database(format!("Failed to insert inbox test task: {e}")))?;

    sqlx::query(
        "INSERT INTO TMTask (uuid, title, type, status, project_uuid, created, modified) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&task_uuid)
    .bind("Research competitors")
    .bind(0) // Todo type
    .bind(0) // Incomplete
    .bind(&project_uuid)
    .bind(&now)
    .bind(&now)
    .execute(pool).await
    .map_err(|e| crate::ThingsError::Database(format!("Failed to insert test task: {e}")))?;

    Ok(())
}

/// Create mock data for testing
///
/// # Panics
///
/// Panics if UUID parsing fails
#[must_use]
pub fn create_mock_areas() -> Vec<Area> {
    vec![
        Area {
            uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
            title: "Work".to_string(),
            notes: Some("Work-related tasks".to_string()),
            created: Utc::now(),
            modified: Utc::now(),
            tags: vec!["work".to_string()],
            projects: Vec::new(),
        },
        Area {
            uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap(),
            title: "Personal".to_string(),
            notes: Some("Personal tasks".to_string()),
            created: Utc::now(),
            modified: Utc::now(),
            tags: vec!["personal".to_string()],
            projects: Vec::new(),
        },
    ]
}

/// Create mock projects for testing
///
/// # Panics
///
/// Panics if UUID parsing fails
#[must_use]
pub fn create_mock_projects() -> Vec<Project> {
    vec![
        Project {
            uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440010").unwrap(),
            title: "Website Redesign".to_string(),
            status: TaskStatus::Incomplete,
            notes: Some("Complete redesign of company website".to_string()),
            start_date: None,
            deadline: None,
            created: Utc::now(),
            modified: Utc::now(),
            area_uuid: Some(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap()),
            tags: vec!["work".to_string(), "web".to_string()],
            tasks: Vec::new(),
        },
        Project {
            uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440011").unwrap(),
            title: "Learn Rust".to_string(),
            status: TaskStatus::Incomplete,
            notes: Some("Learn the Rust programming language".to_string()),
            start_date: None,
            deadline: None,
            created: Utc::now(),
            modified: Utc::now(),
            area_uuid: Some(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap()),
            tags: vec!["personal".to_string(), "learning".to_string()],
            tasks: Vec::new(),
        },
    ]
}

/// Create mock tasks for testing
///
/// # Panics
///
/// Panics if UUID parsing fails
#[must_use]
pub fn create_mock_tasks() -> Vec<Task> {
    vec![
        Task {
            uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440100").unwrap(),
            title: "Research competitors".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: Some("Look at competitor websites for inspiration".to_string()),
            start_date: None,
            deadline: None,
            created: Utc::now(),
            modified: Utc::now(),
            project_uuid: Some(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440010").unwrap()),
            area_uuid: Some(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap()),
            parent_uuid: None,
            tags: vec!["research".to_string()],
            children: Vec::new(),
        },
        Task {
            uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440101").unwrap(),
            title: "Read Rust book".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: Some("Read The Rust Programming Language book".to_string()),
            start_date: None,
            deadline: None,
            created: Utc::now(),
            modified: Utc::now(),
            project_uuid: Some(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440011").unwrap()),
            area_uuid: Some(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap()),
            parent_uuid: None,
            tags: vec!["reading".to_string()],
            children: Vec::new(),
        },
    ]
}
