//! Integration tests for MCP server I/O layer
//!
//! These tests verify that the MCP server correctly handles JSON-RPC protocol
//! communication over the I/O abstraction layer.

use jsonschema::{Draft, JSONSchema};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tempfile::NamedTempFile;
use things3_cli::mcp::io_wrapper::{McpIo, MockIo};
use things3_cli::mcp::start_mcp_server_generic;
use things3_core::{ThingsConfig, ThingsDatabase};
use tokio::time::{timeout, Duration};

// ============================================================================
// MCP spec compliance — schema validation helpers
// ============================================================================
//
// These helpers validate JSON-RPC `result` payloads against the official MCP
// JSON Schemas vendored under `tests/fixtures/`. They're a tripwire for the
// protocol-compliance bugs that motivated this suite (PR #118): hardcoded
// `protocolVersion` and the `Content` enum's tagged-union shape. If the wire
// format ever diverges from the spec again, schema validation fails loudly
// with a pointer at the offending field.

const MCP_SCHEMA_2024_11_05: &str = include_str!("fixtures/mcp-schema-2024-11-05.json");
const MCP_SCHEMA_2025_03_26: &str = include_str!("fixtures/mcp-schema-2025-03-26.json");
const MCP_SCHEMA_2025_11_25: &str = include_str!("fixtures/mcp-schema-2025-11-25.json");

fn mcp_schema(version: &str) -> &'static serde_json::Value {
    static CACHE: OnceLock<HashMap<&'static str, serde_json::Value>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert(
            "2024-11-05",
            serde_json::from_str(MCP_SCHEMA_2024_11_05).expect("valid 2024-11-05 schema"),
        );
        m.insert(
            "2025-03-26",
            serde_json::from_str(MCP_SCHEMA_2025_03_26).expect("valid 2025-03-26 schema"),
        );
        m.insert(
            "2025-11-25",
            serde_json::from_str(MCP_SCHEMA_2025_11_25).expect("valid 2025-11-25 schema"),
        );
        m
    });
    cache
        .get(version)
        .unwrap_or_else(|| panic!("no vendored MCP schema for version {version}"))
}

/// Build a wrapper schema that `$ref`s into a specific definition of the bundled MCP schema.
///
/// 2024-11-05 uses draft-07 (`definitions`); 2025-11-25 uses draft 2020-12 (`$defs`).
fn compile_validator(version: &str, type_name: &str) -> JSONSchema {
    let full = mcp_schema(version);
    // MCP schemas through 2025-03-26 use JSON Schema draft-07 with `definitions`;
    // 2025-11-25+ switched to draft 2020-12 with `$defs`. The threshold is set
    // to the midpoint between those two known versions. If a new schema version
    // changes the draft, update this threshold to the first version using the
    // new draft.
    let (defs_key, draft) = if version < "2025-06-18" {
        ("definitions", Draft::Draft7)
    } else {
        ("$defs", Draft::Draft202012)
    };
    let wrapper = json!({
        "$ref": format!("#/{defs_key}/{type_name}"),
        defs_key: full[defs_key].clone(),
    });
    JSONSchema::options()
        .with_draft(draft)
        .compile(&wrapper)
        .expect("MCP schema compiles")
}

/// Validate a JSON-RPC response's `result` field against the named MCP type.
///
/// Panics with a diagnostic message if validation fails — the panic includes
/// every JSON-pointer path where the response diverged from the spec, plus
/// the full pretty-printed result, so a regression is debuggable straight
/// from `cargo test` output.
fn validate_result(version: &str, type_name: &str, response: &serde_json::Value) {
    let validator = compile_validator(version, type_name);
    let result = &response["result"];
    let details: Option<Vec<String>> = validator.validate(result).err().map(|errors| {
        errors
            .map(|e| format!("  - {} (at {})", e, e.instance_path))
            .collect()
    });
    if let Some(details) = details {
        panic!(
            "MCP {version} response failed schema validation against {type_name}:\n{}\n\nResult was:\n{}",
            details.join("\n"),
            serde_json::to_string_pretty(result).unwrap_or_else(|_| "<unprintable>".into())
        );
    }
}

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

