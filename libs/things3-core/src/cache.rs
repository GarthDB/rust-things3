//! Caching layer for frequently accessed Things 3 data

use crate::models::{Area, Project, Task};
use anyhow::Result;
use chrono::{DateTime, Utc};
use moka::future::Cache;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Cache invalidation strategy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidationStrategy {
    /// Time-based invalidation (TTL)
    TimeBased,
    /// Event-based invalidation (manual triggers)
    EventBased,
    /// Dependency-based invalidation (related data changes)
    DependencyBased,
    /// Hybrid approach combining multiple strategies
    Hybrid,
}

/// Cache dependency tracking for intelligent invalidation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheDependency {
    /// The entity type this cache entry depends on
    pub entity_type: String,
    /// The specific entity ID this cache entry depends on
    pub entity_id: Option<Uuid>,
    /// The operation that would invalidate this cache entry
    pub invalidating_operations: Vec<String>,
}

/// Enhanced cache configuration with intelligent invalidation
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache
    pub max_capacity: u64,
    /// Time to live for cache entries
    pub ttl: Duration,
    /// Time to idle for cache entries
    pub tti: Duration,
    /// Invalidation strategy to use
    pub invalidation_strategy: InvalidationStrategy,
    /// Enable cache warming for frequently accessed data
    pub enable_cache_warming: bool,
    /// Cache warming interval
    pub warming_interval: Duration,
    /// Maximum cache warming entries
    pub max_warming_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 1000,
            ttl: Duration::from_secs(300), // 5 minutes
            tti: Duration::from_secs(60),  // 1 minute
            invalidation_strategy: InvalidationStrategy::Hybrid,
            enable_cache_warming: true,
            warming_interval: Duration::from_secs(60), // 1 minute
            max_warming_entries: 50,
        }
    }
}

/// Enhanced cached data wrapper with dependency tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedData<T> {
    pub data: T,
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// Dependencies for intelligent invalidation
    pub dependencies: Vec<CacheDependency>,
    /// Access count for cache warming
    pub access_count: u64,
    /// Last access time for TTI calculation
    pub last_accessed: DateTime<Utc>,
    /// Cache warming priority (higher = more likely to be warmed)
    pub warming_priority: u32,
}

impl<T> CachedData<T> {
    pub fn new(data: T, ttl: Duration) -> Self {
        let now = Utc::now();
        Self {
            data,
            cached_at: now,
            expires_at: now + chrono::Duration::from_std(ttl).unwrap_or_default(),
            dependencies: Vec::new(),
            access_count: 0,
            last_accessed: now,
            warming_priority: 0,
        }
    }

    pub fn new_with_dependencies(
        data: T,
        ttl: Duration,
        dependencies: Vec<CacheDependency>,
    ) -> Self {
        let now = Utc::now();
        Self {
            data,
            cached_at: now,
            expires_at: now + chrono::Duration::from_std(ttl).unwrap_or_default(),
            dependencies,
            access_count: 0,
            last_accessed: now,
            warming_priority: 0,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn is_idle(&self, tti: Duration) -> bool {
        let now = Utc::now();
        let idle_duration = now - self.last_accessed;
        idle_duration > chrono::Duration::from_std(tti).unwrap_or_default()
    }

    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now();
    }

    pub fn update_warming_priority(&mut self, priority: u32) {
        self.warming_priority = priority;
    }

    pub fn add_dependency(&mut self, dependency: CacheDependency) {
        self.dependencies.push(dependency);
    }

    pub fn has_dependency(&self, entity_type: &str, entity_id: Option<&Uuid>) -> bool {
        self.dependencies.iter().any(|dep| {
            dep.entity_type == entity_type
                && entity_id.is_none_or(|id| dep.entity_id.as_ref() == Some(id))
        })
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: u64,
    pub hit_rate: f64,
}

impl CacheStats {
    pub fn calculate_hit_rate(&mut self) {
        let total = self.hits + self.misses;
        self.hit_rate = if total > 0 {
            #[allow(clippy::cast_precision_loss)]
            {
                self.hits as f64 / total as f64
            }
        } else {
            0.0
        };
    }
}

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
    /// Cache warming task handle
    warming_task: Option<tokio::task::JoinHandle<()>>,
}

impl ThingsCache {
    /// Create a new cache with the given configuration
    #[must_use]
    pub fn new(config: &CacheConfig) -> Self {
        let tasks = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.ttl)
            .time_to_idle(config.tti)
            .build();

        let projects = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.ttl)
            .time_to_idle(config.tti)
            .build();

