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

    // Create the Things 3 schema - matches real database structure
    sqlx::query(
        r"
        -- TMTask table (main tasks table) - matches real Things 3 schema
        CREATE TABLE IF NOT EXISTS TMTask (
            uuid TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            type INTEGER NOT NULL DEFAULT 0,
            status INTEGER NOT NULL DEFAULT 0,
            notes TEXT,
            startDate INTEGER,
            deadline INTEGER,
            creationDate REAL NOT NULL,
            userModificationDate REAL NOT NULL,
            project TEXT,
            area TEXT,
            heading TEXT,
            trashed INTEGER NOT NULL DEFAULT 0,
            tags TEXT DEFAULT '[]',
            cachedTags BLOB
        )
        ",
    )
    .execute(&pool)
    .await
    .map_err(|e| crate::ThingsError::Database(format!("Failed to create TMTask table: {e}")))?;

    // Note: Projects are stored in TMTask table with type=1, no separate TMProject table

    sqlx::query(
        r"
        -- TMArea table (areas table) - matches real Things 3 schema
        CREATE TABLE IF NOT EXISTS TMArea (
            uuid TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            visible INTEGER NOT NULL DEFAULT 1,
            'index' INTEGER NOT NULL DEFAULT 0
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
    // Use a safe conversion for timestamp to avoid precision loss
    let timestamp_i64 = Utc::now().timestamp();
    let now_timestamp = if timestamp_i64 <= i64::from(i32::MAX) {
        f64::from(i32::try_from(timestamp_i64).unwrap_or(0))
    } else {
        // For very large timestamps, use a reasonable test value
        1_700_000_000.0 // Represents a date around 2023
    };

    // Generate valid UUIDs for test data
    let area_uuid = Uuid::new_v4().to_string();
    let project_uuid = Uuid::new_v4().to_string();
    let task_uuid = Uuid::new_v4().to_string();

    // Insert test areas
    sqlx::query("INSERT INTO TMArea (uuid, title, visible, 'index') VALUES (?, ?, ?, ?)")
        .bind(&area_uuid)
        .bind("Work")
        .bind(1) // Visible
        .bind(0) // Index
        .execute(pool)
        .await
        .map_err(|e| crate::ThingsError::Database(format!("Failed to insert test area: {e}")))?;

    // Insert test projects (as TMTask with type=1)
    sqlx::query(
        "INSERT INTO TMTask (uuid, title, type, status, area, creationDate, userModificationDate, trashed) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&project_uuid)
    .bind("Website Redesign")
    .bind(1) // Project type
    .bind(0) // Incomplete
    .bind(&area_uuid)
    .bind(now_timestamp)
    .bind(now_timestamp)
    .bind(0) // Not trashed
    .execute(pool).await
    .map_err(|e| crate::ThingsError::Database(format!("Failed to insert test project: {e}")))?;

    // Insert test tasks - one in inbox (no project), one in project
    let inbox_task_uuid = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO TMTask (uuid, title, type, status, project, creationDate, userModificationDate, trashed) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&inbox_task_uuid)
    .bind("Inbox Task")
    .bind(0) // Todo type
    .bind(0) // Incomplete
    .bind::<Option<String>>(None) // No project (inbox) - use NULL instead of empty string
    .bind(now_timestamp)
    .bind(now_timestamp)
    .bind(0) // Not trashed
    .execute(pool).await
    .map_err(|e| crate::ThingsError::Database(format!("Failed to insert inbox test task: {e}")))?;

    sqlx::query(
        "INSERT INTO TMTask (uuid, title, type, status, project, creationDate, userModificationDate, trashed) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&task_uuid)
    .bind("Research competitors")
    .bind(0) // Todo type
    .bind(0) // Incomplete
    .bind(&project_uuid)
    .bind(now_timestamp)
    .bind(now_timestamp)
    .bind(0) // Not trashed
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_create_test_database() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        let result = create_test_database(db_path).await;
        assert!(result.is_ok(), "Should successfully create test database");

        // Verify the database file exists
        assert!(db_path.exists(), "Database file should exist");
    }

    #[tokio::test]
    async fn test_create_test_database_with_data() {
        use sqlx::SqlitePool;

        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        create_test_database(db_path).await.unwrap();

        // Connect to the database and verify data exists
        let database_url = format!("sqlite:{}", db_path.display());
        let pool = SqlitePool::connect(&database_url).await.unwrap();

        // Check that tables exist and have data
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM TMTask")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert!(task_count > 0, "Should have test tasks in database");

        let area_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM TMArea")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert!(area_count > 0, "Should have test areas in database");
    }

    #[test]
    fn test_create_mock_areas() {
        let areas = create_mock_areas();
        assert_eq!(areas.len(), 2, "Should create 2 mock areas");

        let work_area = &areas[0];
        assert_eq!(work_area.title, "Work");
        assert!(work_area.notes.is_some());
        assert_eq!(work_area.tags, vec!["work"]);

        let personal_area = &areas[1];
        assert_eq!(personal_area.title, "Personal");
        assert!(personal_area.notes.is_some());
        assert_eq!(personal_area.tags, vec!["personal"]);
    }

    #[test]
    fn test_create_mock_projects() {
        let projects = create_mock_projects();
        assert_eq!(projects.len(), 2, "Should create 2 mock projects");

        let website_project = &projects[0];
        assert_eq!(website_project.title, "Website Redesign");
        assert_eq!(website_project.status, TaskStatus::Incomplete);
        assert!(website_project.notes.is_some());
        assert!(website_project.area_uuid.is_some());
        assert_eq!(website_project.tags, vec!["work", "web"]);

        let rust_project = &projects[1];
        assert_eq!(rust_project.title, "Learn Rust");
        assert_eq!(rust_project.status, TaskStatus::Incomplete);
        assert!(rust_project.notes.is_some());
        assert!(rust_project.area_uuid.is_some());
        assert_eq!(rust_project.tags, vec!["personal", "learning"]);
    }

    #[test]
    fn test_create_mock_tasks() {
        let tasks = create_mock_tasks();
        assert_eq!(tasks.len(), 2, "Should create 2 mock tasks");

        let research_task = &tasks[0];
        assert_eq!(research_task.title, "Research competitors");
        assert_eq!(research_task.task_type, TaskType::Todo);
        assert_eq!(research_task.status, TaskStatus::Incomplete);
        assert!(research_task.notes.is_some());
        assert!(research_task.project_uuid.is_some());
        assert!(research_task.area_uuid.is_some());
        assert_eq!(research_task.tags, vec!["research"]);

        let rust_task = &tasks[1];
        assert_eq!(rust_task.title, "Read Rust book");
        assert_eq!(rust_task.task_type, TaskType::Todo);
        assert_eq!(rust_task.status, TaskStatus::Incomplete);
        assert!(rust_task.notes.is_some());
        assert!(rust_task.project_uuid.is_some());
        assert!(rust_task.area_uuid.is_some());
        assert_eq!(rust_task.tags, vec!["reading"]);
    }

    #[test]
    fn test_mock_data_consistency() {
        let areas = create_mock_areas();
        let projects = create_mock_projects();
        let tasks = create_mock_tasks();

        // Verify that project area UUIDs match area UUIDs
        let work_area_uuid = areas[0].uuid;
        let personal_area_uuid = areas[1].uuid;

        let website_project = &projects[0];
        let rust_project = &projects[1];

        assert_eq!(website_project.area_uuid, Some(work_area_uuid));
        assert_eq!(rust_project.area_uuid, Some(personal_area_uuid));

        // Verify that task project and area UUIDs match
        let website_project_uuid = projects[0].uuid;
        let rust_project_uuid = projects[1].uuid;

        let research_task = &tasks[0];
        let rust_task = &tasks[1];

        assert_eq!(research_task.project_uuid, Some(website_project_uuid));
        assert_eq!(research_task.area_uuid, Some(work_area_uuid));

        assert_eq!(rust_task.project_uuid, Some(rust_project_uuid));
        assert_eq!(rust_task.area_uuid, Some(personal_area_uuid));
    }

    #[test]
    fn test_mock_data_uuids_are_valid() {
        let areas = create_mock_areas();
        let projects = create_mock_projects();
        let tasks = create_mock_tasks();

        // Test that all UUIDs are valid
        for area in &areas {
            assert!(
                !area.uuid.to_string().is_empty(),
                "Area UUID should be valid"
            );
        }

        for project in &projects {
            assert!(
                !project.uuid.to_string().is_empty(),
                "Project UUID should be valid"
            );
            if let Some(area_uuid) = project.area_uuid {
                assert!(
                    !area_uuid.to_string().is_empty(),
                    "Project area UUID should be valid"
                );
            }
        }

        for task in &tasks {
            assert!(
                !task.uuid.to_string().is_empty(),
                "Task UUID should be valid"
            );
            if let Some(project_uuid) = task.project_uuid {
                assert!(
                    !project_uuid.to_string().is_empty(),
                    "Task project UUID should be valid"
                );
            }
            if let Some(area_uuid) = task.area_uuid {
                assert!(
                    !area_uuid.to_string().is_empty(),
                    "Task area UUID should be valid"
                );
            }
        }
    }

    #[test]
    fn test_mock_data_timestamps() {
        let areas = create_mock_areas();
        let projects = create_mock_projects();
        let tasks = create_mock_tasks();

        let now = Utc::now();

        // Test that all timestamps are recent (within last minute)
        for area in &areas {
            let diff = now.signed_duration_since(area.created).num_seconds().abs();
            assert!(diff < 60, "Area created timestamp should be recent");

            let diff = now.signed_duration_since(area.modified).num_seconds().abs();
            assert!(diff < 60, "Area modified timestamp should be recent");
        }

        for project in &projects {
            let diff = now
                .signed_duration_since(project.created)
                .num_seconds()
                .abs();
            assert!(diff < 60, "Project created timestamp should be recent");

            let diff = now
                .signed_duration_since(project.modified)
                .num_seconds()
                .abs();
            assert!(diff < 60, "Project modified timestamp should be recent");
        }

        for task in &tasks {
            let diff = now.signed_duration_since(task.created).num_seconds().abs();
            assert!(diff < 60, "Task created timestamp should be recent");

            let diff = now.signed_duration_since(task.modified).num_seconds().abs();
            assert!(diff < 60, "Task modified timestamp should be recent");
        }
    }

    #[tokio::test]
    async fn test_insert_test_data_function() {
        use sqlx::SqlitePool;

        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        // Create database with schema but no data
        let database_url = format!("sqlite:{}", db_path.display());
        let pool = SqlitePool::connect(&database_url).await.unwrap();

        // Create tables
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS TMTask (
                uuid TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                type INTEGER NOT NULL DEFAULT 0,
                status INTEGER NOT NULL DEFAULT 0,
                notes TEXT,
                startDate INTEGER,
                deadline INTEGER,
                creationDate REAL NOT NULL,
                userModificationDate REAL NOT NULL,
                project TEXT,
                area TEXT,
                parent TEXT,
                trashed INTEGER NOT NULL DEFAULT 0,
                tags TEXT DEFAULT '[]',
                cachedTags BLOB
            )
            ",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS TMArea (
                uuid TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                visible INTEGER NOT NULL DEFAULT 1,
                'index' INTEGER NOT NULL DEFAULT 0
            )
            ",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Test the insert_test_data function directly
        let result = insert_test_data(&pool).await;
        assert!(result.is_ok(), "Should successfully insert test data");

        // Verify data was inserted
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM TMTask")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(
            task_count >= 3,
            "Should have at least 3 test tasks (1 project + 2 tasks)"
        );

        let area_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM TMArea")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(area_count >= 1, "Should have at least 1 test area");

        pool.close().await;
    }

    #[test]
    fn test_create_mock_areas_structure() {
        let areas = create_mock_areas();

        // Test structure and content
        assert_eq!(areas.len(), 2);

        // Test first area (Work)
        let work_area = &areas[0];
        assert_eq!(work_area.title, "Work");
        assert!(work_area.notes.is_some());
        assert_eq!(work_area.notes.as_ref().unwrap(), "Work-related tasks");
        assert_eq!(work_area.tags, vec!["work"]);
        assert!(work_area.projects.is_empty());

        // Test second area (Personal)
        let personal_area = &areas[1];
        assert_eq!(personal_area.title, "Personal");
        assert!(personal_area.notes.is_some());
        assert_eq!(personal_area.notes.as_ref().unwrap(), "Personal tasks");
        assert_eq!(personal_area.tags, vec!["personal"]);
        assert!(personal_area.projects.is_empty());

        // Test that UUIDs are different
        assert_ne!(work_area.uuid, personal_area.uuid);
    }

    #[test]
    fn test_create_mock_projects_structure() {
        let projects = create_mock_projects();

        // Test structure and content
        assert_eq!(projects.len(), 2);

        // Test first project (Website Redesign)
        let website_project = &projects[0];
        assert_eq!(website_project.title, "Website Redesign");
        assert_eq!(website_project.status, TaskStatus::Incomplete);
        assert!(website_project.notes.is_some());
        assert_eq!(
            website_project.notes.as_ref().unwrap(),
            "Complete redesign of company website"
        );
        assert_eq!(website_project.tags, vec!["work", "web"]);
        assert!(website_project.tasks.is_empty());
        assert!(website_project.area_uuid.is_some());
        assert!(website_project.start_date.is_none());
        assert!(website_project.deadline.is_none());

        // Test second project (Learn Rust)
        let rust_project = &projects[1];
        assert_eq!(rust_project.title, "Learn Rust");
        assert_eq!(rust_project.status, TaskStatus::Incomplete);
        assert!(rust_project.notes.is_some());
        assert_eq!(
            rust_project.notes.as_ref().unwrap(),
            "Learn the Rust programming language"
        );
        assert_eq!(rust_project.tags, vec!["personal", "learning"]);
        assert!(rust_project.tasks.is_empty());
        assert!(rust_project.area_uuid.is_some());

        // Test that UUIDs are different
        assert_ne!(website_project.uuid, rust_project.uuid);
    }

    #[test]
    fn test_create_mock_tasks_structure() {
        let tasks = create_mock_tasks();

        // Test structure and content
        assert_eq!(tasks.len(), 2);

        // Test first task (Research competitors)
        let research_task = &tasks[0];
        assert_eq!(research_task.title, "Research competitors");
        assert_eq!(research_task.task_type, TaskType::Todo);
        assert_eq!(research_task.status, TaskStatus::Incomplete);
        assert!(research_task.notes.is_some());
        assert_eq!(
            research_task.notes.as_ref().unwrap(),
            "Look at competitor websites for inspiration"
        );
        assert_eq!(research_task.tags, vec!["research"]);
        assert!(research_task.children.is_empty());
        assert!(research_task.project_uuid.is_some());
        assert!(research_task.area_uuid.is_some());
        assert!(research_task.parent_uuid.is_none());
        assert!(research_task.start_date.is_none());
        assert!(research_task.deadline.is_none());

        // Test second task (Read Rust book)
        let rust_task = &tasks[1];
        assert_eq!(rust_task.title, "Read Rust book");
        assert_eq!(rust_task.task_type, TaskType::Todo);
        assert_eq!(rust_task.status, TaskStatus::Incomplete);
        assert!(rust_task.notes.is_some());
        assert_eq!(
            rust_task.notes.as_ref().unwrap(),
            "Read The Rust Programming Language book"
        );
        assert_eq!(rust_task.tags, vec!["reading"]);
        assert!(rust_task.children.is_empty());
        assert!(rust_task.project_uuid.is_some());
        assert!(rust_task.area_uuid.is_some());

        // Test that UUIDs are different
        assert_ne!(research_task.uuid, rust_task.uuid);
    }

    #[tokio::test]
    async fn test_create_test_database_error_handling() {
        // Test with invalid path (should still work with SQLite in-memory)
        let result = create_test_database("").await;
        // This might succeed or fail depending on SQLite behavior with empty paths
        // The important thing is that it doesn't panic
        match result {
            Ok(()) => {
                // Success is fine
            }
            Err(e) => {
                // Error is also fine, as long as it's a proper error
                assert!(!e.to_string().is_empty(), "Error should have a message");
            }
        }
    }

    #[test]
    fn test_mock_data_uuid_format() {
        let areas = create_mock_areas();
        let projects = create_mock_projects();
        let tasks = create_mock_tasks();

        // Test that all UUIDs are properly formatted
        for area in &areas {
            let uuid_str = area.uuid.to_string();
            assert_eq!(uuid_str.len(), 36, "UUID should be 36 characters long");
            assert_eq!(
                uuid_str.chars().filter(|&c| c == '-').count(),
                4,
                "UUID should have 4 hyphens"
            );
        }

        for project in &projects {
            let uuid_str = project.uuid.to_string();
            assert_eq!(uuid_str.len(), 36, "UUID should be 36 characters long");
            assert_eq!(
                uuid_str.chars().filter(|&c| c == '-').count(),
                4,
                "UUID should have 4 hyphens"
            );
        }

        for task in &tasks {
            let uuid_str = task.uuid.to_string();
            assert_eq!(uuid_str.len(), 36, "UUID should be 36 characters long");
            assert_eq!(
                uuid_str.chars().filter(|&c| c == '-').count(),
                4,
                "UUID should have 4 hyphens"
            );
        }
    }
}
