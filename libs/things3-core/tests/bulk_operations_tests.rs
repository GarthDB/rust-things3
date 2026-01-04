//! Comprehensive tests for bulk operations

use chrono::NaiveDate;
use things3_core::models::{
    BulkCompleteRequest, BulkDeleteRequest, BulkMoveRequest, BulkUpdateDatesRequest,
};
use things3_core::test_utils::{create_test_database_and_connect, TaskRequestBuilder};
use things3_core::ThingsError;
use uuid::Uuid;

// ============================================================================
// Bulk Move Tests
// ============================================================================

#[tokio::test]
async fn test_bulk_move_to_project() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create a project
    let project_request = things3_core::models::CreateProjectRequest {
        title: "Target Project".to_string(),
        notes: None,
        area_uuid: None,
        start_date: None,
        deadline: None,
        tags: None,
    };
    let project_uuid = db.create_project(project_request).await.unwrap();

    // Create 3 tasks
    let mut task_uuids = Vec::new();
    for i in 1..=3 {
        let request = TaskRequestBuilder::new()
            .title(format!("Task {}", i))
            .build();
        let task_uuid = db.create_task(request).await.unwrap();
        task_uuids.push(task_uuid);
    }

    // Bulk move to project
    let bulk_request = BulkMoveRequest {
        task_uuids: task_uuids.clone(),
        project_uuid: Some(project_uuid),
        area_uuid: None,
    };

    let result = db.bulk_move(bulk_request).await.unwrap();
    assert!(result.success);
    assert_eq!(result.processed_count, 3);

    // Verify all tasks now have the project
    for uuid in &task_uuids {
        let task = db.get_task_by_uuid(uuid).await.unwrap().unwrap();
        assert_eq!(task.project_uuid, Some(project_uuid));
    }
}

#[tokio::test]
async fn test_bulk_move_to_area() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create an area
    let area_request = things3_core::models::CreateAreaRequest {
        title: "Target Area".to_string(),
    };
    let area_uuid = db.create_area(area_request).await.unwrap();

    // Create 3 tasks
    let mut task_uuids = Vec::new();
    for i in 1..=3 {
        let request = TaskRequestBuilder::new()
            .title(format!("Task {}", i))
            .build();
        let task_uuid = db.create_task(request).await.unwrap();
        task_uuids.push(task_uuid);
    }

    // Bulk move to area
    let bulk_request = BulkMoveRequest {
        task_uuids: task_uuids.clone(),
        project_uuid: None,
        area_uuid: Some(area_uuid),
    };

    let result = db.bulk_move(bulk_request).await.unwrap();
    assert!(result.success);
    assert_eq!(result.processed_count, 3);

    // Verify all tasks now have the area
    for uuid in &task_uuids {
        let task = db.get_task_by_uuid(uuid).await.unwrap().unwrap();
        assert_eq!(task.area_uuid, Some(area_uuid));
    }
}

#[tokio::test]
async fn test_bulk_move_empty_uuids() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    let bulk_request = BulkMoveRequest {
        task_uuids: vec![],
        project_uuid: Some(Uuid::new_v4()),
        area_uuid: None,
    };

    let result = db.bulk_move(bulk_request).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(ThingsError::Validation { .. })));
}

#[tokio::test]
async fn test_bulk_move_invalid_uuid() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create a project
    let project_request = things3_core::models::CreateProjectRequest {
        title: "Target Project".to_string(),
        notes: None,
        area_uuid: None,
        start_date: None,
        deadline: None,
        tags: None,
    };
    let project_uuid = db.create_project(project_request).await.unwrap();

    // Create one valid task
    let request = TaskRequestBuilder::new().title("Valid Task").build();
    let valid_uuid = db.create_task(request).await.unwrap();

    // Try to bulk move with one valid and one invalid UUID
    let invalid_uuid = Uuid::new_v4(); // Doesn't exist in database
    let bulk_request = BulkMoveRequest {
        task_uuids: vec![valid_uuid, invalid_uuid],
        project_uuid: Some(project_uuid),
        area_uuid: None,
    };

    let result = db.bulk_move(bulk_request).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(ThingsError::TaskNotFound { .. })));

    // Verify the valid task was NOT moved (transaction rolled back)
    let task = db.get_task_by_uuid(&valid_uuid).await.unwrap().unwrap();
    assert_eq!(
        task.project_uuid, None,
        "Task should not be moved due to rollback"
    );
}

