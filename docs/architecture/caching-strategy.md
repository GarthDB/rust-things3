# Caching Strategy Design

This document outlines the comprehensive caching strategy for the Rust Things library, designed to maximize performance while maintaining data consistency.

## Caching Architecture

### Multi-Level Caching

The caching system implements a three-tier architecture:

1. **L1 Cache (Memory)**: Fast in-memory cache for frequently accessed data
2. **L2 Cache (Disk)**: Persistent disk cache for larger datasets
3. **L3 Cache (Database)**: Database-level query result caching

### Cache Hierarchy

```
┌─────────────────┐
│   Application   │
└─────────────────┘
         │
         ▼
┌─────────────────┐
│   L1 Cache      │ ← Memory (Moka)
│   (Memory)      │
└─────────────────┘
         │
         ▼
┌─────────────────┐
│   L2 Cache      │ ← Disk (SQLite)
│   (Disk)        │
└─────────────────┘
         │
         ▼
┌─────────────────┐
│   L3 Cache      │ ← Database
│   (Database)    │
└─────────────────┘
```

## Cache Implementation

### L1 Cache (Memory Cache)

```rust
/// L1 memory cache implementation using Moka
pub struct MemoryCache {
    cache: moka::future::Cache<String, CachedValue>,
    config: MemoryCacheConfig,
    stats: Arc<RwLock<CacheStats>>,
}

/// Memory cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCacheConfig {
    /// Maximum number of entries
    pub max_entries: u64,
    /// Maximum weight (memory usage)
    pub max_weight: u64,
    /// Time to live
    pub ttl: Duration,
    /// Time to idle
    pub tti: Duration,
    /// Eviction policy
    pub eviction_policy: EvictionPolicy,
}

/// Cached value wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedValue {
    /// The actual cached data
    pub data: Vec<u8>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last access timestamp
    pub last_accessed: DateTime<Utc>,
    /// Access count
    pub access_count: u64,
    /// Cache level
    pub level: CacheLevel,
    /// Compression enabled
    pub compressed: bool,
}

/// Cache levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheLevel {
    L1, // Memory
    L2, // Disk
    L3, // Database
}
```

### L2 Cache (Disk Cache)

```rust
/// L2 disk cache implementation using SQLite
pub struct DiskCache {
    db: rusqlite::Connection,
    config: DiskCacheConfig,
    stats: Arc<RwLock<CacheStats>>,
}

/// Disk cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskCacheConfig {
    /// Cache database path
    pub db_path: PathBuf,
    /// Maximum cache size in bytes
    pub max_size: u64,
    /// Time to live
    pub ttl: Duration,
    /// Compression enabled
    pub compression: bool,
    /// Cleanup interval
    pub cleanup_interval: Duration,
}

/// Disk cache schema
const DISK_CACHE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS cache_entries (
    key TEXT PRIMARY KEY,
    value BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    last_accessed INTEGER NOT NULL,
    access_count INTEGER NOT NULL DEFAULT 0,
    compressed BOOLEAN NOT NULL DEFAULT 0,
    size_bytes INTEGER NOT NULL,
    ttl INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cache_created_at ON cache_entries(created_at);
CREATE INDEX IF NOT EXISTS idx_cache_last_accessed ON cache_entries(last_accessed);
CREATE INDEX IF NOT EXISTS idx_cache_ttl ON cache_entries(ttl);
"#;
```

### L3 Cache (Database Cache)

```rust
/// L3 database cache for query results
pub struct DatabaseCache {
    cache: moka::future::Cache<String, QueryResult>,
    config: DatabaseCacheConfig,
    stats: Arc<RwLock<CacheStats>>,
}

/// Database cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseCacheConfig {
    /// Maximum number of cached queries
    pub max_queries: u64,
    /// Time to live for query results
    pub ttl: Duration,
    /// Enable query result caching
    pub enabled: bool,
    /// Cache invalidation strategy
    pub invalidation_strategy: InvalidationStrategy,
}

/// Query result cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Query hash
    pub query_hash: String,
    /// Result data
    pub data: Vec<u8>,
    /// Query parameters
    pub parameters: HashMap<String, Value>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Result count
    pub result_count: usize,
    /// Execution time
    pub execution_time: Duration,
}
```

## Cache Operations

### Unified Cache Interface

```rust
/// Unified cache interface for all cache levels
#[async_trait]
pub trait UnifiedCache {
    /// Get value from cache
    async fn get<K, V>(&self, key: &K) -> Result<Option<V>>
    where
        K: Serialize + Send + Sync,
        V: DeserializeOwned + Send + Sync;
    
    /// Put value in cache
    async fn put<K, V>(&self, key: K, value: V) -> Result<()>
    where
        K: Serialize + Send + Sync,
        V: Serialize + Send + Sync;
    
    /// Remove value from cache
    async fn remove<K>(&self, key: &K) -> Result<()>
    where
        K: Serialize + Send + Sync;
    
    /// Clear all cache entries
    async fn clear(&self) -> Result<()>;
    
    /// Get cache statistics
    fn stats(&self) -> CacheStats;
    
    /// Invalidate cache entries matching pattern
    async fn invalidate_pattern(&self, pattern: &str) -> Result<()>;
    
    /// Warm up cache with frequently accessed data
    async fn warm_up(&self) -> Result<()>;
    
    /// Clean up expired entries
    async fn cleanup(&self) -> Result<()>;
}
```

