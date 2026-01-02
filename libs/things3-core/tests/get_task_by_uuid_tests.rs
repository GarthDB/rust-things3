use tempfile::NamedTempFile;
use things3_core::{
    test_utils::create_test_database, CreateTaskRequest, DeleteChildHandling, TaskStatus, TaskType,
    ThingsDatabase,
};
use uuid::Uuid;

// Helper function to create a test database and connect
// Returns both the database and the temp file to keep the file alive
async fn create_test_database_and_connect() -> (ThingsDatabase, NamedTempFile) {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();
    (db, temp_file)
}

// ============================================================================
// get_task_by_uuid Tests
// ============================================================================

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_get_task_by_uuid_existing_task() {
    let (db, _temp_file) = create_test_database_and_connect().await;

    // Create a task
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
    let task_uuid = db.create_task(request).await.unwrap();

    // Retrieve it
    let task = db.get_task_by_uuid(&task_uuid).await.unwrap();
    assert!(task.is_some());
    let task = task.unwrap();
    assert_eq!(task.uuid, task_uuid);
    assert_eq!(task.title, "Test Task");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_get_task_by_uuid_nonexistent() {
    let (db, _temp_file) = create_test_database_and_connect().await;

    let nonexistent_uuid = Uuid::new_v4();
    let result = db.get_task_by_uuid(&nonexistent_uuid).await.unwrap();
    assert!(result.is_none(), "Should return None for nonexistent task");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_get_task_by_uuid_trashed_task() {
    let (db, _temp_file) = create_test_database_and_connect().await;

    // Create a task
    let request = CreateTaskRequest {
        title: "Task to Trash".to_string(),
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
    let task_uuid = db.create_task(request).await.unwrap();

    // Delete (trash) it
    db.delete_task(&task_uuid, DeleteChildHandling::Error)
        .await
        .unwrap();

    // Try to retrieve it - should return None
    let result = db.get_task_by_uuid(&task_uuid).await.unwrap();
    assert!(
        result.is_none(),
        "get_task_by_uuid should return None for trashed tasks"
    );
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_get_task_by_uuid_with_project() {
    let (db, _temp_file) = create_test_database_and_connect().await;

    // Create a project
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

    // Create a task in the project
    let task_request = CreateTaskRequest {
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
    let task_uuid = db.create_task(task_request).await.unwrap();

    // Retrieve and verify project reference
    let task = db.get_task_by_uuid(&task_uuid).await.unwrap().unwrap();
    assert_eq!(task.project_uuid, Some(project_uuid));
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_get_task_by_uuid_with_parent() {
    let (db, _temp_file) = create_test_database_and_connect().await;

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

    // Create child task
    let child_request = CreateTaskRequest {
        title: "Child Task".to_string(),
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
    let child_uuid = db.create_task(child_request).await.unwrap();

    // Retrieve and verify parent reference
    let child = db.get_task_by_uuid(&child_uuid).await.unwrap().unwrap();
    assert_eq!(child.parent_uuid, Some(parent_uuid));
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_get_task_by_uuid_completed_task() {
    let (db, _temp_file) = create_test_database_and_connect().await;

    // Create and complete a task
    let request = CreateTaskRequest {
        title: "Task to Complete".to_string(),
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
    let task_uuid = db.create_task(request).await.unwrap();
    db.complete_task(&task_uuid).await.unwrap();

    // Retrieve and verify it's completed
    let task = db.get_task_by_uuid(&task_uuid).await.unwrap().unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
    assert!(task.stop_date.is_some());
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_get_task_by_uuid_all_fields() {
    let (db, _temp_file) = create_test_database_and_connect().await;

    // Create a task with many fields populated
    let start_date = chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    let deadline = chrono::NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();

    let request = CreateTaskRequest {
        title: "Full Task".to_string(),
        task_type: Some(TaskType::Todo),
        notes: Some("Test notes".to_string()),
        start_date: Some(start_date),
        deadline: Some(deadline),
        status: Some(TaskStatus::Incomplete),
        project_uuid: None,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
    };
    let task_uuid = db.create_task(request).await.unwrap();

    // Retrieve and verify all fields
    let task = db.get_task_by_uuid(&task_uuid).await.unwrap().unwrap();
    assert_eq!(task.uuid, task_uuid);
    assert_eq!(task.title, "Full Task");
    assert_eq!(task.task_type, TaskType::Todo);
    assert_eq!(task.notes, Some("Test notes".to_string()));
    assert_eq!(task.start_date, Some(start_date));
    assert_eq!(task.deadline, Some(deadline));
    assert_eq!(task.status, TaskStatus::Incomplete);
    assert!(task.stop_date.is_none());
}
