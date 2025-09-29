//! Performance testing utilities for MCP operations

use crate::mcp::test_harness::McpTestHarness;
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Performance test configuration
#[derive(Debug, Clone)]
pub struct PerformanceTestConfig {
    /// Maximum allowed duration for operations
    pub max_duration: Duration,
    /// Number of iterations to run for benchmarking
    pub iterations: usize,
    /// Whether to run concurrent tests
    pub run_concurrent: bool,
    /// Whether to include memory usage tracking
    pub track_memory: bool,
}

impl Default for PerformanceTestConfig {
    fn default() -> Self {
        Self {
            max_duration: Duration::from_secs(1),
            iterations: 10,
            run_concurrent: true,
            track_memory: false,
        }
    }
}

/// Performance test results
#[derive(Debug, Clone)]
pub struct PerformanceTestResults {
    /// Average duration across all iterations
    pub average_duration: Duration,
    /// Minimum duration observed
    pub min_duration: Duration,
    /// Maximum duration observed
    pub max_duration: Duration,
    /// Standard deviation of durations
    pub std_deviation: Duration,
    /// Number of successful operations
    pub success_count: usize,
    /// Number of failed operations
    pub failure_count: usize,
    /// Whether the test passed the performance threshold
    pub passed: bool,
}

/// Performance test runner for MCP operations
pub struct McpPerformanceTestRunner {
    harness: McpTestHarness,
    config: PerformanceTestConfig,
}

impl Default for McpPerformanceTestRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl McpPerformanceTestRunner {
    /// Create a new performance test runner
    #[must_use]
    pub fn new() -> Self {
        Self {
            harness: McpTestHarness::new(),
            config: PerformanceTestConfig::default(),
        }
    }

    /// Create a new performance test runner with custom configuration
    #[must_use]
    pub fn with_config(config: PerformanceTestConfig) -> Self {
        Self {
            harness: McpTestHarness::new(),
            config,
        }
    }

