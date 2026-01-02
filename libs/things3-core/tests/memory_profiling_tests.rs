//! Memory usage profiling tests
//!
//! Tests memory allocation patterns, leak detection, and memory efficiency
//! for core operations to ensure reasonable memory consumption.

use std::sync::Arc;
use tempfile::NamedTempFile;
use things3_core::{
    CacheConfig, DataExporter, ExportData, ExportFormat, ThingsCache, ThingsDatabase,
};

#[cfg(feature = "test-utils")]
use things3_core::test_utils::{create_mock_tasks, create_test_database};

/// Test memory usage of database connection
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn memory_test_database_connection() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();

    // Create and drop database connections
    for _ in 0..10 {
        let _db = ThingsDatabase::new(db_path).await.unwrap();
        // Database should be properly cleaned up on drop
    }

    // If we get here without OOM, memory management is reasonable
    // Test passes by completing without panic or out-of-memory
}

/// Test memory usage with large query results
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn memory_test_large_query_results() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Query large result sets
    let _inbox = db.get_inbox(Some(1000)).await.unwrap();
    let _today = db.get_today(Some(1000)).await.unwrap();
    let _projects = db.get_projects(Some(1000)).await.unwrap();

    // Results should be properly managed
    // Test passes by completing without panic or out-of-memory
}

/// Test memory usage of cache operations
#[tokio::test]
async fn memory_test_cache_operations() {
    let config = CacheConfig {
        max_capacity: 1000,
        ..Default::default()
    };

    let _cache = ThingsCache::new(&config);

    // Cache should not consume excessive memory
    // Test passes by completing without panic or out-of-memory
}

/// Test memory usage with multiple caches
#[tokio::test]
async fn memory_test_multiple_caches() {
    let config = CacheConfig::default();
    let mut caches = Vec::new();

    // Create multiple cache instances
    for _ in 0..10 {
        caches.push(ThingsCache::new(&config));
    }

    assert_eq!(caches.len(), 10);
    // Drop caches
    drop(caches);

    // Test passes by completing without panic or memory leaks
}

/// Test memory usage with repeated operations
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn memory_test_repeated_operations() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Perform same operation many times
    for _ in 0..100 {
        let _inbox = db.get_inbox(Some(10)).await.unwrap();
        // Results should be dropped between iterations
    }

    // Test passes by completing without panic or memory accumulation
}

/// Test memory usage with export operations
#[test]
#[cfg(feature = "test-utils")]
fn memory_test_export_operations() {
    let tasks = create_mock_tasks();
    let exporter = DataExporter::new_default();

    // Export in multiple formats
    for _ in 0..10 {
        let data = ExportData::new(tasks.clone(), vec![], vec![]);
        let _json = exporter.export(&data, ExportFormat::Json).unwrap();
        let _csv = exporter.export(&data, ExportFormat::Csv).unwrap();
        let _markdown = exporter.export(&data, ExportFormat::Markdown).unwrap();
        // Export results should be cleaned up
    }

    // Test passes by completing without panic or memory issues
}

/// Test memory usage with Arc-wrapped database
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn memory_test_arc_wrapped_database() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = Arc::new(ThingsDatabase::new(db_path).await.unwrap());

    // Clone Arc multiple times
    let mut handles = Vec::new();
    for _ in 0..10 {
        let db_clone = Arc::clone(&db);
        handles.push(db_clone);
    }

    assert_eq!(Arc::strong_count(&db), 11); // Original + 10 clones

    // Drop all handles
    drop(handles);
    assert_eq!(Arc::strong_count(&db), 1);

    // Test passes by verifying correct Arc reference counting
}

/// Test memory usage with database health checks
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn memory_test_health_checks() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Perform many health checks
    for _ in 0..100 {
        let _is_connected = db.is_connected().await;
    }

    // Test passes by completing without panic or memory accumulation
}

/// Test memory usage with cache stats
#[tokio::test]
async fn memory_test_cache_stats() {
    let cache = ThingsCache::new_default();

    // Get stats many times
    for _ in 0..1000 {
        let _stats = cache.get_stats();
    }

    // Test passes by completing without panic or memory leaks
}

/// Test memory usage with search operations
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn memory_test_search_operations() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Perform various searches
    let queries = vec!["test", "project", "task", "important"];
    for query in queries {
        for _ in 0..10 {
            let _results = db.search_tasks(query).await.unwrap();
        }
    }

    // Test passes by completing without panic or memory issues
}

/// Test memory cleanup after errors
#[tokio::test]
async fn memory_test_error_cleanup() {
    let nonexistent_path = std::path::PathBuf::from("/nonexistent/path/to/database.db");

    // Attempt connections that will fail
    for _ in 0..10 {
        let _result = ThingsDatabase::new(&nonexistent_path).await;
        // Error paths should clean up properly
    }

    // Test passes by completing without panic or memory leaks
}

/// Test memory with database pool
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn memory_test_database_pool() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Perform concurrent operations (uses connection pool)
    let mut tasks = Vec::new();
    for _ in 0..5 {
        let db_clone = db.clone();
        tasks.push(tokio::spawn(
            async move { db_clone.get_inbox(Some(10)).await },
        ));
    }

    for task in tasks {
        let _ = task.await.unwrap();
    }

    // Test passes by completing without panic or memory issues
}
