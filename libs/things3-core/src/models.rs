//! Data models for Things 3 entities

use chrono::{DateTime, NaiveDate, Utc};
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

/// How to handle child tasks when deleting a parent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteChildHandling {
    /// Return error if task has children (default)
    #[serde(rename = "error")]
    Error,
    /// Delete parent and all children
    #[serde(rename = "cascade")]
    Cascade,
    /// Delete parent only, orphan children
    #[serde(rename = "orphan")]
    Orphan,
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
    /// Completion timestamp (when status changed to completed)
    pub stop_date: Option<DateTime<Utc>>,
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
    /// Task title (required)
    pub title: String,
    /// Task type (defaults to Todo)
    pub task_type: Option<TaskType>,
    /// Optional notes
    pub notes: Option<String>,
    /// Start date
    pub start_date: Option<NaiveDate>,
    /// Deadline
    pub deadline: Option<NaiveDate>,
    /// Project UUID (validated if provided)
    pub project_uuid: Option<Uuid>,
    /// Area UUID (validated if provided)
    pub area_uuid: Option<Uuid>,
    /// Parent task UUID (for subtasks)
    pub parent_uuid: Option<Uuid>,
    /// Tags (as string names)
    pub tags: Option<Vec<String>>,
    /// Initial status (defaults to Incomplete)
    pub status: Option<TaskStatus>,
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
    /// New project UUID
    pub project_uuid: Option<Uuid>,
    /// New area UUID
    pub area_uuid: Option<Uuid>,
    /// New tags
    pub tags: Option<Vec<String>>,
}

