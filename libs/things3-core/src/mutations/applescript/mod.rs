//! AppleScript-based mutation backend.
//!
//! Drives Things 3 via `osascript` — CulturedCode's [documented Mac-only
//! scripting API](https://culturedcode.com/things/support/articles/4562654/).
//! Replaces direct-SQLite writes (which CulturedCode warns can corrupt the
//! user's database — see [the safety
//! article](https://culturedcode.com/things/support/articles/5510170/)) for
//! every mutation operation rust-things3 exposes.
//!
//! ## Layout
//!
//! - [`escape`] — pure string-literal escaping; the script-injection guard
//! - [`runner`] — `osascript` process spawn + error mapping
//! - [`script`] — pure builders that emit AppleScript text from typed requests
//! - [`parse`] — turn osascript stdout into typed values (UUIDs)
//! - This module ([`AppleScriptBackend`]) — wires the four together behind the
//!   [`crate::mutations::MutationBackend`] trait. Gated `#[cfg(target_os = "macos")]`.
//!
//! ## Implementation status
//!
//! All 21 [`MutationBackend`] methods are implemented end-to-end:
//! tasks (Phase B, #134), projects/areas/bulk ops (Phase C, #135), and tags
//! (Phase D, #136). Live integration tests + the production default-switch
//! land in Phase E (#137) and #125 respectively. Until #125 ships, no
//! production code constructs an `AppleScriptBackend`.

pub(crate) mod escape;
pub(crate) mod parse;
pub(crate) mod runner;
pub(crate) mod script;

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::Row;

use super::MutationBackend;
use crate::database::ThingsDatabase;
use crate::error::{Result as ThingsResult, ThingsError};
use crate::models::{
    BulkCompleteRequest, BulkCreateTasksRequest, BulkDeleteRequest, BulkMoveRequest,
    BulkOperationResult, BulkUpdateDatesRequest, CreateAreaRequest, CreateProjectRequest,
    CreateTagRequest, CreateTaskRequest, DeleteChildHandling, ProjectChildHandling,
    TagAssignmentResult, TagCreationResult, TagMatch, ThingsId, UpdateAreaRequest,
    UpdateProjectRequest, UpdateTagRequest, UpdateTaskRequest,
};

/// AppleScript-driven mutation backend.
///
/// Holds an [`Arc<ThingsDatabase>`] for read-only side-channel queries — used
/// today only by [`AppleScriptBackend::delete_task`] when it needs to discover
/// a task's subtasks before deciding how to handle them. Reads are safe per
/// CulturedCode; only writes risk corruption.
pub struct AppleScriptBackend {
    db: Arc<ThingsDatabase>,
}

impl AppleScriptBackend {
    #[must_use]
    pub fn new(db: Arc<ThingsDatabase>) -> Self {
        Self { db }
    }

