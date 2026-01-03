//! Database module - organized submodules for better maintainability

mod core;
pub mod date_utils;
pub mod mappers;
pub mod query_builders;
pub mod tag_utils;
pub mod validators;

// Re-export everything from core for backward compatibility
pub use core::*;

// Re-export mapper functions for easy access
pub use mappers::{map_task_row, parse_optional_uuid, parse_uuid_with_fallback};

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
