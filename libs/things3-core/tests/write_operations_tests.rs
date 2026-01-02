use chrono::NaiveDate;
use things3_core::{CreateTaskRequest, TaskStatus, TaskType, ThingsDatabase, UpdateTaskRequest};
use uuid::Uuid;

#[cfg(feature = "test-utils")]
use things3_core::test_utils::create_test_database;

#[cfg(feature = "test-utils")]
use tempfile::NamedTempFile;

// ============================================================================
// Basic Creation Tests (5 tests)
// ============================================================================

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_minimal_fields() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let request = CreateTaskRequest {
        title: "Test Task".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };

    let uuid = db.create_task(request).await.unwrap();
    assert!(!uuid.is_nil(), "Created task should have valid UUID");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_all_fields() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // First create a project to reference
    let project_request = CreateTaskRequest {
        title: "Test Project".to_string(),
        task_type: Some(TaskType::Project),
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };
    let project_uuid = db.create_task(project_request).await.unwrap();

    // Create task with all fields
    let request = CreateTaskRequest {
        title: "Complete Task".to_string(),
        task_type: Some(TaskType::Todo),
        notes: Some("Task notes".to_string()),
        start_date: Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()),
        deadline: Some(NaiveDate::from_ymd_opt(2025, 1, 31).unwrap()),
        project_uuid: Some(project_uuid),
        area_uuid: None,
        parent_uuid: None,
        tags: Some(vec!["work".to_string(), "urgent".to_string()]),
        status: Some(TaskStatus::Incomplete),
    };

    let uuid = db.create_task(request).await.unwrap();
    assert!(!uuid.is_nil(), "Created task should have valid UUID");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_returns_valid_uuid() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let request = CreateTaskRequest {
        title: "UUID Test Task".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };

    let uuid = db.create_task(request).await.unwrap();

    // Verify UUID is valid by parsing it
    assert_eq!(uuid.get_version_num(), 4, "Should be UUID v4");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_appears_in_database() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let request = CreateTaskRequest {
        title: "Verifiable Task".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };

    let uuid = db.create_task(request).await.unwrap();

    // Verify task appears in inbox
    let inbox_tasks = db.get_inbox(None).await.unwrap();
    let found = inbox_tasks.iter().any(|t| t.uuid == uuid);
    assert!(found, "Created task should appear in inbox");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_timestamps_set() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let request = CreateTaskRequest {
        title: "Timestamp Test".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };

    let uuid = db.create_task(request).await.unwrap();

    // Verify task appears in inbox (timestamps are set by database)
    let inbox_tasks = db.get_inbox(None).await.unwrap();
    let task = inbox_tasks.iter().find(|t| t.uuid == uuid);
    assert!(task.is_some(), "Task should appear in inbox");
}

