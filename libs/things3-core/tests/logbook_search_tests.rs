#![cfg(feature = "test-utils")]

use chrono::Utc;
use things3_core::{
    database::ThingsDatabase,
    models::CreateTaskRequest,
    test_utils::{create_test_database_and_connect, TaskRequestBuilder},
};
use uuid::Uuid;

/// Helper to complete a task
async fn complete_task(db: &ThingsDatabase, uuid: Uuid) {
    db.complete_task(&uuid)
        .await
        .expect("Failed to complete task");
}

/// Helper to create and complete a task
async fn create_and_complete_task(db: &ThingsDatabase, request: CreateTaskRequest) -> Uuid {
    let uuid = db
        .create_task(request)
        .await
        .expect("Failed to create task");
    complete_task(db, uuid).await;
    uuid
}

#[tokio::test]
async fn test_search_logbook_all_completed() {
    let (db, _temp_file) = create_test_database_and_connect()
        .await
        .expect("Failed to create database");

    // Create 5 completed tasks
    for i in 1..=5 {
        create_and_complete_task(
            &db,
            TaskRequestBuilder::new()
                .title(format!("Completed task {i}"))
                .build(),
        )
        .await;
    }

    // Create 3 incomplete tasks (should not appear in results)
    for i in 1..=3 {
        db.create_task(
            TaskRequestBuilder::new()
                .title(format!("Incomplete task {i}"))
                .build(),
        )
        .await
        .expect("Failed to create incomplete task");
    }

    // Search with no filters
    let results = db
        .search_logbook(None, None, None, None, None, None, None)
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 5, "Should return only completed tasks");

    // Verify all tasks are completed
    for task in &results {
        assert_eq!(task.status, things3_core::models::TaskStatus::Completed);
        assert!(
            task.stop_date.is_some(),
            "Completed task should have a completion date"
        );
    }

    // Verify order (most recent first)
    for i in 0..results.len() - 1 {
        let current = results[i].stop_date.unwrap();
        let next = results[i + 1].stop_date.unwrap();
        assert!(
            current >= next,
            "Results should be ordered by completion date descending"
        );
    }
}

#[tokio::test]
async fn test_search_logbook_text_search() {
    let (db, _temp_file) = create_test_database_and_connect()
        .await
        .expect("Failed to create database");

    // Create tasks with various titles
    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Buy groceries for dinner")
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Write project report".to_string())
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Finish coding project".to_string())
            .build(),
    )
    .await;

    // Search for "project"
    let results = db
        .search_logbook(
            Some("project".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to search logbook");

    assert_eq!(
        results.len(),
        2,
        "Should find 2 tasks with 'project' in title"
    );
    assert!(results
        .iter()
        .all(|t| t.title.to_lowercase().contains("project")));

    // Search for "groceries"
    let results = db
        .search_logbook(
            Some("groceries".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to search logbook");

    assert_eq!(
        results.len(),
        1,
        "Should find 1 task with 'groceries' in title"
    );
}

#[tokio::test]
async fn test_search_logbook_text_search_in_notes() {
    let (db, _temp_file) = create_test_database_and_connect()
        .await
        .expect("Failed to create database");

    // Create task with text in notes
    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Task title")
            .notes("Important meeting notes about the project")
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Another task")
            .notes("Random notes")
            .build(),
    )
    .await;

    // Search for "meeting" (only in notes)
    let results = db
        .search_logbook(
            Some("meeting".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 1, "Should find task with 'meeting' in notes");
}

#[tokio::test]
async fn test_search_logbook_date_range() {
    let (db, _temp_file) = create_test_database_and_connect()
        .await
        .expect("Failed to create database");

    // Create tasks and complete them (they'll have today's date)
    let today = Utc::now().date_naive();
    let yesterday = today - chrono::Duration::days(1);
    let tomorrow = today + chrono::Duration::days(1);

    // We can't control stopDate directly in the current API, so we'll test the query logic
    // by creating tasks today and filtering by date
    for i in 1..=3 {
        create_and_complete_task(
            &db,
            TaskRequestBuilder::new().title(format!("Task {i}")).build(),
        )
        .await;
    }

    // Search for tasks completed today or after
    let results = db
        .search_logbook(None, Some(today), None, None, None, None, None)
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 3, "Should find all 3 tasks completed today");

    // Search for tasks completed yesterday (should be none)
    let results = db
        .search_logbook(None, None, Some(yesterday), None, None, None, None)
        .await
        .expect("Failed to search logbook");

    assert_eq!(
        results.len(),
        0,
        "Should find no tasks completed before today"
    );

    // Search for tasks completed from yesterday to tomorrow
    let results = db
        .search_logbook(
            None,
            Some(yesterday),
            Some(tomorrow),
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 3, "Should find all 3 tasks in date range");
}

#[tokio::test]
async fn test_search_logbook_from_date_only() {
    let (db, _temp_file) = create_test_database_and_connect()
        .await
        .expect("Failed to create database");

    let today = Utc::now().date_naive();
    let future_date = today + chrono::Duration::days(7);

    // Create and complete tasks today
    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Task 1".to_string())
            .build(),
    )
    .await;

    // Search from today
    let results = db
        .search_logbook(None, Some(today), None, None, None, None, None)
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 1, "Should find task completed from today");

    // Search from future date (should be empty)
    let results = db
        .search_logbook(None, Some(future_date), None, None, None, None, None)
        .await
        .expect("Failed to search logbook");

    assert_eq!(
        results.len(),
        0,
        "Should find no tasks completed in the future"
    );
}

