//! Caching middleware for MCP (Model Context Protocol) tool results

use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// MCP tool result cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPCacheEntry<T> {
    pub tool_name: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub result: T,
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub access_count: u64,
    pub last_accessed: DateTime<Utc>,
    pub cache_key: String,
    pub result_size_bytes: usize,
    pub compression_ratio: f64,
}

/// MCP cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPCacheConfig {
    /// Maximum number of cached results
    pub max_entries: usize,
    /// Time to live for cache entries
    pub ttl: Duration,
    /// Time to idle for cache entries
    pub tti: Duration,
    /// Enable compression for large results
    pub enable_compression: bool,
    /// Compression threshold in bytes
    pub compression_threshold: usize,
    /// Maximum result size to cache
    pub max_result_size: usize,
    /// Enable cache warming for frequently used tools
    pub enable_cache_warming: bool,
    /// Cache warming interval
    pub warming_interval: Duration,
}

impl Default for MCPCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            ttl: Duration::from_secs(3600), // 1 hour
            tti: Duration::from_secs(300),  // 5 minutes
            enable_compression: true,
            compression_threshold: 1024,       // 1KB
            max_result_size: 10 * 1024 * 1024, // 10MB
            enable_cache_warming: true,
            warming_interval: Duration::from_secs(60), // 1 minute
        }
    }
}

/// MCP cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MCPCacheStats {
    pub total_entries: u64,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub total_size_bytes: u64,
    pub compressed_entries: u64,
    pub uncompressed_entries: u64,
    pub evictions: u64,
    pub warming_entries: u64,
    pub average_access_time_ms: f64,
}

impl MCPCacheStats {
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

/// MCP tool cache middleware
pub struct MCPCacheMiddleware<T> {
    /// Cache entries by tool name and parameters
    cache: Arc<RwLock<HashMap<String, MCPCacheEntry<T>>>>,
    /// Configuration
    config: MCPCacheConfig,
    /// Statistics
    stats: Arc<RwLock<MCPCacheStats>>,
    /// Cache warming entries (key -> priority)
    warming_entries: Arc<RwLock<HashMap<String, u32>>>,
    /// Cache warming task handle
    warming_task: Option<tokio::task::JoinHandle<()>>,
}

impl<T> MCPCacheMiddleware<T>
where
    T: Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    /// Create a new MCP cache middleware
    #[must_use]
    pub fn new(config: &MCPCacheConfig) -> Self {
        let mut middleware = Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
            stats: Arc::new(RwLock::new(MCPCacheStats::default())),
            warming_entries: Arc::new(RwLock::new(HashMap::new())),
            warming_task: None,
        };

        // Start cache warming task if enabled
        if config.enable_cache_warming {
            middleware.start_cache_warming();
        }

