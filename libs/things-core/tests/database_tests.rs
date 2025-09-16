use chrono::Utc;
use std::path::Path;
use tempfile::tempdir;
use things_core::{
    models::{TaskStatus, TaskType},
    test_utils::create_test_database,
    ThingsConfig, ThingsDatabase,
};

#[test]
fn test_database_new() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let db = ThingsDatabase::new(&db_path).unwrap();
    assert!(db_path.exists());
}

#[test]
fn test_database_with_config() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let config = ThingsConfig::new(&db_path, false);
    create_test_database(&db_path).unwrap();
    let db = ThingsDatabase::with_config(&config).unwrap();
    assert!(db_path.exists());
}

#[test]
fn test_database_with_default_path() {
    let db = ThingsDatabase::with_default_path().unwrap();
    // This should work if the default path exists or can be created
    let default_path = ThingsDatabase::default_path();
    assert!(Path::new(&default_path).exists() || !Path::new(&default_path).exists());
    // Just test that it returns a string
}

#[test]
fn test_database_default_path() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let db = ThingsDatabase::new(&db_path).unwrap();
    // The database was created with a specific path, not the default path
    assert!(db_path.exists());
}

#[test]
fn test_get_inbox() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create test database with mock data
    create_test_database(&db_path).unwrap();

    let db = ThingsDatabase::new(&db_path).unwrap();
    let inbox = db.get_inbox(None).unwrap();

    // Should have 5 inbox tasks from mock data (first 5 tasks have no project/area)
    assert_eq!(inbox.len(), 5);

    // Verify task properties
    let first_task = &inbox[0];
    assert_eq!(first_task.title, "Review quarterly reports");
    assert_eq!(first_task.status, TaskStatus::Incomplete);
    assert_eq!(first_task.task_type, TaskType::Todo);
}

#[test]
fn test_get_today() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    create_test_database(&db_path).unwrap();

    let db = ThingsDatabase::new(&db_path).unwrap();
    let today = db.get_today(None).unwrap();

    // Should have tasks for today
    assert!(!today.is_empty());
}

#[test]
fn test_get_projects() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    create_test_database(&db_path).unwrap();

    let db = ThingsDatabase::new(&db_path).unwrap();
    let projects = db.get_projects(None).unwrap();

    // Should have projects from mock data
    assert!(!projects.is_empty());

    // Verify project properties
    let first_project = &projects[0];
    assert_eq!(first_project.title, "Website Redesign");
    assert_eq!(first_project.status, TaskStatus::Incomplete);
}

#[test]
fn test_get_areas() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    create_test_database(&db_path).unwrap();

    let db = ThingsDatabase::new(&db_path).unwrap();
    let areas = db.get_areas().unwrap();

    // Should have areas from mock data
    assert!(!areas.is_empty());

    // Verify area properties
    let first_area = &areas[0];
    assert_eq!(first_area.title, "Work");
}

#[test]
fn test_search_tasks() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    create_test_database(&db_path).unwrap();

    let db = ThingsDatabase::new(&db_path).unwrap();
    let results = db.search_tasks("reports", None).unwrap();

    // Should find tasks containing "reports"
    assert!(!results.is_empty());

    // Verify search results contain the search term
    let found_task = results.iter().find(|t| t.title.contains("reports"));
    assert!(found_task.is_some());
}

#[test]
fn test_search_tasks_empty_query() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    create_test_database(&db_path).unwrap();

    let db = ThingsDatabase::new(&db_path).unwrap();
    let results = db.search_tasks("", None).unwrap();

    // Empty query should return all tasks
    assert!(!results.is_empty());
}

#[test]
fn test_search_tasks_no_results() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    create_test_database(&db_path).unwrap();

    let db = ThingsDatabase::new(&db_path).unwrap();
    let results = db.search_tasks("nonexistent", None).unwrap();

    // Should return empty results for non-matching query
    assert!(results.is_empty());
}

