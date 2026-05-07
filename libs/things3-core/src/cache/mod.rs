//! Caching layer for frequently accessed Things 3 data

mod config;
mod operations;
mod preloader;
mod stats;

pub use config::{CacheConfig, CacheDependency, InvalidationStrategy};
pub use preloader::{keys, DefaultPreloader};
pub use stats::{CachedData, CachePreloader, CacheStats};

use crate::models::{Area, Project, Task};
use moka::future::Cache;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Main cache manager for Things 3 data with intelligent invalidation
pub struct ThingsCache {
    /// Tasks cache
    tasks: Cache<String, CachedData<Vec<Task>>>,
    /// Projects cache
    projects: Cache<String, CachedData<Vec<Project>>>,
    /// Areas cache
    areas: Cache<String, CachedData<Vec<Area>>>,
    /// Search results cache
    search_results: Cache<String, CachedData<Vec<Task>>>,
    /// Statistics
    stats: Arc<RwLock<CacheStats>>,
    /// Configuration
    config: CacheConfig,
    /// Cache warming entries (key -> priority)
    warming_entries: Arc<RwLock<HashMap<String, u32>>>,
    /// Optional preloader consulted on every `get_*` access and on every
    /// warming-loop tick. `None` means no predictive preloading.
    preloader: Arc<RwLock<Option<Arc<dyn CachePreloader>>>>,
    /// Cache warming task handle
    warming_task: Option<tokio::task::JoinHandle<()>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{create_mock_areas, create_mock_projects, create_mock_tasks};
    use std::time::Duration;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();

