//! Things Core - Core library for Things 3 database access and data models
//!
//! This library provides high-performance access to the Things 3 database,
//! with comprehensive data models and efficient querying capabilities.

pub mod database;
pub mod models;
pub mod error;
pub mod query;

pub use database::ThingsDatabase;
pub use error::{ThingsError, Result};
pub use models::*;

/// Re-export commonly used types
pub use chrono::{DateTime, Utc, NaiveDate};
pub use serde::{Deserialize, Serialize};
pub use uuid::Uuid;