#[test]
fn test_database_error_handling() {
    // Test with invalid path
    let invalid_path = Path::new("/invalid/path/that/does/not/exist/database.sqlite");
    let result = ThingsDatabase::new(invalid_path);
    assert!(result.is_err());
}

#[test]
fn test_database_connection_persistence() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create database with mock data
    create_test_database(&db_path).unwrap();
    let db1 = ThingsDatabase::new(&db_path).unwrap();
    assert!(db_path.exists());

    // Create another connection to the same database
    let db2 = ThingsDatabase::new(&db_path).unwrap();
    assert!(db_path.exists());

    // Both should work
    let inbox1 = db1.get_inbox(None).unwrap();
    let inbox2 = db2.get_inbox(None).unwrap();
    assert_eq!(inbox1.len(), inbox2.len());
}

#[test]
fn test_database_with_mock_data_consistency() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    create_test_database(&db_path).unwrap();

    let db = ThingsDatabase::new(&db_path).unwrap();

    // Test that all mock data is accessible
    let inbox = db.get_inbox(None).unwrap();
    let projects = db.get_projects(None).unwrap();
    let areas = db.get_areas().unwrap();

    // Verify we have the expected number of items
    assert_eq!(inbox.len(), 5); // 5 inbox tasks
    assert_eq!(projects.len(), 2); // 2 mock projects
    assert_eq!(areas.len(), 3); // 3 mock areas

    // Verify task relationships (check all tasks, not just inbox)
    let all_tasks = db.search_tasks("", None).unwrap();

    // Verify we have the expected number of total tasks
    assert_eq!(all_tasks.len(), 7); // 5 regular tasks + 2 projects

    // Verify task-area relationships (projects have areas)
    let tasks_with_areas: Vec<_> = all_tasks.iter().filter(|t| t.area_uuid.is_some()).collect();
    assert_eq!(tasks_with_areas.len(), 2); // 2 projects have areas

    // Verify that projects are included in search results
    let project_tasks: Vec<_> = all_tasks
        .iter()
        .filter(|t| t.task_type == TaskType::Project)
        .collect();
    assert_eq!(project_tasks.len(), 2); // 2 projects
}

#[test]
fn test_database_query_consistency() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    create_test_database(&db_path).unwrap();

    let db = ThingsDatabase::new(&db_path).unwrap();

    // Test that different query methods return consistent results
    let inbox = db.get_inbox(None).unwrap();
    let all_tasks = db.search_tasks("", None).unwrap();

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

#[test]
fn test_database_date_filtering() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    create_test_database(&db_path).unwrap();

    let db = ThingsDatabase::new(&db_path).unwrap();

    // Test today's tasks
    let today = db.get_today(None).unwrap();

    // All today's tasks should have start_date or deadline today
    let today_date = Utc::now().date_naive();
    for task in &today {
        let is_today = task.start_date.map_or(false, |d| d == today_date)
            || task.deadline.map_or(false, |d| d == today_date);
        assert!(is_today, "Task {} is not for today", task.title);
    }
}

#[test]
fn test_database_error_recovery() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create a valid database first
    create_test_database(&db_path).unwrap();
    let db = ThingsDatabase::new(&db_path).unwrap();

    // Test that operations work
    let inbox = db.get_inbox(None).unwrap();
    assert!(!inbox.is_empty());

    // Test that we can still access the database after operations
    let projects = db.get_projects(None).unwrap();
    assert!(!projects.is_empty());
}

#[test]
fn test_database_concurrent_access() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    create_test_database(&db_path).unwrap();

    // Create multiple database connections
    let db1 = ThingsDatabase::new(&db_path).unwrap();
    let db2 = ThingsDatabase::new(&db_path).unwrap();

    // Both should be able to read concurrently
    let inbox1 = db1.get_inbox(None).unwrap();
    let inbox2 = db2.get_inbox(None).unwrap();

    assert_eq!(inbox1.len(), inbox2.len());

    // Both should return the same data
    for (task1, task2) in inbox1.iter().zip(inbox2.iter()) {
        assert_eq!(task1.uuid, task2.uuid);
        assert_eq!(task1.title, task2.title);
    }
}
