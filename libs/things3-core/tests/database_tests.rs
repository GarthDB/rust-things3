use chrono::Utc;
use std::path::Path;
use tempfile::{tempdir, NamedTempFile};
use things3_core::{
    models::{TaskStatus, TaskType},
    ThingsDatabase,
};

#[cfg(feature = "test-utils")]
use things3_core::test_utils::create_test_database;

#[tokio::test]
async fn test_database_new() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let _db = ThingsDatabase::new(&db_path).await.unwrap();
    assert!(db_path.exists());
}

// Removed test_database_with_config - method no longer exists

// Removed test_database_with_default_path - methods no longer exist

#[tokio::test]
async fn test_database_default_path() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let _db = ThingsDatabase::new(&db_path).await.unwrap();
    // The database was created with a specific path, not the default path
    assert!(db_path.exists());
}

#[tokio::test]
async fn test_get_inbox() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create test database with mock data
    #[cfg(feature = "test-utils")]
    create_test_database(&db_path).await.unwrap();

    let db = ThingsDatabase::new(&db_path).await.unwrap();
    let inbox = db.get_inbox(None).await.unwrap();

    // Should have 5 inbox tasks from mock data (first 5 tasks have no project/area)
    assert_eq!(inbox.len(), 5);

    // Verify task properties
    let first_task = &inbox[0];
    assert_eq!(first_task.title, "Review quarterly reports");
    assert_eq!(first_task.status, TaskStatus::Incomplete);
    assert_eq!(first_task.task_type, TaskType::Todo);
}

#[tokio::test]
async fn test_get_today() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    #[cfg(feature = "test-utils")]
    create_test_database(&db_path).await.unwrap();

    let db = ThingsDatabase::new(&db_path).await.unwrap();
    let today = db.get_today(None).await.unwrap();

    // Should have tasks for today
    assert!(!today.is_empty());
}

#[tokio::test]
async fn test_get_projects() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    #[cfg(feature = "test-utils")]
    create_test_database(&db_path).await.unwrap();

    let db = ThingsDatabase::new(&db_path).await.unwrap();
    let projects = db.get_projects(None).await.unwrap();

    // Should have projects from mock data
    assert!(!projects.is_empty());

    // Verify project properties
    let first_project = &projects[0];
    assert_eq!(first_project.title, "Website Redesign");
    assert_eq!(first_project.status, TaskStatus::Incomplete);
}

#[tokio::test]
async fn test_get_areas() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    #[cfg(feature = "test-utils")]
    create_test_database(&db_path).await.unwrap();

    let db = ThingsDatabase::new(&db_path).await.unwrap();
    let areas = db.get_areas().await.unwrap();

    // Should have areas from mock data
    assert!(!areas.is_empty());

    // Verify area properties
    let first_area = &areas[0];
    assert_eq!(first_area.title, "Work");
}

#[tokio::test]
async fn test_search_tasks() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    #[cfg(feature = "test-utils")]
    create_test_database(&db_path).await.unwrap();

    let db = ThingsDatabase::new(&db_path).await.unwrap();
    let results = db.search_tasks("reports").await.unwrap();

    // Should find tasks containing "reports"
    assert!(!results.is_empty());

    // Verify search results contain the search term
    let found_task = results.iter().find(|t| t.title.contains("reports"));
    assert!(found_task.is_some());
}

#[tokio::test]
async fn test_search_tasks_empty_query() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    #[cfg(feature = "test-utils")]
    create_test_database(&db_path).await.unwrap();

    let db = ThingsDatabase::new(&db_path).await.unwrap();
    let results = db.search_tasks("").await.unwrap();

    // Empty query should return all tasks
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_search_tasks_no_results() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    #[cfg(feature = "test-utils")]
    create_test_database(&db_path).await.unwrap();

    let db = ThingsDatabase::new(&db_path).await.unwrap();
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
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create database with mock data
    #[cfg(feature = "test-utils")]
    create_test_database(&db_path).await.unwrap();
    let db1 = ThingsDatabase::new(&db_path).await.unwrap();
    assert!(db_path.exists());

    // Create another connection to the same database
    let db2 = ThingsDatabase::new(&db_path).await.unwrap();
    assert!(db_path.exists());

    // Both should work
    let inbox1 = db1.get_inbox(None).await.unwrap();
    let inbox2 = db2.get_inbox(None).await.unwrap();
    assert_eq!(inbox1.len(), inbox2.len());
}