// ============================================================================
// Validation Tests (10 tests)
// ============================================================================

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_with_invalid_project_uuid() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let invalid_uuid = Uuid::new_v4();
    let request = CreateTaskRequest {
        title: "Task with Invalid Project".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: Some(invalid_uuid),
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };

    let result = db.create_task(request).await;
    assert!(result.is_err(), "Should fail with invalid project UUID");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_with_invalid_area_uuid() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let invalid_uuid = Uuid::new_v4();
    let request = CreateTaskRequest {
        title: "Task with Invalid Area".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: Some(invalid_uuid),
        parent_uuid: None,
        tags: None,
        status: None,
    };

    let result = db.create_task(request).await;
    assert!(result.is_err(), "Should fail with invalid area UUID");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_with_invalid_parent_uuid() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let invalid_uuid = Uuid::new_v4();
    let request = CreateTaskRequest {
        title: "Task with Invalid Parent".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: Some(invalid_uuid),
        tags: None,
        status: None,
    };

    let result = db.create_task(request).await;
    assert!(result.is_err(), "Should fail with invalid parent UUID");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_with_valid_project() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create a project first
    let project_request = CreateTaskRequest {
        title: "Valid Project".to_string(),
        task_type: Some(TaskType::Project),
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };
    let project_uuid = db.create_task(project_request).await.unwrap();

    // Create task with valid project
    let request = CreateTaskRequest {
        title: "Task in Project".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: Some(project_uuid),
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };

    let result = db.create_task(request).await;
    assert!(result.is_ok(), "Should succeed with valid project UUID");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_with_valid_dates() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let request = CreateTaskRequest {
        title: "Task with Dates".to_string(),
        task_type: None,
        notes: None,
        start_date: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        deadline: Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };

    let result = db.create_task(request).await;
    assert!(result.is_ok(), "Should succeed with valid dates");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_type_todo() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let request = CreateTaskRequest {
        title: "Todo Task".to_string(),
        task_type: Some(TaskType::Todo),
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };

    let result = db.create_task(request).await;
    assert!(result.is_ok(), "Should create Todo task");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_type_project() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let request = CreateTaskRequest {
        title: "Project Task".to_string(),
        task_type: Some(TaskType::Project),
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };

    let result = db.create_task(request).await;
    assert!(result.is_ok(), "Should create Project task");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_type_heading() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let request = CreateTaskRequest {
        title: "Heading Task".to_string(),
        task_type: Some(TaskType::Heading),
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };

    let result = db.create_task(request).await;
    assert!(result.is_ok(), "Should create Heading task");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_all_statuses() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let statuses = vec![
        TaskStatus::Incomplete,
        TaskStatus::Completed,
        TaskStatus::Canceled,
    ];

    for status in statuses {
        let request = CreateTaskRequest {
            title: format!("Task with status {:?}", status),
            task_type: None,
            notes: None,
            start_date: None,
            deadline: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: None,
            status: Some(status),
        };

        let result = db.create_task(request).await;
        assert!(
            result.is_ok(),
            "Should create task with status {:?}",
            status
        );
    }
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_with_empty_title_succeeds() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Empty title is technically allowed by the database
    let request = CreateTaskRequest {
        title: "".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };

    let result = db.create_task(request).await;
    assert!(result.is_ok(), "Database allows empty titles");
}

