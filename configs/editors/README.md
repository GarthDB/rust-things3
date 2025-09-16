# Editor Configuration Files

This directory contains MCP (Model Context Protocol) configuration files for different editors to integrate with the Things 3 CLI.

## Available Configurations

### Cursor (`cursor.json`)
Configuration for Cursor editor with MCP server integration.

### VS Code (`vscode.json`)
Configuration for Visual Studio Code with MCP server integration.

### Zed (`zed.json`)
Configuration for Zed editor with MCP server integration.

### Generic MCP (`mcp-config.json`)
Generic MCP configuration that can be used with any MCP-compatible client.

## Installation

### Cursor

1. Copy the contents of `cursor.json`
2. Add to your Cursor settings under MCP servers
3. Ensure `things-cli` is in your PATH

### VS Code

1. Copy the contents of `vscode.json`
2. Add to your VS Code settings under MCP servers
3. Ensure `things-cli` is in your PATH

### Zed

1. Copy the contents of `zed.json`
2. Add to your Zed settings under MCP servers
3. Ensure `things-cli` is in your PATH

### Generic MCP Client

1. Use `mcp-config.json` as a reference
2. Configure your MCP client with the provided settings
3. Ensure `things-cli` is in your PATH

## Configuration Details

All configurations include:

- **Command**: `things-cli`
- **Args**: `["mcp"]`
- **Environment**: `RUST_LOG=info` for logging

## Available Tools

The MCP server provides the following tools:

- `get_inbox` - Get tasks from the inbox
- `get_today` - Get tasks scheduled for today
- `get_projects` - Get all projects
- `get_areas` - Get all areas
- `search_tasks` - Search for tasks
- `create_task` - Create a new task
- `update_task` - Update an existing task
- `get_productivity_metrics` - Get productivity metrics
- `export_data` - Export data in various formats
- `bulk_create_tasks` - Create multiple tasks
- `get_recent_tasks` - Get recently created/modified tasks

## Troubleshooting

### Common Issues

1. **Command not found**: Ensure `things-cli` is installed and in your PATH
2. **Permission denied**: Check file permissions for the Things 3 database
3. **Database not found**: Verify Things 3 is installed and the database exists

### Debug Mode

To enable debug logging, modify the environment variable in the configuration:

```json
{
  "env": {
    "RUST_LOG": "debug"
  }
}
```

### Testing the Configuration

Test the MCP server manually:

```bash
things-cli mcp
```

The server should start and display available tools.

## Support

For issues with editor integration:

1. Check the [main CLI documentation](../docs/CLI.md)
2. Verify your editor supports MCP
3. Test the CLI manually first
4. Check editor-specific MCP documentation
