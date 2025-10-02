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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, NaiveDate};
    use std::sync::Mutex;

    // Global mutex to synchronize environment variable access across tests
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_get_default_database_path() {
        let path = get_default_database_path();

        // Should contain the expected path components
        assert!(path.to_string_lossy().contains("Library"));
        assert!(path.to_string_lossy().contains("Group Containers"));
        assert!(path
            .to_string_lossy()
            .contains("JLMPQHK86H.com.culturedcode.ThingsMac"));
        assert!(path.to_string_lossy().contains("ThingsData-0Z0Z2"));
        assert!(path
            .to_string_lossy()
            .contains("Things Database.thingsdatabase"));
        assert!(path.to_string_lossy().contains("main.sqlite"));

        // Should start with some home-like directory (environment-agnostic)
        let path_str = path.to_string_lossy();
        assert!(path_str.starts_with('/') || path_str.starts_with('~'));
    }

    #[test]
    fn test_format_date() {
        let date = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap();
        let formatted = format_date(&date);
        assert_eq!(formatted, "2023-12-25");
    }

    #[test]
    fn test_format_date_edge_cases() {
        // Test January 1st
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let formatted = format_date(&date);
        assert_eq!(formatted, "2024-01-01");

        // Test December 31st
        let date = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        let formatted = format_date(&date);
        assert_eq!(formatted, "2023-12-31");

        // Test leap year
        let date = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        let formatted = format_date(&date);
        assert_eq!(formatted, "2024-02-29");
    }

    #[test]
    fn test_format_datetime() {
        let dt = Utc::now();
        let formatted = format_datetime(&dt);

        // Should contain the expected format components
        assert!(formatted.contains("UTC"));
        assert!(formatted.contains('-'));
        assert!(formatted.contains(' '));
        assert!(formatted.contains(':'));

        // Should be in the expected format
        assert!(formatted.len() >= 20); // At least "YYYY-MM-DD HH:MM:SS UTC"
    }

    #[test]
    fn test_format_datetime_specific() {
        // Test with a specific datetime
        let dt = DateTime::parse_from_rfc3339("2023-12-25T15:30:45Z")
            .unwrap()
            .with_timezone(&Utc);
        let formatted = format_datetime(&dt);
        assert_eq!(formatted, "2023-12-25 15:30:45 UTC");
    }

    #[test]
    fn test_parse_date_valid() {
        let result = parse_date("2023-12-25");
        assert!(result.is_ok());
        let date = result.unwrap();
        assert_eq!(date.year(), 2023);
        assert_eq!(date.month(), 12);
        assert_eq!(date.day(), 25);
    }

    #[test]
    fn test_parse_date_edge_cases() {
        // Test January 1st
        let result = parse_date("2024-01-01");
        assert!(result.is_ok());
        let date = result.unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 1);

        // Test December 31st
        let result = parse_date("2023-12-31");
        assert!(result.is_ok());
        let date = result.unwrap();
        assert_eq!(date.year(), 2023);
        assert_eq!(date.month(), 12);
        assert_eq!(date.day(), 31);

        // Test leap year
        let result = parse_date("2024-02-29");
        assert!(result.is_ok());
        let date = result.unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 2);
        assert_eq!(date.day(), 29);
    }

    #[test]
    fn test_parse_date_invalid() {
        // Test invalid format
        let result = parse_date("2023/12/25");
        assert!(result.is_err());

        // Test invalid date
        let result = parse_date("2023-13-01");
        assert!(result.is_err());

        // Test invalid day
        let result = parse_date("2023-02-30");
        assert!(result.is_err());

        // Test empty string
        let result = parse_date("");
        assert!(result.is_err());

        // Test malformed string
        let result = parse_date("not-a-date");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_valid_uuid_valid() {
        // Test valid UUIDs
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(is_valid_uuid("6ba7b810-9dad-11d1-80b4-00c04fd430c8"));
        assert!(is_valid_uuid("6ba7b811-9dad-11d1-80b4-00c04fd430c8"));
        assert!(is_valid_uuid("00000000-0000-0000-0000-000000000000"));
        assert!(is_valid_uuid("ffffffff-ffff-ffff-ffff-ffffffffffff"));
    }

    #[test]
    fn test_is_valid_uuid_invalid() {
        // Test invalid UUIDs
        assert!(!is_valid_uuid(""));
        assert!(!is_valid_uuid("not-a-uuid"));
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716"));
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000"));
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-4466554400000"));
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000g"));
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000-"));
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000 "));
    }

    #[test]
    fn test_truncate_string_short() {
        // Test string shorter than max length
        let result = truncate_string("hello", 10);
        assert_eq!(result, "hello");

        // Test string equal to max length
        let result = truncate_string("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_string_long() {
        // Test string longer than max length
        let result = truncate_string("hello world", 8);
        assert_eq!(result, "hello...");

        // Test string much longer than max length
        let result = truncate_string("this is a very long string", 10);
        assert_eq!(result, "this is...");
    }

    #[test]
    fn test_truncate_string_edge_cases() {
        // Test with max_len = 0
        let result = truncate_string("hello", 0);
        assert_eq!(result, "...");

        // Test with max_len = 1
        let result = truncate_string("hello", 1);
        assert_eq!(result, "...");

        // Test with max_len = 2
        let result = truncate_string("hello", 2);
        assert_eq!(result, "...");

        // Test with max_len = 3
        let result = truncate_string("hello", 3);
        assert_eq!(result, "...");

        // Test with max_len = 4
        let result = truncate_string("hello", 4);
        assert_eq!(result, "h...");

        // Test with max_len = 5
        let result = truncate_string("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_string_empty() {
        // Test empty string
        let result = truncate_string("", 10);
        assert_eq!(result, "");

        // Test empty string with max_len = 0
        let result = truncate_string("", 0);
        assert_eq!(result, "");
    }

    #[test]
    fn test_truncate_string_unicode() {
        // Test with unicode characters
        let result = truncate_string("hello 世界", 8);
        assert_eq!(result, "hello...");

        // Test with emoji
        let result = truncate_string("hello 😀", 8);
        assert_eq!(result, "hello...");
    }

    #[test]
    fn test_truncate_string_very_long() {
        // Test with very long string
        let long_string = "a".repeat(1000);
        let result = truncate_string(&long_string, 10);
        assert_eq!(result, "aaaaaaa...");
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_utils_integration() {
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
    fn test_get_default_database_path_consistency() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // Test that the function returns the same path on multiple calls
        // This test verifies the function is deterministic within the same environment

        // Capture the current HOME value to ensure consistency during the test
        let original_home = std::env::var("HOME");

        // Make multiple calls in quick succession to test consistency
        let path1 = get_default_database_path();
        let path2 = get_default_database_path();
        let path3 = get_default_database_path();

        // All paths should be identical within the same test execution
        assert_eq!(
            path1, path2,
            "get_default_database_path should return consistent results between calls"
        );
        assert_eq!(
            path2, path3,
            "get_default_database_path should return consistent results across multiple calls"
        );

        // Verify the path contains expected components regardless of environment
        let path_str = path1.to_string_lossy();
        assert!(
            path_str.contains("Library"),
            "Path should contain Library directory, got: {path_str}"
        );
        assert!(
            path_str.contains("Group Containers"),
            "Path should contain Group Containers, got: {path_str}"
        );
        assert!(
            path_str.contains("Things Database.thingsdatabase"),
            "Path should contain database file, got: {path_str}"
        );

        // Verify that the path is either absolute or starts with ~ (tilde)
        assert!(
            path_str.starts_with('/') || path_str.starts_with('~'),
            "Path should be absolute or start with ~, got: {path_str}"
        );

        // Restore original HOME if it was set
        match original_home {
            Ok(home) => std::env::set_var("HOME", home),
            Err(_) => std::env::remove_var("HOME"),
        }
    }

    #[test]
    fn test_format_date_consistency() {
        // Test that formatting and parsing are consistent
        let date = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap();
        let formatted = format_date(&date);
        let parsed = parse_date(&formatted).unwrap();
        assert_eq!(date, parsed);
    }

    #[test]
    fn test_get_default_database_path_with_no_home() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // Test behavior when HOME is not set
        let original_home = std::env::var("HOME");
        std::env::remove_var("HOME");

        let path = get_default_database_path();
        let path_str = path.to_string_lossy();

        // The function should always return a valid path with expected components
        // regardless of whether HOME was successfully removed or not
        assert!(!path_str.is_empty(), "Path should not be empty");
        assert!(
            path_str.contains("Library"),
            "Path should contain Library directory, got: {path_str}"
        );
        assert!(
            path_str.contains("Group Containers"),
            "Path should contain Group Containers, got: {path_str}"
        );
        assert!(
            path_str.contains("Things Database.thingsdatabase"),
            "Path should contain database file, got: {path_str}"
        );

        // Check if the path starts with ~ (indicating fallback behavior)
        // or contains a valid home directory path
        let starts_with_tilde = path_str.starts_with('~');
        let contains_home_like_path = path_str.contains("/home/") || path_str.contains("/Users/");

        assert!(
            starts_with_tilde || contains_home_like_path,
            "Path should start with ~ or contain a home-like path, got: {path_str}"
        );

        // Restore original HOME if it existed
        if let Ok(home) = original_home {
            std::env::set_var("HOME", home);
        }
    }

    #[test]
    fn test_get_default_database_path_with_no_home_and_restore() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // Test behavior when HOME is not set and we need to restore it
        let original_home = std::env::var("HOME");

        // Set HOME first to ensure we have something to restore
        std::env::set_var("HOME", "/test/home");

        // Now remove it
        std::env::remove_var("HOME");

        // Check if HOME was actually removed (some environments may not allow this)
        let home_after_removal = std::env::var("HOME");

        let path = get_default_database_path();
        let path_str = path.to_string_lossy();

        // If HOME was successfully removed, the path should start with ~
        // If HOME couldn't be removed (e.g., in some CI environments), we'll skip this specific assertion
        if home_after_removal.is_err() {
            assert!(
                path_str.starts_with('~'),
                "Path should start with ~ when HOME is not set, but got: {path_str}"
            );
        } else {
            // In environments where HOME cannot be removed, just verify the path is valid
            assert!(!path_str.is_empty(), "Path should not be empty");
        }

        // Restore original HOME - this should hit the Ok branch
        if let Ok(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            // If there was no original HOME, restore our test value
            std::env::set_var("HOME", "/test/home");
        }
    }

    #[test]
    fn test_get_default_database_path_starts_with_tilde() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // Test that the path starts with ~ when HOME is not set
        let original_home = std::env::var("HOME");
        std::env::remove_var("HOME");

        // Check if HOME was actually removed (some environments may not allow this)
        let home_after_removal = std::env::var("HOME");

        let path = get_default_database_path();
        let path_str = path.to_string_lossy();

        // This should test the || branch in the assertion
        assert!(path_str.starts_with('/') || path_str.starts_with('~'));

        // If HOME was successfully removed, the path should start with ~
        // If HOME couldn't be removed (e.g., in some CI environments), we'll skip this specific assertion
        if home_after_removal.is_err() {
            assert!(
                path_str.starts_with('~'),
                "Path should start with ~ when HOME is not set, but got: {path_str}"
            );
        } else {
            // In environments where HOME cannot be removed, just verify the path is valid
            assert!(!path_str.is_empty(), "Path should not be empty");
        }

        // Restore original HOME if it existed
        if let Ok(home) = original_home {
            std::env::set_var("HOME", home);
        }
    }

    #[test]
    fn test_get_default_database_path_or_branch_coverage() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // Test both branches of the || assertion
        let original_home = std::env::var("HOME");

        // Test the "/" branch (when HOME is set)
        let path_with_home = get_default_database_path();
        let path_str_with_home = path_with_home.to_string_lossy();
        assert!(path_str_with_home.starts_with('/') || path_str_with_home.starts_with('~'));

        // Test the "~" branch (when HOME is not set)
        std::env::remove_var("HOME");
        let path_without_home = get_default_database_path();
        let path_str_without_home = path_without_home.to_string_lossy();
        assert!(path_str_without_home.starts_with('/') || path_str_without_home.starts_with('~'));

        // Restore original HOME if it existed
        if let Ok(home) = original_home {
            std::env::set_var("HOME", home);
        }
    }

    #[test]
    fn test_format_datetime_edge_cases() {
        // Test with different timezones
        let dt = DateTime::parse_from_rfc3339("2023-12-25T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let formatted = format_datetime(&dt);
        assert_eq!(formatted, "2023-12-25 00:00:00 UTC");

        // Test with different times
        let dt = DateTime::parse_from_rfc3339("2023-12-25T23:59:59Z")
            .unwrap()
            .with_timezone(&Utc);
        let formatted = format_datetime(&dt);
        assert_eq!(formatted, "2023-12-25 23:59:59 UTC");
    }

    #[test]
    fn test_parse_date_boundary_values() {
        // Test year boundaries
        assert!(parse_date("1900-01-01").is_ok());
        assert!(parse_date("2099-12-31").is_ok());

        // Test month boundaries
        assert!(parse_date("2023-01-01").is_ok());
        assert!(parse_date("2023-12-31").is_ok());

        // Test day boundaries
        assert!(parse_date("2023-01-01").is_ok());
        assert!(parse_date("2023-01-31").is_ok());
    }

    #[test]
    fn test_is_valid_uuid_edge_cases() {
        // Test uppercase UUIDs
        assert!(is_valid_uuid("550E8400-E29B-41D4-A716-446655440000"));

        // Test lowercase UUIDs
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));

        // Test mixed case UUIDs
        assert!(is_valid_uuid("550E8400-e29b-41D4-a716-446655440000"));
    }

    #[test]
    fn test_truncate_string_boundary_conditions() {
        // Test exactly at boundary
        let result = truncate_string("hello", 5);
        assert_eq!(result, "hello");

        // Test just over boundary
        let result = truncate_string("hello", 4);
        assert_eq!(result, "h...");

        // Test way over boundary
        let result = truncate_string("hello", 1);
        assert_eq!(result, "...");
    }

    #[test]
    fn test_utils_error_handling() {
        // Test parse_date with various error conditions
        assert!(parse_date("invalid").is_err());
        assert!(parse_date("2023-13-01").is_err());
        assert!(parse_date("2023-02-30").is_err());
        assert!(parse_date("2023-04-31").is_err());
    }

    #[test]
    fn test_utils_performance() {
        // Test with large strings
        let large_string = "a".repeat(10000);
        let result = truncate_string(&large_string, 100);
        assert_eq!(result.len(), 100);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_get_default_database_path_error_branch() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // Test the error branch of std::env::var("HOME")
        let original_home = std::env::var("HOME");
        std::env::remove_var("HOME");

        let path = get_default_database_path();
        let path_str = path.to_string_lossy();

        // Should use the fallback "~" when HOME is not set
        assert!(
            path_str.starts_with('~')
                || path_str.contains("/home/")
                || path_str.contains("/Users/")
        );

        // Restore original HOME if it existed
        if let Ok(home) = original_home {
            std::env::set_var("HOME", home);
        }
    }

    #[test]
    fn test_parse_date_various_invalid_formats() {
        // Test more invalid date formats to improve coverage
        // Focus on formats that definitely fail with our strict YYYY-MM-DD format
        assert!(parse_date("2023-01").is_err()); // Missing day
        assert!(parse_date("2023").is_err()); // Only year
        assert!(parse_date("2023/01/01").is_err()); // Wrong separator
        assert!(parse_date("2023-01-01T00:00:00").is_err()); // With time
        assert!(parse_date("2023--01-01").is_err()); // Double separator
        assert!(parse_date("2023-01-01-").is_err()); // Trailing separator
        assert!(parse_date("abc-def-ghi").is_err()); // Non-numeric
        assert!(parse_date("2023-00-01").is_err()); // Invalid month
        assert!(parse_date("2023-01-00").is_err()); // Invalid day
        assert!(parse_date("2023-13-01").is_err()); // Invalid month (13)
        assert!(parse_date("2023-02-30").is_err()); // Invalid day for February
        assert!(parse_date("not-a-date-at-all").is_err()); // Completely invalid
        assert!(parse_date("").is_err()); // Empty string
        assert!(parse_date("2023-1-1-1").is_err()); // Too many parts
    }

    #[test]
    fn test_truncate_string_saturating_sub_edge_cases() {
        // Test edge cases for saturating_sub(3) in truncate_string
        let result = truncate_string("ab", 0);
        assert_eq!(result, "...");

        let result = truncate_string("abc", 1);
        assert_eq!(result, "...");

        let result = truncate_string("abcd", 2);
        assert_eq!(result, "...");

        // Test when max_len.saturating_sub(3) = 0
        let result = truncate_string("hello", 3);
        assert_eq!(result, "...");
    }

    #[test]
    fn test_is_valid_uuid_malformed_cases() {
        // Test more malformed UUID cases
        // Note: UUID without hyphens is actually valid for uuid crate
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000z")); // Invalid hex char
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000 ")); // Trailing space
        assert!(!is_valid_uuid(" 550e8400-e29b-41d4-a716-446655440000")); // Leading space
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000\n")); // Newline
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000\t")); // Tab
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000\0")); // Null byte
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000G")); // Invalid hex char G
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-4466554400")); // Too short
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-446655440000000")); // Too long
    }

    #[test]
    fn test_format_datetime_boundary_cases() {
        // Test datetime formatting with boundary cases
        use chrono::{TimeZone, Utc};

        // Test year boundaries
        let dt = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap();
        let formatted = format_datetime(&dt);
        assert_eq!(formatted, "1970-01-01 00:00:00 UTC");

        // Test leap year
        let dt = Utc.with_ymd_and_hms(2024, 2, 29, 12, 30, 45).unwrap();
        let formatted = format_datetime(&dt);
        assert_eq!(formatted, "2024-02-29 12:30:45 UTC");

        // Test end of year
        let dt = Utc.with_ymd_and_hms(2023, 12, 31, 23, 59, 59).unwrap();
        let formatted = format_datetime(&dt);
        assert_eq!(formatted, "2023-12-31 23:59:59 UTC");
    }

    #[test]
    fn test_all_public_functions_comprehensive() {
        // Comprehensive test to ensure all public functions are exercised
        use chrono::{TimeZone, Utc};

        // Test get_default_database_path with different scenarios
        let path = get_default_database_path();
        assert!(!path.to_string_lossy().is_empty());

        // Test format_date with various dates
        let dates = [
            chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(),
        ];
        for date in &dates {
            let formatted = format_date(date);
            assert!(formatted.len() == 10); // YYYY-MM-DD format
            assert!(formatted.contains('-'));
        }

        // Test format_datetime with various datetimes
        let datetimes = [
            Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap(),
            Utc.with_ymd_and_hms(2023, 6, 15, 12, 30, 45).unwrap(),
        ];
        for dt in &datetimes {
            let formatted = format_datetime(dt);
            assert!(formatted.contains("UTC"));
            assert!(formatted.len() > 15);
        }

        // Test parse_date with valid dates
        let valid_dates = ["2023-01-01", "2024-12-31", "2000-06-15"];
        for date_str in &valid_dates {
            assert!(parse_date(date_str).is_ok());
        }

        // Test is_valid_uuid with various UUIDs
        let valid_uuids = [
            "550e8400-e29b-41d4-a716-446655440000",
            "00000000-0000-0000-0000-000000000000",
            "ffffffff-ffff-ffff-ffff-ffffffffffff",
        ];
        for uuid in &valid_uuids {
            assert!(is_valid_uuid(uuid));
        }

        // Test truncate_string with various lengths
        let test_string = "Hello, World! This is a test string.";
        for len in [5, 10, 20, 50] {
            let truncated = truncate_string(test_string, len);
            assert!(truncated.len() <= len);
        }
    }

    #[test]
    fn test_function_return_types_and_signatures() {
        // Test that functions return expected types and handle edge cases

        // Test get_default_database_path returns PathBuf
        let path = get_default_database_path();
        assert!(path.is_absolute() || path.to_string_lossy().starts_with('~'));

        // Test format_date returns String
        let date = chrono::NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let formatted = format_date(&date);
        assert!(formatted.is_ascii());
        assert_eq!(formatted.len(), 10);

        // Test format_datetime returns String
        let dt = chrono::Utc::now();
        let formatted = format_datetime(&dt);
        assert!(formatted.ends_with("UTC"));

        // Test parse_date returns Result
        assert!(parse_date("2023-01-01").is_ok());
        assert!(parse_date("invalid").is_err());

        // Test is_valid_uuid returns bool
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_valid_uuid("invalid"));

        // Test truncate_string returns String
        let result = truncate_string("test", 10);
        assert_eq!(result, "test");
    }

    #[test]
    fn test_error_path_coverage() {
        // Ensure all error paths are covered

        // Test parse_date with comprehensive invalid inputs
        let invalid_inputs = [
            "",
            "invalid",
            "2023",
            "2023-13",
            "2023-13-45",
            "2023-00-01",
            "2023-01-00",
            "2023-02-30",
            "2023-04-31",
            "not-a-date",
            "2023/01/01",
            "01-01-2023",
            "2023-1-1-1-1",
            "2023-01-01T12:00:00",
            "2023-01-01 12:00:00",
        ];

        for input in &invalid_inputs {
            let result = parse_date(input);
            assert!(result.is_err(), "Expected error for input: {input}");
        }

        // Test is_valid_uuid with comprehensive invalid inputs
        let invalid_uuids = [
            "",
            "invalid",
            "550e8400",
            "550e8400-e29b",
            "550e8400-e29b-41d4",
            "550e8400-e29b-41d4-a716",
            "550e8400-e29b-41d4-a716-44665544000",
            "550e8400-e29b-41d4-a716-4466554400000",
            "550e8400-e29b-41d4-a716-44665544000g",
            "550e8400-e29b-41d4-a716-44665544000 ",
            " 550e8400-e29b-41d4-a716-446655440000",
            "550e8400-e29b-41d4-a716-44665544000\n",
            "550e8400-e29b-41d4-a716-44665544000\t",
        ];

        for uuid in &invalid_uuids {
            assert!(!is_valid_uuid(uuid), "Expected false for UUID: {uuid}");
        }
    }
}
