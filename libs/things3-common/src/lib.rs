//! Things Common - Shared utilities and types for Things 3 integration
//!
//! This crate provides shared utilities and constants for Things 3 integration.
//!
//! # Examples
//!
//! ```
//! use things3_common::{DATABASE_FILENAME, get_default_database_path, truncate_string};
//!
//! // Use constants
//! assert_eq!(DATABASE_FILENAME, "main.sqlite");
//!
//! // Use utility functions
//! let path = get_default_database_path();
//! assert!(!path.to_string_lossy().is_empty());
//!
//! let truncated = truncate_string("hello world", 5);
//! assert_eq!(truncated, "he...");
//! ```

pub mod constants;
pub mod utils;

pub use constants::*;
pub use utils::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_re_exported_constants() {
        // Test that constants are properly re-exported from the crate root
        assert_eq!(DATABASE_FILENAME, "main.sqlite");
        assert_eq!(DATABASE_DIR, "Things Database.thingsdatabase");
        assert_eq!(THINGS_CONTAINER, "JLMPQHK8H4.com.culturedcode.Things3");
        assert_eq!(DEFAULT_QUERY_LIMIT, 100);
        assert_eq!(MAX_QUERY_LIMIT, 1000);
        assert_eq!(DEFAULT_MCP_PORT, 3000);

        // Test that arrays are accessible
        assert_eq!(DATE_FORMATS.len(), 3);
        assert_eq!(DATETIME_FORMATS.len(), 3);

        // Test that we can access specific array elements
        assert_eq!(DATE_FORMATS[0], "%Y-%m-%d");
        assert_eq!(DATETIME_FORMATS[0], "%Y-%m-%d %H:%M:%S");
    }

    #[test]
    fn test_re_exported_functions() {
        // Test that utility functions are properly re-exported from the crate root
        use chrono::{NaiveDate, Utc};

        // Test get_default_database_path
        let path = get_default_database_path();
        assert!(!path.to_string_lossy().is_empty());

        // Test format_date
        let date = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap();
        let formatted = format_date(&date);
        assert_eq!(formatted, "2023-12-25");

        // Test format_datetime
        let dt = Utc::now();
        let formatted = format_datetime(&dt);
        assert!(formatted.contains("UTC"));

        // Test parse_date
        let result = parse_date("2023-12-25");
        assert!(result.is_ok());

        // Test is_valid_uuid
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_valid_uuid("invalid-uuid"));

        // Test truncate_string
        assert_eq!(truncate_string("hello world", 5), "he...");
        assert_eq!(truncate_string("hi", 10), "hi");
    }

    #[test]
    fn test_module_accessibility() {
        // Test that modules are accessible and contain expected items

        // Test constants module
        assert_eq!(constants::DATABASE_FILENAME, "main.sqlite");
        assert_eq!(constants::DEFAULT_QUERY_LIMIT, 100);

        // Test utils module
        let path = utils::get_default_database_path();
        assert!(!path.to_string_lossy().is_empty());

        // Test that we can use module-qualified names
        assert!(utils::is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert_eq!(utils::truncate_string("test", 2), "...");
    }

    #[test]
    fn test_crate_level_imports() {
        // Test that importing from crate root works as expected
        // This ensures the pub use statements are working correctly

        // Test importing constants directly
        use crate::{DATABASE_FILENAME, DATE_FORMATS, DEFAULT_QUERY_LIMIT};
        // Test importing functions directly
        use crate::{get_default_database_path, is_valid_uuid, truncate_string};

        assert_eq!(DATABASE_FILENAME, "main.sqlite");
        assert_eq!(DEFAULT_QUERY_LIMIT, 100);
        assert_eq!(DATE_FORMATS.len(), 3);

        let path = get_default_database_path();
        assert!(!path.to_string_lossy().is_empty());
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert_eq!(truncate_string("hello", 3), "...");
    }

    #[test]
    fn test_wildcard_imports() {
        // Test that wildcard imports work correctly
        // This tests the pub use *; statements

        // Create a scope to test wildcard import
        {
            use crate::constants::*;
            assert_eq!(DATABASE_FILENAME, "main.sqlite");
            assert_eq!(DEFAULT_QUERY_LIMIT, 100);
        }

        {
            use crate::utils::*;
            let path = get_default_database_path();
            assert!(!path.to_string_lossy().is_empty());
            assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        }
    }
}
