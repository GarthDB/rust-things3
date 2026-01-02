//! Row mapping utilities for converting database rows to domain models
//!
//! This module provides reusable mapping functions to eliminate duplication
//! in Task construction from SQL query results.

use crate::{
    database::{safe_timestamp_convert, things_date_to_naive_date, things_uuid_to_uuid},
    error::Result as ThingsResult,
    models::{Task, TaskStatus, TaskType},
};
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use uuid::Uuid;

/// Parse a UUID string with fallback to Things UUID conversion
///
/// First attempts to parse as a standard UUID format, then falls back
/// to the Things 3 UUID conversion if that fails.
pub fn parse_uuid_with_fallback(uuid_str: &str) -> Uuid {
    Uuid::parse_str(uuid_str).unwrap_or_else(|_| things_uuid_to_uuid(uuid_str))
}

/// Parse an optional UUID string with fallback
///
/// Handles Option<String> from database columns, returning None if the
/// input is None, otherwise using the fallback UUID parsing logic.
pub fn parse_optional_uuid(opt_str: Option<String>) -> Option<Uuid> {
    opt_str.map(|s| {
        Uuid::parse_str(&s)
            .ok()
            .unwrap_or_else(|| things_uuid_to_uuid(&s))
    })
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
    let uuid_str: String = row.get("uuid");
    let uuid = parse_uuid_with_fallback(&uuid_str);

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

    let project_uuid = row
        .get::<Option<String>, _>("project")
        .map(|s| parse_uuid_with_fallback(&s));

    let area_uuid = row
        .get::<Option<String>, _>("area")
        .map(|s| parse_uuid_with_fallback(&s));

    let parent_uuid = row
        .get::<Option<String>, _>("heading")
        .map(|s| parse_uuid_with_fallback(&s));

    // Try to get cachedTags as binary data
    let tags = row
        .get::<Option<Vec<u8>>, _>("cachedTags")
        .map(|_| Vec::new()) // TODO: Parse binary tag data
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uuid_with_fallback_standard() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let uuid = parse_uuid_with_fallback(uuid_str);
        assert_eq!(uuid.to_string(), uuid_str);
    }

    #[test]
    fn test_parse_uuid_with_fallback_things_format() {
        // Things 3 uses a different format - should fall back to things_uuid_to_uuid
        let things_id = "ABC123XYZ";
        let uuid1 = parse_uuid_with_fallback(things_id);
        let uuid2 = parse_uuid_with_fallback(things_id);
        // Should be consistent
        assert_eq!(uuid1, uuid2);
    }

    #[test]
    fn test_parse_optional_uuid_some() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_optional_uuid(Some(uuid_str.to_string()));
        assert!(result.is_some());
        assert_eq!(result.unwrap().to_string(), uuid_str);
    }

    #[test]
    fn test_parse_optional_uuid_none() {
        let result = parse_optional_uuid(None);
        assert!(result.is_none());
    }
}
