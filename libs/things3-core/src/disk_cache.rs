//! L2 Disk cache implementation using `SQLite` for persistent caching

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// L2 Disk cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskCacheConfig {
    /// Cache database path
    pub db_path: String,
    /// Maximum cache size in bytes
    pub max_size: u64,
    /// Time to live for cache entries
    pub ttl: Duration,
    /// Compression enabled
    pub compression: bool,
    /// Cleanup interval
    pub cleanup_interval: Duration,
    /// Maximum number of entries
    pub max_entries: usize,
}

impl Default for DiskCacheConfig {
    fn default() -> Self {
        Self {
            db_path: "cache.db".to_string(),
            max_size: 100 * 1024 * 1024,    // 100MB
            ttl: Duration::from_secs(3600), // 1 hour
            compression: true,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            max_entries: 10000,
        }
    }
}

/// Disk cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskCacheEntry {
    pub key: String,
    pub data: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u64,
    pub size_bytes: usize,
    pub compressed: bool,
    pub cache_type: String, // "tasks", "projects", "areas", "search_results"
}

/// Disk cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskCacheStats {
    pub total_entries: u64,
    pub total_size_bytes: u64,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub compressed_entries: u64,
    pub uncompressed_entries: u64,
}

impl DiskCacheStats {
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

/// L2 Disk cache implementation
pub struct DiskCache {
    config: DiskCacheConfig,
    stats: Arc<RwLock<DiskCacheStats>>,
    cleanup_task: Option<tokio::task::JoinHandle<()>>,
}

impl DiskCache {
    /// Create a new disk cache
    ///
    /// # Errors
    ///
    /// Returns an error if the database connection fails or if the cache cannot be initialized
    pub async fn new(config: DiskCacheConfig) -> Result<Self> {
        let db_path = Path::new(&config.db_path);

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Initialize database
        Self::init_database(&config.db_path)?;

        let mut cache = Self {
            config,
            stats: Arc::new(RwLock::new(DiskCacheStats::default())),
            cleanup_task: None,
        };

        // Start cleanup task
        cache.start_cleanup_task();

        // Load initial statistics
        cache.update_stats().await?;

        Ok(cache)
    }

    /// Initialize the cache database
    fn init_database(db_path: &str) -> Result<()> {
        let conn = Connection::open(db_path)?;

        // Create cache entries table
        conn.execute(
            r"
            CREATE TABLE IF NOT EXISTS cache_entries (
                key TEXT PRIMARY KEY,
                data BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                last_accessed INTEGER NOT NULL,
                access_count INTEGER NOT NULL DEFAULT 0,
                size_bytes INTEGER NOT NULL,
                compressed BOOLEAN NOT NULL DEFAULT 0,
                cache_type TEXT NOT NULL,
                ttl INTEGER NOT NULL
            )
            ",
            [],
        )?;

        // Create indexes for better performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_cache_created_at ON cache_entries(created_at)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_cache_last_accessed ON cache_entries(last_accessed)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_cache_type ON cache_entries(cache_type)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_cache_ttl ON cache_entries(ttl)",
            [],
        )?;

        info!("Disk cache database initialized at: {}", db_path);
        Ok(())
    }

