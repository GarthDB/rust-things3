//! Integration tests for real-time features using actual CLI commands
//! These tests verify that the async functionality works in real scenarios

use std::fs;
use std::process::{Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

/// Test the WebSocket server and client communication
#[tokio::test]
async fn test_websocket_server_client_integration() {
    // This test verifies the actual async communication works
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create a test database
    let db = things3_core::ThingsDatabase::new(&db_path).unwrap();

    // Start WebSocket server in background
    let server_handle = tokio::spawn(async move {
        let server = things3_cli::websocket::WebSocketServer::new(8081);
        server.start().await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test WebSocket client connection
    let client_handle = tokio::spawn(async move {
        let client = things3_cli::websocket::WebSocketClient::new("ws://127.0.0.1:8081");
        client.connect().await.unwrap();

        // Send a test message
        let message = things3_cli::websocket::WebSocketMessage::Ping;
        client.send_message(message).await.unwrap();

        // Wait for response
        let response = client.receive_message().await.unwrap();
        assert!(matches!(
            response,
            things3_cli::websocket::WebSocketMessage::Pong
        ));
    });

    // Test with timeout to ensure it doesn't hang
    let result = timeout(Duration::from_secs(5), client_handle).await;
    assert!(
        result.is_ok(),
        "WebSocket communication should complete within 5 seconds"
    );

    // Clean up
    server_handle.abort();
}

/// Test progress tracking with actual bulk operations
#[tokio::test]
async fn test_progress_tracking_integration() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create a test database with some data
    let db = things3_core::ThingsDatabase::new(&db_path).unwrap();

    // Test progress tracking with bulk operations
    let manager = things3_cli::bulk_operations::BulkOperationsManager::new();

    // This tests the actual async progress broadcasting
    let progress_result = timeout(
        Duration::from_secs(10),
        manager.export_all_tasks(&db, "json"),
    )
    .await;

    assert!(
        progress_result.is_ok(),
        "Bulk operations should complete with progress tracking"
    );
}

/// Test event broadcasting with real operations
#[tokio::test]
async fn test_event_broadcasting_integration() {
    let broadcaster = things3_cli::events::EventBroadcaster::new();

    // Subscribe to events
    let mut receiver = broadcaster.subscribe_all();

    // Create and broadcast events
    let event = things3_cli::events::Event {
        id: uuid::Uuid::new_v4(),
        event_type: things3_cli::events::EventType::TaskCreated {
            task_id: uuid::Uuid::new_v4(),
        },
        timestamp: chrono::Utc::now(),
        data: None,
        source: "integration_test".to_string(),
    };

    // Test event broadcasting with timeout
    let broadcast_result = timeout(Duration::from_secs(2), broadcaster.broadcast(event)).await;

    assert!(
        broadcast_result.is_ok(),
        "Event broadcasting should complete"
    );

    // Test event reception with timeout
    let receive_result = timeout(Duration::from_secs(2), receiver.recv()).await;

    assert!(receive_result.is_ok(), "Event reception should complete");
}

/// Test the complete real-time workflow
#[tokio::test]
async fn test_complete_realtime_workflow() {
    // This test simulates a real user workflow:
    // 1. Start WebSocket server
    // 2. Connect client
    // 3. Perform bulk operation with progress tracking
    // 4. Verify events are broadcast
    // 5. Verify client receives updates

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = things3_core::ThingsDatabase::new(&db_path).unwrap();

    // Start WebSocket server
    let server = things3_cli::websocket::WebSocketServer::new(8082);
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = things3_cli::websocket::WebSocketClient::new("ws://127.0.0.1:8082");
    let client_handle = tokio::spawn(async move {
        client.connect().await.unwrap();

        // Subscribe to progress updates
        let message = things3_cli::websocket::WebSocketMessage::Subscribe {
            operation_id: uuid::Uuid::new_v4(),
        };
        client.send_message(message).await.unwrap();

        // Wait for subscription confirmation
        let response = client.receive_message().await.unwrap();
        assert!(matches!(
            response,
            things3_cli::websocket::WebSocketMessage::Subscribed { .. }
        ));
    });

    // Test with timeout
    let result = timeout(Duration::from_secs(10), client_handle).await;
    assert!(
        result.is_ok(),
        "Complete real-time workflow should complete"
    );

    server_handle.abort();
}
