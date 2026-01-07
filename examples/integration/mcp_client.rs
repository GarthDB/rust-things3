//! MCP Client Example
//!
//! This example demonstrates how to create a custom MCP (Model Context Protocol) client
//! that connects to the Things 3 MCP server and makes tool calls.
//!
//! This is useful for:
//! - Building custom AI/LLM integrations
//! - Creating automated workflows
//! - Testing MCP server functionality
//!
//! Run this example with:
//! ```bash
//! cargo run --example mcp_client --features mcp-server
//! ```

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Simple MCP client that communicates with the Things 3 MCP server
struct McpClient {
    process: std::process::Child,
    request_id: u64,
}

impl McpClient {
    /// Start a new MCP server process
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let process = Command::new("cargo")
            .args(["run", "--bin", "things3", "--features", "mcp-server", "--", "mcp"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()) // Suppress stderr to avoid interfering with JSON-RPC
            .spawn()?;

        Ok(Self {
            process,
            request_id: 0,
        })
    }

    /// Send a JSON-RPC request to the server
    fn send_request(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error>> {
        self.request_id += 1;
        
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params
        });

        // Write request to stdin
        if let Some(stdin) = &mut self.process.stdin {
            let request_str = serde_json::to_string(&request)?;
            writeln!(stdin, "{}", request_str)?;
            stdin.flush()?;
        }

        // Read response from stdout
        if let Some(stdout) = &mut self.process.stdout {
            let mut reader = BufReader::new(stdout);
            let mut response_line = String::new();
            reader.read_line(&mut response_line)?;
            
            let response: Value = serde_json::from_str(&response_line)?;
            Ok(response)
        } else {
            Err("No stdout available".into())
        }
    }

    /// Initialize the MCP connection
    fn initialize(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        self.send_request("initialize", json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "example-mcp-client",
                "version": "0.1.0"
            }
        }))
    }

    /// List available tools
    fn list_tools(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        self.send_request("tools/list", json!({}))
    }

    /// Call a tool
    fn call_tool(&mut self, tool_name: &str, arguments: Value) -> Result<Value, Box<dyn std::error::Error>> {
        self.send_request("tools/call", json!({
            "name": tool_name,
            "arguments": arguments
        }))
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting MCP Client Example\n");

    // Start MCP server
    println!("Starting MCP server...");
    let mut client = McpClient::new()?;

    // Initialize connection
    println!("Initializing connection...");
    let init_response = client.initialize()?;
    println!("✓ Initialized: {}\n", init_response);

    // List available tools
    println!("Listing available tools...");
    let tools_response = client.list_tools()?;
    println!("✓ Available tools: {}\n", tools_response);

    // Example 1: Get inbox tasks
    println!("Example 1: Getting inbox tasks...");
    let inbox_response = client.call_tool("get_inbox", json!({
        "limit": 5
    }))?;
    println!("✓ Inbox response: {}\n", inbox_response);

    // Example 2: Search tasks
    println!("Example 2: Searching for tasks...");
    let search_response = client.call_tool("search_tasks", json!({
        "query": "meeting",
        "limit": 3
    }))?;
    println!("✓ Search response: {}\n", search_response);

    // Example 3: Get today's tasks
    println!("Example 3: Getting today's tasks...");
    let today_response = client.call_tool("get_today", json!({
        "limit": 5
    }))?;
    println!("✓ Today response: {}\n", today_response);

    // Example 4: Create a task
    println!("Example 4: Creating a task...");
    let create_response = client.call_tool("create_task", json!({
        "title": "Example task created by MCP client",
        "notes": "This task was created programmatically via the MCP protocol"
    }))?;
    println!("✓ Create response: {}\n", create_response);

    println!("✅ MCP Client Example Complete!");

    Ok(())
}

/* 
 * Alternative: Using async/await with tokio
 * 
 * If you want to build a more sophisticated async client, here's a pattern:
 */

#[cfg(feature = "async-example")]
mod async_client {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::process::{Child, Command};
    use serde_json::{json, Value};

    pub struct AsyncMcpClient {
        process: Child,
        request_id: u64,
    }

    impl AsyncMcpClient {
        pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let process = Command::new("cargo")
                .args(["run", "--bin", "things3", "--features", "mcp-server", "--", "mcp"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()?;

            Ok(Self {
                process,
                request_id: 0,
            })
        }

        pub async fn send_request(
            &mut self,
            method: &str,
            params: Value,
        ) -> Result<Value, Box<dyn std::error::Error>> {
            self.request_id += 1;

            let request = json!({
                "jsonrpc": "2.0",
                "id": self.request_id,
                "method": method,
                "params": params
            });

            // Write request
            if let Some(stdin) = &mut self.process.stdin {
                let request_str = serde_json::to_string(&request)?;
                stdin.write_all(request_str.as_bytes()).await?;
                stdin.write_all(b"\n").await?;
                stdin.flush().await?;
            }

            // Read response
            if let Some(stdout) = &mut self.process.stdout {
                let mut reader = BufReader::new(stdout);
                let mut response_line = String::new();
                reader.read_line(&mut response_line).await?;

                let response: Value = serde_json::from_str(&response_line)?;
                Ok(response)
            } else {
                Err("No stdout available".into())
            }
        }

        pub async fn call_tool(
            &mut self,
            tool_name: &str,
            arguments: Value,
        ) -> Result<Value, Box<dyn std::error::Error>> {
            self.send_request("tools/call", json!({
                "name": tool_name,
                "arguments": arguments
            })).await
        }
    }
}

/*
 * Usage Tips:
 * 
 * 1. Error Handling: In production, add proper error handling and retries
 * 2. Connection Management: Implement connection pooling for multiple clients
 * 3. Streaming: For large result sets, implement streaming responses
 * 4. Authentication: Add authentication if needed for multi-user scenarios
 * 5. Logging: Add structured logging for debugging
 */

