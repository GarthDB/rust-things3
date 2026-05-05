//! Live integration tests for `AppleScriptBackend` against a real Things 3
//! install (Phase E, #137).
//!
//! Every test in this file is `#[ignore]` AND env-gated on
//! `THINGS3_LIVE_TESTS=1`. A normal `cargo test --workspace` run never invokes
//! them. The `#![cfg(target_os = "macos")]` at the top of the file means
//! Linux CI compiles an empty crate.
//!
//! Run with:
//!
//! ```bash
//! THINGS3_LIVE_TESTS=1 cargo test -p things3-core --test applescript_live \
//!     -- --ignored --test-threads=1
//! ```
//!
//! `--test-threads=1` is required: every test mutates the single shared
//! Things 3 instance, and concurrent runs would race.
//!
//! Each test creates entities with a UUID-prefixed title so leftovers from a
//! crash are easy to spot in Things 3, and uses a `Guard` whose `Drop` impl
//! deletes the entity even on panic.

#![cfg(target_os = "macos")]

use std::sync::Arc;

use things3_core::{
    mutations::{AppleScriptBackend, MutationBackend},
    CreateAreaRequest, CreateProjectRequest, CreateTagRequest, CreateTaskRequest,
    DeleteChildHandling, ProjectChildHandling, ThingsDatabase, ThingsId, UpdateAreaRequest,
    UpdateProjectRequest, UpdateTagRequest, UpdateTaskRequest,
};

/// Skip the test body if `THINGS3_LIVE_TESTS=1` is not set in the env.
/// Lets `cargo test --include-ignored` work for cross-checking gating
/// without firing osascript.
fn live_tests_enabled() -> bool {
    std::env::var("THINGS3_LIVE_TESTS").as_deref() == Ok("1")
}

/// Connect to the user's real Things 3 SQLite database and wrap it in an
/// `AppleScriptBackend`. Read-only DB access is CulturedCode-safe; the
/// mutations route through osascript.
///
/// Honors `THINGS_DB_PATH` / `THINGS_DATABASE_PATH` so installs whose
/// `ThingsData-*` group container suffix differs from the project default
/// can still run this suite.
async fn live_backend() -> Arc<AppleScriptBackend> {
    let db_path = std::env::var("THINGS_DB_PATH")
        .or_else(|_| std::env::var("THINGS_DATABASE_PATH"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| things3_core::get_default_database_path());
    let db = Arc::new(
        ThingsDatabase::new(&db_path)
            .await
            .expect("failed to open Things 3 database"),
    );
    Arc::new(AppleScriptBackend::new(db))
}

/// Unique title suffix so concurrent or stale runs don't collide.
fn unique_suffix() -> String {
    format!(
        "{}-{}",
        chrono::Utc::now().timestamp(),
        ThingsId::new_v4().as_str()
    )
}

#[derive(Clone, Copy)]
enum Kind {
    Task,
    Project,
    Area,
    Tag,
}

/// RAII guard that deletes its entity from Things 3 on `Drop`. Block on a
/// fresh single-threaded runtime in a spawned thread so we don't try to
/// re-enter the test's existing tokio runtime (which would panic).
///
/// Call [`Guard::dismiss`] on the happy path after explicit deletion to
/// suppress the cleanup. Only a panic should let the `Drop` actually fire.
struct Guard {
    backend: Arc<AppleScriptBackend>,
    id: Option<ThingsId>,
    kind: Kind,
}

impl Guard {
    fn new(backend: Arc<AppleScriptBackend>, id: ThingsId, kind: Kind) -> Self {
        Self {
            backend,
            id: Some(id),
            kind,
        }
    }

    fn dismiss(&mut self) {
        self.id = None;
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let Some(id) = self.id.take() else { return };
        let backend = Arc::clone(&self.backend);
        let kind = self.kind;
        let _ = std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(_) => return,
            };
            rt.block_on(async move {
                let _ = match kind {
                    Kind::Task => backend.delete_task(&id, DeleteChildHandling::Error).await,
                    Kind::Project => {
                        backend
                            .delete_project(&id, ProjectChildHandling::Error)
                            .await
                    }
                    Kind::Area => backend.delete_area(&id).await,
                    Kind::Tag => backend.delete_tag(&id, false).await,
                };
            });
        })
        .join();
    }
}

