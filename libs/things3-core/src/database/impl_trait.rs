//! [`ThingsDatabase`] trait impl for [`SqliteThingsDatabase`].
//!
//! This is the single required `impl Trait for Type` block. Every method
//! delegates to the identically-named inherent method on
//! [`SqliteThingsDatabase`]; the inherent methods live in the per-concern
//! submodules (`queries/`, `mutations/`, `health.rs`, etc.). Rust's method
//! resolution prefers inherent methods over trait methods, so
//! `self.method_name()` inside each body calls the inherent implementation,
//! not the trait method — there is no infinite recursion.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::database::{
    pool::{ComprehensiveHealthStatus, PoolHealthStatus, PoolMetrics},
    stats::DatabaseStats,
    traits::ThingsDatabase,
    SqliteThingsDatabase,
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

#[cfg(feature = "batch-operations")]
use crate::models::Project as BatchProject;

#[async_trait]
impl ThingsDatabase for SqliteThingsDatabase {
    // ── Queries: Tasks ────────────────────────────────────────────────────

    async fn get_all_tasks(&self) -> ThingsResult<Vec<Task>> {
        self.get_all_tasks().await
    }

    async fn get_tasks_by_status(
        &self,
        status: crate::models::TaskStatus,
    ) -> ThingsResult<Vec<Task>> {
        self.get_tasks_by_status(status).await
    }

    async fn search_tasks(&self, query: &str) -> ThingsResult<Vec<Task>> {
        self.search_tasks(query).await
    }

