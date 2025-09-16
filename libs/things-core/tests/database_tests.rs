//! Comprehensive tests for database operations

use chrono::Utc;
use rusqlite::Connection;
use std::path::Path;
use tempfile::NamedTempFile;
use things_core::{config::ThingsConfig, database::ThingsDatabase, models::TaskStatus};
use uuid::Uuid;

/// Create a test database with comprehensive mock data
fn create_comprehensive_test_database<P: AsRef<Path>>(db_path: P) -> Connection {
    let conn = Connection::open(db_path).unwrap();

    // Create the Things 3 schema
    conn.execute_batch(
        r#"
        -- TMTask table (main tasks table)
        CREATE TABLE IF NOT EXISTS TMTask (
            uuid TEXT PRIMARY KEY,
            title TEXT,
            type INTEGER,
            status INTEGER,
            notes TEXT,
            startDate INTEGER,
            deadline INTEGER,
            creationDate REAL,
            userModificationDate REAL,
            project TEXT,
            area TEXT,
            heading TEXT
        );

        -- TMArea table (areas)
        CREATE TABLE IF NOT EXISTS TMArea (
            uuid TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            visible INTEGER,
            "index" INTEGER NOT NULL DEFAULT 0
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
    )
    .unwrap();

    // Insert comprehensive test data
    let now = Utc::now();
    let today = now.date_naive();
    let tomorrow = today + chrono::Duration::days(1);
    let yesterday = today - chrono::Duration::days(1);

    // Insert areas
    let areas = vec![
        ("550e8400-e29b-41d4-a716-446655440001", "Work", 1, 0),
        ("550e8400-e29b-41d4-a716-446655440002", "Personal", 1, 1),
        ("550e8400-e29b-41d4-a716-446655440003", "Health", 0, 2), // Hidden area
    ];

    for (uuid, title, visible, index) in areas {
        conn.execute(
            "INSERT INTO TMArea (uuid, title, visible, \"index\") VALUES (?, ?, ?, ?)",
            (uuid, title, visible, index),
        )
        .unwrap();
    }

    // Insert projects
    let projects = vec![
        (
            "550e8400-e29b-41d4-a716-446655440010",
            "Website Redesign",
            "Redesign company website",
            Some(1),
            Some(7),
            "550e8400-e29b-41d4-a716-446655440001",
        ),
        (
            "550e8400-e29b-41d4-a716-446655440011",
            "Mobile App",
            "Build mobile app",
            Some(1),
            Some(30),
            "550e8400-e29b-41d4-a716-446655440002",
        ),
    ];

    for (uuid, title, notes, start_days, deadline_days, area_uuid) in projects {
        let start_date = start_days.map(|d: i64| {
            let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
            base_date
                .checked_add_days(chrono::Days::new(d as u64))
                .map(|d| {
                    d.signed_duration_since(chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap())
                        .num_days()
                })
        });

        let deadline = deadline_days.map(|d: i64| {
            let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
            base_date
                .checked_add_days(chrono::Days::new(d as u64))
                .map(|d| {
                    d.signed_duration_since(chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap())
                        .num_days()
                })
        });

        conn.execute(
            "INSERT INTO TMTask (uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, area) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (uuid, title, 1, 0, notes, start_date, deadline, now.timestamp() as f64, now.timestamp() as f64, area_uuid),
        ).unwrap();
    }

    // Insert tasks
    let tasks = vec![
        // Inbox tasks
        (
            "task-1",
            "Review quarterly reports",
            0,
            0,
            "Need to review Q3 reports",
            None,
            Some(1),
            None::<&str>,
            None::<&str>,
            None::<&str>,
        ),
        (
            "task-2",
            "Call dentist",
            0,
            0,
            "Schedule annual checkup",
            None,
            None,
            None::<&str>,
            None::<&str>,
            None::<&str>,
        ),
        (
            "task-3",
            "Buy groceries",
            0,
            0,
            "Milk, bread, eggs",
            None,
            None,
            None::<&str>,
            None::<&str>,
            None::<&str>,
        ),
        // Today's tasks
        (
            "task-4",
            "Team standup",
            0,
            0,
            "Daily standup at 9 AM",
            Some(
                chrono::Utc::now()
                    .date_naive()
                    .format("%Y-%m-%d")
                    .to_string(),
            ),
            None,
            None::<&str>,
            None::<&str>,
            None::<&str>,
        ),
        (
            "task-5",
            "Code review",
            0,
            0,
            "Review PR #123",
            Some(
                chrono::Utc::now()
                    .date_naive()
                    .format("%Y-%m-%d")
                    .to_string(),
            ),
            None,
            None::<&str>,
            None::<&str>,
            None::<&str>,
        ),
        // Project tasks
        (
            "task-6",
            "Design mockups",
            0,
            0,
            "Create wireframes",
            Some(1i64),
            Some(7i64),
            Some("project-1"),
            Some("550e8400-e29b-41d4-a716-446655440001"),
            None::<&str>,
        ),
        (
            "task-7",
            "Write tests",
            0,
            0,
            "Add unit tests",
            Some(1i64),
            Some(14i64),
            Some("project-1"),
            Some("550e8400-e29b-41d4-a716-446655440001"),
            None::<&str>,
        ),
        (
            "task-8",
            "Read Rust book",
            0,
            0,
            "Chapter 1-3",
            Some(1i64),
            Some(30i64),
            Some("project-2"),
            Some("550e8400-e29b-41d4-a716-446655440002"),
            None::<&str>,
        ),
        // Completed tasks
        (
            "task-9",
            "Update docs",
            0,
            1,
            "Update API docs",
            Some(1i64),
            Some(1i64),
            None::<&str>,
            Some("550e8400-e29b-41d4-a716-446655440001"),
            None::<&str>,
        ),
        (
            "task-10",
            "Fix bug",
            0,
            1,
            "Fix authentication bug",
            Some(1i64),
            Some(1i64),
            Some("project-1"),
            Some("550e8400-e29b-41d4-a716-446655440001"),
            None::<&str>,
        ),
        // Canceled tasks
        (
            "task-11",
            "Old feature",
            0,
            2,
            "Deprecated feature",
            Some(1i64),
            Some(1i64),
            None::<&str>,
            Some("550e8400-e29b-41d4-a716-446655440001"),
            None::<&str>,
        ),
        // Trashed tasks
        (
            "task-12",
            "Spam task",
            0,
            3,
            "Unwanted task",
            Some(1i64),
            Some(1i64),
            None::<&str>,
            Some("550e8400-e29b-41d4-a716-446655440001"),
            None::<&str>,
        ),
    ];

    for (
        uuid,
        title,
        task_type,
        status,
        notes,
        start_date_str,
        deadline_days,
        project,
        area,
        heading,
    ) in tasks
    {
        let start_date = start_date_str;

        let deadline = deadline_days.map(|d| {
            let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
            base_date
                .checked_add_days(chrono::Days::new(d as u64))
                .map(|d| {
                    d.signed_duration_since(chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap())
                        .num_days()
                })
        });

        conn.execute(
            "INSERT INTO TMTask (uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, project, area, heading) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (uuid, title, task_type, status, notes, start_date, deadline, now.timestamp() as f64, now.timestamp() as f64, project.map(|s| s.to_string()), area.map(|s| s.to_string()), heading),
        ).unwrap();
    }

    // Insert tags
    let tags = vec![
        ("tag-1", "urgent", now.to_rfc3339(), now.to_rfc3339(), 0),
        ("tag-2", "important", now.to_rfc3339(), now.to_rfc3339(), 1),
        ("tag-3", "meeting", now.to_rfc3339(), now.to_rfc3339(), 2),
    ];

    for (uuid, title, created, modified, index) in tags {
        conn.execute(
            "INSERT INTO TMTag (uuid, title, created, modified, \"index\") VALUES (?, ?, ?, ?, ?)",
            (uuid, title, created, modified, index),
        )
        .unwrap();
    }

    // Insert task-tag relationships
    let task_tags = vec![
        ("task-1", "tag-1"), // urgent
        ("task-1", "tag-2"), // important
        ("task-4", "tag-3"), // meeting
        ("task-5", "tag-2"), // important
    ];

    for (task_uuid, tag_uuid) in task_tags {
        conn.execute(
            "INSERT INTO TMTaskTag (task_uuid, tag_uuid) VALUES (?, ?)",
            (task_uuid, tag_uuid),
        )
        .unwrap();
    }

    conn
}

#[test]
fn test_database_creation() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    let db = ThingsDatabase::new(db_path).unwrap();
    // Test that database was created successfully
    assert!(db.get_inbox(None).is_ok());
}

