use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use things3_core::ThingsId;
use tokio::sync::broadcast;
use uuid::Uuid;

use super::types::{Event, EventType};

/// Event filter for subscriptions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventFilter {
    pub event_types: Option<Vec<EventType>>,
    pub entity_ids: Option<Vec<ThingsId>>,
    pub sources: Option<Vec<String>>,
    pub since: Option<DateTime<Utc>>,
}

impl EventFilter {
    /// Check if an event matches this filter
    #[must_use]
    pub fn matches(&self, event: &Event) -> bool {
        // Check event types
        if let Some(ref types) = self.event_types {
            if !types
                .iter()
                .any(|t| std::mem::discriminant(t) == std::mem::discriminant(&event.event_type))
            {
                return false;
            }
        }

        // Check entity IDs (applies to task/project/area events; progress events have no entity ID)
        if let Some(ref ids) = self.entity_ids {
            let event_entity_id: Option<&ThingsId> = match &event.event_type {
                EventType::TaskCreated { task_id }
                | EventType::TaskUpdated { task_id }
                | EventType::TaskDeleted { task_id }
                | EventType::TaskCompleted { task_id }
                | EventType::TaskCancelled { task_id } => Some(task_id),
                EventType::ProjectCreated { project_id }
                | EventType::ProjectUpdated { project_id }
                | EventType::ProjectDeleted { project_id }
                | EventType::ProjectCompleted { project_id } => Some(project_id),
                EventType::AreaCreated { area_id }
                | EventType::AreaUpdated { area_id }
                | EventType::AreaDeleted { area_id } => Some(area_id),
                EventType::ProgressStarted { .. }
                | EventType::ProgressUpdated { .. }
                | EventType::ProgressCompleted { .. }
                | EventType::ProgressFailed { .. } => None,
            };

            if let Some(entity_id) = event_entity_id {
                if !ids.contains(entity_id) {
                    return false;
                }
            }
        }

        // Check sources
        if let Some(ref sources) = self.sources {
            if !sources.contains(&event.source) {
                return false;
            }
        }

        // Check timestamp
        if let Some(since) = self.since {
            if event.timestamp < since {
                return false;
            }
        }

        true
    }
}

/// Event subscription
#[derive(Debug, Clone)]
pub struct EventSubscription {
    pub id: Uuid,
    pub filter: EventFilter,
    pub sender: broadcast::Sender<Event>,
}