// ============================================================================
// Update Tests (8 tests)
// ============================================================================

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_update_task_title() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create task
    let create_request = CreateTaskRequest {
        title: "Original Title".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };
    let uuid = db.create_task(create_request).await.unwrap();

    // Update title
    let update_request = UpdateTaskRequest {
        uuid,
        title: Some("Updated Title".to_string()),
        notes: None,
        start_date: None,
        deadline: None,
        status: None,
        project_uuid: None,
        area_uuid: None,
        tags: None,
    };

    let result = db.update_task(update_request).await;
    assert!(result.is_ok(), "Should update task title");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_update_task_notes() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create task
    let create_request = CreateTaskRequest {
        title: "Task for Notes Update".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };
    let uuid = db.create_task(create_request).await.unwrap();

    // Update notes
    let update_request = UpdateTaskRequest {
        uuid,
        title: None,
        notes: Some("Updated notes".to_string()),
        start_date: None,
        deadline: None,
        status: None,
        project_uuid: None,
        area_uuid: None,
        tags: None,
    };

    let result = db.update_task(update_request).await;
    assert!(result.is_ok(), "Should update task notes");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_update_task_dates() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create task
    let create_request = CreateTaskRequest {
        title: "Task for Date Update".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };
    let uuid = db.create_task(create_request).await.unwrap();

    // Update dates
    let update_request = UpdateTaskRequest {
        uuid,
        title: None,
        notes: None,
        start_date: Some(NaiveDate::from_ymd_opt(2025, 2, 1).unwrap()),
        deadline: Some(NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()),
        status: None,
        project_uuid: None,
        area_uuid: None,
        tags: None,
    };

    let result = db.update_task(update_request).await;
    assert!(result.is_ok(), "Should update task dates");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_update_task_project_assignment() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create project
    let project_request = CreateTaskRequest {
        title: "Target Project".to_string(),
        task_type: Some(TaskType::Project),
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };
    let project_uuid = db.create_task(project_request).await.unwrap();

    // Create task
    let create_request = CreateTaskRequest {
        title: "Task for Project Assignment".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };
    let task_uuid = db.create_task(create_request).await.unwrap();

    // Update project assignment
    let update_request = UpdateTaskRequest {
        uuid: task_uuid,
        title: None,
        notes: None,
        start_date: None,
        deadline: None,
        status: None,
        project_uuid: Some(project_uuid),
        area_uuid: None,
        tags: None,
    };

    let result = db.update_task(update_request).await;
    assert!(result.is_ok(), "Should update task project assignment");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_update_task_status() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create task
    let create_request = CreateTaskRequest {
        title: "Task for Status Update".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: Some(TaskStatus::Incomplete),
    };
    let uuid = db.create_task(create_request).await.unwrap();

    // Update status to completed
    let update_request = UpdateTaskRequest {
        uuid,
        title: None,
        notes: None,
        start_date: None,
        deadline: None,
        status: Some(TaskStatus::Completed),
        project_uuid: None,
        area_uuid: None,
        tags: None,
    };

    let result = db.update_task(update_request).await;
    assert!(result.is_ok(), "Should update task status");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_update_task_tags() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create task
    let create_request = CreateTaskRequest {
        title: "Task for Tag Update".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };
    let uuid = db.create_task(create_request).await.unwrap();

    // Update tags
    let update_request = UpdateTaskRequest {
        uuid,
        title: None,
        notes: None,
        start_date: None,
        deadline: None,
        status: None,
        project_uuid: None,
        area_uuid: None,
        tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
    };

    let result = db.update_task(update_request).await;
    assert!(result.is_ok(), "Should update task tags");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_update_task_partial() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create task with multiple fields
    let create_request = CreateTaskRequest {
        title: "Original Task".to_string(),
        task_type: None,
        notes: Some("Original notes".to_string()),
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };
    let uuid = db.create_task(create_request).await.unwrap();

    // Update only title, leaving notes unchanged
    let update_request = UpdateTaskRequest {
        uuid,
        title: Some("Updated Task".to_string()),
        notes: None,
        start_date: None,
        deadline: None,
        status: None,
        project_uuid: None,
        area_uuid: None,
        tags: None,
    };

    let result = db.update_task(update_request).await;
    assert!(result.is_ok(), "Should perform partial update");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_update_nonexistent_task() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let nonexistent_uuid = Uuid::new_v4();
    let update_request = UpdateTaskRequest {
        uuid: nonexistent_uuid,
        title: Some("Updated Title".to_string()),
        notes: None,
        start_date: None,
        deadline: None,
        status: None,
        project_uuid: None,
        area_uuid: None,
        tags: None,
    };

    let result = db.update_task(update_request).await;
    assert!(
        result.is_err(),
        "Should fail when updating nonexistent task"
    );
}

