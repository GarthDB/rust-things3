//! Database query performance tracking and optimization

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::debug;
use uuid::Uuid;

/// Query execution context for tracking performance
#[derive(Debug, Clone)]
pub struct QueryContext {
    pub query_id: Uuid,
    pub query_type: String,
    pub query_text: String,
    pub parameters: Vec<String>,
    pub start_time: Instant,
    pub cache_hit: bool,
    pub result_size: Option<usize>,
}

/// Query performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPerformanceMetrics {
    pub query_id: Uuid,
    pub query_type: String,
    pub query_text: String,
    pub execution_time_ms: u64,
    pub cache_hit: bool,
    pub result_size: Option<usize>,
    pub memory_usage_bytes: Option<u64>,
    pub cpu_usage_percent: Option<f64>,
    pub timestamp: DateTime<Utc>,
    pub parameters: Vec<String>,
    pub optimization_applied: Vec<String>,
}

/// Aggregated query performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPerformanceStats {
    pub query_type: String,
    pub total_executions: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub average_execution_time_ms: f64,
    pub min_execution_time_ms: u64,
    pub max_execution_time_ms: u64,
    pub p95_execution_time_ms: u64,
    pub p99_execution_time_ms: u64,
    pub average_result_size: f64,
    pub total_memory_usage_bytes: u64,
    pub average_cpu_usage_percent: f64,
    pub cache_hit_rate: f64,
    pub slow_queries_count: u64,
    pub fast_queries_count: u64,
    pub last_executed: Option<DateTime<Utc>>,
}

/// Query optimization suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOptimizationSuggestion {
    pub query_type: String,
    pub suggestion_type: OptimizationType,
    pub description: String,
    pub potential_improvement_percent: f64,
    pub priority: OptimizationPriority,
    pub implementation_effort: ImplementationEffort,
}

/// Types of query optimizations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OptimizationType {
    /// Add database index
    AddIndex,
    /// Use prepared statement
    UsePreparedStatement,
    /// Optimize query structure
    OptimizeQuery,
    /// Add caching
    AddCaching,
    /// Reduce result set size
    ReduceResultSet,
    /// Use connection pooling
    UseConnectionPooling,
}

/// Priority levels for optimizations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptimizationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Implementation effort levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImplementationEffort {
    Low,
    Medium,
    High,
}

/// Query performance tracker
pub struct QueryPerformanceTracker {
    /// Individual query metrics
    metrics: Arc<RwLock<Vec<QueryPerformanceMetrics>>>,
    /// Aggregated statistics by query type
    stats: Arc<RwLock<HashMap<String, QueryPerformanceStats>>>,
    /// Optimization suggestions
    suggestions: Arc<RwLock<Vec<QueryOptimizationSuggestion>>>,
    /// Maximum number of metrics to keep
    max_metrics: usize,
    /// Slow query threshold in milliseconds
    slow_query_threshold_ms: u64,
    /// Fast query threshold in milliseconds
    fast_query_threshold_ms: u64,
}

