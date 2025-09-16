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
    pub fn new(operation_name: String) -> Self {
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

        self.average_duration =
            Duration::from_nanos(self.total_duration.as_nanos() as u64 / self.total_calls);

        self.success_rate = if self.total_calls > 0 {
            self.successful_calls as f64 / self.total_calls as f64
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
    pub fn new(max_metrics: usize) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
            system: Arc::new(RwLock::new(System::new_all())),
            max_metrics,
        }
    }

    /// Create a new performance monitor with default settings
    pub fn new_default() -> Self {
        Self::new(10000) // Keep last 10,000 metrics
    }

    /// Start timing an operation
    pub fn start_operation(&self, operation_name: &str) -> OperationTimer {
        OperationTimer {
            monitor: self.clone(),
            operation_name: operation_name.to_string(),
            start_time: Instant::now(),
        }
    }

    /// Record a completed operation
    pub fn record_operation(&self, metric: OperationMetrics) {
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
        {
            let mut stats = self.stats.write();
            let operation_stats = stats
                .entry(metric.operation_name.clone())
                .or_insert_with(|| PerformanceStats::new(metric.operation_name.clone()));
            operation_stats.add_metric(&metric);
        }
    }

    /// Get all operation metrics
    pub fn get_metrics(&self) -> Vec<OperationMetrics> {
        self.metrics.read().clone()
    }

    /// Get aggregated statistics for all operations
    pub fn get_all_stats(&self) -> HashMap<String, PerformanceStats> {
        self.stats.read().clone()
    }

    /// Get statistics for a specific operation
    pub fn get_operation_stats(&self, operation_name: &str) -> Option<PerformanceStats> {
        self.stats.read().get(operation_name).cloned()
    }

    /// Get current system metrics
    pub fn get_system_metrics(&self) -> Result<SystemMetrics> {
        let mut system = self.system.write();
        system.refresh_all();

        Ok(SystemMetrics {
            timestamp: Utc::now(),
            memory_usage_mb: system.used_memory() as f64 / 1024.0 / 1024.0,
            cpu_usage_percent: system
                .cpus()
                .iter()
                .map(|cpu| cpu.cpu_usage() as f64)
                .sum::<f64>()
                / system.cpus().len() as f64,
            available_memory_mb: system.available_memory() as f64 / 1024.0 / 1024.0,
            total_memory_mb: system.total_memory() as f64 / 1024.0 / 1024.0,
        })
    }

    /// Clear all metrics and statistics
    pub fn clear(&self) {
        self.metrics.write().clear();
        self.stats.write().clear();
    }

    /// Get performance summary
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
                total_successful as f64 / total_operations as f64
            } else {
                0.0
            },
            total_duration,
            average_operation_duration: if total_operations > 0 {
                Duration::from_nanos(total_duration.as_nanos() as u64 / total_operations)
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
        self.monitor.record_operation(metric);
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
        self.monitor.record_operation(metric);
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

        monitor.record_operation(metric1);

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
}
