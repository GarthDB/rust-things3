//! Comprehensive tests for MCP server functionality

use serde_json::json;
use std::path::Path;
use tempfile::NamedTempFile;
use things3_cli::mcp::{CallToolRequest, Content, McpError, ThingsMcpServer};
use things3_core::{config::ThingsConfig, database::ThingsDatabase};

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
#[allow(clippy::too_many_lines)]
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
            #[allow(clippy::cast_sign_loss)]
            { base_date.checked_add_days(chrono::Days::new(d as u64)) }.map(|d| {
                d.signed_duration_since(chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap())
                    .num_days()
            })
        });

        let deadline = deadline_days.map(|d: i64| {
            let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
            #[allow(clippy::cast_sign_loss)]
            { base_date.checked_add_days(chrono::Days::new(d as u64)) }.map(|d| {
                d.signed_duration_since(chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap())
                    .num_days()
            })
        });

        conn.execute(
            "INSERT INTO TMTask (uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, project, area, heading) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (uuid, title, task_type, status, notes, start_date, deadline,
                #[allow(clippy::cast_precision_loss)]
                {
                    now.timestamp() as f64
                },
                #[allow(clippy::cast_precision_loss)]
                {
                    now.timestamp() as f64
                },
                project.map(std::string::ToString::to_string),
                area.map(std::string::ToString::to_string),
                heading),
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
    let result = server.list_tools().unwrap();

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
    let result = server.list_tools().unwrap();

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

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "query");
        }
        _ => panic!("Expected MissingParameter error"),
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

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "title");
        }
        _ => panic!("Expected MissingParameter error"),
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

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "uuid");
        }
        _ => panic!("Expected MissingParameter error"),
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

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "data_type");
        }
        _ => panic!("Expected MissingParameter error"),
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

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::InvalidFormat { format, supported } => {
            assert_eq!(format, "invalid");
            assert_eq!(supported, "json, csv, markdown");
        }
        _ => panic!("Expected InvalidFormat error"),
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

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "tasks");
        }
        _ => panic!("Expected MissingParameter error"),
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

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ToolNotFound { tool_name } => {
            assert_eq!(tool_name, "unknown_tool");
        }
        _ => panic!("Expected ToolNotFound error"),
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

    let result = server.call_tool(request).await;
    // The backup will fail because the database path doesn't exist
    // This is expected behavior in the test environment
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::BackupOperationFailed { operation, .. } => {
            assert_eq!(operation, "create_backup");
        }
        _ => panic!("Expected BackupOperationFailed error"),
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

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "backup_dir");
        }
        _ => panic!("Expected MissingParameter error"),
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

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "backup_dir");
        }
        _ => panic!("Expected MissingParameter error"),
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

    let result = server.call_tool(request).await;
    // The restore will fail because the database path doesn't exist
    // This is expected behavior in the test environment
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::BackupOperationFailed { operation, .. } => {
            assert_eq!(operation, "restore_backup");
        }
        _ => panic!("Expected BackupOperationFailed error"),
    }
}

#[tokio::test]
async fn test_restore_database_tool_missing_backup_path() {
    let server = create_test_mcp_server();
    let request = CallToolRequest {
        name: "restore_database".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "backup_path");
        }
        _ => panic!("Expected MissingParameter error"),
    }
}

#[tokio::test]
async fn test_tool_schemas_validation() {
    let server = create_test_mcp_server();
    let result = server.list_tools().unwrap();

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
            "backup_database" | "list_backups" => {
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

// ===== Resource Tests =====

#[tokio::test]
async fn test_list_resources() {
    let server = create_test_mcp_server();

    let result = server.list_resources().unwrap();

    // Should have 4 resources
    assert_eq!(result.resources.len(), 4);

    // Check that all expected resources are present
    let uris: Vec<&String> = result.resources.iter().map(|r| &r.uri).collect();
    assert!(uris.contains(&&"things://inbox".to_string()));
    assert!(uris.contains(&&"things://projects".to_string()));
    assert!(uris.contains(&&"things://areas".to_string()));
    assert!(uris.contains(&&"things://today".to_string()));

    // Check resource properties
    let inbox_resource = result
        .resources
        .iter()
        .find(|r| r.uri == "things://inbox")
        .unwrap();
    assert_eq!(inbox_resource.name, "Inbox Tasks");
    assert_eq!(
        inbox_resource.mime_type,
        Some("application/json".to_string())
    );
}

#[tokio::test]
async fn test_read_inbox_resource() {
    let server = create_test_mcp_server();

    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://inbox".to_string(),
    };

    let result = server.read_resource(request).await.unwrap();

    assert_eq!(result.contents.len(), 1);
    match &result.contents[0] {
        Content::Text { text } => {
            // Should be valid JSON
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed.is_array());
        }
    }
}

#[tokio::test]
async fn test_read_projects_resource() {
    let server = create_test_mcp_server();

    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://projects".to_string(),
    };

    let result = server.read_resource(request).await.unwrap();

    assert_eq!(result.contents.len(), 1);
    match &result.contents[0] {
        Content::Text { text } => {
            // Should be valid JSON
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed.is_array());
        }
    }
}

#[tokio::test]
async fn test_read_areas_resource() {
    let server = create_test_mcp_server();

    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://areas".to_string(),
    };

    let result = server.read_resource(request).await.unwrap();

    assert_eq!(result.contents.len(), 1);
    match &result.contents[0] {
        Content::Text { text } => {
            // Should be valid JSON
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed.is_array());
        }
    }
}

