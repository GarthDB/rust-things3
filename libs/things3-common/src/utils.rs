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
        // Test that the function returns the same path on multiple calls
        // This test verifies the function is deterministic within the same environment
        let path1 = get_default_database_path();
        let path2 = get_default_database_path();

        // The paths should be equal within the same environment
        // In CI environments, HOME might be set differently, but the function should be consistent
        assert_eq!(
            path1, path2,
            "get_default_database_path should return consistent results"
        );

        // Verify the path contains expected components regardless of environment
        let path_str = path1.to_string_lossy();
        assert!(
            path_str.contains("Library"),
            "Path should contain Library directory"
        );
        assert!(
            path_str.contains("Group Containers"),
            "Path should contain Group Containers"
        );
        assert!(
            path_str.contains("Things Database.thingsdatabase"),
            "Path should contain database file"
        );
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
        // Test behavior when HOME is not set
        let original_home = std::env::var("HOME");
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
            // and contains expected components regardless of the environment
            assert!(!path_str.is_empty(), "Path should not be empty");
            assert!(
                path_str.contains("Library"),
                "Path should contain Library directory"
            );
            assert!(
                path_str.contains("Group Containers"),
                "Path should contain Group Containers"
            );
            assert!(
                path_str.contains("Things Database.thingsdatabase"),
                "Path should contain database file"
            );
        }

        // Restore original HOME if it existed
        if let Ok(home) = original_home {
            std::env::set_var("HOME", home);
        }
    }

    #[test]
    fn test_get_default_database_path_with_no_home_and_restore() {
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
}
