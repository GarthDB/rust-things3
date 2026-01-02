//! SQL Query Builder utilities for type-safe query construction
//!
//! This module provides builder patterns for constructing SQL queries,
//! reducing the risk of SQL injection and making complex queries easier to maintain.

use crate::models::UpdateTaskRequest;

/// Builder for UPDATE queries on the TMTask table
///
/// Provides a type-safe API for building dynamic UPDATE statements
/// based on which fields are being updated.
#[derive(Debug, Clone)]
pub struct TaskUpdateBuilder {
    updates: Vec<String>,
}

impl TaskUpdateBuilder {
    /// Create a new TaskUpdateBuilder
    #[must_use]
    pub fn new() -> Self {
        Self {
            updates: Vec::new(),
        }
    }

    /// Create a builder from an `UpdateTaskRequest`
    ///
    /// Automatically marks all fields that are present in the request
    #[must_use]
    pub fn from_request(request: &UpdateTaskRequest) -> Self {
        let mut builder = Self::new();

        if request.title.is_some() {
            builder = builder.add_field("title");
        }

        if request.notes.is_some() {
            builder = builder.add_field("notes");
        }

        if request.start_date.is_some() {
            builder = builder.add_field("startDate");
        }

        if request.deadline.is_some() {
            builder = builder.add_field("deadline");
        }

        if request.status.is_some() {
            builder = builder.add_field("status");
        }

        if request.project_uuid.is_some() {
            builder = builder.add_field("project");
        }

        if request.area_uuid.is_some() {
            builder = builder.add_field("area");
        }

        if request.tags.is_some() {
            builder = builder.add_field("cachedTags");
        }

        builder
    }

    /// Add a field to the UPDATE list
    #[must_use]
    pub fn add_field(mut self, field_name: &str) -> Self {
        self.updates.push(format!("{field_name} = ?"));
        self
    }

    /// Check if any fields have been set
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.updates.is_empty()
    }

    /// Get the number of fields being updated
    #[must_use]
    pub fn len(&self) -> usize {
        self.updates.len()
    }

    /// Build the complete UPDATE query string
    ///
    /// Always includes userModificationDate update
    #[must_use]
    pub fn build_query_string(&self) -> String {
        if self.updates.is_empty() {
            // Even with no fields, still update modification date for consistency
            return "UPDATE TMTask SET userModificationDate = ? WHERE uuid = ?".to_string();
        }

        let mut all_updates = self.updates.clone();
        all_updates.push("userModificationDate = ?".to_string());
        format!(
            "UPDATE TMTask SET {} WHERE uuid = ?",
            all_updates.join(", ")
        )
    }

    /// Get the field names being updated (for validation and logging)
    #[must_use]
    pub fn fields(&self) -> Vec<String> {
        self.updates
            .iter()
            .map(|u| u.split(" = ").next().unwrap_or("").to_string())
            .collect()
    }
}

impl Default for TaskUpdateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TaskStatus;
    use chrono::NaiveDate;
    use uuid::Uuid;

    #[test]
    fn test_task_update_builder_empty() {
        let builder = TaskUpdateBuilder::new();
        assert!(builder.is_empty());
        assert_eq!(builder.len(), 0);
        // Empty builder should still generate a valid query (update modification date only)
        let query = builder.build_query_string();
        assert!(query.contains("userModificationDate = ?"));
    }

    #[test]
    fn test_task_update_builder_single_field() {
        let builder = TaskUpdateBuilder::new().add_field("title");
        assert!(!builder.is_empty());
        assert_eq!(builder.len(), 1);
        let query = builder.build_query_string();
        assert!(query.contains("title = ?"));
        assert!(query.contains("userModificationDate = ?"));
    }

    #[test]
    fn test_task_update_builder_multiple_fields() {
        let builder = TaskUpdateBuilder::new()
            .add_field("title")
            .add_field("notes")
            .add_field("status");
        assert_eq!(builder.len(), 3);
        let query = builder.build_query_string();
        assert!(query.contains("title = ?"));
        assert!(query.contains("notes = ?"));
        assert!(query.contains("status = ?"));
    }

    #[test]
    fn test_task_update_builder_from_request() {
        let request = UpdateTaskRequest {
            uuid: Uuid::new_v4(),
            title: Some("Updated Title".to_string()),
            notes: Some("Updated Notes".to_string()),
            start_date: Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()),
            deadline: Some(NaiveDate::from_ymd_opt(2025, 2, 1).unwrap()),
            status: Some(TaskStatus::Incomplete),
            project_uuid: Some(Uuid::new_v4()),
            area_uuid: Some(Uuid::new_v4()),
            tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
        };

        let builder = TaskUpdateBuilder::from_request(&request);
        assert_eq!(builder.len(), 8);

        let query = builder.build_query_string();
        assert!(query.contains("title = ?"));
        assert!(query.contains("notes = ?"));
        assert!(query.contains("startDate = ?"));
        assert!(query.contains("deadline = ?"));
        assert!(query.contains("status = ?"));
        assert!(query.contains("project = ?"));
        assert!(query.contains("area = ?"));
        assert!(query.contains("cachedTags = ?"));
    }

    #[test]
    fn test_task_update_builder_from_partial_request() {
        let request = UpdateTaskRequest {
            uuid: Uuid::new_v4(),
            title: Some("Updated Title".to_string()),
            notes: None,
            start_date: None,
            deadline: None,
            status: None,
            project_uuid: None,
            area_uuid: None,
            tags: None,
        };

        let builder = TaskUpdateBuilder::from_request(&request);
        assert_eq!(builder.len(), 1);

        let query = builder.build_query_string();
        assert!(query.contains("title = ?"));
        assert!(!query.contains("notes = ?"));
        assert!(!query.contains("status = ?"));
    }

    #[test]
    fn test_task_update_builder_fields() {
        let builder = TaskUpdateBuilder::new()
            .add_field("title")
            .add_field("status");
        let fields = builder.fields();
        assert_eq!(fields.len(), 2);
        assert!(fields.contains(&"title".to_string()));
        assert!(fields.contains(&"status".to_string()));
    }
}