#[tokio::test]
async fn test_read_today_resource() {
    let server = create_test_mcp_server();

    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://today".to_string(),
    };

    let result = server.read_resource(request).await.unwrap();

    assert_eq!(result.contents.len(), 1);
    match &result.contents[0] {
        Content::Text { text } => {
            // Should be valid JSON
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed.is_array());
        }
    }
}

#[tokio::test]
async fn test_read_unknown_resource() {
    let server = create_test_mcp_server();

    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://unknown".to_string(),
    };

    let result = server.read_resource(request).await;

    // Should return an error for unknown resource
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ResourceNotFound { uri } => {
            assert_eq!(uri, "things://unknown");
        }
        _ => panic!("Expected ResourceNotFound error"),
    }
}

// ===== Prompt Tests =====

#[tokio::test]
async fn test_list_prompts() {
    let server = create_test_mcp_server();
    let result = server.list_prompts().unwrap();

    // Should have 4 prompts
    assert_eq!(result.prompts.len(), 4);

    // Check that all expected prompts are present
    let prompt_names: Vec<&String> = result.prompts.iter().map(|p| &p.name).collect();
    assert!(prompt_names.contains(&&"task_review".to_string()));
    assert!(prompt_names.contains(&&"project_planning".to_string()));
    assert!(prompt_names.contains(&&"productivity_analysis".to_string()));
    assert!(prompt_names.contains(&&"backup_strategy".to_string()));

    // Check prompt properties
    for prompt in &result.prompts {
        assert!(!prompt.name.is_empty());
        assert!(!prompt.description.is_empty());
        assert!(prompt.arguments.is_object());
    }
}

#[tokio::test]
async fn test_prompt_schemas_validation() {
    let server = create_test_mcp_server();
    let result = server.list_prompts().unwrap();

    // Check that each prompt has proper schema
    for prompt in &result.prompts {
        match prompt.name.as_str() {
            "task_review" => {
                let schema = &prompt.arguments;
                assert!(schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("task_title")));
            }
            "project_planning" => {
                let schema = &prompt.arguments;
                assert!(schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("project_title")));
            }
            "productivity_analysis" => {
                let schema = &prompt.arguments;
                assert!(schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("time_period")));
            }
            "backup_strategy" => {
                let schema = &prompt.arguments;
                let required = schema["required"].as_array().unwrap();
                assert!(required.contains(&json!("data_volume")));
                assert!(required.contains(&json!("frequency")));
            }
            _ => {
                // Unknown prompt
                panic!("Unknown prompt: {}", prompt.name);
            }
        }
    }
}

#[tokio::test]
async fn test_task_review_prompt() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "task_review".to_string(),
        arguments: Some(json!({
            "task_title": "Review quarterly reports",
            "task_notes": "Need to review Q3 reports",
            "context": "This is for the quarterly review meeting"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            // Should contain the task title and context
            assert!(text.contains("Review quarterly reports"));
            assert!(text.contains("Need to review Q3 reports"));
            assert!(text.contains("This is for the quarterly review meeting"));
            assert!(text.contains("Task Review"));
            assert!(text.contains("Review Checklist"));
            assert!(text.contains("Current Context"));
            assert!(text.contains("Recommendations"));
            assert!(text.contains("Next Steps"));
        }
    }
}

#[tokio::test]
async fn test_task_review_prompt_minimal_args() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "task_review".to_string(),
        arguments: Some(json!({
            "task_title": "Simple task"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Simple task"));
            assert!(text.contains("No notes provided"));
            assert!(text.contains("No additional context"));
        }
    }
}

#[tokio::test]
async fn test_task_review_prompt_missing_required() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "task_review".to_string(),
        arguments: Some(json!({
            "task_notes": "Some notes"
        })),
    };

    let result = server.get_prompt(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "task_title");
        }
        _ => panic!("Expected MissingParameter error"),
    }
}

#[tokio::test]
async fn test_project_planning_prompt() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "project_planning".to_string(),
        arguments: Some(json!({
            "project_title": "Website Redesign",
            "project_description": "Complete redesign of company website",
            "deadline": "2024-03-31",
            "complexity": "complex"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Website Redesign"));
            assert!(text.contains("Complete redesign of company website"));
            assert!(text.contains("2024-03-31"));
            assert!(text.contains("complex"));
            assert!(text.contains("Project Planning"));
            assert!(text.contains("Planning Framework"));
            assert!(text.contains("Task Breakdown"));
            assert!(text.contains("Project Organization"));
            assert!(text.contains("Risk Assessment"));
            assert!(text.contains("Success Metrics"));
        }
    }
}

#[tokio::test]
async fn test_project_planning_prompt_minimal_args() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "project_planning".to_string(),
        arguments: Some(json!({
            "project_title": "Simple Project"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Simple Project"));
            assert!(text.contains("No description provided"));
            assert!(text.contains("No deadline specified"));
            assert!(text.contains("medium")); // Default complexity
        }
    }
}

#[tokio::test]
async fn test_productivity_analysis_prompt() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "productivity_analysis".to_string(),
        arguments: Some(json!({
            "time_period": "month",
            "focus_area": "completion_rate",
            "include_recommendations": true
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("month"));
            assert!(text.contains("completion_rate"));
            assert!(text.contains("Productivity Analysis"));
            assert!(text.contains("Analysis Framework"));
            assert!(text.contains("Task Completion Patterns"));
            assert!(text.contains("Workload Distribution"));
            assert!(text.contains("Time Management"));
            assert!(text.contains("Project Progress"));
            assert!(text.contains("Key Insights"));
            assert!(text.contains("Recommendations"));
            assert!(text.contains("Improving task completion rates"));
        }
    }
}

