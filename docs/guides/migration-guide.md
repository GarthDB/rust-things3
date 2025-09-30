# Migration Guide: rusqlite to SQLx

This guide helps users migrate from the previous rusqlite-based version to the new SQLx-based version of Rust Things.

## Breaking Changes

### 1. Database Initialization

**Before:**
```rust
use things3_core::{ThingsDatabase, ThingsConfig};

// Synchronous initialization
let config = ThingsConfig::new("/path/to/things.db", false);
let db = ThingsDatabase::with_config(config)?;
```

**After:**
```rust
use things3_core::ThingsDatabase;
use std::path::Path;

// Async initialization
let db = ThingsDatabase::new(Path::new("/path/to/things.db")).await?;
```

### 2. All Database Operations Are Now Async

**Before:**
```rust
// Synchronous operations
let tasks = db.get_inbox(Some(10))?;
let projects = db.get_projects(None, None)?;
let areas = db.get_areas()?;
```

**After:**
```rust
// Async operations
let tasks = db.get_inbox(Some(10)).await?;
let projects = db.get_projects(None).await?;
let areas = db.get_areas().await?;
```

### 3. Error Handling Changes

**Before:**
```rust
use things3_core::ThingsError;

match db.get_inbox(Some(5)) {
    Ok(tasks) => println!("Found {} tasks", tasks.len()),
    Err(ThingsError::DatabaseNotFound) => {
        eprintln!("Database not found");
    }
    Err(ThingsError::Database(e)) => {
        eprintln!("Database error: {}", e);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

**After:**
```rust
use things3_core::ThingsError;

match db.get_inbox(Some(5)).await {
    Ok(tasks) => println!("Found {} tasks", tasks.len()),
    Err(ThingsError::Database(msg)) => {
        eprintln!("Database error: {}", msg);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

### 4. MCP Server Integration

**Before:**
```rust
use things3_cli::mcp::{ThingsMcpServer, CallToolRequest};

// Synchronous MCP server
let server = ThingsMcpServer::new(db, config)?;
let result = server.call_tool(request)?;
```

**After:**
```rust
use things3_cli::mcp::{ThingsMcpServer, CallToolRequest};
use std::sync::Arc;

// Async MCP server
let server = ThingsMcpServer::new(Arc::new(db), config);
let result = server.call_tool(request).await?;
```

## Migration Steps

### Step 1: Update Dependencies

Update your `Cargo.toml`:

```toml
[dependencies]
things3-core = { version = "0.2.0" }  # Updated version
things3-cli = { version = "0.2.0" }   # Updated version
```

### Step 2: Update Database Initialization

Replace synchronous database initialization:

```rust
// OLD
let db = ThingsDatabase::with_default_path()?;

// NEW
let db = ThingsDatabase::new(&get_default_database_path()).await?;
```

### Step 3: Make Functions Async

Update all functions that use the database to be async:

```rust
// OLD
fn process_tasks() -> Result<()> {
    let db = ThingsDatabase::with_default_path()?;
    let tasks = db.get_inbox(Some(10))?;
    // Process tasks...
    Ok(())
}

// NEW
async fn process_tasks() -> Result<()> {
    let db = ThingsDatabase::new(&get_default_database_path()).await?;
    let tasks = db.get_inbox(Some(10)).await?;
    // Process tasks...
    Ok(())
}
```

### Step 4: Update Main Function

Make your main function async:

```rust
// OLD
fn main() -> Result<()> {
    process_tasks()?;
    Ok(())
}

// NEW
#[tokio::main]
async fn main() -> Result<()> {
    process_tasks().await?;
    Ok(())
}
```

### Step 5: Update Error Handling

Simplify error handling for database errors:

```rust
// OLD
match db.get_inbox(Some(5)) {
    Err(ThingsError::DatabaseNotFound) => {
        // Handle database not found
    }
    Err(ThingsError::Database(e)) => {
        // Handle database error
    }
    // ... other error types
}

// NEW
match db.get_inbox(Some(5)).await {
    Err(ThingsError::Database(msg)) => {
        // Handle database error (simplified)
        eprintln!("Database error: {}", msg);
    }
    // ... other error types
}
```

## New Features

### Web Servers

The new version includes built-in web servers:

```rust
// Health check server
things3 health-server --port 8080

// Monitoring dashboard
things3 dashboard --port 8081
```

### Improved Performance

- Better concurrent database operations
- Connection pooling
- Reduced memory usage
- Thread-safe operations

### Better Async Integration

- Native async/await support
- Better integration with tokio runtime
- Improved error handling

## Testing

Update your tests to use async:

```rust
#[tokio::test]
async fn test_get_inbox() {
    let db = ThingsDatabase::new(Path::new(":memory:")).await.unwrap();
    let tasks = db.get_inbox(Some(10)).await.unwrap();
    assert!(!tasks.is_empty());
}
```

## Troubleshooting

### Common Issues

1. **Missing `.await`**: Make sure to add `.await` to all database operations
2. **Sync context**: Ensure you're in an async context when calling database methods
3. **Error types**: Update error handling to use the simplified error types
4. **Dependencies**: Make sure you're using the updated version of the crates

### Getting Help

If you encounter issues during migration:

1. Check the [GitHub Issues](https://github.com/GarthDB/rust-things3/issues)
2. Review the [API documentation](docs/api/README.md)
3. Look at the [examples](docs/examples/README.md)

## Benefits of Migration

- **Better Performance**: SQLx provides better performance for concurrent operations
- **Thread Safety**: Proper `Send + Sync` support for web servers
- **Modern Rust**: Better integration with async Rust patterns
- **Simplified Architecture**: Cleaner code without workarounds
- **Future-Proof**: Built on modern async database libraries

## Conclusion

The migration to SQLx provides significant improvements in performance, thread safety, and code quality. While it requires updating your code to use async patterns, the benefits far outweigh the migration effort.

