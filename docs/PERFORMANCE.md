# Performance Characteristics

This document describes the performance characteristics of the `rust-things3` library, including benchmark results, optimization strategies, and performance considerations.

## Overview

The library is designed for high performance with the following key features:
- **Async/await** throughout for non-blocking I/O
- **Connection pooling** with SQLx for efficient database access
- **Query caching** using Moka for frequently accessed data
- **Batch operations** for efficient bulk updates
- **Zero-copy deserialization** where possible

## Benchmark Suite

We maintain a comprehensive benchmark suite using [Criterion.rs](https://github.com/bheisler/criterion.rs) that covers:

1. **Database Query Benchmarks** (`database_benchmarks.rs`)
   - `get_inbox` - Retrieve inbox tasks
   - `get_today` - Retrieve today's tasks
   - `search_tasks` - Full-text search
   - `get_projects` - Retrieve all projects
   - `get_areas` - Retrieve all areas
   - `get_stats` - Calculate statistics

2. **Bulk Operations Benchmarks** (`bulk_operations_benchmarks.rs`)
   - `bulk_move` - Move multiple tasks
   - `bulk_complete` - Complete multiple tasks
   - `bulk_delete` - Delete multiple tasks

3. **Cache Performance Benchmarks** (`cache_benchmarks.rs`)
   - `cache_cold_read` - First read (cache miss)
   - `cache_warm_read` - Subsequent reads (cache hit)
   - `cache_hit_rate` - Mixed query patterns
   - `cache_eviction` - Large dataset queries

## Running Benchmarks

### Locally

```bash
# Run all benchmarks
cargo bench --workspace --features test-utils

# Run specific benchmark suite
cargo bench --package things3-core --bench database_benchmarks --features test-utils

# Run with baseline comparison
cargo bench --workspace --features test-utils -- --save-baseline main
cargo bench --workspace --features test-utils -- --baseline main
```

### In CI

Benchmarks run automatically on:
- Every push to `main`
- Every pull request
- Manual workflow dispatch

Results are:
- Stored as artifacts
- Compared against baselines
- Posted as PR comments

## Performance Characteristics

### Database Queries

| Operation | 10 tasks | 50 tasks | 100 tasks | 500 tasks | 1000 tasks |
|-----------|----------|----------|-----------|-----------|------------|
| `get_inbox` | ~50μs | ~150μs | ~250μs | ~1ms | ~2ms |
| `get_today` | ~50μs | ~150μs | ~250μs | ~1ms | ~2ms |
| `search_tasks` | ~100μs | ~300μs | ~500μs | ~2.5ms | ~5ms |
| `get_projects` | ~30μs | ~100μs | ~180μs | ~800μs | ~1.5ms |
| `get_areas` | ~20μs | ~50μs | ~80μs | ~300μs | ~500μs |

*Note: These are approximate values. Actual performance depends on hardware, database size, and system load.*

### Bulk Operations

| Operation | 10 tasks | 50 tasks | 100 tasks | 500 tasks | 1000 tasks |
|-----------|----------|----------|-----------|-----------|------------|
| `bulk_move` | ~200μs | ~800μs | ~1.5ms | ~7ms | ~14ms |
| `bulk_complete` | ~150μs | ~600μs | ~1.2ms | ~6ms | ~12ms |
| `bulk_delete` | ~180μs | ~700μs | ~1.4ms | ~6.5ms | ~13ms |

**Key Characteristics**:
- **Linear scaling**: O(n) with number of tasks
- **Transactional**: All-or-nothing semantics
- **Batch validation**: Single query for existence checks (not N+1)
- **Max batch size**: 1000 tasks per operation

### Cache Performance

| Metric | Cold Read | Warm Read | Improvement |
|--------|-----------|-----------|-------------|
| 100 tasks | ~250μs | ~5μs | **50x faster** |
| 500 tasks | ~1ms | ~10μs | **100x faster** |
| 1000 tasks | ~2ms | ~15μs | **133x faster** |

**Cache Hit Rate**:
- **Typical workload**: 80-95% hit rate
- **Cache size**: Configurable (default: 10,000 entries)
- **Eviction policy**: LRU (Least Recently Used)
- **TTL**: 5 minutes (configurable)

## Optimization Strategies

### 1. Connection Pooling

```rust
// Optimal pool configuration
let pool = SqlitePoolOptions::new()
    .max_connections(5)  // Adjust based on workload
    .min_connections(1)
    .acquire_timeout(Duration::from_secs(30))
    .idle_timeout(Duration::from_secs(600))
    .connect(&database_url)
    .await?;
```

**Recommendations**:
- **Read-heavy**: 3-5 connections
- **Write-heavy**: 1-2 connections (SQLite write serialization)
- **Mixed**: 3-5 connections

### 2. Query Caching

The library uses Moka for caching frequently accessed data:

```rust
// Cache configuration
Cache::builder()
    .max_capacity(10_000)
    .time_to_live(Duration::from_secs(300))
    .build()
```

**Best Practices**:
- Cache read-heavy queries (inbox, today, projects)
- Invalidate on writes
- Use TTL to prevent stale data
- Monitor cache hit rate

### 3. Batch Operations

Always use bulk operations for multiple tasks:

```rust
// ❌ Slow: Individual operations
for uuid in task_uuids {
    db.complete_task(uuid).await?;
}

// ✅ Fast: Bulk operation
db.bulk_complete(BulkCompleteRequest { task_uuids }).await?;
```

**Performance Gain**: 10-50x faster for large batches

### 4. Prepared Statements

SQLx automatically uses prepared statements for:
- Reduced parsing overhead
- Better query plan caching
- Protection against SQL injection

### 5. Async I/O

All database operations are async:
- Non-blocking I/O
- Efficient resource utilization
- High concurrency support

## Performance Considerations

### Database Size

| Tasks | Projects | Areas | Typical Query Time |
|-------|----------|-------|-------------------|
| 100 | 10 | 5 | < 1ms |
| 1,000 | 50 | 10 | 1-5ms |
| 10,000 | 200 | 20 | 5-20ms |
| 100,000+ | 1000+ | 50+ | 20-100ms+ |

**Recommendations**:
- For databases with 10,000+ tasks, consider pagination
- Use search filters to reduce result sets
- Monitor query performance with tracing

### Memory Usage

| Component | Typical Usage | Peak Usage |
|-----------|---------------|------------|
| Connection Pool | 5-10 MB | 20 MB |
| Query Cache | 10-50 MB | 100 MB |
| Task Objects | ~1 KB each | Varies |
| Total (1000 tasks) | ~20 MB | ~50 MB |

### Concurrency

- **Read operations**: Fully concurrent (no locks)
- **Write operations**: Serialized by SQLite
- **Bulk operations**: Transactional (atomic)

**Recommendations**:
- Use read replicas for read-heavy workloads (if supported)
- Batch writes when possible
- Avoid long-running transactions

## Performance Regression Detection

Our CI pipeline automatically detects performance regressions:

1. **Baseline**: Benchmarks from `main` branch
2. **Comparison**: Benchmarks from PR branch
3. **Threshold**: >10% slowdown triggers warning
4. **Review**: Manual review required for significant regressions

## Profiling

### CPU Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Profile benchmarks
cargo flamegraph --bench database_benchmarks --features test-utils -- --bench
```

### Memory Profiling

```bash
# Install valgrind (Linux)
sudo apt-get install valgrind

# Profile memory usage
valgrind --tool=massif \
  cargo bench --package things3-core --bench database_benchmarks --features test-utils
```

### Async Profiling

```bash
# Install tokio-console
cargo install tokio-console

# Enable tokio-console in your app
# Then connect with:
tokio-console
```

## Future Optimizations

Planned performance improvements:

1. **Query Optimization**
   - [ ] Add database indexes for common queries
   - [ ] Optimize full-text search with FTS5
   - [ ] Implement query result streaming

2. **Caching Improvements**
   - [ ] Multi-level caching (L1: memory, L2: disk)
   - [ ] Predictive cache warming
   - [ ] Cache compression

3. **Parallel Processing**
   - [ ] Parallel bulk operations (where safe)
   - [ ] Concurrent query execution
   - [ ] Background cache refresh

4. **Database Optimizations**
   - [ ] WAL mode for better concurrency
   - [ ] Vacuum automation
   - [ ] Analyze statistics

## Benchmarking Best Practices

When adding new benchmarks:

1. **Use realistic data**: Test with production-like datasets
2. **Measure what matters**: Focus on user-facing operations
3. **Include warm-up**: Cache and connection pool warm-up
4. **Test at scale**: Include large dataset benchmarks
5. **Document results**: Update this file with findings

## Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [SQLx Performance Guide](https://github.com/launchbadge/sqlx/blob/main/FAQ.md#performance)
- [Tokio Performance Guide](https://tokio.rs/tokio/topics/performance)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)

## Questions?

For performance-related questions or issues:
- Open an issue on GitHub
- Check existing benchmark results
- Profile your specific use case
- Consider contributing benchmarks