    /// Read the IDs of every non-trashed direct subtask of `parent` via
    /// sqlx. Read-only, CulturedCode-safe.
    async fn list_subtask_uuids(&self, parent: &ThingsId) -> ThingsResult<Vec<ThingsId>> {
        let rows = sqlx::query("SELECT uuid FROM TMTask WHERE heading = ? AND trashed = 0")
            .bind(parent.as_str())
            .fetch_all(&self.db.pool)
            .await
            .map_err(|e| {
                ThingsError::applescript(format!("failed to query subtasks of {parent}: {e}"))
            })?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let s: String = row.get("uuid");
                ThingsId::from_trusted(s)
            })
            .collect())
    }

    /// Read the IDs of every non-trashed direct child task of `project` via
    /// sqlx. Used by `complete_project` / `delete_project` to handle
    /// `ProjectChildHandling`. Read-only, CulturedCode-safe.
    async fn list_project_task_uuids(&self, project: &ThingsId) -> ThingsResult<Vec<ThingsId>> {
        let rows = sqlx::query("SELECT uuid FROM TMTask WHERE project = ? AND trashed = 0")
            .bind(project.as_str())
            .fetch_all(&self.db.pool)
            .await
            .map_err(|e| {
                ThingsError::applescript(format!(
                    "failed to query child tasks of project {project}: {e}"
                ))
            })?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let s: String = row.get("uuid");
                ThingsId::from_trusted(s)
            })
            .collect())
    }

    /// Read the title of a tag by UUID. Errors if the tag doesn't exist.
    /// Used by `merge_tags` and `delete_tag(remove_from_tasks=true)` to find
    /// what tag-name to look for in tasks' `cachedTags`.
    async fn read_tag_title(&self, id: &ThingsId) -> ThingsResult<String> {
        let row = sqlx::query("SELECT title FROM TMTag WHERE uuid = ?")
            .bind(id.as_str())
            .fetch_optional(&self.db.pool)
            .await
            .map_err(|e| ThingsError::applescript(format!("failed to read tag {id}: {e}")))?;
        let row = row.ok_or_else(|| ThingsError::applescript(format!("tag not found: {id}")))?;
        Ok(row.get("title"))
    }

    /// Read a task's current tag-title list from the DB's `cachedTags` blob.
    /// Returns an empty list if the task has no tags. Errors if the task
    /// doesn't exist.
    ///
    /// CulturedCode-safe: read-only access.
    async fn read_task_tag_titles(&self, task_id: &ThingsId) -> ThingsResult<Vec<String>> {
        let row = sqlx::query("SELECT cachedTags FROM TMTask WHERE uuid = ?")
            .bind(task_id.as_str())
            .fetch_optional(&self.db.pool)
            .await
            .map_err(|e| {
                ThingsError::applescript(format!("failed to read tags for task {task_id}: {e}"))
            })?;
        let row =
            row.ok_or_else(|| ThingsError::applescript(format!("task not found: {task_id}")))?;
        let blob: Option<Vec<u8>> = row.get("cachedTags");
        match blob {
            None => Ok(Vec::new()),
            Some(b) => crate::database::deserialize_tags_from_blob(&b),
        }
    }

    /// List `(task_id, current_tag_titles)` for every non-trashed task whose
    /// `cachedTags` contains a tag matching `tag_title` (case-insensitive
    /// after `normalize_tag_title`). Used by `merge_tags` and
    /// `delete_tag(remove_from_tasks=true)` to plan per-task rewrites.
    ///
    /// Pre-filters via `json_extract(cachedTags, '$') LIKE` to narrow the
    /// candidate set, then deserializes each blob and re-checks the
    /// normalized title to drop false positives.
    async fn list_tasks_with_tag_title(
        &self,
        tag_title: &str,
    ) -> ThingsResult<Vec<(ThingsId, Vec<String>)>> {
        use crate::database::tag_utils::normalize_tag_title;

        // SQLite LIKE only treats `\` as an escape when paired with an explicit
        // ESCAPE clause; without it `\_` is a literal two-char sequence and a
        // tag containing `_` or `%` would silently skip its tasks.
        let escaped = tag_title
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%\"{escaped}\"%");
        let rows = sqlx::query(
            r"SELECT uuid, cachedTags FROM TMTask
             WHERE cachedTags IS NOT NULL
             AND json_extract(cachedTags, '$') LIKE ? ESCAPE '\'
             AND trashed = 0",
        )
        .bind(&pattern)
        .fetch_all(&self.db.pool)
        .await
        .map_err(|e| {
            ThingsError::applescript(format!("failed to query tasks with tag '{tag_title}': {e}"))
        })?;

        let normalized_target = normalize_tag_title(tag_title);
        let mut out = Vec::new();
        for row in rows {
            let id_str: String = row.get("uuid");
            let blob: Vec<u8> = row.get("cachedTags");
            let tags = crate::database::deserialize_tags_from_blob(&blob)?;
            if tags
                .iter()
                .any(|t| normalize_tag_title(t) == normalized_target)
            {
                out.push((ThingsId::from_trusted(id_str), tags));
            }
        }
        Ok(out)
    }

    /// Create a tag via osascript and parse its returned UUID. Used by
    /// `create_tag(force=true)` and the auto-create branches of
    /// `add_tag_to_task` / `set_task_tags`.
    async fn create_tag_via_as(&self, request: &CreateTagRequest) -> ThingsResult<ThingsId> {
        if request.shortcut.is_some() || request.parent_uuid.is_some() {
            tracing::debug!(
                tag = %request.title,
                "shortcut and/or parent_uuid set on CreateTagRequest; \
                 Things AppleScript does not expose those properties on `tag`, \
                 so they are silently dropped (#136)"
            );
        }
        let script = script::create_tag_script(request);
        let stdout = runner::run_script(&script).await?;
        parse::extract_id(&stdout)
    }
}

const MAX_BULK_BATCH_SIZE: usize = 1000;

