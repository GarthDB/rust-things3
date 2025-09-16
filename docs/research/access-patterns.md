# Things 3 Database Access Patterns

This document defines the recommended access patterns for interacting with the Things 3 database, based on the schema analysis and performance considerations.

## Core Access Patterns

### 1. Task Retrieval Patterns

#### Get Inbox Tasks
```sql
-- Get all incomplete tasks in the inbox (no area, project, or heading)
SELECT uuid, title, notes, type, status, startDate, deadline, 
       creationDate, userModificationDate, project, area, heading
FROM TMTask 
WHERE status = 0 
  AND area IS NULL 
  AND project IS NULL 
  AND heading IS NULL
ORDER BY creationDate DESC
LIMIT ?;
```

**Use Cases:**
- Display inbox in UI
- MCP `get_inbox` tool
- Task management workflows

#### Get Today's Tasks
```sql
-- Get tasks scheduled for today
SELECT uuid, title, notes, type, status, startDate, deadline,
       creationDate, userModificationDate, project, area, heading
FROM TMTask 
WHERE status = 0 
  AND startDate = ?
ORDER BY todayIndex;
```

**Use Cases:**
- Today view in UI
- MCP `get_today` tool
- Daily planning workflows

#### Get Tasks by Area
```sql
-- Get all tasks in a specific area
SELECT uuid, title, notes, type, status, startDate, deadline,
       creationDate, userModificationDate, project, area, heading
FROM TMTask 
WHERE area = ? 
  AND status = 0
ORDER BY creationDate DESC;
```

**Use Cases:**
- Area-specific task views
- MCP `get_projects` tool (filtered by area)
- Organizational workflows

#### Get Projects
```sql
-- Get all projects (type = 1)
SELECT uuid, title, notes, startDate, deadline, creationDate, 
       userModificationDate, area, status
FROM TMTask 
WHERE type = 1 
  AND area = ?  -- Optional area filter
ORDER BY creationDate DESC;
```

**Use Cases:**
- Project management views
- MCP `get_projects` tool
- Project planning workflows

#### Get Tasks by Project
```sql
-- Get all tasks in a specific project
SELECT uuid, title, notes, type, status, startDate, deadline,
       creationDate, userModificationDate, project, area, heading
FROM TMTask 
WHERE project = ? 
  AND status = 0
ORDER BY creationDate DESC;
```

**Use Cases:**
- Project detail views
- Task organization
- Project progress tracking

### 2. Search Patterns

#### Text Search
```sql
-- Search tasks by title or notes
SELECT uuid, title, notes, type, status, startDate, deadline,
       creationDate, userModificationDate, project, area, heading
FROM TMTask 
WHERE (title LIKE ? OR notes LIKE ?) 
  AND status = 0
ORDER BY creationDate DESC
LIMIT ?;
```

**Search Strategies:**
- Use `%query%` for partial matches
- Use `query%` for prefix matches
- Use `%query` for suffix matches
- Consider full-text search for complex queries

#### Tag-based Search
```sql
-- Search tasks by tag
SELECT t.uuid, t.title, t.notes, t.type, t.status, t.startDate, t.deadline,
       t.creationDate, t.userModificationDate, t.project, t.area, t.heading
FROM TMTask t
JOIN TMTaskTag tt ON t.uuid = tt.task
WHERE tt.tag = ? 
  AND t.status = 0
ORDER BY t.creationDate DESC;
```

**Use Cases:**
- Tag-based filtering
- Categorization workflows
- Advanced search features

### 3. Area and Tag Patterns

#### Get All Areas
```sql
-- Get all visible areas
SELECT uuid, title, visible, index
FROM TMArea 
WHERE visible = 1 
ORDER BY index;
```

**Use Cases:**
- Area selection UI
- MCP `get_areas` tool
- Navigation menus

#### Get All Tags
```sql
-- Get all tags
SELECT uuid, title, shortcut, usedDate, parent, index
FROM TMTag 
ORDER BY index;
```

**Use Cases:**
- Tag selection UI
- Tag management
- Filtering options

#### Get Tags for Task
```sql
-- Get all tags associated with a task
SELECT t.uuid, t.title, t.shortcut, t.usedDate, t.parent, t.index
FROM TMTag t
JOIN TMTaskTag tt ON t.uuid = tt.tag
WHERE tt.task = ?
ORDER BY t.index;
```

**Use Cases:**
- Task detail views
- Tag display
- Tag management

### 4. Checklist Patterns

#### Get Checklist Items
```sql
-- Get all checklist items for a task
SELECT uuid, title, status, creationDate, userModificationDate, index
FROM TMChecklistItem 
WHERE task = ?
ORDER BY index;
```

**Use Cases:**
- Task detail views
- Checklist management
- Progress tracking

#### Update Checklist Item
```sql
-- Update checklist item status
UPDATE TMChecklistItem 
SET status = ?, userModificationDate = ?
WHERE uuid = ?;
```

**Use Cases:**
- Checklist interactions
- Task completion
- Progress updates

## Advanced Access Patterns

### 1. Hierarchical Queries

#### Get Project Hierarchy
```sql
-- Get project with all its tasks and headings
WITH RECURSIVE project_hierarchy AS (
  -- Base case: the project itself
  SELECT uuid, title, notes, type, status, startDate, deadline,
         creationDate, userModificationDate, project, area, heading, 0 as level
  FROM TMTask 
  WHERE uuid = ?
  
  UNION ALL
  
  -- Recursive case: tasks and headings under the project
  SELECT t.uuid, t.title, t.notes, t.type, t.status, t.startDate, t.deadline,
         t.creationDate, t.userModificationDate, t.project, t.area, t.heading, ph.level + 1
  FROM TMTask t
  JOIN project_hierarchy ph ON t.project = ph.uuid OR t.heading = ph.uuid
  WHERE t.status = 0
)
SELECT * FROM project_hierarchy ORDER BY level, creationDate;
```

