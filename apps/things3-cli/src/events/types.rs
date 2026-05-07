use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use things3_core::ThingsId;
use uuid::Uuid;

/// Event types for Things 3 entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event_type")]
pub enum EventType {
    /// Task events
    TaskCreated {
        task_id: ThingsId,
    },
    TaskUpdated {
        task_id: ThingsId,
    },
    TaskDeleted {
        task_id: ThingsId,
    },
    TaskCompleted {
        task_id: ThingsId,
    },
    TaskCancelled {
        task_id: ThingsId,
    },

    /// Project events
    ProjectCreated {
        project_id: ThingsId,
    },
    ProjectUpdated {
        project_id: ThingsId,
    },
    ProjectDeleted {
        project_id: ThingsId,
    },
    ProjectCompleted {
        project_id: ThingsId,
    },

    /// Area events
    AreaCreated {
        area_id: ThingsId,
    },
    AreaUpdated {
        area_id: ThingsId,
    },
    AreaDeleted {
        area_id: ThingsId,
    },

    /// Progress events
    ProgressStarted {
        operation_id: Uuid,
    },
    ProgressUpdated {
        operation_id: Uuid,
    },
    ProgressCompleted {
        operation_id: Uuid,
    },
    ProgressFailed {
        operation_id: Uuid,
    },
}

/// Event data structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: Uuid,
    pub event_type: EventType,
    pub timestamp: DateTime<Utc>,
    pub data: Option<serde_json::Value>,
    pub source: String,
}
