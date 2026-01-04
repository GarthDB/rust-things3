//! MCP integration tests for bulk operations

use serde_json::json;
use tempfile::NamedTempFile;
use things3_cli::mcp::{CallToolRequest, ThingsMcpServer};
use things3_core::{test_utils::create_test_database, ThingsConfig, ThingsDatabase};

// Test harness for MCP server
struct McpTestHarness {
    server: ThingsMcpServer,
    _temp_file: NamedTempFile,
}

impl McpTestHarness {
    async fn new() -> Self {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).await.unwrap();
        let db = ThingsDatabase::new(db_path).await.unwrap();
        let config = ThingsConfig::default();
        let server = ThingsMcpServer::new(std::sync::Arc::new(db), config);

        Self {
            server,
            _temp_file: temp_file,
        }
    }

    async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> serde_json::Value {
        let request = CallToolRequest {
            name: name.to_string(),
            arguments: Some(arguments),
        };

        let result = self.server.call_tool_with_fallback(request).await;

        // Extract text content
        if let Some(content) = result.content.first() {
            if let things3_cli::mcp::Content::Text { text } = content {
                return serde_json::from_str(text).expect("Failed to parse response JSON");
            }
        }
        panic!("No content in response");
    }
}

#[tokio::test]
async fn test_bulk_move_mcp() {
    let harness = McpTestHarness::new().await;

    // Create a project
    let project_args = json!({
        "title": "Target Project"
    });
    let project_response = harness.call_tool("create_project", project_args).await;
    let project_uuid = project_response["uuid"].as_str().unwrap();

    // Create 3 tasks
    let mut task_uuids = Vec::new();
    for i in 1..=3 {
        let task_args = json!({
            "title": format!("Task {}", i)
        });
        let task_response = harness.call_tool("create_task", task_args).await;
        task_uuids.push(task_response["uuid"].as_str().unwrap().to_string());
    }

    // Bulk move to project
    let bulk_args = json!({
        "task_uuids": task_uuids,
        "project_uuid": project_uuid
    });
    let response = harness.call_tool("bulk_move", bulk_args).await;

    assert!(response["success"].as_bool().unwrap());
    assert_eq!(response["processed_count"].as_u64().unwrap(), 3);
}

#[tokio::test]
async fn test_bulk_update_dates_mcp() {
    let harness = McpTestHarness::new().await;

    // Create 3 tasks
    let mut task_uuids = Vec::new();
    for i in 1..=3 {
        let task_args = json!({
            "title": format!("Task {}", i)
        });
        let task_response = harness.call_tool("create_task", task_args).await;
        task_uuids.push(task_response["uuid"].as_str().unwrap().to_string());
    }

    // Bulk update dates
    let bulk_args = json!({
        "task_uuids": task_uuids,
        "start_date": "2024-01-01",
        "deadline": "2024-12-31"
    });
    let response = harness.call_tool("bulk_update_dates", bulk_args).await;

    assert!(response["success"].as_bool().unwrap());
    assert_eq!(response["processed_count"].as_u64().unwrap(), 3);
}

#[tokio::test]
async fn test_bulk_complete_mcp() {
    let harness = McpTestHarness::new().await;

    // Create 5 tasks
    let mut task_uuids = Vec::new();
    for i in 1..=5 {
        let task_args = json!({
            "title": format!("Task {}", i)
        });
        let task_response = harness.call_tool("create_task", task_args).await;
        task_uuids.push(task_response["uuid"].as_str().unwrap().to_string());
    }

    // Bulk complete
    let bulk_args = json!({
        "task_uuids": task_uuids
    });
    let response = harness.call_tool("bulk_complete", bulk_args).await;

    assert!(response["success"].as_bool().unwrap());
    assert_eq!(response["processed_count"].as_u64().unwrap(), 5);
}

