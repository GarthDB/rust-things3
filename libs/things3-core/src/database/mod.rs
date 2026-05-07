//! Database module - organized submodules for better maintainability

pub mod conversions;
mod core;
pub mod date_utils;
pub mod mappers;
pub mod path_discovery;
pub mod pool;
pub mod query_builders;
pub mod stats;
pub mod tag_utils;
pub mod validators;

// Re-export everything from core for backward compatibility
pub use core::*;

// Re-export conversions
pub use conversions::{
    deserialize_tags_from_blob, naive_date_to_things_timestamp, serialize_tags_to_blob,
};
// Crate-internal helpers used by sibling submodules (mappers.rs, core.rs).
pub(crate) use conversions::{safe_timestamp_convert, things_date_to_naive_date};

// Re-export path discovery
pub use path_discovery::get_default_database_path;

// Re-export pool/health types
pub use pool::{
    ComprehensiveHealthStatus, DatabasePoolConfig, PoolHealthStatus, PoolMetrics,
    SqliteOptimizations,
};

// Re-export stats
pub use stats::DatabaseStats;

// Re-export mapper functions for easy access
pub use mappers::{map_project_row, map_task_row};

// Re-export query builders
pub use query_builders::TaskUpdateBuilder;

// Re-export validators
pub use validators::{validate_area_exists, validate_project_exists, validate_task_exists};

// Re-export date utilities
pub use date_utils::{
    add_days, format_date_for_display, is_date_in_future, is_date_in_past,
    is_valid_things_timestamp, parse_date_from_string, safe_naive_date_to_things_timestamp,
    safe_things_date_to_naive_date, validate_date_range, DateConversionError, DateValidationError,
};