#[tokio::test]
async fn test_bulk_move_nonexistent_project() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create a task
    let request = TaskRequestBuilder::new().title("Test Task").build();
    let task_uuid = db.create_task(request).await.unwrap();

    // Try to move to non-existent project
    let fake_project_uuid = Uuid::new_v4();
    let bulk_request = BulkMoveRequest {
        task_uuids: vec![task_uuid],
        project_uuid: Some(fake_project_uuid),
        area_uuid: None,
    };

    let result = db.bulk_move(bulk_request).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(ThingsError::ProjectNotFound { .. })));
}

// ============================================================================
// Bulk Update Dates Tests
// ============================================================================

#[tokio::test]
async fn test_bulk_update_dates_both() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create 3 tasks
    let mut task_uuids = Vec::new();
    for i in 1..=3 {
        let request = TaskRequestBuilder::new()
            .title(format!("Task {}", i))
            .build();
        let task_uuid = db.create_task(request).await.unwrap();
        task_uuids.push(task_uuid);
    }

    // Bulk update dates
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let deadline = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    let bulk_request = BulkUpdateDatesRequest {
        task_uuids: task_uuids.clone(),
        start_date: Some(start_date),
        deadline: Some(deadline),
        clear_start_date: false,
        clear_deadline: false,
    };

    let result = db.bulk_update_dates(bulk_request).await.unwrap();
    assert!(result.success);
    assert_eq!(result.processed_count, 3);

    // Verify all tasks have the new dates
    for uuid in &task_uuids {
        let task = db.get_task_by_uuid(uuid).await.unwrap().unwrap();
        assert_eq!(task.start_date, Some(start_date));
        assert_eq!(task.deadline, Some(deadline));
    }
}

#[tokio::test]
async fn test_bulk_update_dates_clear() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create tasks with dates
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let deadline = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    let mut task_uuids = Vec::new();
    for i in 1..=2 {
        let request = TaskRequestBuilder::new()
            .title(format!("Task {}", i))
            .start_date(start_date)
            .deadline(deadline)
            .build();
        let task_uuid = db.create_task(request).await.unwrap();
        task_uuids.push(task_uuid);
    }

    // Bulk clear dates
    let bulk_request = BulkUpdateDatesRequest {
        task_uuids: task_uuids.clone(),
        start_date: None,
        deadline: None,
        clear_start_date: true,
        clear_deadline: true,
    };

    let result = db.bulk_update_dates(bulk_request).await.unwrap();
    assert!(result.success);
    assert_eq!(result.processed_count, 2);

    // Verify dates are cleared
    for uuid in &task_uuids {
        let task = db.get_task_by_uuid(uuid).await.unwrap().unwrap();
        assert_eq!(task.start_date, None);
        assert_eq!(task.deadline, None);
    }
}

#[tokio::test]
async fn test_bulk_update_dates_invalid_range() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create a task
    let request = TaskRequestBuilder::new().title("Test Task").build();
    let task_uuid = db.create_task(request).await.unwrap();

    // Try to set deadline before start_date
    let bulk_request = BulkUpdateDatesRequest {
        task_uuids: vec![task_uuid],
        start_date: Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
        deadline: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
        clear_start_date: false,
        clear_deadline: false,
    };

    let result = db.bulk_update_dates(bulk_request).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(ThingsError::DateValidation(_))));
}

#[tokio::test]
async fn test_bulk_update_dates_merge_validation() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create task with existing start date
    let start_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let request = TaskRequestBuilder::new()
        .title("Test Task")
        .start_date(start_date)
        .build();
    let task_uuid = db.create_task(request).await.unwrap();

    // Try to set deadline before existing start_date
    let bulk_request = BulkUpdateDatesRequest {
        task_uuids: vec![task_uuid],
        start_date: None, // Don't update start_date
        deadline: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()), // Before existing start
        clear_start_date: false,
        clear_deadline: false,
    };

    let result = db.bulk_update_dates(bulk_request).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(ThingsError::DateValidation(_))));
}

// ============================================================================
// Bulk Complete Tests
// ============================================================================

