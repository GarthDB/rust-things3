# Data Models

This document describes the core data structures used in the Rust Things library.

## Overview

The library provides strongly-typed data models for all Things 3 entities, with comprehensive serialization support and validation.

## Core Types

### Task

The main task entity representing individual tasks, projects, headings, and areas.

```rust
use things_core::{Task, TaskType, TaskStatus, Priority, RecurrenceRule, ChecklistItem};
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;
use std::collections::HashMap;
use serde_json::Value;

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
    /// Parent task UUID (for headings)
    pub parent_uuid: Option<Uuid>,
    /// Associated tags
    pub tags: Vec<Tag>,
    /// Checklist items
    pub checklist_items: Vec<ChecklistItem>,
    /// Child tasks (for projects and headings)
    pub children: Vec<Task>,
    /// Recurrence information
    pub recurrence: Option<RecurrenceRule>,
    /// Priority level
    pub priority: Priority,
    /// Completion percentage (0-100)
    pub completion_percentage: u8,
    /// Estimated duration
    pub estimated_duration: Option<Duration>,
    /// Actual time spent
    pub time_spent: Option<Duration>,
    /// Custom metadata
    pub metadata: HashMap<String, Value>,
}
```

### TaskType

Enumeration of task types in Things 3.

```rust
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
```

### TaskStatus

Enumeration of task statuses.

```rust
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
```

### Priority

Task priority levels.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}
```

### Project

Project entity for organizing tasks.

```rust
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
    pub tags: Vec<Tag>,
    /// Project status
    pub status: TaskStatus,
    /// Child tasks
    pub tasks: Vec<Task>,
    /// Project progress (0-100)
    pub progress: u8,
    /// Project priority
    pub priority: Priority,
    /// Project color (for UI)
    pub color: Option<Color>,
    /// Project icon (for UI)
    pub icon: Option<String>,
    /// Custom metadata
    pub metadata: HashMap<String, Value>,
}
```

### Area

Area entity for organizing projects.

```rust
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
    pub tags: Vec<Tag>,
    /// Child projects
    pub projects: Vec<Project>,
    /// Area color (for UI)
    pub color: Option<Color>,
    /// Area icon (for UI)
    pub icon: Option<String>,
    /// Visibility status
    pub visible: bool,
    /// Sort order
    pub index: i32,
    /// Custom metadata
    pub metadata: HashMap<String, Value>,
}
```

### Tag

Tag entity for categorizing tasks and projects.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// Unique identifier
    pub uuid: Uuid,
    /// Tag title
    pub title: String,
    /// Tag color (for UI)
    pub color: Option<Color>,
    /// Usage count
    pub usage_count: u32,
    /// Last used timestamp
    pub last_used: Option<DateTime<Utc>>,
    /// Parent tag (for hierarchical tags)
    pub parent_uuid: Option<Uuid>,
    /// Child tags
    pub children: Vec<Tag>,
    /// Sort order
    pub index: i32,
    /// Custom metadata
    pub metadata: HashMap<String, Value>,
}
```

### ChecklistItem

Checklist item for task breakdown.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    /// Unique identifier
    pub uuid: Uuid,
    /// Item text
    pub title: String,
    /// Completion status
    pub completed: bool,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// Completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
    /// Sort order
    pub index: u32,
}
```

### RecurrenceRule

Recurrence rule for repeating tasks.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurrenceRule {
    /// Recurrence frequency
    pub frequency: RecurrenceFrequency,
    /// Interval (every N days/weeks/months)
    pub interval: u32,
    /// Days of week (for weekly recurrence)
    pub days_of_week: Option<Vec<Weekday>>,
    /// Days of month (for monthly recurrence)
    pub days_of_month: Option<Vec<u8>>,
    /// End date for recurrence
    pub end_date: Option<NaiveDate>,
    /// Maximum occurrences
    pub max_occurrences: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}
```

### Color

Color representation for UI elements.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}
```

## Request/Response Types

### CreateTaskRequest

Request for creating a new task.

```rust
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
    /// Priority level
    pub priority: Option<Priority>,
    /// Recurrence rule
    pub recurrence: Option<RecurrenceRule>,
}
```

### UpdateTaskRequest

Request for updating an existing task.

```rust
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
    /// New priority
    pub priority: Option<Priority>,
    /// New recurrence rule
    pub recurrence: Option<RecurrenceRule>,
}
```

### TaskFilters

Filters for querying tasks.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
```

## Serialization

All data models support comprehensive serialization:

```rust
use things_core::{Task, SerializationFormat, SerializationManager};

// Serialize to JSON
let task_json = serde_json::to_string(&task)?;

// Serialize to MessagePack
let task_msgpack = rmp_serde::to_vec(&task)?;

// Serialize to Bincode
let task_bincode = bincode::serialize(&task)?;

// Using the serialization manager
let manager = SerializationManager::new(SerializationFormat::Json)?;
let task_data = manager.serialize(&task)?;
```

## Validation

Data models include validation methods:

```rust
impl Task {
    /// Validate task data
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.title.is_empty() {
            return Err(ValidationError::new("Title cannot be empty"));
        }
        
        if let Some(deadline) = self.deadline {
            if let Some(start_date) = self.start_date {
                if deadline < start_date {
                    return Err(ValidationError::new("Deadline cannot be before start date"));
                }
            }
        }
        
        if self.completion_percentage > 100 {
            return Err(ValidationError::new("Completion percentage cannot exceed 100"));
        }
        
        Ok(())
    }
}
```

## Examples

### Creating a Task

```rust
use things_core::{CreateTaskRequest, Priority};
use chrono::Utc;

let create_request = CreateTaskRequest {
    title: "Learn Rust".to_string(),
    notes: Some("Study the Rust programming language".to_string()),
    start_date: Some(Utc::now().date_naive()),
    deadline: None,
    project_uuid: None,
    area_uuid: None,
    tags: vec!["learning".to_string(), "programming".to_string()],
    priority: Some(Priority::High),
    recurrence: None,
};

let task = db.create_task(&create_request).await?;
```

### Querying Tasks

```rust
use things_core::{TaskFilters, TaskStatus, TaskType};

let filters = TaskFilters {
    status: Some(TaskStatus::Incomplete),
    task_type: Some(TaskType::Todo),
    tags: Some(vec!["urgent".to_string()]),
    start_date_from: Some(Utc::now().date_naive()),
    limit: Some(20),
    ..Default::default()
};

let tasks = db.get_tasks_filtered(&filters).await?;
```

### Updating a Task

```rust
use things_core::{UpdateTaskRequest, TaskStatus, Priority};

let update_request = UpdateTaskRequest {
    uuid: task.uuid,
    title: Some("Learn Rust and WebAssembly".to_string()),
    status: Some(TaskStatus::Completed),
    priority: Some(Priority::Normal),
    ..Default::default()
};

let updated_task = db.update_task(&update_request).await?;
```
