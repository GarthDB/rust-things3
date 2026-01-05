//! Things Core - Core library for Things 3 database access and data models
//!
//! This library provides high-performance access to the Things 3 database,
//! with comprehensive data models and efficient querying capabilities.
//!
//! # Features
//!
//! - **Async Database Access**: Built on SQLx for type-safe, async database operations
//! - **Comprehensive Data Models**: Full support for Tasks, Projects, Areas, and Tags
//! - **Bulk Operations**: Efficient batch operations with transactional guarantees
//! - **Caching Layer**: High-performance caching with configurable TTL
//! - **Export Support**: Multiple export formats (JSON, CSV, OPML, Markdown)
//! - **Observability**: Built-in metrics, logging, and health checks
//! - **Performance Monitoring**: Query performance tracking and optimization suggestions
//!
//! # Quick Start
//!
//! ```no_run
//! use things3_core::{ThingsDatabase, ThingsError};
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), ThingsError> {
//! // Connect to Things 3 database
//! let db = ThingsDatabase::new(Path::new("/path/to/things.db")).await?;
//!
//! // Get inbox tasks
//! let tasks = db.get_inbox(None).await?;
//! println!("Found {} tasks in inbox", tasks.len());
//!
//! // Search for tasks
//! let results = db.search_tasks("meeting").await?;
//! println!("Found {} matching tasks", results.len());
//! # Ok(())
//! # }
//! ```
//!
//! # Examples
//!
//! See the [examples directory](https://github.com/GarthDB/rust-things3/tree/main/examples)
//! for more comprehensive usage examples.
//!
//! # Crate Features
//!
//! - `test-utils`: Enable test utilities (for testing only)

pub mod backup;
pub mod cache;
pub mod cache_invalidation_middleware;
pub mod config;
pub mod config_hot_reload;
pub mod config_loader;
pub mod database;
pub mod disk_cache;
pub mod error;
pub mod export;
pub mod mcp_cache_middleware;
pub mod mcp_config;
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
pub use config_hot_reload::{
    ConfigChangeHandler, ConfigHotReloader, ConfigHotReloaderWithHandler,
    DefaultConfigChangeHandler,
};
pub use config_loader::{load_config, load_config_from_env, load_config_with_paths, ConfigLoader};
pub use database::{
    get_default_database_path, ComprehensiveHealthStatus, DatabasePoolConfig, DatabaseStats,
    PoolHealthStatus, PoolMetrics, SqliteOptimizations, ThingsDatabase,
};
pub use disk_cache::{DiskCache, DiskCacheConfig, DiskCacheStats};
pub use error::{Result, ThingsError};
pub use export::{DataExporter, ExportConfig, ExportData, ExportFormat};
pub use mcp_cache_middleware::{MCPCacheConfig, MCPCacheEntry, MCPCacheMiddleware, MCPCacheStats};
pub use mcp_config::McpServerConfig;
pub use models::*;
// Explicitly re-export DeleteChildHandling for clarity
pub use models::DeleteChildHandling;
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
