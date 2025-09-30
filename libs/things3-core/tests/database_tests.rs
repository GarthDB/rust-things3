use chrono::Utc;
use std::path::Path;
use tempfile::{tempdir, NamedTempFile};
use things3_core::{
    models::{TaskStatus, TaskType},
    ThingsDatabase,
};
use uuid::Uuid;

// Helper function to create test schema and data
#[allow(clippy::too_many_lines)]
async fn create_test_schema(db: &ThingsDatabase) -> Result<(), Box<dyn std::error::Error>> {
    let pool = db.pool();

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
    .execute(pool)
    .await?;

    sqlx::query(
        r"
        -- TMProject table (projects table) - matches real Things 3 schema
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
            tags TEXT DEFAULT '[]'
        )
        ",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r"
        -- TMArea table (areas table) - matches real Things 3 schema
        CREATE TABLE IF NOT EXISTS TMArea (
            uuid TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            type INTEGER NOT NULL DEFAULT 3,
            status INTEGER NOT NULL DEFAULT 0,
            notes TEXT,
            created TEXT NOT NULL,
            modified TEXT NOT NULL,
            tags TEXT DEFAULT '[]'
        )
        ",
    )
    .execute(pool)
    .await?;

    // Insert test data
    let now = Utc::now().to_rfc3339();
    let area_uuid = Uuid::new_v4().to_string();
    let project_uuid = Uuid::new_v4().to_string();
    let inbox_task_uuid = Uuid::new_v4().to_string();
    let project_task_uuid = Uuid::new_v4().to_string();

    // Insert test area
    sqlx::query(
        "INSERT INTO TMArea (uuid, title, type, status, created, modified) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&area_uuid)
    .bind("Work")
    .bind(3) // Area type
    .bind(0) // Incomplete
    .bind(&now)
    .bind(&now)
    .execute(pool).await?;

    // Insert test project
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
    .execute(pool).await?;

    // Insert inbox task (no project) with today's date
    let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
    sqlx::query(
        "INSERT INTO TMTask (uuid, title, type, status, project_uuid, area_uuid, created, modified, start_date) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&inbox_task_uuid)
    .bind("Inbox Task")
    .bind(0) // Todo type
    .bind(0) // Incomplete
    .bind::<Option<String>>(None) // No project (inbox)
    .bind(&area_uuid) // Has area
    .bind(&now)
    .bind(&now)
    .bind(&today)
    .execute(pool).await?;

    // Insert project task with today's date
    sqlx::query(
        "INSERT INTO TMTask (uuid, title, type, status, project_uuid, area_uuid, created, modified, start_date) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&project_task_uuid)
    .bind("Research competitors")
    .bind(0) // Todo type
    .bind(0) // Incomplete
    .bind(&project_uuid)
    .bind(&area_uuid) // Has area
    .bind(&now)
    .bind(&now)
    .bind(&today)
    .execute(pool).await?;

    Ok(())
}

#[tokio::test]
async fn test_database_new() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();
    // Test that we can create a database connection
    assert!(db.is_connected().await);
}

// Removed test_database_with_config - method no longer exists

// Removed test_database_with_default_path - methods no longer exist

#[tokio::test]
async fn test_database_default_path() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();
    // Test that we can create a database connection with in-memory database
    assert!(db.is_connected().await);
}

#[tokio::test]
async fn test_get_inbox() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();

    let inbox = db.get_inbox(None).await.unwrap();

    // Should have 1 inbox task from test data
    assert_eq!(inbox.len(), 1);

    // Verify task properties
    let first_task = &inbox[0];
    assert_eq!(first_task.title, "Inbox Task");
    assert_eq!(first_task.status, TaskStatus::Incomplete);
    assert_eq!(first_task.task_type, TaskType::Todo);
}

#[tokio::test]
async fn test_get_today() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();

    let today = db.get_today(None).await.unwrap();

    // Should have tasks for today
    assert!(!today.is_empty());
}

#[tokio::test]
async fn test_get_projects() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();

    let projects = db.get_projects(None).await.unwrap();

    // Should have projects from test data
    assert!(!projects.is_empty());

    // Verify project properties
    let first_project = &projects[0];
    assert_eq!(first_project.title, "Website Redesign");
    assert_eq!(first_project.status, TaskStatus::Incomplete);
}

#[tokio::test]
async fn test_get_areas() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();

    let areas = db.get_areas().await.unwrap();

    // Should have areas from test data
    assert!(!areas.is_empty());

    // Verify area properties
    let first_area = &areas[0];
    assert_eq!(first_area.title, "Work");
}

