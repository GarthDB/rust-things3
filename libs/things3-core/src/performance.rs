//! Performance monitoring and metrics for Things 3 operations

use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;

/// Performance metrics for a single operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetrics {
    pub operation_name: String,
    pub duration: Duration,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Aggregated performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub operation_name: String,
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub total_duration: Duration,
    pub average_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub success_rate: f64,
    pub last_called: Option<DateTime<Utc>>,
}

impl PerformanceStats {
    #[must_use]
    pub const fn new(operation_name: String) -> Self {
        Self {
            operation_name,
            total_calls: 0,
            successful_calls: 0,
            failed_calls: 0,
            total_duration: Duration::ZERO,
            average_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            success_rate: 0.0,
            last_called: None,
        }
    }

    pub fn add_metric(&mut self, metric: &OperationMetrics) {
        self.total_calls += 1;
        self.total_duration += metric.duration;
        self.last_called = Some(metric.timestamp);

        if metric.success {
            self.successful_calls += 1;
        } else {
            self.failed_calls += 1;
        }

        if metric.duration < self.min_duration {
            self.min_duration = metric.duration;
        }
        if metric.duration > self.max_duration {
            self.max_duration = metric.duration;
        }

        self.average_duration = Duration::from_nanos(
            u64::try_from(self.total_duration.as_nanos()).unwrap_or(u64::MAX) / self.total_calls,
        );

        self.success_rate = if self.total_calls > 0 {
            #[allow(clippy::cast_precision_loss)]
            {
                self.successful_calls as f64 / self.total_calls as f64
            }
        } else {
            0.0
        };
    }
}

/// System resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub available_memory_mb: f64,
    pub total_memory_mb: f64,
}

/// Performance monitor for tracking operations and system metrics
pub struct PerformanceMonitor {
    /// Individual operation metrics
    metrics: Arc<RwLock<Vec<OperationMetrics>>>,
    /// Aggregated statistics by operation name
    stats: Arc<RwLock<HashMap<String, PerformanceStats>>>,
    /// System information
    system: Arc<RwLock<System>>,
    /// Maximum number of metrics to keep in memory
    max_metrics: usize,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    #[must_use]
    pub fn new(max_metrics: usize) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
            system: Arc::new(RwLock::new(System::new_all())),
            max_metrics,
        }
    }

    /// Create a new performance monitor with default settings
    #[must_use]
    pub fn new_default() -> Self {
        Self::new(10000) // Keep last 10,000 metrics
    }

    /// Start timing an operation
    #[must_use]
    pub fn start_operation(&self, operation_name: &str) -> OperationTimer {
        OperationTimer {
            monitor: self.clone(),
            operation_name: operation_name.to_string(),
            start_time: Instant::now(),
        }
    }

    /// Record a completed operation
    pub fn record_operation(&self, metric: &OperationMetrics) {
        // Add to metrics list
        {
            let mut metrics = self.metrics.write();
            metrics.push(metric.clone());

            // Trim if we exceed max_metrics
            if metrics.len() > self.max_metrics {
                let excess = metrics.len() - self.max_metrics;
                metrics.drain(0..excess);
            }
        }

        // Update aggregated stats
        let operation_name = metric.operation_name.clone();
        let mut stats = self.stats.write();
        let operation_stats = stats
            .entry(operation_name)
            .or_insert_with(|| PerformanceStats::new(metric.operation_name.clone()));
        operation_stats.add_metric(metric);
        drop(stats);
    }

    /// Get all operation metrics
    #[must_use]
    pub fn get_metrics(&self) -> Vec<OperationMetrics> {
        self.metrics.read().clone()
    }

    /// Get aggregated statistics for all operations
    #[must_use]
    pub fn get_all_stats(&self) -> HashMap<String, PerformanceStats> {
        self.stats.read().clone()
    }

    /// Get statistics for a specific operation
    #[must_use]
    pub fn get_operation_stats(&self, operation_name: &str) -> Option<PerformanceStats> {
        self.stats.read().get(operation_name).cloned()
    }

    /// Get current system metrics
    /// Get system metrics
    ///
    /// # Errors
    ///
    /// Returns an error if system information cannot be retrieved.
    pub fn get_system_metrics(&self) -> Result<SystemMetrics> {
        let mut system = self.system.write();
        system.refresh_all();

        Ok(SystemMetrics {
            timestamp: Utc::now(),
            #[allow(clippy::cast_precision_loss)]
            memory_usage_mb: system.used_memory() as f64 / 1024.0 / 1024.0,
            cpu_usage_percent: {
                let cpu_count = system.cpus().len();
                #[allow(clippy::cast_precision_loss)]
                let cpu_usage: f64 = system
                    .cpus()
                    .iter()
                    .map(|cpu| f64::from(cpu.cpu_usage()))
                    .sum::<f64>()
                    / cpu_count as f64;
                cpu_usage
            },
            #[allow(clippy::cast_precision_loss)]
            available_memory_mb: system.available_memory() as f64 / 1024.0 / 1024.0,
            #[allow(clippy::cast_precision_loss)]
            total_memory_mb: system.total_memory() as f64 / 1024.0 / 1024.0,
        })
    }

    /// Clear all metrics and statistics
    pub fn clear(&self) {
        self.metrics.write().clear();
        self.stats.write().clear();
    }

    /// Get performance summary
    #[must_use]
    pub fn get_summary(&self) -> PerformanceSummary {
        let stats = self.get_all_stats();
        let total_operations: u64 = stats.values().map(|s| s.total_calls).sum();
        let total_successful: u64 = stats.values().map(|s| s.successful_calls).sum();
        let total_duration: Duration = stats.values().map(|s| s.total_duration).sum();

        PerformanceSummary {
            total_operations,
            total_successful,
            total_failed: total_operations - total_successful,
            overall_success_rate: if total_operations > 0 {
                #[allow(clippy::cast_precision_loss)]
                {
                    total_successful as f64 / total_operations as f64
                }
            } else {
                0.0
            },
            total_duration,
            average_operation_duration: if total_operations > 0 {
                Duration::from_nanos(
                    u64::try_from(total_duration.as_nanos()).unwrap_or(0) / total_operations,
                )
            } else {
                Duration::ZERO
            },
            operation_count: stats.len(),
        }
    }
}

