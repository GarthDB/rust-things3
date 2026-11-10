#![allow(deprecated)]
//! Health check and metrics methods for `SqliteThingsDatabase`.

use chrono::Utc;
use tracing::{debug, error, instrument};

use crate::{
    database::{
        pool::{ComprehensiveHealthStatus, PoolHealthStatus, PoolMetrics},
        stats::DatabaseStats,
        SqliteThingsDatabase,
    },
    error::{Result as ThingsResult, ThingsError},
};

impl SqliteThingsDatabase {
    /// Check if the database is connected
    #[instrument]
    pub async fn is_connected(&self) -> bool {
        match sqlx::query("SELECT 1").fetch_one(&self.pool).await {
            Ok(_) => {
                debug!("Database connection is healthy");
                true
            }
            Err(e) => {
                error!("Database connection check failed: {}", e);
                false
            }
        }
    }

    /// Get connection pool health status
    ///
    /// # Errors
    ///
    /// Returns an error if the health check fails
    #[instrument]
    pub async fn get_pool_health(&self) -> ThingsResult<PoolHealthStatus> {
        let pool_size = self.pool.size();
        let idle_connections = self.pool.num_idle();
        let active_connections = pool_size - u32::try_from(idle_connections).unwrap_or(0);

        let is_healthy = self.is_connected().await;

        Ok(PoolHealthStatus {
            is_healthy,
            pool_size,
            active_connections,
            idle_connections: u32::try_from(idle_connections).unwrap_or(0),
            max_connections: self.config.max_connections,
            min_connections: self.config.min_connections,
            connection_timeout: self.config.connect_timeout,
            idle_timeout: Some(self.config.idle_timeout),
            max_lifetime: Some(self.config.max_lifetime),
        })
    }

    /// Get detailed connection pool metrics
    ///
    /// # Errors
    ///
    /// Returns an error if the metrics collection fails
    #[instrument]
    pub async fn get_pool_metrics(&self) -> ThingsResult<PoolMetrics> {
        let pool_size = self.pool.size();
        let idle_connections = self.pool.num_idle();
        let active_connections = pool_size - u32::try_from(idle_connections).unwrap_or(0);

        let max_connections = self.config.max_connections;
        let utilization_percentage = if max_connections > 0 {
            (f64::from(active_connections) / f64::from(max_connections)) * 100.0
        } else {
            0.0
        };

        let start_time = std::time::Instant::now();
        let is_connected = self.is_connected().await;
        let response_time_ms = u64::try_from(start_time.elapsed().as_millis()).unwrap_or(0);

        Ok(PoolMetrics {
            pool_size,
            active_connections,
            idle_connections: u32::try_from(idle_connections).unwrap_or(0),
            max_connections,
            min_connections: self.config.min_connections,
            utilization_percentage,
            is_healthy: is_connected,
            response_time_ms,
            connection_timeout: self.config.connect_timeout,
            idle_timeout: Some(self.config.idle_timeout),
            max_lifetime: Some(self.config.max_lifetime),
        })
    }

    /// Perform a comprehensive health check including pool and database
    ///
    /// # Errors
    ///
    /// Returns an error if the health check fails
    #[instrument]
    pub async fn comprehensive_health_check(&self) -> ThingsResult<ComprehensiveHealthStatus> {
        let pool_health = self.get_pool_health().await?;
        let pool_metrics = self.get_pool_metrics().await?;
        let db_stats = self.get_stats().await?;

        let overall_healthy = pool_health.is_healthy && pool_metrics.is_healthy;

        Ok(ComprehensiveHealthStatus {
            overall_healthy,
            pool_health,
            pool_metrics,
            database_stats: db_stats,
            timestamp: Utc::now(),
        })
    }

    /// Get database statistics
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument]
    pub async fn get_stats(&self) -> ThingsResult<DatabaseStats> {
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM TMTask")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to get task count: {e}")))?;

        let project_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM TMTask WHERE type = 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to get project count: {e}")))?;

        let area_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM TMArea")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to get area count: {e}")))?;

        Ok(DatabaseStats {
            task_count: task_count.try_into().unwrap_or(0),
            project_count: project_count.try_into().unwrap_or(0),
            area_count: area_count.try_into().unwrap_or(0),
        })
    }
}
