# Rust Things

A high-performance Rust library and CLI for Things 3 integration with integrated MCP server support.

## Features

- 🚀 **High Performance**: Built with Rust for maximum speed and reliability
- 🔧 **CLI Tool**: Command-line interface for managing Things 3 data
- 🤖 **MCP Integration**: Integrated MCP server for AI/LLM integration
- 📊 **Comprehensive API**: Full access to Things 3 database
- 🏗️ **Moon Workspace**: Organized monorepo with Moon build system
- 🧪 **Well Tested**: Comprehensive test suite and benchmarks

## Installation

### From Source

```bash
git clone https://github.com/GarthDB/rust-things
cd rust-things
cargo build --release
```

### Using Moon (Recommended)

```bash
# Install Moon if you haven't already
curl -fsSL https://moonrepo.dev/install | bash

# Clone and setup
git clone https://github.com/GarthDB/rust-things
cd rust-things
moon run :dev-pipeline
```

## Usage

### CLI Usage

```bash
# Show inbox tasks
things-cli inbox

# Show today's tasks
things-cli today

# Show all projects
things-cli projects

# Show all areas
things-cli areas

# Search for tasks
things-cli search "meeting"

# Health check
things-cli health

# Start MCP server (for AI/LLM integration)
things-cli mcp
```

### MCP Integration

Configure your AI/LLM environment to use the MCP server:

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
