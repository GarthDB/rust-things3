//! Data models for Things 3 entities

use chrono::{DateTime, Utc, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Task status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    #[serde(rename = "incomplete")]
    Incomplete,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "canceled")]
    Canceled,
    #[serde(rename = "trashed")]
    Trashed,
}

/// Task type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    #[serde(rename = "to-do")]
    Todo,
    #[serde(rename = "project")]
    Project,
    #[serde(rename = "heading")]
    Heading,
    #[serde(rename = "area")]
    Area,
}

/// Main task entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier
    pub uuid: Uuid,
    /// Task title
    pub title: String,
    /// Task type
    pub task_type: TaskType,
    /// Task status
    pub status: TaskStatus,
    /// Optional notes
    pub notes: Option<String>,
    /// Start date
    pub start_date: Option<NaiveDate>,
    /// Deadline
    pub deadline: Option<NaiveDate>,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// Last modification timestamp
    pub modified: DateTime<Utc>,
    /// Parent project UUID
    pub project_uuid: Option<Uuid>,
    /// Parent area UUID
    pub area_uuid: Option<Uuid>,
    /// Parent task UUID
    pub parent_uuid: Option<Uuid>,
    /// Associated tags
    pub tags: Vec<String>,
    /// Child tasks
    pub children: Vec<Task>,
}

/// Project entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique identifier
    pub uuid: Uuid,
    /// Project title
    pub title: String,
    /// Optional notes
    pub notes: Option<String>,
    /// Start date
    pub start_date: Option<NaiveDate>,
    /// Deadline
    pub deadline: Option<NaiveDate>,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// Last modification timestamp
    pub modified: DateTime<Utc>,
    /// Parent area UUID
    pub area_uuid: Option<Uuid>,
    /// Associated tags
    pub tags: Vec<String>,
    /// Project status
    pub status: TaskStatus,
    /// Child tasks
    pub tasks: Vec<Task>,
}

/// Area entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    /// Unique identifier
    pub uuid: Uuid,
    /// Area title
    pub title: String,
    /// Optional notes
    pub notes: Option<String>,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// Last modification timestamp
    pub modified: DateTime<Utc>,
    /// Associated tags
    pub tags: Vec<String>,
    /// Child projects
    pub projects: Vec<Project>,
}

/// Tag entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// Unique identifier
    pub uuid: Uuid,
    /// Tag title
    pub title: String,
    /// Usage count
    pub usage_count: u32,
    /// Associated tasks
    pub tasks: Vec<Uuid>,
}

/// Task creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    /// Task title
    pub title: String,
    /// Optional notes
    pub notes: Option<String>,
    /// Start date
    pub start_date: Option<NaiveDate>,
    /// Deadline
    pub deadline: Option<NaiveDate>,
    /// Parent project UUID
    pub project_uuid: Option<Uuid>,
    /// Parent area UUID
    pub area_uuid: Option<Uuid>,
    /// Associated tags
    pub tags: Vec<String>,
}

/// Task update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskRequest {
    /// Task UUID
    pub uuid: Uuid,
    /// New title
    pub title: Option<String>,
    /// New notes
    pub notes: Option<String>,
    /// New start date
    pub start_date: Option<NaiveDate>,
    /// New deadline
    pub deadline: Option<NaiveDate>,
    /// New status
    pub status: Option<TaskStatus>,
    /// New tags
    pub tags: Option<Vec<String>>,
}

/// Task filters for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFilters {
    /// Filter by status
    pub status: Option<TaskStatus>,
    /// Filter by task type
    pub task_type: Option<TaskType>,
    /// Filter by project UUID
    pub project_uuid: Option<Uuid>,
    /// Filter by area UUID
    pub area_uuid: Option<Uuid>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Filter by start date range
    pub start_date_from: Option<NaiveDate>,
    pub start_date_to: Option<NaiveDate>,
    /// Filter by deadline range
    pub deadline_from: Option<NaiveDate>,
    pub deadline_to: Option<NaiveDate>,
    /// Search query
    pub search_query: Option<String>,
    /// Limit results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl Default for TaskFilters {
    fn default() -> Self {
        Self {
            status: None,
            task_type: None,
            project_uuid: None,
            area_uuid: None,
            tags: None,
            start_date_from: None,
            start_date_to: None,
            deadline_from: None,
            deadline_to: None,
            search_query: None,
            limit: None,
            offset: None,
        }
    }
}
