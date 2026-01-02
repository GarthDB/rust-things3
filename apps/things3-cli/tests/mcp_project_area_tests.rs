//! MCP integration tests for project and area operations

mod mcp_tests;

use mcp_tests::common::{create_test_mcp_server, json, CallToolRequest, Content};
use serde_json::Value;

/// Helper to parse CallToolResult into a Value for easier assertions
fn parse_tool_result(result: &things3_cli::mcp::CallToolResult) -> Value {
    if let Some(Content::Text { text }) = result.content.first() {
        serde_json::from_str(text).unwrap_or(json!({"text": text}))
    } else {
        json!({})
    }
}

#[tokio::test]
async fn test_create_project_tool() {
    let server = create_test_mcp_server().await;

    let request = CallToolRequest {
        name: "create_project".to_string(),
        arguments: Some(json!({
            "title": "Test MCP Project",
            "notes": "Created via MCP"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let response = parse_tool_result(&result);

    assert!(response["message"]
        .as_str()
        .unwrap()
        .contains("created successfully"));
    assert!(response["uuid"].is_string());
}

#[tokio::test]
async fn test_update_project_tool() {
    let server = create_test_mcp_server().await;

    // Create a project first
    let create_request = CallToolRequest {
        name: "create_project".to_string(),
        arguments: Some(json!({
            "title": "Original Project"
        })),
    };
    let create_result = server.call_tool(create_request).await.unwrap();
    let create_response = parse_tool_result(&create_result);
    let uuid = create_response["uuid"].as_str().unwrap();

    // Update it
    let update_request = CallToolRequest {
        name: "update_project".to_string(),
        arguments: Some(json!({
            "uuid": uuid,
            "title": "Updated Project",
            "notes": "New notes"
        })),
    };
    let update_result = server.call_tool(update_request).await.unwrap();
    let update_response = parse_tool_result(&update_result);

    assert!(update_response["message"]
        .as_str()
        .unwrap()
        .contains("updated successfully"));
}

#[tokio::test]
async fn test_complete_project_tool() {
    let server = create_test_mcp_server().await;

    // Create a project
    let create_request = CallToolRequest {
        name: "create_project".to_string(),
        arguments: Some(json!({
            "title": "Project to Complete"
        })),
    };
    let create_result = server.call_tool(create_request).await.unwrap();
    let create_response = parse_tool_result(&create_result);
    let uuid = create_response["uuid"].as_str().unwrap();

    // Complete it
    let complete_request = CallToolRequest {
        name: "complete_project".to_string(),
        arguments: Some(json!({
            "uuid": uuid,
            "child_handling": "error"
        })),
    };
    let complete_result = server.call_tool(complete_request).await.unwrap();
    let complete_response = parse_tool_result(&complete_result);

    assert!(complete_response["message"]
        .as_str()
        .unwrap()
        .contains("completed successfully"));
}

#[tokio::test]
async fn test_delete_project_tool() {
    let server = create_test_mcp_server().await;

    // Create a project
    let create_request = CallToolRequest {
        name: "create_project".to_string(),
        arguments: Some(json!({
            "title": "Project to Delete"
        })),
    };
    let create_result = server.call_tool(create_request).await.unwrap();
    let create_response = parse_tool_result(&create_result);
    let uuid = create_response["uuid"].as_str().unwrap();

    // Delete it
    let delete_request = CallToolRequest {
        name: "delete_project".to_string(),
        arguments: Some(json!({
            "uuid": uuid,
            "child_handling": "error"
        })),
    };
    let delete_result = server.call_tool(delete_request).await.unwrap();
    let delete_response = parse_tool_result(&delete_result);

    assert!(delete_response["message"]
        .as_str()
        .unwrap()
        .contains("deleted successfully"));
}

#[tokio::test]
async fn test_create_area_tool() {
    let server = create_test_mcp_server().await;

    let request = CallToolRequest {
        name: "create_area".to_string(),
        arguments: Some(json!({
            "title": "Test MCP Area"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let response = parse_tool_result(&result);

    assert!(response["message"]
        .as_str()
        .unwrap()
        .contains("created successfully"));
    assert!(response["uuid"].is_string());
}

#[tokio::test]
async fn test_update_area_tool() {
    let server = create_test_mcp_server().await;

    // Create an area first
    let create_request = CallToolRequest {
        name: "create_area".to_string(),
        arguments: Some(json!({
            "title": "Original Area"
        })),
    };
    let create_result = server.call_tool(create_request).await.unwrap();
    let create_response = parse_tool_result(&create_result);
    let uuid = create_response["uuid"].as_str().unwrap();

    // Update it
    let update_request = CallToolRequest {
        name: "update_area".to_string(),
        arguments: Some(json!({
            "uuid": uuid,
            "title": "Updated Area"
        })),
    };
    let update_result = server.call_tool(update_request).await.unwrap();
    let update_response = parse_tool_result(&update_result);

    assert!(update_response["message"]
        .as_str()
        .unwrap()
        .contains("updated successfully"));
}

#[tokio::test]
async fn test_delete_area_tool() {
    let server = create_test_mcp_server().await;

    // Create an area
    let create_request = CallToolRequest {
        name: "create_area".to_string(),
        arguments: Some(json!({
            "title": "Area to Delete"
        })),
    };
    let create_result = server.call_tool(create_request).await.unwrap();
    let create_response = parse_tool_result(&create_result);
    let uuid = create_response["uuid"].as_str().unwrap();

    // Delete it
    let delete_request = CallToolRequest {
        name: "delete_area".to_string(),
        arguments: Some(json!({
            "uuid": uuid
        })),
    };
    let delete_result = server.call_tool(delete_request).await.unwrap();
    let delete_response = parse_tool_result(&delete_result);

    assert!(delete_response["message"]
        .as_str()
        .unwrap()
        .contains("deleted successfully"));
}

#[tokio::test]
async fn test_project_area_integration() {
    let server = create_test_mcp_server().await;

    // Create an area
    let area_request = CallToolRequest {
        name: "create_area".to_string(),
        arguments: Some(json!({
            "title": "Work"
        })),
    };
    let area_result = server.call_tool(area_request).await.unwrap();
    let area_response = parse_tool_result(&area_result);
    let area_uuid = area_response["uuid"].as_str().unwrap();

    // Create a project in that area
    let project_request = CallToolRequest {
        name: "create_project".to_string(),
        arguments: Some(json!({
            "title": "Work Project",
            "area_uuid": area_uuid
        })),
    };
    let project_result = server.call_tool(project_request).await.unwrap();
    let project_response = parse_tool_result(&project_result);
    let project_uuid = project_response["uuid"].as_str().unwrap();

    assert!(project_response["message"]
        .as_str()
        .unwrap()
        .contains("created successfully"));

    // Create a task in the project
    let task_request = CallToolRequest {
        name: "create_task".to_string(),
        arguments: Some(json!({
            "title": "Task in Project",
            "project_uuid": project_uuid
        })),
    };
    let task_result = server.call_tool(task_request).await.unwrap();
    let task_response = parse_tool_result(&task_result);

    assert!(task_response["message"]
        .as_str()
        .unwrap()
        .contains("created successfully"));

    // Complete the project with cascade to complete the task
    let complete_request = CallToolRequest {
        name: "complete_project".to_string(),
        arguments: Some(json!({
            "uuid": project_uuid,
            "child_handling": "cascade"
        })),
    };
    let complete_result = server.call_tool(complete_request).await.unwrap();
    let complete_response = parse_tool_result(&complete_result);

    assert!(complete_response["message"]
        .as_str()
        .unwrap()
        .contains("completed successfully"));
}