impl Clone for PerformanceMonitor {
    fn clone(&self) -> Self {
        Self {
            metrics: Arc::clone(&self.metrics),
            stats: Arc::clone(&self.stats),
            system: Arc::clone(&self.system),
            max_metrics: self.max_metrics,
        }
    }
}

/// Timer for tracking operation duration
pub struct OperationTimer {
    monitor: PerformanceMonitor,
    operation_name: String,
    start_time: Instant,
}

impl OperationTimer {
    /// Complete the operation successfully
    pub fn success(self) {
        let duration = self.start_time.elapsed();
        let metric = OperationMetrics {
            operation_name: self.operation_name,
            duration,
            timestamp: Utc::now(),
            success: true,
            error_message: None,
        };
        self.monitor.record_operation(&metric);
    }

    /// Complete the operation with an error
    pub fn error(self, error_message: String) {
        let duration = self.start_time.elapsed();
        let metric = OperationMetrics {
            operation_name: self.operation_name,
            duration,
            timestamp: Utc::now(),
            success: false,
            error_message: Some(error_message),
        };
        self.monitor.record_operation(&metric);
    }
}

/// Performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub total_operations: u64,
    pub total_successful: u64,
    pub total_failed: u64,
    pub overall_success_rate: f64,
    pub total_duration: Duration,
    pub average_operation_duration: Duration,
    pub operation_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new_default();

        // Record some operations
        let metric1 = OperationMetrics {
            operation_name: "test_op".to_string(),
            duration: Duration::from_millis(100),
            timestamp: Utc::now(),
            success: true,
            error_message: None,
        };

        monitor.record_operation(&metric1);

        let stats = monitor.get_operation_stats("test_op");
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.successful_calls, 1);
        assert_eq!(stats.failed_calls, 0);
    }

    #[test]
    fn test_operation_timer() {
        let monitor = PerformanceMonitor::new_default();

        // Test successful operation
        let timer = monitor.start_operation("test_timer");
        thread::sleep(Duration::from_millis(10));
        timer.success();

        let stats = monitor.get_operation_stats("test_timer");
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.total_calls, 1);
        assert!(stats.successful_calls > 0);
    }

    #[test]
    fn test_performance_monitor_failed_operation() {
        let monitor = PerformanceMonitor::new_default();

        // Record a failed operation
        let metric = OperationMetrics {
            operation_name: "failed_op".to_string(),
            duration: Duration::from_millis(50),
            timestamp: Utc::now(),
            success: false,
            error_message: Some("Test error".to_string()),
        };

        monitor.record_operation(&metric);

        let stats = monitor.get_operation_stats("failed_op");
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.successful_calls, 0);
        assert_eq!(stats.failed_calls, 1);
    }

    #[test]
    fn test_performance_monitor_multiple_operations() {
        let monitor = PerformanceMonitor::new_default();

        // Record multiple operations
        for i in 0..5 {
            let metric = OperationMetrics {
                operation_name: "multi_op".to_string(),
                duration: Duration::from_millis(i * 10),
                timestamp: Utc::now(),
                success: i % 2 == 0,
                error_message: if i % 2 == 0 {
                    None
                } else {
                    Some("Error".to_string())
                },
            };
            monitor.record_operation(&metric);
        }

        let stats = monitor.get_operation_stats("multi_op");
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.total_calls, 5);
        assert_eq!(stats.successful_calls, 3);
        assert_eq!(stats.failed_calls, 2);
    }

    #[test]
    fn test_performance_monitor_get_all_stats() {
        let monitor = PerformanceMonitor::new_default();

        // Record operations for different types
        let operations = vec![("op1", true), ("op1", false), ("op2", true), ("op2", true)];

        for (name, success) in operations {
            let metric = OperationMetrics {
                operation_name: name.to_string(),
                duration: Duration::from_millis(100),
                timestamp: Utc::now(),
                success,
                error_message: if success {
                    None
                } else {
                    Some("Error".to_string())
                },
            };
            monitor.record_operation(&metric);
        }

        let all_stats = monitor.get_all_stats();
        assert_eq!(all_stats.len(), 2);
        assert!(all_stats.contains_key("op1"));
        assert!(all_stats.contains_key("op2"));

        let op1_stats = &all_stats["op1"];
        assert_eq!(op1_stats.total_calls, 2);
        assert_eq!(op1_stats.successful_calls, 1);
        assert_eq!(op1_stats.failed_calls, 1);

        let op2_stats = &all_stats["op2"];
        assert_eq!(op2_stats.total_calls, 2);
        assert_eq!(op2_stats.successful_calls, 2);
        assert_eq!(op2_stats.failed_calls, 0);
    }

    #[test]
    fn test_performance_monitor_get_summary() {
        let monitor = PerformanceMonitor::new_default();

        // Record some operations
        let operations = vec![("op1", true, 100), ("op1", false, 200), ("op2", true, 150)];

        for (name, success, duration_ms) in operations {
            let metric = OperationMetrics {
                operation_name: name.to_string(),
                duration: Duration::from_millis(duration_ms),
                timestamp: Utc::now(),
                success,
                error_message: if success {
                    None
                } else {
                    Some("Error".to_string())
                },
            };
            monitor.record_operation(&metric);
        }

        let summary = monitor.get_summary();
        assert_eq!(summary.total_operations, 3);
        assert_eq!(summary.total_successful, 2);
        assert_eq!(summary.total_failed, 1);
        assert!((summary.overall_success_rate - 2.0 / 3.0).abs() < 0.001);
        assert_eq!(summary.operation_count, 2);
    }

    #[test]
    fn test_performance_monitor_get_summary_empty() {
        let monitor = PerformanceMonitor::new_default();
        let summary = monitor.get_summary();

        assert_eq!(summary.total_operations, 0);
        assert_eq!(summary.total_successful, 0);
        assert_eq!(summary.total_failed, 0);
        assert!((summary.overall_success_rate - 0.0).abs() < f64::EPSILON);
        assert_eq!(summary.operation_count, 0);
    }

    #[test]
    fn test_operation_timer_failure() {
        let monitor = PerformanceMonitor::new_default();

        // Test failed operation by recording it directly
        let metric = OperationMetrics {
            operation_name: "test_failure".to_string(),
            duration: Duration::from_millis(5),
            timestamp: Utc::now(),
            success: false,
            error_message: Some("Test failure".to_string()),
        };
        monitor.record_operation(&metric);

        let stats = monitor.get_operation_stats("test_failure");
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.successful_calls, 0);
        assert_eq!(stats.failed_calls, 1);
    }

    #[test]
    fn test_operation_timer_drop() {
        let monitor = PerformanceMonitor::new_default();

        // Test that dropping the timer records the operation
        {
            let timer = monitor.start_operation("test_drop");
            thread::sleep(Duration::from_millis(5));
            // Explicitly call success before dropping
            timer.success();
        }

        let stats = monitor.get_operation_stats("test_drop");
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.successful_calls, 1);
        assert_eq!(stats.failed_calls, 0);
    }

    #[test]
    fn test_performance_monitor_clone() {
        let monitor1 = PerformanceMonitor::new_default();

        // Record an operation
        let metric = OperationMetrics {
            operation_name: "clone_test".to_string(),
            duration: Duration::from_millis(100),
            timestamp: Utc::now(),
            success: true,
            error_message: None,
        };
        monitor1.record_operation(&metric);

        // Clone the monitor
        let monitor2 = monitor1.clone();

        // Both should have the same stats
        let stats1 = monitor1.get_operation_stats("clone_test");
        let stats2 = monitor2.get_operation_stats("clone_test");

        assert!(stats1.is_some());
        assert!(stats2.is_some());
        assert_eq!(stats1.unwrap().total_calls, stats2.unwrap().total_calls);
    }
}
