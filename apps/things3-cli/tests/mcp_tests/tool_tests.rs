//! Tool-related tests for MCP server

#![cfg(feature = "mcp-server")]

use crate::mcp_tests::common::create_test_mcp_server;
use serde_json::json;
use tempfile::NamedTempFile;
use things3_cli::mcp::{CallToolRequest, Content, McpError, ThingsMcpServer};
use things3_core::{config::ThingsConfig, database::ThingsDatabase};

#[tokio::test]
async fn test_mcp_server_creation() {
    let _server = create_test_mcp_server().await;
    // Server should be created successfully - if we get here, creation succeeded
}

#[tokio::test]
async fn test_list_tools() {
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
    let request = CallToolRequest {
        name: "create_task".to_string(),
        arguments: Some(json!({
            "title": "Test Task",
            "notes": "Test notes"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed.get("uuid").is_some(), "Response should contain UUID");
            assert_eq!(parsed["message"], "Task created successfully");
        }
    }
}

#[tokio::test]
async fn test_create_task_tool_missing_title() {
    let server = create_test_mcp_server().await;
    let request = CallToolRequest {
        name: "create_task".to_string(),
        arguments: Some(json!({
            "notes": "Test notes"
        })),
    };

    let result = server.call_tool(request).await;
    // Missing title should cause a deserialization error
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_task_tool() {
    let server = create_test_mcp_server().await;

    // First create a task to update
    let create_request = CallToolRequest {
        name: "create_task".to_string(),
        arguments: Some(json!({
            "title": "Task to Update"
        })),
    };
    let create_result = server.call_tool(create_request).await.unwrap();
    let create_text = match &create_result.content[0] {
        Content::Text { text } => text,
    };
    let created: serde_json::Value = serde_json::from_str(create_text).unwrap();
    let uuid = created["uuid"].as_str().unwrap();

    // Now update it
    let request = CallToolRequest {
        name: "update_task".to_string(),
        arguments: Some(json!({
            "uuid": uuid,
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
            assert_eq!(parsed["message"], "Task updated successfully");
        }
    }
}

#[tokio::test]
async fn test_update_task_tool_missing_uuid() {
    let server = create_test_mcp_server().await;
    let request = CallToolRequest {
        name: "update_task".to_string(),
        arguments: Some(json!({
            "title": "Updated Task"
        })),
    };

    let result = server.call_tool(request).await;
    // Missing uuid should cause a deserialization error
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_productivity_metrics_tool() {
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
async fn test_export_data_csv_tasks() {
    let server = create_test_mcp_server().await;
    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "csv",
            "data_type": "tasks"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(
                text.contains("Type,Title,Status"),
                "expected task CSV header, got: {text}"
            );
            assert!(text.contains("Inbox Task"), "expected inbox task row");
        }
    }
}

#[tokio::test]
async fn test_export_data_csv_projects() {
    let server = create_test_mcp_server().await;
    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "csv",
            "data_type": "projects"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(
                text.contains("Title,Status,Notes,Start Date,Deadline,Created,Modified,Area"),
                "expected full project CSV header, got: {text}"
            );
            assert!(text.contains("Website Redesign"), "expected project row");
        }
    }
}

#[tokio::test]
async fn test_export_data_csv_areas() {
    let server = create_test_mcp_server().await;
    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "csv",
            "data_type": "areas"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(
                text.contains("Title,Notes,Created,Modified"),
                "expected full area CSV header, got: {text}"
            );
            assert!(text.contains("Work"), "expected area row");
        }
    }
}

#[tokio::test]
async fn test_export_data_csv_all_rejected() {
    let server = create_test_mcp_server().await;
    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "csv",
            "data_type": "all"
        })),
    };

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::InvalidParameter {
            parameter_name,
            message,
        } => {
            assert_eq!(parameter_name, "data_type");
            assert!(
                message.contains("tasks")
                    && message.contains("projects")
                    && message.contains("areas"),
                "error should name the valid alternatives, got: {message}"
            );
        }
        e => panic!("Expected InvalidParameter error, got: {e:?}"),
    }
}

