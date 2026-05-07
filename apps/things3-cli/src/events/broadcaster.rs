use std::collections::HashMap;
use std::sync::Arc;
use things3_core::{Result, ThingsId};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::progress::ProgressUpdate;

use super::filter::{EventFilter, EventSubscription};
use super::types::{Event, EventType};

/// Event broadcaster for managing and broadcasting events
pub struct EventBroadcaster {
    pub(super) sender: broadcast::Sender<Event>,
    pub(super) subscriptions: Arc<RwLock<HashMap<Uuid, EventSubscription>>>,
}

impl EventBroadcaster {
    /// Create a new event broadcaster
    #[must_use]
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            sender,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe to events with a filter
    pub async fn subscribe(&self, filter: EventFilter) -> broadcast::Receiver<Event> {
        let subscription_id = Uuid::new_v4();
        let (sub_sender, receiver) = broadcast::channel(100);

        let subscription = EventSubscription {
            id: subscription_id,
            filter,
            sender: sub_sender,
        };

        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions.insert(subscription_id, subscription);
        }

        receiver
    }

    /// Unsubscribe from events
    pub async fn unsubscribe(&self, subscription_id: Uuid) {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.remove(&subscription_id);
    }

    /// Broadcast an event
    ///
    /// # Errors
    /// Returns an error if broadcasting fails
    pub async fn broadcast(&self, event: Event) -> Result<()> {
        // Send to main channel (ignore if no receivers)
        let _ = self.sender.send(event.clone());

        // Send to filtered subscriptions
        let subscriptions = self.subscriptions.read().await;
        for subscription in subscriptions.values() {
            if subscription.filter.matches(&event) {
                let _ = subscription.sender.send(event.clone());
            }
        }

        Ok(())
    }

    /// Create and broadcast a task event
    ///
    /// # Errors
    /// Returns an error if broadcasting fails
    pub async fn broadcast_task_event(
        &self,
        event_type: EventType,
        _task_id: ThingsId,
        data: Option<serde_json::Value>,
        source: &str,
    ) -> Result<()> {
        let event = Event {
            id: Uuid::new_v4(),
            event_type,
            timestamp: chrono::Utc::now(),
            data,
            source: source.to_string(),
        };

        self.broadcast(event).await
    }

    /// Create and broadcast a project event
    ///
    /// # Errors
    /// Returns an error if broadcasting fails
    pub async fn broadcast_project_event(
        &self,
        event_type: EventType,
        _project_id: ThingsId,
        data: Option<serde_json::Value>,
        source: &str,
    ) -> Result<()> {
        let event = Event {
            id: Uuid::new_v4(),
            event_type,
            timestamp: chrono::Utc::now(),
            data,
            source: source.to_string(),
        };

        self.broadcast(event).await
    }

    /// Create and broadcast an area event
    ///
    /// # Errors
    /// Returns an error if broadcasting fails
    pub async fn broadcast_area_event(
        &self,
        event_type: EventType,
        _area_id: ThingsId,
        data: Option<serde_json::Value>,
        source: &str,
    ) -> Result<()> {
        let event = Event {
            id: Uuid::new_v4(),
            event_type,
            timestamp: chrono::Utc::now(),
            data,
            source: source.to_string(),
        };

        self.broadcast(event).await
    }

    /// Create and broadcast a progress event
    ///
    /// # Errors
    /// Returns an error if broadcasting fails
    pub async fn broadcast_progress_event(
        &self,
        event_type: EventType,
        _operation_id: Uuid,
        data: Option<serde_json::Value>,
        source: &str,
    ) -> Result<()> {
        let event = Event {
            id: Uuid::new_v4(),
            event_type,
            timestamp: chrono::Utc::now(),
            data,
            source: source.to_string(),
        };

        self.broadcast(event).await
    }

    /// Convert a progress update to an event
    ///
    /// # Errors
    /// Returns an error if broadcasting fails
    pub async fn broadcast_progress_update(
        &self,
        update: ProgressUpdate,
        source: &str,
    ) -> Result<()> {
        let event_type = match update.status {
            crate::progress::ProgressStatus::Started => EventType::ProgressStarted {
                operation_id: update.operation_id,
            },
            crate::progress::ProgressStatus::InProgress => EventType::ProgressUpdated {
                operation_id: update.operation_id,
            },
            crate::progress::ProgressStatus::Completed => EventType::ProgressCompleted {
                operation_id: update.operation_id,
            },
            crate::progress::ProgressStatus::Failed
            | crate::progress::ProgressStatus::Cancelled => EventType::ProgressFailed {
                operation_id: update.operation_id,
            },
        };

        let data = serde_json::to_value(&update)?;
        self.broadcast_progress_event(event_type, update.operation_id, Some(data), source)
            .await
    }

    /// Get the number of active subscriptions
    pub async fn subscription_count(&self) -> usize {
        self.subscriptions.read().await.len()
    }

    /// Get a receiver for all events (unfiltered)
    #[must_use]
    pub fn subscribe_all(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}
