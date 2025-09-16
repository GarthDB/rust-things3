# Things 3 Database Schema Analysis

This document provides a comprehensive analysis of the Things 3 database schema based on real database inspection.

## Database Location

**Primary Database:**
```
~/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite
```

**Backup Location:**
```
~/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Backups/
```

## Database Overview

The Things 3 database is a SQLite database with the following characteristics:
- **Total Tables:** 15
- **Primary Tables:** TMTask, TMArea, TMTag, TMChecklistItem
- **Sample Data:** 1,036 tasks, 6 areas, 79 tags

## Table Schema Analysis

### TMTask (Main Tasks Table)

The `TMTask` table is the core table containing all tasks, projects, headings, and areas.

#### Schema Definition
```sql
CREATE TABLE TMTask (
    "uuid"                              TEXT PRIMARY KEY,
    "leavesTombstone"                   INTEGER,
    "creationDate"                      REAL,
    "userModificationDate"              REAL,
    "type"                              INTEGER,
    "status"                            INTEGER,
    "stopDate"                          REAL,
    "trashed"                           INTEGER,
    "title"                             TEXT,
    "notes"                             TEXT,
    "notesSync"                         INTEGER,
    "cachedTags"                        BLOB,
    "start"                             INTEGER,
    "startDate"                         INTEGER,
    "startBucket"                       INTEGER,
    "reminderTime"                      INTEGER,
    "lastReminderInteractionDate"       REAL,
    "deadline"                          INTEGER,
    "deadlineSuppressionDate"           INTEGER,
    "t2_deadlineOffset"                 INTEGER,
    "index"                             INTEGER,
    "todayIndex"                        INTEGER,
    "todayIndexReferenceDate"           INTEGER,
    "area"                              TEXT,
    "project"                           TEXT,
    "heading"                           TEXT,
    "contact"                           TEXT,
    "untrashedLeafActionsCount"         INTEGER,
    "openUntrashedLeafActionsCount"     INTEGER,
    "checklistItemsCount"               INTEGER,
    "openChecklistItemsCount"           INTEGER,
    "rt1_repeatingTemplate"             TEXT,
    "rt1_recurrenceRule"                BLOB,
    "rt1_instanceCreationStartDate"     INTEGER,
    "rt1_instanceCreationPaused"        INTEGER,
    "rt1_instanceCreationCount"         INTEGER,
    "rt1_afterCompletionReferenceDate"  INTEGER,
    "rt1_nextInstanceStartDate"         INTEGER,
    "experimental"                      BLOB,
    "repeater"                          BLOB,
    "repeaterMigrationDate"             REAL
);
```

#### Key Fields Analysis

| Field | Type | Description | Notes |
|-------|------|-------------|-------|
| `uuid` | TEXT | Primary key | Unique identifier for the task |
| `type` | INTEGER | Task type | 0=Todo, 1=Project, 2=Heading, 3=Area |
| `status` | INTEGER | Task status | 0=Incomplete, 1=Completed, 2=Canceled, 3=Trashed |
| `title` | TEXT | Task title | Main display text |
| `notes` | TEXT | Task notes | Additional description |
| `startDate` | INTEGER | Start date | Days since 2001-01-01 |
| `deadline` | INTEGER | Deadline | Days since 2001-01-01 |
| `creationDate` | REAL | Creation timestamp | Core Data timestamp (seconds since 2001-01-01) |
| `userModificationDate` | REAL | Last modified | Core Data timestamp |
| `area` | TEXT | Area UUID | Foreign key to TMArea.uuid |
| `project` | TEXT | Project UUID | Foreign key to TMTask.uuid (type=1) |
| `heading` | TEXT | Heading UUID | Foreign key to TMTask.uuid (type=2) |

#### Indexes
```sql
CREATE INDEX index_TMTask_stopDate ON TMTask(stopDate);
CREATE INDEX index_TMTask_project ON TMTask(project);
CREATE INDEX index_TMTask_heading ON TMTask(heading);
CREATE INDEX index_TMTask_area ON TMTask(area);
CREATE INDEX index_TMTask_repeatingTemplate ON TMTask(rt1_repeatingTemplate);
```

### TMArea (Areas Table)

Areas are the top-level organizational units in Things 3.

#### Schema Definition
```sql
CREATE TABLE IF NOT EXISTS 'TMArea' (
    'uuid'                 TEXT PRIMARY KEY,
    'title'                TEXT,
    'visible'              INTEGER,
    'index'                INTEGER,
    'cachedTags'           BLOB,
    'experimental'         BLOB
);
```

#### Key Fields Analysis

| Field | Type | Description | Notes |
|-------|------|-------------|-------|
| `uuid` | TEXT | Primary key | Unique identifier for the area |
| `title` | TEXT | Area name | Display name of the area |
| `visible` | INTEGER | Visibility flag | 1=visible, 0=hidden |
| `index` | INTEGER | Sort order | Display order in the UI |

### TMTag (Tags Table)

Tags are used for categorization and filtering.

#### Schema Definition
```sql
CREATE TABLE TMTag (
    'uuid'                 TEXT PRIMARY KEY,
    'title'                TEXT,
    'shortcut'             TEXT,
    'usedDate'             REAL,
    'parent'               TEXT,
    'index'                INTEGER,
    'experimental'         BLOB
);
```

#### Key Fields Analysis

| Field | Type | Description | Notes |
|-------|------|-------------|-------|
| `uuid` | TEXT | Primary key | Unique identifier for the tag |
| `title` | TEXT | Tag name | Display name of the tag |
| `shortcut` | TEXT | Keyboard shortcut | Quick access key |
| `usedDate` | REAL | Last used | Core Data timestamp |
| `parent` | TEXT | Parent tag UUID | For hierarchical tags |
| `index` | INTEGER | Sort order | Display order |

