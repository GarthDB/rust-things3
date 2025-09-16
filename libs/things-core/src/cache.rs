//! Caching layer for frequently accessed Things 3 data

use crate::models::{Area, Project, Task};
use anyhow::Result;
use chrono::{DateTime, Utc};
use moka::future::Cache;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache
    pub max_capacity: u64,
    /// Time to live for cache entries
    pub ttl: Duration,
    /// Time to idle for cache entries
    pub tti: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 1000,
            ttl: Duration::from_secs(300), // 5 minutes
            tti: Duration::from_secs(60),  // 1 minute
        }
    }
}

/// Cached data wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedData<T> {
    pub data: T,
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl<T> CachedData<T> {
    pub fn new(data: T, ttl: Duration) -> Self {
        let now = Utc::now();
        Self {
            data,
            cached_at: now,
            expires_at: now + chrono::Duration::from_std(ttl).unwrap_or_default(),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
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
            self.hits as f64 / total as f64
        } else {
            0.0
        };
    }
}

/// Main cache manager for Things 3 data
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
}

impl ThingsCache {
    /// Create a new cache with the given configuration
    pub fn new(config: CacheConfig) -> Self {
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

        Self {
            tasks,
            projects,
            areas,
            search_results,
            stats: Arc::new(RwLock::new(CacheStats::default())),
            config,
        }
    }

    /// Create a new cache with default configuration
    pub fn new_default() -> Self {
        Self::new(CacheConfig::default())
    }

    /// Get tasks from cache or execute the provided function
    pub async fn get_tasks<F, Fut>(&self, key: &str, fetcher: F) -> Result<Vec<Task>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<Task>>>,
    {
        if let Some(cached) = self.tasks.get(key).await {
            if !cached.is_expired() {
                self.record_hit();
                return Ok(cached.data);
            }
        }

        self.record_miss();
        let data = fetcher().await?;
        let cached_data = CachedData::new(data.clone(), self.config.ttl);
        self.tasks.insert(key.to_string(), cached_data).await;
        Ok(data)
    }

    /// Get projects from cache or execute the provided function
    pub async fn get_projects<F, Fut>(&self, key: &str, fetcher: F) -> Result<Vec<Project>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<Project>>>,
    {
        if let Some(cached) = self.projects.get(key).await {
            if !cached.is_expired() {
                self.record_hit();
                return Ok(cached.data);
            }
        }

        self.record_miss();
        let data = fetcher().await?;
        let cached_data = CachedData::new(data.clone(), self.config.ttl);
        self.projects.insert(key.to_string(), cached_data).await;
        Ok(data)
    }

    /// Get areas from cache or execute the provided function
    pub async fn get_areas<F, Fut>(&self, key: &str, fetcher: F) -> Result<Vec<Area>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<Area>>>,
    {
        if let Some(cached) = self.areas.get(key).await {
            if !cached.is_expired() {
                self.record_hit();
                return Ok(cached.data);
            }
        }

        self.record_miss();
        let data = fetcher().await?;
        let cached_data = CachedData::new(data.clone(), self.config.ttl);
        self.areas.insert(key.to_string(), cached_data).await;
        Ok(data)
    }

    /// Get search results from cache or execute the provided function
    pub async fn get_search_results<F, Fut>(&self, key: &str, fetcher: F) -> Result<Vec<Task>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<Task>>>,
    {
        if let Some(cached) = self.search_results.get(key).await {
            if !cached.is_expired() {
                self.record_hit();
                return Ok(cached.data);
            }
        }

        self.record_miss();
        let data = fetcher().await?;
        let cached_data = CachedData::new(data.clone(), self.config.ttl);
        self.search_results
            .insert(key.to_string(), cached_data)
            .await;
        Ok(data)
    }

    /// Invalidate all caches
    pub async fn invalidate_all(&self) {
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
}

/// Cache key generators
pub mod keys {
    /// Generate cache key for inbox tasks
    pub fn inbox(limit: Option<usize>) -> String {
        format!(
            "inbox:{}",
            limit.map_or("all".to_string(), |l| l.to_string())
        )
    }

    /// Generate cache key for today's tasks
    pub fn today(limit: Option<usize>) -> String {
        format!(
            "today:{}",
            limit.map_or("all".to_string(), |l| l.to_string())
        )
    }

    /// Generate cache key for projects
    pub fn projects(area_uuid: Option<&str>) -> String {
        format!("projects:{}", area_uuid.unwrap_or("all"))
    }

    /// Generate cache key for areas
    pub fn areas() -> String {
        "areas:all".to_string()
    }

    /// Generate cache key for search results
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
    use std::time::Duration;

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
    async fn test_cache_expiration() {
        let config = CacheConfig {
            max_capacity: 100,
            ttl: Duration::from_millis(10),
            tti: Duration::from_millis(5),
        };
        let cache = ThingsCache::new(config);

        // Insert data
        let _ = cache.get_tasks("test", || async { Ok(vec![]) }).await;

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Should be a miss due to expiration
        let _ = cache.get_tasks("test", || async { Ok(vec![]) }).await;

        let stats = cache.get_stats();
        assert_eq!(stats.misses, 2);
    }
}
