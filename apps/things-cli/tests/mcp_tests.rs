//! Comprehensive tests for MCP server functionality

use serde_json::json;
use std::path::Path;
use tempfile::NamedTempFile;
use things_cli::mcp::{CallToolRequest, Content, ThingsMcpServer};
use things_core::{config::ThingsConfig, database::ThingsDatabase};

/// Create a test MCP server with mock database
fn create_test_mcp_server() -> ThingsMcpServer {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_comprehensive_test_database(db_path);

    let db = ThingsDatabase::new(db_path).unwrap();
    let config = ThingsConfig::new(db_path, false);

    ThingsMcpServer::new(db, config)
}

/// Create a comprehensive test database with mock data
fn create_comprehensive_test_database<P: AsRef<Path>>(db_path: P) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(db_path).unwrap();

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

        -- TMTaskTag table (task-tag relationships)
        CREATE TABLE IF NOT EXISTS TMTaskTag (
            task_uuid TEXT NOT NULL,
            tag_uuid TEXT NOT NULL,
            PRIMARY KEY (task_uuid, tag_uuid),
            FOREIGN KEY (task_uuid) REFERENCES TMTask(uuid),
            FOREIGN KEY (tag_uuid) REFERENCES TMTag(uuid)
        );
        "#,
    )
    .unwrap();

    let now = chrono::Utc::now();

    // Insert areas
    let areas = vec![("area-1", "Work", 1, 0), ("area-2", "Personal", 1, 1)];

    for (uuid, title, visible, index) in areas {
        conn.execute(
            "INSERT INTO TMArea (uuid, title, visible, \"index\") VALUES (?, ?, ?, ?)",
            (uuid, title, visible, index),
        )
        .unwrap();
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
    ];

    for (
        uuid,
        title,
        task_type,
        status,
        notes,
        start_days,
        deadline_days,
        project,
        area,
        heading,
    ) in tasks
    {
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
            "INSERT INTO TMTask (uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, project, area, heading) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (uuid, title, task_type, status, notes, start_date, deadline, now.timestamp() as f64, now.timestamp() as f64, project.map(|s| s.to_string()), area.map(|s| s.to_string()), heading),
        ).unwrap();
    }

    conn
}

#[tokio::test]
async fn test_mcp_server_creation() {
    let _server = create_test_mcp_server();
    // Server should be created successfully - if we get here, creation succeeded
}

#[tokio::test]
async fn test_list_tools() {
    let server = create_test_mcp_server();
    let result = server.list_tools().await.unwrap();

    assert!(result.tools.len() > 10); // Should have many tools

    // Check for specific tools
    let tool_names: Vec<&String> = result.tools.iter().map(|t| &t.name).collect();
    assert!(tool_names.contains(&&"get_inbox".to_string()));
    assert!(tool_names.contains(&&"get_today".to_string()));
    assert!(tool_names.contains(&&"get_projects".to_string()));
    assert!(tool_names.contains(&&"get_areas".to_string()));
    assert!(tool_names.contains(&&"search_tasks".to_string()));
    assert!(tool_names.contains(&&"create_task".to_string()));
    assert!(tool_names.contains(&&"update_task".to_string()));
    assert!(tool_names.contains(&&"get_productivity_metrics".to_string()));
    assert!(tool_names.contains(&&"export_data".to_string()));
    assert!(tool_names.contains(&&"backup_database".to_string()));
}

#[tokio::test]
async fn test_tool_schemas() {
    let server = create_test_mcp_server();
    let result = server.list_tools().await.unwrap();

    // Check that each tool has proper schema
    for tool in &result.tools {
        assert!(!tool.name.is_empty());
        assert!(!tool.description.is_empty());
        assert!(tool.input_schema.is_object());
    }
}

#[tokio::test]
async fn test_get_inbox_tool() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "get_inbox".to_string(),
        arguments: Some(json!({ "limit": 5 })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            // Should be valid JSON
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed.is_array());
            // Should have some tasks
            let tasks = parsed.as_array().unwrap();
            assert!(!tasks.is_empty());
        }
    }
}

#[tokio::test]
async fn test_get_inbox_tool_no_limit() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "get_inbox".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
}

#[tokio::test]
async fn test_get_today_tool() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "get_today".to_string(),
        arguments: Some(json!({ "limit": 3 })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed.is_array());
        }
    }
}

