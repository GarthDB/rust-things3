//! Test utilities and mock data for Things 3 integration

use crate::models::{Area, Project, Task, TaskStatus, TaskType};
use chrono::Utc;
use rusqlite::Connection;
use std::path::Path;
use uuid::Uuid;

/// Create a test database with mock data
///
/// # Errors
/// Returns `ThingsError::Database` if the database cannot be created
pub fn create_test_database<P: AsRef<Path>>(db_path: P) -> crate::Result<Connection> {
    let conn = Connection::open(db_path)?;

    // Create the Things 3 schema
    conn.execute_batch(
        r#"
        -- TMTask table (main tasks table) - matches real Things 3 schema
        CREATE TABLE IF NOT EXISTS TMTask (
            uuid TEXT PRIMARY KEY,
            leavesTombstone INTEGER,
            creationDate REAL,
            userModificationDate REAL,
            type INTEGER,
            status INTEGER,
            stopDate REAL,
            trashed INTEGER,
            title TEXT,
            notes TEXT,
            notesSync INTEGER,
            cachedTags BLOB,
            start INTEGER,
            startDate INTEGER,
            startBucket INTEGER,
            reminderTime INTEGER,
            lastReminderInteractionDate REAL,
            deadline INTEGER,
            deadlineSuppressionDate INTEGER,
            t2_deadlineOffset INTEGER,
            "index" INTEGER,
            todayIndex INTEGER,
            todayIndexReferenceDate INTEGER,
            area TEXT,
            project TEXT,
            heading TEXT,
            contact TEXT,
            untrashedLeafActionsCount INTEGER,
            openUntrashedLeafActionsCount INTEGER,
            checklistItemsCount INTEGER,
            openChecklistItemsCount INTEGER,
            rt1_repeatingTemplate TEXT,
            rt1_recurrenceRule BLOB,
            rt1_instanceCreationStartDate INTEGER,
            rt1_instanceCreationPaused INTEGER,
            rt1_instanceCreationCount INTEGER,
            rt1_afterCompletionReferenceDate INTEGER,
            rt1_nextInstanceStartDate INTEGER,
            experimental BLOB,
            repeater BLOB,
            repeaterMigrationDate REAL
        );

        -- TMArea table (areas)
        CREATE TABLE IF NOT EXISTS TMArea (
            uuid TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            visible INTEGER,
            "index" INTEGER NOT NULL DEFAULT 0,
            cachedTags BLOB,
            experimental BLOB
        );

        -- TMTag table (tags)
        CREATE TABLE IF NOT EXISTS TMTag (
            uuid TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            created TEXT NOT NULL,
            modified TEXT NOT NULL,
            "index" INTEGER NOT NULL DEFAULT 0
        );

        -- TMTaskTag table (many-to-many relationship)
        CREATE TABLE IF NOT EXISTS TMTaskTag (
            task_uuid TEXT NOT NULL,
            tag_uuid TEXT NOT NULL,
            PRIMARY KEY (task_uuid, tag_uuid)
        );
        "#,
    )?;

    // Insert mock data
    insert_mock_data(&conn)?;

    Ok(conn)
}