### Cache Key Strategy

```rust
/// Cache key generation strategy
pub struct CacheKeyStrategy {
    /// Key prefix for different data types
    pub prefixes: HashMap<DataType, String>,
    /// Key separator
    pub separator: String,
    /// Include version in key
    pub include_version: bool,
    /// Version string
    pub version: String,
}

/// Data types for cache key generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataType {
    Task,
    Project,
    Area,
    Tag,
    ChecklistItem,
    QueryResult,
    Statistics,
    Configuration,
}

impl CacheKeyStrategy {
    /// Generate cache key for data type and identifier
    pub fn generate_key(&self, data_type: DataType, id: &str) -> String {
        let prefix = self.prefixes.get(&data_type).unwrap_or(&"unknown".to_string());
        let version = if self.include_version {
            format!("{}{}", self.version, self.separator)
        } else {
            String::new()
        };
        format!("{}{}{}{}", prefix, self.separator, version, id)
    }
    
    /// Generate cache key for query
    pub fn generate_query_key(&self, query: &str, parameters: &HashMap<String, Value>) -> String {
        let query_hash = self.hash_query(query, parameters);
        self.generate_key(DataType::QueryResult, &query_hash)
    }
    
    /// Hash query and parameters
    fn hash_query(&self, query: &str, parameters: &HashMap<String, Value>) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        parameters.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}
```

## Cache Invalidation Strategies

### Invalidation Patterns

```rust
/// Cache invalidation strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvalidationStrategy {
    /// Time-based invalidation
    TimeBased,
    /// Event-based invalidation
    EventBased,
    /// Manual invalidation
    Manual,
    /// Hybrid (time + event)
    Hybrid,
}

/// Invalidation events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvalidationEvent {
    /// Task created
    TaskCreated { uuid: Uuid },
    /// Task updated
    TaskUpdated { uuid: Uuid },
    /// Task deleted
    TaskDeleted { uuid: Uuid },
    /// Project created
    ProjectCreated { uuid: Uuid },
    /// Project updated
    ProjectUpdated { uuid: Uuid },
    /// Project deleted
    ProjectDeleted { uuid: Uuid },
    /// Area created
    AreaCreated { uuid: Uuid },
    /// Area updated
    AreaUpdated { uuid: Uuid },
    /// Area deleted
    AreaDeleted { uuid: Uuid },
    /// Tag created
    TagCreated { uuid: Uuid },
    /// Tag updated
    TagUpdated { uuid: Uuid },
    /// Tag deleted
    TagDeleted { uuid: Uuid },
    /// Bulk operation
    BulkOperation { operation: String, count: usize },
}

/// Cache invalidation manager
pub struct CacheInvalidationManager {
    strategy: InvalidationStrategy,
    event_handlers: HashMap<InvalidationEvent, Vec<InvalidationHandler>>,
    cache: Arc<dyn UnifiedCache>,
}

/// Invalidation handler
pub type InvalidationHandler = Box<dyn Fn(&InvalidationEvent) -> Result<()> + Send + Sync>;

impl CacheInvalidationManager {
    /// Register invalidation handler
    pub fn register_handler(
        &mut self,
        event: InvalidationEvent,
        handler: InvalidationHandler,
    ) {
        self.event_handlers.entry(event).or_insert_with(Vec::new).push(handler);
    }
    
    /// Handle invalidation event
    pub async fn handle_event(&self, event: &InvalidationEvent) -> Result<()> {
        if let Some(handlers) = self.event_handlers.get(event) {
            for handler in handlers {
                handler(event)?;
            }
        }
        Ok(())
    }
}
```

## Cache Statistics and Monitoring

### Cache Statistics

