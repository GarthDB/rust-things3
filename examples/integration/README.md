# Integration Examples

This directory contains comprehensive examples demonstrating how to integrate `rust-things3` into different types of applications and workflows.

## Examples Overview

### 1. MCP Client (`mcp_client.rs`)

**What it does**: Demonstrates how to build a custom MCP (Model Context Protocol) client that communicates with the Things 3 MCP server.

**Use cases**:
- Building custom AI/LLM integrations
- Creating automated workflows
- Testing MCP server functionality
- Integrating with AI tools like Claude, ChatGPT

**Run it**:
```bash
cargo run --example mcp_client --features mcp-server
```

**Key concepts**:
- JSON-RPC protocol communication
- Tool discovery and invocation
- Error handling
- Async/await patterns

---

### 2. CLI Extension (`cli_extension.rs`)

**What it does**: Shows how to extend the Things 3 CLI with custom commands and functionality.

**Use cases**:
- Adding organization-specific commands
- Creating custom workflows (e.g., weekly reports)
- Building specialized tools (e.g., overdue task finder)
- Custom bulk operations

**Run it**:
```bash
cargo run --example cli_extension -- --help
cargo run --example cli_extension -- overdue
cargo run --example cli_extension -- weekly-report
```

**Key concepts**:
- Custom CLI with `clap`
- Extending existing functionality
- Custom command implementation
- Formatted output (text, JSON, Markdown)

---

### 3. Web API (`web_api.rs`)

**What it does**: Builds a RESTful API on top of Things 3 using the Axum web framework.

**Use cases**:
- Building web dashboards
- Creating mobile app backends
- Team access to Things 3 data
- Integrating with other systems via HTTP

**Run it**:
```bash
cargo run --example web_api
```

**Then test with**:
```bash
# Health check
curl http://localhost:3000/health

# Get inbox
curl http://localhost:3000/api/inbox?limit=5

# Search tasks
curl "http://localhost:3000/api/search?q=meeting"

# Create task
curl -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{"title": "New task from API"}'

# Get statistics
curl http://localhost:3000/api/stats
```

**Key concepts**:
- REST API design with Axum
- Request/response handling
- Query parameters and path parameters
- Error handling
- CORS configuration

---

### 4. Background Service (`background_service.rs`)

**What it does**: Demonstrates a long-running background service with graceful shutdown handling.

**Use cases**:
- Scheduled task processing
- Automated notifications
- Data synchronization
- Monitoring and alerting
- Continuous integration

**Run it**:
```bash
cargo run --example background_service
```

**Stop with**: `Ctrl+C` (graceful shutdown)

**Key concepts**:
- Worker task patterns
- Graceful shutdown handling
- Signal handling (Ctrl+C, SIGTERM)
- Periodic task execution
- Health monitoring
- Multi-worker coordination

---

### 5. Custom Middleware (`custom_middleware.rs`)

**What it does**: Shows how to create custom middleware to extend the library's functionality.

**Use cases**:
- Custom caching strategies
- Request/response transformation
- Logging and monitoring
- Performance tracking
- Rate limiting
- Access control

**Run it**:
```bash
cargo run --example custom_middleware
```

**Key concepts**:
- Middleware pattern
- Request interception
- Performance monitoring
- Caching implementation
- Rate limiting
- Validation and transformation

---

## Running Examples

### Prerequisites

1. **Rust 1.70+**:
   ```bash
   rustup update
   ```

2. **Things 3 installed** (macOS):
   - Available from the Mac App Store
   - Or have access to a Things 3 database file

3. **Database path** (optional):
   ```bash
   export THINGS_DB_PATH="/path/to/your/things.db"
   ```

### Quick Start

```bash
# Clone the repository
git clone https://github.com/GarthDB/rust-things3
cd rust-things3

# Run any example
cargo run --example <example_name>

# With features
cargo run --example mcp_client --features mcp-server

# With release optimizations
cargo run --release --example web_api
```

## Example Combinations

### Building a Complete System

You can combine these examples to build a complete system:

```
┌─────────────────┐
│  Background     │ ← Monitors tasks, generates reports
│  Service        │
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│  Things 3       │ ← Core database
│  Database       │
└────────┬────────┘
         │
    ┌────┴────┬────────┬────────┐
    ↓         ↓        ↓        ↓
┌──────┐  ┌──────┐ ┌─────┐  ┌─────┐
│ Web  │  │ MCP  │ │ CLI │  │More │
│ API  │  │Server│ │Ext  │  │...  │
└──────┘  └──────┘ └─────┘  └─────┘
```

### Example Workflow

1. **Development**: Use `cli_extension` for custom commands
2. **Testing**: Use `mcp_client` to test MCP integration
3. **Production**: Deploy `web_api` and `background_service`
4. **Monitoring**: Use `custom_middleware` for observability

## Advanced Topics

### 1. Database Connection Pooling

```rust
use things3_core::{ThingsDatabase, DatabasePoolConfig};

let pool_config = DatabasePoolConfig {
    max_connections: 10,
    min_connections: 2,
    ..Default::default()
};

let db = ThingsDatabase::with_pool_config(&path, pool_config).await?;
```

### 2. Error Handling

```rust
use things3_core::{ThingsError, Result};

match db.get_inbox(None).await {
    Ok(tasks) => println!("Got {} tasks", tasks.len()),
    Err(ThingsError::DatabaseNotFound { path }) => {
        eprintln!("Database not found: {}", path);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

### 3. Async Patterns

```rust
use tokio::task::JoinSet;

let mut join_set = JoinSet::new();

for i in 0..10 {
    let db = Arc::clone(&db);
    join_set.spawn(async move {
        db.get_inbox(Some(10)).await
    });
}

while let Some(result) = join_set.join_next().await {
    match result {
        Ok(Ok(tasks)) => println!("Got tasks: {}", tasks.len()),
        Ok(Err(e)) => eprintln!("Error: {}", e),
        Err(e) => eprintln!("Join error: {}", e),
    }
}
```

## Troubleshooting

### "Database not found"

Set the database path:
```bash
export THINGS_DB_PATH="/path/to/things.db"

# Or find it automatically:
find ~/Library/Group\ Containers -name "main.sqlite" 2>/dev/null
```

### "Permission denied"

Close Things 3 app before running examples:
```bash
killall Things3
```

### Compilation errors

Update dependencies:
```bash
cargo update
cargo clean
cargo build
```

## Contributing

Want to add more examples? We welcome contributions!

1. Fork the repository
2. Create a new example in `examples/integration/`
3. Add documentation to this README
4. Submit a pull request

## Related Documentation

- [Feature Flags](../../docs/FEATURES.md) - Optional functionality
- [User Guide](../../docs/USER_GUIDE.md) - Comprehensive usage guide
- [Architecture](../../docs/ARCHITECTURE.md) - System design
- [MCP Integration](../../docs/MCP_INTEGRATION.md) - MCP server details
- [Performance Guide](../../docs/PERFORMANCE.md) - Optimization strategies

## Support

- Issues: https://github.com/GarthDB/rust-things3/issues
- Discussions: https://github.com/GarthDB/rust-things3/discussions
- Documentation: https://docs.rs/things3-core

---

**Happy Coding!** 🚀

