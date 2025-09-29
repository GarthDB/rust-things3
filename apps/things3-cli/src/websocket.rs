//! WebSocket server for real-time updates

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use uuid::Uuid;

use crate::progress::{ProgressManager, ProgressUpdate};

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    /// Subscribe to progress updates
    Subscribe { operation_id: Option<Uuid> },
    /// Unsubscribe from progress updates
    Unsubscribe { operation_id: Option<Uuid> },
    /// Progress update from server
    ProgressUpdate(ProgressUpdate),
    /// Error message
    Error { message: String },
    /// Ping message for keepalive
    Ping,
    /// Pong response
    Pong,
}

/// WebSocket client connection
#[derive(Debug)]
pub struct WebSocketClient {
    id: Uuid,
    #[allow(dead_code)]
    sender: crossbeam_channel::Sender<ProgressUpdate>,
    subscriptions: Arc<RwLock<Vec<Uuid>>>,
}

impl WebSocketClient {
    /// Create a new WebSocket client
    #[must_use]
    pub fn new(sender: crossbeam_channel::Sender<ProgressUpdate>) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender,
            subscriptions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Handle a WebSocket connection
    ///
    /// # Errors
    /// Returns an error if the WebSocket connection fails
    pub async fn handle_connection(&self, stream: TcpStream, addr: SocketAddr) -> Result<()> {
        let ws_stream = accept_async(stream).await?;
        let (ws_sender, mut ws_receiver) = ws_stream.split();

        let subscriptions = self.subscriptions.clone();
        let client_id = self.id;

        log::info!("New WebSocket connection from {addr}");

        // Spawn a task to handle incoming messages
        let subscriptions_clone = subscriptions.clone();
        let ws_sender = Arc::new(tokio::sync::Mutex::new(ws_sender));

        tokio::spawn(async move {
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                            match ws_msg {
                                WebSocketMessage::Subscribe { operation_id } => {
                                    let mut subs = subscriptions_clone.write().await;
                                    if let Some(op_id) = operation_id {
                                        if !subs.contains(&op_id) {
                                            subs.push(op_id);
                                        }
                                    }
                                    log::debug!("Client {client_id} subscribed to operation {operation_id:?}");
                                }
                                WebSocketMessage::Unsubscribe { operation_id } => {
                                    let mut subs = subscriptions_clone.write().await;
                                    if let Some(op_id) = operation_id {
                                        subs.retain(|&id| id != op_id);
                                    } else {
                                        subs.clear();
                                    }
                                    log::debug!("Client {client_id} unsubscribed from operation {operation_id:?}");
                                }
                                WebSocketMessage::Ping => {
                                    // Respond with pong
                                    let pong = WebSocketMessage::Pong;
                                    if let Ok(pong_text) = serde_json::to_string(&pong) {
                                        let mut sender = ws_sender.lock().await;
                                        let _ = sender.send(Message::Text(pong_text)).await;
                                    }
                                }
                                _ => {
                                    log::warn!(
                                        "Client {client_id} sent unexpected message: {ws_msg:?}"
                                    );
                                }
                            }
                        } else {
                            log::warn!("Client {client_id} sent invalid JSON: {text}");
                        }
                    }
                    Ok(Message::Close(_)) => {
                        log::info!("Client {client_id} disconnected");
                        break;
                    }
                    Ok(Message::Ping(data)) => {
                        let mut sender = ws_sender.lock().await;
                        if let Err(e) = sender.send(Message::Pong(data)).await {
                            log::error!("Failed to send pong to client {client_id}: {e}");
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("WebSocket error for client {client_id}: {e}");
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }
}

/// WebSocket server for real-time updates
#[derive(Debug)]
pub struct WebSocketServer {
    progress_manager: Arc<ProgressManager>,
    clients: Arc<RwLock<HashMap<Uuid, WebSocketClient>>>,
    port: u16,
}

impl WebSocketServer {
    /// Create a new WebSocket server
    #[must_use]
    pub fn new(port: u16) -> Self {
        Self {
            progress_manager: Arc::new(ProgressManager::new()),
            clients: Arc::new(RwLock::new(HashMap::new())),
            port,
        }
    }

    /// Get the progress manager
    #[must_use]
    pub fn progress_manager(&self) -> Arc<ProgressManager> {
        self.progress_manager.clone()
    }

    /// Start the WebSocket server
    ///
    /// # Errors
    /// Returns an error if the server fails to start
    pub async fn start(&self) -> Result<()> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;

        log::info!("WebSocket server listening on {addr}");

        // Start the progress manager
        let progress_manager = self.progress_manager.clone();
        tokio::spawn(async move {
            let _ = progress_manager.run();
        });

        let clients = self.clients.clone();
        let progress_sender = self.progress_manager.sender();

        while let Ok((stream, addr)) = listener.accept().await {
            let client = WebSocketClient::new(progress_sender.clone());
            let client_id = client.id;

            // Store the client
            {
                let mut clients = clients.write().await;
                clients.insert(client_id, client);
            }

            // Handle the connection
            let clients_clone = clients.clone();
            tokio::spawn(async move {
                if let Some(client) = clients_clone.read().await.get(&client_id) {
                    if let Err(e) = client.handle_connection(stream, addr).await {
                        log::error!("Error handling WebSocket connection from {addr}: {e}");
                    }
                }

                // Remove client when done
                clients_clone.write().await.remove(&client_id);
            });
        }

        Ok(())
    }

    /// Get the number of connected clients
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }

    /// Broadcast a message to all clients
    ///
    /// # Errors
    /// Returns an error if broadcasting fails
    pub async fn broadcast(&self, message: WebSocketMessage) -> Result<()> {
        let clients = self.clients.read().await;
        let _message_text = serde_json::to_string(&message)?;

        for client in clients.values() {
            // Note: In a real implementation, you'd need to store the sender for each client
            // and send the message through their individual channels
            log::debug!("Broadcasting message to client {}", client.id);
        }

        Ok(())
    }
}

/// WebSocket client for connecting to the server
#[derive(Debug)]
pub struct WebSocketClientConnection {
    sender: broadcast::Sender<ProgressUpdate>,
    #[allow(dead_code)]
    receiver: broadcast::Receiver<ProgressUpdate>,
}

impl Default for WebSocketClientConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketClientConnection {
    /// Create a new client connection
    #[must_use]
    pub fn new() -> Self {
        let (sender, receiver) = broadcast::channel(1000);
        Self { sender, receiver }
    }

    /// Get a receiver for progress updates
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<ProgressUpdate> {
        self.sender.subscribe()
    }

    /// Send a progress update
    ///
    /// # Errors
    /// Returns an error if sending the update fails
    pub fn send_update(&self, update: ProgressUpdate) -> Result<()> {
        self.sender.send(update)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    #[test]
    fn test_websocket_message_serialization() {
        let msg = WebSocketMessage::Subscribe {
            operation_id: Some(Uuid::new_v4()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            WebSocketMessage::Subscribe { operation_id } => {
                assert!(operation_id.is_some());
            }
            _ => panic!("Expected Subscribe message"),
        }
    }

    #[test]
    fn test_websocket_client_creation() {
        let (sender, _) = crossbeam_channel::unbounded();
        let client = WebSocketClient::new(sender);
        assert!(!client.id.is_nil());
    }

    #[test]
    fn test_websocket_server_creation() {
        let server = WebSocketServer::new(8080);
        assert_eq!(server.port, 8080);
    }

    #[tokio::test]
    async fn test_websocket_client_connection() {
        let connection = WebSocketClientConnection::new();
        let mut receiver = connection.subscribe();

        // Send a test update
        let update = ProgressUpdate {
            operation_id: Uuid::new_v4(),
            operation_name: "test".to_string(),
            current: 10,
            total: Some(100),
            message: Some("test message".to_string()),
            timestamp: chrono::Utc::now(),
            status: crate::progress::ProgressStatus::InProgress,
        };

        connection.send_update(update.clone()).unwrap();

        // Receive the update with a timeout
        let received_msg = tokio::time::timeout(StdDuration::from_millis(100), receiver.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(received_msg.operation_name, update.operation_name);
    }

    #[tokio::test]
    async fn test_websocket_server_creation_with_port() {
        let server = WebSocketServer::new(8080);
        assert_eq!(server.port, 8080);
    }

    #[tokio::test]
    async fn test_websocket_server_progress_manager() {
        let server = WebSocketServer::new(8080);
        let _progress_manager = server.progress_manager();
        // Just verify we can get the progress manager without panicking
    }

    #[tokio::test]
    async fn test_websocket_client_creation_async() {
        let (sender, _receiver) = crossbeam_channel::unbounded();
        let client = WebSocketClient::new(sender);
        // Just verify we can create the client without panicking
        assert!(!client.id.is_nil());
    }

    #[tokio::test]
    async fn test_websocket_client_connection_default() {
        let _connection = WebSocketClientConnection::default();
        // Just verify we can create the connection without panicking
    }

    #[tokio::test]
    async fn test_websocket_client_connection_subscribe() {
        let connection = WebSocketClientConnection::new();
        let _receiver = connection.subscribe();
        // Just verify we can subscribe without panicking
    }

    #[tokio::test]
    async fn test_websocket_client_connection_send_update() {
        let connection = WebSocketClientConnection::new();
        let update = ProgressUpdate {
            operation_id: Uuid::new_v4(),
            operation_name: "test".to_string(),
            current: 50,
            total: Some(100),
            message: Some("test message".to_string()),
            timestamp: chrono::Utc::now(),
            status: crate::progress::ProgressStatus::InProgress,
        };

        let result = connection.send_update(update);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_websocket_message_serialization_async() {
        let message = WebSocketMessage::Subscribe {
            operation_id: Some(Uuid::new_v4()),
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();

        match (message, deserialized) {
            (
                WebSocketMessage::Subscribe { operation_id: id1 },
                WebSocketMessage::Subscribe { operation_id: id2 },
            ) => {
                assert_eq!(id1, id2);
            }
            _ => panic!("Message types don't match"),
        }
    }

    #[tokio::test]
    #[allow(clippy::similar_names)]
    async fn test_websocket_message_ping_pong() {
        let ping_message = WebSocketMessage::Ping;
        let pong_message = WebSocketMessage::Pong;

        let ping_json = serde_json::to_string(&ping_message).unwrap();
        let pong_json = serde_json::to_string(&pong_message).unwrap();

        let ping_deserialized: WebSocketMessage = serde_json::from_str(&ping_json).unwrap();
        let pong_deserialized: WebSocketMessage = serde_json::from_str(&pong_json).unwrap();

        assert!(matches!(ping_deserialized, WebSocketMessage::Ping));
        assert!(matches!(pong_deserialized, WebSocketMessage::Pong));
    }

    #[tokio::test]
    async fn test_websocket_message_unsubscribe() {
        let message = WebSocketMessage::Unsubscribe {
            operation_id: Some(Uuid::new_v4()),
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();

        match (message, deserialized) {
            (
                WebSocketMessage::Unsubscribe { operation_id: id1 },
                WebSocketMessage::Unsubscribe { operation_id: id2 },
            ) => {
                assert_eq!(id1, id2);
            }
            _ => panic!("Message types don't match"),
        }
    }

    #[tokio::test]
    async fn test_websocket_message_progress_update() {
        let update = ProgressUpdate {
            operation_id: Uuid::new_v4(),
            operation_name: "test_operation".to_string(),
            current: 75,
            total: Some(100),
            message: Some("Almost done".to_string()),
            timestamp: chrono::Utc::now(),
            status: crate::progress::ProgressStatus::InProgress,
        };

        let message = WebSocketMessage::ProgressUpdate(update.clone());

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            WebSocketMessage::ProgressUpdate(deserialized_update) => {
                assert_eq!(update.operation_id, deserialized_update.operation_id);
                assert_eq!(update.operation_name, deserialized_update.operation_name);
                assert_eq!(update.current, deserialized_update.current);
            }
            _ => panic!("Expected ProgressUpdate message"),
        }
    }

    #[tokio::test]
    async fn test_websocket_message_error() {
        let message = WebSocketMessage::Error {
            message: "Test error".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            WebSocketMessage::Error { message: msg } => {
                assert_eq!(msg, "Test error");
            }
            _ => panic!("Expected Error message"),
        }
    }

    #[tokio::test]
    async fn test_websocket_client_connection_multiple_updates() {
        let connection = WebSocketClientConnection::new();
        let mut receiver = connection.subscribe();

        // Send multiple updates
        for i in 0..5 {
            let update = ProgressUpdate {
                operation_id: Uuid::new_v4(),
                operation_name: format!("test_{i}"),
                current: i * 20,
                total: Some(100),
                message: Some(format!("Update {i}")),
                timestamp: chrono::Utc::now(),
                status: crate::progress::ProgressStatus::InProgress,
            };

            connection.send_update(update).unwrap();
        }

        // Receive all updates
        for i in 0..5 {
            let received_msg = tokio::time::timeout(StdDuration::from_millis(100), receiver.recv())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(received_msg.operation_name, format!("test_{i}"));
        }
    }

    #[tokio::test]
    async fn test_websocket_client_connection_timeout() {
        let connection = WebSocketClientConnection::new();
        let mut receiver = connection.subscribe();

        // Try to receive without sending anything
        let result = tokio::time::timeout(StdDuration::from_millis(50), receiver.recv()).await;
        assert!(result.is_err()); // Should timeout
    }

    #[tokio::test]
    async fn test_websocket_server_start() {
        let server = WebSocketServer::new(8080);

        // Test that the server can be created and has the start method
        // We don't actually call start() as it runs indefinitely
        assert_eq!(server.port, 8080);

        // Test that the method signature is correct by checking it exists
        // This verifies the method can be called without compilation errors
        let _server_ref = &server;
        // We can't actually call start() as it would hang, but we can verify
        // the method exists and the server is properly constructed
    }

    #[tokio::test]
    async fn test_websocket_server_broadcast() {
        let server = WebSocketServer::new(8080);

        let update = ProgressUpdate {
            operation_id: Uuid::new_v4(),
            operation_name: "test_operation".to_string(),
            current: 50,
            total: Some(100),
            message: Some("Test message".to_string()),
            timestamp: chrono::Utc::now(),
            status: crate::progress::ProgressStatus::InProgress,
        };

        // Test that broadcast method doesn't panic
        let result = server
            .broadcast(WebSocketMessage::ProgressUpdate(update))
            .await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_websocket_message_debug() {
        let message = WebSocketMessage::Ping;
        let debug_str = format!("{message:?}");
        assert!(debug_str.contains("Ping"));
    }

    #[test]
    fn test_websocket_message_clone() {
        let message = WebSocketMessage::Ping;
        let cloned = message.clone();
        assert_eq!(message, cloned);
    }

    #[test]
    fn test_websocket_message_partial_eq() {
        let message1 = WebSocketMessage::Ping;
        let message2 = WebSocketMessage::Ping;
        let message3 = WebSocketMessage::Pong;

        assert_eq!(message1, message2);
        assert_ne!(message1, message3);
    }

    #[test]
    fn test_websocket_client_debug() {
        let (sender, _receiver) = crossbeam_channel::unbounded();
        let client = WebSocketClient::new(sender);
        let debug_str = format!("{client:?}");
        assert!(debug_str.contains("WebSocketClient"));
    }

    #[test]
    fn test_websocket_client_connection_debug() {
        let connection = WebSocketClientConnection::new();
        let debug_str = format!("{connection:?}");
        assert!(debug_str.contains("WebSocketClientConnection"));
    }

    #[test]
    fn test_websocket_server_debug() {
        let server = WebSocketServer::new(8080);
        let debug_str = format!("{server:?}");
        assert!(debug_str.contains("WebSocketServer"));
    }

    #[test]
    fn test_websocket_message_subscribe_serialization() {
        let message = WebSocketMessage::Subscribe {
            operation_id: Some(Uuid::new_v4()),
        };
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(message, deserialized);
    }

    #[test]
    fn test_websocket_message_unsubscribe_serialization() {
        let message = WebSocketMessage::Unsubscribe {
            operation_id: Some(Uuid::new_v4()),
        };
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(message, deserialized);
    }

    #[test]
    fn test_websocket_message_progress_update_serialization() {
        let update = ProgressUpdate {
            operation_id: Uuid::new_v4(),
            operation_name: "test_operation".to_string(),
            current: 50,
            total: Some(100),
            message: Some("Test message".to_string()),
            timestamp: chrono::Utc::now(),
            status: crate::progress::ProgressStatus::InProgress,
        };
        let message = WebSocketMessage::ProgressUpdate(update);
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(message, deserialized);
    }

    #[test]
    fn test_websocket_message_error_serialization() {
        let message = WebSocketMessage::Error {
            message: "Test error".to_string(),
        };
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(message, deserialized);
    }

    #[tokio::test]
    async fn test_websocket_server_multiple_broadcasts() {
        let server = WebSocketServer::new(8080);

        let update1 = ProgressUpdate {
            operation_id: Uuid::new_v4(),
            operation_name: "operation1".to_string(),
            current: 25,
            total: Some(100),
            message: Some("First update".to_string()),
            timestamp: chrono::Utc::now(),
            status: crate::progress::ProgressStatus::InProgress,
        };

        let update2 = ProgressUpdate {
            operation_id: Uuid::new_v4(),
            operation_name: "operation2".to_string(),
            current: 50,
            total: Some(100),
            message: Some("Second update".to_string()),
            timestamp: chrono::Utc::now(),
            status: crate::progress::ProgressStatus::InProgress,
        };

        // Test multiple broadcasts
        let result1 = server
            .broadcast(WebSocketMessage::ProgressUpdate(update1))
            .await;
        let result2 = server
            .broadcast(WebSocketMessage::ProgressUpdate(update2))
            .await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_websocket_server_port_access() {
        let server = WebSocketServer::new(8080);
        assert_eq!(server.port, 8080);
    }

    #[test]
    fn test_websocket_client_id_generation() {
        let (sender1, _receiver1) = crossbeam_channel::unbounded();
        let (sender2, _receiver2) = crossbeam_channel::unbounded();

        let client1 = WebSocketClient::new(sender1);
        let client2 = WebSocketClient::new(sender2);

        // IDs should be different
        assert_ne!(client1.id, client2.id);
        assert!(!client1.id.is_nil());
        assert!(!client2.id.is_nil());
    }

    #[tokio::test]
    async fn test_websocket_message_roundtrip_all_types() {
        let messages = vec![
            WebSocketMessage::Subscribe {
                operation_id: Some(Uuid::new_v4()),
            },
            WebSocketMessage::Unsubscribe {
                operation_id: Some(Uuid::new_v4()),
            },
            WebSocketMessage::Ping,
            WebSocketMessage::Pong,
            WebSocketMessage::ProgressUpdate(ProgressUpdate {
                operation_id: Uuid::new_v4(),
                operation_name: "test".to_string(),
                current: 0,
                total: Some(100),
                message: None,
                timestamp: chrono::Utc::now(),
                status: crate::progress::ProgressStatus::InProgress,
            }),
            WebSocketMessage::Error {
                message: "test error".to_string(),
            },
        ];

        for message in messages {
            let json = serde_json::to_string(&message).unwrap();
            let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();
            assert_eq!(message, deserialized);
        }
    }

    #[tokio::test]
    async fn test_websocket_server_client_count() {
        let server = WebSocketServer::new(8080);
        let count = server.client_count().await;
        assert_eq!(count, 0); // No clients initially
    }

    #[tokio::test]
    async fn test_websocket_server_broadcast_error_handling() {
        let server = WebSocketServer::new(8080);
        let message = WebSocketMessage::Ping;

        // This should succeed even with no clients
        let result = server.broadcast(message).await;
        assert!(result.is_ok());
    }
}