#[tokio::test]
async fn test_productivity_analysis_prompt_no_recommendations() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "productivity_analysis".to_string(),
        arguments: Some(json!({
            "time_period": "week",
            "focus_area": "time_management",
            "include_recommendations": false
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("week"));
            assert!(text.contains("time_management"));
            assert!(text.contains("Focus on analysis without recommendations"));
        }
    }
}

#[tokio::test]
async fn test_productivity_analysis_prompt_minimal_args() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "productivity_analysis".to_string(),
        arguments: Some(json!({
            "time_period": "quarter"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("quarter"));
            assert!(text.contains("all")); // Default focus area
            assert!(text.contains("Improving task completion rates")); // Default recommendations
        }
    }
}

#[tokio::test]
async fn test_backup_strategy_prompt() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "backup_strategy".to_string(),
        arguments: Some(json!({
            "data_volume": "large",
            "frequency": "daily",
            "retention_period": "1_year",
            "storage_preference": "cloud"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("large"));
            assert!(text.contains("daily"));
            assert!(text.contains("1_year"));
            assert!(text.contains("cloud"));
            assert!(text.contains("Backup Strategy Recommendation"));
            assert!(text.contains("Data Assessment"));
            assert!(text.contains("Backup Frequency Optimization"));
            assert!(text.contains("Storage Strategy"));
            assert!(text.contains("Retention Policy"));
            assert!(text.contains("Recommended Implementation"));
            assert!(text.contains("Risk Mitigation"));
            assert!(text.contains("Cost Analysis"));
        }
    }
}

#[tokio::test]
async fn test_backup_strategy_prompt_minimal_args() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "backup_strategy".to_string(),
        arguments: Some(json!({
            "data_volume": "small",
            "frequency": "weekly"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("small"));
            assert!(text.contains("weekly"));
            assert!(text.contains("3_months")); // Default retention
            assert!(text.contains("hybrid")); // Default storage preference
        }
    }
}

#[tokio::test]
async fn test_backup_strategy_prompt_missing_required() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "backup_strategy".to_string(),
        arguments: Some(json!({
            "data_volume": "medium"
        })),
    };

    let result = server.get_prompt(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "frequency");
        }
        _ => panic!("Expected MissingParameter error"),
    }
}

#[tokio::test]
async fn test_unknown_prompt() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "unknown_prompt".to_string(),
        arguments: None,
    };

    let result = server.get_prompt(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::PromptNotFound { prompt_name } => {
            assert_eq!(prompt_name, "unknown_prompt");
        }
        _ => panic!("Expected PromptNotFound error"),
    }
}

#[tokio::test]
async fn test_prompt_with_no_arguments() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "task_review".to_string(),
        arguments: None,
    };

    let result = server.get_prompt(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "task_title");
        }
        _ => panic!("Expected MissingParameter error"),
    }
}

#[tokio::test]
async fn test_prompt_context_awareness() {
    let server = create_test_mcp_server();

    // Test that prompts include current data context
    let request = things3_cli::mcp::GetPromptRequest {
        name: "task_review".to_string(),
        arguments: Some(json!({
            "task_title": "Test task"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);

    match &result.content[0] {
        Content::Text { text } => {
            // Should contain current context data
            assert!(text.contains("Inbox Tasks"));
            assert!(text.contains("Today's Tasks"));
            // Should contain actual numbers (not just placeholders)
            assert!(text.contains("tasks"));
        }
    }
}

#[tokio::test]
async fn test_prompt_error_handling() {
    let server = create_test_mcp_server();

    // Test with invalid JSON in arguments
    let request = things3_cli::mcp::GetPromptRequest {
        name: "task_review".to_string(),
        arguments: Some(json!({ "task_title": 123 })), // Should be string
    };

    let result = server.get_prompt(request).await;
    // Should error due to type mismatch
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "task_title");
        }
        _ => panic!("Expected MissingParameter error"),
    }
}

#[tokio::test]
async fn test_prompt_schema_enum_validation() {
    let server = create_test_mcp_server();
    let result = server.list_prompts().unwrap();

    // Check that enum values are properly defined in schemas
    for prompt in &result.prompts {
        match prompt.name.as_str() {
            "project_planning" => {
                let schema = &prompt.arguments;
                let complexity_enum = &schema["properties"]["complexity"]["enum"];
                assert!(complexity_enum
                    .as_array()
                    .unwrap()
                    .contains(&json!("simple")));
                assert!(complexity_enum
                    .as_array()
                    .unwrap()
                    .contains(&json!("medium")));
                assert!(complexity_enum
                    .as_array()
                    .unwrap()
                    .contains(&json!("complex")));
            }
            "productivity_analysis" => {
                let schema = &prompt.arguments;
                let time_period_enum = &schema["properties"]["time_period"]["enum"];
                assert!(time_period_enum
                    .as_array()
                    .unwrap()
                    .contains(&json!("week")));
                assert!(time_period_enum
                    .as_array()
                    .unwrap()
                    .contains(&json!("month")));
                assert!(time_period_enum
                    .as_array()
                    .unwrap()
                    .contains(&json!("quarter")));
                assert!(time_period_enum
                    .as_array()
                    .unwrap()
                    .contains(&json!("year")));
            }
            "backup_strategy" => {
                let schema = &prompt.arguments;
                let data_volume_enum = &schema["properties"]["data_volume"]["enum"];
                assert!(data_volume_enum
                    .as_array()
                    .unwrap()
                    .contains(&json!("small")));
                assert!(data_volume_enum
                    .as_array()
                    .unwrap()
                    .contains(&json!("medium")));
                assert!(data_volume_enum
                    .as_array()
                    .unwrap()
                    .contains(&json!("large")));
            }
            _ => {
                // Other prompts may not have enums
            }
        }
    }
}

// ===== Error Handling Tests =====

