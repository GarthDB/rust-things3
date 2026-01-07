# Migration Guide: 0.x → 1.0.0

This guide helps you migrate your application from `rust-things3` version 0.x to 1.0.0.

## Overview

Version 1.0.0 is the first stable release with guaranteed API stability. While we've maintained backward compatibility where possible, there are some intentional changes to improve the library's modularity and flexibility.

**Estimated Migration Time**: 10-30 minutes for most projects

---

## Quick Migration Checklist

- [ ] Update `Cargo.toml` dependencies
- [ ] Add required feature flags
- [ ] Run `cargo check` and fix any compilation errors
- [ ] Update configuration files (if needed)
- [ ] Run tests
- [ ] Run `cargo audit` for security
- [ ] Review new best practices

---

## Step 1: Update Dependencies

### Cargo.toml Changes

#### Option A: Full Compatibility (Recommended for Quick Migration)

**Before (0.x)**:
```toml
[dependencies]
things3-core = "0.2"
```

**After (1.0.0)**:
```toml
[dependencies]
things3-core = { version = "1.0", features = ["full"] }
```

This enables all features, matching the old default behavior.

#### Option B: Minimal Build (Recommended for Production)

Choose only the features you need:

```toml
[dependencies]
things3-core = { version = "1.0", default-features = false, features = [
    "export-csv",      # If you export to CSV
    "export-opml",     # If you export to OPML
    "observability",   # If you use metrics/health checks
] }
```

**Benefits**: 24% smaller binary, faster compilation

#### Option C: CLI Application

**Before (0.x)**:
```toml
[dependencies]
things3-cli = "0.2"
```

**After (1.0.0)**:
```toml
[dependencies]
things3-cli = { version = "1.0", features = ["full"] }
```

---

## Step 2: Feature Flag Selection

### Understanding Feature Flags

| Feature | Includes | Use When | Binary Size Impact |
|---------|----------|----------|-------------------|
| *(none)* | Core database access, JSON export | You only need basic functionality | **Minimal** (24% smaller) |
| `export-csv` | CSV export via `DataExporter` | You export tasks to CSV | +~500 KB |
| `export-opml` | OPML export via `DataExporter` | You export tasks to OPML | +~300 KB |
| `observability` | Metrics, health checks, tracing | You monitor application health | +~1 MB |
| `mcp-server` (CLI only) | MCP server functionality | You use the MCP server | +~2 MB |
| `full` | All of the above | You want everything | **Full** (0.2.x equivalent) |

### Decision Tree

**Do you export data?**
- Yes, CSV → Include `export-csv`
- Yes, OPML → Include `export-opml`
- No → Omit both

**Do you need monitoring/metrics?**
- Yes → Include `observability`
- No → Omit

**Do you run the MCP server?** (CLI only)
- Yes → Include `mcp-server`
- No → Omit

**Not sure? Use `full`** for now, optimize later.

---

## Step 3: Code Changes

### Breaking Change #1: Default Features

**What Changed**: Default features changed from "all enabled" to "minimal"

**Impact**: Code using optional features may fail to compile without explicit feature flags

#### Example: CSV Export

**Before (0.x)** - Worked automatically:
```rust
use things3_core::{DataExporter, ExportFormat};

let exporter = DataExporter::new(config);
exporter.export(&data, ExportFormat::Csv)?;
```

**After (1.0.0)** - Requires feature flag:

**Cargo.toml**:
```toml
things3-core = { version = "1.0", features = ["export-csv"] }
```

**Code** (unchanged):
```rust
use things3_core::{DataExporter, ExportFormat};

let exporter = DataExporter::new(config);
exporter.export(&data, ExportFormat::Csv)?;
```

If you forget the feature flag, you'll get:
```
error[E0432]: unresolved import `things3_core::DataExporter`
```

**Fix**: Add the appropriate feature flag to `Cargo.toml`

#### Example: Observability

**Before (0.x)** - Worked automatically:
```rust
use things3_core::{ObservabilityManager, ObservabilityConfig};

let obs = ObservabilityManager::new(config)?;
```

