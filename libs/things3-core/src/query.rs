//! Query builder for filtering and searching tasks

use crate::models::{TaskFilters, TaskStatus, TaskType};
use chrono::{Datelike, Duration, NaiveDate, Utc};
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

    /// Filter by tags (AND semantics — task must contain every listed tag).
    #[must_use]
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.filters.tags = Some(tags);
        self
    }

    /// Filter to tasks containing **any** of these tags (OR semantics).
    ///
    /// Composes with `.tags()` (AND) and `.exclude_tags()` (NOT) — all
    /// active tag filters must be satisfied.
    #[must_use]
    pub fn any_tags(mut self, tags: Vec<String>) -> Self {
        self.filters.any_tags = Some(tags);
        self
    }

    /// Filter out tasks containing any of these tags.
    #[must_use]
    pub fn exclude_tags(mut self, tags: Vec<String>) -> Self {
        self.filters.exclude_tags = Some(tags);
        self
    }

    /// Filter to tasks with at least `min` tags total.
    #[must_use]
    pub const fn tag_count(mut self, min: usize) -> Self {
        self.filters.tag_count_min = Some(min);
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

    /// Filter to tasks whose deadline is today.
    #[must_use]
    pub fn due_today(self) -> Self {
        let today = today();
        self.deadline_range(Some(today), Some(today))
    }

    /// Filter to tasks whose deadline falls between today and the upcoming Sunday
    /// (Monday-Sunday week).
    #[must_use]
    pub fn due_this_week(self) -> Self {
        let today = today();
        self.deadline_range(Some(today), Some(end_of_week(today)))
    }

    /// Filter to tasks whose deadline falls in next calendar week (next Monday
    /// through Sunday, Monday-Sunday week).
    #[must_use]
    pub fn due_next_week(self) -> Self {
        let today = today();
        let next_monday = end_of_week(today) + Duration::days(1);
        self.deadline_range(Some(next_monday), Some(end_of_week(next_monday)))
    }

    /// Filter to tasks whose deadline is between today and `days` days from now (inclusive).
    #[must_use]
    pub fn due_in(self, days: i64) -> Self {
        let today = today();
        self.deadline_range(Some(today), Some(today + Duration::days(days)))
    }

    /// Filter to overdue tasks: deadline strictly before today.
    ///
    /// If no `status` filter has already been set, this also restricts results
    /// to incomplete tasks (a completed task isn't meaningfully overdue). An
    /// explicit `.status(...)` call before this helper is preserved.
    #[must_use]
    pub fn overdue(mut self) -> Self {
        let yesterday = today() - Duration::days(1);
        self.filters.deadline_from = None;
        self.filters.deadline_to = Some(yesterday);
        if self.filters.status.is_none() {
            self.filters.status = Some(TaskStatus::Incomplete);
        }
        self
    }

    /// Filter to tasks with a start date of today.
    #[must_use]
    pub fn starting_today(self) -> Self {
        let today = today();
        self.start_date_range(Some(today), Some(today))
    }

    /// Filter to tasks with a start date between today and the upcoming Sunday
    /// (Monday-Sunday week).
    #[must_use]
    pub fn starting_this_week(self) -> Self {
        let today = today();
        self.start_date_range(Some(today), Some(end_of_week(today)))
    }

    /// Build the final filters
    #[must_use]
    pub fn build(self) -> TaskFilters {
        self.filters
    }

    /// Execute the query against a live database connection.
    ///
    /// Requires the `advanced-queries` feature flag.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or task data cannot be mapped.
    #[cfg(feature = "advanced-queries")]
    pub async fn execute(
        &self,
        db: &crate::database::ThingsDatabase,
    ) -> crate::error::Result<Vec<crate::models::Task>> {
        db.query_tasks(&self.filters).await
    }
}

impl Default for TaskQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