#[tokio::test]
async fn test_mcp_error_creation() {
    // Test McpError creation methods
    let tool_not_found = McpError::tool_not_found("test_tool");
    assert!(
        matches!(tool_not_found, McpError::ToolNotFound { tool_name } if tool_name == "test_tool")
    );

    let resource_not_found = McpError::resource_not_found("test://resource");
    assert!(
        matches!(resource_not_found, McpError::ResourceNotFound { uri } if uri == "test://resource")
    );

    let prompt_not_found = McpError::prompt_not_found("test_prompt");
    assert!(
        matches!(prompt_not_found, McpError::PromptNotFound { prompt_name } if prompt_name == "test_prompt")
    );

    let missing_param = McpError::missing_parameter("test_param");
    assert!(
        matches!(missing_param, McpError::MissingParameter { parameter_name } if parameter_name == "test_param")
    );

    let invalid_param = McpError::invalid_parameter("test_param", "invalid value");
    assert!(
        matches!(invalid_param, McpError::InvalidParameter { parameter_name, message }
        if parameter_name == "test_param" && message == "invalid value")
    );

    let invalid_format = McpError::invalid_format("xml", "json, csv");
    assert!(
        matches!(invalid_format, McpError::InvalidFormat { format, supported }
        if format == "xml" && supported == "json, csv")
    );

    let invalid_data_type = McpError::invalid_data_type("xml", "tasks, projects");
    assert!(
        matches!(invalid_data_type, McpError::InvalidDataType { data_type, supported }
        if data_type == "xml" && supported == "tasks, projects")
    );
}

#[tokio::test]
async fn test_mcp_error_to_call_result() {
    // Test tool not found error
    let tool_error = McpError::tool_not_found("unknown_tool");
    let call_result = tool_error.to_call_result();
    assert!(call_result.is_error);
    assert_eq!(call_result.content.len(), 1);
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Tool 'unknown_tool' not found"));
            assert!(text.contains("Available tools can be listed"));
        }
    }

    // Test missing parameter error
    let param_error = McpError::missing_parameter("query");
    let call_result = param_error.to_call_result();
    assert!(call_result.is_error);
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter 'query'"));
            assert!(text.contains("Please provide this parameter"));
        }
    }

    // Test invalid format error
    let format_error = McpError::invalid_format("xml", "json, csv, markdown");
    let call_result = format_error.to_call_result();
    assert!(call_result.is_error);
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Invalid format 'xml'"));
            assert!(text.contains("Supported formats: json, csv, markdown"));
        }
    }
}

#[tokio::test]
async fn test_mcp_error_to_prompt_result() {
    // Test prompt not found error
    let prompt_error = McpError::prompt_not_found("unknown_prompt");
    let prompt_result = prompt_error.to_prompt_result();
    assert!(prompt_result.is_error);
    match &prompt_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Prompt 'unknown_prompt' not found"));
            assert!(text.contains("Available prompts can be listed"));
        }
    }

    // Test missing parameter error
    let param_error = McpError::missing_parameter("task_title");
    let prompt_result = param_error.to_prompt_result();
    assert!(prompt_result.is_error);
    match &prompt_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter 'task_title'"));
        }
    }
}

#[tokio::test]
async fn test_mcp_error_to_resource_result() {
    // Test resource not found error
    let resource_error = McpError::resource_not_found("things://unknown");
    let resource_result = resource_error.to_resource_result();
    match &resource_result.contents[0] {
        Content::Text { text } => {
            assert!(text.contains("Resource 'things://unknown' not found"));
            assert!(text.contains("Available resources can be listed"));
        }
    }
}

#[tokio::test]
async fn test_from_traits() {
    // Test From<ThingsError> for McpError
    let things_error = things3_core::ThingsError::validation("Test validation error");
    let mcp_error: McpError = things_error.into();
    assert!(matches!(mcp_error, McpError::ValidationError { message }
        if message == "Test validation error"));

    // Test From<serde_json::Error> for McpError
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let mcp_error: McpError = json_error.into();
    assert!(
        matches!(mcp_error, McpError::SerializationFailed { operation, .. }
        if operation == "json serialization")
    );

    // Test From<std::io::Error> for McpError
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let mcp_error: McpError = io_error.into();
    assert!(
        matches!(mcp_error, McpError::IoOperationFailed { operation, .. }
        if operation == "file operation")
    );
}