#[tokio::test]
async fn test_bulk_complete_multiple() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create 5 tasks
    let mut task_uuids = Vec::new();
    for i in 1..=5 {
        let request = TaskRequestBuilder::new()
            .title(format!("Task {}", i))
            .build();
        let task_uuid = db.create_task(request).await.unwrap();
        task_uuids.push(task_uuid);
    }

    // Bulk complete
    let bulk_request = BulkCompleteRequest {
        task_uuids: task_uuids.clone(),
    };

    let result = db.bulk_complete(bulk_request).await.unwrap();
    assert!(result.success);
    assert_eq!(result.processed_count, 5);

    // Verify all tasks are completed
    for uuid in &task_uuids {
        let task = db.get_task_by_uuid(uuid).await.unwrap().unwrap();
        assert_eq!(task.status, things3_core::models::TaskStatus::Completed);
        assert!(task.stop_date.is_some());
    }
}

#[tokio::test]
async fn test_bulk_complete_already_completed() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create a task and complete it
    let request = TaskRequestBuilder::new().title("Task 1").build();
    let task_uuid = db.create_task(request).await.unwrap();
    db.complete_task(&task_uuid).await.unwrap();

    // Try to bulk complete again (should succeed, idempotent)
    let bulk_request = BulkCompleteRequest {
        task_uuids: vec![task_uuid],
    };

    let result = db.bulk_complete(bulk_request).await;
    assert!(result.is_ok());
}

// ============================================================================
// Bulk Delete Tests
// ============================================================================

#[tokio::test]
async fn test_bulk_delete_soft_delete() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create 3 tasks
    let mut task_uuids = Vec::new();
    for i in 1..=3 {
        let request = TaskRequestBuilder::new()
            .title(format!("Task {}", i))
            .build();
        let task_uuid = db.create_task(request).await.unwrap();
        task_uuids.push(task_uuid);
    }

    // Bulk delete
    let bulk_request = BulkDeleteRequest {
        task_uuids: task_uuids.clone(),
    };

    let result = db.bulk_delete(bulk_request).await.unwrap();
    assert!(result.success);
    assert_eq!(result.processed_count, 3);

    // Verify all tasks are soft-deleted (trashed)
    for uuid in &task_uuids {
        let task = db.get_task_by_uuid(uuid).await.unwrap();
        assert!(task.is_none(), "Deleted task should not be returned");
    }
}

#[tokio::test]
async fn test_bulk_delete_empty_uuids() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    let bulk_request = BulkDeleteRequest { task_uuids: vec![] };

    let result = db.bulk_delete(bulk_request).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(ThingsError::Validation { .. })));
}

// ============================================================================
// Transaction Rollback Tests
// ============================================================================

#[tokio::test]
async fn test_transaction_rollback_on_error() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create 2 valid tasks
    let request1 = TaskRequestBuilder::new().title("Task 1").build();
    let uuid1 = db.create_task(request1).await.unwrap();
    let request2 = TaskRequestBuilder::new().title("Task 2").build();
    let uuid2 = db.create_task(request2).await.unwrap();

    // Try to complete with one valid and one invalid UUID
    let invalid_uuid = Uuid::new_v4();
    let bulk_request = BulkCompleteRequest {
        task_uuids: vec![uuid1, uuid2, invalid_uuid],
    };

    let result = db.bulk_complete(bulk_request).await;
    assert!(result.is_err());

    // Verify NO tasks were completed (transaction rolled back)
    let task1 = db.get_task_by_uuid(&uuid1).await.unwrap().unwrap();
    let task2 = db.get_task_by_uuid(&uuid2).await.unwrap().unwrap();
    assert_eq!(task1.status, things3_core::models::TaskStatus::Incomplete);
    assert_eq!(task2.status, things3_core::models::TaskStatus::Incomplete);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test]
async fn test_bulk_operations_with_single_task() {
    let (db, _temp_file) = create_test_database_and_connect().await.unwrap();

    // Create one task
    let request = TaskRequestBuilder::new().title("Single Task").build();
    let task_uuid = db.create_task(request).await.unwrap();

    // Test bulk complete with single task
    let bulk_request = BulkCompleteRequest {
        task_uuids: vec![task_uuid],
    };

    let result = db.bulk_complete(bulk_request).await.unwrap();
    assert!(result.success);
    assert_eq!(result.processed_count, 1);
}
