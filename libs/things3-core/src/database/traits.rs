//! [`ThingsDatabase`] async trait — the contract for all Things 3 database
//! operations.  [`crate::database::SqliteThingsDatabase`] is the production
//! implementation; alternative backends (in-memory test fixture, PostgreSQL,
//! …) implement this trait to satisfy the same interface.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::database::{
    pool::{ComprehensiveHealthStatus, PoolHealthStatus, PoolMetrics},
    stats::DatabaseStats,
};
use crate::error::Result as ThingsResult;
use crate::models::{
    Area, BulkCompleteRequest, BulkDeleteRequest, BulkMoveRequest, BulkOperationResult,
    BulkUpdateDatesRequest, CreateAreaRequest, CreateProjectRequest, CreateTagRequest,
    CreateTaskRequest, DeleteChildHandling, Project, ProjectChildHandling, Tag,
    TagAssignmentResult, TagCompletion, TagCreationResult, TagMatch, TagPair, TagStatistics, Task,
    ThingsId, UpdateAreaRequest, UpdateProjectRequest, UpdateTagRequest, UpdateTaskRequest,
};

#[cfg(any(feature = "advanced-queries", feature = "batch-operations"))]
use crate::models::TaskFilters;

/// Async trait covering all Things 3 database operations.
///
/// Implementations must be `Send + Sync` so they can be shared across async
/// tasks via `Arc<dyn ThingsDatabase>`.
///
/// The production implementation is
/// [`SqliteThingsDatabase`](crate::database::SqliteThingsDatabase).
/// Constructor methods (`new`, `new_with_config`, `from_connection_string`,
/// `from_connection_string_with_config`) are inherent methods on concrete
/// types and are not part of this trait.
#[async_trait]
pub trait ThingsDatabase: Send + Sync {
    // ── Queries: Tasks ────────────────────────────────────────────────────

    async fn get_all_tasks(&self) -> ThingsResult<Vec<Task>>;

    async fn get_tasks_by_status(
        &self,
        status: crate::models::TaskStatus,
    ) -> ThingsResult<Vec<Task>>;

    async fn search_tasks(&self, query: &str) -> ThingsResult<Vec<Task>>;

    #[cfg(any(feature = "advanced-queries", feature = "batch-operations"))]
    async fn query_tasks(&self, filters: &TaskFilters) -> ThingsResult<Vec<Task>>;

