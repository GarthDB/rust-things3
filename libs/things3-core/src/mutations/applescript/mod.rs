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
//! ## Phase B scope (#134)
//!
//! Only the five most-used task methods are implemented end-to-end:
//! `create_task`, `update_task`, `complete_task`, `uncomplete_task`,
//! `delete_task`. The remaining 16 trait methods return
//! [`ThingsError::AppleScript`] with a message pointing at the issue tracking
//! the relevant phase. Until #125 ships, no production code constructs an
//! `AppleScriptBackend`, so these stubs are unreachable in the default config.

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
}

const MAX_BULK_BATCH_SIZE: usize = 1000;

/// Helper: every stub method gets the same error shape so callers can grep
/// for the phase that adds it.
fn not_yet_implemented(method: &str, phase: &str, issue: &str) -> ThingsError {
    ThingsError::applescript(format!(
        "{method} is not yet implemented in AppleScriptBackend ({phase} — {issue})"
    ))
}

#[async_trait]
impl MutationBackend for AppleScriptBackend {
    // ---- Tasks (Phase B — implemented) ----

    async fn create_task(&self, request: CreateTaskRequest) -> ThingsResult<ThingsId> {
        let script = script::create_task_script(&request);
        let stdout = runner::run_script(&script).await?;
        parse::extract_id(&stdout)
    }

