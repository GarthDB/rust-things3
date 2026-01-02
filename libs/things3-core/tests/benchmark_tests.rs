//! Benchmark tests for critical performance paths
//!
//! Tests database queries, cache operations, and data processing
//! to ensure acceptable performance and identify bottlenecks.

use std::time::Instant;
use tempfile::NamedTempFile;
use things3_core::{CacheConfig, ThingsCache, ThingsDatabase};

#[cfg(feature = "test-utils")]
use things3_core::test_utils::create_test_database;

/// Benchmark database connection establishment
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn benchmark_database_connection() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();

    let start = Instant::now();
    let _db = ThingsDatabase::new(db_path).await.unwrap();
    let duration = start.elapsed();

    println!("Database connection time: {:?}", duration);
    assert!(
        duration.as_millis() < 1000,
        "Database connection should take less than 1 second, took {:?}",
        duration
    );
}

/// Benchmark inbox query performance
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn benchmark_get_inbox() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let start = Instant::now();
    let _tasks = db.get_inbox(Some(100)).await.unwrap();
    let duration = start.elapsed();

    println!("Inbox query time (100 items): {:?}", duration);
    assert!(
        duration.as_millis() < 500,
        "Inbox query should take less than 500ms, took {:?}",
        duration
    );
}

/// Benchmark today tasks query performance
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn benchmark_get_today() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let start = Instant::now();
    let _tasks = db.get_today(Some(100)).await.unwrap();
    let duration = start.elapsed();

    println!("Today query time (100 items): {:?}", duration);
    assert!(
        duration.as_millis() < 500,
        "Today query should take less than 500ms, took {:?}",
        duration
    );
}

/// Benchmark projects query performance
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn benchmark_get_projects() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let start = Instant::now();
    let _projects = db.get_projects(Some(100)).await.unwrap();
    let duration = start.elapsed();

    println!("Projects query time (100 items): {:?}", duration);
    assert!(
        duration.as_millis() < 500,
        "Projects query should take less than 500ms, took {:?}",
        duration
    );
}

/// Benchmark search query performance
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn benchmark_search_tasks() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let start = Instant::now();
    let _results = db.search_tasks("test").await.unwrap();
    let duration = start.elapsed();

    println!("Search query time: {:?}", duration);
    assert!(
        duration.as_millis() < 1000,
        "Search should take less than 1 second, took {:?}",
        duration
    );
}

/// Benchmark cache creation
#[tokio::test]
async fn benchmark_cache_creation() {
    let config = CacheConfig::default();

    let start = Instant::now();
    let _cache = ThingsCache::new(&config);
    let duration = start.elapsed();

    println!("Cache creation time: {:?}", duration);
    assert!(
        duration.as_millis() < 100,
        "Cache creation should take less than 100ms, took {:?}",
        duration
    );
}

/// Benchmark sequential database queries
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn benchmark_sequential_queries() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let start = Instant::now();
    for _ in 0..10 {
        let _ = db.get_inbox(Some(10)).await.unwrap();
    }
    let duration = start.elapsed();

    let avg_time = duration.as_millis() / 10;
    println!("Average sequential query time: {}ms", avg_time);
    assert!(
        avg_time < 100,
        "Average query should take less than 100ms, took {}ms",
        avg_time
    );
}

/// Benchmark database health check
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn benchmark_health_check() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let start = Instant::now();
    let _is_connected = db.is_connected().await;
    let duration = start.elapsed();

    println!("Health check time: {:?}", duration);
    assert!(
        duration.as_millis() < 100,
        "Health check should take less than 100ms, took {:?}",
        duration
    );
}

/// Benchmark multiple queries with different types
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn benchmark_mixed_queries() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let start = Instant::now();
    let _ = db.get_inbox(Some(10)).await.unwrap();
    let _ = db.get_today(Some(10)).await.unwrap();
    let _ = db.get_projects(Some(10)).await.unwrap();
    let _ = db.get_areas().await.unwrap();
    let duration = start.elapsed();

    println!("Mixed queries time: {:?}", duration);
    assert!(
        duration.as_millis() < 2000,
        "Mixed queries should take less than 2 seconds, took {:?}",
        duration
    );
}

/// Benchmark cache stats computation
#[tokio::test]
async fn benchmark_cache_stats() {
    let cache = ThingsCache::new_default();

    let start = Instant::now();
    let _stats = cache.get_stats();
    let duration = start.elapsed();

    println!("Cache stats time: {:?}", duration);
    assert!(
        duration.as_micros() < 1000,
        "Cache stats should take less than 1ms, took {:?}",
        duration
    );
}