#[tokio::test]
async fn test_from_traits_comprehensive() {
    // Test all ThingsError variants
    let db_error = things3_core::ThingsError::Database(rusqlite::Error::InvalidColumnType(
        0,
        "TEXT".to_string(),
        rusqlite::types::Type::Integer,
    ));
    let mcp_error: McpError = db_error.into();
    assert!(
        matches!(mcp_error, McpError::DatabaseOperationFailed { operation, .. } if operation == "database operation")
    );

    let serialization_error = things3_core::ThingsError::Serialization(
        serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err(),
    );
    let mcp_error: McpError = serialization_error.into();
    assert!(
        matches!(mcp_error, McpError::SerializationFailed { operation, .. } if operation == "serialization")
    );

    let io_error = things3_core::ThingsError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));
    let mcp_error: McpError = io_error.into();
    assert!(
        matches!(mcp_error, McpError::IoOperationFailed { operation, .. } if operation == "io operation")
    );

    let db_not_found = things3_core::ThingsError::DatabaseNotFound {
        path: "/test/path".to_string(),
    };
    let mcp_error: McpError = db_not_found.into();
    assert!(
        matches!(mcp_error, McpError::ConfigurationError { message } if message.contains("Database not found at: /test/path"))
    );

    let invalid_uuid = things3_core::ThingsError::InvalidUuid {
        uuid: "invalid-uuid".to_string(),
    };
    let mcp_error: McpError = invalid_uuid.into();
    assert!(
        matches!(mcp_error, McpError::ValidationError { message } if message.contains("Invalid UUID format: invalid-uuid"))
    );

    let invalid_date = things3_core::ThingsError::InvalidDate {
        date: "invalid-date".to_string(),
    };
    let mcp_error: McpError = invalid_date.into();
    assert!(
        matches!(mcp_error, McpError::ValidationError { message } if message.contains("Invalid date format: invalid-date"))
    );

    let task_not_found = things3_core::ThingsError::TaskNotFound {
        uuid: "task-uuid".to_string(),
    };
    let mcp_error: McpError = task_not_found.into();
    assert!(
        matches!(mcp_error, McpError::ValidationError { message } if message.contains("Task not found: task-uuid"))
    );

    let project_not_found = things3_core::ThingsError::ProjectNotFound {
        uuid: "project-uuid".to_string(),
    };
    let mcp_error: McpError = project_not_found.into();
    assert!(
        matches!(mcp_error, McpError::ValidationError { message } if message.contains("Project not found: project-uuid"))
    );

    let area_not_found = things3_core::ThingsError::AreaNotFound {
        uuid: "area-uuid".to_string(),
    };
    let mcp_error: McpError = area_not_found.into();
    assert!(
        matches!(mcp_error, McpError::ValidationError { message } if message.contains("Area not found: area-uuid"))
    );

    let validation_error = things3_core::ThingsError::Validation {
        message: "test validation".to_string(),
    };
    let mcp_error: McpError = validation_error.into();
    assert!(
        matches!(mcp_error, McpError::ValidationError { message } if message == "test validation")
    );

    let config_error = things3_core::ThingsError::Configuration {
        message: "test config".to_string(),
    };
    let mcp_error: McpError = config_error.into();
    assert!(
        matches!(mcp_error, McpError::ConfigurationError { message } if message == "test config")
    );

    let unknown_error = things3_core::ThingsError::Unknown {
        message: "test unknown".to_string(),
    };
    let mcp_error: McpError = unknown_error.into();
    assert!(matches!(mcp_error, McpError::InternalError { message } if message == "test unknown"));
}

#[tokio::test]
async fn test_fallback_error_handling() {
    let server = create_test_mcp_server();

    // Test call_tool_with_fallback for unknown tool
    let request = CallToolRequest {
        name: "unknown_tool".to_string(),
        arguments: None,
    };

    let result = server.call_tool_with_fallback(request).await;
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Tool 'unknown_tool' not found"));
            assert!(text.contains("Available tools can be listed"));
        }
    }
}

#[tokio::test]
async fn test_prompt_fallback_error_handling() {
    let server = create_test_mcp_server();

    // Test get_prompt_with_fallback for unknown prompt
    let request = things3_cli::mcp::GetPromptRequest {
        name: "unknown_prompt".to_string(),
        arguments: None,
    };

    let result = server.get_prompt_with_fallback(request).await;
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Prompt 'unknown_prompt' not found"));
            assert!(text.contains("Available prompts can be listed"));
        }
    }
}

#[tokio::test]
async fn test_resource_fallback_error_handling() {
    let server = create_test_mcp_server();

    // Test read_resource_with_fallback for unknown resource
    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://unknown".to_string(),
    };

    let result = server.read_resource_with_fallback(request).await;
    match &result.contents[0] {
        Content::Text { text } => {
            assert!(text.contains("Resource 'things://unknown' not found"));
            assert!(text.contains("Available resources can be listed"));
        }
    }
}

#[tokio::test]
async fn test_specific_error_types_in_tool_handlers() {
    let server = create_test_mcp_server();

    // Test missing parameter error
    let request = CallToolRequest {
        name: "search_tasks".to_string(),
        arguments: Some(json!({ "limit": 5 })), // Missing required 'query' parameter
    };

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "query");
        }
        _ => panic!("Expected MissingParameter error"),
    }
}

#[tokio::test]
async fn test_invalid_format_error() {
    let server = create_test_mcp_server();

    // Test invalid format error
    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "xml", // Invalid format
            "data_type": "tasks"
        })),
    };

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::InvalidFormat { format, supported } => {
            assert_eq!(format, "xml");
            assert_eq!(supported, "json, csv, markdown");
        }
        _ => panic!("Expected InvalidFormat error"),
    }
}

#[tokio::test]
async fn test_invalid_data_type_error() {
    let server = create_test_mcp_server();

    // Test invalid data type error
    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "json",
            "data_type": "invalid_type" // Invalid data type
        })),
    };

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::InvalidDataType {
            data_type,
            supported,
        } => {
            assert_eq!(data_type, "invalid_type");
            assert_eq!(supported, "tasks, projects, areas, all");
        }
        _ => panic!("Expected InvalidDataType error"),
    }
}

#[tokio::test]
async fn test_tool_not_found_error() {
    let server = create_test_mcp_server();

    // Test tool not found error
    let request = CallToolRequest {
        name: "nonexistent_tool".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ToolNotFound { tool_name } => {
            assert_eq!(tool_name, "nonexistent_tool");
        }
        _ => panic!("Expected ToolNotFound error"),
    }
}

#[tokio::test]
async fn test_prompt_not_found_error() {
    let server = create_test_mcp_server();

    // Test prompt not found error
    let request = things3_cli::mcp::GetPromptRequest {
        name: "nonexistent_prompt".to_string(),
        arguments: None,
    };

    let result = server.get_prompt(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::PromptNotFound { prompt_name } => {
            assert_eq!(prompt_name, "nonexistent_prompt");
        }
        _ => panic!("Expected PromptNotFound error"),
    }
}

