//! MCP protocol edge case tests
//!
//! Tests various edge cases and error conditions in JSON-RPC protocol handling
//! to ensure robust error handling and clear error messages.

use serde_json::json;
use things3_cli::mcp::test_harness::{McpTestHarness, McpTestUtils};

/// Test that calling tools with missing arguments is handled gracefully
#[tokio::test]
async fn test_tool_call_missing_arguments() {
    let harness = McpTestHarness::new();

    // Try to call a tool with None arguments (should use defaults or error gracefully)
    let result = harness.call_tool("get_inbox", None).await;

    // Should either succeed with defaults or return a clear error
    assert!(
        !result.is_error
            || result.content.iter().any(|c| {
                if let things3_cli::mcp::Content::Text { text } = c {
                    !text.is_empty()
                } else {
                    false
                }
            }),
        "Should handle missing arguments gracefully"
    );
}

/// Test calling a tool with invalid arguments type
#[tokio::test]
async fn test_tool_call_invalid_arguments_type() {
    let harness = McpTestHarness::new();

    // Try to call with invalid argument types
    let invalid_args = json!({
        "limit": "not_a_number"  // Should be a number
    });

    let result = harness.call_tool("get_inbox", Some(invalid_args)).await;

    // Should handle type mismatch gracefully
    assert!(
        !result.content.is_empty(),
        "Should return some content even with invalid args"
    );
}

/// Test calling a non-existent tool
#[tokio::test]
async fn test_nonexistent_tool() {
    let harness = McpTestHarness::new();

    // Use fallback method which handles errors gracefully
    let result = harness
        .call_tool_with_fallback("nonexistent_tool", None)
        .await;

    assert!(result.is_error, "Should error for non-existent tool");
    assert!(!result.content.is_empty(), "Should provide error message");
}

/// Test tool with extremely large limit parameter
#[tokio::test]
async fn test_tool_with_extreme_limit() {
    let harness = McpTestHarness::new();

    // Try with very large limit
    let args = json!({"limit": 999999});
    let result = harness.call_tool("get_inbox", Some(args)).await;

    // Should handle extreme values gracefully (may cap at max or return error)
    assert!(!result.content.is_empty(), "Should return content or error");
}

/// Test tool with negative limit parameter
#[tokio::test]
async fn test_tool_with_negative_limit() {
    let harness = McpTestHarness::new();

    let args = json!({"limit": -10});
    let result = harness.call_tool("get_inbox", Some(args)).await;

    // Should handle negative values gracefully
    assert!(
        !result.content.is_empty(),
        "Should return content or error for negative limit"
    );
}

/// Test tool with zero limit
#[tokio::test]
async fn test_tool_with_zero_limit() {
    let harness = McpTestHarness::new();

    let args = json!({"limit": 0});
    let result = harness.call_tool("get_inbox", Some(args)).await;

    // Should handle zero limit gracefully
    assert!(
        !result.content.is_empty(),
        "Should return content or error for zero limit"
    );
}

/// Test tool with null arguments
#[tokio::test]
async fn test_tool_with_null_arguments() {
    let harness = McpTestHarness::new();

    let args = json!(null);
    let result = harness.call_tool("get_inbox", Some(args)).await;

    // Should handle null arguments gracefully
    assert!(
        !result.content.is_empty(),
        "Should return content or error for null arguments"
    );
}

/// Test tool with extra unexpected arguments
#[tokio::test]
async fn test_tool_with_extra_arguments() {
    let harness = McpTestHarness::new();

    let args = json!({
        "limit": 10,
        "unexpected_field": "should be ignored",
        "another_field": 12345
    });

    let result = harness.call_tool("get_inbox", Some(args)).await;

    // Should process normally, ignoring extra fields
    assert!(
        !result.is_error,
        "Should process normally with extra fields"
    );
}

/// Test rapid sequential tool calls
#[tokio::test]
async fn test_rapid_sequential_tool_calls() {
    let harness = McpTestHarness::new();

    // Call multiple tools rapidly
    for _ in 0..10 {
        let result = harness
            .call_tool("get_inbox", Some(json!({"limit": 1})))
            .await;
        assert!(!result.is_error, "Rapid calls should work without errors");
    }
}

/// Test tool with very long string argument
#[tokio::test]
async fn test_tool_with_very_long_string_argument() {
    let harness = McpTestHarness::new();

    let long_query = "a".repeat(10000);
    let args = json!({"query": long_query});

    let result = harness.call_tool("search_tasks", Some(args)).await;

    // Should handle long strings gracefully
    assert!(!result.content.is_empty(), "Should handle long strings");
}

/// Test tool with deeply nested argument structure
#[tokio::test]
async fn test_tool_with_deeply_nested_arguments() {
    let harness = McpTestHarness::new();

    // Create deeply nested structure
    let mut nested = json!({"value": "deep"});
    for _ in 0..50 {
        nested = json!({"nested": nested});
    }

    let result = harness.call_tool("get_inbox", Some(nested)).await;

    // Should handle or reject deeply nested structures gracefully
    assert!(
        !result.content.is_empty(),
        "Should handle deeply nested args"
    );
}