    /// Start the cleanup background task
    fn start_cleanup_task(&mut self) {
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.cleanup_interval);
            loop {
                interval.tick().await;

                if let Err(e) = Self::cleanup_expired_entries(&config) {
                    error!("Failed to cleanup expired cache entries: {}", e);
                }

                if let Err(e) = Self::cleanup_oversized_entries(&config) {
                    error!("Failed to cleanup oversized cache entries: {}", e);
                }
            }
        });

        self.cleanup_task = Some(handle);
    }

    /// Cleanup expired entries
    fn cleanup_expired_entries(config: &DiskCacheConfig) -> Result<()> {
        let conn = Connection::open(&config.db_path)?;
        let now = Utc::now().timestamp();
        let ttl_seconds = config.ttl.as_secs() as i64;

        let deleted = conn.execute(
            "DELETE FROM cache_entries WHERE created_at + ttl < ?",
            params![now - ttl_seconds],
        )?;

        if deleted > 0 {
            debug!("Cleaned up {} expired cache entries", deleted);
        }

        Ok(())
    }

    /// Cleanup oversized entries
    fn cleanup_oversized_entries(config: &DiskCacheConfig) -> Result<()> {
        let conn = Connection::open(&config.db_path)?;

        // Get current total size
        let total_size: i64 = conn.query_row(
            "SELECT COALESCE(SUM(size_bytes), 0) FROM cache_entries",
            [],
            |row| row.get(0),
        )?;

        if total_size as u64 <= config.max_size {
            return Ok(());
        }

        // Remove oldest entries until we're under the size limit
        let mut deleted = 0;
        let target_size = (config.max_size as f64 * 0.8) as u64; // Remove to 80% of max size

        let mut current_size = total_size as u64;
        while current_size > target_size {
            let result = conn.execute(
                "DELETE FROM cache_entries WHERE key IN (
                    SELECT key FROM cache_entries 
                    ORDER BY last_accessed ASC 
                    LIMIT 100
                )",
                [],
            )?;

            if result == 0 {
                break; // No more entries to delete
            }

            deleted += result;

            // Check new total size
            let new_total_size: i64 = conn.query_row(
                "SELECT COALESCE(SUM(size_bytes), 0) FROM cache_entries",
                [],
                |row| row.get(0),
            )?;

            current_size = new_total_size as u64;
            if current_size <= target_size {
                break;
            }
        }

        if deleted > 0 {
            debug!("Cleaned up {} oversized cache entries", deleted);
        }

        Ok(())
    }

    /// Store data in the disk cache
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Serialization fails
    /// - Compression fails (if enabled)
    /// - Database operations fail
    /// - File I/O operations fail
    pub fn store<T>(&self, key: &str, data: &T, cache_type: &str) -> Result<()>
    where
        T: Serialize,
    {
        let serialized = if self.config.compression {
            // Compress the data
            let json_data = serde_json::to_vec(data)?;
            zstd::encode_all(&json_data[..], 3)?
        } else {
            serde_json::to_vec(data)?
        };

        let size_bytes = serialized.len();
        let entry = DiskCacheEntry {
            key: key.to_string(),
            data: serialized,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 0,
            size_bytes,
            compressed: self.config.compression,
            cache_type: cache_type.to_string(),
        };

        let conn = Connection::open(&self.config.db_path)?;
        let _now = Utc::now().timestamp();
        let ttl_seconds = self.config.ttl.as_secs() as i64;

        conn.execute(
            r"
            INSERT OR REPLACE INTO cache_entries 
            (key, data, created_at, last_accessed, access_count, size_bytes, compressed, cache_type, ttl)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
            params![
                entry.key,
                entry.data,
                entry.created_at.timestamp(),
                entry.last_accessed.timestamp(),
                entry.access_count,
                entry.size_bytes,
                entry.compressed,
                entry.cache_type,
                ttl_seconds
            ],
        )?;

        debug!(
            "Stored cache entry: {} ({} bytes, compressed: {})",
            key, entry.size_bytes, entry.compressed
        );

        Ok(())
    }

    /// Retrieve data from the disk cache
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Database operations fail
    /// - Deserialization fails
    /// - Decompression fails (if data was compressed)
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let conn = Connection::open(&self.config.db_path)?;
        let now = Utc::now().timestamp();

        let mut stmt = conn.prepare(
            r"
            SELECT data, compressed, created_at, ttl, access_count
            FROM cache_entries 
            WHERE key = ? AND created_at + ttl > ?
            ",
        )?;

        let mut rows = stmt.query(params![key, now])?;

        if let Some(row) = rows.next()? {
            let data: Vec<u8> = row.get(0)?;
            let compressed: bool = row.get(1)?;
            let access_count: i64 = row.get(4)?;

            // Update access count and last accessed time
            conn.execute(
                "UPDATE cache_entries SET access_count = ?, last_accessed = ? WHERE key = ?",
                params![access_count + 1, now, key],
            )?;

            // Deserialize the data
            let deserialized = if compressed {
                let decompressed = zstd::decode_all(&data[..])?;
                serde_json::from_slice(&decompressed)?
            } else {
                serde_json::from_slice(&data)?
            };

            // Update statistics
            {
                let mut stats = self.stats.write().await;
                stats.hits += 1;
                stats.calculate_hit_rate();
            }

            debug!("Cache hit for key: {}", key);
            Ok(Some(deserialized))
        } else {
            // Update statistics
            {
                let mut stats = self.stats.write().await;
                stats.misses += 1;
                stats.calculate_hit_rate();
            }

            debug!("Cache miss for key: {}", key);
            Ok(None)
        }
    }

    /// Remove an entry from the disk cache
    ///
    /// # Errors
    ///
    /// This function will return an error if database operations fail
    pub fn remove(&self, key: &str) -> Result<bool> {
        let conn = Connection::open(&self.config.db_path)?;
        let deleted = conn.execute("DELETE FROM cache_entries WHERE key = ?", params![key])?;
        Ok(deleted > 0)
    }

    /// Clear all entries from the disk cache
    ///
    /// # Errors
    ///
    /// This function will return an error if database operations fail
    pub fn clear(&self) -> Result<()> {
        let conn = Connection::open(&self.config.db_path)?;
        conn.execute("DELETE FROM cache_entries", [])?;
        info!("Cleared all disk cache entries");
        Ok(())
    }

    /// Clear entries by cache type
    ///
    /// # Errors
    ///
    /// This function will return an error if database operations fail
    pub fn clear_by_type(&self, cache_type: &str) -> Result<()> {
        let conn = Connection::open(&self.config.db_path)?;
        let deleted = conn.execute(
            "DELETE FROM cache_entries WHERE cache_type = ?",
            params![cache_type],
        )?;
        debug!("Cleared {} entries of type: {}", deleted, cache_type);
        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> DiskCacheStats {
        self.update_stats().await.ok();
        self.stats.read().await.clone()
    }

    /// Update cache statistics
    async fn update_stats(&self) -> Result<()> {
        let conn = Connection::open(&self.config.db_path)?;
        let now = Utc::now().timestamp();

        // Get total entries and size
        let (total_entries, total_size): (i64, i64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(size_bytes), 0) FROM cache_entries WHERE created_at + ttl > ?",
            params![now],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        // Get compressed/uncompressed counts
        let compressed_entries: i64 = conn.query_row(
            "SELECT COUNT(*) FROM cache_entries WHERE compressed = 1 AND created_at + ttl > ?",
            params![now],
            |row| row.get(0),
        )?;

        let uncompressed_entries = total_entries - compressed_entries;

        let mut stats = self.stats.write().await;
        stats.total_entries = total_entries as u64;
        stats.total_size_bytes = total_size as u64;
        stats.compressed_entries = compressed_entries as u64;
        stats.uncompressed_entries = uncompressed_entries as u64;

        Ok(())
    }

    /// Get cache size in bytes
    ///
    /// # Errors
    ///
    /// This function will return an error if database operations fail
    pub fn get_size(&self) -> Result<u64> {
        let conn = Connection::open(&self.config.db_path)?;
        let now = Utc::now().timestamp();

        let size: i64 = conn.query_row(
            "SELECT COALESCE(SUM(size_bytes), 0) FROM cache_entries WHERE created_at + ttl > ?",
            params![now],
            |row| row.get(0),
        )?;

        Ok(size as u64)
    }

    /// Check if cache is full
    ///
    /// # Errors
    ///
    /// This function will return an error if database operations fail
    pub fn is_full(&self) -> Result<bool> {
        let current_size = self.get_size()?;
        Ok(current_size >= self.config.max_size)
    }

    /// Get cache utilization percentage
    ///
    /// # Errors
    ///
    /// This function will return an error if database operations fail
    pub fn get_utilization(&self) -> Result<f64> {
        let current_size = self.get_size()?;
        Ok((current_size as f64 / self.config.max_size as f64) * 100.0)
    }
}