    #[allow(clippy::too_many_arguments)]
    async fn search_logbook(
        &self,
        search_text: Option<String>,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
        project_uuid: Option<ThingsId>,
        area_uuid: Option<ThingsId>,
        tags: Option<Vec<String>>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> ThingsResult<Vec<Task>>;

    async fn get_inbox(&self, limit: Option<usize>) -> ThingsResult<Vec<Task>>;

    async fn get_today(&self, limit: Option<usize>) -> ThingsResult<Vec<Task>>;

    async fn get_task_by_uuid(&self, id: &ThingsId) -> ThingsResult<Option<Task>>;

    // ── Queries: Tags ─────────────────────────────────────────────────────

    async fn find_tag_by_normalized_title(&self, normalized: &str) -> ThingsResult<Option<Tag>>;

    async fn find_similar_tags(
        &self,
        title: &str,
        min_similarity: f32,
    ) -> ThingsResult<Vec<TagMatch>>;

    async fn search_tags(&self, query: &str) -> ThingsResult<Vec<Tag>>;

    async fn get_all_tags(&self) -> ThingsResult<Vec<Tag>>;

    async fn get_popular_tags(&self, limit: usize) -> ThingsResult<Vec<Tag>>;

    async fn get_recent_tags(&self, limit: usize) -> ThingsResult<Vec<Tag>>;

    async fn get_tag_completions(
        &self,
        partial_input: &str,
        limit: usize,
    ) -> ThingsResult<Vec<TagCompletion>>;

    async fn get_tag_statistics(&self, id: &ThingsId) -> ThingsResult<TagStatistics>;

    async fn find_duplicate_tags(&self, min_similarity: f32) -> ThingsResult<Vec<TagPair>>;

    // ── Queries: Projects ─────────────────────────────────────────────────

    async fn get_all_projects(&self) -> ThingsResult<Vec<Project>>;

    async fn get_projects(&self, limit: Option<usize>) -> ThingsResult<Vec<Project>>;

    async fn get_project_by_uuid(&self, id: &ThingsId) -> ThingsResult<Option<Project>>;

    // ── Queries: Areas ────────────────────────────────────────────────────

    async fn get_all_areas(&self) -> ThingsResult<Vec<Area>>;

    async fn get_areas(&self) -> ThingsResult<Vec<Area>>;

    // ── Mutations: Tasks ──────────────────────────────────────────────────

    async fn create_task(&self, request: CreateTaskRequest) -> ThingsResult<ThingsId>;

    async fn update_task(&self, request: UpdateTaskRequest) -> ThingsResult<()>;

    async fn complete_task(&self, id: &ThingsId) -> ThingsResult<()>;

    async fn uncomplete_task(&self, id: &ThingsId) -> ThingsResult<()>;

    async fn delete_task(
        &self,
        id: &ThingsId,
        child_handling: DeleteChildHandling,
    ) -> ThingsResult<()>;

    // ── Mutations: Tags ───────────────────────────────────────────────────

    async fn create_tag_smart(&self, request: CreateTagRequest) -> ThingsResult<TagCreationResult>;

    async fn create_tag_force(&self, request: CreateTagRequest) -> ThingsResult<ThingsId>;

    async fn update_tag(&self, request: UpdateTagRequest) -> ThingsResult<()>;

    async fn delete_tag(&self, id: &ThingsId, remove_from_tasks: bool) -> ThingsResult<()>;

    async fn merge_tags(&self, source_id: &ThingsId, target_id: &ThingsId) -> ThingsResult<()>;

    async fn add_tag_to_task(
        &self,
        task_id: &ThingsId,
        tag_title: &str,
    ) -> ThingsResult<TagAssignmentResult>;

    async fn remove_tag_from_task(&self, task_id: &ThingsId, tag_title: &str) -> ThingsResult<()>;

    async fn set_task_tags(
        &self,
        task_id: &ThingsId,
        tag_titles: Vec<String>,
    ) -> ThingsResult<Vec<TagMatch>>;

    // ── Mutations: Projects ───────────────────────────────────────────────

    async fn create_project(&self, request: CreateProjectRequest) -> ThingsResult<ThingsId>;

    async fn update_project(&self, request: UpdateProjectRequest) -> ThingsResult<()>;

    async fn complete_project(
        &self,
        id: &ThingsId,
        child_handling: ProjectChildHandling,
    ) -> ThingsResult<()>;

    async fn delete_project(
        &self,
        id: &ThingsId,
        child_handling: ProjectChildHandling,
    ) -> ThingsResult<()>;

    // ── Mutations: Areas ──────────────────────────────────────────────────

    async fn create_area(&self, request: CreateAreaRequest) -> ThingsResult<ThingsId>;

    async fn update_area(&self, request: UpdateAreaRequest) -> ThingsResult<()>;

    async fn delete_area(&self, id: &ThingsId) -> ThingsResult<()>;

    // ── Mutations: Bulk ───────────────────────────────────────────────────

    async fn bulk_move(&self, request: BulkMoveRequest) -> ThingsResult<BulkOperationResult>;

    async fn bulk_update_dates(
        &self,
        request: BulkUpdateDatesRequest,
    ) -> ThingsResult<BulkOperationResult>;

    async fn bulk_complete(
        &self,
        request: BulkCompleteRequest,
    ) -> ThingsResult<BulkOperationResult>;

    async fn bulk_delete(&self, request: BulkDeleteRequest) -> ThingsResult<BulkOperationResult>;

    // ── Health & Diagnostics ──────────────────────────────────────────────

    async fn is_connected(&self) -> bool;

    async fn get_pool_health(&self) -> ThingsResult<PoolHealthStatus>;

    async fn get_pool_metrics(&self) -> ThingsResult<PoolMetrics>;

    async fn comprehensive_health_check(&self) -> ThingsResult<ComprehensiveHealthStatus>;

    async fn get_stats(&self) -> ThingsResult<DatabaseStats>;

    // ── Batch operations (feature-gated) ──────────────────────────────────

    #[cfg(feature = "batch-operations")]
    async fn get_tasks_batch(&self, uuids: &[ThingsId]) -> ThingsResult<Vec<Task>>;

    #[cfg(feature = "batch-operations")]
    async fn get_projects_batch(&self, uuids: &[ThingsId]) -> ThingsResult<Vec<Project>>;
}
