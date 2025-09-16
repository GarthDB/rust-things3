# Things 3 Data Model Mapping

This document maps the Things 3 database schema to our Rust data models and provides implementation guidance.

## Core Data Models

### Task Model

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

#### Database Mapping

| Rust Field | Database Field | Type | Conversion |
|------------|----------------|------|------------|
| `uuid` | `TMTask.uuid` | TEXT | `Uuid::parse_str()` |
| `title` | `TMTask.title` | TEXT | Direct mapping |
| `notes` | `TMTask.notes` | TEXT | `Option<String>` |
| `task_type` | `TMTask.type` | INTEGER | Enum conversion |
| `status` | `TMTask.status` | INTEGER | Enum conversion |
| `start_date` | `TMTask.startDate` | INTEGER | Days to `NaiveDate` |
| `deadline` | `TMTask.deadline` | INTEGER | Days to `NaiveDate` |
| `created` | `TMTask.creationDate` | REAL | Core Data timestamp |
| `modified` | `TMTask.userModificationDate` | REAL | Core Data timestamp |
| `project_uuid` | `TMTask.project` | TEXT | `Uuid::parse_str()` |
| `area_uuid` | `TMTask.area` | TEXT | `Uuid::parse_str()` |
| `parent_uuid` | `TMTask.heading` | TEXT | `Uuid::parse_str()` |
| `tags` | `TMTaskTag` | Many-to-many | Join query |

### Project Model

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

#### Database Mapping

Projects are stored in the `TMTask` table with `type = 1`:

| Rust Field | Database Field | Type | Conversion |
|------------|----------------|------|------------|
| `uuid` | `TMTask.uuid` | TEXT | `Uuid::parse_str()` |
| `title` | `TMTask.title` | TEXT | Direct mapping |
| `notes` | `TMTask.notes` | TEXT | `Option<String>` |
| `start_date` | `TMTask.startDate` | INTEGER | Days to `NaiveDate` |
| `deadline` | `TMTask.deadline` | INTEGER | Days to `NaiveDate` |
| `created` | `TMTask.creationDate` | REAL | Core Data timestamp |
| `modified` | `TMTask.userModificationDate` | REAL | Core Data timestamp |
| `area_uuid` | `TMTask.area` | TEXT | `Uuid::parse_str()` |
| `status` | `TMTask.status` | INTEGER | Enum conversion |
| `tags` | `TMTaskTag` | Many-to-many | Join query |
| `tasks` | `TMTask` | One-to-many | Separate query |

### Area Model

```rust
pub struct Area {
    pub uuid: Uuid,
    pub title: String,
    pub notes: Option<String>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}
```

#### Database Mapping

| Rust Field | Database Field | Type | Conversion |
|------------|----------------|------|------------|
| `uuid` | `TMArea.uuid` | TEXT | `Uuid::parse_str()` |
| `title` | `TMArea.title` | TEXT | Direct mapping |
| `notes` | `TMArea.title` | TEXT | Not available in TMArea |
| `created` | N/A | N/A | Not tracked in TMArea |
| `modified` | N/A | N/A | Not tracked in TMArea |

**Note:** Areas don't have notes, created, or modified fields in the database.

### Tag Model

```rust
pub struct Tag {
    pub uuid: Uuid,
    pub title: String,
    pub shortcut: Option<String>,
    pub used_date: Option<DateTime<Utc>>,
    pub parent_uuid: Option<Uuid>,
    pub index: i32,
}
```

#### Database Mapping

| Rust Field | Database Field | Type | Conversion |
|------------|----------------|------|------------|
| `uuid` | `TMTag.uuid` | TEXT | `Uuid::parse_str()` |
| `title` | `TMTag.title` | TEXT | Direct mapping |
| `shortcut` | `TMTag.shortcut` | TEXT | `Option<String>` |
| `used_date` | `TMTag.usedDate` | REAL | Core Data timestamp |
| `parent_uuid` | `TMTag.parent` | TEXT | `Uuid::parse_str()` |
| `index` | `TMTag.index` | INTEGER | Direct mapping |

### ChecklistItem Model

```rust
pub struct ChecklistItem {
    pub uuid: Uuid,
    pub title: String,
    pub status: ChecklistStatus,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub task_uuid: Uuid,
    pub index: i32,
}
```

#### Database Mapping

| Rust Field | Database Field | Type | Conversion |
|------------|----------------|------|------------|
| `uuid` | `TMChecklistItem.uuid` | TEXT | `Uuid::parse_str()` |
| `title` | `TMChecklistItem.title` | TEXT | Direct mapping |
| `status` | `TMChecklistItem.status` | INTEGER | Enum conversion |
| `created` | `TMChecklistItem.creationDate` | REAL | Core Data timestamp |
| `modified` | `TMChecklistItem.userModificationDate` | REAL | Core Data timestamp |
| `task_uuid` | `TMChecklistItem.task` | TEXT | `Uuid::parse_str()` |
| `index` | `TMChecklistItem.index` | INTEGER | Direct mapping |