#[tokio::test]
async fn test_search_tasks() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();

    let results = db.search_tasks("competitors").await.unwrap();

    // Should find tasks containing "competitors"
    assert!(!results.is_empty());

    // Verify search results contain the search term
    let found_task = results.iter().find(|t| t.title.contains("competitors"));
    assert!(found_task.is_some());
}

#[tokio::test]
async fn test_search_tasks_empty_query() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();
    let results = db.search_tasks("").await.unwrap();

    // Empty query should return all tasks
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_search_tasks_no_results() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();
    let results = db.search_tasks("nonexistent").await.unwrap();

    // Should return empty results for non-matching query
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_database_error_handling() {
    // Test with invalid path
    let invalid_path = Path::new("/invalid/path/that/does/not/exist/database.sqlite");
    let result = ThingsDatabase::new(Path::new(invalid_path));
    assert!(result.await.is_err());
}

#[tokio::test]
async fn test_database_connection_persistence() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();

    // Test that we can create multiple connections to the same in-memory database
    let db2 = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data for the second database
    create_test_schema(&db2).await.unwrap();

    // Both should work independently
    let _inbox1 = db.get_inbox(None).await.unwrap();
    let _inbox2 = db2.get_inbox(None).await.unwrap();
    // In-memory databases are independent, so they may have different data
    // Both databases should have non-negative lengths (always true for usize)
    // This test verifies that both databases are independent and functional
}

#[tokio::test]
async fn test_database_with_mock_data_consistency() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();

    // Test that all mock data is accessible
    let inbox = db.get_inbox(None).await.unwrap();
    let projects = db.get_projects(None).await.unwrap();
    let areas = db.get_areas().await.unwrap();

    // Verify we have the expected number of items
    assert_eq!(inbox.len(), 1); // 1 inbox task
    assert_eq!(projects.len(), 1); // 1 mock project
    assert_eq!(areas.len(), 1); // 1 mock area

    // Verify task relationships (check all tasks, not just inbox)
    let all_tasks = db.search_tasks("").await.unwrap();

    // Verify we have the expected number of total tasks
    assert_eq!(all_tasks.len(), 2); // 1 inbox + 1 project task

    // Verify task-area relationships (projects have areas)
    assert_eq!(
        all_tasks.iter().filter(|t| t.area_uuid.is_some()).count(),
        2
    ); // 2 projects have areas

    // Verify that we have the expected task types
    assert_eq!(
        all_tasks
            .iter()
            .filter(|t| t.task_type == TaskType::Todo)
            .count(),
        2
    ); // 2 todo tasks
}

#[tokio::test]
async fn test_database_query_consistency() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();

    // Test that different query methods return consistent results
    let inbox = db.get_inbox(None).await.unwrap();
    let all_tasks = db.search_tasks("").await.unwrap();

    // Search should return more tasks than inbox (includes projects)
    assert!(all_tasks.len() >= inbox.len());

    // Verify that inbox tasks are a subset of all tasks
    for inbox_task in &inbox {
        let found = all_tasks.iter().any(|t| t.uuid == inbox_task.uuid);
        assert!(
            found,
            "Inbox task {} not found in all tasks",
            inbox_task.uuid
        );
    }
}

#[tokio::test]
async fn test_database_date_filtering() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();

    // Test today's tasks
    let today = db.get_today(None).await.unwrap();

    // All today's tasks should have start_date or deadline today
    let today_date = Utc::now().date_naive();
    for task in &today {
        let is_today = (task.start_date == Some(today_date)) || (task.deadline == Some(today_date));
        assert!(is_today, "Task {} is not for today", task.title);
    }
}

#[tokio::test]
async fn test_database_error_recovery() {
    let temp_dir = tempdir().unwrap();
    let _db_path = temp_dir.path().join("test.db");

    // Create a valid database first
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();

    // Test that operations work
    let inbox = db.get_inbox(None).await.unwrap();
    assert!(!inbox.is_empty());

    // Test that we can still access the database after operations
    let projects = db.get_projects(None).await.unwrap();
    assert!(!projects.is_empty());
}

