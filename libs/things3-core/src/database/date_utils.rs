//! Date validation and conversion utilities for Things 3
//!
//! This module provides safe date conversion between Things 3's internal format
//! (seconds since 2001-01-01) and standard date types, along with comprehensive
//! validation to ensure date consistency.

use chrono::{Datelike, NaiveDate, NaiveTime, TimeZone, Utc};
use thiserror::Error;

/// Things 3 epoch: 2001-01-01 00:00:00 UTC
const THINGS_EPOCH_YEAR: i32 = 2001;

/// Maximum reasonable year for Things 3 dates (year 2100)
const MAX_YEAR: i32 = 2100;

/// Minimum reasonable timestamp (2000-01-01, before Things 3 was created but allows some leeway)
const MIN_REASONABLE_TIMESTAMP: i64 = -31536000; // ~1 year before epoch

/// Maximum reasonable timestamp (2100-01-01)
const MAX_REASONABLE_TIMESTAMP: i64 = 3_124_224_000; // ~99 years after epoch

/// Errors that can occur during date conversion
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

/// Check if a Things 3 timestamp is within a reasonable range
///
/// Things 3 was released in 2009, so dates before 2000 are suspicious.
/// Dates after 2100 are likely errors or overflow.
///
/// # Arguments
/// * `seconds` - Seconds since 2001-01-01
///
/// # Returns
/// `true` if the timestamp is reasonable, `false` otherwise
pub fn is_valid_things_timestamp(seconds: i64) -> bool {
    (MIN_REASONABLE_TIMESTAMP..=MAX_REASONABLE_TIMESTAMP).contains(&seconds)
}

/// Convert Things 3 timestamp to NaiveDate with comprehensive error handling
///
/// Things 3 stores dates as seconds since 2001-01-01 00:00:00 UTC.
///
/// # Arguments
/// * `seconds_since_2001` - Seconds since the Things 3 epoch
///
/// # Returns
/// `Ok(NaiveDate)` if conversion succeeds, `Err` with detailed error otherwise
///
/// # Errors
/// Returns error if:
/// - Timestamp is invalid or would cause overflow
/// - Resulting date is before 2000 or after 2100
pub fn safe_things_date_to_naive_date(
    seconds_since_2001: i64,
) -> Result<NaiveDate, DateConversionError> {
    // Check for reasonable range
    if !is_valid_things_timestamp(seconds_since_2001) {
        return Err(DateConversionError::InvalidTimestamp(seconds_since_2001));
    }

    // Base date: 2001-01-01 00:00:00 UTC
    let base_date = Utc
        .with_ymd_and_hms(THINGS_EPOCH_YEAR, 1, 1, 0, 0, 0)
        .single()
        .ok_or(DateConversionError::Overflow)?;

    // Add seconds to get the actual date
    let date_time = base_date
        .checked_add_signed(chrono::Duration::seconds(seconds_since_2001))
        .ok_or(DateConversionError::Overflow)?;

    let naive_date = date_time.date_naive();

    // Verify the result is reasonable
    if naive_date.year() > MAX_YEAR {
        return Err(DateConversionError::TooFarFuture(naive_date));
    }

    Ok(naive_date)
}

/// Convert NaiveDate to Things 3 timestamp with validation
///
/// # Arguments
/// * `date` - The date to convert
///
/// # Returns
/// `Ok(i64)` timestamp if conversion succeeds, `Err` with detailed error otherwise
///
/// # Errors
/// Returns error if:
/// - Date is before the Things 3 epoch (2001-01-01)
/// - Date is too far in the future (after 2100)
/// - Calculation would overflow
pub fn safe_naive_date_to_things_timestamp(date: NaiveDate) -> Result<i64, DateConversionError> {
    // Check if date is before epoch
    let epoch_date =
        NaiveDate::from_ymd_opt(THINGS_EPOCH_YEAR, 1, 1).ok_or(DateConversionError::Overflow)?;

    if date < epoch_date {
        return Err(DateConversionError::BeforeEpoch(date));
    }

    // Check if date is too far in the future
    if date.year() > MAX_YEAR {
        return Err(DateConversionError::TooFarFuture(date));
    }

    // Base date: 2001-01-01 00:00:00 UTC
    let base_date = Utc
        .with_ymd_and_hms(THINGS_EPOCH_YEAR, 1, 1, 0, 0, 0)
        .single()
        .ok_or(DateConversionError::Overflow)?;

    // Convert NaiveDate to DateTime at midnight UTC
    let date_time = date
        .and_time(NaiveTime::from_hms_opt(0, 0, 0).ok_or(DateConversionError::Overflow)?)
        .and_local_timezone(Utc)
        .single()
        .ok_or(DateConversionError::Overflow)?;

    // Calculate seconds difference
    let seconds = date_time.signed_duration_since(base_date).num_seconds();

    Ok(seconds)
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
/// Supports:
/// - ISO 8601: "YYYY-MM-DD"
///
/// # Arguments
/// * `s` - String to parse
///
/// # Returns
/// `Ok(NaiveDate)` if parsing succeeds, `Err` otherwise
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

    #[test]
    fn test_is_valid_things_timestamp() {
        // Valid timestamps
        assert!(is_valid_things_timestamp(0)); // Epoch
        assert!(is_valid_things_timestamp(86400)); // 1 day after
        assert!(is_valid_things_timestamp(31536000)); // 1 year after

        // Invalid - too far in past
        assert!(!is_valid_things_timestamp(-100000000));

        // Invalid - too far in future (beyond 2100)
        assert!(!is_valid_things_timestamp(4000000000));
    }

    #[test]
    fn test_safe_things_date_conversion_epoch() {
        // Epoch should convert to 2001-01-01
        let date = safe_things_date_to_naive_date(0).unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2001, 1, 1).unwrap());
    }

    #[test]
    fn test_safe_things_date_conversion_normal() {
        // 1 day after epoch
        let date = safe_things_date_to_naive_date(86400).unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2001, 1, 2).unwrap());
    }

    #[test]
    fn test_safe_things_date_conversion_invalid() {
        // Way too far in future
        assert!(safe_things_date_to_naive_date(10000000000).is_err());

        // Way too far in past
        assert!(safe_things_date_to_naive_date(-100000000).is_err());
    }

    #[test]
    fn test_safe_naive_date_to_things_timestamp_epoch() {
        let date = NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
        let timestamp = safe_naive_date_to_things_timestamp(date).unwrap();
        assert_eq!(timestamp, 0);
    }

    #[test]
    fn test_safe_naive_date_to_things_timestamp_normal() {
        let date = NaiveDate::from_ymd_opt(2001, 1, 2).unwrap();
        let timestamp = safe_naive_date_to_things_timestamp(date).unwrap();
        assert_eq!(timestamp, 86400);
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
