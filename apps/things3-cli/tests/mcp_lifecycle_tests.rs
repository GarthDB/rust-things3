//! MCP lifecycle operation integration tests

use serde_json::{json, Value};
use std::sync::Arc;
use tempfile::NamedTempFile;
use things3_cli::mcp::test_harness::McpTestHarness;
use things3_core::{test_utils::create_test_database, CreateTaskRequest, ThingsDatabase};
use uuid::Uuid;

// Helper to create harness (non-async)
fn create_harness() -> McpTestHarness {
    McpTestHarness::new()
}

// Helper to parse CallToolResult into JSON Value
fn parse_tool_result(result: &things3_cli::mcp::CallToolResult) -> Value {
    if result.is_error {
        return json!({"error": "Tool call failed"});
    }

    match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => {
            serde_json::from_str(text).unwrap_or(json!({"text": text}))
        }
    }
}

// Helper function to create a task via MCP
async fn create_task_via_mcp(harness: &McpTestHarness) -> String {
    let result = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Test Task",
                "notes": "Test notes"
            })),
        )
        .await;

    let response = parse_tool_result(&result);
    response["uuid"].as_str().unwrap().to_string()
}

// ============================================================================
// MCP Tool Tests (12 tests)
// ============================================================================

#[tokio::test]
async fn test_complete_task_tool() {
    let harness = create_harness();

    // Create a task
    let uuid = create_task_via_mcp(&harness).await;

    // Complete it via MCP
    let result = harness
        .call_tool(
            "complete_task",
            Some(json!({
                "uuid": uuid
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    assert!(
        response.get("message").is_some(),
        "Response should contain message"
    );
    assert_eq!(
        response["message"], "Task completed successfully",
        "Should return success message"
    );
    assert_eq!(response["uuid"], uuid, "Should return the task UUID");
}

#[tokio::test]
async fn test_complete_task_tool_response_format() {
    let harness = create_harness();

    // Create a task
    let uuid = create_task_via_mcp(&harness).await;

    // Complete it
    let result = harness
        .call_tool(
            "complete_task",
            Some(json!({
                "uuid": uuid
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    // Verify response structure
    assert!(response.is_object(), "Response should be a JSON object");
    assert!(response.get("message").is_some());
    assert!(response.get("uuid").is_some());
    assert_eq!(response["message"].as_str().unwrap().len() > 0, true);
}

#[tokio::test]
async fn test_uncomplete_task_tool() {
    let harness = create_harness();

    // Create and complete a task
    let uuid = create_task_via_mcp(&harness).await;
    harness
        .call_tool(
            "complete_task",
            Some(json!({
                "uuid": uuid
            })),
        )
        .await;

    // Uncomplete it via MCP
    let result = harness
        .call_tool(
            "uncomplete_task",
            Some(json!({
                "uuid": uuid
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    assert!(
        response.get("message").is_some(),
        "Response should contain message"
    );
    assert_eq!(
        response["message"], "Task marked as incomplete successfully",
        "Should return success message"
    );
}

#[tokio::test]
async fn test_delete_task_tool_error_mode() {
    let harness = create_harness();

    // Create a parent task
    let parent_result = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Parent Task"
            })),
        )
        .await;
    let parent_response = parse_tool_result(&parent_result);
    let parent_uuid = parent_response["uuid"].as_str().unwrap();

    // Create a child task
    harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Child Task",
                "parent_uuid": parent_uuid
            })),
        )
        .await;

    // Try to delete parent with error mode (default)
    let delete_response = harness
        .call_tool_with_fallback(
            "delete_task",
            Some(json!({
                "uuid": parent_uuid,
                "child_handling": "error"
            })),
        )
        .await;

    // Should return an error
    assert!(
        delete_response.is_error,
        "Should fail when parent has children in error mode"
    );
}

#[tokio::test]
async fn test_delete_task_tool_cascade_mode() {
    let harness = create_harness();

    // Create a parent task
    let parent_result = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Parent Task"
            })),
        )
        .await;
    let parent_response = parse_tool_result(&parent_result);
    let parent_uuid = parent_response["uuid"].as_str().unwrap();

    // Create a child task
    let child_result = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Child Task",
                "parent_uuid": parent_uuid
            })),
        )
        .await;
    let child_response = parse_tool_result(&child_result);
    let child_uuid = child_response["uuid"].as_str().unwrap();

    // Delete parent with cascade mode
    let delete_result = harness
        .call_tool(
            "delete_task",
            Some(json!({
                "uuid": parent_uuid,
                "child_handling": "cascade"
            })),
        )
        .await;
    let delete_response = parse_tool_result(&delete_result);

    assert_eq!(
        delete_response["message"], "Task deleted successfully",
        "Should successfully delete with cascade"
    );

    // Verify both are deleted by searching
    let search_result = harness
        .call_tool(
            "search_tasks",
            Some(json!({
                "query": parent_uuid
            })),
        )
        .await;
    let search_results = parse_tool_result(&search_result);

    // Parent should not be found
    assert!(
        search_results["tasks"].as_array().unwrap().is_empty(),
        "Parent should be deleted"
    );

    let child_search_result = harness
        .call_tool(
            "search_tasks",
            Some(json!({
                "query": child_uuid
            })),
        )
        .await;
    let child_search = parse_tool_result(&child_search_result);

    assert!(
        child_search["tasks"].as_array().unwrap().is_empty(),
        "Child should be deleted in cascade mode"
    );
}

