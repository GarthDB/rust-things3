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
}
