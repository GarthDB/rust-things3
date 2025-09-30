# Things CLI Documentation

## Overview

Things CLI is a command-line interface for Things 3 with integrated MCP (Model Context Protocol) server capabilities. It provides access to your Things 3 database through both traditional CLI commands and AI/LLM integration via MCP.

## Installation

### From Source

```bash
git clone https://github.com/GarthDB/rust-things3.git
cd rust-things
cargo build --release --bin things-cli
```

### Binary Installation

```bash
# Download the latest release binary for your platform
# Place it in your PATH
```

## Basic Usage

### Command Structure

```bash
things-cli [OPTIONS] <COMMAND>
```

### Global Options

- `-d, --database <DATABASE>`: Database path (defaults to Things 3 default location)
- `--fallback-to-default`: Fall back to default database path if specified path doesn't exist
- `-v, --verbose`: Enable verbose output
- `-h, --help`: Print help
- `-V, --version`: Print version

## Commands

### 1. Inbox

Show tasks from the inbox.

```bash
things3 inbox [OPTIONS]
```

**Options:**
- `-l, --limit <LIMIT>`: Limit number of results

**Example:**
```bash
things-cli inbox --limit 10
```

### 2. Today

Show tasks scheduled for today.

```bash
things-cli today [OPTIONS]
```

**Options:**
- `-l, --limit <LIMIT>`: Limit number of results

**Example:**
```bash
things-cli today
```

### 3. Projects

Show all projects, optionally filtered by area.

```bash
things-cli projects [OPTIONS]
```

**Options:**
- `--area <AREA>`: Filter by area UUID

**Example:**
```bash
things-cli projects --area "15c0f1a2-3b4c-5d6e-7f8a-9b0c1d2e3f4a"
```

### 4. Areas

Show all areas.

```bash
things-cli areas
```

**Example:**
```bash
things-cli areas
```

### 5. Search

Search for tasks by query.

```bash
things-cli search <QUERY> [OPTIONS]
```

**Arguments:**
- `<QUERY>`: Search query

**Options:**
- `-l, --limit <LIMIT>`: Limit number of results

**Example:**
```bash
things-cli search "meeting" --limit 5
```

### 6. MCP Server

Start the MCP server for AI/LLM integration.

```bash
things-cli mcp
```

**Example:**
```bash
things-cli mcp
```

This starts the MCP server that can be used by AI assistants and LLM applications.

### 7. Health Check

Check database connection and system health.

```bash
things-cli health
```

**Example:**
```bash
things-cli health
```

## MCP Integration

### Overview

The MCP (Model Context Protocol) server allows AI assistants and LLM applications to interact with your Things 3 data. The server provides a set of tools that can be called by MCP-compatible clients.

### Available MCP Tools

#### 1. `get_inbox`
Get tasks from the inbox.

**Parameters:**
- `limit` (optional): Maximum number of tasks to return

**Example:**
```json
{
  "name": "get_inbox",
  "arguments": {
    "limit": 10
  }
}
```

#### 2. `get_today`
Get tasks scheduled for today.

**Parameters:**
- `limit` (optional): Maximum number of tasks to return

#### 3. `get_projects`
Get all projects, optionally filtered by area.

**Parameters:**
- `area_uuid` (optional): Area UUID to filter projects

#### 4. `get_areas`
Get all areas.

**Parameters:** None

#### 5. `search_tasks`
Search for tasks by query.

**Parameters:**
- `query` (required): Search query
- `limit` (optional): Maximum number of tasks to return

#### 6. `create_task`
Create a new task.

**Parameters:**
- `title` (required): Task title
- `notes` (optional): Task notes
- `project_uuid` (optional): Project UUID
- `area_uuid` (optional): Area UUID

#### 7. `update_task`
Update an existing task.

**Parameters:**
- `uuid` (required): Task UUID
- `title` (optional): New task title
- `notes` (optional): New task notes
- `status` (optional): New task status (incomplete, completed, canceled, trashed)

