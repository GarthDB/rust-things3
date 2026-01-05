# Error Handling Guide

Comprehensive guide to error handling in `things3-core`.

## Table of Contents

1. [Error Types](#error-types)
2. [Error Handling Patterns](#error-handling-patterns)
3. [Recovery Strategies](#recovery-strategies)
4. [Best Practices](#best-practices)
5. [Common Errors](#common-errors)

## Error Types

### ThingsError Enum

All operations return `Result<T, ThingsError>`. The error enum includes:

```rust
pub enum ThingsError {
    Database(String),
    Serialization(serde_json::Error),
    Io(std::io::Error),
    DatabaseNotFound { path: String },
    InvalidUuid { uuid: String },
    InvalidDate { date: String },
    DateValidation(DateValidationError),
    DateConversion(DateConversionError),
    TaskNotFound { uuid: String },
    ProjectNotFound { uuid: String },
    AreaNotFound { uuid: String },
    Validation { message: String },
    Configuration { message: String },
    Unknown { message: String },
}
```

### Error Sources

Errors implement `std::error::Error` and support the `source()` method:

```rust
if let Err(e) = db.create_task(request).await {
    if let Some(source) = e.source() {
        eprintln!("Underlying error: {}", source);
    }
}
```

## Error Handling Patterns

### Pattern 1: Match on Specific Errors

```rust
match db.get_task_by_uuid(&uuid).await {
    Ok(Some(task)) => println!("Found: {}", task.title),
    Ok(None) => println!("Task not found"),
    Err(ThingsError::TaskNotFound { uuid }) => {
        eprintln!("Task {} does not exist", uuid);
    }
    Err(ThingsError::DatabaseNotFound { path }) => {
        eprintln!("Database not found at: {}", path);
    }
    Err(e) => eprintln!("Unexpected error: {}", e),
}
```

### Pattern 2: Use `?` Operator with Context

```rust
use anyhow::{Context, Result};

fn create_task_with_context(db: &ThingsDatabase, title: &str) -> Result<Uuid> {
    let request = CreateTaskRequest {
        title: title.to_string(),
        // ... other fields
    };
    
    db.create_task(request)
        .await
        .context("Failed to create task")?;
    
    Ok(uuid)
}
```

### Pattern 3: Convert to Application Errors

```rust
enum AppError {
    TaskNotFound,
    DatabaseUnavailable,
    InvalidInput(String),
}

impl From<ThingsError> for AppError {
    fn from(err: ThingsError) -> Self {
        match err {
            ThingsError::TaskNotFound { .. } => AppError::TaskNotFound,
            ThingsError::DatabaseNotFound { .. } => AppError::DatabaseUnavailable,
            ThingsError::Validation { message } => AppError::InvalidInput(message),
            _ => AppError::InvalidInput(err.to_string()),
        }
    }
}
```

### Pattern 4: Retry Logic

```rust
use std::time::Duration;
use tokio::time::sleep;

async fn create_task_with_retry(
    db: &ThingsDatabase,
    request: CreateTaskRequest,
    max_retries: u32,
) -> Result<Uuid, ThingsError> {
    for attempt in 0..max_retries {
        match db.create_task(request.clone()).await {
            Ok(uuid) => return Ok(uuid),
            Err(ThingsError::Database(msg)) if attempt < max_retries - 1 => {
                eprintln!("Database error (attempt {}): {}, retrying...", attempt + 1, msg);
                sleep(Duration::from_millis(100 * (attempt + 1) as u64)).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Err(ThingsError::Unknown {
        message: "Max retries exceeded".to_string(),
    })
}
```

## Recovery Strategies

### Database Not Found

**Error:** `ThingsError::DatabaseNotFound { path }`

**Recovery:**
```rust
match db.get_inbox(None).await {
    Err(ThingsError::DatabaseNotFound { path }) => {
        eprintln!("Database not found at: {}", path);
        eprintln!("Please ensure Things 3 is installed and has been opened at least once.");
        // Optionally: prompt user for custom path
    }
    Ok(tasks) => println!("Found {} tasks", tasks.len()),
    Err(e) => eprintln!("Error: {}", e),
}
```

### Task Not Found

**Error:** `ThingsError::TaskNotFound { uuid }`

**Recovery:**
```rust
match db.get_task_by_uuid(&uuid).await {
    Ok(Some(task)) => println!("Found: {}", task.title),
    Ok(None) | Err(ThingsError::TaskNotFound { .. }) => {
        eprintln!("Task not found. It may have been deleted.");
        // Optionally: search for similar tasks
        let results = db.search_tasks(&partial_title).await?;
        if !results.is_empty() {
            println!("Did you mean one of these?");
            for task in results {
                println!("  - {}", task.title);
            }
        }
    }
    Err(e) => return Err(e),
}
```

### Date Validation Errors

**Error:** `ThingsError::DateValidation(DateValidationError)`

**Recovery:**
```rust
use things3_core::DateValidationError;

match db.create_task(request).await {
    Err(ThingsError::DateValidation(DateValidationError::DeadlineBeforeStartDate {
        start_date,
        deadline,
    })) => {
        eprintln!("Invalid date range: deadline ({}) is before start date ({})", 
                  deadline, start_date);
        // Suggest correction
        eprintln!("Suggestion: Set deadline to {} or later", start_date);
    }
    Err(ThingsError::DateValidation(e)) => {
        eprintln!("Date validation error: {}", e);
    }
    Ok(uuid) => println!("Created: {}", uuid),
    Err(e) => eprintln!("Error: {}", e),
}
```

### Invalid UUID

**Error:** `ThingsError::InvalidUuid { uuid }`

**Recovery:**
```rust
match db.get_task_by_uuid(&uuid).await {
    Err(ThingsError::InvalidUuid { uuid }) => {
        eprintln!("Invalid UUID format: {}", uuid);
        eprintln!("UUIDs should be in format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx");
        // Optionally: try to parse and fix
        if let Ok(parsed) = Uuid::parse_str(&uuid.trim()) {
            return db.get_task_by_uuid(&parsed).await;
        }
    }
    result => result,
}
```

### Validation Errors

**Error:** `ThingsError::Validation { message }`

**Recovery:**
```rust
match db.create_task(request).await {
    Err(ThingsError::Validation { message }) => {
        eprintln!("Validation failed: {}", message);
        // Parse message and provide specific guidance
        if message.contains("project") {
            eprintln!("The specified project does not exist or has been deleted.");
        } else if message.contains("area") {
            eprintln!("The specified area does not exist or has been deleted.");
        }
    }
    Ok(uuid) => println!("Created: {}", uuid),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Best Practices

### 1. Always Handle Errors Explicitly

```rust
// ✅ Good: Explicit error handling
match db.create_task(request).await {
    Ok(uuid) => println!("Created: {}", uuid),
    Err(e) => {
        eprintln!("Failed to create task: {}", e);
        // Handle error
    }
}

// ⚠️ Acceptable: Propagate with context
db.create_task(request)
    .await
    .context("Failed to create task")?;
```

### 2. Provide User-Friendly Messages

```rust
// ✅ Good: User-friendly message
match db.get_task_by_uuid(&uuid).await {
    Err(ThingsError::TaskNotFound { uuid }) => {
        eprintln!("Task not found. It may have been deleted or moved.");
    }
    // ...
}

// ❌ Bad: Raw error message
match db.get_task_by_uuid(&uuid).await {
    Err(e) => eprintln!("{}", e), // Too technical
}
```

### 3. Log Errors with Context

```rust
use tracing::{error, warn};

match db.create_task(request).await {
    Ok(uuid) => {
        info!("Created task: {}", uuid);
    }
    Err(ThingsError::Validation { message }) => {
        warn!("Validation failed: {}", message);
        // User-facing message
        eprintln!("Please check your input and try again.");
    }
    Err(e) => {
        error!("Unexpected error creating task: {}", e);
        eprintln!("An unexpected error occurred. Please try again later.");
    }
}
```

### 4. Use Error Types for Control Flow

```rust
// Check if task exists before updating
match db.get_task_by_uuid(&uuid).await {
    Ok(Some(_)) => {
        // Task exists, proceed with update
        db.update_task(update_request).await?;
    }
    Ok(None) | Err(ThingsError::TaskNotFound { .. }) => {
        // Task doesn't exist, create it instead
        db.create_task(create_request).await?;
    }
    Err(e) => return Err(e),
}
```

### 5. Validate Input Before Database Operations

```rust
// ✅ Good: Validate before database call
if request.deadline < request.start_date {
    return Err(ThingsError::DateValidation(
        DateValidationError::DeadlineBeforeStartDate {
            start_date: request.start_date,
            deadline: request.deadline,
        }
    ));
}

db.create_task(request).await?;

// ❌ Bad: Let database validate
db.create_task(request).await?; // May fail with unclear error
```

## Common Errors

### "Database not found"

**Cause:** Things 3 database file doesn't exist at the expected path.

**Solution:**
1. Ensure Things 3 is installed and has been opened at least once
2. Check the default path: `~/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite`
3. Specify a custom path if needed

### "Task not found"

**Cause:** The UUID doesn't exist in the database (may have been deleted).

**Solution:**
1. Verify the UUID is correct
2. Check if the task was deleted (soft delete sets `trashed = 1`)
3. Search for the task by title instead

### "Date validation failed: deadline before start date"

**Cause:** The deadline date is earlier than the start date.

**Solution:**
1. Ensure `deadline >= start_date`
2. Use `validate_date_range()` before creating/updating

### "Invalid UUID"

**Cause:** UUID string is not in the correct format.

**Solution:**
1. Ensure UUID is in format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
2. Use `Uuid::parse_str()` to validate before use

### "Project not found" / "Area not found"

**Cause:** Referenced project/area doesn't exist or has been deleted.

**Solution:**
1. Verify the UUID is correct
2. Check if the entity was deleted
3. List all projects/areas to find the correct UUID

## Error Recovery Examples

### Complete Example: Robust Task Creation

```rust
use things3_core::{ThingsDatabase, CreateTaskRequest, ThingsError, DateValidationError};
use chrono::NaiveDate;

async fn create_task_robust(
    db: &ThingsDatabase,
    title: &str,
    start_date: Option<NaiveDate>,
    deadline: Option<NaiveDate>,
    project_uuid: Option<Uuid>,
) -> Result<Uuid, ThingsError> {
    // Validate dates before database call
    if let (Some(start), Some(deadline)) = (start_date, deadline) {
        if deadline < start {
            return Err(ThingsError::DateValidation(
                DateValidationError::DeadlineBeforeStartDate { start_date: start, deadline }
            ));
        }
    }
    
    // Validate project exists if provided
    if let Some(uuid) = project_uuid {
        if db.get_project_by_uuid(&uuid).await?.is_none() {
            return Err(ThingsError::ProjectNotFound { uuid: uuid.to_string() });
        }
    }
    
    // Create task
    let request = CreateTaskRequest {
        title: title.to_string(),
        start_date,
        deadline,
        project_uuid,
        // ... other fields
    };
    
    match db.create_task(request).await {
        Ok(uuid) => Ok(uuid),
        Err(ThingsError::Validation { message }) => {
            eprintln!("Validation error: {}", message);
            Err(ThingsError::Validation { message })
        }
        Err(e) => {
            eprintln!("Unexpected error: {}", e);
            Err(e)
        }
    }
}
```

## Next Steps

- [User Guide](USER_GUIDE.md) - Comprehensive usage guide
- [API Documentation](https://docs.rs/things3-core) - Full API reference
- [Examples](../libs/things3-core/examples/) - Code examples