#[tokio::test]
async fn test_search_logbook_to_date_only() {
    let (db, _temp_file) = create_test_database_and_connect()
        .await
        .expect("Failed to create database");

    let today = Utc::now().date_naive();
    let past_date = today - chrono::Duration::days(7);

    // Create and complete tasks today
    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Task 1".to_string())
            .build(),
    )
    .await;

    // Search up to today
    let results = db
        .search_logbook(None, None, Some(today), None, None, None, None)
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 1, "Should find task completed up to today");

    // Search up to past date (should be empty)
    let results = db
        .search_logbook(None, None, Some(past_date), None, None, None, None)
        .await
        .expect("Failed to search logbook");

    assert_eq!(
        results.len(),
        0,
        "Should find no tasks completed before past date"
    );
}

#[tokio::test]
async fn test_search_logbook_project_filter() {
    let (db, _temp_file) = create_test_database_and_connect()
        .await
        .expect("Failed to create database");

    // Create projects
    let project1_uuid = db
        .create_project(things3_core::models::CreateProjectRequest {
            title: "Project 1".to_string(),
            notes: None,
            deadline: None,
            area_uuid: None,
            start_date: None,
            tags: None,
        })
        .await
        .expect("Failed to create project 1");

    let project2_uuid = db
        .create_project(things3_core::models::CreateProjectRequest {
            title: "Project 2".to_string(),
            notes: None,
            deadline: None,
            area_uuid: None,
            start_date: None,
            tags: None,
        })
        .await
        .expect("Failed to create project 2");

    // Create tasks in different projects
    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Project 1 task 1".to_string())
            .project(project1_uuid)
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Project 1 task 2".to_string())
            .project(project1_uuid)
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Project 2 task 1".to_string())
            .project(project2_uuid)
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("No project task".to_string())
            .build(),
    )
    .await;

    // Search by project 1
    let results = db
        .search_logbook(None, None, None, Some(project1_uuid), None, None, None)
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 2, "Should find 2 tasks in project 1");
    assert!(results
        .iter()
        .all(|t| t.project_uuid == Some(project1_uuid)));

    // Search by project 2
    let results = db
        .search_logbook(None, None, None, Some(project2_uuid), None, None, None)
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 1, "Should find 1 task in project 2");
}

#[tokio::test]
async fn test_search_logbook_area_filter() {
    let (db, _temp_file) = create_test_database_and_connect()
        .await
        .expect("Failed to create database");

    // Create areas
    let area1_uuid = db
        .create_area(things3_core::models::CreateAreaRequest {
            title: "Area 1".to_string(),
        })
        .await
        .expect("Failed to create area 1");

    let area2_uuid = db
        .create_area(things3_core::models::CreateAreaRequest {
            title: "Area 2".to_string(),
        })
        .await
        .expect("Failed to create area 2");

    // Create tasks in different areas
    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Area 1 task 1".to_string())
            .area(area1_uuid)
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Area 1 task 2".to_string())
            .area(area1_uuid)
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Area 2 task 1".to_string())
            .area(area2_uuid)
            .build(),
    )
    .await;

    // Search by area 1
    let results = db
        .search_logbook(None, None, None, None, Some(area1_uuid), None, None)
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 2, "Should find 2 tasks in area 1");
    assert!(results.iter().all(|t| t.area_uuid == Some(area1_uuid)));

    // Search by area 2
    let results = db
        .search_logbook(None, None, None, None, Some(area2_uuid), None, None)
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 1, "Should find 1 task in area 2");
}

#[tokio::test]
async fn test_search_logbook_tag_filter() {
    let (db, _temp_file) = create_test_database_and_connect()
        .await
        .expect("Failed to create database");

    // Create tasks with various tags
    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Task with work tag")
            .tags(vec!["work".to_string()])
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Task with personal tag")
            .tags(vec!["personal".to_string()])
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Task with both tags")
            .tags(vec!["work".to_string(), "personal".to_string()])
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Task with no tags".to_string())
            .build(),
    )
    .await;

    // Search by "work" tag
    let results = db
        .search_logbook(
            None,
            None,
            None,
            None,
            None,
            Some(vec!["work".to_string()]),
            None,
        )
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 2, "Should find 2 tasks with 'work' tag");
    assert!(results.iter().all(|t| t.tags.contains(&"work".to_string())));

    // Search by "personal" tag
    let results = db
        .search_logbook(
            None,
            None,
            None,
            None,
            None,
            Some(vec!["personal".to_string()]),
            None,
        )
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 2, "Should find 2 tasks with 'personal' tag");
}