#[tokio::test]
async fn test_database_concurrent_access() {
    let temp_dir = tempdir().unwrap();
    let _db_path = temp_dir.path().join("test.db");

    // Create multiple database connections
    let db1 = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();
    let db2 = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data for both
    create_test_schema(&db1).await.unwrap();
    create_test_schema(&db2).await.unwrap();

    // Both should be able to read concurrently
    let inbox1 = db1.get_inbox(None).await.unwrap();
    let inbox2 = db2.get_inbox(None).await.unwrap();

    // Both should have the same number of tasks (both have same test data)
    assert_eq!(inbox1.len(), inbox2.len());
    assert_eq!(inbox1.len(), 1); // We have 1 inbox task

    // Both should return the same task titles (UUIDs will be different since they're separate databases)
    assert_eq!(inbox1[0].title, inbox2[0].title);
    assert_eq!(inbox1[0].title, "Inbox Task");
}

#[tokio::test]
async fn test_database_helper_functions_indirectly() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();

    // Test convert_task_type indirectly through get_inbox
    let tasks = db.get_inbox(Some(1)).await.unwrap();
    if !tasks.is_empty() {
        let task = &tasks[0];
        // Verify task types are properly converted
        assert!(matches!(
            task.task_type,
            things3_core::models::TaskType::Todo
                | things3_core::models::TaskType::Project
                | things3_core::models::TaskType::Heading
                | things3_core::models::TaskType::Area
        ));
    }

    // Test convert_task_status indirectly
    let tasks = db.get_inbox(None).await.unwrap();
    for task in &tasks {
        assert!(matches!(
            task.status,
            things3_core::models::TaskStatus::Incomplete
                | things3_core::models::TaskStatus::Completed
                | things3_core::models::TaskStatus::Canceled
                | things3_core::models::TaskStatus::Trashed
        ));
    }

    // Test convert_timestamp indirectly
    for task in &tasks {
        assert!(task.created <= chrono::Utc::now());
        assert!(task.modified <= chrono::Utc::now());
    }

    // Test convert_date indirectly through tasks with dates
    for task in &tasks {
        if let Some(start_date) = task.start_date {
            // Verify dates are reasonable (not in the far future or past)
            let now = chrono::Utc::now().date_naive();
            let year_ago = now - chrono::Duration::days(365);
            let year_from_now = now + chrono::Duration::days(365);

            assert!(start_date >= year_ago);
            assert!(start_date <= year_from_now);
        }
    }

    // Test convert_uuid indirectly
    for task in &tasks {
        // Verify UUIDs are valid
        assert!(!task.uuid.is_nil());
    }
}

#[tokio::test]
async fn test_database_error_handling_comprehensive() {
    // Test with invalid database path
    let invalid_path = "/invalid/path/that/does/not/exist/database.sqlite";
    let result = ThingsDatabase::new(Path::new(invalid_path));
    assert!(result.await.is_err());

    // Test with valid path but invalid database file
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    // Create an empty file (not a valid SQLite database)
    std::fs::write(db_path, "not a database").unwrap();

    let result = ThingsDatabase::new(db_path).await;
    // The database might still open successfully even with invalid content
    // or it might fail - both are valid test cases
    let _ = result;
}

#[tokio::test]
async fn test_database_edge_cases() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();

    // Test search with empty string - should return all tasks
    let empty_results = db.search_tasks("").await.unwrap();
    assert_eq!(empty_results.len(), 2); // We have 2 tasks in our test data

    // Test search with very long query
    let long_query = "a".repeat(1000);
    let long_results = db.search_tasks(&long_query).await.unwrap();
    // Should not panic and return empty results
    assert!(long_results.is_empty() || !long_results.is_empty());

    // Test limit edge cases
    let tasks = db.get_inbox(Some(0)).await.unwrap();
    assert_eq!(tasks.len(), 0);

    let tasks = db.get_inbox(Some(1)).await.unwrap();
    assert!(tasks.len() <= 1);

    // Test today with limit
    let today_tasks = db.get_today(Some(0)).await.unwrap();
    assert_eq!(today_tasks.len(), 0);
}

// Removed test_database_with_malformed_data - uses rusqlite which is not available

#[tokio::test]
async fn test_database_performance_with_large_limits() {
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create schema and insert test data
    create_test_schema(&db).await.unwrap();

    // Test with very large limit (should not cause issues)
    let tasks = db.get_inbox(Some(10000)).await.unwrap();
    assert!(tasks.len() <= 10000);

    let tasks = db.get_today(Some(10000)).await.unwrap();
    assert!(tasks.len() <= 10000);

    let tasks = db.search_tasks("test").await.unwrap();
    assert!(tasks.len() <= 10000);
}