## Type Conversions

### Date Conversions

#### Core Data Timestamps to DateTime<Utc>
```rust
fn core_data_timestamp_to_datetime(timestamp: f64) -> DateTime<Utc> {
    let base_date = DateTime::parse_from_rfc3339("2001-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    base_date + Duration::seconds(timestamp as i64)
}
```

#### Days since 2001 to NaiveDate
```rust
fn days_since_2001_to_date(days: i32) -> Option<NaiveDate> {
    let base_date = NaiveDate::from_ymd_opt(2001, 1, 1)?;
    base_date.checked_add_days(Days::new(days as u64))
}
```

#### NaiveDate to days since 2001
```rust
fn date_to_days_since_2001(date: NaiveDate) -> i32 {
    let base_date = NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
    date.signed_duration_since(base_date).num_days() as i32
}
```

### Enum Conversions

#### Task Type
```rust
impl From<i32> for TaskType {
    fn from(value: i32) -> Self {
        match value {
            0 => TaskType::Todo,
            1 => TaskType::Project,
            2 => TaskType::Heading,
            3 => TaskType::Area,
            _ => TaskType::Todo, // Default fallback
        }
    }
}

impl From<TaskType> for i32 {
    fn from(task_type: TaskType) -> Self {
        match task_type {
            TaskType::Todo => 0,
            TaskType::Project => 1,
            TaskType::Heading => 2,
            TaskType::Area => 3,
        }
    }
}
```

#### Task Status
```rust
impl From<i32> for TaskStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => TaskStatus::Incomplete,
            1 => TaskStatus::Completed,
            2 => TaskStatus::Canceled,
            3 => TaskStatus::Trashed,
            _ => TaskStatus::Incomplete, // Default fallback
        }
    }
}

impl From<TaskStatus> for i32 {
    fn from(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Incomplete => 0,
            TaskStatus::Completed => 1,
            TaskStatus::Canceled => 2,
            TaskStatus::Trashed => 3,
        }
    }
}
```

## Query Patterns

### Common Query Templates

#### Get Tasks by Type
```sql
SELECT * FROM TMTask WHERE type = ? AND status = 0 ORDER BY creationDate DESC;
```

#### Get Tasks by Area
```sql
SELECT * FROM TMTask WHERE area = ? AND status = 0 ORDER BY creationDate DESC;
```

#### Get Tasks by Project
```sql
SELECT * FROM TMTask WHERE project = ? AND status = 0 ORDER BY creationDate DESC;
```

#### Get Inbox Tasks
```sql
SELECT * FROM TMTask 
WHERE status = 0 
  AND area IS NULL 
  AND project IS NULL 
  AND heading IS NULL
ORDER BY creationDate DESC;
```

#### Get Today's Tasks
```sql
SELECT * FROM TMTask 
WHERE status = 0 
  AND startDate = ?
ORDER BY todayIndex;
```

#### Search Tasks
```sql
SELECT * FROM TMTask 
WHERE (title LIKE ? OR notes LIKE ?) 
  AND status = 0
ORDER BY creationDate DESC;
```

### Tag Relationships

#### Get Tags for Task
```sql
SELECT t.* FROM TMTag t
JOIN TMTaskTag tt ON t.uuid = tt.tag
WHERE tt.task = ?;
```

#### Get Tasks with Tag
```sql
SELECT t.* FROM TMTask t
JOIN TMTaskTag tt ON t.uuid = tt.task
WHERE tt.tag = ? AND t.status = 0;
```

## Error Handling

### Database Errors
- Invalid UUIDs should be handled gracefully
- Missing foreign key references should be logged
- Type conversion errors should be caught and handled
- Database connection errors should be retried

### Data Validation
- Validate UUIDs before database operations
- Check required fields before insertion
- Validate date ranges and constraints
- Handle null/empty values appropriately

## Performance Considerations

### Indexing Strategy
- Use indexed columns in WHERE clauses
- Avoid full table scans when possible
- Use LIMIT clauses for large result sets
- Consider compound indexes for complex queries

### Caching Strategy
- Cache frequently accessed data (areas, tags)
- Use appropriate TTL for different data types
- Invalidate cache on data modifications
- Consider memory usage vs. performance trade-offs

### Query Optimization
- Use prepared statements for repeated queries
- Batch operations when possible
- Use transactions for multiple related operations
- Monitor query performance and optimize as needed

## Migration and Compatibility

### Schema Changes
- Handle missing fields gracefully
- Provide default values for new fields
- Maintain backward compatibility where possible
- Version database schema changes

### Data Integrity
- Validate data before database operations
- Use transactions for atomic operations
- Handle constraint violations appropriately
- Maintain referential integrity at application level

This mapping provides the foundation for implementing robust database access patterns in the Rust Things library while maintaining compatibility with the Things 3 database schema.
