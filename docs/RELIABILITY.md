# Reliability Guide

Comprehensive guide to building reliable applications with `rust-things3`.

## Table of Contents

1. [Overview](#overview)
2. [Connection Pool Management](#connection-pool-management)
3. [Error Recovery Patterns](#error-recovery-patterns)
4. [Concurrent Access Patterns](#concurrent-access-patterns)
5. [Resource Management](#resource-management)
6. [Graceful Degradation](#graceful-degradation)
7. [Testing for Reliability](#testing-for-reliability)
8. [Best Practices](#best-practices)
9. [Common Pitfalls](#common-pitfalls)

## Overview

`rust-things3` is designed for reliability with:
- **Async/await** for efficient non-blocking I/O
- **Connection pooling** with configurable limits
- **Transactional operations** for data consistency
- **Thread-safe** database access (`Send + Sync`)
- **Comprehensive error handling** with recovery strategies
- **Resource cleanup** via RAII and `Drop`

## Connection Pool Management

### Default Configuration

The library uses SQLx connection pooling with sensible defaults:

```rust
use things3_core::{ThingsDatabase, DatabasePoolConfig};
use std::time::Duration;

// Default configuration
let config = DatabasePoolConfig {
    max_connections: 10,        // Maximum concurrent connections
    min_connections: 1,         // Keep alive for fast response
    connect_timeout: Duration::from_secs(30),
    idle_timeout: Duration::from_secs(600),     // 10 minutes
    max_lifetime: Duration::from_secs(1800),    // 30 minutes
    test_before_acquire: true,  // Ensure connection health
    sqlite_optimizations: Default::default(),
};
```

### Connection Pool Tuning

#### For Read-Heavy Workloads

```rust
let config = DatabasePoolConfig {
    max_connections: 20,        // More connections for parallelism
    min_connections: 5,         // Keep connections warm
    idle_timeout: Duration::from_secs(900),    // 15 minutes
    test_before_acquire: false, // Reduce overhead for high throughput
    ..Default::default()
};
```

#### For Write-Heavy Workloads

```rust
let config = DatabasePoolConfig {
    max_connections: 5,         // Limit to reduce contention
    min_connections: 2,         // Keep connections available
    connect_timeout: Duration::from_secs(60), // Longer timeout for writes
    ..Default::default()
};
```

#### For Resource-Constrained Environments

```rust
let config = DatabasePoolConfig {
    max_connections: 3,         // Minimal connections
    min_connections: 1,         // Single persistent connection
    idle_timeout: Duration::from_secs(300),    // 5 minutes
    max_lifetime: Duration::from_secs(900),    // 15 minutes
    ..Default::default()
};
```

### SQLite Optimizations

```rust
use things3_core::SqliteOptimizations;

let optimizations = SqliteOptimizations {
    wal_autocheckpoint: 1000,   // Checkpoint after 1000 pages
    cache_size: -2000,          // Use 2MB cache (negative = KB)
    page_size: 4096,            // 4KB pages
    busy_timeout: 5000,         // Wait 5s for locks
    synchronous: "NORMAL".to_string(),  // Balance durability/performance
    journal_mode: "WAL".to_string(),    // Write-Ahead Logging
    temp_store: "MEMORY".to_string(),   // In-memory temp storage
    mmap_size: 30000000,        // 30MB memory mapping
    enable_query_planner: true,
};

let config = DatabasePoolConfig {
    sqlite_optimizations: optimizations,
    ..Default::default()
};
```

### Monitoring Pool Health

```rust
use things3_core::ThingsDatabase;

async fn monitor_pool(db: &ThingsDatabase) {
    let health = db.get_comprehensive_health().await.unwrap();
    
    println!("Pool Status:");
    println!("  Connections: {}/{}", 
        health.pool_metrics.num_connections,
        health.pool_metrics.max_connections
    );
    println!("  Idle: {}", health.pool_metrics.idle_connections);
    
    if health.pool_health.connections_at_limit {
        eprintln!("⚠️ Pool at capacity - consider increasing max_connections");
    }
    
    if health.pool_health.high_idle_connections {
        eprintln!("⚠️ Many idle connections - consider reducing max_connections");
    }
}
```

## Error Recovery Patterns

### Transactional Operations

All database modifications are wrapped in transactions for automatic rollback on error:

```rust
use things3_core::{ThingsDatabase, CreateTaskRequest};

async fn reliable_task_creation(db: &ThingsDatabase) -> Result<String, Box<dyn std::error::Error>> {
    // This automatically rolls back on error
    let task_uuid = db.create_task(CreateTaskRequest {
        title: "Important Task".to_string(),
        ..Default::default()
    }).await?;
    
    Ok(task_uuid)
}
```

### Manual Transaction Control

For complex multi-step operations:

```rust
use sqlx::Transaction;

async fn complex_operation(db: &ThingsDatabase) -> Result<(), Box<dyn std::error::Error>> {
    let mut tx: Transaction<'_, sqlx::Sqlite> = db.pool().begin().await?;
    
    // Step 1: Create project
    sqlx::query("INSERT INTO TMTask (...) VALUES (...)")
        .execute(&mut *tx)
        .await?;
    
    // Step 2: Create child tasks
    sqlx::query("INSERT INTO TMTask (...) VALUES (...)")
        .execute(&mut *tx)
        .await?;
    
    // Commit or rollback automatically on drop
    tx.commit().await?;
    
    Ok(())
}
```

### Retry with Exponential Backoff

```rust
use std::time::Duration;
use tokio::time::sleep;

async fn retry_with_backoff<F, T, E>(
    mut operation: F,
    max_retries: u32,
) -> Result<T, E>
where
    F: FnMut() -> futures::future::BoxFuture<'static, Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;
                if attempt >= max_retries {
                    eprintln!("Operation failed after {max_retries} attempts: {e}");
                    return Err(e);
                }
                
                let backoff = Duration::from_millis(100 * 2_u64.pow(attempt));
                eprintln!("Attempt {attempt} failed: {e}. Retrying in {backoff:?}...");
                sleep(backoff).await;
            }
        }
    }
}

// Usage
async fn robust_search(db: &ThingsDatabase, query: String) {
    let result = retry_with_backoff(
        || Box::pin(async {
            db.search_tasks(&query).await
        }),
        3, // max retries
    ).await;
}
```

### Error Context Enrichment

```rust
use things3_core::{ThingsError, ThingsDatabase};

async fn enrich_errors(db: &ThingsDatabase, task_uuid: &str) 
    -> Result<(), Box<dyn std::error::Error>> 
{
    match db.get_task_by_uuid(&uuid::Uuid::parse_str(task_uuid)?).await {
        Ok(task) => Ok(()),
        Err(ThingsError::TaskNotFound { uuid }) => {
            Err(format!("Task {uuid} not found. It may have been deleted or moved to trash.").into())
        }
        Err(ThingsError::DatabaseNotFound { path }) => {
            Err(format!("Things 3 database not found at {path}. Is Things 3 installed?").into())
        }
        Err(e) => Err(e.into()),
    }
}
```

## Concurrent Access Patterns

### Thread-Safe Database Access

`ThingsDatabase` implements `Clone`, `Send`, and `Sync`:

```rust
use things3_core::ThingsDatabase;
use std::sync::Arc;
use tokio::task::JoinSet;

async fn concurrent_operations(db: Arc<ThingsDatabase>) {
    let mut join_set = JoinSet::new();
    
    // Spawn multiple concurrent tasks
    for i in 0..10 {
        let db_clone = Arc::clone(&db);
        join_set.spawn(async move {
            db_clone.get_inbox(Some(100)).await
        });
    }
    
    // Wait for all to complete
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(tasks)) => println!("Got {} tasks", tasks.len()),
            Ok(Err(e)) => eprintln!("Database error: {e}"),
            Err(e) => eprintln!("Task join error: {e}"),
        }
    }
}
```

### Read-Heavy Concurrent Pattern

```rust
use std::sync::Arc;
use tokio::task;

async fn parallel_reads(db: Arc<ThingsDatabase>) {
    let (inbox, today, projects, areas) = tokio::join!(
        task::spawn({
            let db = Arc::clone(&db);
            async move { db.get_inbox(None).await }
        }),
        task::spawn({
            let db = Arc::clone(&db);
            async move { db.get_today(None).await }
        }),
        task::spawn({
            let db = Arc::clone(&db);
            async move { db.get_all_projects().await }
        }),
        task::spawn({
            let db = Arc::clone(&db);
            async move { db.get_all_areas().await }
        }),
    );
    
    // Handle results
    let inbox_tasks = inbox.unwrap().unwrap();
    let today_tasks = today.unwrap().unwrap();
    let project_list = projects.unwrap().unwrap();
    let area_list = areas.unwrap().unwrap();
}
```

### Write Serialization Pattern

```rust
use tokio::sync::Mutex;
use std::sync::Arc;

struct WriteSynchronizer {
    db: Arc<ThingsDatabase>,
    write_lock: Arc<Mutex<()>>,
}

impl WriteSynchronizer {
    async fn synchronized_write(&self, request: CreateTaskRequest) 
        -> Result<String, ThingsError> 
    {
        let _guard = self.write_lock.lock().await;
        self.db.create_task(request).await
    }
}
```

## Resource Management

### RAII and Drop

Resources are automatically cleaned up via `Drop`:

```rust
use things3_core::ThingsDatabase;

async fn automatic_cleanup() {
    // Database opens connection pool
    let db = ThingsDatabase::new(path).await.unwrap();
    
    // Use database
    let tasks = db.get_inbox(None).await.unwrap();
    
    // Connection pool automatically closed when `db` goes out of scope
} // <- pool.close() called here automatically
```

### Explicit Resource Cleanup

```rust
async fn explicit_cleanup(db: ThingsDatabase) {
    // Perform operations
    let _ = db.get_all_tasks().await;
    
    // Explicitly close pool
    db.pool().close().await;
    
    // Any further operations will fail gracefully
}
```

### Temporary File Management

```rust
use tempfile::NamedTempFile;

async fn test_with_cleanup() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    
    // Create test database
    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();
    
    // Perform operations
    let tasks = db.get_inbox(None).await.unwrap();
    
    // Clean up
    drop(db);  // Close connections
    drop(temp_file);  // Delete temporary file
}
```

### Long-Running Process Pattern

```rust
use tokio::signal;

async fn long_running_service(db: ThingsDatabase) {
    println!("Starting service...");
    
    // Set up graceful shutdown
    let shutdown = tokio::spawn(async {
        signal::ctrl_c().await.unwrap();
        println!("Shutdown signal received");
    });
    
    // Main service loop
    loop {
        tokio::select! {
            _ = shutdown => {
                println!("Shutting down gracefully...");
                db.pool().close().await;
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs(10)) => {
                // Periodic work
                let health = db.get_comprehensive_health().await.unwrap();
                println!("Health check: {} connections", health.pool_metrics.num_connections);
            }
        }
    }
}
```

## Graceful Degradation

### Fallback to Cache

```rust
use things3_core::{ThingsDatabase, ThingsError};

async fn get_tasks_with_fallback(db: &ThingsDatabase, cache: &mut Option<Vec<Task>>) 
    -> Result<Vec<Task>, ThingsError> 
{
    match db.get_inbox(None).await {
        Ok(tasks) => {
            *cache = Some(tasks.clone());
            Ok(tasks)
        }
        Err(e) => {
            eprintln!("Database error: {e}");
            if let Some(cached) = cache {
                println!("Returning cached data");
                Ok(cached.clone())
            } else {
                Err(e)
            }
        }
    }
}
```

### Circuit Breaker Pattern

```rust
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::sync::Arc;

struct CircuitBreaker {
    failure_count: AtomicU32,
    is_open: AtomicBool,
    failure_threshold: u32,
}

impl CircuitBreaker {
    fn new(threshold: u32) -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            is_open: AtomicBool::new(false),
            failure_threshold: threshold,
        }
    }
    
    fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.is_open.store(false, Ordering::Relaxed);
    }
    
    fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        if failures >= self.failure_threshold {
            self.is_open.store(true, Ordering::Relaxed);
            eprintln!("⚠️ Circuit breaker OPEN after {failures} failures");
        }
    }
    
    fn is_open(&self) -> bool {
        self.is_open.load(Ordering::Relaxed)
    }
}

async fn operation_with_circuit_breaker(
    db: &ThingsDatabase,
    breaker: &CircuitBreaker,
) -> Result<Vec<Task>, ThingsError> {
    if breaker.is_open() {
        return Err(ThingsError::unknown("Circuit breaker is open"));
    }
    
    match db.get_inbox(None).await {
        Ok(tasks) => {
            breaker.record_success();
            Ok(tasks)
        }
        Err(e) => {
            breaker.record_failure();
            Err(e)
        }
    }
}
```

### Timeout Protection

```rust
use tokio::time::{timeout, Duration};

async fn operation_with_timeout(db: &ThingsDatabase) 
    -> Result<Vec<Task>, Box<dyn std::error::Error>> 
{
    match timeout(Duration::from_secs(5), db.get_inbox(None)).await {
        Ok(Ok(tasks)) => Ok(tasks),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err("Operation timed out after 5 seconds".into()),
    }
}
```

## Testing for Reliability

### Concurrent Access Testing

```rust
use std::sync::Arc;
use tokio::task::JoinSet;

#[tokio::test]
async fn test_concurrent_reads() {
    let db = Arc::new(create_test_db().await);
    let mut join_set = JoinSet::new();
    
    // Spawn 20 concurrent readers
    for i in 0..20 {
        let db_clone = Arc::clone(&db);
        join_set.spawn(async move {
            for _ in 0..5 {
                let inbox = db_clone.get_inbox(Some(50)).await.unwrap();
                assert!(!inbox.is_empty(), "Task {} got empty inbox", i);
            }
        });
    }
    
    // All should succeed
    while let Some(result) = join_set.join_next().await {
        result.unwrap();
    }
}
```

### Error Recovery Testing

```rust
#[tokio::test]
async fn test_error_recovery() {
    let db = create_test_db().await;
    
    // Test invalid UUID
    let result = db.get_task_by_uuid(&Uuid::new_v4()).await;
    assert!(matches!(result, Err(ThingsError::TaskNotFound { .. })));
    
    // Database should still be operational
    let inbox = db.get_inbox(None).await.unwrap();
    assert!(!inbox.is_empty());
}
```

### Resource Cleanup Testing

```rust
#[tokio::test]
async fn test_resource_cleanup() {
    let start_connections = get_open_connection_count();
    
    {
        let db = create_test_db().await;
        let _ = db.get_inbox(None).await;
    } // db dropped here
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let end_connections = get_open_connection_count();
    assert_eq!(start_connections, end_connections, "Connection leak detected");
}
```

### Load Testing

```rust
#[tokio::test]
async fn test_sustained_load() {
    let db = Arc::new(create_test_db().await);
    let duration = Duration::from_secs(60);
    let start = Instant::now();
    
    let mut operations = 0;
    while start.elapsed() < duration {
        let _ = db.get_inbox(None).await.unwrap();
        operations += 1;
    }
    
    println!("Completed {} operations in {:?}", operations, duration);
    println!("Throughput: {} ops/sec", operations / 60);
}
```

## Best Practices

### 1. Use Appropriate Pool Sizing

```rust
// ✅ Good: Sized for expected load
let config = DatabasePoolConfig {
    max_connections: 10,    // Based on expected concurrency
    min_connections: 2,     // Keep some warm
    ..Default::default()
};

// ❌ Bad: Over-provisioned
let config = DatabasePoolConfig {
    max_connections: 100,   // Wastes resources
    ..Default::default()
};
```

### 2. Always Use Transactions for Multi-Step Operations

```rust
// ✅ Good: Atomic operation
async fn create_project_with_tasks(db: &ThingsDatabase) -> Result<()> {
    let mut tx = db.pool().begin().await?;
    
    sqlx::query("INSERT INTO TMTask (type=1) VALUES (...)")
        .execute(&mut *tx).await?;
    
    sqlx::query("INSERT INTO TMTask (parent_uuid=?) VALUES (...)")
        .execute(&mut *tx).await?;
    
    tx.commit().await?;
    Ok(())
}

// ❌ Bad: No transaction, partial failure possible
async fn create_project_unsafe(db: &ThingsDatabase) -> Result<()> {
    db.create_project(request1).await?;  // Might succeed
    db.create_task(request2).await?;      // Might fail
    Ok(())
}
```

### 3. Handle All Error Cases

```rust
// ✅ Good: Comprehensive error handling
match db.get_task_by_uuid(&uuid).await {
    Ok(task) => process_task(task),
    Err(ThingsError::TaskNotFound { .. }) => {
        println!("Task not found");
    }
    Err(ThingsError::DatabaseNotFound { .. }) => {
        eprintln!("Database missing");
        return Err("Please install Things 3");
    }
    Err(e) => {
        eprintln!("Unexpected error: {e}");
        return Err("Operation failed");
    }
}

// ❌ Bad: Ignoring errors
let task = db.get_task_by_uuid(&uuid).await.unwrap();  // Panics on error!
```

### 4. Clean Up Resources Explicitly in Long-Running Processes

```rust
// ✅ Good: Explicit cleanup
async fn service_loop() {
    let db = ThingsDatabase::new(path).await.unwrap();
    
    tokio::select! {
        _ = shutdown_signal() => {
            db.pool().close().await;
        }
    }
}

// ❌ Bad: Relying on drop in long-running process
async fn service_loop_unsafe() {
    let db = ThingsDatabase::new(path).await.unwrap();
    loop {
        // Never exits, connections stay open indefinitely
    }
}
```

### 5. Monitor Pool Health in Production

```rust
// ✅ Good: Regular health checks
async fn monitor_service(db: &ThingsDatabase) {
    loop {
        let health = db.get_comprehensive_health().await.unwrap();
        
        if health.pool_health.connections_at_limit {
            log::warn!("Pool at capacity");
        }
        
        if health.database_stats.task_count > 10000 {
            log::info!("Large database detected");
        }
        
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
```

## Common Pitfalls

### 1. Not Using Arc for Shared Database

```rust
// ❌ Bad: Multiple clones create multiple pools
async fn bad_sharing() {
    let db = ThingsDatabase::new(path).await.unwrap();
    
    tokio::spawn(async move {
        db.get_inbox(None).await  // Moved, original unusable
    });
    
    // Can't use db here!
}

// ✅ Good: Use Arc for sharing
async fn good_sharing() {
    let db = Arc::new(ThingsDatabase::new(path).await.unwrap());
    
    tokio::spawn({
        let db = Arc::clone(&db);
        async move {
            db.get_inbox(None).await
        }
    });
    
    // Can still use db here!
    db.get_today(None).await.unwrap();
}
```

### 2. Holding Transactions Too Long

```rust
// ❌ Bad: Long-held transaction
async fn slow_transaction(db: &ThingsDatabase) {
    let mut tx = db.pool().begin().await.unwrap();
    
    // Expensive operation while holding transaction
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    sqlx::query("UPDATE ...").execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
}

// ✅ Good: Minimize transaction scope
async fn fast_transaction(db: &ThingsDatabase) {
    // Do expensive work first
    let data = expensive_computation().await;
    
    // Quick transaction
    let mut tx = db.pool().begin().await.unwrap();
    sqlx::query("UPDATE ...").bind(data).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
}
```

### 3. Ignoring Connection Pool Exhaustion

```rust
// ❌ Bad: No error handling for pool exhaustion
async fn might_deadlock(db: Arc<ThingsDatabase>) {
    let mut tasks = vec![];
    
    // Spawn way more tasks than pool connections
    for _ in 0..100 {
        let db = Arc::clone(&db);
        tasks.push(tokio::spawn(async move {
            db.get_inbox(None).await.unwrap()  // Might panic!
        }));
    }
}

// ✅ Good: Handle gracefully
async fn handles_contention(db: Arc<ThingsDatabase>) {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(10));
    
    for _ in 0..100 {
        let db = Arc::clone(&db);
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        
        tokio::spawn(async move {
            let _permit = permit;  // Hold until task completes
            db.get_inbox(None).await
        });
    }
}
```

## Related Documentation

- [Error Handling Guide](ERROR_HANDLING.md) - Comprehensive error handling patterns
- [Performance Guide](PERFORMANCE.md) - Optimization strategies and benchmarks
- [Architecture](ARCHITECTURE.md) - System design and patterns
- [MCP Integration](MCP_INTEGRATION.md) - MCP server reliability

## Further Reading

- [SQLx Documentation](https://docs.rs/sqlx/) - Connection pooling and transactions
- [Tokio Documentation](https://tokio.rs/) - Async runtime and patterns
- [Rust Async Book](https://rust-lang.github.io/async-book/) - Async/await patterns

---

**Last Updated**: January 2026  
**For**: rust-things3 v0.5.0+

