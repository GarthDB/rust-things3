//! Things Core - Core library for Things 3 database access and data models
//!
//! This library provides high-performance access to the Things 3 database,
//! with comprehensive data models and efficient querying capabilities.

pub mod backup;
pub mod cache;
pub mod config;
pub mod database;
pub mod error;
pub mod export;
pub mod models;
pub mod observability;
pub mod performance;
pub mod query;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub use backup::{BackupManager, BackupMetadata, BackupStats};
pub use cache::{CacheConfig, CacheStats, ThingsCache};
pub use config::ThingsConfig;
pub use database::ThingsDatabase;
pub use error::{Result, ThingsError};
pub use export::{DataExporter, ExportConfig, ExportData, ExportFormat};
pub use models::*;
pub use observability::{
    CheckResult, HealthStatus, ObservabilityConfig, ObservabilityError, ObservabilityManager,
    ThingsMetrics,
};
pub use performance::{OperationMetrics, PerformanceMonitor, PerformanceStats, PerformanceSummary};

/// Re-export commonly used types
pub use chrono::{DateTime, NaiveDate, Utc};
pub use serde::{Deserialize, Serialize};
pub use uuid::Uuid;