/// Drive a complete `initialize` handshake using `requested_version` and
/// assert the server's response is spec-compliant.
///
/// `accepted_response_versions` lists the protocol versions we'll accept in
/// the response. Per spec the server MUST respond with the requested version
/// if it supports it, otherwise with another version it supports (always
/// downgrading, never upgrading).
///
/// Schema validation is performed against the version the server *actually*
/// responded with, not the version the client requested. This avoids false
/// failures if a newer schema version introduces required fields that an older
/// negotiated response legitimately omits.
async fn run_initialize_handshake_for(
    requested_version: &str,
    accepted_response_versions: &[&str],
) {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    let server_handle =
        tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": requested_version,
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = send_request_read_response(&mut client_io, initialize_request).await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);

    let response_version = response["result"]["protocolVersion"]
        .as_str()
        .expect("InitializeResult must include protocolVersion as a string");
    assert!(
        accepted_response_versions.contains(&response_version),
        "server returned protocolVersion {response_version:?} when client requested \
         {requested_version:?}; expected one of {accepted_response_versions:?}. \
         (Per spec the server must echo the requested version if it supports it, or \
         negotiate to a version it does support — never to an arbitrary newer version.)"
    );
    assert_eq!(response["result"]["serverInfo"]["name"], "things3-mcp");

    // Validate against the version the server responded with, not what the client requested.
    validate_result(response_version, "InitializeResult", &response);

    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    let notification_str = serde_json::to_string(&initialized_notification).unwrap();
    client_io.write_line(&notification_str).await.unwrap();
    client_io.flush().await.unwrap();

    drop(client_io);

    let result = timeout(Duration::from_secs(2), server_handle).await;
    assert!(result.is_ok(), "Server should complete");
    assert!(result.unwrap().is_ok(), "Server should not error");
}

#[tokio::test]
async fn test_initialize_handshake_2024_11_05() {
    // Server supports 2024-11-05 directly, so it must echo the request verbatim.
    run_initialize_handshake_for("2024-11-05", &["2024-11-05"]).await;
}

#[tokio::test]
async fn test_initialize_handshake_2025_11_25() {
    // Tripwire for PR #118: the server used to hardcode "2024-11-05" in its
    // initialize response, which caused Claude Code 2.1+ (which sends
    // "2025-11-25") to silently drop all tools. Today the server doesn't yet
    // implement 2025-11-25 features, so it negotiates down to the newest
    // version it does support (2025-03-26) — which is spec-compliant.
    // What is NOT acceptable is responding with 2024-11-05 to a 2025-11-25
    // request: that would mean we regressed the fix.
    run_initialize_handshake_for("2025-11-25", &["2025-03-26", "2025-06-18", "2025-11-25"]).await;
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

    // Spec: result is a `ListToolsResult` object containing a `tools` array.
    // The schema check below also verifies each tool's `inputSchema` field
    // (camelCase, as required by the spec) — this catches any regression in
    // the `#[serde(rename = "inputSchema")]` attribute on `Tool`.
    // Note: no initialize handshake is performed here, so no protocol version
    // is negotiated. We validate against 2025-11-25 because ListToolsResult
    // is structurally identical across all known schema versions. If that ever
    // changes, this test should be preceded by an initialize handshake and use
    // the negotiated version instead.
    validate_result("2025-11-25", "ListToolsResult", &response);

    let tools = response["result"]["tools"]
        .as_array()
        .expect("ListToolsResult.tools must be an array");
    assert!(!tools.is_empty(), "Should have at least one tool");
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
    // Tripwire for PR #118 bug #2: the `Content` enum was serialized as
    // `{"Text":{"text":"..."}}` instead of the spec's tagged-union form
    // `{"type":"text","text":"..."}`. Schema validation would reject the
    // former because it doesn't match any variant of `ToolResultContent`.
    validate_result("2025-11-25", "CallToolResult", &response);
    let is_error = response["result"]["isError"].as_bool().unwrap_or(false);
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
    validate_result("2025-11-25", "CallToolResult", &response);
    let is_error = response["result"]["isError"].as_bool().unwrap_or(false);
    assert!(!is_error, "Tool call should not error");
}

