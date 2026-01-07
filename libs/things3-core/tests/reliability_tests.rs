#![cfg(feature = "test-utils")]

use std::sync::Arc;
use things3_core::test_utils::create_test_database;
use things3_core::{CreateTaskRequest, ThingsDatabase};
use tokio::task::JoinSet;

/// Test concurrent read operations
#[tokio::test]
async fn test_concurrent_reads() {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_path_buf();

    create_test_database(&db_path).await.unwrap();
    let db = Arc::new(ThingsDatabase::new(&db_path).await.unwrap());

    // Create test data
    for i in 0..100 {
        let request = CreateTaskRequest {
            title: format!("Concurrent Test Task {}", i),
            notes: Some(format!("Task for concurrent testing {}", i)),
            deadline: None,
            start_date: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: None,
            task_type: None,
            status: None,
        };
        db.create_task(request).await.unwrap();
    }

    // Spawn 20 concurrent read operations
    let mut join_set = JoinSet::new();

    for i in 0..20 {
        let db_clone = Arc::clone(&db);
        join_set.spawn(async move {
            // Each task performs multiple reads
            for _ in 0..5 {
                let inbox = db_clone.get_inbox(Some(50)).await.unwrap();
                assert!(!inbox.is_empty(), "Task {} got empty inbox", i);

                let search = db_clone.search_tasks("Test").await.unwrap();
                assert!(!search.is_empty(), "Task {} got empty search results", i);
            }
        });
    }

    // Wait for all tasks to complete
    while let Some(result) = join_set.join_next().await {
        result.unwrap();
    }
}

/// Test concurrent write operations (should be serialized by SQLite)
#[tokio::test]
async fn test_concurrent_writes() {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_path_buf();

    create_test_database(&db_path).await.unwrap();
    let db = Arc::new(ThingsDatabase::new(&db_path).await.unwrap());

    // Spawn 10 concurrent write operations
    let mut join_set = JoinSet::new();

    for i in 0..10 {
        let db_clone = Arc::clone(&db);
        join_set.spawn(async move {
            for j in 0..10 {
                let request = CreateTaskRequest {
                    title: format!("Concurrent Write Task {}-{}", i, j),
                    notes: Some(format!("Task from thread {}", i)),
                    deadline: None,
                    start_date: None,
                    project_uuid: None,
                    area_uuid: None,
                    parent_uuid: None,
                    tags: None,
                    task_type: None,
                    status: None,
                };
                db_clone.create_task(request).await.unwrap();
            }
        });
    }

    // Wait for all tasks to complete
    while let Some(result) = join_set.join_next().await {
        result.unwrap();
    }

    // Verify all tasks were created (at least 100, may include test data)
    let all_tasks = db.get_inbox(None).await.unwrap();
    assert!(
        all_tasks.len() >= 100,
        "Expected at least 100 tasks, got {}",
        all_tasks.len()
    );
}

/// Test mixed concurrent read/write operations
#[tokio::test]
async fn test_concurrent_mixed_operations() {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_path_buf();

    create_test_database(&db_path).await.unwrap();
    let db = Arc::new(ThingsDatabase::new(&db_path).await.unwrap());

    // Create initial data
    for i in 0..50 {
        let request = CreateTaskRequest {
            title: format!("Initial Task {}", i),
            notes: None,
            deadline: None,
            start_date: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: None,
            task_type: None,
            status: None,
        };
        db.create_task(request).await.unwrap();
    }

    let mut join_set = JoinSet::new();

    // Spawn readers
    for i in 0..10 {
        let db_clone = Arc::clone(&db);
        join_set.spawn(async move {
            for _ in 0..10 {
                let _ = db_clone.get_inbox(Some(20)).await.unwrap();
                let _ = db_clone.search_tasks("Task").await.unwrap();
            }
            i
        });
    }

    // Spawn writers
    for i in 0..5 {
        let db_clone = Arc::clone(&db);
        join_set.spawn(async move {
            for j in 0..5 {
                let request = CreateTaskRequest {
                    title: format!("New Task {}-{}", i, j),
                    notes: None,
                    deadline: None,
                    start_date: None,
                    project_uuid: None,
                    area_uuid: None,
                    parent_uuid: None,
                    tags: None,
                    task_type: None,
                    status: None,
                };
                db_clone.create_task(request).await.unwrap();
            }
            i + 100
        });
    }

    // Wait for all operations to complete
    while let Some(result) = join_set.join_next().await {
        result.unwrap();
    }

    // Verify database is in consistent state (at least 75 tasks, may include test data)
    let final_tasks = db.get_inbox(None).await.unwrap();
    assert!(
        final_tasks.len() >= 75,
        "Expected at least 75 tasks (50 initial + 25 new), got {}",
        final_tasks.len()
    );
}