### TMChecklistItem (Checklist Items Table)

Checklist items are sub-tasks within tasks.

#### Schema Definition
```sql
CREATE TABLE TMChecklistItem (
    'uuid'                 TEXT PRIMARY KEY,
    'userModificationDate' REAL,
    'creationDate'         REAL,
    'title'                TEXT,
    'status'               INTEGER,
    'stopDate'             REAL,
    'index'                INTEGER,
    'task'                 TEXT,
    'leavesTombstone'      INTEGER,
    'experimental'         BLOB
);
```

#### Key Fields Analysis

| Field | Type | Description | Notes |
|-------|------|-------------|-------|
| `uuid` | TEXT | Primary key | Unique identifier for the checklist item |
| `title` | TEXT | Item text | The checklist item content |
| `status` | INTEGER | Completion status | 0=incomplete, 1=completed |
| `task` | TEXT | Parent task UUID | Foreign key to TMTask.uuid |
| `creationDate` | REAL | Creation timestamp | Core Data timestamp |
| `userModificationDate` | REAL | Last modified | Core Data timestamp |

### TMSettings (Settings Table)

Application settings and preferences.

#### Schema Definition
```sql
CREATE TABLE TMSettings (
    'uuid'                 TEXT PRIMARY KEY,
    'logInterval'          INTEGER,
    'manualLogDate'        REAL,
    'groupTodayByParent'   INTEGER,
    'uriSchemeAuthenticationToken' TEXT,
    'experimental'         BLOB
);
```

## Data Type Mappings

### Date/Time Fields

| Database Type | Rust Type | Description | Conversion |
|---------------|-----------|-------------|------------|
| `REAL` (Core Data timestamp) | `DateTime<Utc>` | Timestamps | `base_date + Duration::seconds(timestamp)` |
| `INTEGER` (Days since 2001) | `NaiveDate` | Dates | `base_date + Days::new(days)` |

**Base Date:** January 1, 2001 (Core Data epoch)

### Status Enums

#### Task Type (TMTask.type)
```rust
pub enum TaskType {
    Todo = 0,      // Regular task
    Project = 1,   // Project (container for tasks)
    Heading = 2,   // Section heading
    Area = 3,      // Area (top-level container)
}
```

#### Task Status (TMTask.status)
```rust
pub enum TaskStatus {
    Incomplete = 0,  // Not completed
    Completed = 1,   // Finished
    Canceled = 2,    // Cancelled
    Trashed = 3,     // Deleted
}
```

## Relationships

### Primary Relationships

1. **TMTask → TMArea**: `TMTask.area` → `TMArea.uuid`
2. **TMTask → TMTask (Project)**: `TMTask.project` → `TMTask.uuid` (where type=1)
3. **TMTask → TMTask (Heading)**: `TMTask.heading` → `TMTask.uuid` (where type=2)
4. **TMChecklistItem → TMTask**: `TMChecklistItem.task` → `TMTask.uuid`
5. **TMTask ↔ TMTag**: Many-to-many via `TMTaskTag` table

### Foreign Key Constraints

- All foreign key relationships are enforced at the application level
- No database-level foreign key constraints are defined
- UUIDs are used for all relationships

## Access Patterns

### Common Queries

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
  AND startDate = ?  -- Today's date in days since 2001-01-01
ORDER BY todayIndex;
```

#### Get Projects in Area
```sql
SELECT * FROM TMTask 
WHERE type = 1 
  AND area = ?  -- Area UUID
ORDER BY creationDate DESC;
```

#### Get Areas
```sql
SELECT * FROM TMArea 
WHERE visible = 1 
ORDER BY index;
```

#### Search Tasks
```sql
SELECT * FROM TMTask 
WHERE (title LIKE ? OR notes LIKE ?) 
  AND status = 0
ORDER BY creationDate DESC;
```

## Performance Considerations

### Indexes
- Primary key on `uuid` fields
- Indexes on foreign key fields (`area`, `project`, `heading`)
- Index on `stopDate` for completed tasks
- Index on `rt1_repeatingTemplate` for recurring tasks

### Query Optimization
- Use appropriate WHERE clauses to leverage indexes
- Limit result sets with LIMIT clauses
- Use ORDER BY on indexed columns when possible
- Consider caching frequently accessed data

## Data Integrity

### Constraints
- All tables use TEXT PRIMARY KEY for UUIDs
- No database-level foreign key constraints
- Application-level validation required

### Tombstone Pattern
- `leavesTombstone` field indicates deleted items
- Soft deletes are used for data recovery
- Tombstone records are kept for sync purposes

## Migration Notes

### Schema Changes
- Field renames are documented in comments
- Type changes (REAL → INTEGER) are noted
- New fields are added with default values
- Experimental fields use BLOB type

### Version Compatibility
- Database schema is versioned
- Migration scripts handle schema updates
- Backward compatibility is maintained where possible

## Security Considerations

### Data Access
- Database files are protected by macOS sandboxing
- No direct network access to database
- Local file system permissions apply

### Data Privacy
- All data is stored locally
- No cloud sync without explicit user consent
- Sensitive data should be encrypted at rest

## Sample Data Analysis

Based on the analyzed database:
- **1,036 tasks** total
- **6 areas** (Adobe, Egghead, Executive Secretary, Home, Seminary, Virtual Assistant)
- **79 tags** for categorization
- Mix of todos, projects, and headings
- Various completion states represented

This analysis provides the foundation for implementing robust database access patterns in the Rust Things library.