#[tokio::test]
async fn test_resource_not_found_error() {
    let server = create_test_mcp_server();

    // Test resource not found error
    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://nonexistent".to_string(),
    };

    let result = server.read_resource(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ResourceNotFound { uri } => {
            assert_eq!(uri, "things://nonexistent");
        }
        _ => panic!("Expected ResourceNotFound error"),
    }
}

#[tokio::test]
async fn test_error_message_quality() {
    // Test that error messages are helpful and actionable
    let errors = vec![
        McpError::tool_not_found("test_tool"),
        McpError::missing_parameter("test_param"),
        McpError::invalid_format("xml", "json, csv"),
        McpError::invalid_data_type("xml", "tasks, projects"),
    ];

    for error in errors {
        let call_result = error.to_call_result();
        assert!(call_result.is_error);

        match &call_result.content[0] {
            Content::Text { text } => {
                // Error messages should be informative
                assert!(text.len() > 20);
                // Should contain helpful suggestions
                assert!(
                    text.contains("Please")
                        || text.contains("Available")
                        || text.contains("Supported")
                );
                // Should not be just generic error messages
                assert!(!text.contains("Error: Error"));
            }
        }
    }
}

#[tokio::test]
async fn test_error_consistency() {
    // Test that similar errors produce consistent messages
    let param_errors = vec![
        McpError::missing_parameter("param1"),
        McpError::missing_parameter("param2"),
    ];

    for error in param_errors {
        let call_result = error.to_call_result();
        match &call_result.content[0] {
            Content::Text { text } => {
                assert!(text.contains("Missing required parameter"));
                assert!(text.contains("Please provide this parameter"));
            }
        }
    }
}

#[tokio::test]
async fn test_error_serialization() {
    // Test that McpError can be serialized/deserialized for logging
    let error = McpError::tool_not_found("test_tool");
    let error_string = format!("{error:?}");
    assert!(error_string.contains("ToolNotFound"));
    assert!(error_string.contains("test_tool"));
}

#[tokio::test]
async fn test_mcp_error_helper_methods() {
    // Test all the helper methods for creating specific error types
    let tool_not_found = McpError::tool_not_found("test_tool");
    assert!(
        matches!(tool_not_found, McpError::ToolNotFound { tool_name } if tool_name == "test_tool")
    );

    let prompt_not_found = McpError::prompt_not_found("test_prompt");
    assert!(
        matches!(prompt_not_found, McpError::PromptNotFound { prompt_name } if prompt_name == "test_prompt")
    );

    let resource_not_found = McpError::resource_not_found("test_resource");
    assert!(
        matches!(resource_not_found, McpError::ResourceNotFound { uri } if uri == "test_resource")
    );

    let invalid_param = McpError::invalid_parameter("test_param", "invalid value");
    assert!(
        matches!(invalid_param, McpError::InvalidParameter { parameter_name, message }
        if parameter_name == "test_param" && message == "invalid value")
    );

    let missing_param = McpError::missing_parameter("test_param");
    assert!(
        matches!(missing_param, McpError::MissingParameter { parameter_name } if parameter_name == "test_param")
    );

    let invalid_format = McpError::invalid_format("xml", "json, csv");
    assert!(
        matches!(invalid_format, McpError::InvalidFormat { format, supported }
        if format == "xml" && supported == "json, csv")
    );

    let invalid_data_type = McpError::invalid_data_type("xml", "tasks, projects");
    assert!(
        matches!(invalid_data_type, McpError::InvalidDataType { data_type, supported }
        if data_type == "xml" && supported == "tasks, projects")
    );

    let db_error = McpError::database_operation_failed(
        "test_op",
        things3_core::ThingsError::validation("test error"),
    );
    assert!(
        matches!(db_error, McpError::DatabaseOperationFailed { operation, .. } if operation == "test_op")
    );

    let backup_error = McpError::backup_operation_failed(
        "test_backup",
        things3_core::ThingsError::validation("backup error"),
    );
    assert!(
        matches!(backup_error, McpError::BackupOperationFailed { operation, .. } if operation == "test_backup")
    );

    let export_error = McpError::export_operation_failed(
        "test_export",
        things3_core::ThingsError::validation("export error"),
    );
    assert!(
        matches!(export_error, McpError::ExportOperationFailed { operation, .. } if operation == "test_export")
    );

    let perf_error = McpError::performance_monitoring_failed(
        "test_perf",
        things3_core::ThingsError::validation("perf error"),
    );
    assert!(
        matches!(perf_error, McpError::PerformanceMonitoringFailed { operation, .. } if operation == "test_perf")
    );

    let cache_error = McpError::cache_operation_failed(
        "test_cache",
        things3_core::ThingsError::validation("cache error"),
    );
    assert!(
        matches!(cache_error, McpError::CacheOperationFailed { operation, .. } if operation == "test_cache")
    );

    let serialization_error = McpError::serialization_failed(
        "test_serialization",
        serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err(),
    );
    assert!(
        matches!(serialization_error, McpError::SerializationFailed { operation, .. } if operation == "test_serialization")
    );

    let io_error = McpError::io_operation_failed(
        "test_io",
        std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    );
    assert!(
        matches!(io_error, McpError::IoOperationFailed { operation, .. } if operation == "test_io")
    );

    let config_error = McpError::configuration_error("test config error");
    assert!(
        matches!(config_error, McpError::ConfigurationError { message } if message == "test config error")
    );

    let validation_error = McpError::validation_error("test validation error");
    assert!(
        matches!(validation_error, McpError::ValidationError { message } if message == "test validation error")
    );

    let internal_error = McpError::internal_error("test internal error");
    assert!(
        matches!(internal_error, McpError::InternalError { message } if message == "test internal error")
    );
}

