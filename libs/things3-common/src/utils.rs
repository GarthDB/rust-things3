//! Utility functions for Things 3 integration

use chrono::{DateTime, NaiveDate, Utc};

/// Format a date for display
///
/// # Examples
///
/// ```
/// use things3_common::format_date;
/// use chrono::NaiveDate;
///
/// let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
/// assert_eq!(format_date(&date), "2024-01-15");
/// ```
#[must_use]
pub fn format_date(date: &NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Format a datetime for display
///
/// # Examples
///
/// ```
/// use things3_common::format_datetime;
/// use chrono::{TimeZone, Utc};
///
/// let dt = Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();
/// assert_eq!(format_datetime(&dt), "2024-01-15 14:30:00 UTC");
/// ```
#[must_use]
pub fn format_datetime(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Parse a date string in YYYY-MM-DD format
///
/// # Examples
///
/// ```
/// use things3_common::parse_date;
///
/// // Valid date
/// let date = parse_date("2024-01-15").unwrap();
/// assert_eq!(date.to_string(), "2024-01-15");
///
/// // Invalid date format returns error
/// assert!(parse_date("01/15/2024").is_err());
/// assert!(parse_date("2024-13-01").is_err()); // Invalid month
/// ```
///
/// # Errors
/// Returns `chrono::ParseError` if the date string is not in the expected format
pub fn parse_date(date_str: &str) -> Result<NaiveDate, chrono::ParseError> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
}

/// Validate a UUID string
///
/// # Examples
///
/// ```
/// use things3_common::is_valid_uuid;
///
/// // Valid UUIDs
/// assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
/// assert!(is_valid_uuid("ffffffff-ffff-ffff-ffff-ffffffffffff"));
///
/// // Invalid UUIDs
/// assert!(!is_valid_uuid("not-a-uuid"));
/// assert!(!is_valid_uuid("550e8400-e29b")); // Too short
/// assert!(!is_valid_uuid("")); // Empty string
/// ```
#[must_use]
pub fn is_valid_uuid(uuid_str: &str) -> bool {
    uuid::Uuid::parse_str(uuid_str).is_ok()
}

/// Truncate a string to a maximum length
///
/// # Examples
///
/// ```
/// use things3_common::truncate_string;
///
/// assert_eq!(truncate_string("hello world", 5), "he...");
/// assert_eq!(truncate_string("hi", 10), "hi");
/// assert_eq!(truncate_string("test", 3), "...");
/// ```
#[must_use]
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, TimeZone, Utc};

    #[test]
    fn test_format_date() {
        let date = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap();
        let formatted = format_date(&date);
        assert_eq!(formatted, "2023-12-25");

        // Test edge cases
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let formatted = format_date(&date);
        assert_eq!(formatted, "2024-01-01");

        let date = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(); // Leap year
        let formatted = format_date(&date);
        assert_eq!(formatted, "2024-02-29");
    }

    #[test]
    fn test_format_datetime() {
        // Test with specific datetime for predictable results
        let dt = Utc.with_ymd_and_hms(2023, 12, 25, 15, 30, 45).unwrap();
        let formatted = format_datetime(&dt);
        assert_eq!(formatted, "2023-12-25 15:30:45 UTC");

        // Test edge cases
        let dt = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let formatted = format_datetime(&dt);
        assert_eq!(formatted, "2024-01-01 00:00:00 UTC");
    }

    #[test]
    fn test_parse_date_valid() {
        let result = parse_date("2023-12-25");
        assert!(result.is_ok());
        let date = result.unwrap();
        assert_eq!(date.year(), 2023);
        assert_eq!(date.month(), 12);
        assert_eq!(date.day(), 25);

        // Test edge cases
        assert!(parse_date("2024-01-01").is_ok());
        assert!(parse_date("2024-02-29").is_ok()); // Leap year
    }

    #[test]
    fn test_parse_date_invalid() {
        // Test invalid formats
        assert!(parse_date("2023/12/25").is_err());
        assert!(parse_date("2023-13-01").is_err()); // Invalid month
        assert!(parse_date("2023-02-30").is_err()); // Invalid day
        assert!(parse_date("").is_err());
        assert!(parse_date("not-a-date").is_err());
        assert!(parse_date("2023-02-29").is_err()); // Non-leap year Feb 29
    }

    #[test]
    fn test_is_valid_uuid_valid() {
        // Test valid UUIDs
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(is_valid_uuid("6ba7b810-9dad-11d1-80b4-00c04fd430c8"));
        assert!(is_valid_uuid("00000000-0000-0000-0000-000000000000"));
        assert!(is_valid_uuid("ffffffff-ffff-ffff-ffff-ffffffffffff"));
        assert!(is_valid_uuid("FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF")); // Uppercase
    }

    #[test]
    fn test_is_valid_uuid_invalid() {
        // Test invalid UUIDs
        assert!(!is_valid_uuid(""));
        assert!(!is_valid_uuid("not-a-uuid"));
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716")); // Too short
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000g")); // Invalid char
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-446655440000-extra")); // Extra content
    }

    #[test]
    fn test_truncate_string() {
        // Test string shorter than max length
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello", 5), "hello");

        // Test string longer than max length
        assert_eq!(truncate_string("hello world", 8), "hello...");
        assert_eq!(truncate_string("hello world", 5), "he...");

        // Test edge cases
        assert_eq!(truncate_string("hello", 3), "...");
        assert_eq!(truncate_string("hello", 4), "h...");
        assert_eq!(truncate_string("", 10), "");
        assert_eq!(truncate_string("", 0), "");
        assert_eq!(truncate_string("test", 0), "...");
    }

    #[test]
    fn test_integration() {
        // Test integration between functions
        let date_str = "2023-12-25";
        let parsed_date = parse_date(date_str).unwrap();
        let formatted_date = format_date(&parsed_date);
        assert_eq!(formatted_date, date_str);

        // Test UUID validation with truncation
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert!(is_valid_uuid(uuid));
        let truncated = truncate_string(uuid, 20);
        assert_eq!(truncated, "550e8400-e29b-41d...");
    }

    #[test]
    fn test_comprehensive_coverage() {
        // Additional tests to ensure comprehensive coverage

        // Test all months for format_date
        for month in 1..=12 {
            let date = NaiveDate::from_ymd_opt(2023, month, 1).unwrap();
            let formatted = format_date(&date);
            assert!(formatted.contains(&format!("{month:02}")));
        }

        // Test various datetime formats
        let times = [(0, 0, 0), (12, 0, 0), (23, 59, 59)];
        for (hour, min, sec) in times {
            let dt = Utc.with_ymd_and_hms(2023, 6, 15, hour, min, sec).unwrap();
            let formatted = format_datetime(&dt);
            assert!(formatted.contains(&format!("{hour:02}:{min:02}:{sec:02}")));
            assert!(formatted.ends_with("UTC"));
        }

        // Test more invalid date formats
        let invalid_dates = [
            "2023",
            "2023-01",
            "01-01-2023",
            "2023.01.01",
            "2023-00-01",
            "2023-01-00",
            "2023-04-31",
        ];
        for date_str in &invalid_dates {
            assert!(parse_date(date_str).is_err());
        }

        // Test more invalid UUIDs
        let invalid_uuids = [
            "550e8400_e29b_41d4_a716_446655440000",  // Underscores
            "550e8400.e29b.41d4.a716.446655440000",  // Dots
            " 550e8400-e29b-41d4-a716-446655440000", // Leading space
            "550e8400-e29b-41d4-a716-446655440000 ", // Trailing space
        ];
        for uuid in &invalid_uuids {
            assert!(!is_valid_uuid(uuid));
        }

        // Test truncate_string with Unicode
        assert_eq!(truncate_string("hello 世界", 8), "hello...");
        assert_eq!(truncate_string("🦀🦀🦀", 3), "...");
    }
}
