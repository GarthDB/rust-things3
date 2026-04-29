//! `SqlxBackend` — direct-SQLite implementation of [`MutationBackend`].
//!
//! Forwards every call to the corresponding method on [`ThingsDatabase`]. Behavior
//! is byte-for-byte identical to calling the database directly. Kept around after
//! AppleScript becomes the default (#125) for offline tests, CI, and the
//! `--unsafe-direct-db` opt-in.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use super::MutationBackend;
use crate::database::ThingsDatabase;
use crate::error::Result as ThingsResult;
use crate::models::{
    BulkCompleteRequest, BulkCreateTasksRequest, BulkDeleteRequest, BulkMoveRequest,
    BulkOperationResult, BulkUpdateDatesRequest, CreateAreaRequest, CreateProjectRequest,
    CreateTagRequest, CreateTaskRequest, DeleteChildHandling, ProjectChildHandling,
    TagAssignmentResult, TagCreationResult, TagMatch, UpdateAreaRequest, UpdateProjectRequest,
    UpdateTagRequest, UpdateTaskRequest,
};

pub struct SqlxBackend {
    db: Arc<ThingsDatabase>,
}

impl SqlxBackend {
    #[must_use]
    pub fn new(db: Arc<ThingsDatabase>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl MutationBackend for SqlxBackend {
    // ---- Tasks ----

    async fn create_task(&self, request: CreateTaskRequest) -> ThingsResult<Uuid> {
        self.db.create_task(request).await
    }

    async fn bulk_create_tasks(
        &self,
        request: BulkCreateTasksRequest,
    ) -> ThingsResult<BulkOperationResult> {
        const MAX_BULK_BATCH_SIZE: usize = 1000;
        if request.tasks.is_empty() {
            return Err(crate::error::ThingsError::validation(
                "Tasks array cannot be empty",
            ));
        }
        if request.tasks.len() > MAX_BULK_BATCH_SIZE {
            return Err(crate::error::ThingsError::validation(format!(
                "Batch size {} exceeds maximum of {}",
                request.tasks.len(),
                MAX_BULK_BATCH_SIZE
            )));
        }
        let total = request.tasks.len();
        let mut processed = 0usize;
        let mut errors: Vec<String> = Vec::new();
        for (idx, task) in request.tasks.into_iter().enumerate() {
            match self.db.create_task(task).await {
                Ok(_) => processed += 1,
                Err(e) => errors.push(format!("task {idx}: {e}")),
            }
        }
        let success = errors.is_empty();
        let message = if success {
            format!("Successfully created {processed} task(s)")
        } else {
            format!("Created {processed}/{total}; errors: {}", errors.join("; "))
        };
        Ok(BulkOperationResult {
            success,
            processed_count: processed,
            message,
        })
    }

    async fn update_task(&self, request: UpdateTaskRequest) -> ThingsResult<()> {
        self.db.update_task(request).await
    }

    async fn complete_task(&self, uuid: &Uuid) -> ThingsResult<()> {
        self.db.complete_task(uuid).await
    }

    async fn uncomplete_task(&self, uuid: &Uuid) -> ThingsResult<()> {
        self.db.uncomplete_task(uuid).await
    }

    async fn delete_task(
        &self,
        uuid: &Uuid,
        child_handling: DeleteChildHandling,
    ) -> ThingsResult<()> {
        self.db.delete_task(uuid, child_handling).await
    }

    async fn bulk_delete(&self, request: BulkDeleteRequest) -> ThingsResult<BulkOperationResult> {
        self.db.bulk_delete(request).await
    }

    async fn bulk_move(&self, request: BulkMoveRequest) -> ThingsResult<BulkOperationResult> {
        self.db.bulk_move(request).await
    }

    async fn bulk_update_dates(
        &self,
        request: BulkUpdateDatesRequest,
    ) -> ThingsResult<BulkOperationResult> {
        self.db.bulk_update_dates(request).await
    }

    async fn bulk_complete(
        &self,
        request: BulkCompleteRequest,
    ) -> ThingsResult<BulkOperationResult> {
        self.db.bulk_complete(request).await
    }

    // ---- Projects ----

    async fn create_project(&self, request: CreateProjectRequest) -> ThingsResult<Uuid> {
        self.db.create_project(request).await
    }

    async fn update_project(&self, request: UpdateProjectRequest) -> ThingsResult<()> {
        self.db.update_project(request).await
    }

    async fn complete_project(
        &self,
        uuid: &Uuid,
        child_handling: ProjectChildHandling,
    ) -> ThingsResult<()> {
        self.db.complete_project(uuid, child_handling).await
    }

    async fn delete_project(
        &self,
        uuid: &Uuid,
        child_handling: ProjectChildHandling,
    ) -> ThingsResult<()> {
        self.db.delete_project(uuid, child_handling).await
    }

    // ---- Areas ----

    async fn create_area(&self, request: CreateAreaRequest) -> ThingsResult<Uuid> {
        self.db.create_area(request).await
    }

    async fn update_area(&self, request: UpdateAreaRequest) -> ThingsResult<()> {
        self.db.update_area(request).await
    }

    async fn delete_area(&self, uuid: &Uuid) -> ThingsResult<()> {
        self.db.delete_area(uuid).await
    }

    // ---- Tags ----

    async fn create_tag(
        &self,
        request: CreateTagRequest,
        force: bool,
    ) -> ThingsResult<TagCreationResult> {
        if force {
            let uuid = self.db.create_tag_force(request).await?;
            Ok(TagCreationResult::Created { uuid, is_new: true })
        } else {
            self.db.create_tag_smart(request).await
        }
    }

    async fn update_tag(&self, request: UpdateTagRequest) -> ThingsResult<()> {
        self.db.update_tag(request).await
    }

    async fn delete_tag(&self, uuid: &Uuid, remove_from_tasks: bool) -> ThingsResult<()> {
        self.db.delete_tag(uuid, remove_from_tasks).await
    }

    async fn merge_tags(&self, source_uuid: &Uuid, target_uuid: &Uuid) -> ThingsResult<()> {
        self.db.merge_tags(source_uuid, target_uuid).await
    }

    async fn add_tag_to_task(
        &self,
        task_uuid: &Uuid,
        tag_title: &str,
    ) -> ThingsResult<TagAssignmentResult> {
        self.db.add_tag_to_task(task_uuid, tag_title).await
    }

    async fn remove_tag_from_task(&self, task_uuid: &Uuid, tag_title: &str) -> ThingsResult<()> {
        self.db.remove_tag_from_task(task_uuid, tag_title).await
    }

    async fn set_task_tags(
        &self,
        task_uuid: &Uuid,
        tag_titles: Vec<String>,
    ) -> ThingsResult<Vec<TagMatch>> {
        self.db.set_task_tags(task_uuid, tag_titles).await
    }
}
