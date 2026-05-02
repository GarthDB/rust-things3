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
use uuid::Uuid;

use super::MutationBackend;
use crate::database::ThingsDatabase;
use crate::error::{Result as ThingsResult, ThingsError};
use crate::models::{
    BulkCompleteRequest, BulkCreateTasksRequest, BulkDeleteRequest, BulkMoveRequest,
    BulkOperationResult, BulkUpdateDatesRequest, CreateAreaRequest, CreateProjectRequest,
    CreateTagRequest, CreateTaskRequest, DeleteChildHandling, ProjectChildHandling,
    TagAssignmentResult, TagCreationResult, TagMatch, UpdateAreaRequest, UpdateProjectRequest,
    UpdateTagRequest, UpdateTaskRequest,
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

    /// Read the UUIDs of every non-trashed direct subtask of `parent` via
    /// sqlx. Read-only, CulturedCode-safe.
    async fn list_subtask_uuids(&self, parent: &Uuid) -> ThingsResult<Vec<Uuid>> {
        let rows = sqlx::query("SELECT uuid FROM TMTask WHERE heading = ? AND trashed = 0")
            .bind(parent.to_string())
            .fetch_all(&self.db.pool)
            .await
            .map_err(|e| {
                ThingsError::applescript(format!("failed to query subtasks of {parent}: {e}"))
            })?;
        rows.into_iter()
            .map(|row| {
                let s: String = row.get("uuid");
                Uuid::parse_str(&s).map_err(|e| {
                    ThingsError::applescript(format!("invalid subtask uuid in DB: {e}"))
                })
            })
            .collect()
    }
}

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

    /// # Known limitation (#139)
    ///
    /// Things 3 IDs are 21–22-char base62-style strings (e.g.
    /// `R4t2G8Q63aGZq4epMHNeCr`), not RFC-4122 UUIDs. [`parse::extract_id`]
    /// will return `Err` for these; today this method only succeeds when the
    /// caller doesn't actually care about the returned UUID. The fix — a
    /// `TaskId` newtype that round-trips both formats — is tracked in #139,
    /// which is a blocker for #125 (default-backend switch).
    async fn create_task(&self, request: CreateTaskRequest) -> ThingsResult<Uuid> {
        let script = script::create_task_script(&request);
        let stdout = runner::run_script(&script).await?;
        parse::extract_id(&stdout)
    }

    async fn update_task(&self, request: UpdateTaskRequest) -> ThingsResult<()> {
        let script = script::update_task_script(&request);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn complete_task(&self, uuid: &Uuid) -> ThingsResult<()> {
        let script = script::complete_task_script(uuid);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn uncomplete_task(&self, uuid: &Uuid) -> ThingsResult<()> {
        let script = script::uncomplete_task_script(uuid);
        runner::run_script(&script).await?;
        Ok(())
    }

    async fn delete_task(
        &self,
        uuid: &Uuid,
        child_handling: DeleteChildHandling,
    ) -> ThingsResult<()> {
        let children = self.list_subtask_uuids(uuid).await?;

        if !children.is_empty() {
            match child_handling {
                DeleteChildHandling::Error => {
                    return Err(ThingsError::applescript(format!(
                        "task {uuid} has {} subtask(s); pass DeleteChildHandling::Cascade or ::Orphan",
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

        let script = script::delete_task_script(uuid);
        runner::run_script(&script).await?;
        Ok(())
    }

    // ---- Tasks (Phase C — stubbed) ----

    async fn bulk_create_tasks(
        &self,
        _request: BulkCreateTasksRequest,
    ) -> ThingsResult<BulkOperationResult> {
        Err(not_yet_implemented("bulk_create_tasks", "Phase C", "#135"))
    }

    async fn bulk_delete(&self, _request: BulkDeleteRequest) -> ThingsResult<BulkOperationResult> {
        Err(not_yet_implemented("bulk_delete", "Phase C", "#135"))
    }

    async fn bulk_move(&self, _request: BulkMoveRequest) -> ThingsResult<BulkOperationResult> {
        Err(not_yet_implemented("bulk_move", "Phase C", "#135"))
    }

    async fn bulk_update_dates(
        &self,
        _request: BulkUpdateDatesRequest,
    ) -> ThingsResult<BulkOperationResult> {
        Err(not_yet_implemented("bulk_update_dates", "Phase C", "#135"))
    }

    async fn bulk_complete(
        &self,
        _request: BulkCompleteRequest,
    ) -> ThingsResult<BulkOperationResult> {
        Err(not_yet_implemented("bulk_complete", "Phase C", "#135"))
    }

    // ---- Projects (Phase C — stubbed) ----

    async fn create_project(&self, _request: CreateProjectRequest) -> ThingsResult<Uuid> {
        Err(not_yet_implemented("create_project", "Phase C", "#135"))
    }

    async fn update_project(&self, _request: UpdateProjectRequest) -> ThingsResult<()> {
        Err(not_yet_implemented("update_project", "Phase C", "#135"))
    }

    async fn complete_project(
        &self,
        _uuid: &Uuid,
        _child_handling: ProjectChildHandling,
    ) -> ThingsResult<()> {
        Err(not_yet_implemented("complete_project", "Phase C", "#135"))
    }

    async fn delete_project(
        &self,
        _uuid: &Uuid,
        _child_handling: ProjectChildHandling,
    ) -> ThingsResult<()> {
        Err(not_yet_implemented("delete_project", "Phase C", "#135"))
    }

    // ---- Areas (Phase C — stubbed) ----

    async fn create_area(&self, _request: CreateAreaRequest) -> ThingsResult<Uuid> {
        Err(not_yet_implemented("create_area", "Phase C", "#135"))
    }

    async fn update_area(&self, _request: UpdateAreaRequest) -> ThingsResult<()> {
        Err(not_yet_implemented("update_area", "Phase C", "#135"))
    }

    async fn delete_area(&self, _uuid: &Uuid) -> ThingsResult<()> {
        Err(not_yet_implemented("delete_area", "Phase C", "#135"))
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

    async fn delete_tag(&self, _uuid: &Uuid, _remove_from_tasks: bool) -> ThingsResult<()> {
        Err(not_yet_implemented("delete_tag", "Phase D", "#136"))
    }

    async fn merge_tags(&self, _source: &Uuid, _target: &Uuid) -> ThingsResult<()> {
        Err(not_yet_implemented("merge_tags", "Phase D", "#136"))
    }

    async fn add_tag_to_task(
        &self,
        _task_uuid: &Uuid,
        _tag_title: &str,
    ) -> ThingsResult<TagAssignmentResult> {
        Err(not_yet_implemented("add_tag_to_task", "Phase D", "#136"))
    }

    async fn remove_tag_from_task(&self, _task_uuid: &Uuid, _tag_title: &str) -> ThingsResult<()> {
        Err(not_yet_implemented(
            "remove_tag_from_task",
            "Phase D",
            "#136",
        ))
    }

    async fn set_task_tags(
        &self,
        _task_uuid: &Uuid,
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

    /// Every Phase-C / Phase-D stub returns AppleScript error pointing at the
    /// tracking issue. Spot-check one per category.
    #[tokio::test]
    async fn unimplemented_methods_return_phase_error() {
        let db = Arc::new(
            ThingsDatabase::from_connection_string("sqlite::memory:")
                .await
                .expect("in-memory db"),
        );
        let backend = AppleScriptBackend::new(db);

        let err = backend
            .create_project(CreateProjectRequest {
                title: "x".into(),
                notes: None,
                area_uuid: None,
                start_date: None,
                deadline: None,
                tags: None,
            })
            .await
            .expect_err("stub");
        match err {
            ThingsError::AppleScript { message } => {
                assert!(message.contains("create_project"));
                assert!(message.contains("Phase C"));
                assert!(message.contains("#135"));
            }
            _ => panic!("expected AppleScript error, got {err:?}"),
        }

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

    /// Smoke test against the user's real Things 3 install — verifies the
    /// AppleScript plumbing reaches Things 3 and a `make new to do` script
    /// executes. Does NOT verify the returned ID round-trips back through the
    /// trait, because Things 3 IDs are 21–22-char base62-style strings (e.g.
    /// `R4t2G8Q63aGZq4epMHNeCr`), not RFC-4122 UUIDs — see the
    /// "Known limitation" note on [`AppleScriptBackend::create_task`].
    ///
    /// The full lifecycle test (`create → update → complete → delete`,
    /// asserting via DB reads) lands in Phase E (#137) once the ID-unification
    /// blocker on #125 is resolved.
    ///
    /// Run explicitly with:
    ///
    /// ```text
    /// cargo test -p things3-core mutations::applescript::tests::create_task_smoke \
    ///     -- --ignored --nocapture
    /// ```
    ///
    /// Creates a clearly-marked test task in the user's Things 3 inbox; you'll
    /// want to delete it manually after running.
    #[tokio::test]
    #[ignore = "requires Things 3 + Automation permission; mutates the user's real DB"]
    async fn create_task_smoke() {
        let title = format!(
            "rust-things3 phase B smoke test {}",
            chrono::Utc::now().timestamp()
        );
        let req = CreateTaskRequest {
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
        };

        let stdout = runner::run_script(&script::create_task_script(&req))
            .await
            .expect("osascript should reach Things 3");

        // Things 3 returns a 21- or 22-char base62-style ID. Until ID
        // unification lands, we don't try to parse this back through the
        // public API — just assert the shape so a script regression would
        // fail loudly here.
        let id = stdout.trim();
        assert!(
            id.len() == 21 || id.len() == 22,
            "expected Things ID of length 21–22, got {}: {id:?}",
            id.len(),
        );
        assert!(
            id.chars().all(|c| c.is_ascii_alphanumeric()),
            "expected base62-style id, got: {id:?}"
        );
    }
}
