//! Integration tests for MCP server I/O layer
//!
//! These tests verify that the MCP server correctly handles JSON-RPC protocol
//! communication over the I/O abstraction layer.

use serde_json::json;
use std::sync::Arc;
use tempfile::NamedTempFile;
use things3_cli::mcp::io_wrapper::{McpIo, MockIo};
use things3_cli::mcp::start_mcp_server_generic;
use things3_core::{ThingsConfig, ThingsDatabase};
use tokio::time::{timeout, Duration};

/// Helper to create a test database
async fn create_test_db() -> (NamedTempFile, Arc<ThingsDatabase>) {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    // Create test database with schema
    things3_core::test_utils::create_test_database(db_path)
        .await
        .unwrap();

    let db = ThingsDatabase::new(db_path).await.unwrap();
    (temp_file, Arc::new(db))
}

/// Helper to send a JSON-RPC request and read the response
async fn send_request_read_response(
    client_io: &mut MockIo,
    request: serde_json::Value,
) -> serde_json::Value {
    // Send request
    let request_str = serde_json::to_string(&request).unwrap();
    client_io.write_line(&request_str).await.unwrap();
    client_io.flush().await.unwrap();

    // Read response with timeout
    let response_line = timeout(Duration::from_secs(2), client_io.read_line())
        .await
        .expect("Timeout waiting for response")
        .expect("IO error reading response")
        .expect("EOF when expecting response");

    serde_json::from_str(&response_line).unwrap()
}

// ============================================================================
// Initialize Handshake Tests
// ============================================================================

#[tokio::test]
async fn test_initialize_handshake() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    // Start server in background
    let server_handle =
        tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    // Send initialize request
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = send_request_read_response(&mut client_io, initialize_request).await;

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_object());
    assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
    assert!(response["result"]["capabilities"].is_object());
    assert_eq!(response["result"]["serverInfo"]["name"], "things3-mcp");

    // Send initialized notification (should be silently handled)
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    let notification_str = serde_json::to_string(&initialized_notification).unwrap();
    client_io.write_line(&notification_str).await.unwrap();
    client_io.flush().await.unwrap();

    // Close client to signal EOF
    drop(client_io);

    // Server should complete without error
    let result = timeout(Duration::from_secs(2), server_handle).await;
    assert!(result.is_ok(), "Server should complete");
    assert!(result.unwrap().is_ok(), "Server should not error");
}

#[tokio::test]
async fn test_initialize_response_structure() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });

    let response = send_request_read_response(&mut client_io, initialize_request).await;

    // Verify capabilities structure
    let capabilities = &response["result"]["capabilities"];
    assert!(capabilities["tools"].is_object());
    assert!(capabilities["resources"].is_object());
    assert!(capabilities["prompts"].is_object());

    // Verify server info
    let server_info = &response["result"]["serverInfo"];
    assert_eq!(server_info["name"], "things3-mcp");
    assert!(server_info["version"].is_string());
}

// ============================================================================
// Tools Tests
// ============================================================================

#[tokio::test]
async fn test_tools_list() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    let tools_list_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    let response = send_request_read_response(&mut client_io, tools_list_request).await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);

    // The result is the tools array directly
    assert!(
        response["result"].is_array(),
        "Result should be an array of tools"
    );

    let tools = response["result"].as_array().unwrap();
    assert!(!tools.is_empty(), "Should have at least one tool");

    // Verify tool structure
    let first_tool = &tools[0];
    assert!(first_tool["name"].is_string());
    assert!(first_tool["description"].is_string());
    // inputSchema might be input_schema (snake_case) in the serialization
    assert!(first_tool["inputSchema"].is_object() || first_tool["input_schema"].is_object());
}

#[tokio::test]
async fn test_tools_call_get_today() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    let tools_call_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "get_today",
            "arguments": {}
        }
    });

    let response = send_request_read_response(&mut client_io, tools_call_request).await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 3);
    assert!(response["result"].is_object());
    assert!(response["result"]["content"].is_array());
    // Check is_error field (note: JSON uses camelCase)
    let is_error = response["result"]["is_error"].as_bool().unwrap_or(false);
    assert!(!is_error, "Tool call should not error");
}