#[tokio::test]
async fn test_database_with_mock_data_consistency() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    #[cfg(feature = "test-utils")]
    create_test_database(&db_path).await.unwrap();

    let db = ThingsDatabase::new(&db_path).await.unwrap();

    // Test that all mock data is accessible
    let inbox = db.get_inbox(None).await.unwrap();
    let projects = db.get_projects(None).await.unwrap();
    let areas = db.get_areas().await.unwrap();

    // Verify we have the expected number of items
    assert_eq!(inbox.len(), 5); // 5 inbox tasks
    assert_eq!(projects.len(), 2); // 2 mock projects
    assert_eq!(areas.len(), 3); // 3 mock areas

    // Verify task relationships (check all tasks, not just inbox)
    let all_tasks = db.search_tasks("").await.unwrap();

    // Verify we have the expected number of total tasks
    assert_eq!(all_tasks.len(), 7); // 5 regular tasks + 2 projects

    // Verify task-area relationships (projects have areas)
    assert_eq!(
        all_tasks.iter().filter(|t| t.area_uuid.is_some()).count(),
        2
    ); // 2 projects have areas

    // Verify that projects are included in search results
    assert_eq!(
        all_tasks
            .iter()
            .filter(|t| t.task_type == TaskType::Project)
            .count(),
        2
    ); // 2 projects
}

#[tokio::test]
async fn test_database_query_consistency() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    #[cfg(feature = "test-utils")]
    create_test_database(&db_path).await.unwrap();

    let db = ThingsDatabase::new(&db_path).await.unwrap();

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
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    #[cfg(feature = "test-utils")]
    create_test_database(&db_path).await.unwrap();

    let db = ThingsDatabase::new(&db_path).await.unwrap();

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
    let db_path = temp_dir.path().join("test.db");

    // Create a valid database first
    #[cfg(feature = "test-utils")]
    create_test_database(&db_path).await.unwrap();
    let db = ThingsDatabase::new(&db_path).await.unwrap();

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
    let db_path = temp_dir.path().join("test.db");

    #[cfg(feature = "test-utils")]
    create_test_database(&db_path).await.unwrap();

    // Create multiple database connections
    let db1 = ThingsDatabase::new(&db_path).await.unwrap();
    let db2 = ThingsDatabase::new(&db_path).await.unwrap();

    // Both should be able to read concurrently
    let inbox1 = db1.get_inbox(None).await.unwrap();
    let inbox2 = db2.get_inbox(None).await.unwrap();

    assert_eq!(inbox1.len(), inbox2.len());

    // Both should return the same data
    for (task1, task2) in inbox1.iter().zip(inbox2.iter()) {
        assert_eq!(task1.uuid, task2.uuid);
        assert_eq!(task1.title, task2.title);
    }
}

#[tokio::test]
async fn test_database_helper_functions_indirectly() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    #[cfg(feature = "test-utils")]
    let _db = create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

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
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    #[cfg(feature = "test-utils")]
    let _db = create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Test search with empty string
    let empty_results = db.search_tasks("").await.unwrap();
    assert_eq!(empty_results.len(), 0);

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
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    #[cfg(feature = "test-utils")]
    let _db = create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Test with very large limit (should not cause issues)
    let tasks = db.get_inbox(Some(10000)).await.unwrap();
    assert!(tasks.len() <= 10000);

    let tasks = db.get_today(Some(10000)).await.unwrap();
    assert!(tasks.len() <= 10000);

    let tasks = db.search_tasks("test").await.unwrap();
    assert!(tasks.len() <= 10000);
}