    /// Run performance tests for tool calls
    pub async fn test_tool_performance(
        &self,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> PerformanceTestResults {
        let mut durations = Vec::new();
        let mut success_count = 0;
        let mut failure_count = 0;

        for _ in 0..self.config.iterations {
            let start = Instant::now();

            let result = timeout(
                self.config.max_duration,
                self.harness.call_tool(tool_name, arguments.clone()),
            )
            .await;

            match result {
                Ok(Ok(_)) => {
                    let duration = start.elapsed();
                    durations.push(duration);
                    success_count += 1;
                }
                Ok(Err(_)) | Err(_) => {
                    failure_count += 1;
                }
            }
        }

        self.calculate_results(&durations, success_count, failure_count)
    }

    /// Run performance tests for resource reads
    pub async fn test_resource_performance(&self, uri: &str) -> PerformanceTestResults {
        let mut durations = Vec::new();
        let mut success_count = 0;
        let mut failure_count = 0;

        for _ in 0..self.config.iterations {
            let start = Instant::now();

            let result = timeout(self.config.max_duration, self.harness.read_resource(uri)).await;

            match result {
                Ok(Ok(_)) => {
                    let duration = start.elapsed();
                    durations.push(duration);
                    success_count += 1;
                }
                Ok(Err(_)) | Err(_) => {
                    failure_count += 1;
                }
            }
        }

        self.calculate_results(&durations, success_count, failure_count)
    }

    /// Run performance tests for prompt calls
    pub async fn test_prompt_performance(
        &self,
        prompt_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> PerformanceTestResults {
        let mut durations = Vec::new();
        let mut success_count = 0;
        let mut failure_count = 0;

        for _ in 0..self.config.iterations {
            let start = Instant::now();

            let result = timeout(
                self.config.max_duration,
                self.harness.get_prompt(prompt_name, arguments.clone()),
            )
            .await;

            match result {
                Ok(Ok(_)) => {
                    let duration = start.elapsed();
                    durations.push(duration);
                    success_count += 1;
                }
                Ok(Err(_)) | Err(_) => {
                    failure_count += 1;
                }
            }
        }

        self.calculate_results(&durations, success_count, failure_count)
    }

    /// Run concurrent performance tests
    pub async fn test_concurrent_performance(&self) -> ConcurrentPerformanceResults {
        if !self.config.run_concurrent {
            return ConcurrentPerformanceResults::default();
        }

        let start = Instant::now();

        // Run multiple operations concurrently
        let tool_futures = vec![
            self.harness.call_tool("get_inbox", None),
            self.harness.call_tool("get_today", None),
            self.harness.call_tool("get_areas", None),
        ];

        let resource_futures = vec![
            self.harness.read_resource("things://inbox"),
            self.harness.read_resource("things://today"),
        ];

        let prompt_futures = vec![self
            .harness
            .get_prompt("task_review", Some(json!({"task_title": "Test"})))];

        let tool_results = timeout(
            self.config.max_duration,
            futures::future::join_all(tool_futures),
        )
        .await;
        let resource_results = timeout(
            self.config.max_duration,
            futures::future::join_all(resource_futures),
        )
        .await;
        let prompt_results = timeout(
            self.config.max_duration,
            futures::future::join_all(prompt_futures),
        )
        .await;

        let mut success_count = 0;
        let mut total_operations = 0;

        if let Ok(results) = tool_results {
            success_count += results.iter().filter(|r| r.is_ok()).count();
            total_operations += results.len();
        }

        if let Ok(results) = resource_results {
            success_count += results.iter().filter(|r| r.is_ok()).count();
            total_operations += results.len();
        }

        if let Ok(results) = prompt_results {
            success_count += results.iter().filter(|r| r.is_ok()).count();
            total_operations += results.len();
        }

        let total_duration = start.elapsed();

        ConcurrentPerformanceResults {
            total_duration,
            success_count,
            total_operations,
            #[allow(clippy::cast_precision_loss)]
            operations_per_second: success_count as f64 / total_duration.as_secs_f64(),
        }
    }

    /// Run comprehensive performance tests
    pub async fn run_comprehensive_tests(&self) -> ComprehensivePerformanceResults {
        let tool_results = self.test_tool_performance("get_inbox", None).await;
        let resource_results = self.test_resource_performance("things://inbox").await;
        let prompt_results = self
            .test_prompt_performance("task_review", Some(json!({"task_title": "Test"})))
            .await;
        let concurrent_results = self.test_concurrent_performance().await;

        ComprehensivePerformanceResults {
            tool_performance: tool_results,
            resource_performance: resource_results,
            prompt_performance: prompt_results,
            concurrent_performance: concurrent_results,
        }
    }

    /// Calculate performance test results
    #[allow(clippy::cast_precision_loss)]
    fn calculate_results(
        &self,
        durations: &[Duration],
        success_count: usize,
        failure_count: usize,
    ) -> PerformanceTestResults {
        if durations.is_empty() {
            return PerformanceTestResults {
                average_duration: Duration::ZERO,
                min_duration: Duration::ZERO,
                max_duration: Duration::ZERO,
                std_deviation: Duration::ZERO,
                success_count,
                failure_count,
                passed: false,
            };
        }

        let total_duration: Duration = durations.iter().sum();
        let average_duration = total_duration / u32::try_from(durations.len()).unwrap_or(1);

        let min_duration = durations.iter().min().copied().unwrap_or(Duration::ZERO);
        let max_duration = durations.iter().max().copied().unwrap_or(Duration::ZERO);

        // Calculate standard deviation
        let variance: f64 = durations
            .iter()
            .map(|d| {
                let diff = d.as_nanos() as f64 - average_duration.as_nanos() as f64;
                diff * diff
            })
            .sum::<f64>()
            / durations.len() as f64;

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let std_deviation = Duration::from_nanos(variance.sqrt() as u64);

        let passed = average_duration <= self.config.max_duration && failure_count == 0;

        PerformanceTestResults {
            average_duration,
            min_duration,
            max_duration,
            std_deviation,
            success_count,
            failure_count,
            passed,
        }
    }
}

/// Results for concurrent performance tests
#[derive(Debug, Clone, Default)]
pub struct ConcurrentPerformanceResults {
    pub total_duration: Duration,
    pub success_count: usize,
    pub total_operations: usize,
    pub operations_per_second: f64,
}

/// Comprehensive performance test results
#[derive(Debug, Clone)]
pub struct ComprehensivePerformanceResults {
    pub tool_performance: PerformanceTestResults,
    pub resource_performance: PerformanceTestResults,
    pub prompt_performance: PerformanceTestResults,
    pub concurrent_performance: ConcurrentPerformanceResults,
}

impl ComprehensivePerformanceResults {
    /// Check if all performance tests passed
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.tool_performance.passed
            && self.resource_performance.passed
            && self.prompt_performance.passed
    }

    /// Get the overall performance score (0.0 to 1.0)
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn performance_score(&self) -> f64 {
        let mut score = 0.0;
        let mut count = 0;

        if self.tool_performance.success_count > 0 {
            score += self.tool_performance.success_count as f64
                / (self.tool_performance.success_count + self.tool_performance.failure_count)
                    as f64;
            count += 1;
        }

        if self.resource_performance.success_count > 0 {
            score += self.resource_performance.success_count as f64
                / (self.resource_performance.success_count
                    + self.resource_performance.failure_count) as f64;
            count += 1;
        }

        if self.prompt_performance.success_count > 0 {
            score += self.prompt_performance.success_count as f64
                / (self.prompt_performance.success_count + self.prompt_performance.failure_count)
                    as f64;
            count += 1;
        }

        if count > 0 {
            score / f64::from(count)
        } else {
            0.0
        }
    }
}

/// Memory usage tracker for performance tests
pub struct MemoryTracker {
    initial_memory: Option<usize>,
    peak_memory: Option<usize>,
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            initial_memory: None,
            peak_memory: None,
        }
    }

    pub fn start(&mut self) {
        self.initial_memory = Some(Self::get_current_memory_usage());
        self.peak_memory = self.initial_memory;
    }

    pub fn update(&mut self) {
        let current = Self::get_current_memory_usage();
        if let Some(peak) = self.peak_memory {
            if current > peak {
                self.peak_memory = Some(current);
            }
        } else {
            self.peak_memory = Some(current);
        }
    }

    #[must_use]
    pub fn get_memory_usage(&self) -> Option<usize> {
        self.peak_memory
            .and_then(|peak| self.initial_memory.map(|initial| peak - initial))
    }

    fn get_current_memory_usage() -> usize {
        // This is a simplified implementation
        // In a real implementation, you might use system-specific APIs
        // or external tools to get accurate memory usage
        0
    }
}

