//! Row mapping utilities for converting database rows to domain models.
//!
//! `uuid` columns in the Things 3 SQLite database hold strings the database
//! itself produced — either Things-native 21–22-char base62 IDs or hyphenated
//! UUIDs that `SqlxBackend` generated for new entities. Both are valid
//! [`ThingsId`] values; we wrap them via [`ThingsId::from_trusted`] without
//! re-validating, since the DB is the source of truth.

use crate::{
    database::{safe_timestamp_convert, things_date_to_naive_date},
    error::Result as ThingsResult,
    models::{Project, Task, TaskStatus, TaskType, ThingsId},
};
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

/// Wrap a `uuid`-column string from the database as a [`ThingsId`].
///
/// No validation happens; the DB is authoritative.
fn id_from_row(s: String) -> ThingsId {
    ThingsId::from_trusted(s)
}

/// Wrap an optional `uuid`-column string as `Option<ThingsId>`.
fn optional_id_from_row(opt: Option<String>) -> Option<ThingsId> {
    opt.map(ThingsId::from_trusted)
}

/// Map a database row to a Task struct
///
/// This function centralizes all the logic for constructing a Task from
/// a SQLite row, including UUID parsing, date conversion, and field mapping.
///
/// # Errors
///
/// Returns an error if required fields are missing or cannot be converted
pub fn map_task_row(row: &SqliteRow) -> ThingsResult<Task> {
    let uuid = id_from_row(row.get("uuid"));

    let title: String = row.get("title");

    let status_i32: i32 = row.get("status");
    let status = match status_i32 {
        1 => TaskStatus::Completed,
        2 => TaskStatus::Canceled,
        3 => TaskStatus::Trashed,
        _ => TaskStatus::Incomplete,
    };

    let type_i32: i32 = row.get("type");
    let task_type = match type_i32 {
        1 => TaskType::Project,
        2 => TaskType::Heading,
        _ => TaskType::Todo,
    };

    let notes: Option<String> = row.get("notes");

    let start_date = row
        .get::<Option<i64>, _>("startDate")
        .and_then(things_date_to_naive_date);

    let deadline = row
        .get::<Option<i64>, _>("deadline")
        .and_then(things_date_to_naive_date);

    let creation_ts: f64 = row.get("creationDate");
    let created = {
        let ts = safe_timestamp_convert(creation_ts);
        DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
    };

    let modification_ts: f64 = row.get("userModificationDate");
    let modified = {
        let ts = safe_timestamp_convert(modification_ts);
        DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
    };

    let stop_date = row.get::<Option<f64>, _>("stopDate").and_then(|ts| {
        let ts_i64 = safe_timestamp_convert(ts);
        DateTime::from_timestamp(ts_i64, 0)
    });

    let project_uuid = optional_id_from_row(row.get::<Option<String>, _>("project"));
    let area_uuid = optional_id_from_row(row.get::<Option<String>, _>("area"));
    let parent_uuid = optional_id_from_row(row.get::<Option<String>, _>("heading"));

    // Try to get cachedTags as binary data and parse it
    let tags = row
        .get::<Option<Vec<u8>>, _>("cachedTags")
        .and_then(|blob| {
            // Parse the JSON blob into a Vec<String>
            crate::database::deserialize_tags_from_blob(&blob).ok()
        })
        .unwrap_or_default();

    Ok(Task {
        uuid,
        title,
        status,
        task_type,
        notes,
        start_date,
        deadline,
        created,
        modified,
        stop_date,
        project_uuid,
        area_uuid,
        parent_uuid,
        tags,
        children: Vec::new(),
    })
}

/// Map a `TMTask` row (where `type = 1`) into a [`Project`].
pub fn map_project_row(row: &SqliteRow) -> Project {
    Project {
        uuid: id_from_row(row.get("uuid")),
        title: row.get("title"),
        status: match row.get::<i32, _>("status") {
            1 => TaskStatus::Completed,
            2 => TaskStatus::Canceled,
            3 => TaskStatus::Trashed,
            _ => TaskStatus::Incomplete,
        },
        area_uuid: optional_id_from_row(row.get::<Option<String>, _>("area")),
        notes: row.get("notes"),
        deadline: row
            .get::<Option<i64>, _>("deadline")
            .and_then(|ts| DateTime::from_timestamp(ts, 0))
            .map(|dt| dt.date_naive()),
        start_date: row
            .get::<Option<i64>, _>("startDate")
            .and_then(|ts| DateTime::from_timestamp(ts, 0))
            .map(|dt| dt.date_naive()),
        tags: Vec::new(),
        tasks: Vec::new(),
        created: {
            let ts = safe_timestamp_convert(row.get::<f64, _>("creationDate"));
            DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
        },
        modified: {
            let ts = safe_timestamp_convert(row.get::<f64, _>("userModificationDate"));
            DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_from_row_preserves_native_things_id() {
        let id = id_from_row("R4t2G8Q63aGZq4epMHNeCr".to_string());
        assert_eq!(id.as_str(), "R4t2G8Q63aGZq4epMHNeCr");
    }

    #[test]
    fn id_from_row_preserves_hyphenated_uuid() {
        let id = id_from_row("550e8400-e29b-41d4-a716-446655440000".to_string());
        assert_eq!(id.as_str(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn optional_id_from_row_passes_through_none() {
        assert!(optional_id_from_row(None).is_none());
    }

    #[test]
    fn optional_id_from_row_wraps_some() {
        let opt = optional_id_from_row(Some("ABC123XYZ456789012345".to_string()));
        assert_eq!(opt.unwrap().as_str(), "ABC123XYZ456789012345");
    }
}
