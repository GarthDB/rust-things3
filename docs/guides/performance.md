# Performance Optimization Guide

This guide covers performance optimization techniques and best practices for Rust Things.

## Table of Contents

- [Caching Strategy](#caching-strategy)
- [Performance Monitoring](#performance-monitoring)
- [Database Optimization](#database-optimization)
- [Memory Management](#memory-management)
- [Async Best Practices](#async-best-practices)
- [Benchmarking](#benchmarking)

## Caching Strategy

### Enable Caching

```rust
use things_core::{ThingsDatabase, ThingsCache, CacheConfig};

// Create cache with custom configuration
let cache_config = CacheConfig {
    max_capacity: 1000,
    ttl: Duration::from_secs(300), // 5 minutes
    tti: Duration::from_secs(60),  // 1 minute
};
let cache = ThingsCache::new(cache_config);

// Use cache for frequently accessed data
let tasks = cache.get_tasks("inbox:10", || async {
    db.get_inbox(Some(10)).await
}).await?;
```

### Cache Key Strategy

```rust
use things_core::cache::keys;

// Use predefined cache keys
let inbox_key = keys::inbox(Some(10));
let today_key = keys::today(None);
let search_key = keys::search("meeting", Some(5));
```

### Cache Statistics

```rust
// Monitor cache performance
let stats = cache.get_stats();
println!("Cache hit rate: {:.2}%", stats.hit_rate * 100.0);
println!("Total entries: {}", stats.entries);
```

## Performance Monitoring

### Enable Performance Monitoring

```rust
use things_core::{ThingsDatabase, PerformanceMonitor};

let monitor = PerformanceMonitor::new_default();

// Monitor operations
let timer = monitor.start_operation("get_inbox");
let tasks = db.get_inbox(Some(10)).await?;
timer.success();

// Get performance statistics
let stats = monitor.get_all_stats();
let summary = monitor.get_summary();
```

### System Metrics

```rust
// Get current system metrics
let metrics = monitor.get_system_metrics()?;
println!("Memory usage: {:.2} MB", metrics.memory_usage_mb);
println!("CPU usage: {:.2}%", metrics.cpu_usage_percent);
```

### Operation Timing

```rust
// Time specific operations
let timer = monitor.start_operation("search_tasks");
let results = db.search_tasks("meeting", Some(10)).await?;
timer.success();

// Get operation statistics
let op_stats = monitor.get_operation_stats("search_tasks");
if let Some(stats) = op_stats {
    println!("Average duration: {:?}", stats.average_duration);
    println!("Success rate: {:.2}%", stats.success_rate * 100.0);
}
```

## Database Optimization

### Connection Pooling

```rust
// Reuse database connections
let db = Arc::new(ThingsDatabase::new(&config)?);

// Use in multiple async tasks
let db_clone = db.clone();
let task1 = async move {
    db_clone.get_inbox(Some(10)).await
};

let db_clone = db.clone();
let task2 = async move {
    db_clone.get_today(None).await
};

let (inbox, today) = tokio::join!(task1, task2);
```

### Query Optimization

```rust
// Use appropriate limits
let tasks = db.get_inbox(Some(10)).await?; // Limit results

// Use specific queries instead of general ones
let projects = db.get_projects(Some(area_uuid)).await?; // Filter by area
```

### Batch Operations

```rust
// Use bulk operations when possible
let tasks = vec![
    Task { title: "Task 1".to_string(), .. },
    Task { title: "Task 2".to_string(), .. },
];

// Bulk create instead of individual creates
for task in tasks {
    db.create_task(&task).await?;
}
```

## Memory Management

### Limit Result Sets

```rust
// Always use limits for large datasets
let tasks = db.get_inbox(Some(100)).await?; // Limit to 100 tasks
let projects = db.get_projects(None).await?; // No limit for projects (usually small)
```

### Use Streaming for Large Exports

```rust
// For large data exports, use streaming
let mut tasks = Vec::new();
let mut offset = 0;
const BATCH_SIZE: usize = 100;

loop {
    let batch = db.get_inbox_with_offset(Some(BATCH_SIZE), offset).await?;
    if batch.is_empty() {
        break;
    }
    tasks.extend(batch);
    offset += BATCH_SIZE;
}
```

### Cache Memory Usage

```rust
// Monitor cache memory usage
let stats = cache.get_stats();
println!("Cache entries: {}", stats.entries);

// Clear cache if needed
if stats.entries > 1000 {
    cache.invalidate_all().await;
}
```

## Async Best Practices

### Use Async/Await Properly

```rust
// Good: Use async/await
async fn get_tasks() -> Result<Vec<Task>> {
    let db = ThingsDatabase::new(&config)?;
    let tasks = db.get_inbox(Some(10)).await?;
    Ok(tasks)
}

// Bad: Blocking in async context
async fn get_tasks_bad() -> Result<Vec<Task>> {
    let db = ThingsDatabase::new(&config)?;
    let tasks = std::thread::spawn(|| {
        // This blocks the async runtime
        db.get_inbox(Some(10)).await
    }).join().unwrap()?;
    Ok(tasks)
}
```

### Parallel Operations

```rust
// Run independent operations in parallel
let (inbox, today, projects) = tokio::join!(
    db.get_inbox(Some(10)),
    db.get_today(None),
    db.get_projects(None)
);

let inbox = inbox?;
let today = today?;
let projects = projects?;
```

### Error Handling

```rust
// Use proper error handling
match db.get_inbox(Some(10)).await {
    Ok(tasks) => {
        println!("Found {} tasks", tasks.len());
    }
    Err(ThingsError::DatabaseError(e)) => {
        eprintln!("Database error: {}", e);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

## Benchmarking

### Built-in Benchmarks

```rust
// Run benchmarks
cargo bench

// Run specific benchmark
cargo bench --bench database_bench
```

### Custom Benchmarks

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_inbox_query(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let db = rt.block_on(ThingsDatabase::new(&config)).unwrap();
    
    c.bench_function("inbox_query", |b| {
        b.to_async(&rt).iter(|| async {
            db.get_inbox(Some(10)).await
        })
    });
}

criterion_group!(benches, benchmark_inbox_query);
criterion_main!(benches);
```

### Performance Testing

```rust
// Test performance under load
#[tokio::test]
async fn test_performance_under_load() {
    let db = ThingsDatabase::new(&config).await?;
    let monitor = PerformanceMonitor::new_default();
    
    // Run 100 concurrent queries
    let handles: Vec<_> = (0..100)
        .map(|_| {
            let db = db.clone();
            let monitor = monitor.clone();
            tokio::spawn(async move {
                let timer = monitor.start_operation("concurrent_query");
                let result = db.get_inbox(Some(10)).await;
                timer.success();
                result
            })
        })
        .collect();
    
    // Wait for all queries to complete
    for handle in handles {
        handle.await.unwrap()?;
    }
    
    // Check performance statistics
    let stats = monitor.get_operation_stats("concurrent_query");
    assert!(stats.is_some());
    let stats = stats.unwrap();
    assert!(stats.average_duration < Duration::from_millis(100));
}
```

## Monitoring and Alerting

### Performance Alerts

```rust
// Set up performance monitoring
let monitor = PerformanceMonitor::new_default();

// Check performance periodically
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        
        let stats = monitor.get_summary();
        if stats.average_operation_duration > Duration::from_millis(500) {
            eprintln!("Warning: High average operation duration: {:?}", 
                     stats.average_operation_duration);
        }
    }
});
```

### Resource Monitoring

```rust
// Monitor system resources
let monitor = PerformanceMonitor::new_default();

tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        
        let metrics = monitor.get_system_metrics().unwrap();
        if metrics.memory_usage_mb > 1000.0 {
            eprintln!("Warning: High memory usage: {:.2} MB", metrics.memory_usage_mb);
        }
        
        if metrics.cpu_usage_percent > 80.0 {
            eprintln!("Warning: High CPU usage: {:.2}%", metrics.cpu_usage_percent);
        }
    }
});
```

## Best Practices Summary

1. **Use caching** for frequently accessed data
2. **Monitor performance** with built-in tools
3. **Limit result sets** to avoid memory issues
4. **Use async/await** properly
5. **Run operations in parallel** when possible
6. **Handle errors gracefully**
7. **Benchmark critical paths**
8. **Monitor system resources**
9. **Use appropriate data structures**
10. **Profile before optimizing**
