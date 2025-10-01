//! Things Core - Core library for Things 3 database access and data models
//!
//! This library provides high-performance access to the Things 3 database,
//! with comprehensive data models and efficient querying capabilities.

pub mod backup;
pub mod cache;
pub mod cache_invalidation_middleware;
pub mod config;
pub mod database;
pub mod disk_cache;
pub mod error;
pub mod export;
pub mod mcp_cache_middleware;
pub mod models;
pub mod observability;
pub mod performance;
pub mod query;
pub mod query_cache;
pub mod query_performance;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub use backup::{BackupManager, BackupMetadata, BackupStats};
pub use cache::{CacheConfig, CacheStats, ThingsCache};
pub use cache_invalidation_middleware::{
    CacheInvalidationHandler, CacheInvalidationMiddleware, InvalidationConfig, InvalidationEvent,
    InvalidationEventType, InvalidationRule, InvalidationStats, InvalidationStrategy,
};
pub use config::ThingsConfig;
pub use database::{
    ComprehensiveHealthStatus, DatabasePoolConfig, DatabaseStats, PoolHealthStatus, PoolMetrics,
    SqliteOptimizations, ThingsDatabase,
};
pub use disk_cache::{DiskCache, DiskCacheConfig, DiskCacheStats};
pub use error::{Result, ThingsError};
pub use export::{DataExporter, ExportConfig, ExportData, ExportFormat};
pub use mcp_cache_middleware::{MCPCacheConfig, MCPCacheEntry, MCPCacheMiddleware, MCPCacheStats};
pub use models::*;
pub use observability::{
    CheckResult, HealthStatus, ObservabilityConfig, ObservabilityError, ObservabilityManager,
    ThingsMetrics,
};
pub use performance::{
    CacheMetrics, ComprehensivePerformanceSummary, OperationMetrics, PerformanceMonitor,
    PerformanceStats, PerformanceSummary, QueryMetrics,
};
pub use query_cache::{QueryCache, QueryCacheConfig, QueryCacheStats};
pub use query_performance::{
    ImplementationEffort, OptimizationPriority, OptimizationType, QueryContext,
    QueryOptimizationSuggestion, QueryPerformanceMetrics, QueryPerformanceStats,
    QueryPerformanceSummary, QueryPerformanceTracker,
};

/// Re-export commonly used types
pub use chrono::{DateTime, NaiveDate, Utc};
pub use serde::{Deserialize, Serialize};
pub use uuid::Uuid;