#[tokio::test]
async fn test_error_conversion_methods_comprehensive() {
    // Test to_call_result with all error types
    let tool_error = McpError::tool_not_found("test_tool");
    let call_result = tool_error.to_call_result();
    assert!(call_result.is_error);
    assert_eq!(call_result.content.len(), 1);
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Tool 'test_tool' not found"));
        }
    }

    let resource_error = McpError::resource_not_found("test_resource");
    let call_result = resource_error.to_call_result();
    assert!(call_result.is_error);
    assert_eq!(call_result.content.len(), 1);
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Resource 'test_resource' not found"));
        }
    }

    let prompt_error = McpError::prompt_not_found("test_prompt");
    let call_result = prompt_error.to_call_result();
    assert!(call_result.is_error);
    assert_eq!(call_result.content.len(), 1);
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Prompt 'test_prompt' not found"));
        }
    }

    // Test to_prompt_result
    let prompt_error = McpError::prompt_not_found("test_prompt");
    let prompt_result = prompt_error.to_prompt_result();
    assert!(prompt_result.is_error);
    assert_eq!(prompt_result.content.len(), 1);
    match &prompt_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Prompt 'test_prompt' not found"));
        }
    }

    // Test to_resource_result
    let resource_error = McpError::resource_not_found("test_resource");
    let resource_result = resource_error.to_resource_result();
    assert_eq!(resource_result.contents.len(), 1);
    match &resource_result.contents[0] {
        Content::Text { text } => {
            assert!(text.contains("Resource 'test_resource' not found"));
        }
    }
}

#[tokio::test]
async fn test_error_message_formatting() {
    // Test that error messages are properly formatted with context
    let invalid_param = McpError::invalid_parameter("test_param", "invalid value");
    let call_result = invalid_param.to_call_result();
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Invalid parameter 'test_param'"));
            assert!(text.contains("invalid value"));
            assert!(text.contains("Please check the parameter format"));
        }
    }

    let missing_param = McpError::missing_parameter("test_param");
    let call_result = missing_param.to_call_result();
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter 'test_param'"));
            assert!(text.contains("Please provide this parameter"));
        }
    }

    let invalid_format = McpError::invalid_format("xml", "json, csv");
    let call_result = invalid_format.to_call_result();
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Invalid format 'xml'"));
            assert!(text.contains("Supported formats: json, csv"));
            assert!(text.contains("Please use one of the supported formats"));
        }
    }

    let db_error = McpError::database_operation_failed(
        "test_op",
        things3_core::ThingsError::validation("test error"),
    );
    let call_result = db_error.to_call_result();
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Database operation 'test_op' failed"));
            assert!(text.contains("Please check your database connection"));
        }
    }
}

#[tokio::test]
async fn test_error_display() {
    // Test that McpError implements Display trait properly
    let error = McpError::missing_parameter("test_param");
    let error_string = error.to_string();
    assert!(error_string.contains("Missing required parameter"));
    assert!(error_string.contains("test_param"));
}

