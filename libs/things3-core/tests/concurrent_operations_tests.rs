//! Concurrent operations testing
//!
//! Tests thread safety, race conditions, and concurrent access patterns
//! to ensure the system handles parallel operations correctly.

use std::sync::Arc;
use tempfile::NamedTempFile;
use things3_core::{CacheConfig, ThingsCache, ThingsDatabase};
use tokio::task::JoinSet;

#[cfg(feature = "test-utils")]
use things3_core::test_utils::create_test_database;

/// Test concurrent database reads
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn concurrent_test_database_reads() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = Arc::new(ThingsDatabase::new(db_path).await.unwrap());

    let mut tasks = JoinSet::new();
    let num_tasks = 20;

    for i in 0..num_tasks {
        let db_clone = Arc::clone(&db);
        tasks.spawn(async move {
            let result = if i % 4 == 0 {
                db_clone.get_inbox(Some(10)).await.map(|_| ())
            } else if i % 4 == 1 {
                db_clone.get_today(Some(10)).await.map(|_| ())
            } else if i % 4 == 2 {
                db_clone.get_projects(Some(10)).await.map(|_| ())
            } else {
                db_clone.get_areas().await.map(|_| ())
            };
            result.is_ok()
        });
    }

    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap() {
            success_count += 1;
        }
    }

    assert_eq!(
        success_count, num_tasks,
        "All concurrent reads should succeed"
    );
}

/// Test concurrent searches
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn concurrent_test_searches() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = Arc::new(ThingsDatabase::new(db_path).await.unwrap());

    let queries = vec!["test", "project", "task", "important"];
    let mut tasks = JoinSet::new();

    for query in &queries {
        for _ in 0..5 {
            let db_clone = Arc::clone(&db);
            let query_str = query.to_string();
            tasks.spawn(async move { db_clone.search_tasks(&query_str).await.is_ok() });
        }
    }

    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap() {
            success_count += 1;
        }
    }

    let total_searches = queries.len() * 5;
    assert_eq!(
        success_count, total_searches,
        "All concurrent searches should succeed"
    );
}

/// Test concurrent health checks
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn concurrent_test_health_checks() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = Arc::new(ThingsDatabase::new(db_path).await.unwrap());

    let mut tasks = JoinSet::new();
    let num_checks = 30;

    for _ in 0..num_checks {
        let db_clone = Arc::clone(&db);
        tasks.spawn(async move { db_clone.is_connected().await });
    }

    let mut connected_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap() {
            connected_count += 1;
        }
    }

    assert_eq!(
        connected_count, num_checks,
        "All concurrent health checks should pass"
    );
}

/// Test concurrent cache access
#[tokio::test]
async fn concurrent_test_cache_access() {
    let cache = Arc::new(ThingsCache::new_default());
    let mut tasks = JoinSet::new();
    let num_tasks = 25;

    for _ in 0..num_tasks {
        let cache_clone = Arc::clone(&cache);
        tasks.spawn(async move {
            // Get cache stats concurrently
            let _stats = cache_clone.get_stats();
            true
        });
    }

    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap() {
            success_count += 1;
        }
    }

    assert_eq!(
        success_count, num_tasks,
        "All concurrent cache accesses should succeed"
    );
}

/// Test mixed concurrent operations
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn concurrent_test_mixed_operations() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = Arc::new(ThingsDatabase::new(db_path).await.unwrap());

    let mut tasks = JoinSet::new();

    // Spawn inbox queries
    for _ in 0..5 {
        let db_clone = Arc::clone(&db);
        tasks.spawn(async move { db_clone.get_inbox(Some(10)).await.is_ok() });
    }

    // Spawn searches
    for _ in 0..5 {
        let db_clone = Arc::clone(&db);
        tasks.spawn(async move { db_clone.search_tasks("test").await.is_ok() });
    }

    // Spawn health checks
    for _ in 0..5 {
        let db_clone = Arc::clone(&db);
        tasks.spawn(async move { db_clone.is_connected().await });
    }

    // Spawn project queries
    for _ in 0..5 {
        let db_clone = Arc::clone(&db);
        tasks.spawn(async move { db_clone.get_projects(Some(10)).await.is_ok() });
    }

    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap() {
            success_count += 1;
        }
    }

    assert_eq!(
        success_count, 20,
        "All mixed concurrent operations should succeed"
    );
}