/// Task filters for queries
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_task_status_serialization() {
        let status = TaskStatus::Incomplete;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"incomplete\"");

        let status = TaskStatus::Completed;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"completed\"");

        let status = TaskStatus::Canceled;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"canceled\"");

        let status = TaskStatus::Trashed;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"trashed\"");
    }

    #[test]
    fn test_task_status_deserialization() {
        let deserialized: TaskStatus = serde_json::from_str("\"incomplete\"").unwrap();
        assert_eq!(deserialized, TaskStatus::Incomplete);

        let deserialized: TaskStatus = serde_json::from_str("\"completed\"").unwrap();
        assert_eq!(deserialized, TaskStatus::Completed);

        let deserialized: TaskStatus = serde_json::from_str("\"canceled\"").unwrap();
        assert_eq!(deserialized, TaskStatus::Canceled);

        let deserialized: TaskStatus = serde_json::from_str("\"trashed\"").unwrap();
        assert_eq!(deserialized, TaskStatus::Trashed);
    }

    #[test]
    fn test_task_type_serialization() {
        let task_type = TaskType::Todo;
        let serialized = serde_json::to_string(&task_type).unwrap();
        assert_eq!(serialized, "\"to-do\"");

        let task_type = TaskType::Project;
        let serialized = serde_json::to_string(&task_type).unwrap();
        assert_eq!(serialized, "\"project\"");

        let task_type = TaskType::Heading;
        let serialized = serde_json::to_string(&task_type).unwrap();
        assert_eq!(serialized, "\"heading\"");

        let task_type = TaskType::Area;
        let serialized = serde_json::to_string(&task_type).unwrap();
        assert_eq!(serialized, "\"area\"");
    }

    #[test]
    fn test_task_type_deserialization() {
        let deserialized: TaskType = serde_json::from_str("\"to-do\"").unwrap();
        assert_eq!(deserialized, TaskType::Todo);

        let deserialized: TaskType = serde_json::from_str("\"project\"").unwrap();
        assert_eq!(deserialized, TaskType::Project);

        let deserialized: TaskType = serde_json::from_str("\"heading\"").unwrap();
        assert_eq!(deserialized, TaskType::Heading);

        let deserialized: TaskType = serde_json::from_str("\"area\"").unwrap();
        assert_eq!(deserialized, TaskType::Area);
    }

    #[test]
    fn test_task_creation() {
        let uuid = Uuid::new_v4();
        let now = Utc::now();
        let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let deadline = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

        let task = Task {
            uuid,
            title: "Test Task".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: Some("Test notes".to_string()),
            start_date: Some(start_date),
            deadline: Some(deadline),
            created: now,
            modified: now,
            stop_date: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: vec!["work".to_string(), "urgent".to_string()],
            children: vec![],
        };

        assert_eq!(task.uuid, uuid);
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.task_type, TaskType::Todo);
        assert_eq!(task.status, TaskStatus::Incomplete);
        assert_eq!(task.notes, Some("Test notes".to_string()));
        assert_eq!(task.start_date, Some(start_date));
        assert_eq!(task.deadline, Some(deadline));
        assert_eq!(task.tags.len(), 2);
        assert!(task.tags.contains(&"work".to_string()));
        assert!(task.tags.contains(&"urgent".to_string()));
    }

    #[test]
    fn test_task_serialization() {
        let uuid = Uuid::new_v4();
        let now = Utc::now();

        let task = Task {
            uuid,
            title: "Test Task".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: None,
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            stop_date: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: vec![],
            children: vec![],
        };

        let serialized = serde_json::to_string(&task).unwrap();
        let deserialized: Task = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.uuid, task.uuid);
        assert_eq!(deserialized.title, task.title);
        assert_eq!(deserialized.task_type, task.task_type);
        assert_eq!(deserialized.status, task.status);
    }

    #[test]
    fn test_project_creation() {
        let uuid = Uuid::new_v4();
        let area_uuid = Uuid::new_v4();
        let now = Utc::now();

        let project = Project {
            uuid,
            title: "Test Project".to_string(),
            notes: Some("Project notes".to_string()),
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            area_uuid: Some(area_uuid),
            tags: vec!["project".to_string()],
            status: TaskStatus::Incomplete,
            tasks: vec![],
        };

        assert_eq!(project.uuid, uuid);
        assert_eq!(project.title, "Test Project");
        assert_eq!(project.area_uuid, Some(area_uuid));
        assert_eq!(project.status, TaskStatus::Incomplete);
        assert_eq!(project.tags.len(), 1);
    }

    #[test]
    fn test_project_serialization() {
        let uuid = Uuid::new_v4();
        let now = Utc::now();

        let project = Project {
            uuid,
            title: "Test Project".to_string(),
            notes: None,
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            area_uuid: None,
            tags: vec![],
            status: TaskStatus::Incomplete,
            tasks: vec![],
        };

        let serialized = serde_json::to_string(&project).unwrap();
        let deserialized: Project = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.uuid, project.uuid);
        assert_eq!(deserialized.title, project.title);
        assert_eq!(deserialized.status, project.status);
    }

    #[test]
    fn test_area_creation() {
        let uuid = Uuid::new_v4();
        let now = Utc::now();

        let area = Area {
            uuid,
            title: "Test Area".to_string(),
            notes: Some("Area notes".to_string()),
            created: now,
            modified: now,
            tags: vec!["area".to_string()],
            projects: vec![],
        };

        assert_eq!(area.uuid, uuid);
        assert_eq!(area.title, "Test Area");
        assert_eq!(area.notes, Some("Area notes".to_string()));
        assert_eq!(area.tags.len(), 1);
    }

    #[test]
    fn test_area_serialization() {
        let uuid = Uuid::new_v4();
        let now = Utc::now();

        let area = Area {
            uuid,
            title: "Test Area".to_string(),
            notes: None,
            created: now,
            modified: now,
            tags: vec![],
            projects: vec![],
        };

        let serialized = serde_json::to_string(&area).unwrap();
        let deserialized: Area = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.uuid, area.uuid);
        assert_eq!(deserialized.title, area.title);
    }

    #[test]
    fn test_tag_creation() {
        let uuid = Uuid::new_v4();
        let task_uuid = Uuid::new_v4();

        let tag = Tag {
            uuid,
            title: "work".to_string(),
            usage_count: 5,
            tasks: vec![task_uuid],
        };

        assert_eq!(tag.uuid, uuid);
        assert_eq!(tag.title, "work");
        assert_eq!(tag.usage_count, 5);
        assert_eq!(tag.tasks.len(), 1);
        assert_eq!(tag.tasks[0], task_uuid);
    }

    #[test]
    fn test_tag_serialization() {
        let uuid = Uuid::new_v4();

        let tag = Tag {
            uuid,
            title: "test".to_string(),
            usage_count: 0,
            tasks: vec![],
        };

        let serialized = serde_json::to_string(&tag).unwrap();
        let deserialized: Tag = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.uuid, tag.uuid);
        assert_eq!(deserialized.title, tag.title);
        assert_eq!(deserialized.usage_count, tag.usage_count);
    }

    #[test]
    fn test_create_task_request() {
        let project_uuid = Uuid::new_v4();
        let area_uuid = Uuid::new_v4();
        let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let request = CreateTaskRequest {
            title: "New Task".to_string(),
            task_type: None,
            notes: Some("Task notes".to_string()),
            start_date: Some(start_date),
            deadline: None,
            project_uuid: Some(project_uuid),
            area_uuid: Some(area_uuid),
            parent_uuid: None,
            tags: Some(vec!["new".to_string()]),
            status: None,
        };

        assert_eq!(request.title, "New Task");
        assert_eq!(request.project_uuid, Some(project_uuid));
        assert_eq!(request.area_uuid, Some(area_uuid));
        assert_eq!(request.start_date, Some(start_date));
        assert_eq!(request.tags.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_create_task_request_serialization() {
        let request = CreateTaskRequest {
            title: "Test".to_string(),
            task_type: None,
            notes: None,
            start_date: None,
            deadline: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: None,
            status: None,
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: CreateTaskRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.title, request.title);
    }

    #[test]
    fn test_update_task_request() {
        let uuid = Uuid::new_v4();

        let request = UpdateTaskRequest {
            uuid,
            title: Some("Updated Title".to_string()),
            notes: Some("Updated notes".to_string()),
            start_date: None,
            deadline: None,
            status: Some(TaskStatus::Completed),
            project_uuid: None,
            area_uuid: None,
            tags: Some(vec!["updated".to_string()]),
        };

        assert_eq!(request.uuid, uuid);
        assert_eq!(request.title, Some("Updated Title".to_string()));
        assert_eq!(request.status, Some(TaskStatus::Completed));
        assert_eq!(request.tags, Some(vec!["updated".to_string()]));
    }

    #[test]
    fn test_update_task_request_serialization() {
        let uuid = Uuid::new_v4();

        let request = UpdateTaskRequest {
            uuid,
            title: None,
            notes: None,
            start_date: None,
            deadline: None,
            status: None,
            project_uuid: None,
            area_uuid: None,
            tags: None,
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: UpdateTaskRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.uuid, request.uuid);
    }

    #[test]
    fn test_task_filters_default() {
        let filters = TaskFilters::default();

        assert!(filters.status.is_none());
        assert!(filters.task_type.is_none());
        assert!(filters.project_uuid.is_none());
        assert!(filters.area_uuid.is_none());
        assert!(filters.tags.is_none());
        assert!(filters.start_date_from.is_none());
        assert!(filters.start_date_to.is_none());
        assert!(filters.deadline_from.is_none());
        assert!(filters.deadline_to.is_none());
        assert!(filters.search_query.is_none());
        assert!(filters.limit.is_none());
        assert!(filters.offset.is_none());
    }

    #[test]
    fn test_task_filters_creation() {
        let project_uuid = Uuid::new_v4();
        let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let filters = TaskFilters {
            status: Some(TaskStatus::Incomplete),
            task_type: Some(TaskType::Todo),
            project_uuid: Some(project_uuid),
            area_uuid: None,
            tags: Some(vec!["work".to_string()]),
            start_date_from: Some(start_date),
            start_date_to: None,
            deadline_from: None,
            deadline_to: None,
            search_query: Some("test".to_string()),
            limit: Some(10),
            offset: Some(0),
        };

        assert_eq!(filters.status, Some(TaskStatus::Incomplete));
        assert_eq!(filters.task_type, Some(TaskType::Todo));
        assert_eq!(filters.project_uuid, Some(project_uuid));
        assert_eq!(filters.search_query, Some("test".to_string()));
        assert_eq!(filters.limit, Some(10));
        assert_eq!(filters.offset, Some(0));
    }

    #[test]
    fn test_task_filters_serialization() {
        let filters = TaskFilters {
            status: Some(TaskStatus::Completed),
            task_type: Some(TaskType::Project),
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
        };

        let serialized = serde_json::to_string(&filters).unwrap();
        let deserialized: TaskFilters = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.status, filters.status);
        assert_eq!(deserialized.task_type, filters.task_type);
    }

    #[test]
    fn test_task_status_equality() {
        assert_eq!(TaskStatus::Incomplete, TaskStatus::Incomplete);
        assert_ne!(TaskStatus::Incomplete, TaskStatus::Completed);
        assert_ne!(TaskStatus::Completed, TaskStatus::Canceled);
        assert_ne!(TaskStatus::Canceled, TaskStatus::Trashed);
    }

    #[test]
    fn test_task_type_equality() {
        assert_eq!(TaskType::Todo, TaskType::Todo);
        assert_ne!(TaskType::Todo, TaskType::Project);
        assert_ne!(TaskType::Project, TaskType::Heading);
        assert_ne!(TaskType::Heading, TaskType::Area);
    }

    #[test]
    fn test_task_with_children() {
        let parent_uuid = Uuid::new_v4();
        let child_uuid = Uuid::new_v4();
        let now = Utc::now();

        let child_task = Task {
            uuid: child_uuid,
            title: "Child Task".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: None,
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            stop_date: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: Some(parent_uuid),
            tags: vec![],
            children: vec![],
        };

        let parent_task = Task {
            uuid: parent_uuid,
            title: "Parent Task".to_string(),
            task_type: TaskType::Heading,
            status: TaskStatus::Incomplete,
            notes: None,
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            stop_date: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: vec![],
            children: vec![child_task],
        };

        assert_eq!(parent_task.children.len(), 1);
        assert_eq!(parent_task.children[0].parent_uuid, Some(parent_uuid));
        assert_eq!(parent_task.children[0].title, "Child Task");
    }

    #[test]
    fn test_project_with_tasks() {
        let project_uuid = Uuid::new_v4();
        let task_uuid = Uuid::new_v4();
        let now = Utc::now();

        let task = Task {
            uuid: task_uuid,
            title: "Project Task".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: None,
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            stop_date: None,
            project_uuid: Some(project_uuid),
            area_uuid: None,
            parent_uuid: None,
            tags: vec![],
            children: vec![],
        };

        let project = Project {
            uuid: project_uuid,
            title: "Test Project".to_string(),
            notes: None,
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            area_uuid: None,
            tags: vec![],
            status: TaskStatus::Incomplete,
            tasks: vec![task],
        };

        assert_eq!(project.tasks.len(), 1);
        assert_eq!(project.tasks[0].project_uuid, Some(project_uuid));
        assert_eq!(project.tasks[0].title, "Project Task");
    }

    #[test]
    fn test_area_with_projects() {
        let area_uuid = Uuid::new_v4();
        let project_uuid = Uuid::new_v4();
        let now = Utc::now();

        let project = Project {
            uuid: project_uuid,
            title: "Area Project".to_string(),
            notes: None,
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            area_uuid: Some(area_uuid),
            tags: vec![],
            status: TaskStatus::Incomplete,
            tasks: vec![],
        };

        let area = Area {
            uuid: area_uuid,
            title: "Test Area".to_string(),
            notes: None,
            created: now,
            modified: now,
            tags: vec![],
            projects: vec![project],
        };

        assert_eq!(area.projects.len(), 1);
        assert_eq!(area.projects[0].area_uuid, Some(area_uuid));
        assert_eq!(area.projects[0].title, "Area Project");
    }
}