/// Test database with empty data
#[tokio::test]
async fn test_empty_database() {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_path_buf();

    create_test_database(&db_path).await.unwrap();
    let db = ThingsDatabase::new(&db_path).await.unwrap();

    // All queries should succeed (may have test data from schema)
    let _inbox = db.get_inbox(None).await.unwrap();
    // Just verify it doesn't error

    let _today = db.get_today(None).await.unwrap();
    // Just verify it doesn't error

    let search = db.search_tasks("nonexistent").await.unwrap();
    assert!(
        search.is_empty(),
        "Search for nonexistent should return empty"
    );

    let _projects = db.get_projects(None).await.unwrap();
    // Just verify it doesn't error

    let _areas = db.get_areas().await.unwrap();
    // Just verify it doesn't error

    let _stats = db.get_stats().await.unwrap();
    // Just verify stats are returned (may have test data)
}

/// Test database with large dataset
#[tokio::test]
async fn test_large_dataset() {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_path_buf();

    create_test_database(&db_path).await.unwrap();
    let db = ThingsDatabase::new(&db_path).await.unwrap();

    // Create 1000 tasks
    for i in 0..1000 {
        let request = CreateTaskRequest {
            title: format!("Large Dataset Task {}", i),
            notes: Some(format!("Task number {} of 1000", i)),
            deadline: None,
            start_date: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: None,
            task_type: None,
            status: None,
        };
        db.create_task(request).await.unwrap();
    }

    // Verify queries still work efficiently
    let inbox = db.get_inbox(Some(100)).await.unwrap();
    assert_eq!(inbox.len(), 100);

    let search = db.search_tasks("Dataset").await.unwrap();
    assert_eq!(search.len(), 1000);

    let stats = db.get_stats().await.unwrap();
    assert!(
        stats.task_count >= 1000,
        "Expected at least 1000 tasks, got {}",
        stats.task_count
    );
}

/// Test resource cleanup (connections are properly released)
#[tokio::test]
async fn test_resource_cleanup() {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_path_buf();

    create_test_database(&db_path).await.unwrap();

    // Create and drop multiple database instances
    for _ in 0..10 {
        let db = ThingsDatabase::new(&db_path).await.unwrap();
        let _ = db.get_inbox(None).await.unwrap();
        // db is dropped here, connections should be released
    }

    // Final connection should still work
    let db = ThingsDatabase::new(&db_path).await.unwrap();
    let _inbox = db.get_inbox(None).await.unwrap();
    // Just verify it doesn't error (may have test data)
}

/// Test multiple database instances (connection pool isolation)
#[tokio::test]
async fn test_multiple_database_instances() {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_path_buf();

    create_test_database(&db_path).await.unwrap();

    // Create multiple instances pointing to same database
    let db1 = Arc::new(ThingsDatabase::new(&db_path).await.unwrap());
    let db2 = Arc::new(ThingsDatabase::new(&db_path).await.unwrap());
    let db3 = Arc::new(ThingsDatabase::new(&db_path).await.unwrap());

    // Write from db1
    let request = CreateTaskRequest {
        title: "Test Task from DB1".to_string(),
        notes: None,
        deadline: None,
        start_date: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        task_type: None,
        status: None,
    };
    db1.create_task(request).await.unwrap();

    // Read from db2 and db3
    let tasks2 = db2.get_inbox(None).await.unwrap();
    let tasks3 = db3.get_inbox(None).await.unwrap();

    // Both should see the same data (at least 1 task)
    assert!(!tasks2.is_empty(), "DB2 should see at least 1 task");
    assert!(!tasks3.is_empty(), "DB3 should see at least 1 task");

    // Find our task in the results
    let found_in_db2 = tasks2.iter().any(|t| t.title == "Test Task from DB1");
    let found_in_db3 = tasks3.iter().any(|t| t.title == "Test Task from DB1");

    assert!(found_in_db2, "DB2 should see the task from DB1");
    assert!(found_in_db3, "DB3 should see the task from DB1");
}

/// Test error recovery from invalid operations
#[tokio::test]
async fn test_error_recovery() {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_path_buf();

    create_test_database(&db_path).await.unwrap();
    let db = ThingsDatabase::new(&db_path).await.unwrap();

    // Try to complete a non-existent task
    let fake_uuid = uuid::Uuid::new_v4();
    let result = db.complete_task(&fake_uuid).await;
    assert!(result.is_err(), "Expected error for non-existent task");

    // Database should still be functional after error
    let _inbox = db.get_inbox(None).await.unwrap();
    // Just verify it doesn't error

    // Create a valid task
    let request = CreateTaskRequest {
        title: "Recovery Test Task".to_string(),
        notes: None,
        deadline: None,
        start_date: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        task_type: None,
        status: None,
    };
    let task_uuid = db.create_task(request).await.unwrap();

    // Complete the valid task
    db.complete_task(&task_uuid).await.unwrap();

    // Verify task is completed (not in inbox)
    let inbox = db.get_inbox(None).await.unwrap();
    let found = inbox.iter().any(|t| t.uuid == task_uuid);
    assert!(!found, "Completed task should not be in inbox");
}