    async fn update_task(&self, request: UpdateTaskRequest) -> ThingsResult<()> {
        let script = script::update_task_script(&request);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn complete_task(&self, id: &ThingsId) -> ThingsResult<()> {
        let script = script::complete_task_script(id);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn uncomplete_task(&self, id: &ThingsId) -> ThingsResult<()> {
        let script = script::uncomplete_task_script(id);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn delete_task(
        &self,
        id: &ThingsId,
        child_handling: DeleteChildHandling,
    ) -> ThingsResult<()> {
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
        let script = script::update_project_script(&request);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn complete_project(
        &self,
        id: &ThingsId,
        child_handling: ProjectChildHandling,
    ) -> ThingsResult<()> {
        let children = self.list_project_task_uuids(id).await?;

        if children.is_empty() {
            // No children — just complete the project regardless of mode.
            let script = script::complete_project_script(id);
            runner::run_script(&script).await?;
            return Ok(());
        }

        if children.len() > MAX_BULK_BATCH_SIZE {
            return Err(ThingsError::validation(format!(
                "project {id} has {} child task(s), which exceeds maximum of {MAX_BULK_BATCH_SIZE}",
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
        let children = self.list_project_task_uuids(id).await?;

        if children.is_empty() {
            let script = script::delete_project_script(id);
            runner::run_script(&script).await?;
            return Ok(());
        }

        if children.len() > MAX_BULK_BATCH_SIZE {
            return Err(ThingsError::validation(format!(
                "project {id} has {} child task(s), which exceeds maximum of {MAX_BULK_BATCH_SIZE}",
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
        let script = script::update_area_script(&request);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn delete_area(&self, id: &ThingsId) -> ThingsResult<()> {
        let script = script::delete_area_script(id);
        runner::run_script(&script).await?;
        Ok(())
    }

    // ---- Tags (Phase D — stubbed) ----

    async fn create_tag(
        &self,
        _request: CreateTagRequest,
        _force: bool,
    ) -> ThingsResult<TagCreationResult> {
        Err(not_yet_implemented("create_tag", "Phase D", "#136"))
    }

    async fn update_tag(&self, _request: UpdateTagRequest) -> ThingsResult<()> {
        Err(not_yet_implemented("update_tag", "Phase D", "#136"))
    }

    async fn delete_tag(&self, _id: &ThingsId, _remove_from_tasks: bool) -> ThingsResult<()> {
        Err(not_yet_implemented("delete_tag", "Phase D", "#136"))
    }

    async fn merge_tags(&self, _source_id: &ThingsId, _target_id: &ThingsId) -> ThingsResult<()> {
        Err(not_yet_implemented("merge_tags", "Phase D", "#136"))
    }

    async fn add_tag_to_task(
        &self,
        _task_id: &ThingsId,
        _tag_title: &str,
    ) -> ThingsResult<TagAssignmentResult> {
        Err(not_yet_implemented("add_tag_to_task", "Phase D", "#136"))
    }

    async fn remove_tag_from_task(
        &self,
        _task_id: &ThingsId,
        _tag_title: &str,
    ) -> ThingsResult<()> {
        Err(not_yet_implemented(
            "remove_tag_from_task",
            "Phase D",
            "#136",
        ))
    }

    async fn set_task_tags(
        &self,
        _task_id: &ThingsId,
        _tag_titles: Vec<String>,
    ) -> ThingsResult<Vec<TagMatch>> {
        Err(not_yet_implemented("set_task_tags", "Phase D", "#136"))
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

    /// Every remaining Phase-D stub returns AppleScript error pointing at the
    /// tracking issue. Phase B (tasks) and Phase C (projects/areas/bulk) are
    /// fully implemented; tags remain stubbed.
    #[tokio::test]
    async fn unimplemented_tag_methods_return_phase_error() {
        let db = Arc::new(
            ThingsDatabase::from_connection_string("sqlite::memory:")
                .await
                .expect("in-memory db"),
        );
        let backend = AppleScriptBackend::new(db);

        let err = backend
            .create_tag(
                CreateTagRequest {
                    title: "x".into(),
                    shortcut: None,
                    parent_uuid: None,
                },
                false,
            )
            .await
            .expect_err("stub");
        match err {
            ThingsError::AppleScript { message } => {
                assert!(message.contains("create_tag"));
                assert!(message.contains("Phase D"));
                assert!(message.contains("#136"));
            }
            _ => panic!("expected AppleScript error, got {err:?}"),
        }
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
                task_uuids: (0..1001).map(|_| ThingsId::new_v4()).collect(),
            })
            .await
            .expect_err("oversize");
        assert!(matches!(err, ThingsError::Validation { .. }));

        let err = backend
            .bulk_move(BulkMoveRequest {
                task_uuids: vec![ThingsId::new_v4()],
                project_uuid: None,
                area_uuid: None,
            })
            .await
            .expect_err("missing destination");
        assert!(matches!(err, ThingsError::Validation { .. }));
    }

    /// Full create→update→complete→delete lifecycle test against the user's real Things 3 install.
    ///
    /// With ID unification (#139) landed, `create_task` now returns a [`ThingsId`] that
    /// correctly round-trips through both Things 3 native IDs (21–22 char base62) and
    /// RFC-4122 UUIDs. The returned ID is immediately usable in subsequent trait calls.
    ///
    /// Run explicitly with:
    ///
    /// ```text
    /// THINGS3_LIVE_TESTS=1 cargo test -p things3-core \
    ///     mutations::applescript::tests::task_lifecycle_round_trip \
    ///     -- --ignored --nocapture
    /// ```
    ///
    /// Creates a clearly-marked test task in the user's Things 3 inbox and
    /// deletes it before returning. If the test panics mid-run, a stale task
    /// may remain in the inbox.
    #[tokio::test]
    #[ignore = "requires Things 3 + Automation permission; set THINGS3_LIVE_TESTS=1"]
    async fn task_lifecycle_round_trip() {
        if std::env::var("THINGS3_LIVE_TESTS").as_deref() != Ok("1") {
            return;
        }

        let db_path = crate::database::get_default_database_path();
        let db = Arc::new(
            ThingsDatabase::new(&db_path)
                .await
                .expect("failed to open Things 3 database"),
        );
        let backend = AppleScriptBackend::new(Arc::clone(&db));

        let title = format!(
            "rust-things3 lifecycle test {}",
            chrono::Utc::now().timestamp()
        );

        // --- create ---
        let id = backend
            .create_task(CreateTaskRequest {
                title: title.clone(),
                task_type: None,
                notes: Some("with \"quotes\" and\nnewline and \\backslash".into()),
                start_date: None,
                deadline: None,
                project_uuid: None,
                area_uuid: None,
                parent_uuid: None,
                tags: None,
                status: None,
            })
            .await
            .expect("create_task should succeed");

        assert!(
            !id.as_str().is_empty(),
            "returned ThingsId should not be empty"
        );
        println!("created task id: {id}");

        // --- update ---
        backend
            .update_task(crate::models::UpdateTaskRequest {
                uuid: id.clone(),
                title: Some(format!("{title} (updated)")),
                notes: None,
                start_date: None,
                deadline: None,
                project_uuid: None,
                area_uuid: None,
                tags: None,
                status: None,
            })
            .await
            .expect("update_task should succeed");

        // --- complete ---
        backend
            .complete_task(&id)
            .await
            .expect("complete_task should succeed");

        // --- delete ---
        backend
            .delete_task(&id, DeleteChildHandling::Error)
            .await
            .expect("delete_task should succeed");
    }
}
