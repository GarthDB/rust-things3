//! MCP integration tests for date handling validation
//!
//! Tests that date validation errors are properly exposed through the MCP server interface

use serde_json::{json, Value};
use things3_cli::mcp::test_harness::McpTestHarness;

// Helper to create harness
fn create_harness() -> McpTestHarness {
    McpTestHarness::new()
}

// Helper to parse CallToolResult into JSON Value
fn parse_tool_result(result: &things3_cli::mcp::CallToolResult) -> Value {
    if result.is_error {
        match &result.content[0] {
            things3_cli::mcp::Content::Text { text } => {
                serde_json::from_str(text).unwrap_or(json!({"error": text}))
            }
        }
    } else {
        match &result.content[0] {
            things3_cli::mcp::Content::Text { text } => {
                serde_json::from_str(text).unwrap_or(json!({"text": text}))
            }
        }
    }
}

#[tokio::test]
async fn test_create_task_mcp_date_validation_error() {
    let harness = create_harness();

    // Try to create a task with deadline before start date
    let result = harness
        .call_tool_with_fallback(
            "create_task",
            Some(json!({
                "title": "Invalid dates task",
                "start_date": "2024-12-31",
                "deadline": "2024-01-01"
            })),
        )
        .await;

    // Should return an error result
    assert!(result.is_error, "Expected error result for invalid dates");

    let response = parse_tool_result(&result);
    let error_msg = response.to_string().to_lowercase();

    // Error message should mention date validation
    assert!(
        error_msg.contains("deadline") || error_msg.contains("date"),
        "Error message should mention date validation issue: {error_msg}"
    );
}

#[tokio::test]
async fn test_update_task_mcp_date_validation_error() {
    let harness = create_harness();

    // First create a valid task
    let create_result = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Test task"
            })),
        )
        .await;

    assert!(!create_result.is_error, "Task creation should succeed");

    let create_response = parse_tool_result(&create_result);
    let task_uuid = create_response["uuid"]
        .as_str()
        .expect("Should have UUID in response");

    // Try to update with invalid dates
    let result = harness
        .call_tool_with_fallback(
            "update_task",
            Some(json!({
                "uuid": task_uuid,
                "start_date": "2024-12-31",
                "deadline": "2024-01-01"
            })),
        )
        .await;

    // Should return an error result
    assert!(result.is_error, "Expected error result for invalid dates");

    let response = parse_tool_result(&result);
    let error_msg = response.to_string().to_lowercase();

    // Error message should mention date validation
    assert!(
        error_msg.contains("deadline") || error_msg.contains("date"),
        "Error message should mention date validation issue: {error_msg}"
    );
}

#[tokio::test]
async fn test_create_project_mcp_date_validation() {
    let harness = create_harness();

    // Try to create a project with deadline before start date
    let result = harness
        .call_tool_with_fallback(
            "create_project",
            Some(json!({
                "title": "Invalid dates project",
                "start_date": "2024-12-31",
                "deadline": "2024-01-01"
            })),
        )
        .await;

    // Should return an error result
    assert!(result.is_error, "Expected error result for invalid dates");

    let response = parse_tool_result(&result);
    let error_msg = response.to_string().to_lowercase();

    // Error message should mention date validation
    assert!(
        error_msg.contains("deadline") || error_msg.contains("date"),
        "Error message should mention date validation issue: {error_msg}"
    );
}

#[tokio::test]
async fn test_date_error_messages_are_clear() {
    let harness = create_harness();

    // Create task with invalid dates
    let result = harness
        .call_tool_with_fallback(
            "create_task",
            Some(json!({
                "title": "Test task",
                "start_date": "2024-06-15",
                "deadline": "2024-01-01"  // Before start date
            })),
        )
        .await;

    assert!(result.is_error, "Expected error for invalid dates");

    let response = parse_tool_result(&result);
    let error_msg = response.to_string();

    // Error message should be descriptive
    assert!(
        error_msg.contains("2024-06-15") || error_msg.contains("2024-01-01"),
        "Error message should include the actual dates: {error_msg}"
    );
    assert!(
        error_msg.contains("before") || error_msg.contains("after"),
        "Error message should explain the relationship: {error_msg}"
    );
}

