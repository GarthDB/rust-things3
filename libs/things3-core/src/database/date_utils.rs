//! Date validation and conversion utilities for Things 3
//!
//! This module provides safe date conversion between Things 3's internal format
//! (packed `year<<16 | month<<12 | day<<7` ints) and standard date types, along with comprehensive
//! validation to ensure date consistency.

use chrono::{Datelike, NaiveDate, Utc};

use super::conversions::{naive_date_to_things_timestamp, things_date_to_naive_date};
use thiserror::Error;

/// Things 3 epoch: 2001-01-01 00:00:00 UTC
const THINGS_EPOCH_YEAR: i32 = 2001;

/// Maximum reasonable year for Things 3 dates (year 2100)
const MAX_YEAR: i32 = 2100;

/// Errors that can occur during date conversion
#[non_exhaustive]
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DateConversionError {
    /// Date is before the Things 3 epoch (2001-01-01)
    #[error("Date is before Things 3 epoch (2001-01-01): {0}")]
    BeforeEpoch(NaiveDate),

    /// Date timestamp is invalid or would cause overflow
    #[error("Date timestamp {0} is invalid or would cause overflow")]
    InvalidTimestamp(i64),

    /// Date is too far in the future (after 2100)
    #[error("Date is too far in the future (after year {MAX_YEAR}): {0}")]
    TooFarFuture(NaiveDate),

    /// Date conversion resulted in overflow
    #[error("Date conversion overflow during calculation")]
    Overflow,

    /// Date string parsing failed
    #[error("Failed to parse date string '{string}': {reason}")]
    ParseError { string: String, reason: String },
}

/// Errors that can occur during date validation
#[non_exhaustive]
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DateValidationError {
    /// Deadline cannot be before start date
    #[error("Deadline {deadline} cannot be before start date {start_date}")]
    DeadlineBeforeStartDate {
        start_date: NaiveDate,
        deadline: NaiveDate,
    },

    /// Date conversion failed
    #[error("Date conversion failed: {0}")]
    ConversionFailed(#[from] DateConversionError),
}

/// Check if a packed Things 3 date decodes to a reasonable date (2000..=2100)
pub fn is_valid_things_timestamp(packed: i64) -> bool {
    things_date_to_naive_date(packed).is_some_and(|d| (2000..=MAX_YEAR).contains(&d.year()))
}

/// Convert a packed Things 3 date (`year<<16 | month<<12 | day<<7`) to a NaiveDate
///
/// # Errors
/// Returns error if the value is not a valid packed date or the year is after 2100
pub fn safe_things_date_to_naive_date(packed: i64) -> Result<NaiveDate, DateConversionError> {
    let date =
        things_date_to_naive_date(packed).ok_or(DateConversionError::InvalidTimestamp(packed))?;
    if date.year() > MAX_YEAR {
        return Err(DateConversionError::TooFarFuture(date));
    }
    Ok(date)
}

/// Convert NaiveDate to a packed Things 3 date with validation
///
/// # Errors
/// Returns error if the date is before 2001-01-01 or after 2100
pub fn safe_naive_date_to_things_timestamp(date: NaiveDate) -> Result<i64, DateConversionError> {
    let epoch_date =
        NaiveDate::from_ymd_opt(THINGS_EPOCH_YEAR, 1, 1).ok_or(DateConversionError::Overflow)?;
    if date < epoch_date {
        return Err(DateConversionError::BeforeEpoch(date));
    }
    if date.year() > MAX_YEAR {
        return Err(DateConversionError::TooFarFuture(date));
    }
    Ok(naive_date_to_things_timestamp(date))
}

/// Validate that a deadline is not before a start date
///
/// # Arguments
/// * `start_date` - Optional start date
/// * `deadline` - Optional deadline
///
/// # Returns
/// `Ok(())` if dates are valid or None, `Err` if deadline is before start date
pub fn validate_date_range(
    start_date: Option<NaiveDate>,
    deadline: Option<NaiveDate>,
) -> Result<(), DateValidationError> {
    if let (Some(start), Some(end)) = (start_date, deadline) {
        if end < start {
            return Err(DateValidationError::DeadlineBeforeStartDate {
                start_date: start,
                deadline: end,
            });
        }
    }
    Ok(())
}

/// Format a date for display, handling None gracefully
///
/// # Arguments
/// * `date` - Optional date to format
///
/// # Returns
/// ISO 8601 formatted date string, or "None" if date is None
pub fn format_date_for_display(date: Option<NaiveDate>) -> String {
    match date {
        Some(d) => d.format("%Y-%m-%d").to_string(),
        None => "None".to_string(),
    }
}

/// Parse a date from a string, supporting multiple formats
///
/// Currently supports:
/// - ISO 8601: "YYYY-MM-DD" (e.g., "2024-12-31")
///
/// Future formats to consider:
/// - US format: "MM/DD/YYYY"
/// - European format: "DD/MM/YYYY"
/// - Natural language: "today", "tomorrow", "next week"
///
/// # Arguments
/// * `s` - String to parse
///
/// # Returns
/// `Ok(NaiveDate)` if parsing succeeds, `Err(DateConversionError)` otherwise
///
/// # Examples
/// ```
/// use things3_core::database::parse_date_from_string;
/// use chrono::NaiveDate;
///
/// let date = parse_date_from_string("2024-12-31").unwrap();
/// assert_eq!(date, NaiveDate::from_ymd_opt(2024, 12, 31).unwrap());
/// ```
pub fn parse_date_from_string(s: &str) -> Result<NaiveDate, DateConversionError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| DateConversionError::ParseError {
        string: s.to_string(),
        reason: e.to_string(),
    })
}

