//! Load testing for MCP server
//!
//! Tests MCP server performance under various load conditions including
//! concurrent requests, sustained load, and stress testing.

use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use things3_cli::mcp::test_harness::McpTestHarness;
use tokio::task::JoinSet;

/// Test MCP server with concurrent tool calls
#[tokio::test]
async fn load_test_concurrent_tool_calls() {
    let harness = Arc::new(McpTestHarness::new());
    let concurrent_requests = 10;

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    for i in 0..concurrent_requests {
        let harness_clone = Arc::clone(&harness);
        tasks.spawn(async move {
            let args = json!({"limit": 10});
            let result = harness_clone.call_tool("get_inbox", Some(args)).await;
            (i, result.is_error)
        });
    }

    let mut error_count = 0;
    while let Some(result) = tasks.join_next().await {
        let (_, is_error) = result.unwrap();
        if is_error {
            error_count += 1;
        }
    }

    let duration = start.elapsed();
    println!(
        "Concurrent requests ({}) completed in {:?}, errors: {}",
        concurrent_requests, duration, error_count
    );

    assert_eq!(error_count, 0, "All concurrent requests should succeed");
    assert!(
        duration.as_secs() < 10,
        "Concurrent requests should complete in under 10 seconds"
    );
}

/// Test sustained load over time
#[tokio::test]
async fn load_test_sustained_requests() {
    let harness = McpTestHarness::new();
    let total_requests = 50;

    let start = Instant::now();
    let mut error_count = 0;
    let mut total_duration = std::time::Duration::ZERO;

    for _ in 0..total_requests {
        let request_start = Instant::now();
        let result = harness
            .call_tool("get_inbox", Some(json!({"limit": 5})))
            .await;
        let request_duration = request_start.elapsed();
        total_duration += request_duration;

        if result.is_error {
            error_count += 1;
        }
    }

    let total_time = start.elapsed();
    let avg_response_time = total_duration / total_requests;

    println!("Sustained load test:");
    println!("  Total requests: {}", total_requests);
    println!("  Total time: {:?}", total_time);
    println!("  Average response time: {:?}", avg_response_time);
    println!("  Errors: {}", error_count);

    assert_eq!(error_count, 0, "All sustained requests should succeed");
    assert!(
        avg_response_time.as_millis() < 500,
        "Average response time should be under 500ms"
    );
}

/// Test mixed workload (different tools)
#[tokio::test]
async fn load_test_mixed_workload() {
    let harness = Arc::new(McpTestHarness::new());
    let requests_per_tool = 5;

    let tools = vec!["get_inbox", "get_today", "get_projects", "get_areas"];

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    for tool in &tools {
        for _ in 0..requests_per_tool {
            let harness_clone = Arc::clone(&harness);
            let tool_name = tool.to_string();
            tasks.spawn(async move {
                let args = if tool_name == "get_areas" {
                    None
                } else {
                    Some(json!({"limit": 5}))
                };
                harness_clone.call_tool(&tool_name, args).await.is_error
            });
        }
    }

    let mut error_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap() {
            error_count += 1;
        }
    }

    let duration = start.elapsed();
    let total_requests = tools.len() * requests_per_tool;

    println!("Mixed workload test:");
    println!("  Total requests: {}", total_requests);
    println!("  Duration: {:?}", duration);
    println!("  Errors: {}", error_count);

    assert_eq!(error_count, 0, "All mixed workload requests should succeed");
}

/// Test rapid sequential requests
#[tokio::test]
async fn load_test_rapid_sequential() {
    let harness = McpTestHarness::new();
    let num_requests = 30;

    let start = Instant::now();
    let mut error_count = 0;

    for _ in 0..num_requests {
        let result = harness
            .call_tool("get_inbox", Some(json!({"limit": 1})))
            .await;
        if result.is_error {
            error_count += 1;
        }
    }

    let duration = start.elapsed();
    let avg_time = duration.as_millis() / num_requests as u128;

    println!("Rapid sequential test:");
    println!("  Requests: {}", num_requests);
    println!("  Total time: {:?}", duration);
    println!("  Average time: {}ms", avg_time);
    println!("  Errors: {}", error_count);

    assert_eq!(error_count, 0, "All rapid requests should succeed");
    assert!(
        avg_time < 200,
        "Average request time should be under 200ms, was {}ms",
        avg_time
    );
}

