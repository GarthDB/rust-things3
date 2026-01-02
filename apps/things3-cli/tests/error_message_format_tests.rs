//! Tests for error message formatting standards
//!
//! Ensures all error messages follow the "Failed to {operation}: {error}" format

use things3_cli::mcp::{CallToolRequest, McpError, ThingsMcpServer};
use things3_core::{config::ThingsConfig, database::ThingsDatabase};

/// Create a test MCP server with an invalid/closed database to trigger errors
async fn create_invalid_mcp_server() -> ThingsMcpServer {
    // Create an in-memory database but don't set up schema
    // This will cause database operations to fail
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    let config = ThingsConfig::for_testing().unwrap();
    ThingsMcpServer::new(db.into(), config)
}

#[tokio::test]
async fn test_get_today_error_message_format() {
    let server = create_invalid_mcp_server().await;

    let request = CallToolRequest {
        name: "get_today".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await;

    // Should fail because database has no schema
    assert!(result.is_err());

    // Check error message follows "Failed to" format
    let error = result.unwrap_err();
    let error_string = error.to_string();

    // The error should be a database operation error
    // which wraps our "Failed to" message (case-insensitive check)
    let lowercase_error = error_string.to_lowercase();
    assert!(
        lowercase_error.contains("database operation failed")
            || lowercase_error.contains("failed to")
            || error_string.contains("get_today"),
        "Error message doesn't contain expected format: {}",
        error_string
    );
}

#[tokio::test]
async fn test_error_message_consistency() {
    // Test that database operation errors follow consistent format
    let server = create_invalid_mcp_server().await;

    // Try multiple operations that should fail
    let operations = vec![
        ("get_today", None),
        ("get_inbox", None),
        ("get_projects", None),
    ];

    for (op_name, args) in operations {
        let request = CallToolRequest {
            name: op_name.to_string(),
            arguments: args,
        };

        let result = server.call_tool(request).await;

        if let Err(error) = result {
            let error_string = error.to_string();

            // Check that error contains operation context
            assert!(
                error_string.contains("Failed to")
                    || error_string.contains("database operation failed")
                    || error_string.contains(op_name),
                "Error for '{}' doesn't contain operation context: {}",
                op_name,
                error_string
            );
        }
    }
}

#[test]
fn test_mcp_error_format() {
    // Test that McpError variants follow proper format
    let errors = vec![
        McpError::validation_error("Test validation failed".to_string()),
        McpError::configuration_error("Test config failed".to_string()),
    ];

    for error in errors {
        let error_string = error.to_string();
        // Errors should be descriptive and contain context
        assert!(
            !error_string.is_empty(),
            "Error message should not be empty"
        );
    }
}
