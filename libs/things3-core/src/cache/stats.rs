use crate::models::ThingsId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::config::CacheDependency;

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

    pub fn has_dependency(&self, entity_type: &str, entity_id: Option<&ThingsId>) -> bool {
        self.dependencies
            .iter()
            .any(|dep| dep.matches(entity_type, entity_id))
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: u64,
    pub hit_rate: f64,
    /// Total number of times the warming loop has called `preloader.warm(key)`.
    pub warmed_keys: u64,
    /// Total number of warming loop ticks that dispatched at least one key to the registered preloader.
    pub warming_runs: u64,
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

/// Hook for predictive cache preloading.
///
/// `ThingsCache` calls [`CachePreloader::predict`] after every `get_*` access
/// (hit or miss) to ask "given that key X was just accessed, what should we
/// queue for background warming?" The returned `(key, priority)` pairs are
/// pushed into the cache's priority queue via [`ThingsCache::add_to_warming`].
///
/// On each warming-loop tick, the cache picks the top-priority queued keys
/// and calls [`CachePreloader::warm`] for each. The implementor is expected
/// to fetch the data and populate the cache (typically by `tokio::spawn`ing
/// a task that calls back into `cache.get_*(key, fetcher)`). `warm` is
/// fire-and-forget — errors must be handled internally.
///
/// The trait is synchronous to stay dyn-compatible without `async-trait`.
/// Implementors that need async work should spawn it inside `warm`.
pub trait CachePreloader: Send + Sync + 'static {
    /// Called after a cache access. Returns `(key, priority)` pairs to enqueue
    /// for background warming. Return `vec![]` to opt out for this access.
    fn predict(&self, accessed_key: &str) -> Vec<(String, u32)>;

    /// Called by the warming loop for each top-priority queued key.
    /// Implementor fetches and populates the cache, typically via `tokio::spawn`.
    fn warm(&self, key: &str);
}