    #[cfg(any(feature = "advanced-queries", feature = "batch-operations"))]
    async fn query_tasks(&self, filters: &TaskFilters) -> ThingsResult<Vec<Task>> {
        self.query_tasks(filters).await
    }

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
    ) -> ThingsResult<Vec<Task>> {
        self.search_logbook(
            search_text,
            from_date,
            to_date,
            project_uuid,
            area_uuid,
            tags,
            limit,
            offset,
        )
        .await
    }

    async fn get_inbox(&self, limit: Option<usize>) -> ThingsResult<Vec<Task>> {
        self.get_inbox(limit).await
    }

    async fn get_today(&self, limit: Option<usize>) -> ThingsResult<Vec<Task>> {
        self.get_today(limit).await
    }

    async fn get_task_by_uuid(&self, id: &ThingsId) -> ThingsResult<Option<Task>> {
        self.get_task_by_uuid(id).await
    }

    // ── Queries: Tags ─────────────────────────────────────────────────────

    async fn find_tag_by_normalized_title(&self, normalized: &str) -> ThingsResult<Option<Tag>> {
        self.find_tag_by_normalized_title(normalized).await
    }

    async fn find_similar_tags(
        &self,
        title: &str,
        min_similarity: f32,
    ) -> ThingsResult<Vec<TagMatch>> {
        self.find_similar_tags(title, min_similarity).await
    }

    async fn search_tags(&self, query: &str) -> ThingsResult<Vec<Tag>> {
        self.search_tags(query).await
    }

    async fn get_all_tags(&self) -> ThingsResult<Vec<Tag>> {
        self.get_all_tags().await
    }

    async fn get_popular_tags(&self, limit: usize) -> ThingsResult<Vec<Tag>> {
        self.get_popular_tags(limit).await
    }

    async fn get_recent_tags(&self, limit: usize) -> ThingsResult<Vec<Tag>> {
        self.get_recent_tags(limit).await
    }

    async fn get_tag_completions(
        &self,
        partial_input: &str,
        limit: usize,
    ) -> ThingsResult<Vec<TagCompletion>> {
        self.get_tag_completions(partial_input, limit).await
    }

    async fn get_tag_statistics(&self, id: &ThingsId) -> ThingsResult<TagStatistics> {
        self.get_tag_statistics(id).await
    }

    async fn find_duplicate_tags(&self, min_similarity: f32) -> ThingsResult<Vec<TagPair>> {
        self.find_duplicate_tags(min_similarity).await
    }

    // ── Queries: Projects ─────────────────────────────────────────────────

    async fn get_all_projects(&self) -> ThingsResult<Vec<Project>> {
        self.get_all_projects().await
    }

    async fn get_projects(&self, limit: Option<usize>) -> ThingsResult<Vec<Project>> {
        self.get_projects(limit).await
    }

    async fn get_project_by_uuid(&self, id: &ThingsId) -> ThingsResult<Option<Project>> {
        self.get_project_by_uuid(id).await
    }

    // ── Queries: Areas ────────────────────────────────────────────────────

    async fn get_all_areas(&self) -> ThingsResult<Vec<Area>> {
        self.get_all_areas().await
    }

    async fn get_areas(&self) -> ThingsResult<Vec<Area>> {
        self.get_areas().await
    }

    // ── Mutations: Tasks ──────────────────────────────────────────────────

    async fn create_task(&self, request: CreateTaskRequest) -> ThingsResult<ThingsId> {
        self.create_task(request).await
    }

    async fn update_task(&self, request: UpdateTaskRequest) -> ThingsResult<()> {
        self.update_task(request).await
    }

    async fn complete_task(&self, id: &ThingsId) -> ThingsResult<()> {
        self.complete_task(id).await
    }

    async fn uncomplete_task(&self, id: &ThingsId) -> ThingsResult<()> {
        self.uncomplete_task(id).await
    }

    async fn delete_task(
        &self,
        id: &ThingsId,
        child_handling: DeleteChildHandling,
    ) -> ThingsResult<()> {
        self.delete_task(id, child_handling).await
    }

    // ── Mutations: Tags ───────────────────────────────────────────────────

    async fn create_tag_smart(&self, request: CreateTagRequest) -> ThingsResult<TagCreationResult> {
        self.create_tag_smart(request).await
    }

    async fn create_tag_force(&self, request: CreateTagRequest) -> ThingsResult<ThingsId> {
        self.create_tag_force(request).await
    }

    async fn update_tag(&self, request: UpdateTagRequest) -> ThingsResult<()> {
        self.update_tag(request).await
    }

    async fn delete_tag(&self, id: &ThingsId, remove_from_tasks: bool) -> ThingsResult<()> {
        self.delete_tag(id, remove_from_tasks).await
    }

    async fn merge_tags(&self, source_id: &ThingsId, target_id: &ThingsId) -> ThingsResult<()> {
        self.merge_tags(source_id, target_id).await
    }

    async fn add_tag_to_task(
        &self,
        task_id: &ThingsId,
        tag_title: &str,
    ) -> ThingsResult<TagAssignmentResult> {
        self.add_tag_to_task(task_id, tag_title).await
    }

    async fn remove_tag_from_task(&self, task_id: &ThingsId, tag_title: &str) -> ThingsResult<()> {
        self.remove_tag_from_task(task_id, tag_title).await
    }

    async fn set_task_tags(
        &self,
        task_id: &ThingsId,
        tag_titles: Vec<String>,
    ) -> ThingsResult<Vec<TagMatch>> {
        self.set_task_tags(task_id, tag_titles).await
    }

    // ── Mutations: Projects ───────────────────────────────────────────────

    async fn create_project(&self, request: CreateProjectRequest) -> ThingsResult<ThingsId> {
        self.create_project(request).await
    }

    async fn update_project(&self, request: UpdateProjectRequest) -> ThingsResult<()> {
        self.update_project(request).await
    }

    async fn complete_project(
        &self,
        id: &ThingsId,
        child_handling: ProjectChildHandling,
    ) -> ThingsResult<()> {
        self.complete_project(id, child_handling).await
    }

    async fn delete_project(
        &self,
        id: &ThingsId,
        child_handling: ProjectChildHandling,
    ) -> ThingsResult<()> {
        self.delete_project(id, child_handling).await
    }

    // ── Mutations: Areas ──────────────────────────────────────────────────

    async fn create_area(&self, request: CreateAreaRequest) -> ThingsResult<ThingsId> {
        self.create_area(request).await
    }

    async fn update_area(&self, request: UpdateAreaRequest) -> ThingsResult<()> {
        self.update_area(request).await
    }

    async fn delete_area(&self, id: &ThingsId) -> ThingsResult<()> {
        self.delete_area(id).await
    }

    // ── Mutations: Bulk ───────────────────────────────────────────────────

    async fn bulk_move(&self, request: BulkMoveRequest) -> ThingsResult<BulkOperationResult> {
        self.bulk_move(request).await
    }

    async fn bulk_update_dates(
        &self,
        request: BulkUpdateDatesRequest,
    ) -> ThingsResult<BulkOperationResult> {
        self.bulk_update_dates(request).await
    }

    async fn bulk_complete(
        &self,
        request: BulkCompleteRequest,
    ) -> ThingsResult<BulkOperationResult> {
        self.bulk_complete(request).await
    }

    async fn bulk_delete(&self, request: BulkDeleteRequest) -> ThingsResult<BulkOperationResult> {
        self.bulk_delete(request).await
    }

    // ── Health & Diagnostics ──────────────────────────────────────────────

    async fn is_connected(&self) -> bool {
        self.is_connected().await
    }

    async fn get_pool_health(&self) -> ThingsResult<PoolHealthStatus> {
        self.get_pool_health().await
    }

    async fn get_pool_metrics(&self) -> ThingsResult<PoolMetrics> {
        self.get_pool_metrics().await
    }

    async fn comprehensive_health_check(&self) -> ThingsResult<ComprehensiveHealthStatus> {
        self.comprehensive_health_check().await
    }

    async fn get_stats(&self) -> ThingsResult<DatabaseStats> {
        self.get_stats().await
    }

    // ── Batch operations (feature-gated) ──────────────────────────────────

    #[cfg(feature = "batch-operations")]
    async fn get_tasks_batch(&self, uuids: &[ThingsId]) -> ThingsResult<Vec<Task>> {
        self.get_tasks_batch(uuids).await
    }

    #[cfg(feature = "batch-operations")]
    async fn get_projects_batch(&self, uuids: &[ThingsId]) -> ThingsResult<Vec<BatchProject>> {
        self.get_projects_batch(uuids).await
    }
}