/// Insert mock data into the test database
#[allow(clippy::too_many_lines)]
fn insert_mock_data(conn: &Connection) -> crate::Result<()> {
    let now = Utc::now();
    let today = now.date_naive();
    let tomorrow = today + chrono::Duration::days(1);
    let yesterday = today - chrono::Duration::days(1);

    // Insert mock areas
    let areas = vec![
        (
            "15c0f1a2-3b4c-5d6e-7f8a-9b0c1d2e3f4a",
            "Work",
            "Professional tasks and projects",
        ),
        (
            "16f2a3b4-5c6d-7e8f-9a0b-1c2d3e4f5a6b",
            "Personal",
            "Personal life and hobbies",
        ),
        (
            "17a3b4c5-6d7e-8f9a-0b1c-2d3e4f5a6b7c",
            "Health & Fitness",
            "Health and wellness tasks",
        ),
    ];

    for (uuid, title, _notes) in areas {
        conn.execute(
            "INSERT INTO TMArea (uuid, title, visible, \"index\") VALUES (?, ?, ?, ?)",
            (
                uuid, title, 1, // visible = 1
                0, // index
            ),
        )?;
    }

    // Insert mock tags
    let tags = vec![
        ("1a2b3c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d", "Urgent"),
        ("2b3c4d5e-6f7a-8b9c-0d1e-2f3a4b5c6d7e", "Important"),
        ("3c4d5e6f-7a8b-9c0d-1e2f-3a4b5c6d7e8f", "Meeting"),
        ("4d5e6f7a-8b9c-0d1e-2f3a-4b5c6d7e8f9a", "Email"),
        ("5e6f7a8b-9c0d-1e2f-3a4b-5c6d7e8f9a0b", "Review"),
    ];

    for (uuid, title) in tags {
        conn.execute(
            "INSERT INTO TMTag (uuid, title, created, modified, \"index\") VALUES (?, ?, ?, ?, ?)",
            (uuid, title, now.to_rfc3339(), now.to_rfc3339(), 0),
        )?;
    }

    // Insert mock tasks
    let tasks = vec![
        // Inbox tasks
        (
            "1a2b3c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d",
            "Review quarterly reports",
            "Need to review Q3 financial reports before board meeting",
            None,
            Some(tomorrow),
            "incomplete",
            "to-do",
            None::<String>,
            None::<String>,
            None::<String>,
        ),
        (
            "2b3c4d5e-6f7a-8b9c-0d1e-2f3a4b5c6d7e",
            "Call dentist",
            "Schedule annual checkup",
            None,
            None,
            "incomplete",
            "to-do",
            None::<String>,
            None::<String>,
            None::<String>,
        ),
        (
            "3c4d5e6f-7a8b-9c0d-1e2f-3a4b5c6d7e8f",
            "Buy groceries",
            "Milk, bread, eggs, vegetables",
            None,
            None,
            "incomplete",
            "to-do",
            None::<String>,
            None::<String>,
            None::<String>,
        ),
        // Today's tasks
        (
            "4d5e6f7a-8b9c-0d1e-2f3a-4b5c6d7e8f9a",
            "Team standup meeting",
            "Daily standup at 9 AM",
            Some(today),
            None,
            "incomplete",
            "to-do",
            None::<String>,
            None::<String>,
            None::<String>,
        ),
        (
            "5e6f7a8b-9c0d-1e2f-3a4b-5c6d7e8f9a0b",
            "Code review for PR #123",
            "Review John's changes to the authentication module",
            Some(today),
            None,
            "incomplete",
            "to-do",
            None::<String>,
            None::<String>,
            None::<String>,
        ),
        // Projects
        (
            "6f7a8b9c-0d1e-2f3a-4b5c-6d7e8f9a0b1c",
            "Website Redesign",
            "Complete redesign of company website",
            Some(yesterday),
            Some(today + chrono::Duration::days(30)),
            "incomplete",
            "project",
            None,
            Some("15c0f1a2-3b4c-5d6e-7f8a-9b0c1d2e3f4a".to_string()),
            None,
        ),
        (
            "7a8b9c0d-1e2f-3a4b-5c6d-7e8f9a0b1c2d",
            "Learn Rust",
            "Master the Rust programming language",
            Some(yesterday),
            None,
            "incomplete",
            "project",
            None,
            Some("16f2a3b4-5c6d-7e8f-9a0b-1c2d3e4f5a6b".to_string()),
            None,
        ),
        // Completed tasks
        (
            "8a9b0c1d-2e3f-4a5b-6c7d-8e9f0a1b2c3d",
            "Update documentation",
            "Update API documentation for new endpoints",
            Some(yesterday),
            Some(yesterday),
            "completed",
            "to-do",
            None,
            Some("15c0f1a2-3b4c-5d6e-7f8a-9b0c1d2e3f4a".to_string()),
            None,
        ),
    ];

    for (
        uuid,
        title,
        notes,
        start_date,
        deadline,
        status,
        task_type,
        project_uuid,
        area_uuid,
        parent_uuid,
    ) in tasks
    {
        conn.execute(
            "INSERT INTO TMTask (uuid, title, notes, startDate, deadline, creationDate, userModificationDate, status, type, project, area, heading, \"index\") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                uuid,
                title,
                notes,
                start_date.map(|d| {
                    // Convert to days since 2001-01-01
                    let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
                    d.signed_duration_since(base_date).num_days()
                }),
                deadline.map(|d| {
                    // Convert to days since 2001-01-01
                    let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
                    d.signed_duration_since(base_date).num_days()
                }),
                #[allow(clippy::cast_precision_loss)]
                {
                    now.timestamp() as f64
                },
                #[allow(clippy::cast_precision_loss)]
                {
                    now.timestamp() as f64
                },
                match status {
                    "completed" => 1,
                    "canceled" => 2,
                    "trashed" => 3,
                    _ => 0,
                },
                match task_type {
                    "project" => 1,
                    "heading" => 2,
                    "area" => 3,
                    _ => 0,
                },
                project_uuid,
                area_uuid,
                parent_uuid,
                0,
            ),
        )?;
    }

    // Insert task-tag relationships
    let task_tags = vec![
        ("task-1", "urgent"),
        ("task-1", "important"),
        ("task-4", "meeting"),
        ("task-5", "review"),
        ("task-6", "important"),
    ];

    for (task_uuid, tag_uuid) in task_tags {
        conn.execute(
            "INSERT INTO TMTaskTag (task_uuid, tag_uuid) VALUES (?, ?)",
            (task_uuid, tag_uuid),
        )?;
    }

    Ok(())
}

