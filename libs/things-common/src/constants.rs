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
