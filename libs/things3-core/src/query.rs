//! Query builder for filtering and searching tasks

use crate::models::{TaskFilters, TaskStatus, TaskType};
use chrono::NaiveDate;
use uuid::Uuid;

/// Builder for constructing task queries with filters
#[derive(Debug, Clone)]
pub struct TaskQueryBuilder {
    filters: TaskFilters,
}

impl TaskQueryBuilder {
    /// Create a new query builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            filters: TaskFilters::default(),
        }
    }

    /// Filter by status
    #[must_use]
    pub const fn status(mut self, status: TaskStatus) -> Self {
        self.filters.status = Some(status);
        self
    }

    /// Filter by task type
    #[must_use]
    pub const fn task_type(mut self, task_type: TaskType) -> Self {
        self.filters.task_type = Some(task_type);
        self
    }

    /// Filter by project UUID
    #[must_use]
    pub const fn project_uuid(mut self, project_uuid: Uuid) -> Self {
        self.filters.project_uuid = Some(project_uuid);
        self
    }

    /// Filter by area UUID
    #[must_use]
    pub const fn area_uuid(mut self, area_uuid: Uuid) -> Self {
        self.filters.area_uuid = Some(area_uuid);
        self
    }

    /// Filter by tags
    #[must_use]
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.filters.tags = Some(tags);
        self
    }

    /// Filter by start date range
    #[must_use]
    pub const fn start_date_range(
        mut self,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Self {
        self.filters.start_date_from = from;
        self.filters.start_date_to = to;
        self
    }

    /// Filter by deadline range
    #[must_use]
    pub const fn deadline_range(mut self, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Self {
        self.filters.deadline_from = from;
        self.filters.deadline_to = to;
        self
    }

    /// Add search query
    #[must_use]
    pub fn search(mut self, query: &str) -> Self {
        self.filters.search_query = Some(query.to_string());
        self
    }

    /// Set limit
    #[must_use]
    pub const fn limit(mut self, limit: usize) -> Self {
        self.filters.limit = Some(limit);
        self
    }

    /// Set offset for pagination
    #[must_use]
    pub const fn offset(mut self, offset: usize) -> Self {
        self.filters.offset = Some(offset);
        self
    }

    /// Build the final filters
    #[must_use]
    pub fn build(self) -> TaskFilters {
        self.filters
    }
}

impl Default for TaskQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use uuid::Uuid;

    #[test]
    fn test_task_query_builder_new() {
        let builder = TaskQueryBuilder::new();
        let filters = builder.build();

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
    fn test_task_query_builder_default() {
        let builder = TaskQueryBuilder::default();
        let filters = builder.build();

        assert!(filters.status.is_none());
        assert!(filters.task_type.is_none());
    }

    #[test]
    fn test_task_query_builder_status() {
        let builder = TaskQueryBuilder::new().status(TaskStatus::Completed);
        let filters = builder.build();

        assert_eq!(filters.status, Some(TaskStatus::Completed));
    }

    #[test]
    fn test_task_query_builder_task_type() {
        let builder = TaskQueryBuilder::new().task_type(TaskType::Project);
        let filters = builder.build();

        assert_eq!(filters.task_type, Some(TaskType::Project));
    }

    #[test]
    fn test_task_query_builder_project_uuid() {
        let uuid = Uuid::new_v4();
        let builder = TaskQueryBuilder::new().project_uuid(uuid);
        let filters = builder.build();

        assert_eq!(filters.project_uuid, Some(uuid));
    }

    #[test]
    fn test_task_query_builder_area_uuid() {
        let uuid = Uuid::new_v4();
        let builder = TaskQueryBuilder::new().area_uuid(uuid);
        let filters = builder.build();

        assert_eq!(filters.area_uuid, Some(uuid));
    }

    #[test]
    fn test_task_query_builder_tags() {
        let tags = vec!["urgent".to_string(), "important".to_string()];
        let builder = TaskQueryBuilder::new().tags(tags.clone());
        let filters = builder.build();

        assert_eq!(filters.tags, Some(tags));
    }

    #[test]
    fn test_task_query_builder_start_date_range() {
        let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let builder = TaskQueryBuilder::new().start_date_range(Some(from), Some(to));
        let filters = builder.build();

        assert_eq!(filters.start_date_from, Some(from));
        assert_eq!(filters.start_date_to, Some(to));
    }

    #[test]
    fn test_task_query_builder_deadline_range() {
        let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let builder = TaskQueryBuilder::new().deadline_range(Some(from), Some(to));
        let filters = builder.build();

        assert_eq!(filters.deadline_from, Some(from));
        assert_eq!(filters.deadline_to, Some(to));
    }

    #[test]
    fn test_task_query_builder_search() {
        let query = "test search";
        let builder = TaskQueryBuilder::new().search(query);
        let filters = builder.build();

        assert_eq!(filters.search_query, Some(query.to_string()));
    }

    #[test]
    fn test_task_query_builder_limit() {
        let builder = TaskQueryBuilder::new().limit(50);
        let filters = builder.build();

        assert_eq!(filters.limit, Some(50));
    }

    #[test]
    fn test_task_query_builder_offset() {
        let builder = TaskQueryBuilder::new().offset(10);
        let filters = builder.build();

        assert_eq!(filters.offset, Some(10));
    }

    #[test]
    fn test_task_query_builder_chaining() {
        let uuid = Uuid::new_v4();
        let tags = vec!["urgent".to_string()];
        let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

        let builder = TaskQueryBuilder::new()
            .status(TaskStatus::Incomplete)
            .task_type(TaskType::Todo)
            .project_uuid(uuid)
            .tags(tags.clone())
            .start_date_range(Some(from), Some(to))
            .search("test")
            .limit(25)
            .offset(5);

        let filters = builder.build();

        assert_eq!(filters.status, Some(TaskStatus::Incomplete));
        assert_eq!(filters.task_type, Some(TaskType::Todo));
        assert_eq!(filters.project_uuid, Some(uuid));
        assert_eq!(filters.tags, Some(tags));
        assert_eq!(filters.start_date_from, Some(from));
        assert_eq!(filters.start_date_to, Some(to));
        assert_eq!(filters.search_query, Some("test".to_string()));
        assert_eq!(filters.limit, Some(25));
        assert_eq!(filters.offset, Some(5));
    }
}