/// Full `create_task` → `update_task` → `complete_task` → `delete_task`
/// round-trip. Reuses the same notes-with-escapes payload from the previous
/// in-module `task_lifecycle_round_trip`, which exercised the script-injection
/// guard.
#[tokio::test]
#[ignore = "requires Things 3 + Automation permission; set THINGS3_LIVE_TESTS=1"]
async fn task_lifecycle_round_trip() {
    if !live_tests_enabled() {
        return;
    }
    let backend = live_backend().await;
    let title = format!("rust-things3 e2e task {}", unique_suffix());

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
    assert!(!id.as_str().is_empty(), "task ThingsId should not be empty");
    let mut guard = Guard::new(Arc::clone(&backend), id.clone(), Kind::Task);
    println!("created task id: {id}");

    backend
        .update_task(UpdateTaskRequest {
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

    backend
        .complete_task(&id)
        .await
        .expect("complete_task should succeed");

    backend
        .delete_task(&id, DeleteChildHandling::Error)
        .await
        .expect("delete_task should succeed");
    guard.dismiss();
}

/// Full `create_project` → `update_project` → `complete_project` →
/// `delete_project` round-trip with no children, so the safest
/// `ProjectChildHandling::Error` mode is exercised.
#[tokio::test]
#[ignore = "requires Things 3 + Automation permission; set THINGS3_LIVE_TESTS=1"]
async fn project_lifecycle_round_trip() {
    if !live_tests_enabled() {
        return;
    }
    let backend = live_backend().await;
    let title = format!("rust-things3 e2e project {}", unique_suffix());

    let id = backend
        .create_project(CreateProjectRequest {
            title: title.clone(),
            notes: Some("e2e notes".into()),
            area_uuid: None,
            start_date: None,
            deadline: None,
            tags: None,
        })
        .await
        .expect("create_project should succeed");
    assert!(
        !id.as_str().is_empty(),
        "project ThingsId should not be empty"
    );
    let mut guard = Guard::new(Arc::clone(&backend), id.clone(), Kind::Project);
    println!("created project id: {id}");

    backend
        .update_project(UpdateProjectRequest {
            uuid: id.clone(),
            title: Some(format!("{title} (updated)")),
            notes: None,
            area_uuid: None,
            start_date: None,
            deadline: None,
            tags: None,
        })
        .await
        .expect("update_project should succeed");

    backend
        .complete_project(&id, ProjectChildHandling::Error)
        .await
        .expect("complete_project should succeed");

    backend
        .delete_project(&id, ProjectChildHandling::Error)
        .await
        .expect("delete_project should succeed");
    guard.dismiss();
}

/// Full `create_area` → `update_area` → `delete_area` round-trip.
/// Areas have no child-handling axis or completion state.
#[tokio::test]
#[ignore = "requires Things 3 + Automation permission; set THINGS3_LIVE_TESTS=1"]
async fn area_lifecycle_round_trip() {
    if !live_tests_enabled() {
        return;
    }
    let backend = live_backend().await;
    let title = format!("rust-things3 e2e area {}", unique_suffix());

    let id = backend
        .create_area(CreateAreaRequest {
            title: title.clone(),
        })
        .await
        .expect("create_area should succeed");
    assert!(!id.as_str().is_empty(), "area ThingsId should not be empty");
    let mut guard = Guard::new(Arc::clone(&backend), id.clone(), Kind::Area);
    println!("created area id: {id}");

    backend
        .update_area(UpdateAreaRequest {
            uuid: id.clone(),
            title: format!("{title} (updated)"),
        })
        .await
        .expect("update_area should succeed");

    backend
        .delete_area(&id)
        .await
        .expect("delete_area should succeed");
    guard.dismiss();
}

/// Full tag lifecycle: `create_tag(force=true)` → `update_tag` (rename) →
/// `add_tag_to_task` (against a throwaway task) → `remove_tag_from_task` →
/// `delete_tag(remove_from_tasks=false)`. Two guards compose so a panic
/// after either creation still cleans up.
#[tokio::test]
#[ignore = "requires Things 3 + Automation permission; set THINGS3_LIVE_TESTS=1"]
async fn tag_lifecycle_round_trip() {
    if !live_tests_enabled() {
        return;
    }
    let backend = live_backend().await;
    let suffix = unique_suffix();
    let tag_title = format!("rust-things3-e2e-tag-{suffix}");
    let task_title = format!("rust-things3 e2e tag-host {suffix}");

    let tag_id = match backend
        .create_tag(
            CreateTagRequest {
                title: tag_title.clone(),
                shortcut: None,
                parent_uuid: None,
            },
            true,
        )
        .await
        .expect("create_tag should succeed")
    {
        things3_core::TagCreationResult::Created { uuid, .. } => uuid,
        other => panic!("expected Created from forced create_tag, got {other:?}"),
    };
    let mut tag_guard = Guard::new(Arc::clone(&backend), tag_id.clone(), Kind::Tag);
    println!("created tag id: {tag_id}");

    let renamed = format!("{tag_title}-renamed");
    backend
        .update_tag(UpdateTagRequest {
            uuid: tag_id.clone(),
            title: Some(renamed.clone()),
            shortcut: None,
            parent_uuid: None,
        })
        .await
        .expect("update_tag should succeed");
    // `add_tag_to_task` immediately looks up the renamed title in the DB.
    // This assumes Things 3 flushes the rename to SQLite synchronously before
    // osascript returns. If the flush were async, `find_tag_by_normalized_title`
    // would return None, fall through to auto-create, and the `tag_uuid`
    // assertion below would fail. In practice Things 3 appears synchronous here.

    let task_id = backend
        .create_task(CreateTaskRequest {
            title: task_title,
            task_type: None,
            notes: None,
            start_date: None,
            deadline: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: None,
            status: None,
        })
        .await
        .expect("create_task (tag-host) should succeed");
    let mut task_guard = Guard::new(Arc::clone(&backend), task_id.clone(), Kind::Task);

    let assigned = backend
        .add_tag_to_task(&task_id, &renamed)
        .await
        .expect("add_tag_to_task should succeed");
    match assigned {
        things3_core::TagAssignmentResult::Assigned { tag_uuid } => {
            assert_eq!(
                tag_uuid, tag_id,
                "should resolve to the freshly-created tag"
            );
        }
        other => panic!("expected Assigned, got {other:?}"),
    }

    backend
        .remove_tag_from_task(&task_id, &renamed)
        .await
        .expect("remove_tag_from_task should succeed");

    backend
        .delete_task(&task_id, DeleteChildHandling::Error)
        .await
        .expect("delete_task (tag-host) should succeed");
    task_guard.dismiss();

    backend
        .delete_tag(&tag_id, false)
        .await
        .expect("delete_tag should succeed");
    tag_guard.dismiss();
}
