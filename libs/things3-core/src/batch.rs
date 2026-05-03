//! Batch fetch-by-id primitives on [`ThingsDatabase`].
//!
//! Two methods extend [`ThingsDatabase`] when the `batch-operations` feature
//! is enabled:
//! - [`ThingsDatabase::get_tasks_batch`] — many tasks by UUID
//! - [`ThingsDatabase::get_projects_batch`] — many projects by UUID
//!
//! Both mirror the filtering semantics of their single-fetch siblings
//! ([`ThingsDatabase::get_task_by_uuid`] and
//! [`ThingsDatabase::get_project_by_uuid`]): trashed rows are omitted, no
//! type filter is applied beyond what `get_project_by_uuid` already does
//! (`type = 1`). Duplicate UUIDs in the input are de-duplicated; empty
//! input returns `Ok(vec![])` without any SQL roundtrip; results are
//! ordered by `(creationDate DESC, uuid DESC)`.
//!
//! Internally the helpers chunk at 500 UUIDs per query — comfortably below
//! SQLite's `SQLITE_LIMIT_VARIABLE_NUMBER` floor (999) and far below the
//! modern bundled limit (32766).

#![cfg(feature = "batch-operations")]

use std::collections::HashSet;

use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::database::mappers::{map_project_row, map_task_row};
use crate::database::ThingsDatabase;
use crate::error::{Result as ThingsResult, ThingsError};
use crate::models::{Project, Task, ThingsId};

/// Conservative chunk size — keeps each round-trip well below SQLite's
/// `SQLITE_LIMIT_VARIABLE_NUMBER` floor (999) so callers can pass arbitrarily
/// long UUID lists without surfacing parameter-limit failures.
const BATCH_CHUNK_SIZE: usize = 500;

impl ThingsDatabase {
    /// Fetch many tasks by UUID in a single batched query.
    ///
    /// Mirrors [`ThingsDatabase::get_task_by_uuid`]: trashed rows are omitted
    /// and there is no task-type filter (a project or heading UUID will
    /// resolve to a [`Task`] mapped from its TMTask row, matching
    /// single-fetch loose semantics). Duplicate UUIDs are de-duplicated.
    /// Empty input returns `Ok(vec![])` without any SQL call. Results are
    /// ordered by `(creationDate DESC, uuid DESC)` for determinism.
    ///
    /// Requires the `batch-operations` feature flag.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying database query fails.
    pub async fn get_tasks_batch(&self, uuids: &[ThingsId]) -> ThingsResult<Vec<Task>> {
        let mut tasks = fetch_in_chunks(
            &self.pool,
            uuids,
            "SELECT uuid, title, type, status, notes, startDate, deadline, stopDate, \
             creationDate, userModificationDate, project, area, heading, cachedTags, trashed \
             FROM TMTask WHERE uuid IN ({placeholders})",
            |row| {
                let trashed: i64 = row.get("trashed");
                if trashed == 1 {
                    return Ok(None);
                }
                map_task_row(row).map(Some)
            },
        )
        .await?;

        tasks.sort_by(|a, b| b.created.cmp(&a.created).then_with(|| b.uuid.cmp(&a.uuid)));
        Ok(tasks)
    }

    /// Fetch many projects by UUID in a single batched query.
    ///
    /// Mirrors [`ThingsDatabase::get_project_by_uuid`]: only `type = 1` rows
    /// are returned, trashed rows are omitted. Duplicate UUIDs are
    /// de-duplicated. Empty input returns `Ok(vec![])` without any SQL
    /// call. Results are ordered by `(creationDate DESC, uuid DESC)`.
    ///
    /// Requires the `batch-operations` feature flag.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying database query fails.
    pub async fn get_projects_batch(&self, uuids: &[ThingsId]) -> ThingsResult<Vec<Project>> {
        let mut projects = fetch_in_chunks(
            &self.pool,
            uuids,
            "SELECT uuid, title, status, area, notes, creationDate, userModificationDate, \
             startDate, deadline, trashed, type \
             FROM TMTask WHERE type = 1 AND uuid IN ({placeholders})",
            |row| {
                let trashed: i64 = row.get("trashed");
                if trashed == 1 {
                    return Ok(None);
                }
                Ok(Some(map_project_row(row)))
            },
        )
        .await?;

        projects.sort_by(|a, b| b.created.cmp(&a.created).then_with(|| b.uuid.cmp(&a.uuid)));
        Ok(projects)
    }
}