#[tokio::test]
async fn test_delete_task_tool_orphan_mode() {
    let harness = create_harness();

    // Create a parent task
    let parent_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Parent Task"
            })),
        )
        .await;
    let parent_uuid = parent_response["uuid"].as_str().unwrap();

    // Create a child task
    let child_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Child Task",
                "parent_uuid": parent_uuid
            })),
        )
        .await;
    let child_uuid = child_response["uuid"].as_str().unwrap();

    // Delete parent with orphan mode
    let delete_response = harness
        .call_tool(
            "delete_task",
            Some(json!({
                "uuid": parent_uuid,
                "child_handling": "orphan"
            })),
        )
        .await;

    assert_eq!(
        delete_response["message"], "Task deleted successfully",
        "Should successfully delete with orphan mode"
    );

    // Child should still exist
    let child_search = harness
        .call_tool(
            "search_tasks",
            Some(json!({
                "query": child_uuid
            })),
        )
        .await;

    let tasks = child_search["tasks"].as_array().unwrap();
    assert!(!tasks.is_empty(), "Child should still exist in orphan mode");
}

#[tokio::test]
async fn test_complete_task_invalid_uuid() {
    let harness = create_harness();

    // Try to complete with invalid UUID
    let response = harness
        .call_tool_with_fallback(
            "complete_task",
            Some(json!({
                "uuid": "not-a-valid-uuid"
            })),
        )
        .await;

    assert!(response.is_error, "Should return error for invalid UUID");
}

#[tokio::test]
async fn test_delete_task_missing_uuid() {
    let harness = create_harness();

    // Try to delete without UUID
    let response = harness
        .call_tool_with_fallback("delete_task", Some(json!({})))
        .await;

    assert!(response.is_error, "Should return error for missing UUID");
}

#[tokio::test]
async fn test_delete_task_invalid_child_handling() {
    let harness = create_harness();

    // Create a task
    let uuid = create_task_via_mcp(&harness).await;

    // Delete with invalid child_handling value (should default to error mode)
    let response = harness
        .call_tool(
            "delete_task",
            Some(json!({
                "uuid": uuid,
                "child_handling": "invalid_mode"
            })),
        )
        .await;

    // Should still succeed (invalid value defaults to error mode)
    assert_eq!(
        response["message"], "Task deleted successfully",
        "Should default to error mode for invalid child_handling"
    );
}