        middleware
    }

    /// Create a new middleware with default configuration
    #[must_use]
    pub fn new_default() -> Self {
        Self::new(&MCPCacheConfig::default())
    }

    /// Execute a tool with caching
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Tool execution fails
    /// - Cache operations fail
    /// - Serialization/deserialization fails
    pub async fn execute_tool<F, Fut>(
        &self,
        tool_name: &str,
        parameters: HashMap<String, serde_json::Value>,
        tool_executor: F,
    ) -> Result<T>
    where
        F: FnOnce(HashMap<String, serde_json::Value>) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let cache_key = Self::generate_cache_key(tool_name, &parameters);

        // Check cache first
        if let Some(cached_entry) = self.get_cached_entry(&cache_key) {
            if !cached_entry.is_expired() && !cached_entry.is_idle(self.config.tti) {
                self.record_hit();
                debug!(
                    "MCP cache hit for tool: {} with key: {}",
                    tool_name, cache_key
                );
                return Ok(cached_entry.result);
            }
        }

        // Cache miss - execute tool
        self.record_miss();
        let start_time = std::time::Instant::now();

        let result = tool_executor(parameters.clone()).await?;
        let execution_time = start_time.elapsed();

        // Check if result is too large to cache
        let result_size = Self::calculate_result_size(&result);
        if result_size > self.config.max_result_size {
            warn!("MCP tool result too large to cache: {} bytes", result_size);
            return Ok(result);
        }

        // Cache the result
        self.cache_result(
            tool_name,
            parameters,
            result.clone(),
            &cache_key,
            result_size,
        );

        debug!(
            "MCP tool executed and cached: {} ({}ms, {} bytes)",
            tool_name,
            execution_time.as_millis(),
            result_size
        );

        Ok(result)
    }

    /// Get a cached result without executing the tool
    #[must_use]
    pub fn get_cached_result(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Option<T> {
        let cache_key = Self::generate_cache_key(tool_name, parameters);

        if let Some(cached_entry) = self.get_cached_entry(&cache_key) {
            if !cached_entry.is_expired() && !cached_entry.is_idle(self.config.tti) {
                self.record_hit();
                return Some(cached_entry.result);
            }
        }

        self.record_miss();
        None
    }

    /// Invalidate cache entries for a specific tool
    pub fn invalidate_tool(&self, tool_name: &str) {
        let mut cache = self.cache.write();
        let keys_to_remove: Vec<String> = cache
            .iter()
            .filter(|(_, entry)| entry.tool_name == tool_name)
            .map(|(key, _)| key.clone())
            .collect();

        let count = keys_to_remove.len();
        for key in keys_to_remove {
            cache.remove(&key);
        }

        debug!(
            "Invalidated {} cache entries for tool: {}",
            count, tool_name
        );
    }

    /// Invalidate all cache entries
    pub fn invalidate_all(&self) {
        let mut cache = self.cache.write();
        cache.clear();
        info!("Invalidated all MCP cache entries");
    }

    /// Get cache statistics
    #[must_use]
    pub fn get_stats(&self) -> MCPCacheStats {
        let mut stats = self.stats.read().clone();
        stats.calculate_hit_rate();
        stats
    }

    /// Get cache size in bytes
    #[must_use]
    pub fn get_cache_size(&self) -> usize {
        let cache = self.cache.read();
        cache.values().map(|entry| entry.result_size_bytes).sum()
    }

    /// Get cache utilization percentage
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn get_utilization(&self) -> f64 {
        let current_size = self.get_cache_size();
        let max_size = self.config.max_entries * self.config.max_result_size;
        (current_size as f64 / max_size as f64) * 100.0
    }

    /// Generate cache key from tool name and parameters
    fn generate_cache_key(
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut key_parts = vec![tool_name.to_string()];

        // Sort parameters for consistent key generation
        let mut sorted_params: Vec<_> = parameters.iter().collect();
        sorted_params.sort_by_key(|(k, _)| *k);

        for (param_name, param_value) in sorted_params {
            key_parts.push(format!("{param_name}:{param_value}"));
        }

        // Use a hash of the key parts to keep it manageable
        let mut hasher = DefaultHasher::new();
        key_parts.join("|").hash(&mut hasher);
        format!("mcp:{}:{}", tool_name, hasher.finish())
    }

    /// Get a cached entry
    fn get_cached_entry(&self, cache_key: &str) -> Option<MCPCacheEntry<T>> {
        let mut cache = self.cache.write();
        if let Some(entry) = cache.get_mut(cache_key) {
            entry.access_count += 1;
            entry.last_accessed = Utc::now();
            Some(entry.clone())
        } else {
            None
        }
    }

    /// Cache a tool result
    fn cache_result(
        &self,
        tool_name: &str,
        parameters: HashMap<String, serde_json::Value>,
        result: T,
        cache_key: &str,
        result_size: usize,
    ) {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::from_std(self.config.ttl).unwrap_or_default();

        let entry = MCPCacheEntry {
            tool_name: tool_name.to_string(),
            parameters,
            result,
            cached_at: now,
            expires_at,
            access_count: 0,
            last_accessed: now,
            cache_key: cache_key.to_string(),
            result_size_bytes: result_size,
            compression_ratio: 1.0, // TODO: Implement compression
        };

        // Check if we need to evict entries
        self.evict_if_needed();

        let mut cache = self.cache.write();
        cache.insert(cache_key.to_string(), entry);

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.total_entries += 1;
            stats.total_size_bytes += result_size as u64;
        }
    }

    /// Calculate result size in bytes
    fn calculate_result_size(result: &T) -> usize {
        serde_json::to_vec(result).map_or(0, |bytes| bytes.len())
    }

    /// Evict entries if cache is full
    fn evict_if_needed(&self) {
        let mut cache = self.cache.write();

        if cache.len() >= self.config.max_entries {
            // Remove oldest entries (LRU)
            let mut entries: Vec<_> = cache
                .iter()
                .map(|(k, v)| (k.clone(), v.last_accessed))
                .collect();
            entries.sort_by_key(|(_, last_accessed)| *last_accessed);

            let entries_to_remove = cache.len() - self.config.max_entries + 1;
            for (key, _) in entries.iter().take(entries_to_remove) {
                cache.remove(key);
            }

            // Update statistics
            {
                let mut stats = self.stats.write();
                stats.evictions += entries_to_remove as u64;
            }
        }
    }

    /// Start cache warming background task
    fn start_cache_warming(&mut self) {
        let warming_entries = Arc::clone(&self.warming_entries);
        let warming_interval = self.config.warming_interval;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(warming_interval);
            loop {
                interval.tick().await;

                // In a real implementation, you would warm frequently accessed entries
                // by calling the appropriate tool executors
                let entries_count = {
                    let entries = warming_entries.read();
                    entries.len()
                };

                if entries_count > 0 {
                    debug!("MCP cache warming {} entries", entries_count);
                }
            }
        });

        self.warming_task = Some(handle);
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