```rust
/// Comprehensive cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// L1 cache statistics
    pub l1: LevelStats,
    /// L2 cache statistics
    pub l2: LevelStats,
    /// L3 cache statistics
    pub l3: LevelStats,
    /// Overall statistics
    pub overall: OverallStats,
    /// Performance metrics
    pub performance: PerformanceMetrics,
}

/// Cache level statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelStats {
    /// Total entries
    pub entries: usize,
    /// Cache size in bytes
    pub size_bytes: usize,
    /// Hit rate (0.0 to 1.0)
    pub hit_rate: f64,
    /// Miss rate (0.0 to 1.0)
    pub miss_rate: f64,
    /// Total hits
    pub hits: u64,
    /// Total misses
    pub misses: u64,
    /// Evictions
    pub evictions: u64,
    /// Expirations
    pub expirations: u64,
    /// Average access time
    pub avg_access_time: Duration,
    /// Average eviction time
    pub avg_eviction_time: Duration,
}

/// Overall cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallStats {
    /// Total entries across all levels
    pub total_entries: usize,
    /// Total size in bytes
    pub total_size_bytes: usize,
    /// Overall hit rate
    pub overall_hit_rate: f64,
    /// Overall miss rate
    pub overall_miss_rate: f64,
    /// Cache efficiency score (0.0 to 1.0)
    pub efficiency_score: f64,
    /// Memory usage percentage
    pub memory_usage_percentage: f64,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average get operation time
    pub avg_get_time: Duration,
    /// Average put operation time
    pub avg_put_time: Duration,
    /// Average remove operation time
    pub avg_remove_time: Duration,
    /// Average cleanup time
    pub avg_cleanup_time: Duration,
    /// Peak memory usage
    pub peak_memory_usage: usize,
    /// Current memory usage
    pub current_memory_usage: usize,
    /// Cache operations per second
    pub operations_per_second: f64,
}
```

### Cache Monitoring

```rust
/// Cache monitoring and alerting
pub struct CacheMonitor {
    stats: Arc<RwLock<CacheStats>>,
    alerts: Vec<CacheAlert>,
    config: MonitoringConfig,
}

/// Cache alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheAlert {
    /// Alert type
    pub alert_type: AlertType,
    /// Threshold value
    pub threshold: f64,
    /// Alert message
    pub message: String,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Enabled status
    pub enabled: bool,
}

/// Alert types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertType {
    HitRateLow,
    MissRateHigh,
    MemoryUsageHigh,
    EvictionRateHigh,
    AccessTimeHigh,
    CacheSizeHigh,
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl CacheMonitor {
    /// Check for alerts
    pub async fn check_alerts(&self) -> Result<Vec<CacheAlert>> {
        let stats = self.stats.read().unwrap();
        let mut triggered_alerts = Vec::new();
        
        for alert in &self.alerts {
            if !alert.enabled {
                continue;
            }
            
            let should_trigger = match alert.alert_type {
                AlertType::HitRateLow => stats.overall.overall_hit_rate < alert.threshold,
                AlertType::MissRateHigh => stats.overall.overall_miss_rate > alert.threshold,
                AlertType::MemoryUsageHigh => stats.overall.memory_usage_percentage > alert.threshold,
                AlertType::EvictionRateHigh => {
                    let total_operations = stats.l1.hits + stats.l1.misses;
                    if total_operations > 0 {
                        (stats.l1.evictions as f64 / total_operations as f64) > alert.threshold
                    } else {
                        false
                    }
                }
                AlertType::AccessTimeHigh => {
                    stats.l1.avg_access_time.as_millis() as f64 > alert.threshold
                }
                AlertType::CacheSizeHigh => {
                    stats.overall.total_size_bytes as f64 > alert.threshold
                }
            };
            
            if should_trigger {
                triggered_alerts.push(alert.clone());
            }
        }
        
        Ok(triggered_alerts)
    }
}
```

## Cache Configuration

### Cache Configuration Management

```rust
/// Cache configuration manager
pub struct CacheConfigManager {
    config: CacheConfig,
    overrides: HashMap<String, Value>,
}

/// Main cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// L1 cache configuration
    pub l1: MemoryCacheConfig,
    /// L2 cache configuration
    pub l2: DiskCacheConfig,
    /// L3 cache configuration
    pub l3: DatabaseCacheConfig,
    /// Key strategy configuration
    pub key_strategy: CacheKeyStrategy,
    /// Invalidation strategy
    pub invalidation: InvalidationStrategy,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    /// Global settings
    pub global: GlobalCacheConfig,
}

/// Global cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalCacheConfig {
    /// Enable caching
    pub enabled: bool,
    /// Default TTL
    pub default_ttl: Duration,
    /// Default TTI
    pub default_tti: Duration,
    /// Compression threshold
    pub compression_threshold: usize,
    /// Cleanup interval
    pub cleanup_interval: Duration,
    /// Statistics collection interval
    pub stats_interval: Duration,
    /// Log cache operations
    pub log_operations: bool,
}

impl CacheConfigManager {
    /// Load configuration from file
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: CacheConfig = toml::from_str(&content)?;
        Ok(Self {
            config,
            overrides: HashMap::new(),
        })
    }
    
    /// Save configuration to file
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(&self.config)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }
    
    /// Apply configuration override
    pub fn apply_override(&mut self, key: String, value: Value) {
        self.overrides.insert(key, value);
    }
}
```

This comprehensive caching strategy provides a robust, high-performance caching system that can handle the demands of a production Things 3 integration library while maintaining data consistency and providing excellent monitoring capabilities.