/// Test high concurrency stress test
#[tokio::test]
async fn load_test_high_concurrency() {
    let harness = Arc::new(McpTestHarness::new());
    let concurrent_requests = 25;

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    for _ in 0..concurrent_requests {
        let harness_clone = Arc::clone(&harness);
        tasks.spawn(async move {
            harness_clone
                .call_tool("get_inbox", Some(json!({"limit": 10})))
                .await
                .is_error
        });
    }

    let mut error_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap() {
            error_count += 1;
        }
    }

    let duration = start.elapsed();

    println!("High concurrency stress test:");
    println!("  Concurrent requests: {}", concurrent_requests);
    println!("  Duration: {:?}", duration);
    println!("  Errors: {}", error_count);

    // Allow some errors under extreme load, but not too many
    assert!(
        error_count < concurrent_requests / 5,
        "Error rate should be less than 20% under stress"
    );
}

/// Test server responsiveness under load
#[tokio::test]
async fn load_test_response_times() {
    let harness = McpTestHarness::new();
    let num_samples = 20;
    let mut response_times = Vec::new();

    for _ in 0..num_samples {
        let start = Instant::now();
        let _ = harness
            .call_tool("get_inbox", Some(json!({"limit": 5})))
            .await;
        let duration = start.elapsed();
        response_times.push(duration.as_millis());
    }

    response_times.sort();
    // Calculate percentile indices using proper formula: (percentile / 100.0) * (n - 1)
    let p50_idx = ((50.0 / 100.0) * (num_samples - 1) as f64).round() as usize;
    let p95_idx = ((95.0 / 100.0) * (num_samples - 1) as f64).round() as usize;
    let p99_idx = ((99.0 / 100.0) * (num_samples - 1) as f64).round() as usize;

    let p50 = response_times[p50_idx];
    let p95 = response_times[p95_idx];
    let p99 = response_times[p99_idx];

    println!("Response time percentiles:");
    println!("  P50: {}ms", p50);
    println!("  P95: {}ms", p95);
    println!("  P99: {}ms", p99);

    assert!(p50 < 300, "P50 response time should be under 300ms");
    assert!(p95 < 500, "P95 response time should be under 500ms");
    assert!(p99 < 1000, "P99 response time should be under 1 second");
}

/// Test search with varying query complexity
#[tokio::test]
async fn load_test_search_queries() {
    let harness = McpTestHarness::new();
    let queries = vec!["a", "test", "project", "important task"];

    let start = Instant::now();
    let mut error_count = 0;

    for query in &queries {
        for _ in 0..5 {
            let result = harness
                .call_tool("search_tasks", Some(json!({"query": query})))
                .await;
            if result.is_error {
                error_count += 1;
            }
        }
    }

    let duration = start.elapsed();
    let total_queries = queries.len() * 5;

    println!("Search load test:");
    println!("  Total queries: {}", total_queries);
    println!("  Duration: {:?}", duration);
    println!("  Errors: {}", error_count);

    assert_eq!(error_count, 0, "All search queries should succeed");
}

/// Test concurrent different operations
#[tokio::test]
async fn load_test_concurrent_mixed_operations() {
    let harness = Arc::new(McpTestHarness::new());
    let operations_per_type = 5;

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // Spawn inbox queries
    for _ in 0..operations_per_type {
        let harness_clone = Arc::clone(&harness);
        tasks.spawn(async move {
            harness_clone
                .call_tool("get_inbox", Some(json!({"limit": 10})))
                .await
                .is_error
        });
    }

    // Spawn search queries
    for _ in 0..operations_per_type {
        let harness_clone = Arc::clone(&harness);
        tasks.spawn(async move {
            harness_clone
                .call_tool("search_tasks", Some(json!({"query": "test"})))
                .await
                .is_error
        });
    }

    // Spawn project queries
    for _ in 0..operations_per_type {
        let harness_clone = Arc::clone(&harness);
        tasks.spawn(async move {
            harness_clone
                .call_tool("get_projects", Some(json!({"limit": 10})))
                .await
                .is_error
        });
    }

    let mut error_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap() {
            error_count += 1;
        }
    }

    let duration = start.elapsed();
    let total_ops = operations_per_type * 3;

    println!("Concurrent mixed operations:");
    println!("  Total operations: {}", total_ops);
    println!("  Duration: {:?}", duration);
    println!("  Errors: {}", error_count);

    assert_eq!(
        error_count, 0,
        "All concurrent mixed operations should succeed"
    );
}
