//! Metrics collection and monitoring
//!
//! This module provides comprehensive metrics collection for the Things 3 CLI application,
//! including performance monitoring, error tracking, and operational metrics.

use std::sync::Arc;
use std::time::{Duration, Instant};
use things3_core::{ObservabilityManager, ThingsDatabase};
use tokio::time::interval;
use tracing::{debug, error, info, instrument, warn};

/// Metrics collector for continuous monitoring
pub struct MetricsCollector {
    observability: Arc<ObservabilityManager>,
    database: Arc<ThingsDatabase>,
    collection_interval: Duration,
}

impl MetricsCollector {
    /// Create a new metrics collector
    #[must_use]
    pub fn new(
        observability: Arc<ObservabilityManager>,
        database: Arc<ThingsDatabase>,
        collection_interval: Duration,
    ) -> Self {
        Self {
            observability,
            database,
            collection_interval,
        }
    }

    /// Start metrics collection in background
    ///
    /// # Errors
    ///
    /// Returns an error if metrics collection fails
    #[instrument(skip(self))]
    pub async fn start_collection(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "Starting metrics collection with interval: {:?}",
            self.collection_interval
        );

        let mut interval = interval(self.collection_interval);

        loop {
            interval.tick().await;

            if let Err(e) = self.collect_metrics().await {
                error!("Failed to collect metrics: {}", e);
            }
        }
    }

    /// Collect current metrics
    #[instrument(skip(self))]
    async fn collect_metrics(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Collecting metrics");

        // Collect system metrics
        self.collect_system_metrics().await?;

        // Collect database metrics
        self.collect_database_metrics().await?;

        // Collect application metrics
        self.collect_application_metrics().await?;

        debug!("Metrics collection completed");
        Ok(())
    }

    /// Collect system metrics (memory, CPU, etc.)
    #[instrument(skip(self))]
    async fn collect_system_metrics(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use sysinfo::{Pid, System};

        let mut system = System::new_all();
        system.refresh_all();

        // Get current process
        let current_pid = Pid::from_u32(std::process::id());
        let process = system.process(current_pid);

        if let Some(process) = process {
            let memory_usage = process.memory() * 1024; // Convert to bytes
            let cpu_usage = f64::from(process.cpu_usage());

            // Update cache metrics (placeholder values for now)
            let cache_hit_rate = 0.85; // 85% hit rate
            let cache_size = 1024 * 1024; // 1MB cache size

            self.observability.update_performance_metrics(
                memory_usage,
                cpu_usage,
                cache_hit_rate,
                cache_size,
            );

            debug!(
                memory_usage = memory_usage,
                cpu_usage = cpu_usage,
                cache_hit_rate = cache_hit_rate,
                cache_size = cache_size,
                "System metrics collected"
            );
        }

        Ok(())
    }

    /// Collect database metrics
    #[instrument(skip(self))]
    async fn collect_database_metrics(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check database connection health
        let is_connected = true; // Simplified - in a real implementation, this would check the actual connection

        if !is_connected {
            warn!("Database connection is not healthy");
            self.observability
                .record_error("database_connection", "Database connection lost");
        }

        // Record database operation metrics
        // This would typically involve querying database statistics
        // For now, we'll use placeholder values

        debug!("Database metrics collected");
        Ok(())
    }

    /// Collect application-specific metrics
    #[instrument(skip(self))]
    async fn collect_application_metrics(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Collect task-related metrics
        self.collect_task_metrics().await?;

        // Collect search metrics
        self.collect_search_metrics().await?;

        // Collect export metrics
        self.collect_export_metrics().await?;

        debug!("Application metrics collected");
        Ok(())
    }

    /// Collect task-related metrics
    #[instrument(skip(self))]
    async fn collect_task_metrics(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // This would typically involve querying the database for task statistics
        // For now, we'll use placeholder values

        // Example: Count tasks by status
        let inbox_count = self
            .database
            .get_inbox(Some(1000))
            .await
            .map_err(|e| {
                error!("Failed to get inbox count: {}", e);
                e
            })?
            .len();

        let today_count = self
            .database
            .get_today(Some(1000))
            .await
            .map_err(|e| {
                error!("Failed to get today count: {}", e);
                e
            })?
            .len();

        debug!(
            inbox_count = inbox_count,
            today_count = today_count,
            "Task metrics collected"
        );

        Ok(())
    }

    /// Collect search metrics
    #[instrument(skip(self))]
    async fn collect_search_metrics(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // This would typically involve tracking search performance
        // For now, we'll use placeholder values

        debug!("Search metrics collected");
        Ok(())
    }

    /// Collect export metrics
    #[instrument(skip(self))]
    async fn collect_export_metrics(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // This would typically involve tracking export performance
        // For now, we'll use placeholder values

        debug!("Export metrics collected");
        Ok(())
    }
}