#[tokio::test]
async fn test_search_logbook_multiple_tags() {
    let (db, _temp_file) = create_test_database_and_connect()
        .await
        .expect("Failed to create database");

    // Create tasks with various tag combinations
    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Task with work and urgent")
            .tags(vec!["work".to_string(), "urgent".to_string()])
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Task with work only")
            .tags(vec!["work".to_string()])
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Task with urgent only")
            .tags(vec!["urgent".to_string()])
            .build(),
    )
    .await;

    // Search by both "work" AND "urgent" tags
    let results = db
        .search_logbook(
            None,
            None,
            None,
            None,
            None,
            Some(vec!["work".to_string(), "urgent".to_string()]),
            None,
        )
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 1, "Should find only task with both tags");
    assert!(results[0].tags.contains(&"work".to_string()));
    assert!(results[0].tags.contains(&"urgent".to_string()));
}

#[tokio::test]
async fn test_search_logbook_combined_filters() {
    let (db, _temp_file) = create_test_database_and_connect()
        .await
        .expect("Failed to create database");

    let project_uuid = db
        .create_project(things3_core::models::CreateProjectRequest {
            title: "Test Project".to_string(),
            notes: None,
            deadline: None,
            area_uuid: None,
            start_date: None,
            tags: None,
        })
        .await
        .expect("Failed to create project");

    let today = Utc::now().date_naive();

    // Create tasks with various properties
    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Matching project task".to_string())
            .project(project_uuid)
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Non-matching task".to_string())
            .build(),
    )
    .await;

    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Another matching project task".to_string())
            .project(project_uuid)
            .build(),
    )
    .await;

    // Search with text + project filter
    let results = db
        .search_logbook(
            Some("matching".to_string()),
            Some(today),
            None,
            Some(project_uuid),
            None,
            None,
            None,
        )
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 2, "Should find 2 tasks matching all filters");
    assert!(results.iter().all(
        |t| t.project_uuid == Some(project_uuid) && t.title.to_lowercase().contains("matching")
    ));
}

#[tokio::test]
async fn test_search_logbook_limit() {
    let (db, _temp_file) = create_test_database_and_connect()
        .await
        .expect("Failed to create database");

    // Create 20 completed tasks
    for i in 1..=20 {
        create_and_complete_task(
            &db,
            TaskRequestBuilder::new().title(format!("Task {i}")).build(),
        )
        .await;
    }

    // Search with limit of 10
    let results = db
        .search_logbook(None, None, None, None, None, None, Some(10))
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 10, "Should return exactly 10 results");

    // Search with limit of 5
    let results = db
        .search_logbook(None, None, None, None, None, None, Some(5))
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 5, "Should return exactly 5 results");

    // Search with default limit (50)
    let results = db
        .search_logbook(None, None, None, None, None, None, None)
        .await
        .expect("Failed to search logbook");

    assert_eq!(
        results.len(),
        20,
        "Should return all 20 results with default limit"
    );
}

#[tokio::test]
async fn test_search_logbook_empty_results() {
    let (db, _temp_file) = create_test_database_and_connect()
        .await
        .expect("Failed to create database");

    // Create some completed tasks
    create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Task 1".to_string())
            .build(),
    )
    .await;

    // Search with non-matching criteria
    let results = db
        .search_logbook(
            Some("nonexistent".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("Failed to search logbook");

    assert_eq!(results.len(), 0, "Should return empty vec, not error");

    // Search with non-existent UUID
    let fake_uuid = Uuid::new_v4();
    let results = db
        .search_logbook(None, None, None, Some(fake_uuid), None, None, None)
        .await
        .expect("Failed to search logbook");

    assert_eq!(
        results.len(),
        0,
        "Should return empty vec for non-existent project"
    );
}

#[tokio::test]
async fn test_search_logbook_excludes_trashed() {
    let (db, _temp_file) = create_test_database_and_connect()
        .await
        .expect("Failed to create database");

    // Create and complete tasks
    let uuid1 = create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Completed task 1".to_string())
            .build(),
    )
    .await;

    let uuid2 = create_and_complete_task(
        &db,
        TaskRequestBuilder::new()
            .title("Completed task 2".to_string())
            .build(),
    )
    .await;

    // Delete one task
    db.delete_task(&uuid2, things3_core::models::DeleteChildHandling::Cascade)
        .await
        .expect("Failed to delete task");

    // Search logbook
    let results = db
        .search_logbook(None, None, None, None, None, None, None)
        .await
        .expect("Failed to search logbook");

    assert_eq!(
        results.len(),
        1,
        "Should find only non-trashed completed task"
    );
    assert_eq!(results[0].uuid, uuid1);
    assert!(
        !results.iter().any(|t| t.uuid == uuid2),
        "Trashed task should not appear"
    );
}