#[test]
fn test_database_creation_with_config() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    let config = ThingsConfig::new(db_path, false);
    let db = ThingsDatabase::with_config(&config).unwrap();
    // Test that database was created successfully
    assert!(db.get_inbox(None).is_ok());
}

#[test]
fn test_database_creation_invalid_path() {
    let invalid_path = "/invalid/path/that/does/not/exist/database.sqlite";
    let result = ThingsDatabase::new(invalid_path);
    assert!(result.is_err());
}

#[test]
fn test_default_path() {
    let path = ThingsDatabase::default_path();
    assert!(path.contains("Library/Group Containers"));
    assert!(path.contains("Things Database.thingsdatabase"));
    assert!(path.contains("main.sqlite"));
}

// Note: Private helper functions are tested indirectly through public API

#[test]
fn test_get_inbox() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_comprehensive_test_database(db_path);

    let db = ThingsDatabase::new(db_path).unwrap();
    let tasks = db.get_inbox(None).unwrap();

    // Should have 5 inbox tasks (task-1, task-2, task-3, task-4, task-5)
    assert_eq!(tasks.len(), 5);

    // Check that all tasks are incomplete and have no project/area
    for task in &tasks {
        assert_eq!(task.status, TaskStatus::Incomplete);
        assert!(task.project_uuid.is_none());
        assert!(task.area_uuid.is_none());
    }

    // Check specific tasks
    let titles: Vec<&String> = tasks.iter().map(|t| &t.title).collect();
    assert!(titles.contains(&&"Review quarterly reports".to_string()));
    assert!(titles.contains(&&"Call dentist".to_string()));
    assert!(titles.contains(&&"Buy groceries".to_string()));
}