#[tokio::test]
async fn test_bulk_delete_mcp() {
    let harness = McpTestHarness::new().await;

    // Create 3 tasks
    let mut task_uuids = Vec::new();
    for i in 1..=3 {
        let task_args = json!({
            "title": format!("Task {}", i)
        });
        let task_response = harness.call_tool("create_task", task_args).await;
        task_uuids.push(task_response["uuid"].as_str().unwrap().to_string());
    }

    // Bulk delete
    let bulk_args = json!({
        "task_uuids": task_uuids
    });
    let response = harness.call_tool("bulk_delete", bulk_args).await;

    assert!(response["success"].as_bool().unwrap());
    assert_eq!(response["processed_count"].as_u64().unwrap(), 3);
}

#[tokio::test]
async fn test_bulk_operation_error_messages() {
    let harness = McpTestHarness::new().await;

    // Try to complete tasks with invalid UUID
    let bulk_args = json!({
        "task_uuids": ["00000000-0000-0000-0000-000000000000"]
    });
    let request = CallToolRequest {
        name: "bulk_complete".to_string(),
        arguments: Some(bulk_args),
    };

    let result = harness.server.call_tool_with_fallback(request).await;

    // Should return error in content
    if let Some(content) = result.content.first() {
        if let things3_cli::mcp::Content::Text { text } = content {
            assert!(
                text.contains("error")
                    || text.contains("not found")
                    || text.contains("TaskNotFound")
            );
        }
    }
}

#[tokio::test]
async fn test_bulk_mixed_valid_invalid_uuids() {
    let harness = McpTestHarness::new().await;

    // Create one valid task
    let task_args = json!({
        "title": "Valid Task"
    });
    let task_response = harness.call_tool("create_task", task_args).await;
    let valid_uuid = task_response["uuid"].as_str().unwrap();

    // Try to move with one valid and one invalid UUID
    let bulk_args = json!({
        "task_uuids": [valid_uuid, "00000000-0000-0000-0000-000000000000"],
        "project_uuid": "00000000-0000-0000-0000-111111111111"
    });
    let request = CallToolRequest {
        name: "bulk_move".to_string(),
        arguments: Some(bulk_args),
    };

    let result = harness.server.call_tool_with_fallback(request).await;

    // Should fail with error (transaction should rollback)
    if let Some(content) = result.content.first() {
        if let things3_cli::mcp::Content::Text { text } = content {
            // Should contain error message
            assert!(
                text.contains("error") || text.contains("not found") || text.contains("NotFound")
            );
        }
    }
}

#[tokio::test]
async fn test_bulk_operations_empty_arrays() {
    let harness = McpTestHarness::new().await;

    // Try bulk complete with empty array
    let bulk_args = json!({
        "task_uuids": []
    });
    let request = CallToolRequest {
        name: "bulk_complete".to_string(),
        arguments: Some(bulk_args),
    };

    let result = harness.server.call_tool_with_fallback(request).await;

    // Should return error
    if let Some(content) = result.content.first() {
        if let things3_cli::mcp::Content::Text { text } = content {
            assert!(
                text.contains("error")
                    || text.contains("empty")
                    || text.contains("cannot be empty")
            );
        }
    }
}

#[tokio::test]
async fn test_bulk_operations_large_batch() {
    let harness = McpTestHarness::new().await;

    // Create 50 tasks
    let mut task_uuids = Vec::new();
    for i in 1..=50 {
        let task_args = json!({
            "title": format!("Task {}", i)
        });
        let task_response = harness.call_tool("create_task", task_args).await;
        task_uuids.push(task_response["uuid"].as_str().unwrap().to_string());
    }

    // Bulk complete all 50
    let bulk_args = json!({
        "task_uuids": task_uuids
    });
    let response = harness.call_tool("bulk_complete", bulk_args).await;

    assert!(response["success"].as_bool().unwrap());
    assert_eq!(response["processed_count"].as_u64().unwrap(), 50);
}