/// Generic batch fetcher: de-dups input, chunks at [`BATCH_CHUNK_SIZE`],
/// substitutes `{placeholders}` with `?,?,...?`, runs each chunk, and
/// flattens the results.
///
/// `map_row` returns `Ok(None)` for rows that should be filtered out (e.g.
/// trashed) — their `Some(T)` siblings are kept.
async fn fetch_in_chunks<T, F>(
    pool: &SqlitePool,
    uuids: &[ThingsId],
    sql_template: &str,
    map_row: F,
) -> ThingsResult<Vec<T>>
where
    F: Fn(&SqliteRow) -> ThingsResult<Option<T>>,
{
    if uuids.is_empty() {
        return Ok(Vec::new());
    }

    let mut seen = HashSet::with_capacity(uuids.len());
    let unique: Vec<&ThingsId> = uuids.iter().filter(|u| seen.insert(u.as_str())).collect();

    let mut out = Vec::with_capacity(unique.len());
    for chunk in unique.chunks(BATCH_CHUNK_SIZE) {
        let placeholders = vec!["?"; chunk.len()].join(",");
        let sql = sql_template.replace("{placeholders}", &placeholders);

        let mut q = sqlx::query(&sql);
        for u in chunk {
            q = q.bind(u.as_str());
        }
        let rows = q
            .fetch_all(pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Batch fetch failed: {e}")))?;

        for row in &rows {
            if let Some(item) = map_row(row)? {
                out.push(item);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    async fn open_test_db() -> (ThingsDatabase, NamedTempFile) {
        let f = NamedTempFile::new().unwrap();
        crate::test_utils::create_test_database(f.path())
            .await
            .unwrap();
        let db = ThingsDatabase::new(f.path()).await.unwrap();
        (db, f)
    }

    async fn insert_task(db: &ThingsDatabase, title: &str) -> ThingsId {
        let raw = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO TMTask \
             (uuid, title, type, status, trashed, creationDate, userModificationDate) \
             VALUES (?, ?, 0, 0, 0, 0, 0)",
        )
        .bind(raw.to_string())
        .bind(title)
        .execute(&db.pool)
        .await
        .unwrap();
        ThingsId::from_trusted(raw.to_string())
    }

    async fn insert_project(db: &ThingsDatabase, title: &str) -> ThingsId {
        let raw = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO TMTask \
             (uuid, title, type, status, trashed, creationDate, userModificationDate) \
             VALUES (?, ?, 1, 0, 0, 0, 0)",
        )
        .bind(raw.to_string())
        .bind(title)
        .execute(&db.pool)
        .await
        .unwrap();
        ThingsId::from_trusted(raw.to_string())
    }

    async fn mark_trashed(db: &ThingsDatabase, id: &ThingsId) {
        sqlx::query("UPDATE TMTask SET trashed = 1 WHERE uuid = ?")
            .bind(id.as_str())
            .execute(&db.pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_get_tasks_batch_empty_input_no_query() {
        let (db, _f) = open_test_db().await;
        let result = db.get_tasks_batch(&[]).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_tasks_batch_returns_existing() {
        let (db, _f) = open_test_db().await;
        let a = insert_task(&db, "alpha").await;
        let b = insert_task(&db, "beta").await;
        let c = insert_task(&db, "gamma").await;

        let result = db
            .get_tasks_batch(&[a.clone(), b.clone(), c.clone()])
            .await
            .unwrap();
        let uuids: HashSet<_> = result.iter().map(|t| t.uuid.clone()).collect();
        assert_eq!(uuids.len(), 3);
        assert!(uuids.contains(&a) && uuids.contains(&b) && uuids.contains(&c));
    }

    #[tokio::test]
    async fn test_get_tasks_batch_filters_unknown_uuids() {
        let (db, _f) = open_test_db().await;
        let real = insert_task(&db, "real").await;
        let phantom1 = ThingsId::new_v4();
        let phantom2 = ThingsId::new_v4();

        let result = db
            .get_tasks_batch(&[real.clone(), phantom1, phantom2])
            .await
            .unwrap();
        let uuids: HashSet<_> = result.iter().map(|t| t.uuid.clone()).collect();
        assert_eq!(uuids.len(), 1);
        assert!(uuids.contains(&real));
    }

    #[tokio::test]
    async fn test_get_tasks_batch_excludes_trashed() {
        let (db, _f) = open_test_db().await;
        let kept = insert_task(&db, "kept").await;
        let trashed_id = insert_task(&db, "trashed").await;
        mark_trashed(&db, &trashed_id).await;

        let result = db
            .get_tasks_batch(&[kept.clone(), trashed_id.clone()])
            .await
            .unwrap();
        let uuids: HashSet<_> = result.iter().map(|t| t.uuid.clone()).collect();
        assert_eq!(uuids.len(), 1);
        assert!(uuids.contains(&kept));
        assert!(!uuids.contains(&trashed_id));
    }

    #[tokio::test]
    async fn test_get_tasks_batch_dedups_duplicate_input() {
        let (db, _f) = open_test_db().await;
        let a = insert_task(&db, "alpha").await;
        let b = insert_task(&db, "beta").await;

        let result = db
            .get_tasks_batch(&[a.clone(), a.clone(), b.clone(), a.clone()])
            .await
            .unwrap();
        assert_eq!(result.len(), 2, "duplicate inputs must collapse to one row");
        let uuids: HashSet<_> = result.iter().map(|t| t.uuid.clone()).collect();
        assert!(uuids.contains(&a) && uuids.contains(&b));
    }

    #[tokio::test]
    async fn test_get_tasks_batch_ordering_is_deterministic() {
        let (db, _f) = open_test_db().await;
        // insert_task hardcodes creationDate = 0, so all tasks tie. ORDER BY
        // uuid DESC (applied in Rust after fetch) is a deterministic tiebreak.
        let mut inserted = Vec::new();
        for i in 0..3 {
            inserted.push(insert_task(&db, &format!("task-{i}")).await);
        }

        let first = db.get_tasks_batch(&inserted).await.unwrap();
        let second = db.get_tasks_batch(&inserted).await.unwrap();
        let first_uuids: Vec<_> = first.iter().map(|t| t.uuid.clone()).collect();
        let second_uuids: Vec<_> = second.iter().map(|t| t.uuid.clone()).collect();
        assert_eq!(first_uuids, second_uuids);
    }

    #[tokio::test]
    async fn test_get_tasks_batch_chunks_large_input() {
        let (db, _f) = open_test_db().await;
        let real_a = insert_task(&db, "real-a").await;
        let real_b = insert_task(&db, "real-b").await;

        // 600 UUIDs forces chunking past BATCH_CHUNK_SIZE (500). Most are
        // phantom; only the two real ones should come back.
        let mut all = Vec::with_capacity(600);
        all.push(real_a.clone());
        for _ in 0..598 {
            all.push(ThingsId::new_v4());
        }
        all.push(real_b.clone());

        let result = db.get_tasks_batch(&all).await.unwrap();
        let uuids: HashSet<_> = result.iter().map(|t| t.uuid.clone()).collect();
        assert_eq!(uuids.len(), 2);
        assert!(uuids.contains(&real_a) && uuids.contains(&real_b));
    }

    #[tokio::test]
    async fn test_get_projects_batch_returns_existing() {
        let (db, _f) = open_test_db().await;
        let p1 = insert_project(&db, "project-1").await;
        let p2 = insert_project(&db, "project-2").await;
        // A regular task should NOT be returned by get_projects_batch even if
        // its UUID is in the input — type filter excludes it.
        let task = insert_task(&db, "not-a-project").await;

        let result = db
            .get_projects_batch(&[p1.clone(), p2.clone(), task.clone()])
            .await
            .unwrap();
        let uuids: HashSet<_> = result.iter().map(|p| p.uuid.clone()).collect();
        assert_eq!(uuids.len(), 2);
        assert!(uuids.contains(&p1) && uuids.contains(&p2));
        assert!(!uuids.contains(&task));
    }
}
