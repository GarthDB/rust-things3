# Quick Start Guide

Get started with `things3-core` in under 5 minutes!

## Installation

Add `things3-core` to your `Cargo.toml`:

```toml
[dependencies]
things3-core = "0.2.0"
tokio = { version = "1", features = ["full"] }
```

## Basic Usage

### 1. Connect to the Database

```rust
use things3_core::{ThingsDatabase, ThingsError};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), ThingsError> {
    // Use the default database path
    let db_path = things3_core::get_default_database_path();
    
    // Or specify a custom path
    // let db_path = Path::new("/path/to/things.db");
    
    let db = ThingsDatabase::new(&db_path).await?;
    Ok(())
}
```

### 2. Get Inbox Tasks

```rust
let tasks = db.get_inbox(Some(10)).await?;
for task in tasks {
    println!("- {}", task.title);
}
```

### 3. Search for Tasks

```rust
let results = db.search_tasks("meeting").await?;
println!("Found {} matching tasks", results.len());
```

### 4. Create a Task

```rust
use things3_core::CreateTaskRequest;
use chrono::NaiveDate;

let request = CreateTaskRequest {
    title: "Buy groceries".to_string(),
    notes: Some("Milk, eggs, bread".to_string()),
    deadline: Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
    start_date: None,
    project_uuid: None,
    area_uuid: None,
    parent_uuid: None,
    tags: None,
    task_type: None,
    status: None,
};

let task_uuid = db.create_task(request).await?;
println!("Created task: {}", task_uuid);
```

### 5. Update a Task

```rust
use things3_core::UpdateTaskRequest;

let update = UpdateTaskRequest {
    uuid: task_uuid,
    title: Some("Buy groceries and cook dinner".to_string()),
    notes: None,
    start_date: None,
    deadline: None,
    project_uuid: None,
    area_uuid: None,
    tags: None,
    status: None,
};

db.update_task(update).await?;
```

## Common Operations

### Get Today's Tasks

```rust
let today = db.get_today(Some(5)).await?;
```

### Get All Projects

```rust
let projects = db.get_projects(None).await?;
```

### Get All Areas

```rust
let areas = db.get_areas().await?;
```

### Bulk Operations

```rust
use things3_core::{BulkCompleteRequest, BulkMoveRequest};
use uuid::Uuid;

// Complete multiple tasks
let complete = BulkCompleteRequest {
    task_uuids: vec![uuid1, uuid2, uuid3],
};
let result = db.bulk_complete(complete).await?;

// Move tasks to a project
let move_req = BulkMoveRequest {
    task_uuids: vec![uuid1, uuid2],
    project_uuid: Some(project_uuid),
    area_uuid: None,
};
let result = db.bulk_move(move_req).await?;
```

## Error Handling

All operations return `Result<T, ThingsError>`. Handle errors appropriately:

```rust
match db.get_inbox(None).await {
    Ok(tasks) => println!("Found {} tasks", tasks.len()),
    Err(ThingsError::DatabaseNotFound { path }) => {
        eprintln!("Database not found at: {}", path);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Next Steps

- Read the [User Guide](USER_GUIDE.md) for comprehensive documentation
- Check out [examples](../libs/things3-core/examples/) for more code samples
- See [API Documentation](https://docs.rs/things3-core) for full API reference

## Troubleshooting

### Database Not Found

If you get a "Database not found" error:

1. Make sure Things 3 is installed and has been opened at least once
2. Check the default path: `~/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite`
3. Or specify a custom path when creating the database connection

### Permission Errors

Ensure your application has the necessary permissions to read the Things 3 database file.

### Date Format Issues

Dates use `chrono::NaiveDate`. Make sure to use valid dates:

```rust
use chrono::NaiveDate;

// Valid date
let date = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

// Invalid date will panic - use from_ymd_opt instead
```

## Examples

See the [examples directory](../libs/things3-core/examples/) for complete working examples:

- `basic_usage.rs` - Basic operations
- `bulk_operations.rs` - Bulk operations

Run an example:

```bash
cargo run --example basic_usage
```