#[tokio::test]
async fn test_create_task_with_valid_dates_mcp() {
    let harness = create_harness();

    // Create task with valid dates
    let result = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Valid dates task",
                "start_date": "2024-01-01",
                "deadline": "2024-12-31"
            })),
        )
        .await;

    // Should succeed
    assert!(
        !result.is_error,
        "Task creation with valid dates should succeed"
    );

    let response = parse_tool_result(&result);
    assert!(
        response["uuid"].is_string(),
        "Should return task UUID on success"
    );
}

#[tokio::test]
async fn test_update_task_dates_mcp() {
    let harness = create_harness();

    // First create a task without dates
    let create_result = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Test task"
            })),
        )
        .await;

    assert!(!create_result.is_error, "Task creation should succeed");

    let create_response = parse_tool_result(&create_result);
    let task_uuid = create_response["uuid"]
        .as_str()
        .expect("Should have UUID in response");

    // Update with valid dates
    let result = harness
        .call_tool(
            "update_task",
            Some(json!({
                "uuid": task_uuid,
                "start_date": "2024-01-01",
                "deadline": "2024-12-31"
            })),
        )
        .await;

    // Should succeed
    assert!(
        !result.is_error,
        "Task update with valid dates should succeed"
    );
}

#[tokio::test]
async fn test_same_date_for_start_and_deadline_is_valid() {
    let harness = create_harness();

    // Create task with same start date and deadline (edge case but valid)
    let result = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Same date task",
                "start_date": "2024-06-15",
                "deadline": "2024-06-15"
            })),
        )
        .await;

    // Should succeed (deadline on same day as start is valid)
    assert!(
        !result.is_error,
        "Task creation with same start and deadline date should succeed"
    );
}

#[tokio::test]
async fn test_only_deadline_no_start_date_is_valid() {
    let harness = create_harness();

    // Create task with only deadline (no start date)
    let result = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Only deadline task",
                "deadline": "2024-12-31"
            })),
        )
        .await;

    // Should succeed
    assert!(
        !result.is_error,
        "Task creation with only deadline should succeed"
    );
}

#[tokio::test]
async fn test_only_start_date_no_deadline_is_valid() {
    let harness = create_harness();

    // Create task with only start date (no deadline)
    let result = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Only start date task",
                "start_date": "2024-01-01"
            })),
        )
        .await;

    // Should succeed
    assert!(
        !result.is_error,
        "Task creation with only start date should succeed"
    );
}

#[tokio::test]
async fn test_update_only_deadline_validates_with_existing_start() {
    let harness = create_harness();

    // Create task with start date
    let create_result = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Test task",
                "start_date": "2024-06-15"
            })),
        )
        .await;

    assert!(!create_result.is_error, "Task creation should succeed");

    let create_response = parse_tool_result(&create_result);
    let task_uuid = create_response["uuid"]
        .as_str()
        .expect("Should have UUID in response");

    // Try to update deadline to before existing start date
    let result = harness
        .call_tool_with_fallback(
            "update_task",
            Some(json!({
                "uuid": task_uuid,
                "deadline": "2024-01-01"  // Before existing start date (2024-06-15)
            })),
        )
        .await;

    // Should fail validation
    assert!(
        result.is_error,
        "Updating deadline to before existing start date should fail"
    );
}

#[tokio::test]
async fn test_update_only_start_date_validates_with_existing_deadline() {
    let harness = create_harness();

    // Create task with deadline
    let create_result = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Test task",
                "deadline": "2024-06-15"
            })),
        )
        .await;

    assert!(!create_result.is_error, "Task creation should succeed");

    let create_response = parse_tool_result(&create_result);
    let task_uuid = create_response["uuid"]
        .as_str()
        .expect("Should have UUID in response");

    // Try to update start date to after existing deadline
    let result = harness
        .call_tool_with_fallback(
            "update_task",
            Some(json!({
                "uuid": task_uuid,
                "start_date": "2024-12-31"  // After existing deadline (2024-06-15)
            })),
        )
        .await;

    // Should fail validation
    assert!(
        result.is_error,
        "Updating start date to after existing deadline should fail"
    );
}