**After (1.0.0)** - Requires feature flag:

**Cargo.toml**:
```toml
things3-core = { version = "1.0", features = ["observability"] }
```

**Code** (unchanged):
```rust
use things3_core::{ObservabilityManager, ObservabilityConfig};

let obs = ObservabilityManager::new(config)?;
```

### Breaking Change #2: Configuration Structure

**What Changed**: Minor additions to configuration options (all optional, backward compatible)

**Impact**: Minimal - existing configs work as-is

#### Example: ThingsConfig

**Before (0.x)**:
```rust
let config = ThingsConfig {
    database_path: Some(path),
    fallback_to_default: true,
    ..Default::default()
};
```

**After (1.0.0)** - Same, with optional new fields:
```rust
let config = ThingsConfig {
    database_path: Some(path),
    fallback_to_default: true,
    // New optional fields (can be omitted):
    // pool_config: Some(DatabasePoolConfig::default()),
    // cache: Some(CacheConfig::default()),
    ..Default::default()
};
```

**No changes required** - your existing code works!

### No Breaking Changes

The following APIs are **unchanged** and work exactly as before:

✅ `ThingsDatabase::new()` and `new_with_config()`  
✅ All database query methods (`get_inbox()`, `search_tasks()`, etc.)  
✅ All data models (`Task`, `Project`, `Area`, `Tag`)  
✅ Error types (`ThingsError`, `Result`)  
✅ Core configuration (`ThingsConfig`)  

---

## Step 4: Update Configuration Files

### YAML/JSON Configuration

If you use YAML or JSON configuration files, no changes are required. New optional fields can be added:

**config.yaml** (optional additions):
```yaml
# Existing config (still works)
database:
  path: "/path/to/Things3.db"
  fallback_to_default: true

# New optional sections (1.0.0):
cache:
  enabled: true
  ttl: 300
  max_size: 1000

pool:
  max_connections: 10
  min_connections: 2
  acquire_timeout: 30

observability:
  metrics_enabled: true
  log_level: "info"
  health_check_interval: 60
```

All new sections are optional and have sensible defaults.

---

## Step 5: Test Your Application

### Run Tests

```bash
# Build and run tests
cargo test --all-features

# Build with your specific features
cargo test --features "export-csv,observability"
```

### Check for Warnings

```bash
# Look for deprecation warnings
cargo build 2>&1 | grep -i "warning.*deprecated"

# Run clippy
cargo clippy -- -D warnings
```

### Run Security Audit

```bash
cargo audit
```

Expected: 1 low-risk warning (unused code path) - see [SECURITY_AUDIT.md](SECURITY_AUDIT.md)

---

## Step 6: Review New Best Practices

### 1. Feature Flag Best Practices

**❌ Don't**:
```toml
# Avoid: Enabling all features unnecessarily
things3-core = { version = "1.0", features = ["full"] }
```

**✅ Do**:
```toml
# Better: Only enable what you use
things3-core = { version = "1.0", features = ["export-csv"] }
```

### 2. Error Handling

Continue using the ergonomic `Result` type:

```rust
use things3_core::Result;

async fn my_function() -> Result<Vec<Task>> {
    let db = ThingsDatabase::new_with_default_path().await?;
    db.get_inbox(None).await
}
```

### 3. Caching

Enable caching for better performance:

```rust
use things3_core::{ThingsConfig, CacheConfig};
use std::time::Duration;

let config = ThingsConfig {
    cache: Some(CacheConfig {
        enabled: true,
        ttl: Duration::from_secs(300),
        max_size: 1000,
        ..Default::default()
    }),
    ..Default::default()
};
```

### 4. Connection Pooling

Configure the connection pool for your workload:

```rust
use things3_core::{ThingsConfig, DatabasePoolConfig};

let config = ThingsConfig {
    pool_config: Some(DatabasePoolConfig {
        max_connections: 10,
        min_connections: 2,
        acquire_timeout: Duration::from_secs(30),
        ..Default::default()
    }),
    ..Default::default()
};
```

---

## Troubleshooting

