# SQLx Migration Guide

This document describes the migration from `rusqlite` to `SQLx` in the Rust Things project.

## Overview

The migration from `rusqlite` to `SQLx` was undertaken to address several key issues:

1. **Thread Safety**: `rusqlite::Connection` is not `Send + Sync`, preventing its use in async web servers
2. **Async Support**: `rusqlite` is synchronous, while `SQLx` provides native async support
3. **Performance**: `SQLx` offers better performance for concurrent database operations
4. **Modern Rust**: `SQLx` is designed for modern async Rust applications

## Key Changes

### Database Layer

**Before (rusqlite):**
```rust
use rusqlite::Connection;

pub struct ThingsDatabase {
    conn: Connection,
}

impl ThingsDatabase {
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }
    
    pub fn get_inbox(&self, limit: Option<usize>) -> Result<Vec<Task>> {
        // Synchronous database operations
    }
}
```

**After (SQLx):**
```rust
use sqlx::SqlitePool;

pub struct ThingsDatabase {
    pool: SqlitePool,
}

impl ThingsDatabase {
    pub async fn new(path: &Path) -> Result<Self> {
        let database_url = format!("sqlite:{}", path.display());
        let pool = SqlitePool::connect(&database_url).await?;
        Ok(Self { pool })
    }
    
    pub async fn get_inbox(&self, limit: Option<usize>) -> Result<Vec<Task>> {
        // Async database operations
    }
}
```

### API Changes

All database operations are now async:

```rust
// Before
let tasks = db.get_inbox(Some(10))?;

// After
let tasks = db.get_inbox(Some(10)).await?;
```

### Error Handling

Error types have been simplified:

```rust
// Before
pub enum ThingsError {
    Database(#[from] rusqlite::Error),
    // ...
}

// After
pub enum ThingsError {
    Database(String),
    // ...
}
```

### Web Server Integration

The migration enables proper web server integration:

```rust
// Health server with SQLx
pub struct HealthServer {
    database: Arc<ThingsDatabase>, // Now Send + Sync
    observability: Arc<ObservabilityManager>,
}

// Dashboard server with SQLx
pub struct DashboardServer {
    database: Arc<ThingsDatabase>, // Now Send + Sync
    observability: Arc<ObservabilityManager>,
}
```

## Migration Benefits

### 1. Thread Safety
- `SQLx` connections are `Send + Sync`
- Can be safely shared across async tasks
- Enables proper web server implementation

### 2. Async Performance
- Native async/await support
- Better concurrency handling
- Improved performance for concurrent operations

### 3. Modern Rust Patterns
- Designed for async Rust applications
- Better integration with `tokio` runtime
- More idiomatic Rust code

### 4. Simplified Architecture
- Removed `ThreadSafeDatabase` wrapper
- Direct database access in web servers
- Cleaner error handling

## Database Schema Compatibility

The migration maintains full compatibility with the existing Things 3 database schema:

```sql
-- TMTask table (main tasks table)
CREATE TABLE IF NOT EXISTS TMTask (
    uuid TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    type INTEGER NOT NULL DEFAULT 0,
    status INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    start_date TEXT,
    due_date TEXT,
    created TEXT NOT NULL,
    modified TEXT NOT NULL,
    project_uuid TEXT,
    area_uuid TEXT,
    parent_uuid TEXT,
    tags TEXT DEFAULT '[]'
);

-- TMProject table (projects table)
CREATE TABLE IF NOT EXISTS TMProject (
    uuid TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    notes TEXT,
    start_date TEXT,
    due_date TEXT,
    created TEXT NOT NULL,
    modified TEXT NOT NULL,
    area_uuid TEXT,
    tags TEXT DEFAULT '[]'
);

-- TMArea table (areas table)
CREATE TABLE IF NOT EXISTS TMArea (
    uuid TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    notes TEXT,
    created TEXT NOT NULL,
    modified TEXT NOT NULL,
    tags TEXT DEFAULT '[]'
);
```

## Testing

The test harness has been updated to use SQLx:

```rust
async fn create_test_database<P: AsRef<Path>>(db_path: P) -> ThingsDatabase {
    use sqlx::SqlitePool;
    
    let database_url = format!("sqlite:{}", db_path.as_ref().display());
    let pool = SqlitePool::connect(&database_url).await.unwrap();

    // Create schema and insert test data
    // ...

    pool.close().await;
    ThingsDatabase::new(db_path.as_ref()).await.unwrap()
}
```

## Performance Impact

The migration provides several performance improvements:

1. **Concurrent Operations**: Better handling of multiple concurrent database operations
2. **Connection Pooling**: `SQLx` provides built-in connection pooling
3. **Async Efficiency**: Reduced blocking in async contexts
4. **Memory Usage**: More efficient memory usage with connection pooling

## Breaking Changes

### API Changes
- All database methods are now async
- `ThingsDatabase::new()` now returns `Future`
- Error types simplified (no more `rusqlite::Error`)

### Dependencies
- Removed `rusqlite` dependency
- Added `sqlx` with SQLite features
- Updated `Cargo.toml` files

### Configuration
- Database connection now uses connection strings
- Environment variable handling updated

## Migration Checklist

- [x] Add SQLx dependencies
- [x] Create new database module with SQLx
- [x] Update all database operations to async
- [x] Migrate web servers to use SQLx
- [x] Update MCP server for SQLx compatibility
- [x] Fix test harness for SQLx
- [x] Remove rusqlite dependencies
- [x] Update documentation
- [x] Test all functionality

## Future Considerations

1. **Database Migrations**: Consider adding proper database migration support
2. **Connection Pooling**: Fine-tune connection pool settings for production
3. **Monitoring**: Add database connection monitoring
4. **Backup/Restore**: Update backup/restore functionality for SQLx
5. **Performance Tuning**: Optimize queries for better performance

## Conclusion

The SQLx migration successfully addresses the thread safety and async support issues while maintaining full compatibility with the existing Things 3 database schema. The new implementation provides better performance, cleaner code, and proper integration with modern async Rust applications.

