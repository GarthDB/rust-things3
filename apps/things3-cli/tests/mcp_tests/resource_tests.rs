//! Resource-related tests for MCP server

#![cfg(feature = "mcp-server")]

use super::common::create_test_mcp_server;
use things3_cli::mcp::{Content, McpError};

#[tokio::test]
async fn test_list_resources() {
    let server = create_test_mcp_server().await;

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
    let server = create_test_mcp_server().await;

    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://inbox".to_string(),
    };

    let result = server.read_resource(request).await.unwrap();

    assert_eq!(result.contents.len(), 1);
    match &result.contents[0] {
        Content::Text { text } => {
            // Should be valid JSON
            let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert!(parsed.is_array());
        }
    }
}

#[tokio::test]
async fn test_read_projects_resource() {
    let server = create_test_mcp_server().await;

    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://projects".to_string(),
    };

    let result = server.read_resource(request).await.unwrap();

    assert_eq!(result.contents.len(), 1);
    match &result.contents[0] {
        Content::Text { text } => {
            // Should be valid JSON
            let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert!(parsed.is_array());
        }
    }
}

#[tokio::test]
async fn test_read_areas_resource() {
    let server = create_test_mcp_server().await;

    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://areas".to_string(),
    };

    let result = server.read_resource(request).await.unwrap();

    assert_eq!(result.contents.len(), 1);
    match &result.contents[0] {
        Content::Text { text } => {
            // Should be valid JSON
            let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert!(parsed.is_array());
        }
    }
}

#[tokio::test]
async fn test_read_today_resource() {
    let server = create_test_mcp_server().await;

    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://today".to_string(),
    };

    let result = server.read_resource(request).await.unwrap();

    assert_eq!(result.contents.len(), 1);
    match &result.contents[0] {
        Content::Text { text } => {
            // Should be valid JSON
            let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert!(parsed.is_array());
        }
    }
}

#[tokio::test]
async fn test_read_unknown_resource() {
    let server = create_test_mcp_server().await;

    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://unknown".to_string(),
    };

    let result = server.read_resource(request).await;

    // Should return an error for unknown resource
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ResourceNotFound { uri } => {
            assert_eq!(uri, "things://unknown");
        }
        _ => panic!("Expected ResourceNotFound error"),
    }
}

#[tokio::test]
async fn test_resource_fallback_error_handling() {
    let server = create_test_mcp_server().await;

    // Test read_resource_with_fallback for unknown resource
    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://unknown".to_string(),
    };

    let result = server.read_resource_with_fallback(request).await;
    match &result.contents[0] {
        Content::Text { text } => {
            assert!(text.contains("Resource 'things://unknown' not found"));
            assert!(text.contains("Available resources can be listed"));
        }
    }
}

#[tokio::test]
async fn test_read_resource_with_fallback() {
    let server = create_test_mcp_server().await;

    // Test with a valid resource
    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://inbox".to_string(),
    };

    let result = server.read_resource_with_fallback(request).await;
    assert!(!result.contents.is_empty());

    // Test with an invalid resource
    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://nonexistent".to_string(),
    };

    let result = server.read_resource_with_fallback(request).await;
    // The fallback should return an error message, not empty contents
    assert!(!result.contents.is_empty());
    let Content::Text { text } = &result.contents[0];
    assert!(text.contains("not found"));
}
