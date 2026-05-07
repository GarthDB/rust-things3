use std::sync::Arc;
use things3_core::ThingsId;
use tokio::sync::broadcast;
use uuid::Uuid;

use super::broadcaster::EventBroadcaster;
use super::filter::EventFilter;
use super::types::{Event, EventType};

/// Event listener for handling events
pub struct EventListener {
    broadcaster: Arc<EventBroadcaster>,
    #[allow(dead_code)]
    subscriptions: Vec<Uuid>,
}

impl EventListener {
    /// Create a new event listener
    #[must_use]
    pub fn new(broadcaster: Arc<EventBroadcaster>) -> Self {
        Self {
            broadcaster,
            subscriptions: Vec::new(),
        }
    }

    /// Subscribe to specific event types
    pub async fn subscribe_to_events(
        &mut self,
        event_types: Vec<EventType>,
    ) -> broadcast::Receiver<Event> {
        let filter = EventFilter {
            event_types: Some(event_types),
            entity_ids: None,
            sources: None,
            since: None,
        };

        self.broadcaster.subscribe(filter).await
    }

    /// Subscribe to events for a specific entity
    pub async fn subscribe_to_entity(&mut self, entity_id: ThingsId) -> broadcast::Receiver<Event> {
        let filter = EventFilter {
            event_types: None,
            entity_ids: Some(vec![entity_id]),
            sources: None,
            since: None,
        };

        self.broadcaster.subscribe(filter).await
    }

    /// Subscribe to all events
    #[must_use]
    pub fn subscribe_to_all(&self) -> broadcast::Receiver<Event> {
        self.broadcaster.subscribe_all()
    }
}
