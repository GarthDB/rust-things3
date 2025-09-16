# Examples and Integration Guides

This section contains practical examples and integration guides for using Rust Things in various scenarios.

## Table of Contents

- [Basic Usage](./basic-usage.md) - Getting started with the CLI and library
- [MCP Integration](./mcp-integration.md) - Setting up MCP with different editors
- [Performance Monitoring](./performance-monitoring.md) - Using performance features
- [Data Export](./data-export.md) - Exporting data in different formats
- [Backup and Restore](./backup-restore.md) - Managing backups
- [Custom Scripts](./custom-scripts.md) - Building custom automation
- [CI/CD Integration](./cicd-integration.md) - Using in CI/CD pipelines

## Quick Start Examples

### CLI Examples

```bash
# Basic task management
things-cli inbox
things-cli today
things-cli search "meeting"

# Health check
things-cli health

# Start MCP server
things-cli mcp
```

### Library Examples

```rust
use things_core::{ThingsDatabase, ThingsConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create database connection
    let config = ThingsConfig::from_env();
    let db = ThingsDatabase::new(&config)?;
    
    // Get inbox tasks
    let tasks = db.get_inbox(Some(10)).await?;
    println!("Found {} inbox tasks", tasks.len());
    
    // Get today's tasks
    let today_tasks = db.get_today(None).await?;
    println!("Found {} tasks for today", today_tasks.len());
    
    // Search tasks
    let search_results = db.search_tasks("meeting", Some(5)).await?;
    println!("Found {} matching tasks", search_results.len());
    
    Ok(())
}
```

### MCP Integration Example

```json
// .cursor/mcp.json
{
  "mcpServers": {
    "things-cli": {
      "command": "things-cli",
      "args": ["mcp"],
      "env": {
        "THINGS_DB_PATH": "/Users/username/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/Data/Things Database.thingsdatabase/main.sqlite"
      }
    }
  }
}
```

## Common Use Cases

### 1. Daily Task Review
```bash
# Get today's tasks
things-cli today

# Get inbox tasks
things-cli inbox

# Search for specific tasks
things-cli search "urgent"
```

### 2. Project Management
```bash
# List all projects
things-cli projects

# List projects in specific area
things-cli projects --area <AREA_UUID>

# List all areas
things-cli areas
```

### 3. Data Export
```bash
# Export to JSON
things-cli mcp # Then use export_data tool

# Export to CSV
things-cli mcp # Then use export_data tool with format=csv
```

### 4. Backup Management
```bash
# Create backup
things-cli mcp # Then use backup_database tool

# List backups
things-cli mcp # Then use list_backups tool

# Restore from backup
things-cli mcp # Then use restore_database tool
```

## Integration Patterns

### With AI/LLM Tools
- Use MCP server for AI tool integration
- Configure editors with MCP support
- Use tools for task automation

### With CI/CD
- Use CLI in build scripts
- Export data for reporting
- Monitor performance metrics

### With Custom Applications
- Use library directly in Rust applications
- Integrate with web services
- Build custom dashboards

## Troubleshooting

### Common Issues

1. **Database not found**
   - Check `THINGS_DB_PATH` environment variable
   - Enable fallback with `THINGS_FALLBACK_TO_DEFAULT=true`

2. **MCP server not responding**
   - Ensure `things-cli mcp` is running
   - Check editor configuration

3. **Performance issues**
   - Use caching features
   - Monitor with performance tools
   - Check system resources

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=debug
things-cli health
```

## Best Practices

1. **Use appropriate limits** for queries
2. **Handle errors gracefully** in scripts
3. **Monitor performance** regularly
4. **Backup data** before major changes
5. **Use caching** for frequently accessed data
6. **Test integrations** thoroughly