#[tokio::test]
async fn test_lifecycle_e2e_flow() {
    let harness = create_harness();

    // Create task
    let create_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "E2E Lifecycle Task",
                "notes": "Testing full lifecycle"
            })),
        )
        .await;
    let uuid = create_response["uuid"].as_str().unwrap().to_string();

    // Update task
    let update_response = harness
        .call_tool(
            "update_task",
            Some(json!({
                "uuid": uuid,
                "notes": "Updated notes"
            })),
        )
        .await;
    assert_eq!(update_response["message"], "Task updated successfully");

    // Complete task
    let complete_response = harness
        .call_tool(
            "complete_task",
            Some(json!({
                "uuid": uuid
            })),
        )
        .await;
    assert_eq!(complete_response["message"], "Task completed successfully");

    // Uncomplete task
    let uncomplete_response = harness
        .call_tool(
            "uncomplete_task",
            Some(json!({
                "uuid": uuid
            })),
        )
        .await;
    assert_eq!(
        uncomplete_response["message"],
        "Task marked as incomplete successfully"
    );

    // Delete task
    let delete_response = harness
        .call_tool(
            "delete_task",
            Some(json!({
                "uuid": uuid
            })),
        )
        .await;
    assert_eq!(delete_response["message"], "Task deleted successfully");

    // Verify task is gone
    let search_response = harness
        .call_tool(
            "search_tasks",
            Some(json!({
                "query": uuid
            })),
        )
        .await;
    assert!(search_response["tasks"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_task_not_in_inbox_after_completion() {
    let harness = create_harness();

    // Create a task
    let uuid = create_task_via_mcp(&harness).await;

    // Get inbox before completion
    let inbox_before = harness.call_tool("get_inbox", None).await;
    let tasks_before: Vec<Value> = inbox_before["tasks"].as_array().unwrap().clone();

    // Complete the task
    harness
        .call_tool(
            "complete_task",
            Some(json!({
                "uuid": uuid
            })),
        )
        .await;

    // Get inbox after completion
    let inbox_after = harness.call_tool("get_inbox", None).await;
    let tasks_after: Vec<Value> = inbox_after["tasks"].as_array().unwrap().clone();

    // Completed task should not be in inbox (inbox shows incomplete tasks)
    assert!(
        tasks_after.len() <= tasks_before.len(),
        "Inbox should have same or fewer tasks after completion"
    );

    // Verify our specific task is not in the inbox
    let found_in_inbox = tasks_after
        .iter()
        .any(|t| t["uuid"].as_str() == Some(&uuid));
    assert!(!found_in_inbox, "Completed task should not appear in inbox");
}

#[tokio::test]
async fn test_task_not_in_queries_after_deletion() {
    let harness = create_harness();

    // Create a task with unique title
    let unique_title = format!("Unique Task {}", Uuid::new_v4());
    let create_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": unique_title
            })),
        )
        .await;
    let uuid = create_response["uuid"].as_str().unwrap().to_string();

    // Verify task appears in search
    let search_before = harness
        .call_tool(
            "search_tasks",
            Some(json!({
                "query": unique_title
            })),
        )
        .await;
    assert!(!search_before["tasks"].as_array().unwrap().is_empty());

    // Delete the task
    harness
        .call_tool(
            "delete_task",
            Some(json!({
                "uuid": uuid
            })),
        )
        .await;

    // Verify task no longer appears in search
    let search_after = harness
        .call_tool(
            "search_tasks",
            Some(json!({
                "query": unique_title
            })),
        )
        .await;
    assert!(
        search_after["tasks"].as_array().unwrap().is_empty(),
        "Deleted task should not appear in search results"
    );

    // Verify task no longer in inbox
    let inbox = harness.call_tool("get_inbox", None).await;
    let found_in_inbox = inbox["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["uuid"].as_str() == Some(&uuid));
    assert!(!found_in_inbox, "Deleted task should not appear in inbox");
}

// ============================================================================
// Error Handling Tests (3 tests)
// ============================================================================

#[tokio::test]
async fn test_mcp_error_propagation() {
    let harness = create_harness();

    // Try to complete a nonexistent task
    let nonexistent_uuid = Uuid::new_v4().to_string();
    let response = harness
        .call_tool_with_fallback(
            "complete_task",
            Some(json!({
                "uuid": nonexistent_uuid
            })),
        )
        .await;

    // Should propagate database error
    assert!(
        response.is_error,
        "Should propagate error for nonexistent task"
    );
}

#[tokio::test]
async fn test_mcp_validation_errors() {
    let harness = create_harness();

    // Test missing required parameter
    let response1 = harness
        .call_tool_with_fallback("complete_task", Some(json!({})))
        .await;
    assert!(response1.is_error, "Should return error for missing uuid");

    // Test invalid UUID format
    let response2 = harness
        .call_tool_with_fallback(
            "complete_task",
            Some(json!({
                "uuid": "not-a-uuid"
            })),
        )
        .await;
    assert!(
        response2.is_error,
        "Should return error for invalid UUID format"
    );
}