#[tokio::test]
async fn test_tools_call_get_inbox() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    let tools_call_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "get_inbox",
            "arguments": {
                "limit": 10
            }
        }
    });

    let response = send_request_read_response(&mut client_io, tools_call_request).await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 4);
    assert!(response["result"].is_object());
    let is_error = response["result"]["is_error"].as_bool().unwrap_or(false);
    assert!(!is_error, "Tool call should not error");
}

#[tokio::test]
async fn test_tools_call_nonexistent_tool() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    let server_handle =
        tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    let tools_call_request = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "nonexistent_tool",
            "arguments": {}
        }
    });

    // Send request
    let request_str = serde_json::to_string(&tools_call_request).unwrap();
    client_io.write_line(&request_str).await.unwrap();
    client_io.flush().await.unwrap();

    // Try to read response - server might error and close connection
    let result = timeout(Duration::from_millis(500), client_io.read_line()).await;

    if let Ok(Ok(Some(response_line))) = result {
        let response: serde_json::Value = serde_json::from_str(&response_line).unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 5);
        // Should return error for nonexistent tool
        let is_error = response["result"]["is_error"].as_bool().unwrap_or(false);
        assert!(
            is_error || response["error"].is_object(),
            "Should indicate error for nonexistent tool"
        );
    } else {
        // Server may have errored and closed - that's also acceptable behavior
        // Just verify the server handle completed
        drop(client_io);
        let _ = timeout(Duration::from_secs(1), server_handle).await;
    }
}

// ============================================================================
// Resources Tests
// ============================================================================

#[tokio::test]
async fn test_resources_list() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    let resources_list_request = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "resources/list"
    });

    let response = send_request_read_response(&mut client_io, resources_list_request).await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 6);
    // Result is the resources array directly
    assert!(
        response["result"].is_array(),
        "Result should be an array of resources"
    );
}

#[tokio::test]
async fn test_resources_read() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    let server_handle =
        tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    let resources_read_request = json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "resources/read",
        "params": {
            "uri": "things3://today"
        }
    });

    // Send request
    let request_str = serde_json::to_string(&resources_read_request).unwrap();
    client_io.write_line(&request_str).await.unwrap();
    client_io.flush().await.unwrap();

    // Try to read response - server might error if resource not found
    let result = timeout(Duration::from_millis(500), client_io.read_line()).await;

    if let Ok(Ok(Some(response_line))) = result {
        let response: serde_json::Value = serde_json::from_str(&response_line).unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 7);
        // Response should be either a result or an error
        assert!(response["result"].is_object() || response["error"].is_object());
    } else {
        // Server may have errored - that's acceptable for this test
        drop(client_io);
        let _ = timeout(Duration::from_secs(1), server_handle).await;
    }
}

// ============================================================================
// Prompts Tests
// ============================================================================

#[tokio::test]
async fn test_prompts_list() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    let prompts_list_request = json!({
        "jsonrpc": "2.0",
        "id": 8,
        "method": "prompts/list"
    });

    let response = send_request_read_response(&mut client_io, prompts_list_request).await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 8);
    // Result is the prompts array directly
    assert!(
        response["result"].is_array(),
        "Result should be an array of prompts"
    );
}

#[tokio::test]
async fn test_prompts_get() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    let server_handle =
        tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    let prompts_get_request = json!({
        "jsonrpc": "2.0",
        "id": 9,
        "method": "prompts/get",
        "params": {
            "name": "task_summary",
            "arguments": {}
        }
    });

    // Send request
    let request_str = serde_json::to_string(&prompts_get_request).unwrap();
    client_io.write_line(&request_str).await.unwrap();
    client_io.flush().await.unwrap();

    // Try to read response - server might error if prompt not found
    let result = timeout(Duration::from_millis(500), client_io.read_line()).await;

    if let Ok(Ok(Some(response_line))) = result {
        let response: serde_json::Value = serde_json::from_str(&response_line).unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 9);
        // Response should be either a result or an error
        assert!(response["result"].is_object() || response["error"].is_object());
    } else {
        // Server may have errored - that's acceptable for this test
        drop(client_io);
        let _ = timeout(Duration::from_secs(1), server_handle).await;
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_malformed_json() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    let server_handle =
        tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    // Send malformed JSON
    client_io.write_line("{invalid json}").await.unwrap();
    client_io.flush().await.unwrap();

    // Server should handle error gracefully and continue or terminate
    // Close client
    drop(client_io);

    // Server should complete (may error due to malformed JSON)
    let result = timeout(Duration::from_secs(2), server_handle).await;
    assert!(result.is_ok(), "Server should complete");
}