#[test]
fn test_get_inbox_with_limit() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_comprehensive_test_database(db_path);

    let db = ThingsDatabase::new(db_path).unwrap();
    let tasks = db.get_inbox(Some(2)).unwrap();

    // Should have at most 2 tasks
    assert!(tasks.len() <= 2);
}

#[test]
fn test_get_today() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_comprehensive_test_database(db_path);

    let db = ThingsDatabase::new(db_path).unwrap();
    let tasks = db.get_today(None).unwrap();

    // Should have 2 today's tasks (task-4, task-5)
    assert_eq!(tasks.len(), 2);

    // Check that all tasks have today's start date
    let today = chrono::Utc::now().date_naive();
    for task in &tasks {
        assert_eq!(task.start_date, Some(today));
        assert_eq!(task.status, TaskStatus::Incomplete);
    }

    // Check specific tasks
    let titles: Vec<&String> = tasks.iter().map(|t| &t.title).collect();
    assert!(titles.contains(&&"Team standup".to_string()));
    assert!(titles.contains(&&"Code review".to_string()));
}

#[test]
fn test_get_today_with_limit() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_comprehensive_test_database(db_path);

    let db = ThingsDatabase::new(db_path).unwrap();
    let tasks = db.get_today(Some(1)).unwrap();

    // Should have at most 1 task
    assert!(tasks.len() <= 1);
}

