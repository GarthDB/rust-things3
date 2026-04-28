# things3 MCP — Host Configuration

Copy-paste config for each supported host. In every snippet, replace `THINGS_DB_PATH` with the path to your Things 3 database, or omit `env` entirely to use the auto-detected default.

**Default database path** (typical macOS install):
```
~/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-*/Containers/com.culturedcode.ThingsMac/Data/Library/Application Support/Cultured Code/Things 3/Things Database.thingsdatabase/main.sqlite
```

The glob `ThingsData-*` means the exact path includes a random suffix. Either expand it manually or let the CLI auto-detect it (default when `THINGS_DB_PATH` is unset).

---

## Claude Desktop

`~/Library/Application Support/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "things3": {
      "command": "things3",
      "args": ["mcp"],
      "env": {
        "THINGS_DB_PATH": "/path/to/main.sqlite"
      }
    }
  }
}
```

Restart Claude Desktop after editing.

---

## Claude Code

```bash
# From your project directory (or globally with --global)
claude mcp add things3 -- things3 mcp
```

To set the database path:
```bash
claude mcp add things3 -e THINGS_DB_PATH=/path/to/main.sqlite -- things3 mcp
```

Or add to `.claude/settings.json` manually:
```json
{
  "mcpServers": {
    "things3": {
      "command": "things3",
      "args": ["mcp"],
      "env": {
        "THINGS_DB_PATH": "/path/to/main.sqlite"
      }
    }
  }
}
```

---

## Cursor

`.cursor/mcp.json` (project) or `~/.cursor/mcp.json` (global):

```json
{
  "mcpServers": {
    "things3": {
      "command": "things3",
      "args": ["mcp"],
      "env": {
        "THINGS_DB_PATH": "/path/to/main.sqlite",
        "RUST_LOG": "info"
      }
    }
  }
}
```

---

## VS Code

`.vscode/mcp.json`:

```json
{
  "servers": {
    "things3": {
      "type": "stdio",
      "command": "things3",
      "args": ["mcp"],
      "cwd": "${workspaceFolder}",
      "env": {
        "THINGS_DB_PATH": "/path/to/main.sqlite"
      }
    }
  }
}
```

---

## Zed

`.zed/settings.json`:

```json
{
  "mcp": {
    "things3": {
      "command": "things3",
      "args": ["mcp"],
      "env": {
        "THINGS_DB_PATH": "/path/to/main.sqlite"
      }
    }
  }
}
```

---

## Environment variables

| Variable | Default | Purpose |
|---|---|---|
| `THINGS_DB_PATH` | Auto-detected | Path to `main.sqlite` |
| `THINGS_DATABASE_PATH` | — | Deprecated alias (logs warning) |
| `RUST_LOG` | `warn` | Log verbosity (`error`/`warn`/`info`/`debug`/`trace`) |

For the full configuration reference including `mcp_config.json` and middleware options, see [`docs/MCP_INTEGRATION.md`](../../../docs/MCP_INTEGRATION.md).