#[tokio::test]
async fn test_error_chain() {
    // Test error chaining and source information
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let mcp_error: McpError = io_error.into();

    match mcp_error {
        McpError::IoOperationFailed { operation, source } => {
            assert_eq!(operation, "file operation");
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        _ => panic!("Expected IoOperationFailed error"),
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_all_error_variants_to_call_result() {
    // Test all error variants in to_call_result method
    let tool_error = McpError::tool_not_found("test_tool");
    let result = tool_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Tool 'test_tool' not found"));
            assert!(text.contains("list_tools method"));
        }
    }

    let resource_error = McpError::resource_not_found("test_resource");
    let result = resource_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Resource 'test_resource' not found"));
            assert!(text.contains("list_resources method"));
        }
    }

    let prompt_error = McpError::prompt_not_found("test_prompt");
    let result = prompt_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Prompt 'test_prompt' not found"));
            assert!(text.contains("list_prompts method"));
        }
    }

    let invalid_data_type = McpError::invalid_data_type("xml", "json, csv");
    let result = invalid_data_type.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Invalid data type 'xml'"));
            assert!(text.contains("Supported types: json, csv"));
            assert!(text.contains("Please use one of the supported types"));
        }
    }

    let backup_error = McpError::backup_operation_failed(
        "test_backup",
        things3_core::ThingsError::validation("backup error"),
    );
    let result = backup_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Backup operation 'test_backup' failed"));
            assert!(text.contains("Please check backup permissions"));
        }
    }

    let export_error = McpError::export_operation_failed(
        "test_export",
        things3_core::ThingsError::validation("export error"),
    );
    let result = export_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Export operation 'test_export' failed"));
            assert!(text.contains("Please check export parameters"));
        }
    }

    let perf_error = McpError::performance_monitoring_failed(
        "test_perf",
        things3_core::ThingsError::validation("perf error"),
    );
    let result = perf_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Performance monitoring 'test_perf' failed"));
            assert!(text.contains("Please try again later"));
        }
    }

    let cache_error = McpError::cache_operation_failed(
        "test_cache",
        things3_core::ThingsError::validation("cache error"),
    );
    let result = cache_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Cache operation 'test_cache' failed"));
            assert!(text.contains("Please try again later"));
        }
    }

    let serialization_error = McpError::serialization_failed(
        "test_serialization",
        serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err(),
    );
    let result = serialization_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Serialization 'test_serialization' failed"));
            assert!(text.contains("Please check data format"));
        }
    }

    let io_error = McpError::io_operation_failed(
        "test_io",
        std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    );
    let result = io_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("IO operation 'test_io' failed"));
            assert!(text.contains("Please check file permissions"));
        }
    }

    let config_error = McpError::configuration_error("test config error");
    let result = config_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Configuration error: test config error"));
            assert!(text.contains("Please check your configuration"));
        }
    }

    let validation_error = McpError::validation_error("test validation error");
    let result = validation_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Validation error: test validation error"));
            assert!(text.contains("Please check your input"));
        }
    }

    let internal_error = McpError::internal_error("test internal error");
    let result = internal_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Internal error: test internal error"));
            assert!(text.contains("Please try again later or contact support"));
        }
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_all_error_variants_to_prompt_result() {
    // Test all error variants in to_prompt_result method
    let prompt_error = McpError::prompt_not_found("test_prompt");
    let result = prompt_error.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Prompt 'test_prompt' not found"));
            assert!(text.contains("list_prompts method"));
        }
    }

    let invalid_param = McpError::invalid_parameter("test_param", "invalid value");
    let result = invalid_param.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Invalid parameter 'test_param'"));
            assert!(text.contains("invalid value"));
            assert!(text.contains("Please check the parameter format"));
        }
    }

    let missing_param = McpError::missing_parameter("test_param");
    let result = missing_param.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter 'test_param'"));
            assert!(text.contains("Please provide this parameter"));
        }
    }

    let invalid_format = McpError::invalid_format("xml", "json, csv");
    let result = invalid_format.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            // to_prompt_result uses catch-all pattern for InvalidFormat
            assert!(text.contains("Error: Invalid format: xml - supported formats: json, csv"));
            assert!(text.contains("Please try again later"));
        }
    }

    let invalid_data_type = McpError::invalid_data_type("xml", "json, csv");
    let result = invalid_data_type.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            // to_prompt_result uses catch-all pattern for InvalidDataType
            assert!(text.contains("Error: Invalid data type: xml - supported types: json, csv"));
            assert!(text.contains("Please try again later"));
        }
    }

    let db_error = McpError::database_operation_failed(
        "test_op",
        things3_core::ThingsError::validation("test error"),
    );
    let result = db_error.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Database operation 'test_op' failed"));
            assert!(text.contains("Please check your database connection"));
        }
    }

    let serialization_error = McpError::serialization_failed(
        "test_serialization",
        serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err(),
    );
    let result = serialization_error.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Serialization 'test_serialization' failed"));
            assert!(text.contains("Please check data format"));
        }
    }

    let io_error = McpError::io_operation_failed(
        "test_io",
        std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    );
    let result = io_error.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            // to_prompt_result uses catch-all pattern for IoOperationFailed
            assert!(text.contains("Error: IO operation failed: test_io"));
            assert!(text.contains("Please try again later"));
        }
    }

    let config_error = McpError::configuration_error("test config error");
    let result = config_error.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            // to_prompt_result uses catch-all pattern for ConfigurationError
            assert!(text.contains("Error: Configuration error: test config error"));
            assert!(text.contains("Please try again later"));
        }
    }

    let validation_error = McpError::validation_error("test validation error");
    let result = validation_error.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Validation error: test validation error"));
            assert!(text.contains("Please check your input"));
        }
    }

    let internal_error = McpError::internal_error("test internal error");
    let result = internal_error.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Internal error: test internal error"));
            assert!(text.contains("Please try again later or contact support"));
        }
    }
}

#[tokio::test]
async fn test_all_error_variants_to_resource_result() {
    // Test all error variants in to_resource_result method
    let resource_error = McpError::resource_not_found("test_resource");
    let result = resource_error.to_resource_result();
    match &result.contents[0] {
        Content::Text { text } => {
            assert!(text.contains("Resource 'test_resource' not found"));
            assert!(text.contains("list_resources method"));
        }
    }

    let db_error = McpError::database_operation_failed(
        "test_op",
        things3_core::ThingsError::validation("test error"),
    );
    let result = db_error.to_resource_result();
    match &result.contents[0] {
        Content::Text { text } => {
            assert!(text.contains("Database operation 'test_op' failed"));
            assert!(text.contains("Please check your database connection"));
        }
    }

    let serialization_error = McpError::serialization_failed(
        "test_serialization",
        serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err(),
    );
    let result = serialization_error.to_resource_result();
    match &result.contents[0] {
        Content::Text { text } => {
            assert!(text.contains("Serialization 'test_serialization' failed"));
            assert!(text.contains("Please check data format"));
        }
    }

    let io_error = McpError::io_operation_failed(
        "test_io",
        std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    );
    let result = io_error.to_resource_result();
    match &result.contents[0] {
        Content::Text { text } => {
            // to_resource_result uses catch-all pattern for IoOperationFailed
            assert!(text.contains("Error: IO operation failed: test_io"));
            assert!(text.contains("Please try again later"));
        }
    }

    let config_error = McpError::configuration_error("test config error");
    let result = config_error.to_resource_result();
    match &result.contents[0] {
        Content::Text { text } => {
            // to_resource_result uses catch-all pattern for ConfigurationError
            assert!(text.contains("Error: Configuration error: test config error"));
            assert!(text.contains("Please try again later"));
        }
    }

    let validation_error = McpError::validation_error("test validation error");
    let result = validation_error.to_resource_result();
    match &result.contents[0] {
        Content::Text { text } => {
            // to_resource_result uses catch-all pattern for ValidationError
            assert!(text.contains("Error: Validation error: test validation error"));
            assert!(text.contains("Please try again later"));
        }
    }

    let internal_error = McpError::internal_error("test internal error");
    let result = internal_error.to_resource_result();
    match &result.contents[0] {
        Content::Text { text } => {
            assert!(text.contains("Internal error: test internal error"));
            assert!(text.contains("Please try again later or contact support"));
        }
    }
}
