# 🦀 Rust Things

A high-performance Rust library and CLI for Things 3 integration with integrated MCP (Model Context Protocol) server support for AI/LLM environments.

**📦 Version 1.0.0 - Production Ready!**

[![CI/CD Pipeline](https://github.com/GarthDB/rust-things3/actions/workflows/ci.yml/badge.svg)](https://github.com/GarthDB/rust-things3/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/GarthDB/rust-things3/branch/main/graph/badge.svg)](https://codecov.io/gh/GarthDB/rust-things3)
[![Crates.io](https://img.shields.io/crates/v/things3-cli.svg)](https://crates.io/crates/things3-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)](RELEASE_NOTES.md)

## ✨ Features

- 🚀 **High Performance**: Built with Rust for maximum speed and reliability
- 🔧 **CLI Tool**: Command-line interface for managing Things 3 data
- 🤖 **MCP Integration**: Integrated MCP server for AI/LLM integration
- 📊 **Comprehensive API**: Full access to Things 3 database with async SQLx
- 🏗️ **Moon Workspace**: Organized monorepo with Moon build system
- 🧪 **Well Tested**: Comprehensive test suite and benchmarks
- 📈 **Performance Monitoring**: Built-in metrics and system monitoring
- 💾 **Caching Layer**: High-performance caching with Moka
- 🔄 **Backup & Restore**: Complete backup system with metadata
- 📤 **Data Export**: Multiple formats (JSON, CSV, OPML, Markdown)
- 🔧 **Advanced MCP Tools**: 17 tools for AI/LLM integration
- ⚡ **Async Database**: SQLx-powered async database operations with thread safety
- 🌐 **Web Servers**: Health check and monitoring dashboard servers

## 🚀 Installation

### Homebrew (macOS)

```bash
# Add the tap (when available)
brew tap GarthDB/rust-things3

# Install
brew install things3-cli
```

### Cargo (Rust)

```bash
# Install from crates.io (when published)
cargo install things3-cli

# Or install from source
cargo install --git https://github.com/GarthDB/rust-things3
```

### From Source

```bash
git clone https://github.com/GarthDB/rust-things3
cd rust-things3
cargo build --release

# Add to PATH
export PATH="$PWD/target/release:$PATH"
```

### Using Moon (Development)

```bash
# Install Moon if you haven't already
curl -fsSL https://moonrepo.dev/install | bash

# Clone and setup
git clone https://github.com/GarthDB/rust-things3
cd rust-things3
moon run :dev-pipeline
```

## ⚙️ Feature Flags

**New in 1.0.0**: Modular compilation with feature flags! Choose only what you need.

### Library (`things3-core`)

```toml
[dependencies]
# Minimal (core functionality only - 24% smaller binary)
things3-core = { version = "1.0", default-features = false }

# With specific features
things3-core = { version = "1.0", features = ["export-csv", "observability"] }

# Full features (recommended for most users)
things3-core = { version = "1.0", features = ["full"] }
```

**Available Features:**
- `export-csv`: CSV export support
- `export-opml`: OPML export support  
- `observability`: Metrics, tracing, and health checks
- `full`: Enable all features
- `test-utils`: Testing utilities (development only)

### CLI (`things3-cli`)

```toml
[dependencies]
# CLI with all features
things3-cli = { version = "1.0", features = ["full"] }

# CLI with specific features
things3-cli = { version = "1.0", features = ["mcp-server", "export-csv"] }
```

**Additional CLI Features:**
- `mcp-server`: MCP server functionality (requires export features)

📚 **See [FEATURES.md](docs/FEATURES.md) for detailed feature documentation and compatibility matrix.**

## 📖 Quick Start

Get started in under 5 minutes! See the [Quick Start Guide](docs/QUICKSTART.md) for detailed instructions.

### Basic Library Usage

```rust
use things3_core::{ThingsDatabase, ThingsError};

#[tokio::main]
async fn main() -> Result<(), ThingsError> {
    // Connect to database
    let db_path = things3_core::get_default_database_path();
    let db = ThingsDatabase::new(&db_path).await?;
    
    // Get inbox tasks
    let tasks = db.get_inbox(Some(10)).await?;
    for task in tasks {
        println!("- {}", task.title);
    }
    
    // Search for tasks
    let results = db.search_tasks("meeting").await?;
    println!("Found {} matching tasks", results.len());
    
    Ok(())
}
```

### CLI Commands

```bash
# Show help
things3 --help

# Health check
things3 health

# Show inbox tasks
things3 inbox
things3 inbox --limit 5

# Show today's tasks
things3 today
things3 today --limit 3

# Show all projects
things3 projects
things3 projects --area <AREA_UUID>

# Show all areas
things3 areas

# Search for tasks
things3 search "meeting"
things3 search "report" --limit 10

# Start MCP server (for AI/LLM integration)
things3 mcp

# Start health check server
things3 health-server --port 8080

# Start monitoring dashboard
things3 dashboard --port 8081
```

### Environment Variables

```bash
# Set custom database path
export THINGS_DB_PATH="/path/to/things.db"

# Enable fallback to default path
export THINGS_FALLBACK_TO_DEFAULT=true

# Enable verbose logging
export RUST_LOG=debug
```

## 🌐 Web Servers

The CLI includes built-in web servers for monitoring and health checks:

### Health Check Server

```bash
# Start health check server
things3 health-server --port 8080

# Test health endpoint
curl http://localhost:8080/health
curl http://localhost:8080/ping
```

### Monitoring Dashboard

```bash
# Start monitoring dashboard
things3 dashboard --port 8081

# Access dashboard
open http://localhost:8081
```

The dashboard provides:
- Real-time metrics and statistics
- Database health monitoring
- Performance metrics
- System resource usage
- Task and project analytics

## 🤖 MCP Integration

The MCP (Model Context Protocol) server provides 21 tools for AI/LLM integration:

### Available MCP Tools

| Tool | Description |
|------|-------------|
| `get_inbox` | Get tasks from the inbox |
| `get_today` | Get tasks scheduled for today |
| `get_projects` | Get all projects, optionally filtered by area |
| `get_areas` | Get all areas |
| `search_tasks` | Search for tasks by title or notes |
| `create_task` | Create a new task |
| `update_task` | Update an existing task |
| `get_productivity_metrics` | Get productivity metrics |
| `export_data` | Export data in various formats |
| `bulk_create_tasks` | Create multiple tasks at once |
| `get_recent_tasks` | Get recently modified tasks |
| `backup_database` | Create a database backup |
| `restore_database` | Restore from a backup |
| `list_backups` | List available backups |
| `get_performance_stats` | Get performance statistics |
| `get_system_metrics` | Get system resource metrics |
| `get_cache_stats` | Get cache performance stats |

### Configuration

#### Cursor
```json
// .cursor/mcp.json
{
  "mcpServers": {
    "things3": {
      "command": "things3",
      "args": ["mcp"],
      "env": {
        "THINGS_DB_PATH": "/path/to/things.db"
      }
    }
  }
}
```

#### VS Code
```json
// .vscode/mcp.json
{
  "servers": {
    "things3": {
      "type": "stdio",
      "command": "things3",
      "args": ["mcp"],
      "cwd": "${workspaceFolder}",
      "env": {
        "THINGS_DB_PATH": "/path/to/things.db"
      }
    }
  }
}
```

#### Zed
```json
// .zed/settings.json
{
  "mcp": {
    "things3": {
      "command": "things3",
      "args": ["mcp"],
      "env": {
        "THINGS_DB_PATH": "/path/to/things.db"
      }
    }
  }
}
```

## Documentation

### Getting Started

- **[Quick Start Guide](docs/QUICKSTART.md)** - Get started in under 5 minutes
- **[User Guide](docs/USER_GUIDE.md)** - Comprehensive usage guide
- **[Error Handling Guide](docs/ERROR_HANDLING.md)** - Error handling patterns and recovery strategies

### Release Documentation (1.0.0)

- **[Release Notes](RELEASE_NOTES.md)** - What's new in 1.0.0
- **[Migration Guide](docs/MIGRATION.md)** - Upgrade from 0.x to 1.0.0
- **[Feature Flags Guide](docs/FEATURES.md)** - Modular compilation with feature flags
- **[Security Audit](docs/SECURITY_AUDIT.md)** - Security audit results
- **[Post-1.0 Roadmap](docs/POST_1.0_ROADMAP.md)** - Future development plans
- **[Changelog](CHANGELOG.md)** - Complete version history

### Core Documentation

- **[Architecture](docs/ARCHITECTURE.md)** - System design and component overview
- **[MCP Integration](docs/MCP_INTEGRATION.md)** - Complete MCP server guide
- **[Reliability Guide](docs/RELIABILITY.md)** - Connection pooling, error recovery, and resilience patterns
- **[Performance Guide](docs/PERFORMANCE.md)** - Benchmarks and optimization strategies
- **[Database Schema](docs/DATABASE_SCHEMA.md)** - Things 3 database structure
- **[Development Guide](docs/DEVELOPMENT.md)** - Setup and development workflow
- **[Coverage Analysis](docs/COVERAGE_ANALYSIS.md)** - Test coverage report

### Examples

#### Basic Examples

See the [`libs/things3-core/examples/`](libs/things3-core/examples/) directory for practical usage examples:
- `basic_usage.rs` - Basic database operations (connect, query, create, update)
- `bulk_operations.rs` - Bulk operation examples (move, complete, delete)
- `search_tasks.rs` - Advanced search functionality
- `export_data.rs` - Data export in multiple formats (JSON, CSV, Markdown)

```bash
cargo run --package things3-core --example basic_usage
cargo run --package things3-core --example bulk_operations
cargo run --package things3-core --example search_tasks
cargo run --package things3-core --example export_data
```

#### Integration Examples (New in 1.0.0)

Real-world integration patterns in [`examples/integration/`](examples/integration/):

- **`mcp_client.rs`** - Custom MCP client implementation
- **`cli_extension.rs`** - Extending the CLI with custom commands
- **`web_api.rs`** - REST API with Axum web framework
- **`background_service.rs`** - Long-running service with graceful shutdown
- **`custom_middleware.rs`** - Custom middleware for cross-cutting concerns

```bash
cd examples/integration
cargo run --example mcp_client
cargo run --example cli_extension -- today
cargo run --example web_api
cargo run --example background_service
cargo run --example custom_middleware
```

See [`examples/integration/README.md`](examples/integration/README.md) for detailed documentation.

### API Documentation

Generate and view API documentation:
```bash
cargo doc --workspace --no-deps --open
```

## Testing

### Test Coverage

- **Total Tests**: 438 tests
- **Coverage**: ~85%+ (target: 85%+)
- **Test Categories**:
  - Database operations (Phase 1)
  - MCP I/O layer (Phase 2)
  - Middleware chain (Phase 3)
  - Observability system (Phase 4)

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific package
cargo test --package things3-core

# With coverage
cargo llvm-cov --workspace --all-features --html
open target/llvm-cov/html/index.html
```

See [Development Guide](docs/DEVELOPMENT.md) for more testing details.

## Development

### Prerequisites

- Rust 1.70+
- Moon (for workspace management)
- Things 3 (for testing)
- cargo-llvm-cov (for coverage)

### Setup

```bash
# Clone the repository
git clone https://github.com/GarthDB/rust-things3
cd rust-things3

# Install dependencies
moon run :local-dev-setup

# Run tests
moon run :test-all

# Run development pipeline
moon run :dev-pipeline
```

### Quick Commands

```bash
# Format code
cargo fmt --all

# Lint code
cargo clippy --workspace -- -D warnings

# Run coverage
cargo llvm-cov --workspace --all-features --html

# Generate docs
cargo doc --workspace --no-deps
```

See [Development Guide](docs/DEVELOPMENT.md) for comprehensive development information.

### Project Structure

```
rust-things3/
├── apps/
│   └── things3-cli/       # CLI application with MCP server
├── libs/
│   ├── things3-core/      # Core library
│   └── things3-common/    # Shared utilities
├── tools/
│   └── xtask/             # Development tools
└── tests/                 # Integration tests
```

## API Reference

### Core Library

#### Basic Usage

```rust
use things3_core::{ThingsDatabase, Task, Project, Area, ThingsConfig};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Create database connection with SQLx
    let db = ThingsDatabase::new("/path/to/things.db").await?;
    
    // Get inbox tasks
    let tasks = db.get_inbox(Some(10)).await?;
    
    // Get today's tasks
    let today_tasks = db.get_today(None).await?;
    
    // Get all projects
    let projects = db.get_projects(None).await?;
    
    // Search tasks
    let search_results = db.search_tasks("meeting").await?;
    
    Ok(())
}
```

#### Advanced Configuration

```rust
use things3_core::{ThingsDatabase, ThingsConfig};
use std::path::Path;

// Custom database path with SQLx
let db = ThingsDatabase::new(Path::new("/custom/path/to/things.db")).await?;

// From environment variables
let config = ThingsConfig::from_env();
let db = ThingsDatabase::new(&config.database_path).await?;
```

#### Error Handling

```rust
use things3_core::{ThingsDatabase, ThingsError};
use anyhow::Result;

async fn handle_errors() -> Result<()> {
    let db = ThingsDatabase::new("/path/to/things.db").await?;
    
    match db.get_inbox(Some(5)).await {
        Ok(tasks) => println!("Found {} tasks", tasks.len()),
        Err(ThingsError::Database(msg)) => {
            eprintln!("Database error: {}", msg);
        }
        Err(e) => {
            eprintln!("Other error: {}", e);
        }
    }
    
    Ok(())
}
```

#### Caching and Performance

```rust
use things3_core::{ThingsDatabase, CacheConfig};
use std::time::Duration;

// Configure caching
let cache_config = CacheConfig {
    max_capacity: 1000,
    time_to_live: Duration::from_secs(300),
    time_to_idle: Duration::from_secs(60),
};
let db = ThingsDatabase::with_cache_config(cache_config)?;

// Get cache statistics
let stats = db.get_cache_stats().await?;
println!("Cache hits: {}, misses: {}", stats.hits, stats.misses);
```

#### Data Export

```rust
use things3_core::{DataExporter, ExportFormat, ExportConfig};

// Export to JSON
let exporter = DataExporter::new_default();
let json_data = exporter.export_json(&tasks, &projects, &areas).await?;

// Export to CSV
let csv_data = exporter.export_csv(&tasks, &projects, &areas).await?;

// Custom export configuration
let config = ExportConfig {
    include_completed: false,
    date_format: "%Y-%m-%d".to_string(),
    time_format: "%H:%M:%S".to_string(),
};
let exporter = DataExporter::new(config);
```

#### MCP Server Integration

```rust
use things3_cli::mcp::{ThingsMcpServer, CallToolRequest};
use serde_json::json;

// Create MCP server
let server = ThingsMcpServer::new(config)?;

// List available tools
let tools = server.list_tools().await?;
println!("Available tools: {:?}", tools.tools);

// Call a tool
let request = CallToolRequest {
    name: "get_inbox".to_string(),
    arguments: Some(json!({
        "limit": 10
    })),
};
let result = server.call_tool(request).await?;
```

### CLI Library

```rust
use things3_cli::{Cli, Commands, print_tasks, print_projects};
use std::io::stdout;

// Parse CLI arguments
let cli = Cli::parse();

// Use CLI functions programmatically
match cli.command {
    Commands::Inbox { limit } => {
        let tasks = db.get_inbox(limit).await?;
        print_tasks(&mut stdout(), &tasks)?;
    }
    Commands::Projects { area_uuid, limit } => {
        let projects = db.get_projects(area_uuid, limit).await?;
        print_projects(&mut stdout(), &projects)?;
    }
    // ... other commands
}
```

### Common Utilities

```rust
use things3_common::utils::{
    get_default_database_path,
    format_date,
    format_datetime,
    parse_date,
    is_valid_uuid,
    truncate_string
};

// Get default database path
let db_path = get_default_database_path();
println!("Default path: {}", db_path.display());

// Format dates
let formatted = format_date(chrono::Utc::now().date_naive());
println!("Today: {}", formatted);

// Parse dates
let date = parse_date("2024-01-15")?;
println!("Parsed date: {}", date);

// Validate UUIDs
let is_valid = is_valid_uuid("550e8400-e29b-41d4-a716-446655440000");
println!("Valid UUID: {}", is_valid);

// Truncate strings
let truncated = truncate_string("Very long string", 10);
println!("Truncated: {}", truncated);
```

## Architecture

The project is organized as a Moon-managed Rust workspace:

```
rust-things3/
├── apps/things3-cli/      # CLI application with MCP server
├── libs/things3-core/     # Core database and business logic
├── libs/things3-common/   # Shared utilities
├── examples/              # Usage examples
├── docs/                  # Documentation
└── tests/                 # Integration tests
```

Key features:
- **Async-first**: Built on Tokio for concurrent operations
- **Type-safe**: SQLx for compile-time SQL verification
- **MCP Protocol**: Industry-standard AI agent communication
- **Middleware**: Extensible request/response processing
- **Observability**: Built-in metrics, logging, and tracing

See [Architecture Documentation](docs/ARCHITECTURE.md) for detailed system design.

## Troubleshooting

### Database Not Found

```bash
# Find your Things 3 database
find ~/Library/Group\ Containers -name "main.sqlite" 2>/dev/null

# Set custom path
export THINGS_DB_PATH="/path/to/main.sqlite"
```

### Permission Issues

Ensure Things 3 is closed when running the CLI:
```bash
killall Things3
```

### Test Failures

Run tests single-threaded if experiencing database lock issues:
```bash
cargo test -- --test-threads=1
```

See [Development Guide](docs/DEVELOPMENT.md) for more troubleshooting tips.

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for detailed information on:

- Development setup
- Code style guidelines
- Testing requirements
- Pull request process
- Issue reporting

### Quick Start

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests (maintain 85%+ coverage)
5. Run the development pipeline: `moon run :dev-pipeline`
6. Submit a pull request

For more details, see [CONTRIBUTING.md](CONTRIBUTING.md) and [Development Guide](docs/DEVELOPMENT.md).

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by [things3](https://github.com/thingsapi/things3)
- Built with [Moon](https://moonrepo.dev) workspace management
- Follows [evelion-apps/things-api](https://github.com/evelion-apps/things-api) patterns
