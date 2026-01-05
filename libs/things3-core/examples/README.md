# Examples

This directory contains practical examples demonstrating how to use `things3-core` and `things3-cli`.

## Running Examples

```bash
# Basic usage
cargo run --example basic_usage

# Search tasks
cargo run --example search_tasks -- "meeting"

# Export data
cargo run --example export_data
```

## Available Examples

### `basic_usage.rs`
Demonstrates basic database operations:
- Connecting to the database
- Getting inbox tasks
- Getting today's tasks
- Listing projects
- Listing areas

### `search_tasks.rs`
Shows how to search for tasks:
- Search by title or notes
- Display search results with details

### `export_data.rs`
Demonstrates data export functionality:
- Export to JSON
- Export to CSV
- Export to Markdown

## Environment Variables

All examples support these environment variables:

```bash
# Custom database path
export THINGS_DB_PATH="/path/to/things.db"

# Enable debug logging
export RUST_LOG=debug

# Fallback to default path
export THINGS_FALLBACK_TO_DEFAULT=true
```

## More Examples

For more advanced examples, see:
- [MCP Integration Guide](../docs/MCP_INTEGRATION.md)
- [Architecture Documentation](../docs/ARCHITECTURE.md)
- [Development Guide](../docs/DEVELOPMENT.md)