/// Performance benchmark for specific MCP operations
pub struct McpBenchmark {
    harness: McpTestHarness,
    config: PerformanceTestConfig,
}

impl Default for McpBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl McpBenchmark {
    #[must_use]
    pub fn new() -> Self {
        Self {
            harness: McpTestHarness::new(),
            config: PerformanceTestConfig::default(),
        }
    }

    #[must_use]
    pub fn with_config(config: PerformanceTestConfig) -> Self {
        Self {
            harness: McpTestHarness::new(),
            config,
        }
    }

    /// Benchmark a specific tool call
    ///
    /// # Panics
    /// Panics if the tool call fails during benchmarking
    pub async fn benchmark_tool(
        &self,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> BenchmarkResults {
        let mut durations = Vec::new();
        let mut memory_tracker = MemoryTracker::new();

        memory_tracker.start();

        for _ in 0..self.config.iterations {
            let start = Instant::now();

            let result = self.harness.call_tool(tool_name, arguments.clone()).await;

            let duration = start.elapsed();
            durations.push(duration);

            memory_tracker.update();

            // Ensure the operation succeeded
            assert!(
                result.is_ok(),
                "Tool call '{tool_name}' failed during benchmark"
            );
        }

        Self::calculate_benchmark_results(&durations, &memory_tracker)
    }

    /// Benchmark a specific resource read
    ///
    /// # Panics
    /// Panics if the resource read fails during benchmarking
    pub async fn benchmark_resource(&self, uri: &str) -> BenchmarkResults {
        let mut durations = Vec::new();
        let mut memory_tracker = MemoryTracker::new();

        memory_tracker.start();

        for _ in 0..self.config.iterations {
            let start = Instant::now();

            let result = self.harness.read_resource(uri).await;

            let duration = start.elapsed();
            durations.push(duration);

            memory_tracker.update();

            // Ensure the operation succeeded
            assert!(
                result.is_ok(),
                "Resource read '{uri}' failed during benchmark"
            );
        }

        Self::calculate_benchmark_results(&durations, &memory_tracker)
    }

    /// Benchmark a specific prompt call
    ///
    /// # Panics
    /// Panics if the prompt call fails during benchmarking
    pub async fn benchmark_prompt(
        &self,
        prompt_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> BenchmarkResults {
        let mut durations = Vec::new();
        let mut memory_tracker = MemoryTracker::new();

        memory_tracker.start();

        for _ in 0..self.config.iterations {
            let start = Instant::now();

            let result = self
                .harness
                .get_prompt(prompt_name, arguments.clone())
                .await;

            let duration = start.elapsed();
            durations.push(duration);

            memory_tracker.update();

            // Ensure the operation succeeded
            assert!(
                result.is_ok(),
                "Prompt call '{prompt_name}' failed during benchmark"
            );
        }

        Self::calculate_benchmark_results(&durations, &memory_tracker)
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn calculate_benchmark_results(
        durations: &[Duration],
        memory_tracker: &MemoryTracker,
    ) -> BenchmarkResults {
        if durations.is_empty() {
            return BenchmarkResults::default();
        }

        let total_duration: Duration = durations.iter().sum();
        let average_duration = total_duration / u32::try_from(durations.len()).unwrap_or(1);

        let min_duration = durations.iter().min().copied().unwrap_or(Duration::ZERO);
        let max_duration = durations.iter().max().copied().unwrap_or(Duration::ZERO);

        // Calculate percentiles
        let mut sorted_durations = durations.to_owned();
        sorted_durations.sort();

        let p50_index = (sorted_durations.len() * 50) / 100;
        let p95_index = (sorted_durations.len() * 95) / 100;
        let p99_index = (sorted_durations.len() * 99) / 100;

        let p50 = sorted_durations
            .get(p50_index)
            .copied()
            .unwrap_or(Duration::ZERO);
        let p95 = sorted_durations
            .get(p95_index)
            .copied()
            .unwrap_or(Duration::ZERO);
        let p99 = sorted_durations
            .get(p99_index)
            .copied()
            .unwrap_or(Duration::ZERO);

        BenchmarkResults {
            iterations: durations.len(),
            average_duration,
            min_duration,
            max_duration,
            p50_duration: p50,
            p95_duration: p95,
            p99_duration: p99,
            memory_usage: memory_tracker.get_memory_usage(),
        }
    }
}

/// Results for benchmark tests
#[derive(Debug, Clone, Default)]
pub struct BenchmarkResults {
    pub iterations: usize,
    pub average_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub p50_duration: Duration,
    pub p95_duration: Duration,
    pub p99_duration: Duration,
    pub memory_usage: Option<usize>,
}

impl BenchmarkResults {
    /// Get the operations per second
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn operations_per_second(&self) -> f64 {
        if self.average_duration.as_nanos() > 0 {
            1_000_000_000.0 / self.average_duration.as_nanos() as f64
        } else {
            0.0
        }
    }

    /// Check if the benchmark meets performance requirements
    #[must_use]
    pub fn meets_requirements(
        &self,
        max_average_duration: Duration,
        min_ops_per_second: f64,
    ) -> bool {
        self.average_duration <= max_average_duration
            && self.operations_per_second() >= min_ops_per_second
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_performance_test_runner() {
        let runner = McpPerformanceTestRunner::new();

        // Test tool performance
        let results = runner.test_tool_performance("get_inbox", None).await;
        assert!(results.success_count > 0);
        assert!(results.average_duration > Duration::ZERO);

        // Test resource performance
        let results = runner.test_resource_performance("things://inbox").await;
        assert!(results.success_count > 0);

        // Test prompt performance
        let results = runner
            .test_prompt_performance("task_review", Some(json!({"task_title": "Test"})))
            .await;
        assert!(results.success_count > 0);
    }

    #[tokio::test]
    async fn test_concurrent_performance() {
        let runner = McpPerformanceTestRunner::new();

        let results = runner.test_concurrent_performance().await;
        assert!(results.success_count > 0);
        assert!(results.operations_per_second > 0.0);
    }

    #[tokio::test]
    async fn test_comprehensive_performance() {
        let runner = McpPerformanceTestRunner::new();

        let results = runner.run_comprehensive_tests().await;
        assert!(results.tool_performance.success_count > 0);
        assert!(results.resource_performance.success_count > 0);
        assert!(results.prompt_performance.success_count > 0);

        let score = results.performance_score();
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[tokio::test]
    async fn test_benchmark() {
        let benchmark = McpBenchmark::new();

        // Test tool benchmark
        let results = benchmark.benchmark_tool("get_inbox", None).await;
        assert!(results.iterations > 0);
        assert!(results.average_duration > Duration::ZERO);
        assert!(results.operations_per_second() > 0.0);

        // Test resource benchmark
        let results = benchmark.benchmark_resource("things://inbox").await;
        assert!(results.iterations > 0);

        // Test prompt benchmark
        let results = benchmark
            .benchmark_prompt("task_review", Some(json!({"task_title": "Test"})))
            .await;
        assert!(results.iterations > 0);
    }

    #[tokio::test]
    async fn test_benchmark_requirements() {
        let benchmark = McpBenchmark::new();

        let results = benchmark.benchmark_tool("get_inbox", None).await;

        // Test that it meets reasonable requirements
        let meets_requirements = results.meets_requirements(Duration::from_secs(1), 1.0);
        assert!(meets_requirements);
    }

    #[tokio::test]
    async fn test_memory_tracker() {
        let mut tracker = MemoryTracker::new();
        tracker.start();
        tracker.update();

        // Memory usage should be available
        let usage = tracker.get_memory_usage();
        assert!(usage.is_some());
    }

    #[tokio::test]
    async fn test_performance_config() {
        let config = PerformanceTestConfig {
            max_duration: Duration::from_millis(500),
            iterations: 5,
            run_concurrent: false,
            track_memory: true,
        };

        let runner = McpPerformanceTestRunner::with_config(config);
        let results = runner.test_tool_performance("get_inbox", None).await;

        assert_eq!(results.success_count + results.failure_count, 5);
    }
}