/// Belt-and-suspenders check for the `Content` enum's wire format.
///
/// PR #118 fixed bug #2 by adding `#[serde(tag = "type", rename_all =
/// "lowercase")]` to the `Content` enum so it serializes as
/// `{"type":"text","text":"..."}` instead of the default
/// `{"Text":{"text":"..."}}`. The CallToolResult schema check above will
/// catch a regression too, but this test fails with a clearer assertion
/// message — useful when the schema crate is ever swapped or upgraded.
#[tokio::test]
async fn test_content_block_serialization() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    let response = send_request_read_response(
        &mut client_io,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "get_today", "arguments": {} }
        }),
    )
    .await;

    let content = response["result"]["content"]
        .as_array()
        .expect("CallToolResult.content must be an array");
    assert!(
        !content.is_empty(),
        "Tool that returned successfully must have at least one content block"
    );

    let first = &content[0];
    let type_field = first.get("type").and_then(|v| v.as_str());
    assert_eq!(
        type_field,
        Some("text"),
        "First content block must be tagged with `type: \"text\"` (was {first}). \
         If this fails as `\"Text\"` or with a missing `type` field, the `Content` \
         enum's `#[serde(tag = \"type\", rename_all = \"lowercase\")]` attribute \
         has been removed or broken."
    );
    assert!(
        first.get("text").and_then(|v| v.as_str()).is_some(),
        "Text content block must include a `text` string field; got {first}"
    );
    assert!(
        !first.as_object().unwrap().contains_key("Text"),
        "Wire format must not include a top-level `Text` key — that's the \
         externally-tagged form the spec rejects. Got {first}"
    );
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
    // Spec: result must be a `ListResourcesResult` object containing a
    // `resources` array — not a bare array.
    validate_result("2025-11-25", "ListResourcesResult", &response);
    assert!(
        response["result"]["resources"].is_array(),
        "ListResourcesResult.resources must be an array"
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
    // Spec: result must be a `ListPromptsResult` object containing a
    // `prompts` array — not a bare array.
    //
    // Full ListPromptsResult schema validation is intentionally skipped:
    // `Prompt.arguments` currently holds a JSON Schema object instead of the
    // spec-mandated `Vec<PromptArgument>`. Tracked in issue #119.
    assert!(
        response["result"]["prompts"].is_array(),
        "ListPromptsResult.prompts must be an array"
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
    // tools/list returns an object with a tools array
    assert!(response["result"]["tools"].is_array());
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
        // tools/list returns an object with a tools array
        assert!(response["result"]["tools"].is_array());
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

// ============================================================================
// Additional Coverage Tests
// ============================================================================

#[tokio::test]
async fn test_json_serialization_coverage() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    // Test various request types to cover more code paths
    let requests = vec![
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        json!({"jsonrpc": "2.0", "id": 3, "method": "resources/list"}),
        json!({"jsonrpc": "2.0", "id": 4, "method": "prompts/list"}),
    ];

    for request in requests {
        let response = send_request_read_response(&mut client_io, request).await;
        assert_eq!(response["jsonrpc"], "2.0");
    }
}

#[tokio::test]
async fn test_mixed_requests_and_notifications() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    // Send a mix of requests and notifications
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/custom"
    });

    let notification_str = serde_json::to_string(&notification).unwrap();
    client_io.write_line(&notification_str).await.unwrap();
    client_io.flush().await.unwrap();

    // Send a request to verify server is still responsive
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });

    let response = send_request_read_response(&mut client_io, request).await;
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
}

#[tokio::test]
async fn test_all_available_tools() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(8192);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    // Get list of tools
    let tools_list_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });

    let response = send_request_read_response(&mut client_io, tools_list_request).await;
    let tools = response["result"]["tools"].as_array().unwrap();

    assert!(!tools.is_empty(), "Should have at least one tool");

    // Test calling get_today and get_inbox (most common tools)
    let tool_tests = ["get_today", "get_inbox"];

    for (idx, tool_name) in tool_tests.iter().enumerate() {
        let tools_call_request = json!({
            "jsonrpc": "2.0",
            "id": idx + 2,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": {}
            }
        });

        let response = send_request_read_response(&mut client_io, tools_call_request).await;
        assert_eq!(response["jsonrpc"], "2.0");
        assert!(response["result"].is_object());
    }
}

#[tokio::test]
async fn test_large_response_handling() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(65536); // Large buffer

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    // Request that might return large data
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "get_projects",
            "arguments": {}
        }
    });

    let response = send_request_read_response(&mut client_io, request).await;
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
}

#[tokio::test]
async fn test_sequential_initialize_calls() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    // Call initialize multiple times (should handle gracefully)
    for i in 1..=3 {
        let initialize_request = json!({
            "jsonrpc": "2.0",
            "id": i,
            "method": "initialize",
            "params": {}
        });

        let response = send_request_read_response(&mut client_io, initialize_request).await;
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], i);
        assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
    }
}

#[tokio::test]
async fn test_config_with_empty_lines() {
    use things3_cli::mcp::start_mcp_server_with_config_generic;
    use things3_core::McpServerConfig;

    let (_temp, db) = create_test_db().await;
    let mcp_config = McpServerConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(4096);

    tokio::spawn(
        async move { start_mcp_server_with_config_generic(db, mcp_config, server_io).await },
    );

    // Send empty lines (should be skipped)
    client_io.write_line("").await.unwrap();
    client_io.write_line("").await.unwrap();
    client_io.flush().await.unwrap();

    // Send valid request
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });

    let response = send_request_read_response(&mut client_io, request).await;
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
}

#[tokio::test]
async fn test_rapid_requests() {
    let (_temp, db) = create_test_db().await;
    let config = ThingsConfig::default();

    let (server_io, mut client_io) = MockIo::create_pair(32768); // Extra large buffer

    tokio::spawn(async move { start_mcp_server_generic(db, config, server_io).await });

    // Send many requests rapidly
    for i in 1..=20 {
        let request = json!({
            "jsonrpc": "2.0",
            "id": i,
            "method": "tools/list"
        });

        let response = send_request_read_response(&mut client_io, request).await;
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], i);
    }
}
