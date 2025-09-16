//! Things Core - Core library for Things 3 database access and data models
//!
//! This library provides high-performance access to the Things 3 database,
//! with comprehensive data models and efficient querying capabilities.

pub mod config;
pub mod database;
pub mod error;
pub mod models;
pub mod query;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub use config::ThingsConfig;
pub use database::ThingsDatabase;
pub use error::{Result, ThingsError};
pub use models::*;

/// Re-export commonly used types
pub use chrono::{DateTime, NaiveDate, Utc};
pub use serde::{Deserialize, Serialize};
pub use uuid::Uuid;
