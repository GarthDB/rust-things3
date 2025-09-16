# Rust Things API Documentation

Welcome to the Rust Things API documentation. This comprehensive guide covers all aspects of the Things 3 integration library for Rust.

## Overview

The Rust Things library provides a high-performance, type-safe interface for interacting with Things 3 databases. It supports both CLI applications and MCP (Model Context Protocol) servers, with comprehensive caching, error handling, and serialization support.

## Quick Start

```rust
use things_core::{ThingsDatabase, ThingsConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create database configuration
    let config = ThingsConfig::new("/path/to/Things.sqlite", true)?;
    
    // Connect to database
    let db = ThingsDatabase::new(config).await?;
    
    // Get inbox tasks
    let tasks = db.get_inbox(Some(10)).await?;
    
    for task in tasks {
        println!("Task: {}", task.title);
    }
    
    Ok(())
}
```

## API Structure

### Core Modules

- **[Models](core/models.md)** - Data structures and types
- **[Database](core/database.md)** - Database operations and queries
- **[Cache](core/cache.md)** - Caching layer and performance
- **[Errors](core/errors.md)** - Error handling and recovery

### Examples

- **[Basic Usage](examples/basic-usage.md)** - Simple operations and queries
- **[Advanced Usage](examples/advanced-usage.md)** - Complex patterns and optimizations
- **[Integration](examples/integration.md)** - MCP server and CLI integration

### Reference

- **[Traits](reference/traits.md)** - Trait documentation
- **[Types](reference/types.md)** - Type definitions
- **[Functions](reference/functions.md)** - Function documentation

## Key Features

### 🚀 Performance
- Async/await throughout
- Multi-level caching (memory, disk, database)
- Connection pooling
- Batch operations
- Lazy loading

### 🛡️ Type Safety
- Strong typing with Rust's type system
- Newtype patterns for domain-specific types
- Enum-based state management
- Option types for optional data

### 🔧 Error Handling
- Comprehensive error types
- Rich error context
- Error recovery strategies
- User-friendly error messages

### 💾 Caching
- L1: Memory cache (Moka)
- L2: Disk cache (SQLite)
- L3: Database query cache
- Smart invalidation strategies

### 📦 Serialization
- Multiple formats (JSON, MessagePack, Bincode, CBOR, YAML, TOML)
- Version compatibility
- Custom serializers
- Performance optimization

## Architecture

The library follows a layered architecture:

```
┌─────────────────┐
│   Application   │ ← CLI, MCP Server
└─────────────────┘
         │
         ▼
┌─────────────────┐
│   Core Library  │ ← things-core
└─────────────────┘
         │
         ▼
┌─────────────────┐
│   Common Types  │ ← things-common
└─────────────────┘
```

## Getting Started

1. **Add to Cargo.toml**:
   ```toml
   [dependencies]
   things-core = "0.1.0"
   things-common = "0.1.0"
   ```

2. **Basic usage**:
   ```rust
   use things_core::{ThingsDatabase, ThingsConfig};
   
   let config = ThingsConfig::new("/path/to/Things.sqlite", true)?;
   let db = ThingsDatabase::new(config).await?;
   ```

3. **Advanced usage**:
   ```rust
   use things_core::{
       ThingsDatabase, ThingsConfig, CacheConfig, 
       SerializationConfig, PerformanceMonitor
   };
   
   let config = ThingsConfig::new("/path/to/Things.sqlite", true)?;
   let cache_config = CacheConfig::default();
   let serialization_config = SerializationConfig::default();
   
   let db = ThingsDatabase::new_with_config(
       config, 
       cache_config, 
       serialization_config
   ).await?;
   ```

## Examples

### Basic Task Operations

```rust
use things_core::{ThingsDatabase, CreateTaskRequest, TaskStatus};

// Create a new task
let create_request = CreateTaskRequest {
    title: "Learn Rust".to_string(),
    notes: Some("Study the Rust programming language".to_string()),
    start_date: Some(chrono::Utc::now().date_naive()),
    deadline: None,
    project_uuid: None,
    area_uuid: None,
    tags: vec!["learning".to_string()],
};

let task = db.create_task(&create_request).await?;
println!("Created task: {}", task.title);
```

### Advanced Querying

```rust
use things_core::{TaskFilters, TaskStatus, TaskType};

// Get tasks with filters
let filters = TaskFilters {
    status: Some(TaskStatus::Incomplete),
    task_type: Some(TaskType::Todo),
    tags: Some(vec!["urgent".to_string()]),
    start_date_from: Some(chrono::Utc::now().date_naive()),
    limit: Some(20),
    ..Default::default()
};

let tasks = db.get_tasks_filtered(&filters).await?;
```

### Caching

```rust
use things_core::{CacheConfig, EvictionPolicy};

// Configure caching
let cache_config = CacheConfig {
    max_size: 100 * 1024 * 1024, // 100MB
    ttl: Duration::from_secs(300), // 5 minutes
    tti: Duration::from_secs(60),  // 1 minute
    eviction_policy: EvictionPolicy::Lru,
    compression: true,
    statistics: true,
};

let db = ThingsDatabase::new_with_cache(config, cache_config).await?;
```

### Serialization

```rust
use things_core::{SerializationConfig, SerializationFormat};

// Configure serialization
let serialization_config = SerializationConfig {
    default_format: SerializationFormat::Json,
    compression: true,
    pretty_print: true,
    include_metadata: true,
    version: "1.0.0".to_string(),
    ..Default::default()
};

let db = ThingsDatabase::new_with_serialization(
    config, 
    cache_config, 
    serialization_config
).await?;
```

## Error Handling

The library provides comprehensive error handling:

```rust
use things_core::{ThingsError, ErrorCategory, ErrorSeverity};

match db.get_task(&task_uuid).await {
    Ok(task) => println!("Task: {}", task.title),
    Err(ThingsError::TaskNotFound { uuid }) => {
        println!("Task {} not found", uuid);
    }
    Err(ThingsError::Database(e)) => {
        println!("Database error: {}", e);
    }
    Err(e) => {
        println!("Error: {}", e);
        
        // Check error properties
        println!("Category: {:?}", e.category());
        println!("Severity: {:?}", e.severity());
        println!("Recoverable: {}", e.is_recoverable());
    }
}
```

## Performance Monitoring

```rust
use things_core::PerformanceMonitor;

// Get performance statistics
let stats = db.get_performance_stats().await?;
println!("Database operations: {}", stats.database_operations);
println!("Cache hit rate: {:.2}%", stats.cache_hit_rate * 100.0);
println!("Average query time: {:?}", stats.avg_query_time);
```

## Contributing

We welcome contributions! Please see our [Contributing Guide](../../CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](../../LICENSE) file for details.

## Support

- **Documentation**: [docs.rust-things.dev](https://docs.rust-things.dev)
- **Issues**: [GitHub Issues](https://github.com/GarthDB/rust-things/issues)
- **Discussions**: [GitHub Discussions](https://github.com/GarthDB/rust-things/discussions)
- **Discord**: [Rust Things Discord](https://discord.gg/rust-things)