/// Create mock data for testing
///
/// # Panics
/// Panics if UUID parsing fails (should not happen with hardcoded UUIDs)
#[must_use]
pub fn create_mock_tasks() -> Vec<Task> {
    let now = Utc::now();
    let today = now.date_naive();
    let tomorrow = today + chrono::Duration::days(1);

    vec![
        Task {
            uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
            title: "Review quarterly reports".to_string(),
            notes: Some("Need to review Q3 financial reports before board meeting".to_string()),
            start_date: None,
            deadline: Some(tomorrow),
            created: now,
            modified: now,
            status: TaskStatus::Incomplete,
            task_type: TaskType::Todo,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: vec![],
            children: vec![],
        },
        Task {
            uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap(),
            title: "Call dentist".to_string(),
            notes: Some("Schedule annual checkup".to_string()),
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            status: TaskStatus::Incomplete,
            task_type: TaskType::Todo,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: vec![],
            children: vec![],
        },
    ]
}

/// Create mock projects for testing
///
/// # Panics
/// Panics if UUID parsing fails (should not happen with hardcoded UUIDs)
#[must_use]
pub fn create_mock_projects() -> Vec<Project> {
    let now = Utc::now();
    let today = now.date_naive();
    let deadline = today + chrono::Duration::days(30);

    vec![Project {
        uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440010").unwrap(),
        title: "Website Redesign".to_string(),
        notes: Some("Complete redesign of company website".to_string()),
        start_date: Some(today),
        deadline: Some(deadline),
        created: now,
        modified: now,
        area_uuid: Some(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440100").unwrap()),
        tags: vec![],
        status: TaskStatus::Incomplete,
        tasks: vec![],
    }]
}

/// Create mock areas for testing
///
/// # Panics
/// Panics if UUID parsing fails (should not happen with hardcoded UUIDs)
#[must_use]
pub fn create_mock_areas() -> Vec<Area> {
    let now = Utc::now();

    vec![
        Area {
            uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440100").unwrap(),
            title: "Work".to_string(),
            notes: Some("Professional tasks and projects".to_string()),
            created: now,
            modified: now,
            tags: vec![],
            projects: vec![],
        },
        Area {
            uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440101").unwrap(),
            title: "Personal".to_string(),
            notes: Some("Personal life and hobbies".to_string()),
            created: now,
            modified: now,
            tags: vec![],
            projects: vec![],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_create_test_database() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        let conn = create_test_database(db_path).unwrap();

        // Test that we can query the mock data
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM TMTask").unwrap();
        let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
        assert!(count > 0);
    }

    #[test]
    fn test_mock_data_creation() {
        let tasks = create_mock_tasks();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "Review quarterly reports");

        let projects = create_mock_projects();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].title, "Website Redesign");

        let areas = create_mock_areas();
        assert_eq!(areas.len(), 2);
        assert_eq!(areas[0].title, "Work");
    }

    #[test]
    fn test_create_test_database_error_handling() {
        // Test with invalid path (should fail)
        let invalid_path = "/invalid/path/that/does/not/exist/database.sqlite";
        let result = create_test_database(invalid_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_tasks_structure() {
        let tasks = create_mock_tasks();

        // Test first task
        let first_task = &tasks[0];
        assert_eq!(first_task.title, "Review quarterly reports");
        assert_eq!(first_task.status, TaskStatus::Incomplete);
        assert_eq!(first_task.task_type, TaskType::Todo);
        assert!(first_task.notes.is_some());
        assert!(first_task.deadline.is_some());
        assert!(first_task.start_date.is_none());
        assert!(first_task.project_uuid.is_none());
        assert!(first_task.area_uuid.is_none());
        assert!(first_task.parent_uuid.is_none());
        assert!(first_task.tags.is_empty());
        assert!(first_task.children.is_empty());

        // Test second task
        let second_task = &tasks[1];
        assert_eq!(second_task.title, "Call dentist");
        assert_eq!(second_task.status, TaskStatus::Incomplete);
        assert_eq!(second_task.task_type, TaskType::Todo);
        assert!(second_task.notes.is_some());
        assert!(second_task.deadline.is_none());
        assert!(second_task.start_date.is_none());
        assert!(second_task.project_uuid.is_none());
        assert!(second_task.area_uuid.is_none());
        assert!(second_task.parent_uuid.is_none());
        assert!(second_task.tags.is_empty());
        assert!(second_task.children.is_empty());
    }

    #[test]
    fn test_mock_projects_structure() {
        let projects = create_mock_projects();
        let project = &projects[0];

        assert_eq!(project.title, "Website Redesign");
        assert_eq!(project.status, TaskStatus::Incomplete);
        assert!(project.notes.is_some());
        assert!(project.start_date.is_some());
        assert!(project.deadline.is_some());
        assert!(project.area_uuid.is_some());
        assert!(project.tags.is_empty());
        assert!(project.tasks.is_empty());
    }

    #[test]
    fn test_mock_areas_structure() {
        let areas = create_mock_areas();

        // Test first area
        let first_area = &areas[0];
        assert_eq!(first_area.title, "Work");
        assert!(first_area.notes.is_some());
        assert!(first_area.tags.is_empty());
        assert!(first_area.projects.is_empty());

        // Test second area
        let second_area = &areas[1];
        assert_eq!(second_area.title, "Personal");
        assert!(second_area.notes.is_some());
        assert!(second_area.tags.is_empty());
        assert!(second_area.projects.is_empty());
    }

    #[test]
    fn test_mock_data_timestamps() {
        let tasks = create_mock_tasks();
        let projects = create_mock_projects();
        let areas = create_mock_areas();

        let now = Utc::now();

        // All entities should have recent timestamps
        for task in &tasks {
            assert!(task.created <= now);
            assert!(task.modified <= now);
        }

        for project in &projects {
            assert!(project.created <= now);
            assert!(project.modified <= now);
        }

        for area in &areas {
            assert!(area.created <= now);
            assert!(area.modified <= now);
        }
    }

    #[test]
    fn test_mock_data_uuid_parsing() {
        let tasks = create_mock_tasks();
        let projects = create_mock_projects();
        let areas = create_mock_areas();

        // All UUIDs should be valid
        for task in &tasks {
            assert!(!task.uuid.is_nil());
        }

        for project in &projects {
            assert!(!project.uuid.is_nil());
            if let Some(area_uuid) = project.area_uuid {
                assert!(!area_uuid.is_nil());
            }
        }

        for area in &areas {
            assert!(!area.uuid.is_nil());
        }
    }
}
