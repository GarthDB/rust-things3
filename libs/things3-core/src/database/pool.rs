//! Connection pool configuration, optimizations, and health/metrics types.

use crate::database::stats::DatabaseStats;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Database connection pool configuration for optimal performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabasePoolConfig {
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections in the pool
    pub min_connections: u32,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Idle timeout for connections
    pub idle_timeout: Duration,
    /// Maximum lifetime of a connection
    pub max_lifetime: Duration,
    /// Test connections before use
    pub test_before_acquire: bool,
    /// SQLite-specific optimizations
    pub sqlite_optimizations: SqliteOptimizations,
}

/// SQLite-specific optimization settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteOptimizations {
    /// Enable WAL mode for better concurrency
    pub enable_wal_mode: bool,
    /// Set synchronous mode (NORMAL, FULL, OFF)
    pub synchronous_mode: String,
    /// Cache size in pages (negative = KB)
    pub cache_size: i32,
    /// Enable foreign key constraints
    pub enable_foreign_keys: bool,
    /// Set journal mode
    pub journal_mode: String,
    /// Set temp store (MEMORY, FILE, DEFAULT)
    pub temp_store: String,
    /// Set mmap size for better performance
    pub mmap_size: i64,
    /// Enable query planner optimizations
    pub enable_query_planner: bool,
}

impl Default for DatabasePoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600), // 10 minutes
            max_lifetime: Duration::from_secs(1800), // 30 minutes
            test_before_acquire: true,
            sqlite_optimizations: SqliteOptimizations::default(),
        }
    }
}

impl Default for SqliteOptimizations {
    fn default() -> Self {
        Self {
            enable_wal_mode: true,
            synchronous_mode: "NORMAL".to_string(),
            cache_size: -20000, // 20MB cache
            enable_foreign_keys: true,
            journal_mode: "WAL".to_string(),
            temp_store: "MEMORY".to_string(),
            mmap_size: 268_435_456, // 256MB
            enable_query_planner: true,
        }
    }
}

/// Connection pool health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolHealthStatus {
    pub is_healthy: bool,
    pub pool_size: u32,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
}

/// Detailed connection pool metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetrics {
    pub pool_size: u32,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub max_connections: u32,
    pub min_connections: u32,
    pub utilization_percentage: f64,
    pub is_healthy: bool,
    pub response_time_ms: u64,
    pub connection_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
}

/// Comprehensive health status including pool and database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveHealthStatus {
    pub overall_healthy: bool,
    pub pool_health: PoolHealthStatus,
    pub pool_metrics: PoolMetrics,
    pub database_stats: DatabaseStats,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_pool_config_default() {
        let config = DatabasePoolConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Duration::from_secs(600));
        assert_eq!(config.max_lifetime, Duration::from_secs(1800));
        assert!(config.test_before_acquire);
    }

    #[test]
    fn test_sqlite_optimizations_default() {
        let opts = SqliteOptimizations::default();
        assert!(opts.enable_wal_mode);
        assert_eq!(opts.cache_size, -20000);
        assert_eq!(opts.synchronous_mode, "NORMAL".to_string());
        assert_eq!(opts.temp_store, "MEMORY".to_string());
        assert_eq!(opts.journal_mode, "WAL".to_string());
        assert_eq!(opts.mmap_size, 268_435_456);
        assert!(opts.enable_foreign_keys);
        assert!(opts.enable_query_planner);
    }

    #[test]
    fn test_pool_health_status_creation() {
        let status = PoolHealthStatus {
            is_healthy: true,
            pool_size: 8,
            active_connections: 5,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };
        assert!(status.is_healthy);
        assert_eq!(status.active_connections, 5);
        assert_eq!(status.idle_connections, 3);
        assert_eq!(status.pool_size, 8);
    }

    #[test]
    fn test_pool_metrics_creation() {
        let metrics = PoolMetrics {
            pool_size: 8,
            active_connections: 5,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            utilization_percentage: 80.0,
            is_healthy: true,
            response_time_ms: 50,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };
        assert!(metrics.is_healthy);
        assert_eq!(metrics.pool_size, 8);
        assert_eq!(metrics.active_connections, 5);
        assert_eq!(metrics.idle_connections, 3);
        assert!((metrics.utilization_percentage - 80.0).abs() < f64::EPSILON);
        assert_eq!(metrics.response_time_ms, 50);
    }

    #[test]
    fn test_comprehensive_health_status_creation() {
        let pool_health = PoolHealthStatus {
            is_healthy: true,
            pool_size: 8,
            active_connections: 5,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };

        let pool_metrics = PoolMetrics {
            pool_size: 8,
            active_connections: 5,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            utilization_percentage: 80.0,
            is_healthy: true,
            response_time_ms: 50,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };

        let db_stats = DatabaseStats {
            task_count: 50,
            project_count: 10,
            area_count: 5,
        };

        let health_status = ComprehensiveHealthStatus {
            overall_healthy: true,
            pool_health,
            pool_metrics,
            database_stats: db_stats,
            timestamp: Utc::now(),
        };

        assert!(health_status.overall_healthy);
        assert_eq!(health_status.database_stats.total_items(), 65);
    }

    #[test]
    fn test_database_pool_config_default_values() {
        let config = DatabasePoolConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Duration::from_secs(600));
        assert_eq!(config.max_lifetime, Duration::from_secs(1800));
        assert!(config.test_before_acquire);
    }

    #[test]
    fn test_pool_health_status_creation_comprehensive() {
        let status = PoolHealthStatus {
            is_healthy: true,
            pool_size: 8,
            active_connections: 2,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };
        assert!(status.is_healthy);
        assert_eq!(status.pool_size, 8);
        assert_eq!(status.max_connections, 10);
    }

    #[test]
    fn test_pool_metrics_creation_comprehensive() {
        let metrics = PoolMetrics {
            pool_size: 8,
            active_connections: 5,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            utilization_percentage: 80.0,
            is_healthy: true,
            response_time_ms: 50,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };
        assert_eq!(metrics.pool_size, 8);
        assert_eq!(metrics.response_time_ms, 50);
        assert!(metrics.is_healthy);
    }

    #[test]
    fn test_comprehensive_health_status_creation_full() {
        let pool_health = PoolHealthStatus {
            is_healthy: true,
            pool_size: 8,
            active_connections: 2,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };

        let pool_metrics = PoolMetrics {
            pool_size: 8,
            active_connections: 5,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            utilization_percentage: 80.0,
            is_healthy: true,
            response_time_ms: 50,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };

        let database_stats = DatabaseStats {
            task_count: 100,
            project_count: 20,
            area_count: 5,
        };

        let status = ComprehensiveHealthStatus {
            overall_healthy: true,
            pool_health,
            pool_metrics,
            database_stats,
            timestamp: Utc::now(),
        };

        assert!(status.overall_healthy);
        assert_eq!(status.database_stats.total_items(), 125);
    }

    #[test]
    fn test_sqlite_optimizations_default_values() {
        let opts = SqliteOptimizations::default();
        assert!(opts.enable_wal_mode);
        assert!(opts.enable_foreign_keys);
        assert_eq!(opts.cache_size, -20000);
        assert_eq!(opts.temp_store, "MEMORY");
        assert_eq!(opts.mmap_size, 268_435_456);
        assert_eq!(opts.synchronous_mode, "NORMAL");
        assert_eq!(opts.journal_mode, "WAL");
    }
}
