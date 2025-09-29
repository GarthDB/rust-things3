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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}
