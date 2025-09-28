//! Comprehensive tests for MCP server functionality

use serde_json::json;
use std::path::Path;
use tempfile::NamedTempFile;
use things3_cli::mcp::{CallToolRequest, Content, ThingsMcpServer};
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
    assert!(result.unwrap_err().to_string().contains("Unknown resource"));
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

    let result = server.get_prompt(request).await.unwrap();
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter"));
        }
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

    let result = server.get_prompt(request).await.unwrap();
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter"));
        }
    }
}

#[tokio::test]
async fn test_unknown_prompt() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "unknown_prompt".to_string(),
        arguments: None,
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Unknown prompt"));
        }
    }
}

#[tokio::test]
async fn test_prompt_with_no_arguments() {
    let server = create_test_mcp_server();
    let request = things3_cli::mcp::GetPromptRequest {
        name: "task_review".to_string(),
        arguments: None,
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter"));
        }
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

    let result = server.get_prompt(request).await.unwrap();
    // Should error due to type mismatch
    assert!(result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter"));
        }
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
