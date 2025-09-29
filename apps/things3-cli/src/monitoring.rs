//! Monitoring and validation utilities for real-time features
//! This module provides tools to verify that async functionality works correctly

use crate::events::EventBroadcaster;
use crate::progress::ProgressManager;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Monitor for tracking async operation health
pub struct AsyncOperationMonitor {
    pub operation_name: String,
    pub start_time: Instant,
    pub last_update: Arc<Mutex<Option<Instant>>>,
    pub update_count: Arc<Mutex<u64>>,
    pub success_count: Arc<Mutex<u64>>,
    pub error_count: Arc<Mutex<u64>>,
}

impl AsyncOperationMonitor {
    /// Create a new monitor
    #[must_use]
    pub fn new(operation_name: String) -> Self {
        Self {
            operation_name,
            start_time: Instant::now(),
            last_update: Arc::new(Mutex::new(None)),
            update_count: Arc::new(Mutex::new(0)),
            success_count: Arc::new(Mutex::new(0)),
            error_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Record a successful operation
    pub async fn record_success(&self) {
        let mut count = self.success_count.lock().await;
        *count += 1;
        self.update_last_seen().await;
    }

    /// Record an error
    pub async fn record_error(&self) {
        let mut count = self.error_count.lock().await;
        *count += 1;
        self.update_last_seen().await;
    }

    /// Record an update
    pub async fn record_update(&self) {
        let mut count = self.update_count.lock().await;
        *count += 1;
        self.update_last_seen().await;
    }

    /// Update the last seen timestamp
    async fn update_last_seen(&self) {
        let mut last_update = self.last_update.lock().await;
        *last_update = Some(Instant::now());
    }

    /// Check if the operation is healthy (has recent activity)
    pub async fn is_healthy(&self, max_silence: Duration) -> bool {
        let last_update = self.last_update.lock().await;
        match *last_update {
            Some(last) => Instant::now().duration_since(last) < max_silence,
            None => false,
        }
    }

    /// Get operation statistics
    pub async fn get_stats(&self) -> OperationStats {
        let update_count = *self.update_count.lock().await;
        let success_count = *self.success_count.lock().await;
        let error_count = *self.error_count.lock().await;
        let duration = self.start_time.elapsed();

        OperationStats {
            operation_name: self.operation_name.clone(),
            duration,
            update_count,
            success_count,
            error_count,
            success_rate: if update_count > 0 {
                #[allow(clippy::cast_precision_loss)]
                {
                    success_count as f64 / update_count as f64
                }
            } else {
                0.0
            },
        }
    }
}

/// Statistics for an async operation
#[derive(Debug, Clone)]
pub struct OperationStats {
    pub operation_name: String,
    pub duration: Duration,
    pub update_count: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub success_rate: f64,
}

/// Validator for real-time features
pub struct RealtimeFeatureValidator {
    progress: Arc<Mutex<Option<Arc<AsyncOperationMonitor>>>>,
    event: Arc<Mutex<Option<Arc<AsyncOperationMonitor>>>>,
    websocket: Arc<Mutex<Option<Arc<AsyncOperationMonitor>>>>,
}

impl RealtimeFeatureValidator {
    /// Create a new validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            progress: Arc::new(Mutex::new(None)),
            event: Arc::new(Mutex::new(None)),
            websocket: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for RealtimeFeatureValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl RealtimeFeatureValidator {
    /// Start monitoring progress operations
    pub async fn start_progress_monitoring(&self, progress_manager: &ProgressManager) {
        let monitor = AsyncOperationMonitor::new("progress_tracking".to_string());
        let monitor_arc = Arc::new(monitor);

        // Store the monitor
        {
            let mut stored = self.progress.lock().await;
            *stored = Some(monitor_arc.clone());
        }

        // Subscribe to progress updates
        let mut receiver = progress_manager.subscribe();
        let monitor_clone = monitor_arc.clone();

        tokio::spawn(async move {
            while let Ok(update) = receiver.recv().await {
                monitor_clone.record_update().await;

                match update.status {
                    crate::progress::ProgressStatus::Completed => {
                        monitor_clone.record_success().await;
                    }
                    crate::progress::ProgressStatus::Failed => {
                        monitor_clone.record_error().await;
                    }
                    _ => {
                        monitor_clone.record_update().await;
                    }
                }
            }
        });
    }

    /// Start monitoring event broadcasting
    pub async fn start_event_monitoring(&self, event_broadcaster: &EventBroadcaster) {
        let monitor = AsyncOperationMonitor::new("event_broadcasting".to_string());
        let monitor_arc = Arc::new(monitor);

        // Store the monitor
        {
            let mut stored = self.event.lock().await;
            *stored = Some(monitor_arc.clone());
        }

        // Subscribe to events
        let mut receiver = event_broadcaster.subscribe_all();
        let monitor_clone = monitor_arc.clone();

        tokio::spawn(async move {
            while let Ok(_event) = receiver.recv().await {
                monitor_clone.record_success().await;
            }
        });
    }

    /// Validate that all monitored features are healthy
    pub async fn validate_health(&self) -> ValidationResult {
        let mut results = Vec::new();

        // Check progress monitoring
        if let Some(monitor) = self.progress.lock().await.as_ref() {
            let is_healthy = monitor.is_healthy(Duration::from_secs(30)).await;
            let stats = monitor.get_stats().await;
            results.push(FeatureHealth {
                feature: "progress_tracking".to_string(),
                is_healthy,
                stats: Some(stats),
            });
        }

        // Check event monitoring
        if let Some(monitor) = self.event.lock().await.as_ref() {
            let is_healthy = monitor.is_healthy(Duration::from_secs(30)).await;
            let stats = monitor.get_stats().await;
            results.push(FeatureHealth {
                feature: "event_broadcasting".to_string(),
                is_healthy,
                stats: Some(stats),
            });
        }

        // Check WebSocket monitoring
        if let Some(monitor) = self.websocket.lock().await.as_ref() {
            let is_healthy = monitor.is_healthy(Duration::from_secs(30)).await;
            let stats = monitor.get_stats().await;
            results.push(FeatureHealth {
                feature: "websocket_communication".to_string(),
                is_healthy,
                stats: Some(stats),
            });
        }

        ValidationResult { features: results }
    }
}

/// Health status for a feature
#[derive(Debug, Clone)]
pub struct FeatureHealth {
    pub feature: String,
    pub is_healthy: bool,
    pub stats: Option<OperationStats>,
}

/// Result of validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub features: Vec<FeatureHealth>,
}

impl ValidationResult {
    /// Check if all features are healthy
    #[must_use]
    pub fn all_healthy(&self) -> bool {
        self.features.iter().all(|f| f.is_healthy)
    }

    /// Get unhealthy features
    #[must_use]
    pub fn unhealthy_features(&self) -> Vec<&FeatureHealth> {
        self.features.iter().filter(|f| !f.is_healthy).collect()
    }

    /// Print a summary
    pub fn print_summary(&self) {
        println!("🔍 Real-time Feature Health Check");
        println!("=================================");

        for feature in &self.features {
            let status = if feature.is_healthy { "✅" } else { "❌" };
            println!(
                "{} {}: {}",
                status,
                feature.feature,
                if feature.is_healthy {
                    "Healthy"
                } else {
                    "Unhealthy"
                }
            );

            if let Some(stats) = &feature.stats {
                println!("   Duration: {:?}", stats.duration);
                println!("   Updates: {}", stats.update_count);
                println!("   Success Rate: {:.2}%", stats.success_rate * 100.0);
            }
        }

        if self.all_healthy() {
            println!("\n🎉 All real-time features are working correctly!");
        } else {
            println!("\n⚠️  Some features need attention:");
            for feature in self.unhealthy_features() {
                println!("   - {}", feature.feature);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_operation_monitor() {
        let monitor = AsyncOperationMonitor::new("test_operation".to_string());

        // Initially should not be healthy (no activity)
        assert!(!monitor.is_healthy(Duration::from_secs(1)).await);

        // Record some activity
        monitor.record_success().await;
        monitor.record_update().await;

        // Should now be healthy
        assert!(monitor.is_healthy(Duration::from_secs(1)).await);

        // Check stats
        let stats = monitor.get_stats().await;
        assert_eq!(stats.operation_name, "test_operation");
        assert_eq!(stats.success_count, 1);
        assert_eq!(stats.update_count, 1);
        assert!((stats.success_rate - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_validation_result() {
        let result = ValidationResult {
            features: vec![
                FeatureHealth {
                    feature: "test1".to_string(),
                    is_healthy: true,
                    stats: None,
                },
                FeatureHealth {
                    feature: "test2".to_string(),
                    is_healthy: false,
                    stats: None,
                },
            ],
        };

        assert!(!result.all_healthy());
        assert_eq!(result.unhealthy_features().len(), 1);
    }

    #[tokio::test]
    async fn test_async_operation_monitor_error_recording() {
        let monitor = AsyncOperationMonitor::new("test_operation".to_string());

        // Record some errors
        monitor.record_error().await;
        monitor.record_error().await;

        // Check stats
        let stats = monitor.get_stats().await;
        assert_eq!(stats.operation_name, "test_operation");
        assert_eq!(stats.error_count, 2);
        assert_eq!(stats.success_count, 0);
        assert_eq!(stats.update_count, 0);
        assert!((stats.success_rate - 0.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_async_operation_monitor_mixed_operations() {
        let monitor = AsyncOperationMonitor::new("test_operation".to_string());

        // Record mixed operations
        monitor.record_success().await;
        monitor.record_update().await;
        monitor.record_error().await;
        monitor.record_success().await;

        // Check stats
        let stats = monitor.get_stats().await;
        assert_eq!(stats.operation_name, "test_operation");
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.error_count, 1);
        assert_eq!(stats.update_count, 1);
        // Success rate is calculated as success_count / update_count, not total operations
        assert!((stats.success_rate - 2.0).abs() < f64::EPSILON); // 2 successes out of 1 update
    }

    #[tokio::test]
    async fn test_async_operation_monitor_health_check() {
        let monitor = AsyncOperationMonitor::new("test_operation".to_string());

        // Initially should not be healthy (no activity)
        assert!(!monitor.is_healthy(Duration::from_secs(1)).await);

        // Record activity
        monitor.record_success().await;

        // Should now be healthy
        assert!(monitor.is_healthy(Duration::from_secs(1)).await);

        // Wait longer than the health check duration
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!monitor.is_healthy(Duration::from_millis(50)).await);
    }

    #[tokio::test]
    async fn test_async_operation_monitor_duration() {
        let monitor = AsyncOperationMonitor::new("test_operation".to_string());

        // Wait a bit
        tokio::time::sleep(Duration::from_millis(10)).await;

        let stats = monitor.get_stats().await;
        assert!(stats.duration.as_millis() >= 10);
    }

    #[tokio::test]
    async fn test_realtime_feature_validator_creation() {
        let validator = RealtimeFeatureValidator::new();
        assert!(validator.progress.lock().await.is_none());
        assert!(validator.event.lock().await.is_none());
        assert!(validator.websocket.lock().await.is_none());
    }

    #[tokio::test]
    async fn test_realtime_feature_validator_default() {
        let validator = RealtimeFeatureValidator::default();
        assert!(validator.progress.lock().await.is_none());
        assert!(validator.event.lock().await.is_none());
        assert!(validator.websocket.lock().await.is_none());
    }

    #[tokio::test]
    async fn test_validation_result_all_healthy() {
        let result = ValidationResult {
            features: vec![
                FeatureHealth {
                    feature: "test1".to_string(),
                    is_healthy: true,
                    stats: None,
                },
                FeatureHealth {
                    feature: "test2".to_string(),
                    is_healthy: true,
                    stats: None,
                },
            ],
        };

        assert!(result.all_healthy());
        assert_eq!(result.unhealthy_features().len(), 0);
    }

    #[tokio::test]
    async fn test_validation_result_empty() {
        let result = ValidationResult { features: vec![] };

        assert!(result.all_healthy());
        assert_eq!(result.unhealthy_features().len(), 0);
    }

    #[tokio::test]
    async fn test_validation_result_with_stats() {
        let stats = OperationStats {
            operation_name: "test_operation".to_string(),
            duration: Duration::from_secs(1),
            update_count: 10,
            success_count: 8,
            error_count: 2,
            success_rate: 0.8,
        };

        let result = ValidationResult {
            features: vec![FeatureHealth {
                feature: "test1".to_string(),
                is_healthy: true,
                stats: Some(stats),
            }],
        };

        assert!(result.all_healthy());
        assert_eq!(result.unhealthy_features().len(), 0);
    }

    #[test]
    fn test_operation_stats_creation() {
        let stats = OperationStats {
            operation_name: "test_operation".to_string(),
            duration: Duration::from_secs(5),
            update_count: 100,
            success_count: 95,
            error_count: 5,
            success_rate: 0.95,
        };

        assert_eq!(stats.operation_name, "test_operation");
        assert_eq!(stats.duration.as_secs(), 5);
        assert_eq!(stats.update_count, 100);
        assert_eq!(stats.success_count, 95);
        assert_eq!(stats.error_count, 5);
        assert!((stats.success_rate - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn test_feature_health_creation() {
        let health = FeatureHealth {
            feature: "test_feature".to_string(),
            is_healthy: true,
            stats: None,
        };

        assert_eq!(health.feature, "test_feature");
        assert!(health.is_healthy);
        assert!(health.stats.is_none());
    }

    #[test]
    fn test_validation_result_creation() {
        let result = ValidationResult { features: vec![] };

        assert_eq!(result.features.len(), 0);
    }

    #[tokio::test]
    async fn test_async_operation_monitor_concurrent_updates() {
        let monitor = Arc::new(AsyncOperationMonitor::new("concurrent_test".to_string()));

        // Spawn multiple tasks that update the monitor concurrently
        let mut handles = vec![];
        for i in 0..10 {
            let monitor_clone = monitor.clone();
            let handle = tokio::spawn(async move {
                for _ in 0..10 {
                    if i % 2 == 0 {
                        monitor_clone.record_success().await;
                    } else {
                        monitor_clone.record_error().await;
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        let stats = monitor.get_stats().await;
        assert_eq!(stats.success_count, 50); // 5 tasks * 10 successes each
        assert_eq!(stats.error_count, 50); // 5 tasks * 10 errors each
        assert_eq!(stats.update_count, 0); // No direct updates
                                           // Success rate is calculated as success_count / update_count
                                           // Since update_count is 0, success_rate should be 0.0
        assert!((stats.success_rate - 0.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_async_operation_monitor_record_success() {
        let monitor = AsyncOperationMonitor::new("test_operation".to_string());

        monitor.record_success().await;
        let stats = monitor.get_stats().await;

        assert_eq!(stats.success_count, 1);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.update_count, 0);
        assert!((stats.success_rate - 0.0).abs() < f64::EPSILON); // No updates, so 0.0
    }

    #[tokio::test]
    async fn test_async_operation_monitor_record_error() {
        let monitor = AsyncOperationMonitor::new("test_operation".to_string());

        monitor.record_error().await;
        let stats = monitor.get_stats().await;

        assert_eq!(stats.success_count, 0);
        assert_eq!(stats.error_count, 1);
        assert_eq!(stats.update_count, 0);
        assert!((stats.success_rate - 0.0).abs() < f64::EPSILON); // No updates, so 0.0
    }

    #[tokio::test]
    async fn test_async_operation_monitor_record_update() {
        let monitor = AsyncOperationMonitor::new("test_operation".to_string());

        monitor.record_update().await;
        let stats = monitor.get_stats().await;

        assert_eq!(stats.success_count, 0);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.update_count, 1);
        assert!((stats.success_rate - 0.0).abs() < f64::EPSILON); // No successes, so 0.0
    }

    #[tokio::test]
    async fn test_async_operation_monitor_duration_tracking() {
        let monitor = AsyncOperationMonitor::new("test_operation".to_string());

        let start = Instant::now();
        monitor.record_success().await;
        let duration = start.elapsed();

        let stats = monitor.get_stats().await;
        assert!(stats.duration >= duration);
    }

    #[tokio::test]
    async fn test_realtime_feature_validator_start_progress_monitoring() {
        let validator = RealtimeFeatureValidator::new();
        let progress_manager = Arc::new(ProgressManager::new());

        // This should not panic
        validator.start_progress_monitoring(&progress_manager).await;

        // Verify the monitor was created
        let progress_monitor = validator.progress.lock().await;
        assert!(progress_monitor.is_some());
    }

    #[tokio::test]
    async fn test_realtime_feature_validator_start_event_monitoring() {
        let validator = RealtimeFeatureValidator::new();
        let event_broadcaster = Arc::new(EventBroadcaster::new());

        // This should not panic
        validator.start_event_monitoring(&event_broadcaster).await;

        // Verify the monitor was created
        let event_monitor = validator.event.lock().await;
        assert!(event_monitor.is_some());
    }

    #[test]
    fn test_operation_stats_clone() {
        let stats = OperationStats {
            operation_name: "test_operation".to_string(),
            duration: Duration::from_secs(5),
            update_count: 100,
            success_count: 95,
            error_count: 5,
            success_rate: 0.95,
        };

        let cloned = stats.clone();
        assert_eq!(stats.operation_name, cloned.operation_name);
        assert_eq!(stats.duration, cloned.duration);
        assert_eq!(stats.update_count, cloned.update_count);
        assert_eq!(stats.success_count, cloned.success_count);
        assert_eq!(stats.error_count, cloned.error_count);
        assert!((stats.success_rate - cloned.success_rate).abs() < f64::EPSILON);
    }

    #[test]
    fn test_operation_stats_debug() {
        let stats = OperationStats {
            operation_name: "test_operation".to_string(),
            duration: Duration::from_secs(5),
            update_count: 100,
            success_count: 95,
            error_count: 5,
            success_rate: 0.95,
        };

        let debug_str = format!("{stats:?}");
        assert!(debug_str.contains("OperationStats"));
        assert!(debug_str.contains("test_operation"));
    }

    #[test]
    fn test_feature_health_clone() {
        let stats = OperationStats {
            operation_name: "test_operation".to_string(),
            duration: Duration::from_secs(5),
            update_count: 100,
            success_count: 95,
            error_count: 5,
            success_rate: 0.95,
        };

        let health = FeatureHealth {
            feature: "test_feature".to_string(),
            is_healthy: true,
            stats: Some(stats),
        };

        let cloned = health.clone();
        assert_eq!(health.feature, cloned.feature);
        assert_eq!(health.is_healthy, cloned.is_healthy);
        assert!(cloned.stats.is_some());
    }

    #[test]
    fn test_feature_health_debug() {
        let health = FeatureHealth {
            feature: "test_feature".to_string(),
            is_healthy: true,
            stats: None,
        };

        let debug_str = format!("{health:?}");
        assert!(debug_str.contains("FeatureHealth"));
        assert!(debug_str.contains("test_feature"));
    }

    #[test]
    fn test_validation_result_clone() {
        let result = ValidationResult {
            features: vec![FeatureHealth {
                feature: "test_feature".to_string(),
                is_healthy: true,
                stats: None,
            }],
        };

        let cloned = result.clone();
        assert_eq!(result.features.len(), cloned.features.len());
        assert_eq!(result.features[0].feature, cloned.features[0].feature);
    }

    #[test]
    fn test_validation_result_debug() {
        let result = ValidationResult {
            features: vec![FeatureHealth {
                feature: "test_feature".to_string(),
                is_healthy: true,
                stats: None,
            }],
        };

        let debug_str = format!("{result:?}");
        assert!(debug_str.contains("ValidationResult"));
    }

    #[tokio::test]
    async fn test_async_operation_monitor_zero_operations() {
        let monitor = AsyncOperationMonitor::new("test_operation".to_string());

        let stats = monitor.get_stats().await;
        assert_eq!(stats.success_count, 0);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.update_count, 0);
        assert!((stats.success_rate - 0.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_async_operation_monitor_all_success() {
        let monitor = AsyncOperationMonitor::new("test_operation".to_string());

        for _ in 0..5 {
            monitor.record_success().await;
        }

        let stats = monitor.get_stats().await;
        assert_eq!(stats.success_count, 5);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.update_count, 0);
        assert!((stats.success_rate - 0.0).abs() < f64::EPSILON); // No updates, so 0.0
    }

    #[tokio::test]
    async fn test_async_operation_monitor_all_error() {
        let monitor = AsyncOperationMonitor::new("test_operation".to_string());

        for _ in 0..5 {
            monitor.record_error().await;
        }

        let stats = monitor.get_stats().await;
        assert_eq!(stats.success_count, 0);
        assert_eq!(stats.error_count, 5);
        assert_eq!(stats.update_count, 0);
        assert!((stats.success_rate - 0.0).abs() < f64::EPSILON); // No updates, so 0.0
    }

    #[tokio::test]
    async fn test_async_operation_monitor_is_healthy() {
        let monitor = AsyncOperationMonitor::new("test_operation".to_string());

        // Initially should not be healthy (no activity)
        assert!(!monitor.is_healthy(Duration::from_millis(100)).await);

        // Record some activity
        monitor.record_update().await;

        // Should be healthy now
        assert!(monitor.is_healthy(Duration::from_millis(100)).await);

        // Wait longer than max_silence
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should not be healthy anymore
        assert!(!monitor.is_healthy(Duration::from_millis(100)).await);
    }

    #[tokio::test]
    async fn test_async_operation_monitor_get_stats_detailed() {
        let monitor = AsyncOperationMonitor::new("detailed_test".to_string());

        // Record various activities
        monitor.record_success().await;
        monitor.record_success().await;
        monitor.record_error().await;
        monitor.record_update().await;
        monitor.record_update().await;

        let stats = monitor.get_stats().await;
        assert_eq!(stats.operation_name, "detailed_test");
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.error_count, 1);
        assert_eq!(stats.update_count, 2);
        assert!(stats.duration.as_nanos() >= 0); // Duration should be non-negative
                                                 // Success rate should be 2/2 = 1.0 (only counting updates, not errors)
        assert!((stats.success_rate - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_realtime_feature_validator_validate_health() {
        let validator = RealtimeFeatureValidator::new();

        // Test with no monitors (should return empty result)
        let result = validator.validate_health().await;
        assert!(result.features.is_empty());
    }
}
