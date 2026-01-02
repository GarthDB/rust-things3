//! Tests for project and area operations

use things3_core::{
    models::{
        CreateAreaRequest, CreateProjectRequest, ProjectChildHandling, UpdateAreaRequest,
        UpdateProjectRequest,
    },
    ThingsDatabase,
};

#[cfg(feature = "test-utils")]
use things3_core::test_utils::{create_test_database, TaskRequestBuilder};

#[cfg(feature = "test-utils")]
use tempfile::NamedTempFile;

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_project_success() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let request = CreateProjectRequest {
        title: "Test Project".to_string(),
        notes: Some("Project notes".to_string()),
        area_uuid: None,
        start_date: None,
        deadline: None,
        tags: Some(vec!["test".to_string()]),
    };

    let uuid = db.create_project(request).await.unwrap();
    assert!(!uuid.is_nil());

    // Verify it was created as a project (type = 1)
    let task = db.get_task_by_uuid(&uuid).await.unwrap();
    assert!(task.is_some());
    let task = task.unwrap();
    assert_eq!(task.title, "Test Project");
    assert_eq!(task.task_type, things3_core::models::TaskType::Project);
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_project_with_area() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create an area first
    let area_request = CreateAreaRequest {
        title: "Work".to_string(),
    };
    let area_uuid = db.create_area(area_request).await.unwrap();

    // Create project in that area
    let request = CreateProjectRequest {
        title: "Work Project".to_string(),
        notes: None,
        area_uuid: Some(area_uuid),
        start_date: None,
        deadline: None,
        tags: None,
    };

    let project_uuid = db.create_project(request).await.unwrap();
    let task = db.get_task_by_uuid(&project_uuid).await.unwrap().unwrap();
    assert_eq!(task.area_uuid, Some(area_uuid));
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_update_project_success() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create a project
    let create_request = CreateProjectRequest {
        title: "Original Title".to_string(),
        notes: None,
        area_uuid: None,
        start_date: None,
        deadline: None,
        tags: None,
    };
    let uuid = db.create_project(create_request).await.unwrap();

    // Update it
    let update_request = UpdateProjectRequest {
        uuid,
        title: Some("Updated Title".to_string()),
        notes: Some("New notes".to_string()),
        area_uuid: None,
        start_date: None,
        deadline: None,
        tags: None,
    };
    db.update_project(update_request).await.unwrap();

    // Verify update
    let task = db.get_task_by_uuid(&uuid).await.unwrap().unwrap();
    assert_eq!(task.title, "Updated Title");
    assert_eq!(task.notes, Some("New notes".to_string()));
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_complete_project_success() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create a project
    let request = CreateProjectRequest {
        title: "Project to Complete".to_string(),
        notes: None,
        area_uuid: None,
        start_date: None,
        deadline: None,
        tags: None,
    };
    let uuid = db.create_project(request).await.unwrap();

    // Complete it (no children, so Error handling is fine)
    db.complete_project(&uuid, ProjectChildHandling::Error)
        .await
        .unwrap();

    // Verify completion
    let task = db.get_task_by_uuid(&uuid).await.unwrap().unwrap();
    assert_eq!(task.status, things3_core::models::TaskStatus::Completed);
    assert!(task.stop_date.is_some());
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_complete_project_with_children_cascade() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create a project
    let project_request = CreateProjectRequest {
        title: "Project with Tasks".to_string(),
        notes: None,
        area_uuid: None,
        start_date: None,
        deadline: None,
        tags: None,
    };
    let project_uuid = db.create_project(project_request).await.unwrap();

    // Add a child task
    let task_request = TaskRequestBuilder::new()
        .title("Child Task")
        .project(project_uuid)
        .build();
    let task_uuid = db.create_task(task_request).await.unwrap();

    // Complete project with cascade
    db.complete_project(&project_uuid, ProjectChildHandling::Cascade)
        .await
        .unwrap();

    // Verify both are completed
    let project = db.get_task_by_uuid(&project_uuid).await.unwrap().unwrap();
    assert_eq!(project.status, things3_core::models::TaskStatus::Completed);

    let task = db.get_task_by_uuid(&task_uuid).await.unwrap().unwrap();
    assert_eq!(task.status, things3_core::models::TaskStatus::Completed);
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_delete_project_with_children_error() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create a project with a child task
    let project_request = CreateProjectRequest {
        title: "Project with Tasks".to_string(),
        notes: None,
        area_uuid: None,
        start_date: None,
        deadline: None,
        tags: None,
    };
    let project_uuid = db.create_project(project_request).await.unwrap();

    let task_request = TaskRequestBuilder::new()
        .title("Child Task")
        .project(project_uuid)
        .build();
    db.create_task(task_request).await.unwrap();

    // Try to delete with Error handling (should fail)
    let result = db
        .delete_project(&project_uuid, ProjectChildHandling::Error)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_delete_project_with_children_orphan() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create a project with a child task
    let project_request = CreateProjectRequest {
        title: "Project with Tasks".to_string(),
        notes: None,
        area_uuid: None,
        start_date: None,
        deadline: None,
        tags: None,
    };
    let project_uuid = db.create_project(project_request).await.unwrap();

    let task_request = TaskRequestBuilder::new()
        .title("Child Task")
        .project(project_uuid)
        .build();
    let task_uuid = db.create_task(task_request).await.unwrap();

    // Delete project with orphan handling
    db.delete_project(&project_uuid, ProjectChildHandling::Orphan)
        .await
        .unwrap();

    // Project should be deleted
    let project = db.get_task_by_uuid(&project_uuid).await.unwrap();
    assert!(project.is_none());

    // Task should still exist but with no project
    let task = db.get_task_by_uuid(&task_uuid).await.unwrap().unwrap();
    assert_eq!(task.project_uuid, None);
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_create_area_success() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let request = CreateAreaRequest {
        title: "Personal".to_string(),
    };

    let uuid = db.create_area(request).await.unwrap();
    assert!(!uuid.is_nil());

    // Verify it was created
    let areas = db.get_all_areas().await.unwrap();
    assert!(areas
        .iter()
        .any(|a| a.uuid == uuid && a.title == "Personal"));
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_update_area_success() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create an area
    let create_request = CreateAreaRequest {
        title: "Original Area".to_string(),
    };
    let uuid = db.create_area(create_request).await.unwrap();

    // Update it
    let update_request = UpdateAreaRequest {
        uuid,
        title: "Updated Area".to_string(),
    };
    db.update_area(update_request).await.unwrap();

    // Verify update
    let areas = db.get_all_areas().await.unwrap();
    let area = areas.iter().find(|a| a.uuid == uuid).unwrap();
    assert_eq!(area.title, "Updated Area");
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_delete_area_with_projects() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create an area
    let area_request = CreateAreaRequest {
        title: "Area to Delete".to_string(),
    };
    let area_uuid = db.create_area(area_request).await.unwrap();

    // Create a project in that area
    let project_request = CreateProjectRequest {
        title: "Project in Area".to_string(),
        notes: None,
        area_uuid: Some(area_uuid),
        start_date: None,
        deadline: None,
        tags: None,
    };
    let project_uuid = db.create_project(project_request).await.unwrap();

    // Delete the area
    db.delete_area(&area_uuid).await.unwrap();

    // Area should be deleted
    let areas = db.get_all_areas().await.unwrap();
    assert!(!areas.iter().any(|a| a.uuid == area_uuid));

    // Project should still exist but with no area
    let project = db.get_task_by_uuid(&project_uuid).await.unwrap().unwrap();
    assert_eq!(project.area_uuid, None);
}

#[tokio::test]
#[cfg(feature = "test-utils")]
async fn test_delete_area_empty() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Create an area with no projects
    let request = CreateAreaRequest {
        title: "Empty Area".to_string(),
    };
    let uuid = db.create_area(request).await.unwrap();

    // Delete it
    db.delete_area(&uuid).await.unwrap();

    // Verify deletion
    let areas = db.get_all_areas().await.unwrap();
    assert!(!areas.iter().any(|a| a.uuid == uuid));
}