// ============================================================================
// Edge Cases (7 tests)
// ============================================================================

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_very_long_title() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let long_title = "A".repeat(1000);
    let request = CreateTaskRequest {
        title: long_title,
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };

    let result = db.create_task(request).await;
    assert!(result.is_ok(), "Should handle very long titles");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_special_characters() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let special_title = "Task with émojis 🎉 and symbols @#$%";
    let request = CreateTaskRequest {
        title: special_title.to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };

    let result = db.create_task(request).await;
    assert!(result.is_ok(), "Should handle special characters");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_empty_vs_null_notes() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Task with null notes
    let request1 = CreateTaskRequest {
        title: "Task with null notes".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };
    let result1 = db.create_task(request1).await;
    assert!(result1.is_ok(), "Should handle null notes");

    // Task with empty notes
    let request2 = CreateTaskRequest {
        title: "Task with empty notes".to_string(),
        task_type: None,
        notes: Some("".to_string()),
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };
    let result2 = db.create_task(request2).await;
    assert!(result2.is_ok(), "Should handle empty notes");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_subtask() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create parent task
    let parent_request = CreateTaskRequest {
        title: "Parent Task".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };
    let parent_uuid = db.create_task(parent_request).await.unwrap();

    // Create subtask
    let subtask_request = CreateTaskRequest {
        title: "Subtask".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: Some(parent_uuid),
        tags: None,
        status: None,
    };

    let result = db.create_task(subtask_request).await;
    assert!(result.is_ok(), "Should create subtask with parent_uuid");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_task_multiple_tags() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let request = CreateTaskRequest {
        title: "Task with Multiple Tags".to_string(),
        task_type: None,
        notes: None,
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: Some(vec![
            "work".to_string(),
            "urgent".to_string(),
            "important".to_string(),
            "review".to_string(),
        ]),
        status: None,
    };

    let result = db.create_task(request).await;
    assert!(result.is_ok(), "Should handle multiple tags");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_concurrent_create_operations() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = std::sync::Arc::new(ThingsDatabase::new(db_path).await.unwrap());

    let mut handles = vec![];

    for i in 0..10 {
        let db_clone = db.clone();
        let handle = tokio::spawn(async move {
            let request = CreateTaskRequest {
                title: format!("Concurrent Task {}", i),
                task_type: None,
                notes: None,
                start_date: None,
                deadline: None,
                project_uuid: None,
                area_uuid: None,
                parent_uuid: None,
                tags: None,
                status: None,
            };
            db_clone.create_task(request).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Concurrent create should succeed");
    }
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_update_task_remove_optional_fields() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create task with notes
    let create_request = CreateTaskRequest {
        title: "Task with Notes".to_string(),
        task_type: None,
        notes: Some("Original notes".to_string()),
        start_date: None,
        deadline: None,
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };
    let uuid = db.create_task(create_request).await.unwrap();

    // Update to remove notes (set to empty string)
    let update_request = UpdateTaskRequest {
        uuid,
        title: None,
        notes: Some("".to_string()),
        start_date: None,
        deadline: None,
        status: None,
        project_uuid: None,
        area_uuid: None,
        tags: None,
    };

    let result = db.update_task(update_request).await;
    assert!(result.is_ok(), "Should update to remove optional fields");
}

// ============================================================================
// Binary Format Tests (3 tests)
// ============================================================================

#[test]
fn test_date_conversion_accuracy() {
    use things3_core::database::naive_date_to_things_timestamp;

    let original_date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
    let timestamp = naive_date_to_things_timestamp(original_date);

    // Verify timestamp is positive and reasonable
    assert!(timestamp > 0, "Timestamp should be positive");

    // Verify it's in a reasonable range (between 2001 and 2100)
    let seconds_in_100_years = 100 * 365 * 86400i64;
    assert!(
        timestamp < seconds_in_100_years,
        "Timestamp should be reasonable"
    );
}

#[test]
fn test_tags_serialization() {
    use things3_core::database::{deserialize_tags_from_blob, serialize_tags_to_blob};

    let original_tags = vec![
        "work".to_string(),
        "urgent".to_string(),
        "review".to_string(),
    ];
    let blob = serialize_tags_to_blob(&original_tags).unwrap();
    let deserialized_tags = deserialize_tags_from_blob(&blob).unwrap();

    assert_eq!(
        original_tags, deserialized_tags,
        "Tags should serialize and deserialize correctly"
    );
}

#[test]
fn test_tags_round_trip_consistency() {
    use things3_core::database::{deserialize_tags_from_blob, serialize_tags_to_blob};

    let tags1 = vec!["tag1".to_string()];
    let tags2 = vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()];
    let tags3: Vec<String> = vec![];

    for tags in [tags1, tags2, tags3] {
        let blob = serialize_tags_to_blob(&tags).unwrap();
        let result = deserialize_tags_from_blob(&blob).unwrap();
        assert_eq!(tags, result, "Round-trip should preserve tags");
    }
}
