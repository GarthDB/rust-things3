//! Cache edge case tests
//!
//! Tests various edge cases in cache functionality to ensure
//! robust handling of extreme values, concurrent access, and edge conditions.

use std::time::Duration;
use things3_core::{CacheConfig, CacheStats, ThingsCache};

/// Test cache with zero capacity
#[tokio::test]
async fn test_cache_zero_capacity() {
    let mut config = CacheConfig::default();
    config.max_capacity = 0;

    // Should create cache even with zero capacity
    let _cache = ThingsCache::new(&config);
}

/// Test cache with very large capacity
#[tokio::test]
async fn test_cache_very_large_capacity() {
    let mut config = CacheConfig::default();
    config.max_capacity = 1_000_000; // Large but reasonable

    let _cache = ThingsCache::new(&config);
}

/// Test cache with very short TTL
#[tokio::test]
async fn test_cache_very_short_ttl() {
    let mut config = CacheConfig::default();
    config.ttl = Duration::from_secs(1);

    let _cache = ThingsCache::new(&config);
}

/// Test cache with very long TTL
#[tokio::test]
async fn test_cache_very_long_ttl() {
    let mut config = CacheConfig::default();
    config.ttl = Duration::from_secs(86400 * 30); // 30 days

    let _cache = ThingsCache::new(&config);
}

/// Test cache stats hit rate calculation with zero hits/misses
#[test]
fn test_cache_stats_zero_hits_misses() {
    let mut stats = CacheStats {
        hits: 0,
        misses: 0,
        entries: 0,
        hit_rate: 0.0,
    };

    stats.calculate_hit_rate();
    assert_eq!(
        stats.hit_rate, 0.0,
        "Hit rate should be 0 with no hits or misses"
    );
}

/// Test cache stats hit rate calculation with only hits
#[test]
fn test_cache_stats_only_hits() {
    let mut stats = CacheStats {
        hits: 100,
        misses: 0,
        entries: 50,
        hit_rate: 0.0,
    };

    stats.calculate_hit_rate();
    assert_eq!(stats.hit_rate, 1.0, "Hit rate should be 1.0 with only hits");
}

/// Test cache stats hit rate calculation with only misses
#[test]
fn test_cache_stats_only_misses() {
    let mut stats = CacheStats {
        hits: 0,
        misses: 100,
        entries: 0,
        hit_rate: 0.0,
    };

    stats.calculate_hit_rate();
    assert_eq!(
        stats.hit_rate, 0.0,
        "Hit rate should be 0.0 with only misses"
    );
}

/// Test cache stats hit rate calculation with mixed hits/misses
#[test]
fn test_cache_stats_mixed() {
    let mut stats = CacheStats {
        hits: 75,
        misses: 25,
        entries: 50,
        hit_rate: 0.0,
    };

    stats.calculate_hit_rate();
    assert_eq!(
        stats.hit_rate, 0.75,
        "Hit rate should be 0.75 with 75/100 hits"
    );
}

/// Test cache with different configurations
#[tokio::test]
async fn test_various_cache_configurations() {
    // Test with different TTL values
    let ttls = vec![1, 60, 3600, 86400];

    for ttl in ttls {
        let mut config = CacheConfig::default();
        config.ttl = Duration::from_secs(ttl);

        let _cache = ThingsCache::new(&config);
    }
}

/// Test cache with warming enabled
#[tokio::test]
async fn test_cache_with_warming_enabled() {
    let mut config = CacheConfig::default();
    config.enable_cache_warming = true;
    config.warming_interval = Duration::from_secs(1);
    config.max_warming_entries = 10;

    let _cache = ThingsCache::new(&config);
    // Cache warming task should be started
}

/// Test cache with zero warming entries
#[tokio::test]
async fn test_cache_zero_warming_entries() {
    let mut config = CacheConfig::default();
    config.enable_cache_warming = true;
    config.max_warming_entries = 0;

    let _cache = ThingsCache::new(&config);
}

/// Test default cache configuration
#[test]
fn test_default_cache_config() {
    let config = CacheConfig::default();

    assert!(config.max_capacity > 0);
    assert!(config.ttl.as_secs() > 0);
    assert!(config.tti.as_secs() > 0);
}

/// Test default cache creation
#[tokio::test]
async fn test_default_cache_creation() {
    let cache = ThingsCache::new_default();
    let stats = cache.get_stats();

    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.entries, 0);
}

/// Test cache stats default values
#[test]
fn test_cache_stats_default() {
    let stats = CacheStats::default();

    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.entries, 0);
    assert_eq!(stats.hit_rate, 0.0);
}

/// Test cache with TTL greater than TTI
#[tokio::test]
async fn test_cache_ttl_greater_than_tti() {
    let mut config = CacheConfig::default();
    config.ttl = Duration::from_secs(3600);
    config.tti = Duration::from_secs(300);

    let _cache = ThingsCache::new(&config);
}

/// Test cache with TTI greater than TTL
#[tokio::test]
async fn test_cache_tti_greater_than_ttl() {
    let mut config = CacheConfig::default();
    config.ttl = Duration::from_secs(300);
    config.tti = Duration::from_secs(3600);

    let _cache = ThingsCache::new(&config);
}
