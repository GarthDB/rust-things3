# API Reference

This section contains comprehensive API documentation for the Rust Things library.

## Table of Contents

- [Core Library](./core.md) - Main database and data access APIs
- [CLI Reference](./cli.md) - Command-line interface documentation
- [MCP Tools](./mcp.md) - Model Context Protocol tools reference
- [Performance](./performance.md) - Performance monitoring and metrics
- [Caching](./caching.md) - Caching layer documentation
- [Export](./export.md) - Data export functionality
- [Backup](./backup.md) - Backup and restore operations

## Quick Start

```rust
use things_core::{ThingsDatabase, ThingsConfig};

// Create database connection
let config = ThingsConfig::from_env();
let db = ThingsDatabase::new(&config)?;

// Get inbox tasks
let tasks = db.get_inbox(Some(10)).await?;

// Get today's tasks
let today_tasks = db.get_today(None).await?;

// Search tasks
let search_results = db.search_tasks("meeting", Some(5)).await?;
```

## Data Models

### Task
```rust
pub struct Task {
    pub uuid: Uuid,
    pub title: String,
    pub notes: Option<String>,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub start_date: Option<NaiveDate>,
    pub deadline: Option<NaiveDate>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub project_uuid: Option<Uuid>,
    pub area_uuid: Option<Uuid>,
    pub parent_uuid: Option<Uuid>,
    pub tags: Vec<String>,
}
```

### Project
```rust
pub struct Project {
    pub uuid: Uuid,
    pub title: String,
    pub notes: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub deadline: Option<NaiveDate>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub area_uuid: Option<Uuid>,
    pub tags: Vec<String>,
    pub status: TaskStatus,
    pub tasks: Vec<Task>,
}
```

### Area
```rust
pub struct Area {
    pub uuid: Uuid,
    pub title: String,
    pub notes: Option<String>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}
```

## Error Handling

All operations return `Result<T, ThingsError>` where `ThingsError` can be:

- `DatabaseError` - Database connection or query errors
- `ConfigError` - Configuration-related errors
- `ValidationError` - Data validation errors
- `IoError` - File I/O errors

```rust
use things_core::{Result, ThingsError};

match db.get_inbox(None).await {
    Ok(tasks) => println!("Found {} tasks", tasks.len()),
    Err(ThingsError::DatabaseError(e)) => eprintln!("Database error: {}", e),
    Err(ThingsError::ConfigError(e)) => eprintln!("Config error: {}", e),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Async Operations

All database operations are async and return `Future` types. Use `.await` to wait for completion:

```rust
// Async operations
let tasks = db.get_inbox(Some(10)).await?;
let projects = db.get_projects(None).await?;
let areas = db.get_areas().await?;
```

## Configuration

The library uses environment variables for configuration:

```rust
// Set custom database path
std::env::set_var("THINGS_DB_PATH", "/path/to/things.db");

// Enable fallback to default path
std::env::set_var("THINGS_FALLBACK_TO_DEFAULT", "true");

// Create config
let config = ThingsConfig::from_env();
```

## Performance Considerations

- Use caching for frequently accessed data
- Limit result sets with appropriate limits
- Use async operations to avoid blocking
- Monitor performance with built-in metrics

See [Performance](./performance.md) for detailed information.
