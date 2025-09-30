# MCP Integration Examples

This guide shows how to integrate the Rust Things MCP server with various AI/LLM environments and editors.

## Table of Contents

- [Cursor Integration](#cursor-integration)
- [VS Code Integration](#vs-code-integration)
- [Zed Integration](#zed-integration)
- [Custom MCP Client](#custom-mcp-client)
- [Python Integration](#python-integration)
- [Node.js Integration](#nodejs-integration)

## Cursor Integration

### Basic Setup

1. **Install things-cli**:
   ```bash
   cargo install --git https://github.com/GarthDB/rust-things3
   ```

2. **Create MCP configuration**:
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

3. **Restart Cursor** to load the MCP server.

### Usage Examples

Once configured, you can use the MCP tools in Cursor:

```
# Get today's tasks
@things-cli get_today

# Search for tasks
@things-cli search_tasks "meeting"

# Create a new task
@things-cli create_task "Review quarterly report" --deadline "2024-01-31"

# Get productivity metrics
@things-cli get_productivity_metrics --start_date "2024-01-01" --end_date "2024-01-31"
```

## VS Code Integration

### Setup with MCP Extension

1. **Install the MCP extension** for VS Code
2. **Create configuration**:
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

### Usage in VS Code

The MCP tools will be available in the command palette and can be used in chat interfaces.

## Zed Integration

### Setup

1. **Create Zed configuration**:
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

2. **Restart Zed** to load the configuration.

## Custom MCP Client

### Rust Client Example

```rust
use serde_json::{json, Value};
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Write};

struct ThingsMcpClient {
    process: std::process::Child,
}

impl ThingsMcpClient {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut process = Command::new("things-cli")
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        Ok(Self { process })
    }
    
    fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let request = json!({
            "name": name,
            "arguments": arguments
        });
        
        let stdin = self.process.stdin.as_mut().unwrap();
        stdin.write_all(serde_json::to_string(&request)?.as_bytes())?;
        stdin.write_all(b"\n")?;
        
        let stdout = self.process.stdout.as_mut().unwrap();
        let reader = BufReader::new(stdout);
        let mut response = String::new();
        reader.read_line(&mut response)?;
        
        let result: Value = serde_json::from_str(&response)?;
        Ok(result)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ThingsMcpClient::new()?;
    
    // Get inbox tasks
    let inbox = client.call_tool("get_inbox", json!({"limit": 10}))?;
    println!("Inbox tasks: {}", serde_json::to_string_pretty(&inbox)?);
    
    // Get today's tasks
    let today = client.call_tool("get_today", json!({}))?;
    println!("Today's tasks: {}", serde_json::to_string_pretty(&today)?);
    
    // Search tasks
    let search = client.call_tool("search_tasks", json!({
        "query": "meeting",
        "limit": 5
    }))?;
    println!("Search results: {}", serde_json::to_string_pretty(&search)?);
    
    Ok(())
}
```

## Python Integration

### Python MCP Client

```python
import subprocess
import json
import sys

class ThingsMcpClient:
    def __init__(self):
        self.process = subprocess.Popen(
            ['things-cli', 'mcp'],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
    
    def call_tool(self, name, arguments=None):
        if arguments is None:
            arguments = {}
        
        request = {
            "name": name,
            "arguments": arguments
        }
        
        self.process.stdin.write(json.dumps(request) + '\n')
        self.process.stdin.flush()
        
        response = self.process.stdout.readline()
        return json.loads(response)
    
    def close(self):
        self.process.terminate()
        self.process.wait()

# Usage example
def main():
    client = ThingsMcpClient()
    
    try:
        # Get inbox tasks
        inbox = client.call_tool("get_inbox", {"limit": 10})
        print("Inbox tasks:", json.dumps(inbox, indent=2))
        
        # Get today's tasks
        today = client.call_tool("get_today")
        print("Today's tasks:", json.dumps(today, indent=2))
        
        # Search tasks
        search = client.call_tool("search_tasks", {
            "query": "meeting",
            "limit": 5
        })
        print("Search results:", json.dumps(search, indent=2))
        
        # Create a task
        new_task = client.call_tool("create_task", {
            "title": "Review project proposal",
            "notes": "Focus on technical feasibility",
            "deadline": "2024-01-31"
        })
        print("Created task:", json.dumps(new_task, indent=2))
        
    finally:
        client.close()

if __name__ == "__main__":
    main()
```

## Node.js Integration

### Node.js MCP Client

```javascript
const { spawn } = require('child_process');
const readline = require('readline');

class ThingsMcpClient {
    constructor() {
        this.process = spawn('things-cli', ['mcp'], {
            stdio: ['pipe', 'pipe', 'pipe']
        });
        
        this.rl = readline.createInterface({
            input: this.process.stdout,
            output: process.stdout
        });
    }
    
    async callTool(name, arguments = {}) {
        return new Promise((resolve, reject) => {
            const request = {
                name,
                arguments
            };
            
            this.process.stdin.write(JSON.stringify(request) + '\n');
            
            this.rl.once('line', (line) => {
                try {
                    const response = JSON.parse(line);
                    resolve(response);
                } catch (error) {
                    reject(error);
                }
            });
        });
    }
    
    close() {
        this.process.kill();
        this.rl.close();
    }
}

// Usage example
async function main() {
    const client = new ThingsMcpClient();
    
    try {
        // Get inbox tasks
        const inbox = await client.callTool('get_inbox', { limit: 10 });
        console.log('Inbox tasks:', JSON.stringify(inbox, null, 2));
        
        // Get today's tasks
        const today = await client.callTool('get_today');
        console.log('Today\'s tasks:', JSON.stringify(today, null, 2));
        
        // Search tasks
        const search = await client.callTool('search_tasks', {
            query: 'meeting',
            limit: 5
        });
        console.log('Search results:', JSON.stringify(search, null, 2));
        
        // Create a task
        const newTask = await client.callTool('create_task', {
            title: 'Review project proposal',
            notes: 'Focus on technical feasibility',
            deadline: '2024-01-31'
        });
        console.log('Created task:', JSON.stringify(newTask, null, 2));
        
    } finally {
        client.close();
    }
}

main().catch(console.error);
```

## Advanced Integration Patterns

### Error Handling

```python
def safe_call_tool(client, name, arguments=None):
    try:
        result = client.call_tool(name, arguments)
        if result.get('is_error', False):
            print(f"Error in {name}: {result}")
            return None
        return result
    except Exception as e:
        print(f"Exception calling {name}: {e}")
        return None

# Usage
inbox = safe_call_tool(client, "get_inbox", {"limit": 10})
if inbox:
    print("Successfully got inbox tasks")
```

### Batch Operations

```python
def batch_operations(client):
    operations = [
        ("get_inbox", {"limit": 10}),
        ("get_today", {}),
        ("get_areas", {}),
        ("get_projects", {})
    ]
    
    results = {}
    for name, args in operations:
        result = safe_call_tool(client, name, args)
        if result:
            results[name] = result
    
    return results
```

### Performance Monitoring

```python
import time

def timed_call_tool(client, name, arguments=None):
    start_time = time.time()
    result = client.call_tool(name, arguments)
    end_time = time.time()
    
    print(f"{name} took {end_time - start_time:.2f} seconds")
    return result
```

## Troubleshooting

### Common Issues

1. **MCP server not starting**
   - Check if `things-cli` is in PATH
   - Verify database path is correct
   - Check permissions

2. **Connection errors**
   - Ensure MCP server is running
   - Check stdin/stdout pipes
   - Verify JSON format

3. **Tool errors**
   - Check tool name spelling
   - Verify argument format
   - Check error messages

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=debug
things-cli mcp
```

### Testing MCP Server

```bash
# Test MCP server manually
echo '{"name": "get_inbox", "arguments": {"limit": 5}}' | things-cli mcp
```

## Best Practices

1. **Always handle errors** gracefully
2. **Use appropriate timeouts** for operations
3. **Close connections** properly
4. **Monitor performance** of MCP calls
5. **Test integrations** thoroughly
6. **Use batch operations** when possible
7. **Cache results** when appropriate
8. **Log operations** for debugging