### Issue: "unresolved import" for Export Types

**Error**:
```
error[E0432]: unresolved import `things3_core::DataExporter`
```

**Solution**: Add the appropriate feature flag:
```toml
things3-core = { version = "1.0", features = ["export-csv", "export-opml"] }
```

### Issue: "unresolved import" for Observability Types

**Error**:
```
error[E0432]: unresolved import `things3_core::ObservabilityManager`
```

**Solution**: Add the observability feature:
```toml
things3-core = { version = "1.0", features = ["observability"] }
```

### Issue: Tests Failing After Upgrade

**Symptoms**: Tests pass in 0.x, fail in 1.0.0

**Common Causes**:
1. Missing feature flags in test dependencies
2. Changed timing in async tests

**Solution**:
```toml
[dev-dependencies]
things3-core = { version = "1.0", features = ["full", "test-utils"] }
```

### Issue: Binary Size Increased

**Symptoms**: Release binary larger than before

**Solution**: You're probably using `features = ["full"]`. Reduce to only what you need:

```toml
# Instead of:
things3-core = { version = "1.0", features = ["full"] }

# Use:
things3-core = { version = "1.0", features = ["export-csv"] }
```

Result: ~24% size reduction

---

## Advanced: Conditional Compilation

If you want to conditionally enable features:

```rust
// Only compile this code when the feature is enabled
#[cfg(feature = "export-csv")]
fn export_to_csv(data: &ExportData) -> Result<String> {
    use things3_core::{DataExporter, ExportFormat, ExportConfig};
    
    let exporter = DataExporter::new(ExportConfig::default());
    exporter.export(data, ExportFormat::Csv)
}

// Provide fallback when feature is disabled
#[cfg(not(feature = "export-csv"))]
fn export_to_csv(_data: &ExportData) -> Result<String> {
    Err(anyhow::anyhow!("CSV export not enabled. Enable the 'export-csv' feature."))
}
```

---

## Migration Examples

### Example 1: Basic CLI Tool

**Before (0.x)**:
```toml
[dependencies]
things3-core = "0.2"
tokio = { version = "1", features = ["full"] }
```

```rust
use things3_core::{ThingsDatabase, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let db = ThingsDatabase::new_with_default_path().await?;
    let tasks = db.get_inbox(None).await?;
    
    for task in tasks {
        println!("{}", task.title);
    }
    
    Ok(())
}
```

**After (1.0.0)** - No code changes needed!
```toml
[dependencies]
things3-core = "1.0"  # Minimal features sufficient
tokio = { version = "1", features = ["full"] }
```

```rust
// Code unchanged - works as-is!
use things3_core::{ThingsDatabase, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let db = ThingsDatabase::new_with_default_path().await?;
    let tasks = db.get_inbox(None).await?;
    
    for task in tasks {
        println!("{}", task.title);
    }
    
    Ok(())
}
```

### Example 2: CSV Export Tool

**Before (0.x)**:
```toml
[dependencies]
things3-core = "0.2"
```

**After (1.0.0)**:
```toml
[dependencies]
things3-core = { version = "1.0", features = ["export-csv"] }
```

Code: **No changes required** ✅

### Example 3: Monitoring Application

**Before (0.x)**:
```toml
[dependencies]
things3-core = "0.2"
```

**After (1.0.0)**:
```toml
[dependencies]
things3-core = { version = "1.0", features = ["observability"] }
```

Code: **No changes required** ✅

---

## Need Help?

- **Documentation**: https://docs.rs/things3-core
- **Examples**: `examples/integration/` directory
- **Issues**: https://github.com/GarthDB/rust-things3/issues
- **Discussions**: https://github.com/GarthDB/rust-things3/discussions

---

## Summary

1. ✅ Update `Cargo.toml` with version `1.0` and appropriate features
2. ✅ Add feature flags for optional functionality you use
3. ✅ Run `cargo check` and fix any compilation errors
4. ✅ Test thoroughly
5. ✅ Review new examples for best practices

**Most projects can migrate in < 30 minutes!**

Welcome to 1.0.0! 🎉