**Use Cases:**
- Project detail views
- Hierarchical task display
- Project structure analysis

### 2. Performance Queries

#### Get Recent Tasks
```sql
-- Get recently modified tasks
SELECT uuid, title, notes, type, status, startDate, deadline,
       creationDate, userModificationDate, project, area, heading
FROM TMTask 
WHERE status = 0 
  AND userModificationDate > ?
ORDER BY userModificationDate DESC
LIMIT ?;
```

**Use Cases:**
- Recent activity views
- MCP `get_recent_tasks` tool
- Change tracking

#### Get Completed Tasks
```sql
-- Get completed tasks in date range
SELECT uuid, title, notes, type, status, startDate, deadline,
       creationDate, userModificationDate, project, area, heading, stopDate
FROM TMTask 
WHERE status = 1 
  AND stopDate >= ? 
  AND stopDate <= ?
ORDER BY stopDate DESC;
```

**Use Cases:**
- Productivity metrics
- MCP `get_productivity_metrics` tool
- Progress tracking

### 3. Bulk Operations

#### Bulk Task Creation
```sql
-- Insert multiple tasks in a single transaction
BEGIN TRANSACTION;
INSERT INTO TMTask (uuid, title, notes, type, status, creationDate, userModificationDate, area, project)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);
-- Repeat for each task
COMMIT;
```

**Use Cases:**
- MCP `bulk_create_tasks` tool
- Data import
- Batch operations

#### Bulk Status Updates
```sql
-- Update multiple tasks' status
UPDATE TMTask 
SET status = ?, userModificationDate = ?
WHERE uuid IN (?, ?, ?, ...);
```

**Use Cases:**
- Batch task completion
- Status changes
- Bulk operations

## Query Optimization Strategies

### 1. Index Usage

#### Leverage Existing Indexes
- Use `area` column for area-based queries
- Use `project` column for project-based queries
- Use `heading` column for heading-based queries
- Use `stopDate` column for completed task queries

#### Compound Queries
```sql
-- Efficient compound query using indexes
SELECT * FROM TMTask 
WHERE area = ? 
  AND status = 0 
  AND startDate = ?
ORDER BY todayIndex;
```

### 2. Pagination

#### Cursor-based Pagination
```sql
-- Get next page using cursor
SELECT * FROM TMTask 
WHERE status = 0 
  AND creationDate < ?  -- Cursor value
ORDER BY creationDate DESC
LIMIT ?;
```

#### Offset-based Pagination
```sql
-- Get page using offset
SELECT * FROM TMTask 
WHERE status = 0 
ORDER BY creationDate DESC
LIMIT ? OFFSET ?;
```

### 3. Caching Strategies

#### Cache Frequently Accessed Data
- Areas (rarely change)
- Tags (moderate change frequency)
- Project structures (moderate change frequency)
- User settings (rarely change)

#### Cache Invalidation
- Invalidate on task creation/update/deletion
- Invalidate on area/tag changes
- Use TTL for time-based invalidation

## Error Handling Patterns

### 1. Database Connection Errors
```rust
// Retry with exponential backoff
async fn execute_with_retry<F, T>(operation: F) -> Result<T>
where
    F: Fn() -> Result<T>,
{
    let mut retries = 3;
    let mut delay = Duration::from_millis(100);
    
    loop {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) if retries > 0 => {
                retries -= 1;
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 2. Data Validation
```rust
// Validate before database operations
fn validate_task(task: &Task) -> Result<()> {
    if task.title.is_empty() {
        return Err(ThingsError::ValidationError("Title cannot be empty".to_string()));
    }
    
    if let Some(deadline) = task.deadline {
        if let Some(start_date) = task.start_date {
            if deadline < start_date {
                return Err(ThingsError::ValidationError("Deadline cannot be before start date".to_string()));
            }
        }
    }
    
    Ok(())
}
```

### 3. Transaction Management
```rust
// Use transactions for related operations
async fn create_task_with_tags(
    db: &ThingsDatabase,
    task: &Task,
    tag_uuids: &[Uuid],
) -> Result<()> {
    let tx = db.begin_transaction().await?;
    
    // Create task
    db.create_task_in_transaction(&tx, task).await?;
    
    // Add tags
    for tag_uuid in tag_uuids {
        db.add_task_tag_in_transaction(&tx, &task.uuid, tag_uuid).await?;
    }
    
    tx.commit().await?;
    Ok(())
}
```

## Performance Monitoring

### 1. Query Timing
```rust
// Time database operations
async fn timed_query<F, T>(name: &str, query: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    let start = Instant::now();
    let result = query.await;
    let duration = start.elapsed();
    
    // Log slow queries
    if duration > Duration::from_millis(100) {
        log::warn!("Slow query '{}' took {:?}", name, duration);
    }
    
    result
}
```

### 2. Connection Pooling
```rust
// Use connection pooling for high concurrency
struct DatabasePool {
    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
}

impl DatabasePool {
    async fn execute<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let conn = self.pool.get()?;
        operation(&conn)
    }
}
```

These access patterns provide a comprehensive foundation for implementing efficient and reliable database operations in the Rust Things library.