#[tokio::test]
async fn test_export_data_markdown_tasks() {
    let server = create_test_mcp_server().await;
    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "markdown",
            "data_type": "tasks"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(
                text.starts_with("# Things 3 Export"),
                "expected markdown heading, got: {text}"
            );
            assert!(text.contains("## Tasks"), "expected Tasks section");
            assert!(text.contains("Inbox Task"), "expected inbox task");
        }
    }
}

#[tokio::test]
async fn test_export_data_markdown_all() {
    let server = create_test_mcp_server().await;
    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "markdown",
            "data_type": "all"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("## Areas"), "expected Areas section");
            assert!(text.contains("## Projects"), "expected Projects section");
            assert!(text.contains("## Tasks"), "expected Tasks section");
        }
    }
}

#[tokio::test]
async fn test_export_data_output_path_writes_file() {
    let server = create_test_mcp_server().await;
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();

    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "json",
            "data_type": "tasks",
            "output_path": path
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["path"].as_str().unwrap(), path);
            assert_eq!(parsed["format"].as_str().unwrap(), "json");
            assert_eq!(parsed["data_type"].as_str().unwrap(), "tasks");
            assert!(parsed["bytes_written"].as_u64().unwrap() > 0);
            assert!(parsed["counts"]["inbox"].is_number());
            assert!(parsed["counts"]["today"].is_number());

            // Verify the file was actually written with valid JSON
            let file_content = std::fs::read_to_string(&path).unwrap();
            let file_json: serde_json::Value = serde_json::from_str(&file_content).unwrap();
            assert!(file_json["inbox"].is_array());
        }
    }
}

#[tokio::test]
async fn test_export_data_output_path_csv() {
    let server = create_test_mcp_server().await;
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();

    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "csv",
            "data_type": "tasks",
            "output_path": path
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);

    match &result.content[0] {
        Content::Text { text } => {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["format"].as_str().unwrap(), "csv");
            assert_eq!(parsed["data_type"].as_str().unwrap(), "tasks");
            assert!(parsed["bytes_written"].as_u64().unwrap() > 0);

            // Verify the file content is CSV (not JSON)
            let file_content = std::fs::read_to_string(&path).unwrap();
            assert!(
                file_content.contains("Type,Title,Status"),
                "expected CSV header in file, got: {file_content}"
            );
        }
    }
}

#[tokio::test]
async fn test_bulk_create_tasks_tool() {
    let server = create_test_mcp_server().await;
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
            assert_eq!(parsed["success"], true);
            assert_eq!(parsed["processed_count"], 2);
        }
    }
}

#[tokio::test]
async fn test_bulk_create_tasks_tool_missing_tasks() {
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let mut server = create_test_mcp_server().await;
    // Bypass the "is Things 3 running" gate (#126); we want to exercise the
    // BackupManager path, not the safety precondition.
    server.set_process_check_for_test(|| false);
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;
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
    let server = create_test_mcp_server().await;

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
    let server = create_test_mcp_server().await;

    // Test with empty arguments object
    let request = CallToolRequest {
        name: "get_inbox".to_string(),
        arguments: Some(json!({})),
    };

    let result = server.call_tool(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
}

#[tokio::test]
async fn test_fallback_error_handling() {
    let server = create_test_mcp_server().await;

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
async fn test_mcp_server_with_custom_middleware() {
    use things3_cli::mcp::middleware::MiddlewareConfig;

    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    things3_core::test_utils::create_test_database(db_path)
        .await
        .unwrap();

    let db = ThingsDatabase::new(db_path).await.unwrap();
    let config = ThingsConfig::new(db_path, false);
    let middleware_config = MiddlewareConfig::default();

    let server = ThingsMcpServer::with_middleware_config(db, config, middleware_config, true);
    assert!(!server.middleware_chain().is_empty());
}

#[tokio::test]
async fn test_call_tool_with_fallback() {
    let server = create_test_mcp_server().await;

    // Test with a valid tool
    let request = CallToolRequest {
        name: "get_inbox".to_string(),
        arguments: Some(json!({"limit": 5})),
    };

    let result = server.call_tool_with_fallback(request).await;
    assert!(!result.is_error);
    assert!(!result.content.is_empty());

    // Test with an invalid tool
    let request = CallToolRequest {
        name: "nonexistent_tool".to_string(),
        arguments: None,
    };

    let result = server.call_tool_with_fallback(request).await;
    assert!(result.is_error);
}