impl<T> MCPCacheEntry<T> {
    /// Check if the cache entry is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if the cache entry is idle
    pub fn is_idle(&self, tti: Duration) -> bool {
        let now = Utc::now();
        let idle_duration = now - self.last_accessed;
        idle_duration > chrono::Duration::from_std(tti).unwrap_or_default()
    }
}

impl<T> Drop for MCPCacheMiddleware<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.warming_task.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_mcp_cache_basic_operations() {
        let middleware = MCPCacheMiddleware::<String>::new_default();

        let mut parameters = HashMap::new();
        parameters.insert(
            "query".to_string(),
            serde_json::Value::String("test".to_string()),
        );

        // First call - should be a cache miss
        let result1 = middleware
            .execute_tool("test_tool", parameters.clone(), |_| async {
                Ok("test_result".to_string())
            })
            .await
            .unwrap();

        assert_eq!(result1, "test_result");

        // Second call - should be a cache hit
        let result2 = middleware
            .execute_tool("test_tool", parameters, |_| async {
                panic!("Should not execute on cache hit")
            })
            .await
            .unwrap();

        assert_eq!(result2, "test_result");

        let stats = middleware.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 0.5).abs() < 1e-9);
    }

    #[tokio::test]
    async fn test_mcp_cache_invalidation() {
        let middleware = MCPCacheMiddleware::<String>::new_default();

        let mut parameters = HashMap::new();
        parameters.insert(
            "query".to_string(),
            serde_json::Value::String("test".to_string()),
        );

        // Cache a result
        middleware
            .execute_tool("test_tool", parameters.clone(), |_| async {
                Ok("test_result".to_string())
            })
            .await
            .unwrap();

        // Verify it's cached
        let cached = middleware.get_cached_result("test_tool", &parameters);
        assert!(cached.is_some());

        // Invalidate the tool
        middleware.invalidate_tool("test_tool");

        // Verify it's no longer cached
        let cached = middleware.get_cached_result("test_tool", &parameters);
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_mcp_cache_key_generation() {
        let _middleware = MCPCacheMiddleware::<String>::new_default();

        let mut params1 = HashMap::new();
        params1.insert("a".to_string(), serde_json::Value::String("1".to_string()));
        params1.insert("b".to_string(), serde_json::Value::String("2".to_string()));

        let mut params2 = HashMap::new();
        params2.insert("b".to_string(), serde_json::Value::String("2".to_string()));
        params2.insert("a".to_string(), serde_json::Value::String("1".to_string()));

        // Same parameters in different order should generate same key
        let key1 = MCPCacheMiddleware::<String>::generate_cache_key("test_tool", &params1);
        let key2 = MCPCacheMiddleware::<String>::generate_cache_key("test_tool", &params2);
        assert_eq!(key1, key2);
    }
}
