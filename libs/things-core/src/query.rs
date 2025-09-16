//! Query builder and filtering utilities

use crate::models::*;
use uuid::Uuid;
use chrono::NaiveDate;

/// Query builder for complex task queries
pub struct TaskQueryBuilder {
    filters: TaskFilters,
}

impl TaskQueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            filters: TaskFilters::default(),
        }
    }

    /// Filter by status
    pub fn status(mut self, status: TaskStatus) -> Self {
        self.filters.status = Some(status);
        self
    }

    /// Filter by task type
    pub fn task_type(mut self, task_type: TaskType) -> Self {
        self.filters.task_type = Some(task_type);
        self
    }

    /// Filter by project UUID
    pub fn project_uuid(mut self, project_uuid: Uuid) -> Self {
        self.filters.project_uuid = Some(project_uuid);
        self
    }

    /// Filter by area UUID
    pub fn area_uuid(mut self, area_uuid: Uuid) -> Self {
        self.filters.area_uuid = Some(area_uuid);
        self
    }

    /// Filter by tags
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.filters.tags = Some(tags);
        self
    }

    /// Filter by start date range
    pub fn start_date_range(mut self, from: NaiveDate, to: NaiveDate) -> Self {
        self.filters.start_date_from = Some(from);
        self.filters.start_date_to = Some(to);
        self
    }

    /// Filter by deadline range
    pub fn deadline_range(mut self, from: NaiveDate, to: NaiveDate) -> Self {
        self.filters.deadline_from = Some(from);
        self.filters.deadline_to = Some(to);
        self
    }

    /// Add search query
    pub fn search(mut self, query: String) -> Self {
        self.filters.search_query = Some(query);
        self
    }

    /// Set limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.filters.limit = Some(limit);
        self
    }

    /// Set offset for pagination
    pub fn offset(mut self, offset: usize) -> Self {
        self.filters.offset = Some(offset);
        self
    }

    /// Build the final filters
    pub fn build(self) -> TaskFilters {
        self.filters
    }
}

impl Default for TaskQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}