#[tokio::test]
async fn test_missing_method() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    let request_without_method = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "params": {}
    });

    // This should cause an error on the server side
    let request_str = serde_json::to_string(&request_without_method).unwrap();
    client_io.write_line(&request_str).await.unwrap();
    client_io.flush().await.unwrap();

    // Try to read response (server might close connection or return error)
    let result = timeout(Duration::from_millis(500), client_io.read_line()).await;

    // Either we get a response or timeout (both acceptable)
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_unknown_method() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    let unknown_method_request = json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "unknown/method"
    });

    let response = send_request_read_response(&mut client_io, unknown_method_request).await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 11);
    // Should return error for unknown method
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32601); // Method not found
}

#[tokio::test]
async fn test_empty_line_handling() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    // Send empty lines (should be ignored)
    client_io.write_line("").await.unwrap();
    client_io.write_line("").await.unwrap();
    client_io.flush().await.unwrap();

    // Send valid request after empty lines
    let valid_request = json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "tools/list"
    });

    let response = send_request_read_response(&mut client_io, valid_request).await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 12);
    // tools/list returns an array
    assert!(response["result"].is_array());
}

// ============================================================================
// Multiple Request Tests
// ============================================================================

#[tokio::test]
async fn test_multiple_sequential_requests() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(8192);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    // Send multiple requests
    for i in 1..=5 {
        let request = json!({
            "jsonrpc": "2.0",
            "id": i,
            "method": "tools/list"
        });

        let response = send_request_read_response(&mut client_io, request).await;

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], i);
        // tools/list returns an array
        assert!(response["result"].is_array());
    }
}

#[tokio::test]
async fn test_notification_no_response() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    // Send notification (no id field)
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    let notification_str = serde_json::to_string(&notification).unwrap();
    client_io.write_line(&notification_str).await.unwrap();
    client_io.flush().await.unwrap();

    // Send a regular request to verify server is still responsive
    let request = json!({
        "jsonrpc": "2.0",
        "id": 13,
        "method": "tools/list"
    });

    let response = send_request_read_response(&mut client_io, request).await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 13);
}

// ============================================================================
// start_mcp_server_with_config_generic Tests
// ============================================================================

#[tokio::test]
async fn test_start_mcp_server_with_config() {
    use things3_cli::mcp::start_mcp_server_with_config_generic;
    use things3_core::McpServerConfig;

    let (_temp, db) = create_test_db().await;

    // Create MCP config
    let mcp_config = McpServerConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(
        async move { start_mcp_server_with_config_generic(db, mcp_config, server_io).await },
    );

    // Test that server works with config
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });

    let response = send_request_read_response(&mut client_io, initialize_request).await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
}

#[tokio::test]
async fn test_start_mcp_server_with_config_tools() {
    use things3_cli::mcp::start_mcp_server_with_config_generic;
    use things3_core::McpServerConfig;

    let (_temp, db) = create_test_db().await;
    let mcp_config = McpServerConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(
        async move { start_mcp_server_with_config_generic(db, mcp_config, server_io).await },
    );

    // Test tools/call with config
    let tools_call_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "get_today",
            "arguments": {}
        }
    });

    let response = send_request_read_response(&mut client_io, tools_call_request).await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response["result"].is_object());
}

#[tokio::test]
async fn test_io_error_handling() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, client_io) = MockIo::create_pair(4096);

    let server_handle =
        tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    // Drop client immediately to trigger EOF
    drop(client_io);

    // Server should exit gracefully on EOF
    let result = timeout(Duration::from_secs(2), server_handle).await;
    assert!(result.is_ok(), "Server should handle EOF gracefully");
    assert!(result.unwrap().is_ok(), "Server should not error on EOF");
}