fn end_of_week(d: NaiveDate) -> NaiveDate {
    let days_from_monday = i64::from(d.weekday().num_days_from_monday());
    d + Duration::days(6 - days_from_monday)
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
    fn test_task_query_builder_any_tags() {
        let tags = vec!["a".to_string(), "b".to_string()];
        let filters = TaskQueryBuilder::new().any_tags(tags.clone()).build();
        assert_eq!(filters.any_tags, Some(tags));
    }

    #[test]
    fn test_task_query_builder_exclude_tags() {
        let tags = vec!["archived".to_string()];
        let filters = TaskQueryBuilder::new().exclude_tags(tags.clone()).build();
        assert_eq!(filters.exclude_tags, Some(tags));
    }

    #[test]
    fn test_task_query_builder_tag_count() {
        let filters = TaskQueryBuilder::new().tag_count(2).build();
        assert_eq!(filters.tag_count_min, Some(2));
    }

    #[test]
    fn test_task_query_builder_chaining_tag_methods() {
        let filters = TaskQueryBuilder::new()
            .tags(vec!["a".to_string()])
            .any_tags(vec!["b".to_string(), "c".to_string()])
            .exclude_tags(vec!["d".to_string()])
            .tag_count(1)
            .build();
        assert_eq!(filters.tags, Some(vec!["a".to_string()]));
        assert_eq!(
            filters.any_tags,
            Some(vec!["b".to_string(), "c".to_string()])
        );
        assert_eq!(filters.exclude_tags, Some(vec!["d".to_string()]));
        assert_eq!(filters.tag_count_min, Some(1));
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

    #[cfg(feature = "advanced-queries")]
    mod execute_tests {
        use super::*;
        use tempfile::NamedTempFile;

        #[tokio::test]
        async fn test_execute_empty_builder() {
            let f = NamedTempFile::new().unwrap();
            crate::test_utils::create_test_database(f.path())
                .await
                .unwrap();
            let db = crate::database::ThingsDatabase::new(f.path())
                .await
                .unwrap();
            let result = TaskQueryBuilder::new().execute(&db).await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_execute_with_status_filter() {
            let f = NamedTempFile::new().unwrap();
            crate::test_utils::create_test_database(f.path())
                .await
                .unwrap();
            let db = crate::database::ThingsDatabase::new(f.path())
                .await
                .unwrap();
            let result = TaskQueryBuilder::new()
                .status(TaskStatus::Incomplete)
                .execute(&db)
                .await;
            assert!(result.is_ok());
            assert!(result
                .unwrap()
                .iter()
                .all(|t| t.status == TaskStatus::Incomplete));
        }
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

    mod date_helper_tests {
        use super::*;

        #[test]
        fn test_due_today_sets_deadline_range_to_today() {
            let filters = TaskQueryBuilder::new().due_today().build();
            let today = today();
            assert_eq!(filters.deadline_from, Some(today));
            assert_eq!(filters.deadline_to, Some(today));
        }

        #[test]
        fn test_due_this_week_ends_on_sunday() {
            let filters = TaskQueryBuilder::new().due_this_week().build();
            let today = today();
            assert_eq!(filters.deadline_from, Some(today));
            let to = filters.deadline_to.unwrap();
            assert_eq!(to.weekday(), chrono::Weekday::Sun);
            assert!(to >= today);
        }

        #[test]
        fn test_due_next_week_spans_monday_to_sunday() {
            let filters = TaskQueryBuilder::new().due_next_week().build();
            let from = filters.deadline_from.unwrap();
            let to = filters.deadline_to.unwrap();
            assert_eq!(from.weekday(), chrono::Weekday::Mon);
            assert_eq!(to.weekday(), chrono::Weekday::Sun);
            assert_eq!(to - from, Duration::days(6));
            assert!(from > today());
        }

        #[test]
        fn test_due_in_n_days() {
            let filters = TaskQueryBuilder::new().due_in(7).build();
            let today = today();
            assert_eq!(filters.deadline_from, Some(today));
            assert_eq!(filters.deadline_to, Some(today + Duration::days(7)));
        }

        #[test]
        fn test_due_in_zero_days_is_today() {
            let filters = TaskQueryBuilder::new().due_in(0).build();
            let today = today();
            assert_eq!(filters.deadline_from, Some(today));
            assert_eq!(filters.deadline_to, Some(today));
        }

        #[test]
        fn test_overdue_sets_deadline_to_yesterday_with_no_lower_bound() {
            let filters = TaskQueryBuilder::new().overdue().build();
            let yesterday = today() - Duration::days(1);
            assert_eq!(filters.deadline_from, None);
            assert_eq!(filters.deadline_to, Some(yesterday));
        }

        #[test]
        fn test_overdue_implicitly_sets_status_incomplete_when_unset() {
            let filters = TaskQueryBuilder::new().overdue().build();
            assert_eq!(filters.status, Some(TaskStatus::Incomplete));
        }

        #[test]
        fn test_overdue_does_not_override_explicit_status() {
            let filters = TaskQueryBuilder::new()
                .status(TaskStatus::Canceled)
                .overdue()
                .build();
            assert_eq!(filters.status, Some(TaskStatus::Canceled));
        }

        #[test]
        fn test_starting_today_sets_start_date_range() {
            let filters = TaskQueryBuilder::new().starting_today().build();
            let today = today();
            assert_eq!(filters.start_date_from, Some(today));
            assert_eq!(filters.start_date_to, Some(today));
        }

        #[test]
        fn test_starting_this_week_ends_on_sunday() {
            let filters = TaskQueryBuilder::new().starting_this_week().build();
            let today = today();
            assert_eq!(filters.start_date_from, Some(today));
            let to = filters.start_date_to.unwrap();
            assert_eq!(to.weekday(), chrono::Weekday::Sun);
            assert!(to >= today);
        }

        #[test]
        fn test_end_of_week_on_monday_returns_following_sunday() {
            let monday = NaiveDate::from_ymd_opt(2026, 4, 27).unwrap();
            assert_eq!(monday.weekday(), chrono::Weekday::Mon);
            let eow = end_of_week(monday);
            assert_eq!(eow, NaiveDate::from_ymd_opt(2026, 5, 3).unwrap());
            assert_eq!(eow.weekday(), chrono::Weekday::Sun);
        }

        #[test]
        fn test_end_of_week_on_sunday_returns_same_day() {
            let sunday = NaiveDate::from_ymd_opt(2026, 5, 3).unwrap();
            assert_eq!(sunday.weekday(), chrono::Weekday::Sun);
            assert_eq!(end_of_week(sunday), sunday);
        }
    }
}