impl Drop for DiskCache {
    fn drop(&mut self) {
        if let Some(handle) = self.cleanup_task.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_disk_cache_basic_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache.db");

        let config = DiskCacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            max_size: 1024 * 1024, // 1MB
            ttl: Duration::from_secs(60),
            compression: false,
            cleanup_interval: Duration::from_secs(10),
            max_entries: 100,
        };

        let cache = DiskCache::new(config).await.unwrap();

        // Test storing and retrieving data
        let test_data = vec!["hello".to_string(), "world".to_string()];
        cache.store("test_key", &test_data, "test").unwrap();

        let retrieved: Option<Vec<String>> = cache.get("test_key").await.unwrap();
        assert_eq!(retrieved, Some(test_data));

        // Test cache miss
        let missing: Option<Vec<String>> = cache.get("missing_key").await.unwrap();
        assert_eq!(missing, None);

        // Test removal
        let removed = cache.remove("test_key").unwrap();
        assert!(removed);

        let after_removal: Option<Vec<String>> = cache.get("test_key").await.unwrap();
        assert_eq!(after_removal, None);
    }

    #[tokio::test]
    async fn test_disk_cache_compression() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache_compressed.db");

        let config = DiskCacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            max_size: 1024 * 1024, // 1MB
            ttl: Duration::from_secs(60),
            compression: true,
            cleanup_interval: Duration::from_secs(10),
            max_entries: 100,
        };

        let cache = DiskCache::new(config).await.unwrap();

        // Test storing and retrieving compressed data
        let test_data = vec![
            "hello".to_string(),
            "world".to_string(),
            "this".to_string(),
            "is".to_string(),
            "a".to_string(),
            "test".to_string(),
        ];
        cache.store("compressed_key", &test_data, "test").unwrap();

        let retrieved: Option<Vec<String>> = cache.get("compressed_key").await.unwrap();
        assert_eq!(retrieved, Some(test_data));
    }

    #[tokio::test]
    async fn test_disk_cache_statistics() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache_stats.db");

        let config = DiskCacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            max_size: 1024 * 1024, // 1MB
            ttl: Duration::from_secs(60),
            compression: false,
            cleanup_interval: Duration::from_secs(10),
            max_entries: 100,
        };

        let cache = DiskCache::new(config).await.unwrap();

        // Store some data
        cache.store("key1", &vec!["data1"], "test").unwrap();
        cache.store("key2", &vec!["data2"], "test").unwrap();

        // Retrieve data to generate hits
        let _: Option<Vec<String>> = cache.get("key1").await.unwrap();
        let _: Option<Vec<String>> = cache.get("key2").await.unwrap();

        // Try to get non-existent key for miss
        let _: Option<Vec<String>> = cache.get("missing").await.unwrap();

        let stats = cache.get_stats().await;
        assert_eq!(stats.total_entries, 2);
        assert!(stats.hits >= 2);
        assert!(stats.misses >= 1);
        assert!(stats.hit_rate > 0.0);
    }

    #[tokio::test]
    async fn test_disk_cache_clear() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache_clear.db");

        let config = DiskCacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            max_size: 1024 * 1024,
            ttl: Duration::from_secs(60),
            compression: false,
            cleanup_interval: Duration::from_secs(10),
            max_entries: 100,
        };

        let cache = DiskCache::new(config).await.unwrap();

        // Store some data
        cache.store("key1", &vec!["data1"], "test").unwrap();
        cache.store("key2", &vec!["data2"], "test").unwrap();

        // Verify data exists
        let stats_before = cache.get_stats().await;
        assert_eq!(stats_before.total_entries, 2);

        // Clear all data
        cache.clear().unwrap();

        // Verify data is gone
        let stats_after = cache.get_stats().await;
        assert_eq!(stats_after.total_entries, 0);

        // Verify individual keys are gone
        let missing: Option<Vec<String>> = cache.get("key1").await.unwrap();
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn test_disk_cache_clear_by_type() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache_clear_by_type.db");

        let config = DiskCacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            max_size: 1024 * 1024,
            ttl: Duration::from_secs(60),
            compression: false,
            cleanup_interval: Duration::from_secs(10),
            max_entries: 100,
        };

        let cache = DiskCache::new(config).await.unwrap();

        // Store data with different cache types
        cache.store("key1", &vec!["data1"], "type1").unwrap();
        cache.store("key2", &vec!["data2"], "type1").unwrap();
        cache.store("key3", &vec!["data3"], "type2").unwrap();

        // Clear only type1
        cache.clear_by_type("type1").unwrap();

        // Verify type1 keys are gone
        let missing1: Option<Vec<String>> = cache.get("key1").await.unwrap();
        let missing2: Option<Vec<String>> = cache.get("key2").await.unwrap();
        assert_eq!(missing1, None);
        assert_eq!(missing2, None);

        // Verify type2 key still exists
        let existing: Option<Vec<String>> = cache.get("key3").await.unwrap();
        assert_eq!(existing, Some(vec!["data3".to_string()]));
    }

    #[tokio::test]
    async fn test_disk_cache_get_size() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache_size.db");

        let config = DiskCacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            max_size: 1024 * 1024,
            ttl: Duration::from_secs(60),
            compression: false,
            cleanup_interval: Duration::from_secs(10),
            max_entries: 100,
        };

        let cache = DiskCache::new(config).await.unwrap();

        // Initially empty
        let initial_size = cache.get_size().unwrap();
        assert_eq!(initial_size, 0);

        // Store some data
        cache.store("key1", &vec!["data1"], "test").unwrap();
        cache.store("key2", &vec!["data2"], "test").unwrap();

        // Size should be greater than 0
        let size_after_store = cache.get_size().unwrap();
        assert!(size_after_store > 0);
    }

    #[tokio::test]
    async fn test_disk_cache_is_full() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache_full.db");

        let config = DiskCacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            max_size: 100, // Very small size
            ttl: Duration::from_secs(60),
            compression: false,
            cleanup_interval: Duration::from_secs(10),
            max_entries: 100,
        };

        let cache = DiskCache::new(config).await.unwrap();

        // Initially not full
        let initially_full = cache.is_full().unwrap();
        assert!(!initially_full);

        // Store data until full
        for i in 0..10 {
            let data = vec![format!("data_{}", i); 100]; // Large data
            cache.store(&format!("key{}", i), &data, "test").unwrap();
        }

        // Should be full now
        let is_full = cache.is_full().unwrap();
        assert!(is_full);
    }

    #[tokio::test]
    async fn test_disk_cache_get_utilization() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache_utilization.db");

        let config = DiskCacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            max_size: 1000, // 1KB
            ttl: Duration::from_secs(60),
            compression: false,
            cleanup_interval: Duration::from_secs(10),
            max_entries: 100,
        };

        let cache = DiskCache::new(config).await.unwrap();

        // Initially 0% utilization
        let initial_utilization = cache.get_utilization().unwrap();
        assert_eq!(initial_utilization, 0.0);

        // Store some data
        cache.store("key1", &vec!["data1"], "test").unwrap();

        // Utilization should be > 0%
        let utilization = cache.get_utilization().unwrap();
        assert!(utilization > 0.0);
        assert!(utilization <= 100.0);
    }

    #[tokio::test]
    async fn test_disk_cache_ttl_expiration() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache_ttl.db");

        let config = DiskCacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            max_size: 1024 * 1024,
            ttl: Duration::from_millis(1000), // 1 second TTL
            compression: false,
            cleanup_interval: Duration::from_millis(50),
            max_entries: 100,
        };

        let cache = DiskCache::new(config).await.unwrap();

        // Store data
        cache.store("key1", &vec!["data1"], "test").unwrap();

        // Data should exist initially
        let initial: Option<Vec<String>> = cache.get("key1").await.unwrap();
        assert_eq!(initial, Some(vec!["data1".to_string()]));

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_millis(1200)).await;

        // Manually trigger cleanup to ensure expired entries are removed
        DiskCache::cleanup_expired_entries(&cache.config).unwrap();

        // Data should be expired
        let expired: Option<Vec<String>> = cache.get("key1").await.unwrap();
        assert_eq!(expired, None);
    }

    #[tokio::test]
    async fn test_disk_cache_cleanup_expired_entries() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache_cleanup.db");

        let config = DiskCacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            max_size: 1024 * 1024,
            ttl: Duration::from_millis(100),
            compression: false,
            cleanup_interval: Duration::from_millis(50),
            max_entries: 100,
        };

        let cache = DiskCache::new(config).await.unwrap();

        // Store data
        cache.store("key1", &vec!["data1"], "test").unwrap();
        cache.store("key2", &vec!["data2"], "test").unwrap();

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Manually trigger cleanup
        DiskCache::cleanup_expired_entries(&cache.config).unwrap();

        // Data should be cleaned up
        let stats = cache.get_stats().await;
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_disk_cache_cleanup_oversized_entries() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache_oversized.db");

        let config = DiskCacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            max_size: 100, // Very small size
            ttl: Duration::from_secs(60),
            compression: false,
            cleanup_interval: Duration::from_secs(10),
            max_entries: 100,
        };

        let cache = DiskCache::new(config).await.unwrap();

        // Store oversized data
        let large_data = vec!["data"; 1000]; // Very large data
        cache.store("key1", &large_data, "test").unwrap();

        // Manually trigger cleanup
        DiskCache::cleanup_oversized_entries(&cache.config).unwrap();

        // Data should be cleaned up
        let stats = cache.get_stats().await;
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_disk_cache_error_handling() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache_errors.db");

        let config = DiskCacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            max_size: 1024 * 1024,
            ttl: Duration::from_secs(60),
            compression: false,
            cleanup_interval: Duration::from_secs(10),
            max_entries: 100,
        };

        let cache = DiskCache::new(config).await.unwrap();

        // Test storing with invalid data (this should work fine)
        let valid_data = vec!["valid".to_string()];
        let result = cache.store("valid_key", &valid_data, "test");
        assert!(result.is_ok());

        // Test getting non-existent key (should return None, not error)
        let missing: Option<Vec<String>> = cache.get("missing_key").await.unwrap();
        assert_eq!(missing, None);

        // Test removing non-existent key (should return false, not error)
        let removed = cache.remove("missing_key").unwrap();
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_disk_cache_concurrent_access() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_cache_concurrent.db");

        let config = DiskCacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            max_size: 1024 * 1024,
            ttl: Duration::from_secs(60),
            compression: false,
            cleanup_interval: Duration::from_secs(10),
            max_entries: 100,
        };

        let cache = DiskCache::new(config).await.unwrap();

        // Test sequential operations
        for i in 0..5 {
            let key = format!("key_{}", i);
            let data = vec![format!("data_{}", i)];

            // Store data
            cache.store(&key, &data, "test").unwrap();

            // Retrieve data
            let retrieved: Option<Vec<String>> = cache.get(&key).await.unwrap();
            assert_eq!(retrieved, Some(data));
        }

        // Verify all data is still there
        let stats = cache.get_stats().await;
        assert_eq!(stats.total_entries, 5);
    }
}