/// Check if a date is in the past
///
/// # Arguments
/// * `date` - Date to check
///
/// # Returns
/// `true` if the date is before today (UTC), `false` otherwise
pub fn is_date_in_past(date: NaiveDate) -> bool {
    date < Utc::now().date_naive()
}

/// Check if a date is in the future
///
/// # Arguments
/// * `date` - Date to check
///
/// # Returns
/// `true` if the date is after today (UTC), `false` otherwise
pub fn is_date_in_future(date: NaiveDate) -> bool {
    date > Utc::now().date_naive()
}

/// Add days to a date with overflow checking
///
/// # Arguments
/// * `date` - Starting date
/// * `days` - Number of days to add (can be negative)
///
/// # Returns
/// `Ok(NaiveDate)` if successful, `Err` if overflow would occur
pub fn add_days(date: NaiveDate, days: i64) -> Result<NaiveDate, DateConversionError> {
    date.checked_add_signed(chrono::Duration::days(days))
        .ok_or(DateConversionError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packed(y: i32, m: u32, d: u32) -> i64 {
        naive_date_to_things_timestamp(NaiveDate::from_ymd_opt(y, m, d).unwrap())
    }

    #[test]
    fn test_is_valid_things_timestamp() {
        assert!(is_valid_things_timestamp(packed(2026, 11, 10)));
        assert!(!is_valid_things_timestamp(0));
        assert!(!is_valid_things_timestamp(86400)); // seconds-style value
        assert!(!is_valid_things_timestamp(packed(2150, 1, 1)));
    }

    #[test]
    fn test_safe_things_date_conversion_known() {
        assert_eq!(
            safe_things_date_to_naive_date(132_822_272).unwrap(),
            NaiveDate::from_ymd_opt(2026, 11, 10).unwrap()
        );
    }

    #[test]
    fn test_safe_things_date_conversion_invalid() {
        assert!(safe_things_date_to_naive_date(0).is_err());
        assert!(safe_things_date_to_naive_date(-100_000_000).is_err());
        assert!(matches!(
            safe_things_date_to_naive_date(packed(2150, 1, 1)),
            Err(DateConversionError::TooFarFuture(_))
        ));
    }

    #[test]
    fn test_safe_naive_date_to_things_timestamp_known() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
        assert_eq!(
            safe_naive_date_to_things_timestamp(date).unwrap(),
            132_822_272
        );
    }

    #[test]
    fn test_safe_naive_date_to_things_timestamp_before_epoch() {
        let date = NaiveDate::from_ymd_opt(2000, 12, 31).unwrap();
        let result = safe_naive_date_to_things_timestamp(date);
        assert!(matches!(result, Err(DateConversionError::BeforeEpoch(_))));
    }

    #[test]
    fn test_safe_naive_date_to_things_timestamp_too_far_future() {
        let date = NaiveDate::from_ymd_opt(2150, 1, 1).unwrap();
        let result = safe_naive_date_to_things_timestamp(date);
        assert!(matches!(result, Err(DateConversionError::TooFarFuture(_))));
    }

    #[test]
    fn test_round_trip_conversion() {
        let original_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let timestamp = safe_naive_date_to_things_timestamp(original_date).unwrap();
        let converted_date = safe_things_date_to_naive_date(timestamp).unwrap();
        assert_eq!(original_date, converted_date);
    }

    #[test]
    fn test_validate_date_range_valid() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        assert!(validate_date_range(Some(start), Some(end)).is_ok());
    }

    #[test]
    fn test_validate_date_range_invalid() {
        let start = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let result = validate_date_range(Some(start), Some(end));
        assert!(matches!(
            result,
            Err(DateValidationError::DeadlineBeforeStartDate { .. })
        ));
    }

    #[test]
    fn test_validate_date_range_same_date() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        assert!(validate_date_range(Some(date), Some(date)).is_ok());
    }

    #[test]
    fn test_validate_date_range_only_start() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert!(validate_date_range(Some(start), None).is_ok());
    }

    #[test]
    fn test_validate_date_range_only_end() {
        let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        assert!(validate_date_range(None, Some(end)).is_ok());
    }

    #[test]
    fn test_validate_date_range_both_none() {
        assert!(validate_date_range(None, None).is_ok());
    }

    #[test]
    fn test_format_date_for_display() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        assert_eq!(format_date_for_display(Some(date)), "2024-06-15");
        assert_eq!(format_date_for_display(None), "None");
    }

    #[test]
    fn test_parse_date_from_string_valid() {
        let date = parse_date_from_string("2024-06-15").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2024, 6, 15).unwrap());
    }

    #[test]
    fn test_parse_date_from_string_invalid() {
        assert!(parse_date_from_string("invalid").is_err());
        assert!(parse_date_from_string("2024-13-01").is_err());
        assert!(parse_date_from_string("2024-06-32").is_err());
    }

    #[test]
    fn test_add_days_positive() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let new_date = add_days(date, 10).unwrap();
        assert_eq!(new_date, NaiveDate::from_ymd_opt(2024, 1, 11).unwrap());
    }

    #[test]
    fn test_add_days_negative() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let new_date = add_days(date, -10).unwrap();
        assert_eq!(new_date, NaiveDate::from_ymd_opt(2024, 1, 5).unwrap());
    }
}
