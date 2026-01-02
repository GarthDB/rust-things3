//! Database module - organized submodules for better maintainability

mod core;
pub mod mappers;

// Re-export everything from core for backward compatibility
pub use core::*;

// Re-export mapper functions for easy access
pub use mappers::{map_task_row, parse_optional_uuid, parse_uuid_with_fallback};
