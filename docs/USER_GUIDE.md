# User Guide

Comprehensive guide to using `things3-core` for Things 3 integration.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Database Operations](#database-operations)
3. [Task Management](#task-management)
4. [Project and Area Management](#project-and-area-management)
5. [Tag Management](#tag-management)
6. [Bulk Operations](#bulk-operations)
7. [Search and Filtering](#search-and-filtering)
8. [Error Handling](#error-handling)
9. [Performance Tips](#performance-tips)
10. [Best Practices](#best-practices)

## Getting Started

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
things3-core = "0.2.0"
tokio = { version = "1", features = ["full"] }
chrono = "0.4"
uuid = { version = "1.0", features = ["v4"] }
```

### Basic Setup

```rust
use things3_core::{ThingsDatabase, ThingsError};

#[tokio::main]
async fn main() -> Result<(), ThingsError> {
    let db_path = things3_core::get_default_database_path();
    let db = ThingsDatabase::new(&db_path).await?;
    
    // Your code here
    
    Ok(())
}
```

## Database Operations

### Connection Options

**Default Connection:**
```rust
let db = ThingsDatabase::new(&db_path).await?;
```

**Custom Configuration:**
```rust
use things3_core::DatabasePoolConfig;
use std::time::Duration;

let config = DatabasePoolConfig {
    max_connections: 10,
    min_connections: 2,
    connect_timeout: Duration::from_secs(5),
    idle_timeout: Duration::from_secs(300),
    max_lifetime: Duration::from_secs(3600),
    test_before_acquire: true,
    sqlite_optimizations: Default::default(),
};

let db = ThingsDatabase::new_with_config(&db_path, config).await?;
```

### Health Checks

```rust
// Quick health check
let is_healthy = db.is_connected().await;

// Comprehensive health check
let health = db.comprehensive_health_check().await?;
println!("Pool healthy: {}", health.overall_healthy);
println!("Task count: {}", health.database_stats.task_count);
```

### Database Statistics

```rust
let stats = db.get_stats().await?;
println!("Tasks: {}", stats.task_count);
println!("Projects: {}", stats.project_count);
println!("Areas: {}", stats.area_count);
```

## Task Management

### Creating Tasks

**Basic Task:**
```rust
use things3_core::CreateTaskRequest;
use chrono::NaiveDate;

let request = CreateTaskRequest {
    title: "Complete project".to_string(),
    notes: None,
    deadline: None,
    start_date: None,
    project_uuid: None,
    area_uuid: None,
    parent_uuid: None,
    tags: None,
    task_type: None,
    status: None,
};

let uuid = db.create_task(request).await?;
```

**Task with Dates:**
```rust
let request = CreateTaskRequest {
    title: "Review proposal".to_string(),
    start_date: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
    deadline: Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()),
    // ... other fields
    ..Default::default()
};
```

**Task in Project:**
```rust
let request = CreateTaskRequest {
    title: "Subtask".to_string(),
    project_uuid: Some(project_uuid),
    // ... other fields
};
```

**Subtask:**
```rust
let request = CreateTaskRequest {
    title: "Subtask".to_string(),
    parent_uuid: Some(parent_task_uuid),
    // ... other fields
};
```

### Updating Tasks

```rust
use things3_core::UpdateTaskRequest;

let update = UpdateTaskRequest {
    uuid: task_uuid,
    title: Some("Updated title".to_string()),
    notes: Some("Updated notes".to_string()),
    deadline: Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
    status: Some(things3_core::TaskStatus::Completed),
    // ... other fields
};

db.update_task(update).await?;
```

### Completing Tasks

```rust
db.complete_task(&task_uuid).await?;
```

### Uncompleting Tasks

```rust
db.uncomplete_task(&task_uuid).await?;
```

### Deleting Tasks

```rust
use things3_core::DeleteChildHandling;

// Soft delete (default)
db.delete_task(&task_uuid, DeleteChildHandling::Error).await?;

// Cascade delete children
db.delete_task(&task_uuid, DeleteChildHandling::Cascade).await?;

// Orphan children
db.delete_task(&task_uuid, DeleteChildHandling::Orphan).await?;
```

### Getting Tasks

**By UUID:**
```rust
if let Some(task) = db.get_task_by_uuid(&uuid).await? {
    println!("Found: {}", task.title);
}
```

**By Status:**
```rust
let completed = db.get_tasks_by_status(things3_core::TaskStatus::Completed).await?;
```

**All Tasks:**
```rust
let all_tasks = db.get_all_tasks().await?;
```

## Project and Area Management

### Creating Projects

```rust
use things3_core::CreateProjectRequest;

let request = CreateProjectRequest {
    title: "New Project".to_string(),
    area_uuid: Some(area_uuid),
    notes: None,
    start_date: None,
    deadline: None,
};

let project_uuid = db.create_project(request).await?;
```

### Updating Projects

```rust
use things3_core::UpdateProjectRequest;

let update = UpdateProjectRequest {
    uuid: project_uuid,
    title: Some("Updated Project".to_string()),
    // ... other fields
};

db.update_project(update).await?;
```

### Completing Projects

```rust
use things3_core::ProjectChildHandling;

// Complete project and all children
db.complete_project(&project_uuid, ProjectChildHandling::Cascade).await?;
```

### Creating Areas

```rust
use things3_core::CreateAreaRequest;

let request = CreateAreaRequest {
    title: "New Area".to_string(),
};

let area_uuid = db.create_area(request).await?;
```

## Tag Management

### Creating Tags

```rust
use things3_core::CreateTagRequest;

// Smart creation (prevents duplicates)
let request = CreateTagRequest {
    title: "work".to_string(),
};

match db.create_tag_smart(request).await? {
    things3_core::TagCreationResult::Created(tag) => {
        println!("Created tag: {}", tag.title);
    }
    things3_core::TagCreationResult::Existing(tag) => {
        println!("Tag already exists: {}", tag.title);
    }
    things3_core::TagCreationResult::SimilarFound { tag, suggestions } => {
        println!("Similar tag found: {}", tag.title);
        println!("Suggestions: {:?}", suggestions);
    }
}
```

### Searching Tags

```rust
let tags = db.search_tags("work").await?;
```

### Adding Tags to Tasks

```rust
db.add_tag_to_task(&task_uuid, &tag_uuid).await?;
```

### Setting Task Tags

```rust
let tag_uuids = vec![tag1_uuid, tag2_uuid];
db.set_task_tags(&task_uuid, &tag_uuids).await?;
```

## Bulk Operations

All bulk operations are transactional - either all succeed or all fail.

### Bulk Move

```rust
use things3_core::BulkMoveRequest;

let request = BulkMoveRequest {
    task_uuids: vec![uuid1, uuid2, uuid3],
    project_uuid: Some(project_uuid),
    area_uuid: None,
};

let result = db.bulk_move(request).await?;
println!("Processed: {}", result.processed_count);
```

### Bulk Update Dates

```rust
use things3_core::BulkUpdateDatesRequest;

let request = BulkUpdateDatesRequest {
    task_uuids: vec![uuid1, uuid2],
    start_date: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
    deadline: Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
    clear_start_date: false,
    clear_deadline: false,
};

let result = db.bulk_update_dates(request).await?;
```

### Bulk Complete

```rust
use things3_core::BulkCompleteRequest;

let request = BulkCompleteRequest {
    task_uuids: vec![uuid1, uuid2, uuid3],
};

let result = db.bulk_complete(request).await?;
```

### Bulk Delete

```rust
use things3_core::BulkDeleteRequest;

let request = BulkDeleteRequest {
    task_uuids: vec![uuid1, uuid2],
};

let result = db.bulk_delete(request).await?;
```

**Note:** Maximum batch size is 1000 tasks per operation.

## Search and Filtering

### Text Search

```rust
let results = db.search_tasks("meeting").await?;
```

### Logbook Search

```rust
let logbook = db.search_logbook(
    Some("meeting"),           // text query
    None,                      // project_uuid filter
    None,                      // area_uuid filter
    None,                      // tag filter
    None,                      // from_date
    None,                      // to_date
    Some(10),                  // limit
).await?;
```

### Filtering by Status

```rust
let incomplete = db.get_tasks_by_status(things3_core::TaskStatus::Incomplete).await?;
```

## Error Handling

### Error Types

```rust
use things3_core::ThingsError;

match db.get_task_by_uuid(&uuid).await {
    Ok(Some(task)) => println!("Found: {}", task.title),
    Ok(None) => println!("Task not found"),
    Err(ThingsError::TaskNotFound { uuid }) => {
        eprintln!("Task {} not found", uuid);
    }
    Err(ThingsError::DatabaseNotFound { path }) => {
        eprintln!("Database not found: {}", path);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

### Date Validation Errors

```rust
match db.create_task(request).await {
    Err(ThingsError::DateValidation(e)) => {
        eprintln!("Date validation failed: {}", e);
        // Handle invalid date range
    }
    Ok(uuid) => println!("Created: {}", uuid),
    Err(e) => eprintln!("Error: {}", e),
}
```

See [Error Handling Guide](ERROR_HANDLING.md) for more details.

## Performance Tips

### Connection Pooling

Use connection pooling for multiple concurrent operations:

```rust
let db = Arc::new(db);

// Spawn multiple tasks
let db1 = db.clone();
let db2 = db.clone();

tokio::spawn(async move {
    db1.get_inbox(None).await
});

tokio::spawn(async move {
    db2.get_today(None).await
});
```

### Batch Operations

Use bulk operations instead of individual operations:

```rust
// ❌ Slow: Individual operations
for uuid in task_uuids {
    db.complete_task(&uuid).await?;
}

// ✅ Fast: Bulk operation
let request = BulkCompleteRequest { task_uuids };
db.bulk_complete(request).await?;
```

### Limit Results

Always use limits for potentially large result sets:

```rust
// ✅ Good
let tasks = db.get_inbox(Some(100)).await?;

// ⚠️ May be slow
let tasks = db.get_inbox(None).await?;
```

## Best Practices

### 1. Always Handle Errors

```rust
match db.create_task(request).await {
    Ok(uuid) => println!("Success: {}", uuid),
    Err(e) => {
        eprintln!("Failed to create task: {}", e);
        // Handle error appropriately
    }
}
```

### 2. Validate Dates

```rust
use things3_core::validate_date_range;

if let Err(e) = validate_date_range(start_date, deadline) {
    eprintln!("Invalid date range: {}", e);
    return;
}
```

### 3. Use Appropriate Types

```rust
// ✅ Good: Use TaskStatus enum
let status = TaskStatus::Completed;

// ❌ Bad: Use raw integer
let status = 1;
```

### 4. Check Entity Existence

```rust
// Before creating a task in a project
if db.get_project_by_uuid(&project_uuid).await?.is_none() {
    return Err("Project not found".into());
}
```

### 5. Use Transactions for Related Operations

Bulk operations are automatically transactional. For custom operations, consider grouping related changes.

## Next Steps

- [API Documentation](https://docs.rs/things3-core)
- [Error Handling Guide](ERROR_HANDLING.md)
- [Architecture Documentation](ARCHITECTURE.md)
- [Examples](../libs/things3-core/examples/)