impl QueryPerformanceTracker {
    /// Create a new query performance tracker
    #[must_use]
    pub fn new(
        max_metrics: usize,
        slow_query_threshold_ms: u64,
        fast_query_threshold_ms: u64,
    ) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
            suggestions: Arc::new(RwLock::new(Vec::new())),
            max_metrics,
            slow_query_threshold_ms,
            fast_query_threshold_ms,
        }
    }

    /// Create a new tracker with default settings
    #[must_use]
    pub fn new_default() -> Self {
        Self::new(10000, 1000, 100) // 10k metrics, 1s slow, 100ms fast
    }

    /// Start tracking a query execution
    #[must_use]
    pub fn start_query(
        &self,
        query_type: &str,
        query_text: &str,
        parameters: Vec<String>,
    ) -> QueryContext {
        QueryContext {
            query_id: Uuid::new_v4(),
            query_type: query_type.to_string(),
            query_text: query_text.to_string(),
            parameters,
            start_time: Instant::now(),
            cache_hit: false,
            result_size: None,
        }
    }

    /// Complete query tracking with results
    pub fn complete_query(
        &self,
        context: QueryContext,
        cache_hit: bool,
        result_size: Option<usize>,
        memory_usage_bytes: Option<u64>,
        cpu_usage_percent: Option<f64>,
        optimization_applied: Vec<String>,
    ) {
        let execution_time = context.start_time.elapsed();
        let execution_time_ms = execution_time.as_millis() as u64;

        let metric = QueryPerformanceMetrics {
            query_id: context.query_id,
            query_type: context.query_type.clone(),
            query_text: context.query_text,
            execution_time_ms,
            cache_hit,
            result_size,
            memory_usage_bytes,
            cpu_usage_percent,
            timestamp: Utc::now(),
            parameters: context.parameters,
            optimization_applied,
        };

        // Add to metrics
        {
            let mut metrics = self.metrics.write();
            metrics.push(metric.clone());

            // Trim if we exceed max_metrics
            if metrics.len() > self.max_metrics {
                let excess = metrics.len() - self.max_metrics;
                metrics.drain(0..excess);
            }
        }

        // Update aggregated statistics
        self.update_stats(&metric);

        // Generate optimization suggestions if needed
        self.generate_optimization_suggestions(&metric);

        debug!(
            "Query completed: {} ({}ms, cache_hit: {}, size: {:?})",
            context.query_type, execution_time_ms, cache_hit, result_size
        );
    }

    /// Get performance statistics for a specific query type
    #[must_use]
    pub fn get_stats(&self, query_type: &str) -> Option<QueryPerformanceStats> {
        let stats = self.stats.read();
        stats.get(query_type).cloned()
    }

    /// Get all performance statistics
    #[must_use]
    pub fn get_all_stats(&self) -> HashMap<String, QueryPerformanceStats> {
        let stats = self.stats.read();
        stats.clone()
    }

    /// Get optimization suggestions
    #[must_use]
    pub fn get_optimization_suggestions(&self) -> Vec<QueryOptimizationSuggestion> {
        let suggestions = self.suggestions.read();
        suggestions.clone()
    }

    /// Get slow queries (above threshold)
    #[must_use]
    pub fn get_slow_queries(&self) -> Vec<QueryPerformanceMetrics> {
        let metrics = self.metrics.read();
        metrics
            .iter()
            .filter(|m| m.execution_time_ms >= self.slow_query_threshold_ms)
            .cloned()
            .collect()
    }

    /// Get fast queries (below threshold)
    #[must_use]
    pub fn get_fast_queries(&self) -> Vec<QueryPerformanceMetrics> {
        let metrics = self.metrics.read();
        metrics
            .iter()
            .filter(|m| m.execution_time_ms <= self.fast_query_threshold_ms)
            .cloned()
            .collect()
    }

    /// Get query performance summary
    #[must_use]
    pub fn get_performance_summary(&self) -> QueryPerformanceSummary {
        let stats = self.get_all_stats();
        let suggestions = self.get_optimization_suggestions();
        let slow_queries = self.get_slow_queries();
        let fast_queries = self.get_fast_queries();

        let total_queries: u64 = stats.values().map(|s| s.total_executions).sum();
        let total_cache_hits: u64 = stats.values().map(|s| s.cache_hits).sum();
        let overall_cache_hit_rate = if total_queries > 0 {
            total_cache_hits as f64 / total_queries as f64
        } else {
            0.0
        };

        let average_execution_time = if stats.is_empty() {
            0.0
        } else {
            stats
                .values()
                .map(|s| s.average_execution_time_ms)
                .sum::<f64>()
                / stats.len() as f64
        };

        QueryPerformanceSummary {
            timestamp: Utc::now(),
            total_queries,
            overall_cache_hit_rate,
            average_execution_time_ms: average_execution_time,
            slow_queries_count: slow_queries.len() as u64,
            fast_queries_count: fast_queries.len() as u64,
            optimization_suggestions_count: suggestions.len() as u64,
            stats,
            suggestions,
        }
    }

    /// Update aggregated statistics
    fn update_stats(&self, metric: &QueryPerformanceMetrics) {
        let mut stats = self.stats.write();
        let entry =
            stats
                .entry(metric.query_type.clone())
                .or_insert_with(|| QueryPerformanceStats {
                    query_type: metric.query_type.clone(),
                    total_executions: 0,
                    cache_hits: 0,
                    cache_misses: 0,
                    average_execution_time_ms: 0.0,
                    min_execution_time_ms: u64::MAX,
                    max_execution_time_ms: 0,
                    p95_execution_time_ms: 0,
                    p99_execution_time_ms: 0,
                    average_result_size: 0.0,
                    total_memory_usage_bytes: 0,
                    average_cpu_usage_percent: 0.0,
                    cache_hit_rate: 0.0,
                    slow_queries_count: 0,
                    fast_queries_count: 0,
                    last_executed: None,
                });

        entry.total_executions += 1;
        entry.last_executed = Some(metric.timestamp);

        if metric.cache_hit {
            entry.cache_hits += 1;
        } else {
            entry.cache_misses += 1;
        }

        // Update execution time statistics
        if metric.execution_time_ms < entry.min_execution_time_ms {
            entry.min_execution_time_ms = metric.execution_time_ms;
        }
        if metric.execution_time_ms > entry.max_execution_time_ms {
            entry.max_execution_time_ms = metric.execution_time_ms;
        }

        // Recalculate average execution time
        entry.average_execution_time_ms = (entry.average_execution_time_ms
            * (entry.total_executions - 1) as f64
            + metric.execution_time_ms as f64)
            / entry.total_executions as f64;

        // Update result size statistics
        if let Some(size) = metric.result_size {
            entry.average_result_size =
                (entry.average_result_size * (entry.total_executions - 1) as f64 + size as f64)
                    / entry.total_executions as f64;
        }

        // Update memory usage
        if let Some(memory) = metric.memory_usage_bytes {
            entry.total_memory_usage_bytes += memory;
        }

        // Update CPU usage
        if let Some(cpu) = metric.cpu_usage_percent {
            entry.average_cpu_usage_percent =
                (entry.average_cpu_usage_percent * (entry.total_executions - 1) as f64 + cpu)
                    / entry.total_executions as f64;
        }

        // Update cache hit rate
        entry.cache_hit_rate = if entry.total_executions > 0 {
            entry.cache_hits as f64 / entry.total_executions as f64
        } else {
            0.0
        };

        // Update slow/fast query counts
        if metric.execution_time_ms >= self.slow_query_threshold_ms {
            entry.slow_queries_count += 1;
        }
        if metric.execution_time_ms <= self.fast_query_threshold_ms {
            entry.fast_queries_count += 1;
        }

        // Calculate percentiles (simplified - in production, use proper percentile calculation)
        self.calculate_percentiles(entry);
    }

    /// Calculate percentiles for execution time
    fn calculate_percentiles(&self, stats: &mut QueryPerformanceStats) {
        // Get all execution times for this query type
        let metrics = self.metrics.read();
        let mut execution_times: Vec<u64> = metrics
            .iter()
            .filter(|m| m.query_type == stats.query_type)
            .map(|m| m.execution_time_ms)
            .collect();

        execution_times.sort_unstable();

        if !execution_times.is_empty() {
            let len = execution_times.len();

            // P95
            let p95_index = (len as f64 * 0.95) as usize;
            stats.p95_execution_time_ms = execution_times[p95_index.min(len - 1)];

            // P99
            let p99_index = (len as f64 * 0.99) as usize;
            stats.p99_execution_time_ms = execution_times[p99_index.min(len - 1)];
        }
    }

    /// Generate optimization suggestions based on query performance
    fn generate_optimization_suggestions(&self, metric: &QueryPerformanceMetrics) {
        let mut suggestions = self.suggestions.write();

        // Remove existing suggestions for this query type
        suggestions.retain(|s| s.query_type != metric.query_type);

        let mut new_suggestions = Vec::new();

        // Slow query suggestions
        if metric.execution_time_ms >= self.slow_query_threshold_ms {
            new_suggestions.push(QueryOptimizationSuggestion {
                query_type: metric.query_type.clone(),
                suggestion_type: OptimizationType::AddIndex,
                description: format!(
                    "Query is slow ({}ms). Consider adding database indexes.",
                    metric.execution_time_ms
                ),
                potential_improvement_percent: 50.0,
                priority: OptimizationPriority::High,
                implementation_effort: ImplementationEffort::Medium,
            });

            new_suggestions.push(QueryOptimizationSuggestion {
                query_type: metric.query_type.clone(),
                suggestion_type: OptimizationType::OptimizeQuery,
                description: "Query structure could be optimized for better performance."
                    .to_string(),
                potential_improvement_percent: 30.0,
                priority: OptimizationPriority::Medium,
                implementation_effort: ImplementationEffort::High,
            });
        }

        // Low cache hit rate suggestions
        if !metric.cache_hit {
            new_suggestions.push(QueryOptimizationSuggestion {
                query_type: metric.query_type.clone(),
                suggestion_type: OptimizationType::AddCaching,
                description:
                    "Query is not cached. Consider implementing caching for better performance."
                        .to_string(),
                potential_improvement_percent: 80.0,
                priority: OptimizationPriority::High,
                implementation_effort: ImplementationEffort::Medium,
            });
        }

        // Large result set suggestions
        if let Some(size) = metric.result_size {
            if size > 1000 {
                new_suggestions.push(QueryOptimizationSuggestion {
                    query_type: metric.query_type.clone(),
                    suggestion_type: OptimizationType::ReduceResultSet,
                    description: format!(
                        "Large result set ({size} items). Consider pagination or filtering."
                    ),
                    potential_improvement_percent: 40.0,
                    priority: OptimizationPriority::Medium,
                    implementation_effort: ImplementationEffort::Low,
                });
            }
        }

        // High memory usage suggestions
        if let Some(memory) = metric.memory_usage_bytes {
            if memory > 10 * 1024 * 1024 {
                // 10MB
                new_suggestions.push(QueryOptimizationSuggestion {
                    query_type: metric.query_type.clone(),
                    suggestion_type: OptimizationType::UsePreparedStatement,
                    description: "High memory usage. Consider using prepared statements."
                        .to_string(),
                    potential_improvement_percent: 20.0,
                    priority: OptimizationPriority::Low,
                    implementation_effort: ImplementationEffort::Low,
                });
            }
        }

        suggestions.extend(new_suggestions);
    }
}