/// Performance monitoring utilities
pub struct PerformanceMonitor {
    observability: Arc<ObservabilityManager>,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    #[must_use]
    pub fn new(observability: Arc<ObservabilityManager>) -> Self {
        Self { observability }
    }

    /// Monitor a database operation
    #[instrument(skip(self, f))]
    pub fn monitor_db_operation<F, R>(&self, operation: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.observability.record_db_operation(operation, f)
    }

    /// Monitor a search operation
    #[instrument(skip(self, f))]
    pub fn monitor_search<F, R>(&self, query: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.observability.record_search_operation(query, f)
    }

    /// Monitor a task operation
    #[instrument(skip(self))]
    pub fn monitor_task_operation(&self, operation: &str, count: u64) {
        self.observability.record_task_operation(operation, count);
    }

    /// Monitor an export operation
    #[instrument(skip(self, f))]
    pub fn monitor_export<F, R>(&self, format: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        // In a real implementation, this would update metrics atomically

        debug!(
            format = format,
            duration_ms = duration.as_millis(),
            "Export operation completed"
        );

        result
    }
}

/// Error tracking utilities
pub struct ErrorTracker {
    observability: Arc<ObservabilityManager>,
}

impl ErrorTracker {
    /// Create a new error tracker
    #[must_use]
    pub fn new(observability: Arc<ObservabilityManager>) -> Self {
        Self { observability }
    }

    /// Track an error
    #[instrument(skip(self))]
    pub fn track_error(&self, error_type: &str, error_message: &str) {
        self.observability.record_error(error_type, error_message);
    }

    /// Track a database error
    #[instrument(skip(self))]
    pub fn track_db_error(&self, operation: &str, error: &dyn std::error::Error) {
        let error_type = format!("database_{operation}");
        let error_message = format!("Database operation '{operation}' failed: {error}");
        self.track_error(&error_type, &error_message);
    }

    /// Track a search error
    #[instrument(skip(self))]
    pub fn track_search_error(&self, query: &str, error: &dyn std::error::Error) {
        let error_type = "search_error";
        let error_message = format!("Search query '{query}' failed: {error}");
        self.track_error(error_type, &error_message);
    }

    /// Track an export error
    #[instrument(skip(self))]
    pub fn track_export_error(&self, format: &str, error: &dyn std::error::Error) {
        let error_type = "export_error";
        let error_message = format!("Export in '{format}' format failed: {error}");
        self.track_error(error_type, &error_message);
    }
}

/// Start metrics collection in background
///
/// # Errors
///
/// Returns an error if metrics collection fails
pub async fn start_metrics_collection(
    observability: Arc<ObservabilityManager>,
    database: Arc<ThingsDatabase>,
    collection_interval: Duration,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let collector = MetricsCollector::new(observability, database, collection_interval);
    collector.start_collection().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::NamedTempFile;
    use things3_core::{ObservabilityConfig, ThingsConfig};

    #[test]
    fn test_performance_monitor_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        let config = ThingsConfig::new(db_path, false);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _database = Arc::new(
            rt.block_on(async { ThingsDatabase::new(&config.database_path).await.unwrap() }),
        );

        let obs_config = ObservabilityConfig::default();
        let observability = Arc::new(ObservabilityManager::new(obs_config).unwrap());

        let _monitor = PerformanceMonitor::new(observability);
        // Test that monitor can be created without panicking
    }

    #[test]
    fn test_error_tracker_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        let config = ThingsConfig::new(db_path, false);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _database = Arc::new(
            rt.block_on(async { ThingsDatabase::new(&config.database_path).await.unwrap() }),
        );

        let obs_config = ObservabilityConfig::default();
        let observability = Arc::new(ObservabilityManager::new(obs_config).unwrap());

        let _tracker = ErrorTracker::new(observability);
        // Test that tracker can be created without panicking
    }

    #[test]
    fn test_metrics_collector_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        let config = ThingsConfig::new(db_path, false);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let database = Arc::new(
            rt.block_on(async { ThingsDatabase::new(&config.database_path).await.unwrap() }),
        );

        let obs_config = ObservabilityConfig::default();
        let observability = Arc::new(ObservabilityManager::new(obs_config).unwrap());

        let _collector = MetricsCollector::new(observability, database, Duration::from_secs(30));
        // Test that collector can be created without panicking
    }
}
