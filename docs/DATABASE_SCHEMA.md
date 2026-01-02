# Things 3 Database Schema

## Table of Contents
- [Overview](#overview)
- [Database Location](#database-location)
- [Core Tables](#core-tables)
- [Data Types and Conversions](#data-types-and-conversions)
- [Query Patterns](#query-patterns)
- [Schema Compatibility](#schema-compatibility)

## Overview

Things 3 uses SQLite as its database engine. The database is read-only for external tools to prevent corruption. This document describes the schema structure and how `rust-things3` interacts with it.

### Database Files

- `main.sqlite` - Primary database
- `main.sqlite-shm` - Shared memory file
- `main.sqlite-wal` - Write-ahead log
- `main.sqlite.temporary-shm` - Temporary shared memory

## Database Location

### macOS Standard Path

```
~/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-XXXXX/Things Database.thingsdatabase/main.sqlite
```

**Path Components**:
- `JLMPQHK86H.com.culturedcode.ThingsMac` - App container ID
- `ThingsData-XXXXX` - Data directory (varies: `0Z0Z2`, `01AEF`, etc.)
- `Things Database.thingsdatabase` - Database package
- `main.sqlite` - SQLite database file

### Finding the Database

```rust
use things3_core::utils::get_default_database_path;

// Automatically finds the database
let db_path = get_default_database_path();
```

## Core Tables

### TMTask (Tasks)

The main table for tasks, projects, and headings.

```sql
CREATE TABLE TMTask (
    uuid                              TEXT PRIMARY KEY,
    leavesTombstone                   INTEGER,
    
    creationDate                      REAL,
    userModificationDate              REAL,
    
    type                              INTEGER,
    status                            INTEGER,
    stopDate                          REAL,
    trashed                           INTEGER,
    
    title                             TEXT,
    notes                             TEXT,
    notesSync                         INTEGER,
    
    cachedTags                        BLOB,
    
    start                             INTEGER,
    startDate                         INTEGER,
    startBucket                       INTEGER,
    reminderTime                      INTEGER,
    lastReminderInteractionDate       REAL,
    
    deadline                          INTEGER,
    deadlineSuppressionDate           INTEGER,
    t2_deadlineOffset                 INTEGER,
    
    index                             INTEGER,
    todayIndex                        INTEGER,
    todayIndexReferenceDate           INTEGER,
    
    area                              TEXT,
    project                           TEXT,
    heading                           TEXT,
    contact                           TEXT,
    
    untrashedLeafActionsCount         INTEGER,
    openUntrashedLeafActionsCount     INTEGER,
    
    checklistItemsCount               INTEGER,
    openChecklistItemsCount           INTEGER,
    
    rt1_repeatingTemplate             TEXT,
    rt1_recurrenceRule                BLOB,
    rt1_instanceCreationStartDate     INTEGER,
    rt1_instanceCreationPaused        INTEGER,
    rt1_instanceCreationCount         INTEGER,
    rt1_afterCompletionReferenceDate  INTEGER,
    rt1_nextInstanceStartDate         INTEGER,
    
    experimental                      BLOB,
    repeater                          BLOB,
    repeaterMigrationDate             REAL
);
```

**Key Fields**:
- `uuid`: Unique identifier (TEXT)
- `type`: 0 = Task, 1 = Project, 2 = Heading
- `status`: 0 = Incomplete, 3 = Completed, 2 = Canceled
- `trashed`: 0 = Active, 1 = Trashed
- `title`: Task/project title
- `notes`: Task notes (Markdown)
- `startDate`: Start date (seconds since 2001-01-01)
- `deadline`: Due date (seconds since 2001-01-01)
- `todayIndex`: Position in Today list (-1 = not in Today)
- `area`: Area UUID (foreign key to TMArea)
- `project`: Project UUID (foreign key to TMTask where type=1)

**Indexes**:
```sql
CREATE INDEX index_TMTask_stopDate ON TMTask(stopDate);
CREATE INDEX index_TMTask_project ON TMTask(project);
CREATE INDEX index_TMTask_heading ON TMTask(heading);
CREATE INDEX index_TMTask_area ON TMTask(area);
CREATE INDEX index_TMTask_repeatingTemplate ON TMTask(rt1_repeatingTemplate);
```

### TMArea (Areas)

Areas for organizing projects and tasks.

```sql
CREATE TABLE TMArea (
    uuid                    TEXT PRIMARY KEY,
    leavesTombstone         INTEGER,
    
    creationDate            REAL,
    userModificationDate    REAL,
    
    title                   TEXT,
    visible                 INTEGER,
    index                   INTEGER,
    
    cachedTags              BLOB
);
```

**Key Fields**:
- `uuid`: Unique identifier
- `title`: Area name
- `visible`: 1 = Visible, 0 = Hidden
- `index`: Sort order

### TMTag (Tags)

Tags for categorizing tasks.

```sql
CREATE TABLE TMTag (
    uuid                    TEXT PRIMARY KEY,
    leavesTombstone         INTEGER,
    
    creationDate            REAL,
    userModificationDate    REAL,
    
    title                   TEXT,
    shortcut                TEXT,
    usedDate                REAL,
    parent                  TEXT,
    index                   INTEGER
);
```

**Key Fields**:
- `uuid`: Unique identifier
- `title`: Tag name
- `shortcut`: Keyboard shortcut
- `parent`: Parent tag UUID (for nested tags)

### TMTaskTag (Task-Tag Relationships)

Many-to-many relationship between tasks and tags.

```sql
CREATE TABLE TMTaskTag (
    tasks                   TEXT,
    tags                    TEXT,
    
    PRIMARY KEY (tasks, tags)
);
```

### TMChecklistItem (Checklist Items)

Checklist items within tasks.

```sql
CREATE TABLE TMChecklistItem (
    uuid                    TEXT PRIMARY KEY,
    leavesTombstone         INTEGER,
    
    creationDate            REAL,
    userModificationDate    REAL,
    
    status                  INTEGER,
    stopDate                REAL,
    
    title                   TEXT,
    task                    TEXT,
    index                   INTEGER
);
```

**Key Fields**:
- `uuid`: Unique identifier
- `title`: Checklist item text
- `status`: 0 = Incomplete, 3 = Completed
- `task`: Parent task UUID
- `index`: Sort order

### TMContact (Contacts)

Contacts assigned to tasks (delegates).

```sql
CREATE TABLE TMContact (
    uuid                    TEXT PRIMARY KEY,
    leavesTombstone         INTEGER,
    
    creationDate            REAL,
    userModificationDate    REAL,
    
    name                    TEXT,
    emailAddress            TEXT,
    usedDate                REAL
);
```

### Meta Tables

**TMSettings**: User preferences and settings
**TMMetaItem**: Metadata for various entities
**BSSyncronyMetadata**: Sync metadata
**TMTombstone**: Deleted items tracking

## Data Types and Conversions

### Date/Time Format

Things 3 uses a custom epoch: **2001-01-01 00:00:00 UTC**

**Conversion Formula**:
```
Unix Timestamp = Things3 Timestamp + 978307200
```

**Implementation**:
```rust
const THINGS_EPOCH: i64 = 978307200; // 2001-01-01 in Unix time

pub fn from_things_date(seconds: i64) -> Option<NaiveDate> {
    if seconds == 0 {
        return None;
    }
    let unix_timestamp = THINGS_EPOCH + seconds;
    NaiveDateTime::from_timestamp_opt(unix_timestamp, 0)
        .map(|dt| dt.date())
}

pub fn to_things_date(date: NaiveDate) -> i64 {
    let datetime = date.and_hms_opt(0, 0, 0).unwrap();
    datetime.timestamp() - THINGS_EPOCH
}
```

### Status Codes

**Task Status**:
- `0` - Incomplete (active)
- `2` - Canceled
- `3` - Completed

**Task Type**:
- `0` - Task (action item)
- `1` - Project (container for tasks)
- `2` - Heading (group within project)

### Boolean Fields

SQLite uses INTEGER for booleans:
- `0` = false
- `1` = true

### BLOB Fields

**cachedTags**: Binary-encoded tag list
**rt1_recurrenceRule**: Recurrence rule data
**experimental**: Experimental features data

## Query Patterns

### Get Inbox Tasks

```sql
SELECT * FROM TMTask
WHERE type = 0
  AND status = 0
  AND trashed = 0
  AND start = 0
  AND project IS NULL
  AND area IS NULL
ORDER BY index ASC
LIMIT ?;
```

### Get Today Tasks

```sql
SELECT * FROM TMTask
WHERE type = 0
  AND status = 0
  AND trashed = 0
  AND todayIndex >= 0
ORDER BY todayIndex ASC
LIMIT ?;
```

### Get Projects

```sql
SELECT * FROM TMTask
WHERE type = 1
  AND status = 0
  AND trashed = 0
ORDER BY index ASC
LIMIT ?;
```

### Get Projects by Area

```sql
SELECT * FROM TMTask
WHERE type = 1
  AND status = 0
  AND trashed = 0
  AND area = ?
ORDER BY index ASC
LIMIT ?;
```

### Search Tasks

```sql
SELECT * FROM TMTask
WHERE type = 0
  AND status = 0
  AND trashed = 0
  AND (title LIKE ? OR notes LIKE ?)
ORDER BY userModificationDate DESC
LIMIT ?;
```

### Get Task with Tags

```sql
SELECT t.*, GROUP_CONCAT(tag.title, ',') as tag_names
FROM TMTask t
LEFT JOIN TMTaskTag tt ON t.uuid = tt.tasks
LEFT JOIN TMTag tag ON tt.tags = tag.uuid
WHERE t.uuid = ?
GROUP BY t.uuid;
```

## Schema Compatibility

### Version Differences

Things 3 has evolved over time. Key schema changes:

**Field Renames**:
- `dueDate` → `deadline`
- `dueDateOffset` → `t2_deadlineOffset`
- `actionGroup` → `heading`
- `delegate` → `contact`
- `lastAlarmInteractionDate` → `lastReminderInteractionDate`

**Type Changes**:
- Date fields: `REAL` → `INTEGER` (seconds since 2001)

### Handling Schema Variations

```rust
// Flexible date field handling
let start_date = row.try_get::<Option<i64>, _>("startDate")
    .or_else(|_| row.try_get::<Option<f64>, _>("startDate")
        .map(|opt| opt.map(|f| f as i64)))
    .ok()
    .flatten()
    .and_then(from_things_date);
```

### Database Migration Tracking

Things 3 tracks migrations in the `Meta` table:

```sql
SELECT * FROM Meta WHERE key = 'databaseVersion';
```

## Best Practices

### Read-Only Access

**Always open database in read-only mode**:

```rust
let db = ThingsDatabase::new(path).await?;
// SQLx opens in read-only mode by default for SQLite
```

### Connection Pooling

```rust
// Use connection pool for concurrent access
let pool = SqlitePool::connect_with(
    SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .create_if_missing(false)
).await?;
```

### Query Optimization

1. **Use indexes**: Leverage existing indexes
2. **Limit results**: Always use LIMIT for large queries
3. **Prepared statements**: SQLx prepares statements automatically
4. **Batch queries**: Combine related queries when possible

### Error Handling

```rust
// Handle missing database gracefully
match ThingsDatabase::new(path).await {
    Ok(db) => { /* use database */ },
    Err(ThingsError::Database(msg)) if msg.contains("not found") => {
        eprintln!("Database not found. Is Things 3 installed?");
    },
    Err(e) => return Err(e),
}
```

### Caching Strategy

```rust
// Cache expensive queries
let cache = ThingsCache::new(1000, Duration::from_secs(300));

let tasks = cache.get_or_insert("inbox", || {
    db.get_inbox(Some(50))
}).await?;
```

## Data Model Mapping

### Rust Structs

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub uuid: String,
    pub title: String,
    pub notes: Option<String>,
    pub status: i32,
    pub start_date: Option<NaiveDate>,
    pub deadline: Option<NaiveDate>,
    pub project: Option<String>,
    pub area: Option<String>,
    pub tags: Vec<String>,
    pub checklist_items: Vec<ChecklistItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub uuid: String,
    pub title: String,
    pub notes: Option<String>,
    pub area: Option<String>,
    pub deadline: Option<NaiveDate>,
    pub tags: Vec<String>,
    pub task_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    pub uuid: String,
    pub title: String,
    pub visible: bool,
    pub tags: Vec<String>,
}
```

## References

- [Things 3 Database Analysis](../THINGS3_DATABASE_ANALYSIS.md)
- [Architecture Documentation](./ARCHITECTURE.md)
- [MCP Integration Guide](./MCP_INTEGRATION.md)
- [SQLite Documentation](https://www.sqlite.org/docs.html)

