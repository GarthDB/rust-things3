# 🦀 Rust Things

A high-performance Rust library and CLI for Things 3 integration with integrated MCP (Model Context Protocol) server support for AI/LLM environments.

[![CI](https://github.com/GarthDB/rust-things/workflows/CI/badge.svg)](https://github.com/GarthDB/rust-things/actions)
[![codecov](https://codecov.io/gh/GarthDB/rust-things/branch/main/graph/badge.svg)](https://codecov.io/gh/GarthDB/rust-things)
[![Crates.io](https://img.shields.io/crates/v/things-cli.svg)](https://crates.io/crates/things-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

## ✨ Features

- 🚀 **High Performance**: Built with Rust for maximum speed and reliability
- 🔧 **CLI Tool**: Command-line interface for managing Things 3 data
- 🤖 **MCP Integration**: Integrated MCP server for AI/LLM integration
- 📊 **Comprehensive API**: Full access to Things 3 database with caching
- 🏗️ **Moon Workspace**: Organized monorepo with Moon build system
- 🧪 **Well Tested**: Comprehensive test suite and benchmarks
- 📈 **Performance Monitoring**: Built-in metrics and system monitoring
- 💾 **Caching Layer**: High-performance caching with Moka
- 🔄 **Backup & Restore**: Complete backup system with metadata
- 📤 **Data Export**: Multiple formats (JSON, CSV, OPML, Markdown)
- 🔧 **Advanced MCP Tools**: 17 tools for AI/LLM integration

## 🚀 Installation

### Homebrew (macOS)

```bash
# Add the tap (when available)
brew tap GarthDB/rust-things

# Install
brew install things-cli
```

### Cargo (Rust)

```bash
# Install from crates.io (when published)
cargo install things-cli

# Or install from source
cargo install --git https://github.com/GarthDB/rust-things
```

### From Source

```bash
git clone https://github.com/GarthDB/rust-things
cd rust-things
cargo build --release

# Add to PATH
export PATH="$PWD/target/release:$PATH"
```

### Using Moon (Development)

```bash
# Install Moon if you haven't already
curl -fsSL https://moonrepo.dev/install | bash

# Clone and setup
git clone https://github.com/GarthDB/rust-things
cd rust-things
moon run :dev-pipeline
```

## 📖 Usage

### CLI Commands

```bash
# Show help
things-cli --help

# Health check
things-cli health

# Show inbox tasks
things-cli inbox
things-cli inbox --limit 5

# Show today's tasks
things-cli today
things-cli today --limit 3

# Show all projects
things-cli projects
things-cli projects --area <AREA_UUID>

# Show all areas
things-cli areas

# Search for tasks
things-cli search "meeting"
things-cli search "report" --limit 10

# Start MCP server (for AI/LLM integration)
things-cli mcp
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

## 🤖 MCP Integration

The MCP (Model Context Protocol) server provides 17 tools for AI/LLM integration:

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
    "things-cli": {
      "command": "things-cli",
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
    "things-cli": {
      "type": "stdio",
      "command": "things-cli",
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
    "things-cli": {
      "command": "things-cli",
      "args": ["mcp"],
      "env": {
        "THINGS_DB_PATH": "/path/to/things.db"
      }
    }
  }
}
```

## Development

### Prerequisites

- Rust 1.70+
- Moon (for workspace management)
- Things 3 (for testing)

### Setup

```bash
# Clone the repository
git clone https://github.com/GarthDB/rust-things
cd rust-things

# Install dependencies
moon run :local-dev-setup

# Run tests
moon run :test-all

# Run development pipeline
moon run :dev-pipeline
```

### Project Structure

```
rust-things/
├── apps/
│   └── things-cli/          # CLI application with MCP server
├── libs/
│   ├── things-core/         # Core library
│   └── things-common/       # Shared utilities
├── tools/
│   └── xtask/              # Development tools
└── tests/                  # Integration tests
```

## API Reference

### Core Library

```rust
use things_core::{ThingsDatabase, Task, Project, Area};

// Create database connection
let db = ThingsDatabase::with_default_path()?;

// Get inbox tasks
let tasks = db.get_inbox(Some(10)).await?;

// Get today's tasks
let today_tasks = db.get_today(None).await?;

// Get all projects
let projects = db.get_projects(None).await?;

// Search tasks
let search_results = db.search_tasks("meeting", Some(5)).await?;
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run the development pipeline: `moon run :dev-pipeline`
6. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by [things-cli](https://github.com/thingsapi/things-cli)
- Built with [Moon](https://moonrepo.dev) workspace management
- Follows [evelion-apps/things-api](https://github.com/evelion-apps/things-api) patterns
# Test commit