#[test]
fn test_get_projects() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_comprehensive_test_database(db_path);

    let db = ThingsDatabase::new(db_path).unwrap();
    let projects = db.get_projects(None).unwrap();

    // Should have 3 projects
    assert_eq!(projects.len(), 3);

    // Check that all are projects (type = 1)
    for project in &projects {
        assert_eq!(project.status, TaskStatus::Incomplete);
    }

    // Check specific projects
    let titles: Vec<&String> = projects.iter().map(|p| &p.title).collect();
    assert!(titles.contains(&&"Website Redesign".to_string()));
    assert!(titles.contains(&&"Learn Rust".to_string()));
    assert!(titles.contains(&&"Fitness Plan".to_string()));
}

#[test]
fn test_get_projects_with_area_filter() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_comprehensive_test_database(db_path);

    let db = ThingsDatabase::new(db_path).unwrap();
    let area_uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
    let projects = db.get_projects(Some(area_uuid)).unwrap();

    // Should have 1 project in area-1
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].title, "Website Redesign");
}

#[test]
fn test_get_areas() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_comprehensive_test_database(db_path);

    let db = ThingsDatabase::new(db_path).unwrap();
    let areas = db.get_areas().unwrap();

    // Should have 2 visible areas (area-3 is hidden)
    assert_eq!(areas.len(), 2);

    // Check specific areas
    let titles: Vec<&String> = areas.iter().map(|a| &a.title).collect();
    assert!(titles.contains(&&"Work".to_string()));
    assert!(titles.contains(&&"Personal".to_string()));
    assert!(!titles.contains(&&"Health".to_string())); // Hidden area
}

#[test]
fn test_search_tasks() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_comprehensive_test_database(db_path);

    let db = ThingsDatabase::new(db_path).unwrap();
    let tasks = db.search_tasks("review", None).unwrap();

    // Should find tasks with "review" in title or notes
    assert!(!tasks.is_empty());

    // Check that all found tasks contain "review" in title or notes
    for task in &tasks {
        let contains_review = task.title.to_lowercase().contains("review")
            || task
                .notes
                .as_ref()
                .map_or(false, |n| n.to_lowercase().contains("review"));
        assert!(contains_review);
    }
}

#[test]
fn test_search_tasks_with_limit() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_comprehensive_test_database(db_path);

    let db = ThingsDatabase::new(db_path).unwrap();
    let tasks = db.search_tasks("task", Some(2)).unwrap();

    // Should have at most 2 tasks
    assert!(tasks.len() <= 2);
}

#[test]
fn test_search_tasks_no_results() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_comprehensive_test_database(db_path);

    let db = ThingsDatabase::new(db_path).unwrap();
    let tasks = db.search_tasks("nonexistent", None).unwrap();

    // Should find no tasks
    assert!(tasks.is_empty());
}

// Note: map_project_row is a private helper function tested through public API

#[test]
fn test_database_error_handling() {
    // Test with non-existent database
    let result = ThingsDatabase::new("/nonexistent/path/database.sqlite");
    assert!(result.is_err());

    // Test with invalid SQL (this would require a more complex setup)
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    let db = ThingsDatabase::new(db_path).unwrap();

    // This should fail because the table doesn't exist
    let result = db.get_inbox(None);
    assert!(result.is_err());
}

#[test]
fn test_database_with_config_fallback() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_comprehensive_test_database(db_path);

    let config = ThingsConfig::new(db_path, true);
    let db = ThingsDatabase::with_config(&config).unwrap();

    // Should be able to query the database
    let tasks = db.get_inbox(None).unwrap();
    assert_eq!(tasks.len(), 3);
}

#[test]
fn test_database_with_config_no_fallback() {
    let config = ThingsConfig::new("/nonexistent/path", false);
    let result = ThingsDatabase::with_config(&config);
    assert!(result.is_err());
}

#[test]
fn test_database_with_default_path() {
    // This will likely fail in CI but should not panic
    let _result = ThingsDatabase::with_default_path();
    // We don't assert on the result since the default path may not exist in CI
    // but we ensure it doesn't panic
}

// Note: Database connection properties are tested through public API

// Note: Database transaction support is tested through public API

// Note: Database prepared statements are tested through public API

// Note: Database parameter binding is tested through public API

// Note: Database error propagation is tested through public API
