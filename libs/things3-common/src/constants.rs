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
}