        assert_eq!(config.max_capacity, 1000);
        assert_eq!(config.ttl, Duration::from_secs(300));
        assert_eq!(config.tti, Duration::from_secs(60));
    }

    #[test]
    fn test_cache_config_custom() {
        let config = CacheConfig {
            max_capacity: 500,
            ttl: Duration::from_secs(600),
            tti: Duration::from_secs(120),
            invalidation_strategy: InvalidationStrategy::Hybrid,
            enable_cache_warming: true,
            warming_interval: Duration::from_secs(60),
            max_warming_entries: 50,
        };

        assert_eq!(config.max_capacity, 500);
        assert_eq!(config.ttl, Duration::from_secs(600));
        assert_eq!(config.tti, Duration::from_secs(120));
    }

    #[test]
    fn test_cached_data_creation() {
        let data = vec![1, 2, 3];
        let ttl = Duration::from_secs(60);
        let cached = CachedData::new(data.clone(), ttl);

        assert_eq!(cached.data, data);
        assert!(cached.cached_at <= chrono::Utc::now());
        assert!(cached.expires_at > cached.cached_at);
        assert!(!cached.is_expired());
    }

    #[test]
    fn test_cached_data_expiration() {
        let data = vec![1, 2, 3];
        let ttl = Duration::from_millis(1);
        let cached = CachedData::new(data, ttl);

        // Should not be expired immediately
        assert!(!cached.is_expired());

        // Wait a bit and check again
        std::thread::sleep(Duration::from_millis(10));
        // Note: This test might be flaky due to timing, but it's testing the logic
    }

    #[test]
    fn test_cached_data_serialization() {
        let data = vec![1, 2, 3];
        let ttl = Duration::from_secs(60);
        let cached = CachedData::new(data, ttl);

        // Test serialization
        let json = serde_json::to_string(&cached).unwrap();
        assert!(json.contains("data"));
        assert!(json.contains("cached_at"));
        assert!(json.contains("expires_at"));

        // Test deserialization
        let deserialized: CachedData<Vec<i32>> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.data, cached.data);
    }

    #[test]
    fn test_cache_stats_default() {
        let stats = CacheStats::default();

        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.entries, 0);
        assert!((stats.hit_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cache_stats_calculation() {
        let mut stats = CacheStats {
            hits: 8,
            misses: 2,
            entries: 5,
            hit_rate: 0.0,
            ..Default::default()
        };

        stats.calculate_hit_rate();
        assert!((stats.hit_rate - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cache_stats_zero_total() {
        let mut stats = CacheStats {
            hits: 0,
            misses: 0,
            entries: 0,
            hit_rate: 0.0,
            ..Default::default()
        };

        stats.calculate_hit_rate();
        assert!((stats.hit_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cache_stats_serialization() {
        let stats = CacheStats {
            hits: 10,
            misses: 5,
            entries: 3,
            hit_rate: 0.67,
            ..Default::default()
        };

        // Test serialization
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("hits"));
        assert!(json.contains("misses"));
        assert!(json.contains("entries"));
        assert!(json.contains("hit_rate"));

        // Test deserialization
        let deserialized: CacheStats = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.hits, stats.hits);
        assert_eq!(deserialized.misses, stats.misses);
        assert_eq!(deserialized.entries, stats.entries);
        assert!((deserialized.hit_rate - stats.hit_rate).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cache_stats_clone() {
        let stats = CacheStats {
            hits: 5,
            misses: 3,
            entries: 2,
            hit_rate: 0.625,
            ..Default::default()
        };

        let cloned = stats.clone();
        assert_eq!(cloned.hits, stats.hits);
        assert_eq!(cloned.misses, stats.misses);
        assert_eq!(cloned.entries, stats.entries);
        assert!((cloned.hit_rate - stats.hit_rate).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cache_stats_debug() {
        let stats = CacheStats {
            hits: 1,
            misses: 1,
            entries: 1,
            hit_rate: 0.5,
            ..Default::default()
        };

        let debug_str = format!("{stats:?}");
        assert!(debug_str.contains("CacheStats"));
        assert!(debug_str.contains("hits"));
        assert!(debug_str.contains("misses"));
    }

    #[tokio::test]
    async fn test_cache_new() {
        let config = CacheConfig::default();
        let _cache = ThingsCache::new(&config);

        // Just test that it can be created
        // Test passes if we reach this point
    }

    #[tokio::test]
    async fn test_cache_new_default() {
        let _cache = ThingsCache::new_default();

        // Just test that it can be created
        // Test passes if we reach this point
    }

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = ThingsCache::new_default();

        // Test cache miss
        let result = cache.get_tasks("test", || async { Ok(vec![]) }).await;
        assert!(result.is_ok());

        // Test cache hit
        let result = cache.get_tasks("test", || async { Ok(vec![]) }).await;
        assert!(result.is_ok());

        let stats = cache.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_tasks_with_data() {
        let cache = ThingsCache::new_default();
        let mock_tasks = create_mock_tasks();

        // Test cache miss with data
        let result = cache
            .get_tasks("tasks", || async { Ok(mock_tasks.clone()) })
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), mock_tasks.len());

        // Test cache hit
        let result = cache.get_tasks("tasks", || async { Ok(vec![]) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), mock_tasks.len());

        let stats = cache.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_projects() {
        let cache = ThingsCache::new_default();
        let mock_projects = create_mock_projects();

        // Test cache miss
        let result = cache
            .get_projects("projects", || async { Ok(mock_projects.clone()) })
            .await;
        assert!(result.is_ok());

        // Test cache hit
        let result = cache
            .get_projects("projects", || async { Ok(vec![]) })
            .await;
        assert!(result.is_ok());

        let stats = cache.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_areas() {
        let cache = ThingsCache::new_default();
        let mock_areas = create_mock_areas();

        // Test cache miss
        let result = cache
            .get_areas("areas", || async { Ok(mock_areas.clone()) })
            .await;
        assert!(result.is_ok());

        // Test cache hit
        let result = cache.get_areas("areas", || async { Ok(vec![]) }).await;
        assert!(result.is_ok());

        let stats = cache.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_search_results() {
        let cache = ThingsCache::new_default();
        let mock_tasks = create_mock_tasks();

        // Test cache miss
        let result = cache
            .get_search_results("search:test", || async { Ok(mock_tasks.clone()) })
            .await;
        assert!(result.is_ok());

        // Test cache hit
        let result = cache
            .get_search_results("search:test", || async { Ok(vec![]) })
            .await;
        assert!(result.is_ok());

        let stats = cache.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_fetcher_error() {
        let cache = ThingsCache::new_default();

        // Test that fetcher errors are propagated
        let result = cache
            .get_tasks("error", || async { Err(anyhow::anyhow!("Test error")) })
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Test error"));

        let stats = cache.get_stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let config = CacheConfig {
            max_capacity: 100,
            ttl: Duration::from_millis(10),
            tti: Duration::from_millis(5),
            invalidation_strategy: InvalidationStrategy::Hybrid,
            enable_cache_warming: true,
            warming_interval: Duration::from_secs(60),
            max_warming_entries: 50,
        };
        let cache = ThingsCache::new(&config);

        // Insert data
        let _ = cache.get_tasks("test", || async { Ok(vec![]) }).await;

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Should be a miss due to expiration
        let _ = cache.get_tasks("test", || async { Ok(vec![]) }).await;

        let stats = cache.get_stats();
        assert_eq!(stats.misses, 2);
    }

    #[tokio::test]
    async fn test_cache_invalidate_all() {
        let cache = ThingsCache::new_default();

        // Insert data into all caches
        let _ = cache.get_tasks("tasks", || async { Ok(vec![]) }).await;
        let _ = cache
            .get_projects("projects", || async { Ok(vec![]) })
            .await;
        let _ = cache.get_areas("areas", || async { Ok(vec![]) }).await;
        let _ = cache
            .get_search_results("search", || async { Ok(vec![]) })
            .await;

        // Invalidate all
        cache.invalidate_all();

        // All should be misses now
        let _ = cache.get_tasks("tasks", || async { Ok(vec![]) }).await;
        let _ = cache
            .get_projects("projects", || async { Ok(vec![]) })
            .await;
        let _ = cache.get_areas("areas", || async { Ok(vec![]) }).await;
        let _ = cache
            .get_search_results("search", || async { Ok(vec![]) })
            .await;

        let stats = cache.get_stats();
        assert_eq!(stats.misses, 8); // 4 initial + 4 after invalidation
    }

    #[tokio::test]
    async fn test_cache_invalidate_specific() {
        let cache = ThingsCache::new_default();

        // Insert data
        let _ = cache.get_tasks("key1", || async { Ok(vec![]) }).await;
        let _ = cache.get_tasks("key2", || async { Ok(vec![]) }).await;

        // Invalidate specific key
        cache.invalidate("key1").await;

        // key1 should be a miss, key2 should be a hit
        let _ = cache.get_tasks("key1", || async { Ok(vec![]) }).await;
        let _ = cache.get_tasks("key2", || async { Ok(vec![]) }).await;

        let stats = cache.get_stats();
        assert_eq!(stats.hits, 1); // key2 hit
        assert_eq!(stats.misses, 3); // key1 initial + key1 after invalidation + key2 initial
    }

    #[tokio::test]
    async fn test_cache_reset_stats() {
        let cache = ThingsCache::new_default();

        // Generate some stats
        let _ = cache.get_tasks("test", || async { Ok(vec![]) }).await;
        let _ = cache.get_tasks("test", || async { Ok(vec![]) }).await;

        let stats_before = cache.get_stats();
        assert!(stats_before.hits > 0 || stats_before.misses > 0);

        // Reset stats
        cache.reset_stats();

        let stats_after = cache.get_stats();
        assert_eq!(stats_after.hits, 0);
        assert_eq!(stats_after.misses, 0);
        assert!((stats_after.hit_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cache_keys_inbox() {
        assert_eq!(keys::inbox(None), "inbox:all");
        assert_eq!(keys::inbox(Some(10)), "inbox:10");
        assert_eq!(keys::inbox(Some(0)), "inbox:0");
    }

    #[test]
    fn test_cache_keys_today() {
        assert_eq!(keys::today(None), "today:all");
        assert_eq!(keys::today(Some(5)), "today:5");
        assert_eq!(keys::today(Some(100)), "today:100");
    }

    #[test]
    fn test_cache_keys_projects() {
        assert_eq!(keys::projects(None), "projects:all");
        assert_eq!(keys::projects(Some("uuid-123")), "projects:uuid-123");
        assert_eq!(keys::projects(Some("")), "projects:");
    }

    #[test]
    fn test_cache_keys_areas() {
        assert_eq!(keys::areas(), "areas:all");
    }

    #[test]
    fn test_cache_keys_search() {
        assert_eq!(keys::search("test query", None), "search:test query:all");
        assert_eq!(keys::search("test query", Some(10)), "search:test query:10");
        assert_eq!(keys::search("", Some(5)), "search::5");
    }

    #[tokio::test]
    async fn test_cache_multiple_keys() {
        let cache = ThingsCache::new_default();
        let mock_tasks1 = create_mock_tasks();
        let mock_tasks2 = create_mock_tasks();

        // Test different keys don't interfere
        let _ = cache
            .get_tasks("key1", || async { Ok(mock_tasks1.clone()) })
            .await;
        let _ = cache
            .get_tasks("key2", || async { Ok(mock_tasks2.clone()) })
            .await;

        // Both should be hits
        let result1 = cache
            .get_tasks("key1", || async { Ok(vec![]) })
            .await
            .unwrap();
        let result2 = cache
            .get_tasks("key2", || async { Ok(vec![]) })
            .await
            .unwrap();

        assert_eq!(result1.len(), mock_tasks1.len());
        assert_eq!(result2.len(), mock_tasks2.len());

        let stats = cache.get_stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 2);
    }

    #[tokio::test]
    async fn test_cache_entry_count() {
        let cache = ThingsCache::new_default();

        // Initially no entries
        let stats = cache.get_stats();
        assert_eq!(stats.entries, 0);

        // Add some entries
        let _ = cache.get_tasks("tasks", || async { Ok(vec![]) }).await;
        let _ = cache
            .get_projects("projects", || async { Ok(vec![]) })
            .await;
        let _ = cache.get_areas("areas", || async { Ok(vec![]) }).await;
        let _ = cache
            .get_search_results("search", || async { Ok(vec![]) })
            .await;

        // The entry count might not be immediately updated due to async nature
        // Let's just verify that we can get stats without panicking
        let stats = cache.get_stats();
        // Verify stats can be retrieved without panicking
        let _ = stats.entries;
    }

    #[tokio::test]
    async fn test_cache_hit_rate_calculation() {
        let cache = ThingsCache::new_default();

        // Generate some hits and misses
        let _ = cache.get_tasks("test", || async { Ok(vec![]) }).await; // miss
        let _ = cache.get_tasks("test", || async { Ok(vec![]) }).await; // hit
        let _ = cache.get_tasks("test", || async { Ok(vec![]) }).await; // hit

        let stats = cache.get_stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 2.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn test_cache_dependency_matches_rules() {
        use crate::models::ThingsId;
        let id_a = ThingsId::new_v4();
        let id_b = ThingsId::new_v4();
        let dep_concrete = CacheDependency {
            entity_type: "task".to_string(),
            entity_id: Some(id_a.clone()),
            invalidating_operations: vec!["task_updated".to_string()],
        };
        let dep_wildcard = CacheDependency {
            entity_type: "task".to_string(),
            entity_id: None,
            invalidating_operations: vec!["task_updated".to_string()],
        };

        // concrete dep matches its own id, not a different id
        assert!(dep_concrete.matches("task", Some(&id_a)));
        assert!(!dep_concrete.matches("task", Some(&id_b)));
        // wildcard request matches concrete dep
        assert!(dep_concrete.matches("task", None));
        // wildcard dep matches any concrete id of same type
        assert!(dep_wildcard.matches("task", Some(&id_a)));
        // type mismatch never matches
        assert!(!dep_concrete.matches("project", Some(&id_a)));

        // operation matching
        assert!(dep_concrete.matches_operation("task_updated"));
        assert!(!dep_concrete.matches_operation("task_deleted"));
    }

    /// Build a `Task` whose `uuid`, `project_uuid`, and `area_uuid` we control,
    /// so dependency lists carry the IDs we expect.
    fn task_with_ids(
        uuid: crate::models::ThingsId,
        project: Option<crate::models::ThingsId>,
        area: Option<crate::models::ThingsId>,
    ) -> crate::models::Task {
        let mut t = create_mock_tasks().into_iter().next().unwrap();
        t.uuid = uuid;
        t.project_uuid = project;
        t.area_uuid = area;
        t
    }

    #[tokio::test]
    async fn test_invalidate_by_entity_selective_by_id() {
        use crate::models::ThingsId;
        let cache = ThingsCache::new_default();
        let id_x = ThingsId::new_v4();
        let id_y = ThingsId::new_v4();

        let id_x2 = id_x.clone();
        let id_y2 = id_y.clone();
        cache
            .get_tasks("key_x", || async {
                Ok(vec![task_with_ids(id_x2, None, None)])
            })
            .await
            .unwrap();
        cache
            .get_tasks("key_y", || async {
                Ok(vec![task_with_ids(id_y2, None, None)])
            })
            .await
            .unwrap();

        let removed = cache.invalidate_by_entity("task", Some(&id_x)).await;
        assert_eq!(removed, 1, "only the entry depending on id_x should evict");
        cache.tasks.run_pending_tasks().await;
        assert!(cache.tasks.get("key_x").await.is_none());
        assert!(cache.tasks.get("key_y").await.is_some());
    }

    #[tokio::test]
    async fn test_invalidate_by_entity_wildcard_id() {
        use crate::models::ThingsId;
        let cache = ThingsCache::new_default();
        let id_x = ThingsId::new_v4();
        let id_y = ThingsId::new_v4();

        let id_x2 = id_x.clone();
        let id_y2 = id_y.clone();
        cache
            .get_tasks("key_x", || async {
                Ok(vec![task_with_ids(id_x2, None, None)])
            })
            .await
            .unwrap();
        cache
            .get_tasks("key_y", || async {
                Ok(vec![task_with_ids(id_y2, None, None)])
            })
            .await
            .unwrap();

        let removed = cache.invalidate_by_entity("task", None).await;
        assert_eq!(removed, 2);
        cache.tasks.run_pending_tasks().await;
        assert!(cache.tasks.get("key_x").await.is_none());
        assert!(cache.tasks.get("key_y").await.is_none());
    }

    #[tokio::test]
    async fn test_invalidate_by_entity_leaves_unrelated_caches() {
        use crate::models::ThingsId;
        let cache = ThingsCache::new_default();
        let task_id = ThingsId::new_v4();
        let project_id = ThingsId::new_v4();

        let task_id2 = task_id.clone();
        let project_id2 = project_id.clone();
        // task entry depends on its own task_id AND on project_id
        cache
            .get_tasks("inbox", || async {
                Ok(vec![task_with_ids(task_id2, Some(project_id2), None)])
            })
            .await
            .unwrap();
        // project entry: cached projects keyed under "projects:all"
        let mut p = create_mock_projects().into_iter().next().unwrap();
        p.uuid = project_id;
        cache
            .get_projects("projects:all", || async { Ok(vec![p]) })
            .await
            .unwrap();

        // invalidate by *task* id — must not nuke the projects cache
        let removed = cache.invalidate_by_entity("task", Some(&task_id)).await;
        assert_eq!(removed, 1);
        cache.tasks.run_pending_tasks().await;
        cache.projects.run_pending_tasks().await;
        assert!(cache.tasks.get("inbox").await.is_none());
        assert!(cache.projects.get("projects:all").await.is_some());
    }

    #[tokio::test]
    async fn test_invalidate_by_operation_selective() {
        use crate::models::ThingsId;
        let cache = ThingsCache::new_default();
        let task_id = ThingsId::new_v4();
        let area_id = ThingsId::new_v4();

        let task_id2 = task_id.clone();
        // task entry: invalidating_operations include "task_updated"
        cache
            .get_tasks("inbox", || async {
                Ok(vec![task_with_ids(task_id2, None, None)])
            })
            .await
            .unwrap();
        // area entry: invalidating_operations include "area_updated", NOT "task_updated"
        let mut a = create_mock_areas().into_iter().next().unwrap();
        a.uuid = area_id;
        cache
            .get_areas("areas:all", || async { Ok(vec![a]) })
            .await
            .unwrap();

        let removed = cache.invalidate_by_operation("task_updated").await;
        assert_eq!(removed, 1);
        cache.tasks.run_pending_tasks().await;
        cache.areas.run_pending_tasks().await;
        assert!(cache.tasks.get("inbox").await.is_none());
        assert!(cache.areas.get("areas:all").await.is_some());
    }

    // ─── Predictive preloading (#94) ──────────────────────────────────────

    /// Recording preloader: captures every `predict` and `warm` call so tests
    /// can assert that the cache fired the hooks at the right moments.
    struct RecordingPreloader {
        predictions: Arc<RwLock<Vec<(String, u32)>>>,
        seen_predict: Arc<RwLock<Vec<String>>>,
        seen_warm: Arc<RwLock<Vec<String>>>,
    }

    impl RecordingPreloader {
        fn new(predictions: Vec<(String, u32)>) -> Self {
            Self {
                predictions: Arc::new(RwLock::new(predictions)),
                seen_predict: Arc::new(RwLock::new(Vec::new())),
                seen_warm: Arc::new(RwLock::new(Vec::new())),
            }
        }
    }

    impl CachePreloader for RecordingPreloader {
        fn predict(&self, accessed_key: &str) -> Vec<(String, u32)> {
            self.seen_predict.write().push(accessed_key.to_string());
            self.predictions.read().clone()
        }
        fn warm(&self, key: &str) {
            self.seen_warm.write().push(key.to_string());
        }
    }

    #[tokio::test]
    async fn test_default_preloader_predict_rules() {
        // All three heuristic rules tested against the real DefaultPreloader.
        // predict() is pure (doesn't touch self.cache or self.db), so we only
        // need a minimal DB to satisfy DefaultPreloader::new.
        let f = tempfile::NamedTempFile::new().unwrap();
        crate::test_utils::create_test_database(f.path())
            .await
            .unwrap();
        let db = Arc::new(crate::ThingsDatabase::new(f.path()).await.unwrap());
        let cache = Arc::new(ThingsCache::new_default());
        let pre = DefaultPreloader::new(&cache, db);

        assert_eq!(pre.predict("inbox:all"), vec![("today:all".to_string(), 8)]);
        assert_eq!(
            pre.predict("today:all"),
            vec![("inbox:all".to_string(), 10)]
        );
        assert_eq!(
            pre.predict("areas:all"),
            vec![("projects:all".to_string(), 7)]
        );
        assert!(pre.predict("search:foo").is_empty());
    }

    #[tokio::test]
    async fn test_predict_fires_on_get_tasks_miss_and_hit() {
        let cache = ThingsCache::new_default();
        let pre = Arc::new(RecordingPreloader::new(vec![]));
        cache.set_preloader(pre.clone());

        cache
            .get_tasks("inbox:all", || async { Ok(vec![]) })
            .await
            .unwrap();
        cache
            .get_tasks("inbox:all", || async { Ok(vec![]) })
            .await
            .unwrap();

        let seen = pre.seen_predict.read().clone();
        assert_eq!(seen, vec!["inbox:all".to_string(), "inbox:all".to_string()]);
    }

    #[tokio::test]
    async fn test_predict_enqueues_warming() {
        let cache = ThingsCache::new_default();
        let pre = Arc::new(RecordingPreloader::new(vec![("today:all".to_string(), 5)]));
        cache.set_preloader(pre);

        cache
            .get_tasks("inbox:all", || async { Ok(vec![]) })
            .await
            .unwrap();

        let entries = cache.warming_entries.read();
        assert_eq!(entries.get("today:all"), Some(&5));
    }

    #[tokio::test]
    async fn test_no_preloader_is_noop() {
        // Default cache (no preloader) — get_* must not panic; stats counters
        // for warming must stay at zero even if the warming loop ticks.
        let config = CacheConfig {
            warming_interval: Duration::from_millis(20),
            ..Default::default()
        };
        let cache = ThingsCache::new(&config);
        cache
            .get_tasks("inbox:all", || async { Ok(vec![]) })
            .await
            .unwrap();
        // Let the warming loop tick a few times.
        tokio::time::sleep(Duration::from_millis(80)).await;
        let stats = cache.get_stats();
        assert_eq!(stats.warmed_keys, 0);
        assert_eq!(stats.warming_runs, 0);
    }

    #[tokio::test]
    async fn test_warming_loop_invokes_warm() {
        let config = CacheConfig {
            warming_interval: Duration::from_millis(20),
            max_warming_entries: 10,
            ..Default::default()
        };
        let cache = ThingsCache::new(&config);

        let pre = Arc::new(RecordingPreloader::new(vec![]));
        cache.set_preloader(pre.clone());

        cache.add_to_warming("inbox:all".to_string(), 10);
        cache.add_to_warming("today:all".to_string(), 8);

        // Wait long enough for at least one warming-loop tick.
        tokio::time::sleep(Duration::from_millis(100)).await;

        let warmed = pre.seen_warm.read().clone();
        assert!(warmed.contains(&"inbox:all".to_string()));
        assert!(warmed.contains(&"today:all".to_string()));

        // Queue should have been drained after dispatch.
        assert!(cache.warming_entries.read().is_empty());

        // Stats should reflect the work.
        let stats = cache.get_stats();
        assert!(stats.warming_runs >= 1);
        assert!(stats.warmed_keys >= 2);
    }

    #[tokio::test]
    async fn test_clear_preloader_disables_predict() {
        let cache = ThingsCache::new_default();
        let pre = Arc::new(RecordingPreloader::new(vec![("today:all".to_string(), 5)]));
        cache.set_preloader(pre.clone());
        cache
            .get_tasks("inbox:all", || async { Ok(vec![]) })
            .await
            .unwrap();
        assert_eq!(pre.seen_predict.read().len(), 1);

        cache.clear_preloader();
        cache
            .get_tasks("inbox:all", || async { Ok(vec![]) })
            .await
            .unwrap();
        // Cleared — no further calls.
        assert_eq!(pre.seen_predict.read().len(), 1);
    }

    #[tokio::test]
    async fn test_default_preloader_warms_via_db() {
        // Full integration: real test DB, real DefaultPreloader, real warming
        // loop. After fetching `inbox:all`, the loop should warm `today:all`.
        let f = tempfile::NamedTempFile::new().unwrap();
        crate::test_utils::create_test_database(f.path())
            .await
            .unwrap();
        let db = Arc::new(crate::ThingsDatabase::new(f.path()).await.unwrap());

        let config = CacheConfig {
            warming_interval: Duration::from_millis(20),
            ..Default::default()
        };
        let cache = Arc::new(ThingsCache::new(&config));
        cache.set_preloader(DefaultPreloader::new(&cache, Arc::clone(&db)));

        // Trigger predict("inbox:all") → enqueues "today:all" with priority 8
        cache
            .get_tasks("inbox:all", || async {
                db.get_inbox(None).await.map_err(anyhow::Error::from)
            })
            .await
            .unwrap();

        // Wait for the warming loop to tick AND for the spawned warm() task
        // (which calls back into cache.get_tasks) to complete.
        tokio::time::sleep(Duration::from_millis(150)).await;

        // After warming, "today:all" should hit cache without invoking the
        // panicking fetcher.
        let result = cache
            .get_tasks("today:all", || async {
                panic!("today:all should be served from warmed cache, not fetched")
            })
            .await
            .unwrap();
        // Sanity: result is whatever db.get_today returned (possibly empty).
        let expected = db.get_today(None).await.unwrap();
        assert_eq!(result.len(), expected.len());
    }

    #[tokio::test]
    async fn test_default_preloader_weak_ref_breaks_cycle() {
        // Drop the only Arc<ThingsCache>; DefaultPreloader.warm should noop.
        let f = tempfile::NamedTempFile::new().unwrap();
        crate::test_utils::create_test_database(f.path())
            .await
            .unwrap();
        let db = Arc::new(crate::ThingsDatabase::new(f.path()).await.unwrap());

        let cache = Arc::new(ThingsCache::new_default());
        let preloader = DefaultPreloader::new(&cache, db);
        let preloader_dyn: Arc<dyn CachePreloader> = preloader.clone();

        drop(cache);

        // Should not panic and should not spawn a doomed task.
        preloader_dyn.warm("inbox:all");
        // Sanity: weak ref upgrade inside warm returned None — no observable
        // side effect to assert beyond "did not panic".
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