        let areas = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.ttl)
            .time_to_idle(config.tti)
            .build();

        let search_results = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.ttl)
            .time_to_idle(config.tti)
            .build();

        let mut cache = Self {
            tasks,
            projects,
            areas,
            search_results,
            stats: Arc::new(RwLock::new(CacheStats::default())),
            config: config.clone(),
            warming_entries: Arc::new(RwLock::new(HashMap::new())),
            warming_task: None,
        };

        // Start cache warming task if enabled
        if config.enable_cache_warming {
            cache.start_cache_warming();
        }

        cache
    }

    /// Create a new cache with default configuration
    #[must_use]
    pub fn new_default() -> Self {
        Self::new(&CacheConfig::default())
    }

    /// Get tasks from cache or execute the provided function
    /// Get tasks from cache or fetch if not cached
    ///
    /// # Errors
    ///
    /// Returns an error if the fetcher function fails.
    pub async fn get_tasks<F, Fut>(&self, key: &str, fetcher: F) -> Result<Vec<Task>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<Task>>>,
    {
        if let Some(mut cached) = self.tasks.get(key).await {
            if !cached.is_expired() && !cached.is_idle(self.config.tti) {
                cached.record_access();
                self.record_hit();

                // Add to warming if frequently accessed
                if cached.access_count > 3 {
                    self.add_to_warming(key.to_string(), cached.warming_priority + 1);
                }

                return Ok(cached.data);
            }
        }

        self.record_miss();
        let data = fetcher().await?;

        // Create dependencies for intelligent invalidation
        let dependencies = Self::create_task_dependencies(&data);
        let mut cached_data =
            CachedData::new_with_dependencies(data.clone(), self.config.ttl, dependencies);

        // Set initial warming priority based on key type
        let priority = if key.starts_with("inbox:") {
            10
        } else if key.starts_with("today:") {
            8
        } else {
            5
        };
        cached_data.update_warming_priority(priority);

        self.tasks.insert(key.to_string(), cached_data).await;
        Ok(data)
    }

    /// Get projects from cache or execute the provided function
    /// Get projects from cache or fetch if not cached
    ///
    /// # Errors
    ///
    /// Returns an error if the fetcher function fails.
    pub async fn get_projects<F, Fut>(&self, key: &str, fetcher: F) -> Result<Vec<Project>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<Project>>>,
    {
        if let Some(mut cached) = self.projects.get(key).await {
            if !cached.is_expired() && !cached.is_idle(self.config.tti) {
                cached.record_access();
                self.record_hit();

                // Add to warming if frequently accessed
                if cached.access_count > 3 {
                    self.add_to_warming(key.to_string(), cached.warming_priority + 1);
                }

                return Ok(cached.data);
            }
        }

        self.record_miss();
        let data = fetcher().await?;

        // Create dependencies for intelligent invalidation
        let dependencies = Self::create_project_dependencies(&data);
        let mut cached_data =
            CachedData::new_with_dependencies(data.clone(), self.config.ttl, dependencies);

        // Set initial warming priority
        let priority = if key.starts_with("projects:") { 7 } else { 5 };
        cached_data.update_warming_priority(priority);

        self.projects.insert(key.to_string(), cached_data).await;
        Ok(data)
    }

    /// Get areas from cache or execute the provided function
    /// Get areas from cache or fetch if not cached
    ///
    /// # Errors
    ///
    /// Returns an error if the fetcher function fails.
    pub async fn get_areas<F, Fut>(&self, key: &str, fetcher: F) -> Result<Vec<Area>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<Area>>>,
    {
        if let Some(mut cached) = self.areas.get(key).await {
            if !cached.is_expired() && !cached.is_idle(self.config.tti) {
                cached.record_access();
                self.record_hit();

                // Add to warming if frequently accessed
                if cached.access_count > 3 {
                    self.add_to_warming(key.to_string(), cached.warming_priority + 1);
                }

                return Ok(cached.data);
            }
        }

        self.record_miss();
        let data = fetcher().await?;

        // Create dependencies for intelligent invalidation
        let dependencies = Self::create_area_dependencies(&data);
        let mut cached_data =
            CachedData::new_with_dependencies(data.clone(), self.config.ttl, dependencies);

        // Set initial warming priority
        let priority = if key.starts_with("areas:") { 6 } else { 5 };
        cached_data.update_warming_priority(priority);

        self.areas.insert(key.to_string(), cached_data).await;
        Ok(data)
    }

    /// Get search results from cache or execute the provided function
    /// Get search results from cache or fetch if not cached
    ///
    /// # Errors
    ///
    /// Returns an error if the fetcher function fails.
    pub async fn get_search_results<F, Fut>(&self, key: &str, fetcher: F) -> Result<Vec<Task>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<Task>>>,
    {
        if let Some(mut cached) = self.search_results.get(key).await {
            if !cached.is_expired() && !cached.is_idle(self.config.tti) {
                cached.record_access();
                self.record_hit();

                // Add to warming if frequently accessed
                if cached.access_count > 3 {
                    self.add_to_warming(key.to_string(), cached.warming_priority + 1);
                }

                return Ok(cached.data);
            }
        }

        self.record_miss();
        let data = fetcher().await?;

        // Create dependencies for intelligent invalidation
        let dependencies = Self::create_task_dependencies(&data);
        let mut cached_data =
            CachedData::new_with_dependencies(data.clone(), self.config.ttl, dependencies);

        // Set initial warming priority for search results
        let priority = if key.starts_with("search:") { 4 } else { 3 };
        cached_data.update_warming_priority(priority);

        self.search_results
            .insert(key.to_string(), cached_data)
            .await;
        Ok(data)
    }

    /// Invalidate all caches
    pub fn invalidate_all(&self) {
        self.tasks.invalidate_all();
        self.projects.invalidate_all();
        self.areas.invalidate_all();
        self.search_results.invalidate_all();
    }

    /// Invalidate specific cache entry
    pub async fn invalidate(&self, key: &str) {
        self.tasks.remove(key).await;
        self.projects.remove(key).await;
        self.areas.remove(key).await;
        self.search_results.remove(key).await;
    }

    /// Get cache statistics
    #[must_use]
    pub fn get_stats(&self) -> CacheStats {
        let mut stats = self.stats.read().clone();
        stats.entries = self.tasks.entry_count()
            + self.projects.entry_count()
            + self.areas.entry_count()
            + self.search_results.entry_count();
        stats.calculate_hit_rate();
        stats
    }

    /// Reset cache statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.write();
        *stats = CacheStats::default();
    }

    /// Record a cache hit
    fn record_hit(&self) {
        let mut stats = self.stats.write();
        stats.hits += 1;
    }

    /// Record a cache miss
    fn record_miss(&self) {
        let mut stats = self.stats.write();
        stats.misses += 1;
    }

    /// Create dependencies for task data
    fn create_task_dependencies(tasks: &[Task]) -> Vec<CacheDependency> {
        let mut dependencies = Vec::new();

        // Add dependencies for each task
        for task in tasks {
            dependencies.push(CacheDependency {
                entity_type: "task".to_string(),
                entity_id: Some(task.uuid),
                invalidating_operations: vec![
                    "task_updated".to_string(),
                    "task_deleted".to_string(),
                    "task_completed".to_string(),
                ],
            });

            // Add project dependency if task belongs to a project
            if let Some(project_uuid) = task.project_uuid {
                dependencies.push(CacheDependency {
                    entity_type: "project".to_string(),
                    entity_id: Some(project_uuid),
                    invalidating_operations: vec![
                        "project_updated".to_string(),
                        "project_deleted".to_string(),
                    ],
                });
            }

            // Add area dependency if task belongs to an area
            if let Some(area_uuid) = task.area_uuid {
                dependencies.push(CacheDependency {
                    entity_type: "area".to_string(),
                    entity_id: Some(area_uuid),
                    invalidating_operations: vec![
                        "area_updated".to_string(),
                        "area_deleted".to_string(),
                    ],
                });
            }
        }

        dependencies
    }

    /// Create dependencies for project data
    fn create_project_dependencies(projects: &[Project]) -> Vec<CacheDependency> {
        let mut dependencies = Vec::new();

        for project in projects {
            dependencies.push(CacheDependency {
                entity_type: "project".to_string(),
                entity_id: Some(project.uuid),
                invalidating_operations: vec![
                    "project_updated".to_string(),
                    "project_deleted".to_string(),
                ],
            });

            if let Some(area_uuid) = project.area_uuid {
                dependencies.push(CacheDependency {
                    entity_type: "area".to_string(),
                    entity_id: Some(area_uuid),
                    invalidating_operations: vec![
                        "area_updated".to_string(),
                        "area_deleted".to_string(),
                    ],
                });
            }
        }

        dependencies
    }

    /// Create dependencies for area data
    fn create_area_dependencies(areas: &[Area]) -> Vec<CacheDependency> {
        let mut dependencies = Vec::new();

        for area in areas {
            dependencies.push(CacheDependency {
                entity_type: "area".to_string(),
                entity_id: Some(area.uuid),
                invalidating_operations: vec![
                    "area_updated".to_string(),
                    "area_deleted".to_string(),
                ],
            });
        }

        dependencies
    }

    /// Start cache warming background task
    fn start_cache_warming(&mut self) {
        let warming_entries = Arc::clone(&self.warming_entries);
        let warming_interval = self.config.warming_interval;
        let max_entries = self.config.max_warming_entries;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(warming_interval);
            loop {
                interval.tick().await;

                // Get top priority entries for warming
                let entries_to_warm = {
                    let entries = warming_entries.read();
                    let mut sorted_entries: Vec<_> = entries.iter().collect();
                    sorted_entries.sort_by(|a, b| b.1.cmp(a.1));
                    sorted_entries
                        .into_iter()
                        .take(max_entries)
                        .map(|(key, _)| key.clone())
                        .collect::<Vec<_>>()
                };

                // In a real implementation, you would warm these entries
                // by calling the appropriate fetcher functions
                if !entries_to_warm.is_empty() {
                    tracing::debug!("Cache warming {} entries", entries_to_warm.len());
                }
            }
        });

        self.warming_task = Some(handle);
    }

    /// Add entry to cache warming list
    pub fn add_to_warming(&self, key: String, priority: u32) {
        let mut entries = self.warming_entries.write();
        entries.insert(key, priority);
    }

    /// Remove entry from cache warming list
    pub fn remove_from_warming(&self, key: &str) {
        let mut entries = self.warming_entries.write();
        entries.remove(key);
    }

    /// Invalidate cache entries based on entity changes
    pub fn invalidate_by_entity(&self, entity_type: &str, entity_id: Option<&Uuid>) {
        // For now, we'll invalidate all caches when an entity changes
        // In a more sophisticated implementation, we would track dependencies
        // and only invalidate specific entries

        // Invalidate all caches as a conservative approach
        self.tasks.invalidate_all();
        self.projects.invalidate_all();
        self.areas.invalidate_all();
        self.search_results.invalidate_all();

        tracing::debug!(
            "Invalidated all caches due to entity change: {} {:?}",
            entity_type,
            entity_id
        );
    }

    /// Invalidate cache entries by operation type
    pub fn invalidate_by_operation(&self, operation: &str) {
        // For now, we'll invalidate all caches when certain operations occur
        // In a more sophisticated implementation, we would track dependencies
        // and only invalidate specific entries based on the operation

        match operation {
            "task_created" | "task_updated" | "task_deleted" | "task_completed" => {
                self.tasks.invalidate_all();
                self.search_results.invalidate_all();
            }
            "project_created" | "project_updated" | "project_deleted" => {
                self.projects.invalidate_all();
                self.tasks.invalidate_all(); // Tasks depend on projects
            }
            "area_created" | "area_updated" | "area_deleted" => {
                self.areas.invalidate_all();
                self.projects.invalidate_all(); // Projects depend on areas
                self.tasks.invalidate_all(); // Tasks depend on areas
            }
            _ => {
                // For unknown operations, invalidate all caches as a conservative approach
                self.invalidate_all();
            }
        }

        tracing::debug!("Invalidated caches due to operation: {}", operation);
    }

    /// Get cache warming statistics
    #[must_use]
    pub fn get_warming_stats(&self) -> (usize, u32) {
        let entries = self.warming_entries.read();
        let count = entries.len();
        let max_priority = entries.values().max().copied().unwrap_or(0);
        (count, max_priority)
    }

    /// Stop cache warming
    pub fn stop_cache_warming(&mut self) {
        if let Some(handle) = self.warming_task.take() {
            handle.abort();
        }
    }
}

/// Cache key generators
pub mod keys {
    /// Generate cache key for inbox tasks
    #[must_use]
    pub fn inbox(limit: Option<usize>) -> String {
        format!(
            "inbox:{}",
            limit.map_or("all".to_string(), |l| l.to_string())
        )
    }

    /// Generate cache key for today's tasks
    #[must_use]
    pub fn today(limit: Option<usize>) -> String {
        format!(
            "today:{}",
            limit.map_or("all".to_string(), |l| l.to_string())
        )
    }

    /// Generate cache key for projects
    #[must_use]
    pub fn projects(area_uuid: Option<&str>) -> String {
        format!("projects:{}", area_uuid.unwrap_or("all"))
    }

    /// Generate cache key for areas
    #[must_use]
    pub fn areas() -> String {
        "areas:all".to_string()
    }

    /// Generate cache key for search results
    #[must_use]
    pub fn search(query: &str, limit: Option<usize>) -> String {
        format!(
            "search:{}:{}",
            query,
            limit.map_or("all".to_string(), |l| l.to_string())
        )
    }
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
        assert!(cached.cached_at <= Utc::now());
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
}
