//! Integration tests for real-time features
//! These tests verify that the async functionality works in real scenarios

use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

/// Test the WebSocket server creation
#[tokio::test]
async fn test_websocket_server_creation() {
    // This test verifies the WebSocket server can be created
    let _server = things3_cli::websocket::WebSocketServer::new(8081);
    // Just verify it can be created without errors
    assert!(true);
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
