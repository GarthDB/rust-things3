# Feature Flags

`rust-things3` uses Cargo feature flags to allow users to opt into only the functionality they need, reducing compile times and binary sizes.

## Table of Contents

- [Quick Reference](#quick-reference)
- [Core Library Features](#core-library-features)
- [CLI Features](#cli-features)
- [Feature Combinations](#feature-combinations)
- [Binary Size Comparison](#binary-size-comparison)
- [Use Cases](#use-cases)

## Quick Reference

### things3-core

| Feature | Description | Default | Dependencies |
|---------|-------------|---------|--------------|
| `export-csv` | Enable CSV export | ❌ | `csv` |
| `export-opml` | Enable OPML export | ❌ | `quick-xml` |
| `observability` | Enable metrics/observability | ❌ | `metrics` |
| `full` | All optional features | ❌ | All above |
| `test-utils` | Test utilities | ❌ | - |

### things3-cli

| Feature | Description | Default | Dependencies |
|---------|-------------|---------|--------------|
| `mcp-server` | Enable MCP server command | ✅ | - |
| `export-csv` | Enable CSV export | ✅ | `things3-core/export-csv`, `csv` |
| `export-opml` | Enable OPML export | ✅ | `things3-core/export-opml`, `quick-xml` |
| `observability` | Enable metrics/dashboards | ✅ | `things3-core/observability`, `metrics`, etc. |
| `full` | All optional features | ❌ | All above |

## Core Library Features

### `export-csv`

Enables CSV export functionality.

**Cargo.toml:**
```toml
[dependencies]
things3-core = { version = "0.2", features = ["export-csv"] }
```

**Usage:**
```rust
use things3_core::{DataExporter, ExportFormat};

let exporter = DataExporter::new_default();
let csv_data = exporter.export(&data, ExportFormat::Csv)?;
```

**What's included:**
- CSV export via `DataExporter`
- `csv` crate dependency

### `export-opml`

Enables OPML export functionality.

**Cargo.toml:**
```toml
[dependencies]
things3-core = { version = "0.2", features = ["export-opml"] }
```

**Usage:**
```rust
use things3_core::{DataExporter, ExportFormat};

let exporter = DataExporter::new_default();
let opml_data = exporter.export(&data, ExportFormat::Opml)?;
```

**What's included:**
- OPML export via `DataExporter`
- `quick-xml` crate dependency

### `observability`

Enables advanced metrics and observability features.

**Cargo.toml:**
```toml
[dependencies]
things3-core = { version = "0.2", features = ["observability"] }
```

**Usage:**
```rust
use things3_core::{ObservabilityManager, ObservabilityConfig};

let config = ObservabilityConfig::default();
let mut obs = ObservabilityManager::new(config)?;
obs.initialize()?;

// Metrics are now available
let metrics = obs.get_metrics().await?;
```

**What's included:**
- `ObservabilityManager` - metrics and health checks
- `ThingsMetrics` - performance metrics
- Metrics exporters (Prometheus, TCP)
- Health check endpoints

**Always included** (not behind feature flag):
- `tracing` - structured logging for debugging
- `tracing-subscriber` - log formatting

### `full`

Enables all optional features.

**Cargo.toml:**
```toml
[dependencies]
things3-core = { version = "0.2", features = ["full"] }
```

Equivalent to:
```toml
[dependencies]
things3-core = { version = "0.2", features = ["export-csv", "export-opml", "observability"] }
```

### `test-utils`

Provides test utilities for integration testing.

**Cargo.toml:**
```toml
[dev-dependencies]
things3-core = { version = "0.2", features = ["test-utils"] }
```

**Usage:**
```rust
#[cfg(test)]
mod tests {
    use things3_core::test_utils::create_test_database;

    #[tokio::test]
    async fn test_something() {
        let db = create_test_database().await;
        // ... your test
    }
}
```

## CLI Features

### `mcp-server`

Enables the MCP (Model Context Protocol) server command.

**Build without MCP server:**
```bash
cargo build --package things3-cli --no-default-features \
    --features "export-csv,export-opml,observability"
```

**What's included:**
- `things3 mcp` command
- MCP server implementation
- JSON-RPC protocol handling

**When to disable:**
- If you only need the CLI commands (inbox, today, search, etc.)
- To reduce binary size
- For environments where MCP is not needed

### `export-csv` / `export-opml`

Enables export functionality in the CLI.

**Build without exports:**
```bash
cargo build --package things3-cli --no-default-features \
    --features "mcp-server,observability"
```

**What's included:**
- Export commands in CLI
- Export format support
- Data serialization

### `observability`

Enables health check server, monitoring dashboard, and metrics.

**Build without observability:**
```bash
cargo build --package things3-cli --no-default-features \
    --features "mcp-server,export-csv,export-opml"
```

**What's included:**
- `things3 health-server` command
- `things3 dashboard` command
- Metrics collection
- Performance monitoring
- Health check endpoints

**When to disable:**
- For minimal CLI-only deployments
- When metrics/monitoring not needed
- To reduce dependencies

## Feature Compatibility Matrix

This matrix shows which features work together and their dependencies:

| Feature Combination | things3-core | things3-cli | Notes |
|---------------------|--------------|-------------|-------|
| `export-csv` | ✅ Works standalone | ✅ Works standalone | Independent feature |
| `export-opml` | ✅ Works standalone | ✅ Works standalone | Independent feature |
| `observability` | ✅ Works standalone | ✅ Works standalone | Independent feature |
| `mcp-server` | N/A | ⚠️ Requires `export-csv` + `export-opml` | See explanation below |
| `export-csv` + `export-opml` | ✅ Compatible | ✅ Compatible | Common combination |
| `export-csv` + `observability` | ✅ Compatible | ✅ Compatible | All features independent |
| `export-opml` + `observability` | ✅ Compatible | ✅ Compatible | All features independent |
| All features (`full`) | ✅ Compatible | ✅ Compatible | Default for CLI |

### Why `mcp-server` Requires Export Features

The MCP (Model Context Protocol) server requires both `export-csv` and `export-opml` features because:

1. **MCP Tool Implementation**: Several MCP tools expose export functionality to AI clients:
   - `export_to_csv` - Exports tasks/projects to CSV format
   - `export_to_opml` - Exports tasks/projects to OPML format
   - `bulk_export` - Exports all data in specified format

2. **AI Client Expectations**: AI/LLM clients using the MCP protocol expect to be able to:
   - Extract data for analysis (CSV format)
   - Import into other tools (OPML format)
   - Generate reports (both formats)

3. **API Consistency**: The MCP server exposes the same export capabilities as the CLI, maintaining a consistent API surface.

**Building MCP Server with All Features:**
```bash
# Default - includes MCP with all exports
cargo build --package things3-cli

# Explicit - same as above
cargo build --package things3-cli --features "mcp-server,export-csv,export-opml"
```

**Why You Can't Build MCP Without Exports:**
```bash
# ❌ This will fail - mcp-server depends on exports
cargo build --package things3-cli --no-default-features --features "mcp-server"

# The feature definition in Cargo.toml:
# mcp-server = ["export-csv", "export-opml"]
```

**Alternative: Use CLI Without MCP:**
If you don't need the MCP server, you can build a minimal CLI:
```bash
# Minimal CLI - no MCP, no observability
cargo build --package things3-cli --no-default-features

# CLI with exports but no MCP
cargo build --package things3-cli --no-default-features \
    --features "export-csv,export-opml"
```

## Feature Combinations

### Minimal Core Library

Smallest footprint, core functionality only:

```toml
[dependencies]
things3-core = { version = "0.2", default-features = false }
```

**Includes:**
- Database access
- Task/Project/Area models
- Caching
- Tracing/logging

**Excludes:**
- CSV/OPML exports
- Metrics/observability

### Core with Exports Only

```toml
[dependencies]
things3-core = { version = "0.2", default-features = false, features = ["export-csv", "export-opml"] }
```

### Minimal CLI

CLI commands only, no MCP server or observability:

```bash
cargo build --package things3-cli --no-default-features
```

**Available commands:**
- `inbox`, `today`, `projects`, `areas`, `search`
- `health` (basic health check)

**Not available:**
- `mcp` (MCP server)
- `health-server` (health check HTTP server)
- `dashboard` (monitoring dashboard)

### CLI with MCP Only

```bash
cargo build --package things3-cli --no-default-features --features "mcp-server"
```

### Full-Featured Build

```bash
cargo build --package things3-cli --features "full"
# or just:
cargo build --package things3-cli  # (same as default)
```

## Binary Size Comparison

Approximate binary sizes (release builds, x86_64):

| Configuration | Size (MB) | Reduction |
|---------------|-----------|-----------|
| Full (default) | ~8.5 | 0% |
| No observability | ~7.2 | 15% |
| No exports | ~8.3 | 2% |
| No MCP server | ~7.8 | 8% |
| Minimal CLI | ~6.5 | 24% |
| Core only | ~5.2 | 39% |

*Note: Sizes are approximate and may vary by platform.*

## Use Cases

### Case 1: Library Integration

You're building an application that uses `things3-core` as a library:

```toml
[dependencies]
# Minimal - just database access
things3-core = { version = "0.2", default-features = false }

# With exports for data migration
things3-core = { version = "0.2", default-features = false, features = ["export-csv"] }
```

### Case 2: CLI-Only Deployment

You only need the CLI commands, no MCP server:

```bash
cargo install things3-cli --no-default-features \
    --features "export-csv,export-opml"
```

### Case 3: MCP Server Only

You only need the MCP server for AI/LLM integration:

```bash
cargo install things3-cli --no-default-features --features "mcp-server"
```

### Case 4: Production Monitoring

You need full observability for production:

```bash
cargo install things3-cli --features "full"
# or just:
cargo install things3-cli  # (default includes everything)
```

### Case 5: Embedded/Resource-Constrained

Minimal binary size for embedded systems:

```bash
cargo build --release --package things3-cli --no-default-features
```

## Compile Time Comparison

Approximate clean build times (M1 MacBook Pro):

| Configuration | Time (seconds) | Reduction |
|---------------|----------------|-----------|
| Full (default) | ~45 | 0% |
| No observability | ~38 | 16% |
| No exports | ~43 | 4% |
| Minimal | ~32 | 29% |

## Feature Flag Best Practices

### For Library Users

1. **Start minimal**: Only enable features you need
   ```toml
   things3-core = { version = "0.2", default-features = false }
   ```

2. **Add features as needed**:
   ```toml
   things3-core = { version = "0.2", default-features = false, features = ["export-csv"] }
   ```

3. **Use `full` for development**:
   ```toml
   [dev-dependencies]
   things3-core = { version = "0.2", features = ["full", "test-utils"] }
   ```

### For CLI Users

1. **Default is fine for most**: The default features provide a full-featured CLI

2. **Customize for deployment**:
   ```bash
   # Production MCP server
   cargo build --release --no-default-features --features "mcp-server"
   
   # CI/automation CLI
   cargo build --release --no-default-features --features "export-csv"
   ```

3. **Check feature availability**:
   ```bash
   cargo tree --package things3-cli --features "mcp-server"
   ```

## Troubleshooting

### Error: "CSV export is not enabled"

You're trying to use CSV export without the feature flag:

```bash
cargo build --features "export-csv"
```

### Error: "OPML export is not enabled"

You're trying to use OPML export without the feature flag:

```bash
cargo build --features "export-opml"
```

### Error: "Command 'mcp' not found"

The MCP server feature is not enabled:

```bash
cargo build --features "mcp-server"
# or use default features:
cargo build
```

### Error: "ObservabilityManager not found"

The observability feature is not enabled:

```bash
cargo build --features "observability"
```

## Related Documentation

- [Architecture](ARCHITECTURE.md) - System design
- [Development Guide](DEVELOPMENT.md) - Development setup
- [Performance Guide](PERFORMANCE.md) - Optimization strategies

---

**Last Updated**: January 2026  
**For**: rust-things3 v0.2.0+