#[async_trait]
impl MutationBackend for AppleScriptBackend {
    fn kind(&self) -> &'static str {
        "applescript"
    }

    // ---- Tasks (Phase B — implemented) ----

    async fn create_task(&self, request: CreateTaskRequest) -> ThingsResult<ThingsId> {
        let script = script::create_task_script(&request);
        let stdout = runner::run_script(&script).await?;
        parse::extract_id(&stdout)
    }

    async fn update_task(&self, request: UpdateTaskRequest) -> ThingsResult<()> {
        request.uuid.as_things_native()?;
        if let Some(p) = &request.project_uuid {
            p.as_things_native()?;
        }
        if let Some(a) = &request.area_uuid {
            a.as_things_native()?;
        }
        let script = script::update_task_script(&request);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn complete_task(&self, id: &ThingsId) -> ThingsResult<()> {
        id.as_things_native()?;
        let script = script::complete_task_script(id);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn uncomplete_task(&self, id: &ThingsId) -> ThingsResult<()> {
        id.as_things_native()?;
        let script = script::uncomplete_task_script(id);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn delete_task(
        &self,
        id: &ThingsId,
        child_handling: DeleteChildHandling,
    ) -> ThingsResult<()> {
        id.as_things_native()?;
        let children = self.list_subtask_uuids(id).await?;

        if !children.is_empty() {
            match child_handling {
                DeleteChildHandling::Error => {
                    return Err(ThingsError::applescript(format!(
                        "task {id} has {} subtask(s); pass DeleteChildHandling::Cascade or ::Orphan",
                        children.len()
                    )));
                }
                DeleteChildHandling::Cascade => {
                    for child in &children {
                        let script = script::delete_task_script(child);
                        runner::run_script(&script).await?;
                    }
                }
                DeleteChildHandling::Orphan => {
                    // Things AS does not expose the `heading` (parent) setter on `to do`,
                    // so we cannot null out the children's parent the way the sqlx path
                    // does. Return early with a clear message rather than silently
                    // deleting them. Will be revisited in Phase C (#135) once we know
                    // whether `move <child> to <project of parent>` is the right shape.
                    return Err(ThingsError::applescript(
                        "DeleteChildHandling::Orphan is not yet supported by AppleScriptBackend; \
                         use ::Cascade or ::Error (tracked in #135)",
                    ));
                }
            }
        }

        let script = script::delete_task_script(id);
        runner::run_script(&script).await?;
        Ok(())
    }

    // ---- Bulk operations (Phase C — implemented) ----

    async fn bulk_create_tasks(
        &self,
        request: BulkCreateTasksRequest,
    ) -> ThingsResult<BulkOperationResult> {
        if request.tasks.is_empty() {
            return Err(ThingsError::validation("Tasks array cannot be empty"));
        }
        if request.tasks.len() > MAX_BULK_BATCH_SIZE {
            return Err(ThingsError::validation(format!(
                "Batch size {} exceeds maximum of {MAX_BULK_BATCH_SIZE}",
                request.tasks.len(),
            )));
        }
        let total = request.tasks.len();
        let script = script::bulk_create_tasks_script(&request);
        let stdout = runner::run_script(&script).await?;
        parse::parse_bulk_result(&stdout, total)
    }

    async fn bulk_delete(&self, request: BulkDeleteRequest) -> ThingsResult<BulkOperationResult> {
        if request.task_uuids.is_empty() {
            return Err(ThingsError::validation("Task UUIDs array cannot be empty"));
        }
        if request.task_uuids.len() > MAX_BULK_BATCH_SIZE {
            return Err(ThingsError::validation(format!(
                "Batch size {} exceeds maximum of {MAX_BULK_BATCH_SIZE}",
                request.task_uuids.len(),
            )));
        }
        for id in &request.task_uuids {
            id.as_things_native()?;
        }
        let total = request.task_uuids.len();
        let script = script::bulk_delete_script(&request);
        let stdout = runner::run_script(&script).await?;
        parse::parse_bulk_result(&stdout, total)
    }

    async fn bulk_move(&self, request: BulkMoveRequest) -> ThingsResult<BulkOperationResult> {
        if request.task_uuids.is_empty() {
            return Err(ThingsError::validation("Task UUIDs array cannot be empty"));
        }
        if request.task_uuids.len() > MAX_BULK_BATCH_SIZE {
            return Err(ThingsError::validation(format!(
                "Batch size {} exceeds maximum of {MAX_BULK_BATCH_SIZE}",
                request.task_uuids.len(),
            )));
        }
        if request.project_uuid.is_none() && request.area_uuid.is_none() {
            return Err(ThingsError::validation(
                "bulk_move requires either project_uuid or area_uuid",
            ));
        }
        for id in &request.task_uuids {
            id.as_things_native()?;
        }
        if let Some(p) = &request.project_uuid {
            p.as_things_native()?;
        }
        if let Some(a) = &request.area_uuid {
            a.as_things_native()?;
        }
        let total = request.task_uuids.len();
        let script = script::bulk_move_script(&request);
        let stdout = runner::run_script(&script).await?;
        parse::parse_bulk_result(&stdout, total)
    }

    async fn bulk_update_dates(
        &self,
        request: BulkUpdateDatesRequest,
    ) -> ThingsResult<BulkOperationResult> {
        if request.task_uuids.is_empty() {
            return Err(ThingsError::validation("Task UUIDs array cannot be empty"));
        }
        if request.task_uuids.len() > MAX_BULK_BATCH_SIZE {
            return Err(ThingsError::validation(format!(
                "Batch size {} exceeds maximum of {MAX_BULK_BATCH_SIZE}",
                request.task_uuids.len(),
            )));
        }
        for id in &request.task_uuids {
            id.as_things_native()?;
        }
        let total = request.task_uuids.len();
        let script = script::bulk_update_dates_script(&request);
        let stdout = runner::run_script(&script).await?;
        parse::parse_bulk_result(&stdout, total)
    }

    async fn bulk_complete(
        &self,
        request: BulkCompleteRequest,
    ) -> ThingsResult<BulkOperationResult> {
        if request.task_uuids.is_empty() {
            return Err(ThingsError::validation("Task UUIDs array cannot be empty"));
        }
        if request.task_uuids.len() > MAX_BULK_BATCH_SIZE {
            return Err(ThingsError::validation(format!(
                "Batch size {} exceeds maximum of {MAX_BULK_BATCH_SIZE}",
                request.task_uuids.len(),
            )));
        }
        for id in &request.task_uuids {
            id.as_things_native()?;
        }
        let total = request.task_uuids.len();
        let script = script::bulk_complete_script(&request);
        let stdout = runner::run_script(&script).await?;
        parse::parse_bulk_result(&stdout, total)
    }

    // ---- Projects (Phase C — implemented) ----

    async fn create_project(&self, request: CreateProjectRequest) -> ThingsResult<ThingsId> {
        let script = script::create_project_script(&request);
        let stdout = runner::run_script(&script).await?;
        parse::extract_id(&stdout)
    }

    async fn update_project(&self, request: UpdateProjectRequest) -> ThingsResult<()> {
        request.uuid.as_things_native()?;
        if let Some(a) = &request.area_uuid {
            a.as_things_native()?;
        }
        let script = script::update_project_script(&request);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn complete_project(
        &self,
        id: &ThingsId,
        child_handling: ProjectChildHandling,
    ) -> ThingsResult<()> {
        id.as_things_native()?;
        let children = self.list_project_task_uuids(id).await?;

        if children.is_empty() {
            // No children — just complete the project regardless of mode.
            let script = script::complete_project_script(id);
            runner::run_script(&script).await?;
            return Ok(());
        }

        if children.len() > MAX_BULK_BATCH_SIZE {
            return Err(ThingsError::validation(format!(
                "Batch size {} exceeds maximum of {MAX_BULK_BATCH_SIZE}",
                children.len(),
            )));
        }

        let script = match child_handling {
            ProjectChildHandling::Error => {
                return Err(ThingsError::applescript(format!(
                    "project {id} has {} child task(s); pass ProjectChildHandling::Cascade or ::Orphan",
                    children.len()
                )));
            }
            ProjectChildHandling::Cascade => script::cascade_complete_project_script(id, &children),
            ProjectChildHandling::Orphan => script::orphan_complete_project_script(id, &children),
        };
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn delete_project(
        &self,
        id: &ThingsId,
        child_handling: ProjectChildHandling,
    ) -> ThingsResult<()> {
        id.as_things_native()?;
        let children = self.list_project_task_uuids(id).await?;

        if children.is_empty() {
            let script = script::delete_project_script(id);
            runner::run_script(&script).await?;
            return Ok(());
        }

        if children.len() > MAX_BULK_BATCH_SIZE {
            return Err(ThingsError::validation(format!(
                "Batch size {} exceeds maximum of {MAX_BULK_BATCH_SIZE}",
                children.len(),
            )));
        }

        let script = match child_handling {
            ProjectChildHandling::Error => {
                return Err(ThingsError::applescript(format!(
                    "project {id} has {} child task(s); pass ProjectChildHandling::Cascade or ::Orphan",
                    children.len()
                )));
            }
            ProjectChildHandling::Cascade => script::cascade_delete_project_script(id, &children),
            ProjectChildHandling::Orphan => script::orphan_delete_project_script(id, &children),
        };
        runner::run_script(&script).await?;
        Ok(())
    }

    // ---- Areas (Phase C — implemented) ----

    async fn create_area(&self, request: CreateAreaRequest) -> ThingsResult<ThingsId> {
        let script = script::create_area_script(&request);
        let stdout = runner::run_script(&script).await?;
        parse::extract_id(&stdout)
    }

    async fn update_area(&self, request: UpdateAreaRequest) -> ThingsResult<()> {
        request.uuid.as_things_native()?;
        let script = script::update_area_script(&request);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn delete_area(&self, id: &ThingsId) -> ThingsResult<()> {
        id.as_things_native()?;
        let script = script::delete_area_script(id);
        runner::run_script(&script).await?;
        Ok(())
    }

    // ---- Tags (Phase D — implemented) ----

    async fn create_tag(
        &self,
        request: CreateTagRequest,
        force: bool,
    ) -> ThingsResult<TagCreationResult> {
        use crate::database::tag_utils::normalize_tag_title;

        if force {
            let uuid = self.create_tag_via_as(&request).await?;
            return Ok(TagCreationResult::Created { uuid, is_new: true });
        }

        // Smart flow: exact-match → similar → create.
        let normalized = normalize_tag_title(&request.title);
        if let Some(existing) = self.db.find_tag_by_normalized_title(&normalized).await? {
            return Ok(TagCreationResult::Existing {
                tag: existing,
                is_new: false,
            });
        }
        let similar = self.db.find_similar_tags(&normalized, 0.8).await?;
        if !similar.is_empty() {
            return Ok(TagCreationResult::SimilarFound {
                similar_tags: similar,
                requested_title: request.title.clone(),
            });
        }
        let uuid = self.create_tag_via_as(&request).await?;
        Ok(TagCreationResult::Created { uuid, is_new: true })
    }

    async fn update_tag(&self, request: UpdateTagRequest) -> ThingsResult<()> {
        request.uuid.as_things_native()?;
        if let Some(p) = &request.parent_uuid {
            p.as_things_native()?;
        }
        if request.shortcut.is_some() || request.parent_uuid.is_some() {
            tracing::debug!(
                tag = %request.uuid,
                "shortcut and/or parent_uuid set on UpdateTagRequest; \
                 Things AppleScript does not expose those properties on `tag`, \
                 so they are silently dropped (#136)"
            );
        }
        let script = script::update_tag_script(&request);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn delete_tag(&self, id: &ThingsId, remove_from_tasks: bool) -> ThingsResult<()> {
        id.as_things_native()?;
        if !remove_from_tasks {
            let script = script::delete_tag_script(id);
            runner::run_script(&script).await?;
            return Ok(());
        }

        // Find the tag's title and the tasks that hold it. AppleScriptBackend
        // implements `remove_from_tasks=true` correctly, while the sqlx path
        // has this as a TODO — divergent capability, called out in the PR.
        use crate::database::tag_utils::normalize_tag_title;
        let title = self.read_tag_title(id).await?;
        let normalized = normalize_tag_title(&title);
        let candidates = self.list_tasks_with_tag_title(&title).await?;

        if !candidates.is_empty() {
            if candidates.len() > MAX_BULK_BATCH_SIZE {
                return Err(ThingsError::validation(format!(
                    "Cannot remove tag from {} tasks; exceeds maximum of {MAX_BULK_BATCH_SIZE}",
                    candidates.len(),
                )));
            }
            let items: Vec<(ThingsId, String)> = candidates
                .into_iter()
                .map(|(task_id, tags)| {
                    let new_tags: Vec<String> = tags
                        .into_iter()
                        .filter(|t| normalize_tag_title(t) != normalized)
                        .collect();
                    (task_id, new_tags.join(", "))
                })
                .collect();

            let total = items.len();
            let rewrite_script = script::bulk_set_task_tag_names_script(&items);
            let stdout = runner::run_script(&rewrite_script).await?;
            let result = parse::parse_bulk_result(&stdout, total)?;
            if !result.success {
                return Err(ThingsError::applescript(format!(
                    "delete_tag(remove_from_tasks=true): per-task rewrite failed; \
                     tag {id} was NOT deleted. {}",
                    result.message
                )));
            }
        }

        let delete_script = script::delete_tag_script(id);
        runner::run_script(&delete_script).await?;
        Ok(())
    }

    async fn merge_tags(&self, source_id: &ThingsId, target_id: &ThingsId) -> ThingsResult<()> {
        use crate::database::tag_utils::normalize_tag_title;

        source_id.as_things_native()?;
        target_id.as_things_native()?;

        if source_id == target_id {
            return Err(ThingsError::validation(
                "merge_tags: source and target must differ",
            ));
        }

        let source_title = self.read_tag_title(source_id).await?;
        let target_title = self.read_tag_title(target_id).await?;
        let source_normalized = normalize_tag_title(&source_title);
        let target_normalized = normalize_tag_title(&target_title);

        let candidates = self.list_tasks_with_tag_title(&source_title).await?;
        if !candidates.is_empty() {
            if candidates.len() > MAX_BULK_BATCH_SIZE {
                return Err(ThingsError::validation(format!(
                    "Cannot merge tag across {} tasks; exceeds maximum of {MAX_BULK_BATCH_SIZE}",
                    candidates.len(),
                )));
            }
            let items: Vec<(ThingsId, String)> = candidates
                .into_iter()
                .map(|(task_id, tags)| {
                    let mut new_tags: Vec<String> = Vec::with_capacity(tags.len());
                    for t in tags {
                        let n = normalize_tag_title(&t);
                        if n == source_normalized {
                            // Replace source with target, deduping.
                            if !new_tags
                                .iter()
                                .any(|nt| normalize_tag_title(nt) == target_normalized)
                            {
                                new_tags.push(target_title.clone());
                            }
                        } else if n == target_normalized {
                            // Keep an existing target reference, but de-dup
                            // against any earlier insertion.
                            if !new_tags
                                .iter()
                                .any(|nt| normalize_tag_title(nt) == target_normalized)
                            {
                                new_tags.push(t);
                            }
                        } else {
                            new_tags.push(t);
                        }
                    }
                    (task_id, new_tags.join(", "))
                })
                .collect();

            let total = items.len();
            let rewrite_script = script::bulk_set_task_tag_names_script(&items);
            let stdout = runner::run_script(&rewrite_script).await?;
            let result = parse::parse_bulk_result(&stdout, total)?;
            if !result.success {
                return Err(ThingsError::applescript(format!(
                    "merge_tags: per-task rewrite failed; source tag {source_id} was NOT deleted. {}",
                    result.message
                )));
            }
        }

        // Source tag is no longer referenced by any task — safe to delete.
        let delete_script = script::delete_tag_script(source_id);
        runner::run_script(&delete_script).await?;
        Ok(())
    }

    async fn add_tag_to_task(
        &self,
        task_id: &ThingsId,
        tag_title: &str,
    ) -> ThingsResult<TagAssignmentResult> {
        use crate::database::tag_utils::normalize_tag_title;

        task_id.as_things_native()?;

        let normalized = normalize_tag_title(tag_title);

        let (resolved_title, resolved_uuid) =
            if let Some(existing) = self.db.find_tag_by_normalized_title(&normalized).await? {
                (existing.title, existing.uuid)
            } else {
                let similar = self.db.find_similar_tags(&normalized, 0.8).await?;
                if !similar.is_empty() {
                    return Ok(TagAssignmentResult::Suggestions {
                        similar_tags: similar,
                    });
                }
                // No existing match and no similar tags — auto-create. Mirrors
                // `ThingsDatabase::add_tag_to_task` (`database/core.rs:2792-2802`).
                let create_req = CreateTagRequest {
                    title: tag_title.to_string(),
                    shortcut: None,
                    parent_uuid: None,
                };
                let new_id = self.create_tag_via_as(&create_req).await?;
                (tag_title.to_string(), new_id)
            };

        let current = self.read_task_tag_titles(task_id).await?;
        let already_present = current.iter().any(|t| normalize_tag_title(t) == normalized);
        if !already_present {
            let mut new_list = current;
            new_list.push(resolved_title);
            let joined = new_list.join(", ");
            let script = script::set_task_tag_names_script(task_id, &joined);
            runner::run_script(&script).await?;
        }
        Ok(TagAssignmentResult::Assigned {
            tag_uuid: resolved_uuid,
        })
    }

    async fn remove_tag_from_task(&self, task_id: &ThingsId, tag_title: &str) -> ThingsResult<()> {
        use crate::database::tag_utils::normalize_tag_title;

        task_id.as_things_native()?;

        let normalized = normalize_tag_title(tag_title);
        let current = self.read_task_tag_titles(task_id).await?;
        let original_len = current.len();
        let new_list: Vec<String> = current
            .into_iter()
            .filter(|t| normalize_tag_title(t) != normalized)
            .collect();
        if new_list.len() == original_len {
            return Ok(());
        }
        let joined = new_list.join(", ");
        let script = script::set_task_tag_names_script(task_id, &joined);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn set_task_tags(
        &self,
        task_id: &ThingsId,
        tag_titles: Vec<String>,
    ) -> ThingsResult<Vec<TagMatch>> {
        use crate::database::tag_utils::normalize_tag_title;

        task_id.as_things_native()?;

        // Intentionally asymmetric with add_tag_to_task: similar-tag suggestions
        // accumulate for the caller but never block the write — every title is
        // auto-created if absent, matching ThingsDatabase::set_task_tags semantics.
        let mut suggestions: Vec<TagMatch> = Vec::new();
        let mut resolved: Vec<String> = Vec::with_capacity(tag_titles.len());

        for title in tag_titles {
            let normalized = normalize_tag_title(&title);
            if let Some(existing) = self.db.find_tag_by_normalized_title(&normalized).await? {
                resolved.push(existing.title);
                continue;
            }
            let similar = self.db.find_similar_tags(&normalized, 0.8).await?;
            if !similar.is_empty() {
                suggestions.extend(similar);
            }
            // Auto-create on miss — mirrors `ThingsDatabase::set_task_tags`
            // (`database/core.rs:2966-2981`). Suggestions are accumulated
            // for the caller's review but do not block creation.
            let create_req = CreateTagRequest {
                title: title.clone(),
                shortcut: None,
                parent_uuid: None,
            };
            self.create_tag_via_as(&create_req).await?;
            resolved.push(title);
        }

        let joined = resolved.join(", ");
        let script = script::set_task_tag_names_script(task_id, &joined);
        runner::run_script(&script).await?;
        Ok(suggestions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Constructing the backend should not touch osascript or the DB.
    #[tokio::test]
    async fn new_does_not_spawn_osascript() {
        let db = Arc::new(
            ThingsDatabase::from_connection_string("sqlite::memory:")
                .await
                .expect("in-memory db"),
        );
        let _backend = AppleScriptBackend::new(db);
    }

    /// `create_tag(force=false)` returns `Existing` for a case-insensitive
    /// exact match without ever spawning osascript. Read-side decision is
    /// pure DB.
    #[tokio::test]
    async fn create_tag_returns_existing_on_exact_case_insensitive_match() {
        let (db, _tmp) = crate::test_utils::create_test_database_and_connect()
            .await
            .expect("test db");
        db.create_tag_force(CreateTagRequest {
            title: "Work".into(),
            shortcut: None,
            parent_uuid: None,
        })
        .await
        .expect("seed tag");
        let backend = AppleScriptBackend::new(Arc::new(db));

        let result = backend
            .create_tag(
                CreateTagRequest {
                    title: "WORK".into(),
                    shortcut: None,
                    parent_uuid: None,
                },
                false,
            )
            .await
            .expect("smart flow");
        match result {
            TagCreationResult::Existing { tag, is_new } => {
                assert_eq!(tag.title, "Work");
                assert!(!is_new);
            }
            other => panic!("expected Existing, got {other:?}"),
        }
    }

    /// `create_tag(force=false)` returns `SimilarFound` when no exact match
    /// but Levenshtein-≥0.8 candidates exist. Pure DB read; no osascript.
    #[tokio::test]
    async fn create_tag_returns_similar_found_on_fuzzy_match() {
        let (db, _tmp) = crate::test_utils::create_test_database_and_connect()
            .await
            .expect("test db");
        db.create_tag_force(CreateTagRequest {
            title: "important".into(),
            shortcut: None,
            parent_uuid: None,
        })
        .await
        .expect("seed tag");
        let backend = AppleScriptBackend::new(Arc::new(db));

        let result = backend
            .create_tag(
                CreateTagRequest {
                    // 1-char typo from "important".
                    title: "importnt".into(),
                    shortcut: None,
                    parent_uuid: None,
                },
                false,
            )
            .await
            .expect("smart flow");
        match result {
            TagCreationResult::SimilarFound {
                similar_tags,
                requested_title,
            } => {
                assert_eq!(requested_title, "importnt");
                assert!(
                    similar_tags.iter().any(|m| m.tag.title == "important"),
                    "should suggest 'important' as similar"
                );
            }
            other => panic!("expected SimilarFound, got {other:?}"),
        }
    }

    /// `add_tag_to_task` returns `Suggestions` and does NOT spawn osascript
    /// when the requested title is ambiguous against existing tags.
    #[tokio::test]
    async fn add_tag_to_task_returns_suggestions_when_ambiguous() {
        let (db, _tmp) = crate::test_utils::create_test_database_and_connect()
            .await
            .expect("test db");
        db.create_tag_force(CreateTagRequest {
            title: "important".into(),
            shortcut: None,
            parent_uuid: None,
        })
        .await
        .expect("seed tag");
        let backend = AppleScriptBackend::new(Arc::new(db));

        let task_id = ThingsId::new_things_native();
        let result = backend
            .add_tag_to_task(&task_id, "importnt")
            .await
            .expect("smart flow");
        match result {
            TagAssignmentResult::Suggestions { similar_tags } => {
                assert!(similar_tags.iter().any(|m| m.tag.title == "important"));
            }
            other => panic!("expected Suggestions, got {other:?}"),
        }
    }

    /// `list_tasks_with_tag_title` (the read-side helper backing
    /// `delete_tag(remove_from_tasks=true)`) correctly finds tasks whose
    /// `cachedTags` blob contains the target tag, and deserializes the full
    /// tag list per task. Also confirms that an unrelated tag name returns
    /// no results.
    #[tokio::test]
    async fn delete_tag_remove_from_tasks_finds_tagged_tasks() {
        let (db, _tmp) = crate::test_utils::create_test_database_and_connect()
            .await
            .expect("test db");

        let task_id = ThingsId::new_things_native();
        let blob =
            crate::database::serialize_tags_to_blob(&["Work".to_string(), "Personal".to_string()])
                .expect("serialize");
        let now = 1_700_000_000.0_f64;
        sqlx::query(
            "INSERT INTO TMTask \
             (uuid, title, type, status, trashed, cachedTags, creationDate, userModificationDate) \
             VALUES (?, 'Tagged Task', 0, 0, 0, ?, ?, ?)",
        )
        .bind(task_id.as_str())
        .bind(&blob)
        .bind(now)
        .bind(now)
        .execute(&db.pool)
        .await
        .expect("insert tagged task");

        let backend = AppleScriptBackend::new(Arc::new(db));

        let found = backend
            .list_tasks_with_tag_title("Work")
            .await
            .expect("query");
        assert_eq!(found.len(), 1, "should find exactly one tagged task");
        let (found_id, found_tags) = &found[0];
        assert_eq!(found_id.as_str(), task_id.as_str());
        assert_eq!(found_tags, &["Work", "Personal"]);

        let not_found = backend
            .list_tasks_with_tag_title("Nonexistent")
            .await
            .expect("query nonexistent");
        assert!(not_found.is_empty(), "no tasks should match an absent tag");
    }

    /// Tag titles containing SQL LIKE wildcards (`_`, `%`) must still match
    /// their exact tasks — the pre-filter escapes wildcards via `ESCAPE '\'`.
    /// Regression: previously `replace('_', "\\_")` produced a literal `\_`
    /// that didn't match because the LIKE clause had no `ESCAPE`.
    #[tokio::test]
    async fn list_tasks_with_tag_title_handles_underscore_in_tag_name() {
        let (db, _tmp) = crate::test_utils::create_test_database_and_connect()
            .await
            .expect("test db");

        let task_id = ThingsId::new_things_native();
        let blob =
            crate::database::serialize_tags_to_blob(&["to_do".to_string()]).expect("serialize");
        let now = 1_700_000_000.0_f64;
        sqlx::query(
            "INSERT INTO TMTask \
             (uuid, title, type, status, trashed, cachedTags, creationDate, userModificationDate) \
             VALUES (?, 'Task with underscored tag', 0, 0, 0, ?, ?, ?)",
        )
        .bind(task_id.as_str())
        .bind(&blob)
        .bind(now)
        .bind(now)
        .execute(&db.pool)
        .await
        .expect("insert");

        let backend = AppleScriptBackend::new(Arc::new(db));

        let found = backend
            .list_tasks_with_tag_title("to_do")
            .await
            .expect("query");
        assert_eq!(found.len(), 1, "should match tag name with underscore");
        assert_eq!(found[0].0.as_str(), task_id.as_str());
    }

    /// `merge_tags` rejects identical source/target with a Validation error.
    /// No DB read or osascript invocation needed.
    #[tokio::test]
    async fn merge_tags_rejects_identical_source_and_target() {
        let db = Arc::new(
            ThingsDatabase::from_connection_string("sqlite::memory:")
                .await
                .expect("in-memory db"),
        );
        let backend = AppleScriptBackend::new(db);
        let id = ThingsId::new_things_native();
        let err = backend
            .merge_tags(&id, &id)
            .await
            .expect_err("same source/target");
        assert!(matches!(err, ThingsError::Validation { .. }));
    }

    /// Validation errors fire before osascript spawn for empty / oversize bulk requests.
    #[tokio::test]
    async fn bulk_validation_rejects_empty_and_oversize() {
        let db = Arc::new(
            ThingsDatabase::from_connection_string("sqlite::memory:")
                .await
                .expect("in-memory db"),
        );
        let backend = AppleScriptBackend::new(db);

        let err = backend
            .bulk_complete(BulkCompleteRequest { task_uuids: vec![] })
            .await
            .expect_err("empty");
        assert!(matches!(err, ThingsError::Validation { .. }));

        let err = backend
            .bulk_delete(BulkDeleteRequest {
                task_uuids: (0..1001).map(|_| ThingsId::new_things_native()).collect(),
            })
            .await
            .expect_err("oversize");
        assert!(matches!(err, ThingsError::Validation { .. }));

        let err = backend
            .bulk_move(BulkMoveRequest {
                task_uuids: vec![ThingsId::new_things_native()],
                project_uuid: None,
                area_uuid: None,
            })
            .await
            .expect_err("missing destination");
        assert!(matches!(err, ThingsError::Validation { .. }));
    }

    // ============================================================================
    // ID format guard (#148) — every mutation method that takes a `ThingsId`
    // must reject hyphenated UUIDs *before* invoking osascript, so the user
    // sees an actionable Validation error instead of AppleScript's opaque
    // `-1728`. The guard fires synchronously before any `runner::run_script`
    // call, which is what makes these tests deterministic in CI without a
    // live Things 3 install.
    // ============================================================================

    fn hyphenated_uuid() -> ThingsId {
        "9d3f1e44-5c2a-4b8e-9c1f-7e2d8a4b3c5e".parse().unwrap()
    }

    async fn guard_test_backend() -> AppleScriptBackend {
        let db = Arc::new(
            ThingsDatabase::from_connection_string("sqlite::memory:")
                .await
                .expect("in-memory db"),
        );
        AppleScriptBackend::new(db)
    }

    fn assert_native_format_validation(err: ThingsError) {
        match &err {
            ThingsError::Validation { .. } => {
                assert!(
                    err.to_string().contains("not in Things native format"),
                    "wrong validation error: {err}"
                );
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn complete_task_rejects_hyphenated_uuid() {
        let backend = guard_test_backend().await;
        let err = backend
            .complete_task(&hyphenated_uuid())
            .await
            .expect_err("guard should fire");
        assert_native_format_validation(err);
    }

    #[tokio::test]
    async fn update_task_rejects_hyphenated_uuid_in_request() {
        let backend = guard_test_backend().await;
        let req = UpdateTaskRequest {
            uuid: hyphenated_uuid(),
            title: Some("x".into()),
            notes: None,
            start_date: None,
            deadline: None,
            status: None,
            project_uuid: None,
            area_uuid: None,
            tags: None,
        };
        let err = backend
            .update_task(req)
            .await
            .expect_err("guard should fire on request.uuid");
        assert_native_format_validation(err);
    }

    #[tokio::test]
    async fn update_task_rejects_hyphenated_secondary_id() {
        // request.uuid is native, but a secondary optional ID is hyphenated —
        // the guard must catch it too.
        let backend = guard_test_backend().await;
        let req = UpdateTaskRequest {
            uuid: ThingsId::new_things_native(),
            title: Some("x".into()),
            notes: None,
            start_date: None,
            deadline: None,
            status: None,
            project_uuid: Some(hyphenated_uuid()),
            area_uuid: None,
            tags: None,
        };
        let err = backend
            .update_task(req)
            .await
            .expect_err("guard should fire on request.project_uuid");
        assert_native_format_validation(err);
    }

    #[tokio::test]
    async fn bulk_delete_rejects_hyphenated_uuid_in_vec() {
        let backend = guard_test_backend().await;
        let req = BulkDeleteRequest {
            // Mix of valid + invalid; guard must fail-fast on the bad one.
            task_uuids: vec![ThingsId::new_things_native(), hyphenated_uuid()],
        };
        let err = backend
            .bulk_delete(req)
            .await
            .expect_err("guard should fire on the bad ID");
        assert_native_format_validation(err);
    }

    // Live lifecycle tests live in `libs/things3-core/tests/applescript_live.rs`
    // (Phase E, #137). They are gated by `THINGS3_LIVE_TESTS=1` and run with
    // `--test-threads=1`.
}
