//! Utility functions for Things 3 integration

use chrono::{DateTime, NaiveDate, Utc};
use std::path::PathBuf;

/// Get the default Things 3 database path
#[must_use]
pub fn get_default_database_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
    PathBuf::from(format!(
        "{home}/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite"
    ))
}

/// Format a date for display
#[must_use]
pub fn format_date(date: &NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Format a datetime for display
#[must_use]
pub fn format_datetime(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Parse a date string in YYYY-MM-DD format
///
/// # Errors
/// Returns `chrono::ParseError` if the date string is not in the expected format
pub fn parse_date(date_str: &str) -> Result<NaiveDate, chrono::ParseError> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
}

/// Validate a UUID string
#[must_use]
pub fn is_valid_uuid(uuid_str: &str) -> bool {
    uuid::Uuid::parse_str(uuid_str).is_ok()
}

/// Truncate a string to a maximum length
#[must_use]
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