#### 8. `get_productivity_metrics`
Get productivity metrics and statistics.

**Parameters:**
- `days` (optional): Number of days to look back for metrics (default: 7)

#### 9. `export_data`
Export data in various formats.

**Parameters:**
- `format` (required): Export format (json, csv, markdown)
- `data_type` (required): Type of data to export (tasks, projects, areas, all)

#### 10. `bulk_create_tasks`
Create multiple tasks at once.

**Parameters:**
- `tasks` (required): Array of task objects to create

#### 11. `get_recent_tasks`
Get recently created or modified tasks.

**Parameters:**
- `limit` (optional): Maximum number of tasks to return
- `hours` (optional): Number of hours to look back (default: 24)

### Editor Configuration

#### Cursor

Add to your Cursor settings:

```json
{
  "mcpServers": {
    "things3": {
      "command": "things-cli",
      "args": ["mcp"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

#### VS Code

Add to your VS Code settings:

```json
{
  "mcpServers": {
    "things3": {
      "command": "things-cli",
      "args": ["mcp"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

#### Zed

Add to your Zed settings:

```json
{
  "mcpServers": {
    "things3": {
      "command": "things-cli",
      "args": ["mcp"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

## Examples

### Basic CLI Usage

```bash
# Check system health
things-cli health

# Get today's tasks
things-cli today

# Search for tasks
things-cli search "project planning"

# Get projects in a specific area
things-cli projects --area "15c0f1a2-3b4c-5d6e-7f8a-9b0c1d2e3f4a"

# Get inbox with limit
things-cli inbox --limit 5
```

### MCP Server Usage

```bash
# Start MCP server
things-cli mcp

# The server will run until stopped with Ctrl+C
# AI assistants can then connect and use the available tools
```

### Database Path Configuration

```bash
# Use custom database path
things-cli --database "/path/to/things.sqlite" inbox

# Use custom path with fallback to default
things-cli --database "/path/to/things.sqlite" --fallback-to-default inbox
```

## Troubleshooting

### Common Issues

1. **Database not found**: Ensure Things 3 is installed and the database exists at the default location, or specify a custom path with `--database`.

2. **Permission denied**: Make sure you have read access to the Things 3 database file.

3. **MCP server not starting**: Check that the `things-cli` binary is in your PATH and has execute permissions.

### Debug Mode

Use the `--verbose` flag to enable debug logging:

```bash
things3 --verbose health
```

### 8. Health Server

Start a health check web server.

```bash
things3 health-server [OPTIONS]
```

**Options:**
- `-p, --port <PORT>`: Port number (default: 8080)

**Example:**
```bash
# Start health server on default port 8080
things3 health-server

# Start health server on custom port
things3 health-server --port 9090

# Test health endpoints
curl http://localhost:8080/health
curl http://localhost:8080/ping
```

**Endpoints:**
- `GET /health`: Comprehensive health check
- `GET /ping`: Simple ping endpoint
- `GET /ready`: Readiness check
- `GET /live`: Liveness check

### 9. Dashboard

Start a monitoring dashboard web server.

```bash
things3 dashboard [OPTIONS]
```

**Options:**
- `-p, --port <PORT>`: Port number (default: 8081)

**Example:**
```bash
# Start dashboard on default port 8081
things3 dashboard

# Start dashboard on custom port
things3 dashboard --port 9091

# Access dashboard
open http://localhost:8081
```

**Features:**
- Real-time metrics and statistics
- Database health monitoring
- Performance metrics
- System resource usage
- Task and project analytics

### Logging

Set the `RUST_LOG` environment variable for more detailed logging:

```bash
RUST_LOG=debug things3 health
```

## Development

### Building from Source

```bash
git clone https://github.com/GarthDB/rust-things3.git
cd rust-things
cargo build --bin things-cli
```

### Running Tests

```bash
cargo test --features test-utils
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

For issues and questions:

1. Check the [GitHub Issues](https://github.com/GarthDB/rust-things3/issues)
2. Create a new issue with detailed information
3. Include system information and error messages
