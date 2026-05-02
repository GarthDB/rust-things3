//! Mutation backend abstraction.
//!
//! This module defines [`MutationBackend`], a trait that abstracts every Things 3
//! mutation operation behind a single interface. The MCP server holds an
//! `Arc<dyn MutationBackend>` and dispatches all writes through it. This unblocks
//! issue #120's migration from direct SQLite writes (which CulturedCode warns can
//! corrupt the user's database) to AppleScript-based mutations.
//!
//! Two implementations are planned:
//! - [`SqlxBackend`] — wraps the existing direct-DB writes on [`crate::ThingsDatabase`].
//!   Today's behavior; useful for offline tests and CI.
//! - `AppleScriptBackend` — to be added in #124. The default in production after #125.
//!
//! ## Why `#[async_trait]` instead of native `async fn` in traits
//!
//! The trait must be object-safe so the server can hold `Arc<dyn MutationBackend>`
//! and choose between backends at runtime. Native async-fn-in-trait (Rust 1.75+)
//! requires `#[trait_variant]` shims for `dyn` dispatch and produces unnameable
//! opaque return types — too much friction for marginal benefit. `#[async_trait]`
//! boxes the future, which is exactly what `dyn` needs.

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Result as ThingsResult;
use crate::models::{
    BulkCompleteRequest, BulkCreateTasksRequest, BulkDeleteRequest, BulkMoveRequest,
    BulkOperationResult, BulkUpdateDatesRequest, CreateAreaRequest, CreateProjectRequest,
    CreateTagRequest, CreateTaskRequest, DeleteChildHandling, ProjectChildHandling,
    TagAssignmentResult, TagCreationResult, TagMatch, UpdateAreaRequest, UpdateProjectRequest,
    UpdateTagRequest, UpdateTaskRequest,
};

mod sqlx;
pub use sqlx::SqlxBackend;

#[cfg(target_os = "macos")]
mod applescript;

/// Abstraction over every Things 3 mutation operation exposed as an MCP tool.
///
/// All implementations must be `Send + Sync` so the server can share them across
/// async tasks via `Arc<dyn MutationBackend>`.
#[async_trait]
pub trait MutationBackend: Send + Sync {
    // ---- Tasks ----

    async fn create_task(&self, request: CreateTaskRequest) -> ThingsResult<Uuid>;
    /// Create multiple tasks in one call. Best-effort and non-atomic — per-item
    /// failures are reported via `BulkOperationResult`.
    async fn bulk_create_tasks(
        &self,
        request: BulkCreateTasksRequest,
    ) -> ThingsResult<BulkOperationResult>;
    async fn update_task(&self, request: UpdateTaskRequest) -> ThingsResult<()>;
    async fn complete_task(&self, uuid: &Uuid) -> ThingsResult<()>;
    async fn uncomplete_task(&self, uuid: &Uuid) -> ThingsResult<()>;
    async fn delete_task(
        &self,
        uuid: &Uuid,
        child_handling: DeleteChildHandling,
    ) -> ThingsResult<()>;
    async fn bulk_delete(&self, request: BulkDeleteRequest) -> ThingsResult<BulkOperationResult>;
    async fn bulk_move(&self, request: BulkMoveRequest) -> ThingsResult<BulkOperationResult>;
    async fn bulk_update_dates(
        &self,
        request: BulkUpdateDatesRequest,
    ) -> ThingsResult<BulkOperationResult>;
    async fn bulk_complete(
        &self,
        request: BulkCompleteRequest,
    ) -> ThingsResult<BulkOperationResult>;

    // ---- Projects ----

    async fn create_project(&self, request: CreateProjectRequest) -> ThingsResult<Uuid>;
    async fn update_project(&self, request: UpdateProjectRequest) -> ThingsResult<()>;
    async fn complete_project(
        &self,
        uuid: &Uuid,
        child_handling: ProjectChildHandling,
    ) -> ThingsResult<()>;
    async fn delete_project(
        &self,
        uuid: &Uuid,
        child_handling: ProjectChildHandling,
    ) -> ThingsResult<()>;

    // ---- Areas ----

    async fn create_area(&self, request: CreateAreaRequest) -> ThingsResult<Uuid>;
    async fn update_area(&self, request: UpdateAreaRequest) -> ThingsResult<()>;
    async fn delete_area(&self, uuid: &Uuid) -> ThingsResult<()>;

    // ---- Tags ----

    /// Create a tag. When `force` is true, skip duplicate / similarity checks
    /// (mirrors the legacy `create_tag_force` path); otherwise run the smart
    /// flow that may return `Existing` or `SimilarFound`.
    async fn create_tag(
        &self,
        request: CreateTagRequest,
        force: bool,
    ) -> ThingsResult<TagCreationResult>;
    async fn update_tag(&self, request: UpdateTagRequest) -> ThingsResult<()>;
    async fn delete_tag(&self, uuid: &Uuid, remove_from_tasks: bool) -> ThingsResult<()>;
    async fn merge_tags(&self, source_uuid: &Uuid, target_uuid: &Uuid) -> ThingsResult<()>;
    async fn add_tag_to_task(
        &self,
        task_uuid: &Uuid,
        tag_title: &str,
    ) -> ThingsResult<TagAssignmentResult>;
    async fn remove_tag_from_task(&self, task_uuid: &Uuid, tag_title: &str) -> ThingsResult<()>;
    async fn set_task_tags(
        &self,
        task_uuid: &Uuid,
        tag_titles: Vec<String>,
    ) -> ThingsResult<Vec<TagMatch>>;
}