#[tokio::test]
async fn test_get_projects_tool() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "get_projects".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed.is_array());
        }
    }
}

#[tokio::test]
async fn test_get_projects_tool_with_area() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "get_projects".to_string(),
        arguments: Some(json!({ "area_uuid": "test-area-uuid" })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
}

#[tokio::test]
async fn test_get_areas_tool() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "get_areas".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed.is_array());
        }
    }
}

#[tokio::test]
async fn test_search_tasks_tool() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "search_tasks".to_string(),
        arguments: Some(json!({ "query": "test", "limit": 5 })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed.is_array());
        }
    }
}

#[tokio::test]
async fn test_search_tasks_tool_missing_query() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "search_tasks".to_string(),
        arguments: Some(json!({ "limit": 5 })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter"));
        }
    }
}

#[tokio::test]
async fn test_create_task_tool() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "create_task".to_string(),
        arguments: Some(json!({
            "title": "Test Task",
            "notes": "Test notes",
            "project_uuid": "test-project-uuid"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["title"], "Test Task");
            assert_eq!(parsed["status"], "placeholder");
        }
    }
}

#[tokio::test]
async fn test_create_task_tool_missing_title() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "create_task".to_string(),
        arguments: Some(json!({
            "notes": "Test notes"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter"));
        }
    }
}

#[tokio::test]
async fn test_update_task_tool() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "update_task".to_string(),
        arguments: Some(json!({
            "uuid": "test-task-uuid",
            "title": "Updated Task",
            "status": "completed"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["uuid"], "test-task-uuid");
            assert_eq!(parsed["status"], "placeholder");
        }
    }
}

#[tokio::test]
async fn test_update_task_tool_missing_uuid() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "update_task".to_string(),
        arguments: Some(json!({
            "title": "Updated Task"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter"));
        }
    }
}

#[tokio::test]
async fn test_get_productivity_metrics_tool() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "get_productivity_metrics".to_string(),
        arguments: Some(json!({ "days": 7 })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["period_days"], 7);
            assert!(parsed["inbox_tasks_count"].is_number());
            assert!(parsed["today_tasks_count"].is_number());
            assert!(parsed["projects_count"].is_number());
            assert!(parsed["areas_count"].is_number());
        }
    }
}

#[tokio::test]
async fn test_get_productivity_metrics_tool_default_days() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "get_productivity_metrics".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["period_days"], 7); // Default value
        }
    }
}

#[tokio::test]
async fn test_export_data_tool_json() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "json",
            "data_type": "tasks"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed["inbox"].is_array());
            assert!(parsed["today"].is_array());
        }
    }
}

#[tokio::test]
async fn test_export_data_tool_all_data() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "json",
            "data_type": "all"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed["inbox"].is_array());
            assert!(parsed["today"].is_array());
            assert!(parsed["projects"].is_array());
            assert!(parsed["areas"].is_array());
        }
    }
}

#[tokio::test]
async fn test_export_data_tool_missing_parameters() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "json"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter"));
        }
    }
}

#[tokio::test]
async fn test_export_data_tool_invalid_format() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "invalid",
            "data_type": "tasks"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Invalid format"));
        }
    }
}

#[tokio::test]
async fn test_bulk_create_tasks_tool() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "bulk_create_tasks".to_string(),
        arguments: Some(json!({
            "tasks": [
                {"title": "Task 1", "notes": "Notes 1"},
                {"title": "Task 2", "notes": "Notes 2"}
            ]
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["tasks_count"], 2);
            assert_eq!(parsed["status"], "placeholder");
        }
    }
}

#[tokio::test]
async fn test_bulk_create_tasks_tool_missing_tasks() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "bulk_create_tasks".to_string(),
        arguments: Some(json!({})),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter"));
        }
    }
}

#[tokio::test]
async fn test_get_recent_tasks_tool() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "get_recent_tasks".to_string(),
        arguments: Some(json!({ "limit": 5, "hours": 24 })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["hours_lookback"], 24);
            assert!(parsed["tasks"].is_array());
        }
    }
}

#[tokio::test]
async fn test_get_recent_tasks_tool_default_hours() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "get_recent_tasks".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["hours_lookback"], 24); // Default value
        }
    }
}