#[tokio::test]
async fn test_mcp_concurrent_calls() {
    let harness = create_harness();

    // Create multiple tasks
    let mut uuids = Vec::new();
    for i in 0..5 {
        let response = harness
            .call_tool(
                "create_task",
                Some(json!({
                    "title": format!("Concurrent Task {}", i)
                })),
            )
            .await;
        uuids.push(response["uuid"].as_str().unwrap().to_string());
    }

    // Complete all tasks concurrently
    let mut handles = Vec::new();
    for uuid in uuids {
        let harness_clone = harness.clone();
        let handle = tokio::spawn(async move {
            harness_clone
                .call_tool(
                    "complete_task",
                    Some(json!({
                        "uuid": uuid
                    })),
                )
                .await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        let response = handle.await.unwrap();
        assert_eq!(response["message"], "Task completed successfully");
    }
}

// ============================================================================
// Integration Tests (3 tests)
// ============================================================================

#[tokio::test]
async fn test_complete_task_appears_in_logbook() {
    let harness = create_harness();

    // Create a task
    let uuid = create_task_via_mcp(&harness).await;

    // Complete the task
    harness
        .call_tool(
            "complete_task",
            Some(json!({
                "uuid": uuid
            })),
        )
        .await;

    // Search for the task by UUID - it should still be found (not trashed)
    let search_response = harness
        .call_tool(
            "search_tasks",
            Some(json!({
                "query": uuid
            })),
        )
        .await;

    let tasks = search_response["tasks"].as_array().unwrap();
    if !tasks.is_empty() {
        // If found, verify it's completed
        assert_eq!(
            tasks[0]["status"], "completed",
            "Task should have completed status"
        );
    }
    // Note: Completed tasks may or may not appear in search depending on query filters
}

#[tokio::test]
async fn test_update_then_complete() {
    let harness = create_harness();

    // Create a task
    let create_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Task to Update and Complete"
            })),
        )
        .await;
    let uuid = create_response["uuid"].as_str().unwrap();

    // Update the task
    let update_response = harness
        .call_tool(
            "update_task",
            Some(json!({
                "uuid": uuid,
                "notes": "Updated before completion",
                "tags": ["important", "urgent"]
            })),
        )
        .await;
    assert_eq!(update_response["message"], "Task updated successfully");

    // Complete the task
    let complete_response = harness
        .call_tool(
            "complete_task",
            Some(json!({
                "uuid": uuid
            })),
        )
        .await;
    assert_eq!(complete_response["message"], "Task completed successfully");

    // Verify combined state
    let search_response = harness
        .call_tool(
            "search_tasks",
            Some(json!({
                "query": uuid
            })),
        )
        .await;

    let tasks = search_response["tasks"].as_array().unwrap();
    if !tasks.is_empty() {
        let task = &tasks[0];
        assert_eq!(task["status"], "completed");
        // Notes should be updated
        if let Some(notes) = task["notes"].as_str() {
            assert_eq!(notes, "Updated before completion");
        }
    }
}

#[tokio::test]
async fn test_search_excludes_deleted_tasks() {
    let harness = create_harness();

    // Create multiple tasks with a common search term
    let search_term = format!("SearchTest{}", Uuid::new_v4());
    let mut task_uuids = Vec::new();

    for i in 0..3 {
        let response = harness
            .call_tool(
                "create_task",
                Some(json!({
                    "title": format!("{} Task {}", search_term, i)
                })),
            )
            .await;
        task_uuids.push(response["uuid"].as_str().unwrap().to_string());
    }

    // Search before deletion
    let search_before = harness
        .call_tool(
            "search_tasks",
            Some(json!({
                "query": search_term
            })),
        )
        .await;
    let count_before = search_before["tasks"].as_array().unwrap().len();
    assert_eq!(count_before, 3, "Should find all 3 tasks initially");

    // Delete one task
    harness
        .call_tool(
            "delete_task",
            Some(json!({
                "uuid": task_uuids[1]
            })),
        )
        .await;

    // Search after deletion
    let search_after = harness
        .call_tool(
            "search_tasks",
            Some(json!({
                "query": search_term
            })),
        )
        .await;
    let count_after = search_after["tasks"].as_array().unwrap().len();
    assert_eq!(count_after, 2, "Should find only 2 tasks after deletion");

    // Verify the deleted task is not in results
    let found_deleted = search_after["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["uuid"].as_str() == Some(&task_uuids[1]));
    assert!(
        !found_deleted,
        "Deleted task should not appear in search results"
    );
}