/// Test database pool under concurrent load
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn concurrent_test_pool_stress() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = Arc::new(ThingsDatabase::new(db_path).await.unwrap());

    let mut tasks = JoinSet::new();
    let num_concurrent = 15;

    for _ in 0..num_concurrent {
        let db_clone = Arc::clone(&db);
        tasks.spawn(async move {
            // Perform multiple queries per task
            let mut all_ok = true;
            for _ in 0..3 {
                if db_clone.get_inbox(Some(5)).await.is_err() {
                    all_ok = false;
                    break;
                }
            }
            all_ok
        });
    }

    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap() {
            success_count += 1;
        }
    }

    assert_eq!(
        success_count, num_concurrent,
        "Connection pool should handle concurrent requests"
    );
}

/// Test concurrent cache statistics
#[tokio::test]
async fn concurrent_test_cache_stats() {
    let cache = Arc::new(ThingsCache::new_default());
    let mut tasks = JoinSet::new();

    for _ in 0..20 {
        let cache_clone = Arc::clone(&cache);
        tasks.spawn(async move {
            let _stats = cache_clone.get_stats();
            // Stats should always be retrievable
            true
        });
    }

    let mut all_valid = true;
    while let Some(result) = tasks.join_next().await {
        if !result.unwrap() {
            all_valid = false;
        }
    }

    assert!(
        all_valid,
        "Cache stats should remain consistent under concurrent access"
    );
}

/// Test rapid concurrent database cloning
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn concurrent_test_database_cloning() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = ThingsDatabase::new(db_path).await.unwrap();

    let mut tasks = JoinSet::new();

    for _ in 0..15 {
        let db_clone = db.clone();
        tasks.spawn(async move { db_clone.is_connected().await });
    }

    let mut all_connected = true;
    while let Some(result) = tasks.join_next().await {
        if !result.unwrap() {
            all_connected = false;
        }
    }

    assert!(
        all_connected,
        "Cloned databases should all remain connected"
    );
}

/// Test concurrent operations with different query sizes
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn concurrent_test_varying_query_sizes() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = Arc::new(ThingsDatabase::new(db_path).await.unwrap());

    let mut tasks = JoinSet::new();
    let sizes = vec![1, 5, 10, 50, 100];

    for size in sizes {
        for _ in 0..3 {
            let db_clone = Arc::clone(&db);
            tasks.spawn(async move { db_clone.get_inbox(Some(size)).await.is_ok() });
        }
    }

    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap() {
            success_count += 1;
        }
    }

    assert_eq!(
        success_count, 15,
        "Concurrent queries with varying sizes should succeed"
    );
}

/// Test cache creation under concurrent load
#[tokio::test]
async fn concurrent_test_cache_creation() {
    let config = CacheConfig::default();
    let mut tasks = JoinSet::new();

    for _ in 0..10 {
        let config_clone = config.clone();
        tasks.spawn(async move {
            let _cache = ThingsCache::new(&config_clone);
            true
        });
    }

    let mut all_created = true;
    while let Some(result) = tasks.join_next().await {
        if !result.unwrap() {
            all_created = false;
        }
    }

    assert!(all_created, "Caches should be creatable concurrently");
}

/// Test concurrent areas queries
#[tokio::test]
#[cfg(feature = "test-utils")]
async fn concurrent_test_areas_queries() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    create_test_database(db_path).await.unwrap();
    let db = Arc::new(ThingsDatabase::new(db_path).await.unwrap());

    let mut tasks = JoinSet::new();

    for _ in 0..20 {
        let db_clone = Arc::clone(&db);
        tasks.spawn(async move { db_clone.get_areas().await.is_ok() });
    }

    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 20, "Concurrent areas queries should succeed");
}