#[tokio::test]
async fn test_get_performance_stats_tool() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "get_performance_stats".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed["summary"].is_object());
            assert!(parsed["operation_stats"].is_object());
        }
    }
}

#[tokio::test]
async fn test_get_system_metrics_tool() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "get_system_metrics".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed.is_object());
        }
    }
}

#[tokio::test]
async fn test_get_cache_stats_tool() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "get_cache_stats".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed.is_object());
        }
    }
}

#[tokio::test]
async fn test_unknown_tool() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "unknown_tool".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Unknown tool"));
        }
    }
}

#[tokio::test]
async fn test_backup_database_tool() {
    let server = create_test_mcp_server();
    let temp_dir = tempfile::tempdir().unwrap();
    let backup_dir = temp_dir.path().to_str().unwrap();

    let request = CallToolRequest {
        name: "backup_database".to_string(),
        arguments: Some(json!({
            "backup_dir": backup_dir,
            "description": "Test backup"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    // The backup will fail because the database path doesn't exist
    // This is expected behavior in the test environment
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Error"));
        }
    }
}

#[tokio::test]
async fn test_backup_database_tool_missing_backup_dir() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "backup_database".to_string(),
        arguments: Some(json!({
            "description": "Test backup"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter"));
        }
    }
}

#[tokio::test]
async fn test_list_backups_tool() {
    let server = create_test_mcp_server();
    let temp_dir = tempfile::tempdir().unwrap();
    let backup_dir = temp_dir.path().to_str().unwrap();

    let request = CallToolRequest {
        name: "list_backups".to_string(),
        arguments: Some(json!({
            "backup_dir": backup_dir
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed["backups"].is_array());
            assert!(parsed["count"].is_number());
        }
    }
}

#[tokio::test]
async fn test_list_backups_tool_missing_backup_dir() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "list_backups".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter"));
        }
    }
}

#[tokio::test]
async fn test_restore_database_tool() {
    let server = create_test_mcp_server();
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let backup_path = temp_file.path().to_str().unwrap();

    let request = CallToolRequest {
        name: "restore_database".to_string(),
        arguments: Some(json!({
            "backup_path": backup_path
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    // The restore will fail because the database path doesn't exist
    // This is expected behavior in the test environment
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Error"));
        }
    }
}

#[tokio::test]
async fn test_restore_database_tool_missing_backup_path() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "restore_database".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter"));
        }
    }
}

#[tokio::test]
async fn test_tool_schemas_validation() {
    let server = create_test_mcp_server();
    let result = server.list_tools().await.unwrap();

    // Check that required tools have proper schemas
    for tool in &result.tools {
        match tool.name.as_str() {
            "search_tasks" => {
                let schema = &tool.input_schema;
                assert!(schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("query")));
            }
            "create_task" => {
                let schema = &tool.input_schema;
                assert!(schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("title")));
            }
            "update_task" => {
                let schema = &tool.input_schema;
                assert!(schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("uuid")));
            }
            "export_data" => {
                let schema = &tool.input_schema;
                let required = schema["required"].as_array().unwrap();
                assert!(required.contains(&json!("format")));
                assert!(required.contains(&json!("data_type")));
            }
            "backup_database" => {
                let schema = &tool.input_schema;
                assert!(schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("backup_dir")));
            }
            "restore_database" => {
                let schema = &tool.input_schema;
                assert!(schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("backup_path")));
            }
            "list_backups" => {
                let schema = &tool.input_schema;
                assert!(schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("backup_dir")));
            }
            _ => {
                // Other tools may not have required fields
            }
        }
    }
}

#[tokio::test]
async fn test_error_handling() {
    let server = create_test_mcp_server();

    // Test with invalid JSON in arguments
    let request = CallToolRequest {
        name: "get_inbox".to_string(),
        arguments: Some(json!({ "limit": "invalid" })), // Should be number
    };

    let result = server.call_tool(request).await.unwrap();
    // Should not error, just ignore invalid limit
    assert!(!result.is_error);
}

#[tokio::test]
async fn test_empty_arguments() {
    let server = create_test_mcp_server();

    // Test with empty arguments object
    let request = CallToolRequest {
        name: "get_inbox".to_string(),
        arguments: Some(json!({})),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
}
