use crate::models::ThingsId;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
    pub entity_id: Option<ThingsId>,
    /// The operation that would invalidate this cache entry
    pub invalidating_operations: Vec<String>,
}

impl CacheDependency {
    /// Test whether this dependency matches a mutation on `(entity_type, entity_id)`.
    ///
    /// `entity_id == None` on either side acts as a wildcard: a dependency with
    /// no specific id matches any concrete mutation of the same type, and a
    /// caller passing `None` matches every dependency of that type.
    #[must_use]
    pub fn matches(&self, entity_type: &str, entity_id: Option<&ThingsId>) -> bool {
        if self.entity_type != entity_type {
            return false;
        }
        match (&self.entity_id, entity_id) {
            (Some(dep_id), Some(req_id)) => dep_id == req_id,
            _ => true,
        }
    }

    /// Test whether this dependency lists `operation` as one of its invalidators.
    #[must_use]
    pub fn matches_operation(&self, operation: &str) -> bool {
        self.invalidating_operations
            .iter()
            .any(|op| op == operation)
    }
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