/// Query performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPerformanceSummary {
    pub timestamp: DateTime<Utc>,
    pub total_queries: u64,
    pub overall_cache_hit_rate: f64,
    pub average_execution_time_ms: f64,
    pub slow_queries_count: u64,
    pub fast_queries_count: u64,
    pub optimization_suggestions_count: u64,
    pub stats: HashMap<String, QueryPerformanceStats>,
    pub suggestions: Vec<QueryOptimizationSuggestion>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_query_performance_tracking() {
        let tracker = QueryPerformanceTracker::new_default();

        // Start a query
        let context = tracker.start_query(
            "test_query",
            "SELECT * FROM tasks",
            vec!["param1".to_string()],
        );

        // Simulate query execution time
        thread::sleep(Duration::from_millis(100));

        // Complete the query
        tracker.complete_query(
            context,
            false,      // cache miss
            Some(100),  // result size
            Some(1024), // memory usage
            Some(5.0),  // CPU usage
            vec!["index_optimization".to_string()],
        );

        // Check statistics
        let stats = tracker.get_stats("test_query");
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.total_executions, 1);
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.cache_hits, 0);
        assert!(stats.average_execution_time_ms >= 100.0);
    }

    #[test]
    fn test_optimization_suggestions() {
        let tracker = QueryPerformanceTracker::new(1000, 50, 10); // Very low thresholds for testing

        // Start a slow query
        let context = tracker.start_query("slow_query", "SELECT * FROM tasks", vec![]);
        thread::sleep(Duration::from_millis(60)); // Above 50ms threshold
        tracker.complete_query(context, false, Some(2000), None, None, vec![]);

        // Check for optimization suggestions
        let suggestions = tracker.get_optimization_suggestions();
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.query_type == "slow_query"));
    }

    #[test]
    fn test_performance_summary() {
        let tracker = QueryPerformanceTracker::new_default();

        // Execute some queries
        for i in 0..5 {
            let context = tracker.start_query("test_query", "SELECT * FROM tasks", vec![]);
            thread::sleep(Duration::from_millis(10));
            tracker.complete_query(
                context,
                i % 2 == 0, // Alternate cache hits/misses
                Some(100),
                None,
                None,
                vec![],
            );
        }

        let summary = tracker.get_performance_summary();
        assert_eq!(summary.total_queries, 5);
        assert!(summary.overall_cache_hit_rate > 0.0);
        assert!(summary.average_execution_time_ms > 0.0);
    }
}
