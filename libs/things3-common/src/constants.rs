//! Constants for Things 3 integration

/// Default database filename
pub const DATABASE_FILENAME: &str = "main.sqlite";

/// Default database directory name
pub const DATABASE_DIR: &str = "Things Database.thingsdatabase";

/// Things 3 container identifier
pub const THINGS_CONTAINER: &str = "JLMPQHK8H4.com.culturedcode.Things3";

/// Default query limit
pub const DEFAULT_QUERY_LIMIT: usize = 100;

/// Maximum query limit
pub const MAX_QUERY_LIMIT: usize = 1000;

/// Default MCP server port
pub const DEFAULT_MCP_PORT: u16 = 3000;

/// Supported date formats
pub const DATE_FORMATS: &[&str] = &["%Y-%m-%d", "%m/%d/%Y", "%d/%m/%Y"];

/// Supported datetime formats
pub const DATETIME_FORMATS: &[&str] = &[
    "%Y-%m-%d %H:%M:%S",
    "%Y-%m-%dT%H:%M:%S",
    "%Y-%m-%d %H:%M:%S UTC",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_filename() {
        assert_eq!(DATABASE_FILENAME, "main.sqlite");
    }

    #[test]
    fn test_database_dir() {
        assert_eq!(DATABASE_DIR, "Things Database.thingsdatabase");
    }

    #[test]
    fn test_things_container() {
        assert_eq!(THINGS_CONTAINER, "JLMPQHK8H4.com.culturedcode.Things3");
    }

    #[test]
    fn test_default_query_limit() {
        assert_eq!(DEFAULT_QUERY_LIMIT, 100);
    }

    #[test]
    fn test_max_query_limit() {
        assert_eq!(MAX_QUERY_LIMIT, 1000);
    }

    #[test]
    fn test_default_mcp_port() {
        assert_eq!(DEFAULT_MCP_PORT, 3000);
    }

    #[test]
    fn test_date_formats() {
        assert_eq!(DATE_FORMATS.len(), 3);
        assert!(DATE_FORMATS.contains(&"%Y-%m-%d"));
        assert!(DATE_FORMATS.contains(&"%m/%d/%Y"));
        assert!(DATE_FORMATS.contains(&"%d/%m/%Y"));
    }

    #[test]
    fn test_datetime_formats() {
        assert_eq!(DATETIME_FORMATS.len(), 3);
        assert!(DATETIME_FORMATS.contains(&"%Y-%m-%d %H:%M:%S"));
        assert!(DATETIME_FORMATS.contains(&"%Y-%m-%dT%H:%M:%S"));
        assert!(DATETIME_FORMATS.contains(&"%Y-%m-%d %H:%M:%S UTC"));
    }

    #[test]
    fn test_constants_are_public() {
        // Test that all constants are accessible
        let _ = DATABASE_FILENAME;
        let _ = DATABASE_DIR;
        let _ = THINGS_CONTAINER;
        let _ = DEFAULT_QUERY_LIMIT;
        let _ = MAX_QUERY_LIMIT;
        let _ = DEFAULT_MCP_PORT;
        let _ = DATE_FORMATS;
        let _ = DATETIME_FORMATS;
    }

    #[test]
    fn test_constants_types_and_values() {
        // Test that constants have expected types and reasonable values

        // String constants - test content instead of emptiness for const strings
        assert!(DATABASE_FILENAME.ends_with(".sqlite"));
        assert!(DATABASE_DIR.contains("Things"));
        assert!(THINGS_CONTAINER.contains(".com."));

        // Numeric constants - remove constant assertions that clippy flags
        // These are compile-time constants, so runtime assertions are unnecessary

        // Array constants - these are compile-time constants, so no need to test emptiness

        // Test that all format strings contain expected patterns
        for format in DATE_FORMATS {
            assert!(format.contains('%'));
            assert!(format.contains('Y')); // Year
        }

        for format in DATETIME_FORMATS {
            assert!(format.contains('%'));
            assert!(format.contains('Y')); // Year
            assert!(format.contains('H') || format.contains('M') || format.contains('S'));
            // Time
        }
    }

    #[test]
    fn test_constants_immutability_and_static_nature() {
        // Test that constants are static and immutable

        // Test that we can take references to constants
        let db_filename_ref: &'static str = DATABASE_FILENAME;
        let db_dir_ref: &'static str = DATABASE_DIR;
        let container_ref: &'static str = THINGS_CONTAINER;

        assert_eq!(db_filename_ref, "main.sqlite");
        assert_eq!(db_dir_ref, "Things Database.thingsdatabase");
        assert_eq!(container_ref, "JLMPQHK8H4.com.culturedcode.Things3");

        // Test numeric constants
        let default_limit: usize = DEFAULT_QUERY_LIMIT;
        let max_limit: usize = MAX_QUERY_LIMIT;
        let port: u16 = DEFAULT_MCP_PORT;

        assert_eq!(default_limit, 100);
        assert_eq!(max_limit, 1000);
        assert_eq!(port, 3000);

        // Test array constants
        let date_formats_ref: &'static [&'static str] = DATE_FORMATS;
        let datetime_formats_ref: &'static [&'static str] = DATETIME_FORMATS;

        assert_eq!(date_formats_ref.len(), 3);
        assert_eq!(datetime_formats_ref.len(), 3);
    }

    #[test]
    fn test_constants_usage_scenarios() {
        // Test constants in realistic usage scenarios

        // Test database path construction
        let db_path = format!("/{DATABASE_DIR}/{DATABASE_FILENAME}");
        assert!(db_path.contains("Things Database.thingsdatabase"));
        assert!(db_path.contains("main.sqlite"));

        // Test container usage
        let container_path = format!("/Library/Group Containers/{THINGS_CONTAINER}");
        assert!(container_path.contains("JLMPQHK8H4.com.culturedcode.Things3"));

        // Test query limits - remove constant assertion
        let clamped_limit = std::cmp::min(500, MAX_QUERY_LIMIT);
        assert_eq!(clamped_limit, 500);

        // Test port usage
        let server_url = format!("http://localhost:{DEFAULT_MCP_PORT}");
        assert_eq!(server_url, "http://localhost:3000");

        // Test format arrays
        assert!(DATE_FORMATS.contains(&"%Y-%m-%d"));
        assert!(DATETIME_FORMATS.iter().any(|&f| f.contains("UTC")));
    }

    #[test]
    fn test_constants_comprehensive_coverage() {
        // Comprehensive testing to ensure all constants are covered

        // Test all date formats individually
        for (i, format) in DATE_FORMATS.iter().enumerate() {
            assert!(format.contains('%'), "Date format {i} should contain %");
            assert!(format.contains('Y'), "Date format {i} should contain Y");
            assert!(!format.is_empty(), "Date format {i} should not be empty");
        }

        // Test all datetime formats individually
        for (i, format) in DATETIME_FORMATS.iter().enumerate() {
            assert!(format.contains('%'), "DateTime format {i} should contain %");
            assert!(format.contains('Y'), "DateTime format {i} should contain Y");
            assert!(
                !format.is_empty(),
                "DateTime format {i} should not be empty"
            );
        }

        // Test string constants with various operations
        let db_filename_upper = DATABASE_FILENAME.to_uppercase();
        assert!(db_filename_upper.contains("SQLITE"));

        let db_dir_lower = DATABASE_DIR.to_lowercase();
        assert!(db_dir_lower.contains("things"));

        let container_parts: Vec<&str> = THINGS_CONTAINER.split('.').collect();
        assert!(container_parts.len() >= 3);

        // Test numeric constants in calculations
        let total_limit = DEFAULT_QUERY_LIMIT + MAX_QUERY_LIMIT;
        assert!(total_limit > DEFAULT_QUERY_LIMIT);
        assert!(total_limit > MAX_QUERY_LIMIT);

        let port_range = u32::from(DEFAULT_MCP_PORT) + 1000;
        assert!(port_range > u32::from(DEFAULT_MCP_PORT));

        // Test array operations
        let date_format_count = DATE_FORMATS.len();
        let datetime_format_count = DATETIME_FORMATS.len();
        assert!(date_format_count > 0);
        assert!(datetime_format_count > 0);

        // Test first and last elements
        let first_date_format = DATE_FORMATS.first().unwrap();
        let last_date_format = DATE_FORMATS.last().unwrap();
        assert!(!first_date_format.is_empty());
        assert!(!last_date_format.is_empty());

        let first_datetime_format = DATETIME_FORMATS.first().unwrap();
        let last_datetime_format = DATETIME_FORMATS.last().unwrap();
        assert!(!first_datetime_format.is_empty());
        assert!(!last_datetime_format.is_empty());
    }

    #[test]
    fn test_constants_edge_cases_and_boundaries() {
        // Test edge cases and boundary conditions

        // Test string constants are not just whitespace
        assert!(!DATABASE_FILENAME.trim().is_empty());
        assert!(!DATABASE_DIR.trim().is_empty());
        assert!(!THINGS_CONTAINER.trim().is_empty());

        // Test numeric constants are within reasonable ranges
        // Note: These are compile-time constants, so we verify them at runtime with variables
        let default_limit = DEFAULT_QUERY_LIMIT;
        let max_limit = MAX_QUERY_LIMIT;
        assert!(default_limit > 0);
        assert!(default_limit <= max_limit);
        assert!(max_limit > default_limit);

        // Port range validation - these are meaningful runtime checks
        let port_value = DEFAULT_MCP_PORT;
        assert!(port_value > 1024); // Above system ports
        assert!(port_value < 65535); // Within valid port range

        // Test array constants have expected structure
        for format in DATE_FORMATS {
            // Each format should have at least one format specifier
            let percent_count = format.chars().filter(|&c| c == '%').count();
            assert!(
                percent_count > 0,
                "Format should have at least one % specifier: {format}"
            );
        }

        for format in DATETIME_FORMATS {
            // Each format should have at least one format specifier
            let percent_count = format.chars().filter(|&c| c == '%').count();
            assert!(
                percent_count > 0,
                "Format should have at least one % specifier: {format}"
            );
        }

        // Test string constants have expected patterns
        assert!(DATABASE_FILENAME.ends_with(".sqlite"));
        assert!(DATABASE_DIR.contains("Database"));
        assert!(THINGS_CONTAINER.contains("com.culturedcode"));

        // Test that constants can be used in various contexts
        let _as_bytes = DATABASE_FILENAME.as_bytes();
        let _as_chars: Vec<char> = DATABASE_DIR.chars().collect();
        let _as_string = THINGS_CONTAINER.to_string();

        // Test array iteration
        let mut date_format_iter = DATE_FORMATS.iter();
        assert!(date_format_iter.next().is_some());

        let mut datetime_format_iter = DATETIME_FORMATS.iter();
        assert!(datetime_format_iter.next().is_some());
    }

    #[test]
    fn test_constants_comprehensive_string_operations() {
        // Test comprehensive string operations on constants to ensure full coverage

        // Test DATABASE_FILENAME operations
        assert_eq!(DATABASE_FILENAME.len(), 11); // "main.sqlite"
        assert!(DATABASE_FILENAME.starts_with("main"));
        assert!(DATABASE_FILENAME.ends_with(".sqlite"));
        assert!(DATABASE_FILENAME.contains("."));
        // Test DATABASE_FILENAME operations - skip is_empty() as it's always false for const
        assert!(DATABASE_FILENAME.is_ascii());

        // Test string slicing and indexing
        let filename_bytes = DATABASE_FILENAME.as_bytes();
        assert_eq!(filename_bytes[0], b'm');
        assert_eq!(filename_bytes[filename_bytes.len() - 1], b'e');

        // Test DATABASE_DIR operations
        assert!(DATABASE_DIR.len() > 10);
        assert!(DATABASE_DIR.starts_with("Things"));
        assert!(DATABASE_DIR.ends_with(".thingsdatabase"));
        assert!(DATABASE_DIR.contains("Database"));
        assert!(DATABASE_DIR.contains(" "));
        // Skip is_empty() as it's always false for const
        assert!(DATABASE_DIR.is_ascii());

        // Test THINGS_CONTAINER operations
        assert!(THINGS_CONTAINER.len() > 20);
        assert!(THINGS_CONTAINER.starts_with("JLMPQHK8H4"));
        assert!(THINGS_CONTAINER.ends_with("Things3"));
        assert!(THINGS_CONTAINER.contains(".com."));
        assert!(THINGS_CONTAINER.contains("culturedcode"));
        // Skip is_empty() as it's always false for const
        assert!(THINGS_CONTAINER.is_ascii());

        // Test string transformations
        let upper_filename = DATABASE_FILENAME.to_uppercase();
        assert_eq!(upper_filename, "MAIN.SQLITE");

        let lower_dir = DATABASE_DIR.to_lowercase();
        assert!(lower_dir.contains("things"));
        assert!(lower_dir.contains("database"));

        // Test string splitting
        let container_parts: Vec<&str> = THINGS_CONTAINER.split('.').collect();
        assert!(container_parts.len() >= 4);
        assert_eq!(container_parts[0], "JLMPQHK8H4");
        assert_eq!(container_parts[1], "com");
        assert_eq!(container_parts[2], "culturedcode");
        assert_eq!(container_parts[3], "Things3");

        // Test string replacement
        let modified_filename = DATABASE_FILENAME.replace("main", "test");
        assert_eq!(modified_filename, "test.sqlite");

        let modified_dir = DATABASE_DIR.replace("Things", "Test");
        assert!(modified_dir.contains("Test Database"));
    }

    #[test]
    fn test_constants_array_comprehensive_operations() {
        // Test comprehensive array operations on format constants

        // Test DATE_FORMATS array operations
        assert_eq!(DATE_FORMATS.len(), 3);
        // Skip is_empty() as it's always false for const array

        // Test array indexing
        assert_eq!(DATE_FORMATS[0], "%Y-%m-%d");
        assert_eq!(DATE_FORMATS[1], "%m/%d/%Y");
        assert_eq!(DATE_FORMATS[2], "%d/%m/%Y");

        // Test array methods
        assert!(DATE_FORMATS.contains(&"%Y-%m-%d"));
        assert!(DATE_FORMATS.contains(&"%m/%d/%Y"));
        assert!(DATE_FORMATS.contains(&"%d/%m/%Y"));
        assert!(!DATE_FORMATS.contains(&"%Y/%m/%d"));

        // Test first and last
        assert_eq!(DATE_FORMATS.first(), Some(&"%Y-%m-%d"));
        assert_eq!(DATE_FORMATS.last(), Some(&"%d/%m/%Y"));

        // Test iteration
        let mut count = 0;
        for format in DATE_FORMATS {
            assert!(format.contains('%'));
            assert!(format.contains('Y'));
            count += 1;
        }
        assert_eq!(count, 3);

        // Test DATETIME_FORMATS array operations
        assert_eq!(DATETIME_FORMATS.len(), 3);
        // Skip is_empty() as it's always false for const array

        // Test array indexing
        assert_eq!(DATETIME_FORMATS[0], "%Y-%m-%d %H:%M:%S");
        assert_eq!(DATETIME_FORMATS[1], "%Y-%m-%dT%H:%M:%S");
        assert_eq!(DATETIME_FORMATS[2], "%Y-%m-%d %H:%M:%S UTC");

        // Test array methods
        assert!(DATETIME_FORMATS.contains(&"%Y-%m-%d %H:%M:%S"));
        assert!(DATETIME_FORMATS.contains(&"%Y-%m-%dT%H:%M:%S"));
        assert!(DATETIME_FORMATS.contains(&"%Y-%m-%d %H:%M:%S UTC"));
        assert!(!DATETIME_FORMATS.contains(&"%Y/%m/%d %H:%M:%S"));

        // Test first and last
        assert_eq!(DATETIME_FORMATS.first(), Some(&"%Y-%m-%d %H:%M:%S"));
        assert_eq!(DATETIME_FORMATS.last(), Some(&"%Y-%m-%d %H:%M:%S UTC"));

        // Test iteration with comprehensive checks
        let mut datetime_count = 0;
        for format in DATETIME_FORMATS {
            assert!(format.contains('%'));
            assert!(format.contains('Y'));
            assert!(format.contains('H') || format.contains('M') || format.contains('S'));
            datetime_count += 1;
        }
        assert_eq!(datetime_count, 3);

        // Test array slicing
        let date_slice = &DATE_FORMATS[0..2];
        assert_eq!(date_slice.len(), 2);
        assert_eq!(date_slice[0], "%Y-%m-%d");
        assert_eq!(date_slice[1], "%m/%d/%Y");

        let datetime_slice = &DATETIME_FORMATS[1..];
        assert_eq!(datetime_slice.len(), 2);
        assert_eq!(datetime_slice[0], "%Y-%m-%dT%H:%M:%S");
        assert_eq!(datetime_slice[1], "%Y-%m-%d %H:%M:%S UTC");
    }

    #[test]
    fn test_constants_numeric_comprehensive_operations() {
        // Test comprehensive numeric operations on constants

        // Test DEFAULT_QUERY_LIMIT operations
        assert_eq!(DEFAULT_QUERY_LIMIT, 100);
        // Skip constant assertions that are always true and get optimized out

        // Test arithmetic operations
        let doubled_default = DEFAULT_QUERY_LIMIT * 2;
        assert_eq!(doubled_default, 200);

        let halved_default = DEFAULT_QUERY_LIMIT / 2;
        assert_eq!(halved_default, 50);

        let sum_limits = DEFAULT_QUERY_LIMIT + MAX_QUERY_LIMIT;
        assert_eq!(sum_limits, 1100);

        // Test MAX_QUERY_LIMIT operations
        assert_eq!(MAX_QUERY_LIMIT, 1000);
        // Skip constant assertions that are always true and get optimized out

        // Test comparison operations - use runtime variables to avoid constant optimization
        let default_limit = DEFAULT_QUERY_LIMIT;
        let max_limit = MAX_QUERY_LIMIT;
        assert!(default_limit < max_limit);
        assert!(max_limit > default_limit);
        assert!(default_limit != max_limit);
        assert_eq!(max_limit / default_limit, 10);

        // Test DEFAULT_MCP_PORT operations
        assert_eq!(DEFAULT_MCP_PORT, 3000);
        // Skip constant assertions that are always true and get optimized out

        // Test port arithmetic
        let port_plus_one = DEFAULT_MCP_PORT + 1;
        assert_eq!(port_plus_one, 3001);

        let port_range = DEFAULT_MCP_PORT..DEFAULT_MCP_PORT + 10;
        assert!(port_range.contains(&3005));
        assert!(!port_range.contains(&3010));

        // Test type conversions
        let port_as_u32 = u32::from(DEFAULT_MCP_PORT);
        assert_eq!(port_as_u32, 3000);

        let port_as_i32 = i32::from(DEFAULT_MCP_PORT);
        assert_eq!(port_as_i32, 3000);

        // Test type compatibility - verify constants can be used in their expected contexts
        let default_limit: usize = DEFAULT_QUERY_LIMIT;
        let max_limit: usize = MAX_QUERY_LIMIT;
        let port_value: u16 = DEFAULT_MCP_PORT;

        // Verify the values are correctly assigned
        assert_eq!(default_limit, 100);
        assert_eq!(max_limit, 1000);
        assert_eq!(port_value, 3000);
    }
}
