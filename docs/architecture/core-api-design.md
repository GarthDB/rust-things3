# Core Library API Design

This document outlines the comprehensive API design for the Rust Things core library, following the key design principles of performance, type safety, error handling, caching support, and serialization support.

## Design Principles

### 1. Performance First
- **Async/Await**: All I/O operations are asynchronous
- **Connection Pooling**: Reuse database connections efficiently
- **Caching**: Multi-level caching for frequently accessed data
- **Batch Operations**: Support for bulk operations
- **Lazy Loading**: Load related data only when needed

### 2. Type Safety
- **Strong Typing**: Use Rust's type system to prevent runtime errors
- **Newtype Patterns**: Wrap primitive types for domain-specific meaning
- **Enum-based State**: Use enums for status and type fields
- **Option Types**: Explicit handling of optional data

### 3. Error Handling
- **Result Types**: All operations return `Result<T, ThingsError>`
- **Error Context**: Rich error information with context
- **Error Recovery**: Graceful handling of recoverable errors
- **Error Propagation**: Proper error bubbling with context

### 4. Caching Support
- **Multi-level Caching**: Memory and disk caching
- **Cache Invalidation**: Smart invalidation strategies
- **Cache Statistics**: Performance monitoring
- **Configurable TTL**: Time-to-live configuration

### 5. Serialization Support
- **Serde Integration**: Full serialization support
- **Multiple Formats**: JSON, MessagePack, Bincode
- **Version Compatibility**: Backward/forward compatibility
- **Custom Serializers**: Domain-specific serialization

## Core Data Structures

### Enhanced Task Model

```rust
/// Enhanced task entity with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier
    pub uuid: Uuid,
    /// Task title
    pub title: String,
    /// Task type
    pub task_type: TaskType,
    /// Task status
    pub status: TaskStatus,
    /// Optional notes
    pub notes: Option<String>,
    /// Start date
    pub start_date: Option<NaiveDate>,
    /// Deadline
    pub deadline: Option<NaiveDate>,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// Last modification timestamp
    pub modified: DateTime<Utc>,
    /// Parent project UUID
    pub project_uuid: Option<Uuid>,
    /// Parent area UUID
    pub area_uuid: Option<Uuid>,
    /// Parent task UUID (for headings)
    pub parent_uuid: Option<Uuid>,
    /// Associated tags
    pub tags: Vec<Tag>,
    /// Checklist items
    pub checklist_items: Vec<ChecklistItem>,
    /// Child tasks (for projects and headings)
    pub children: Vec<Task>,
    /// Recurrence information
    pub recurrence: Option<RecurrenceRule>,
    /// Priority level
    pub priority: Priority,
    /// Completion percentage (0-100)
    pub completion_percentage: u8,
    /// Estimated duration
    pub estimated_duration: Option<Duration>,
    /// Actual time spent
    pub time_spent: Option<Duration>,
    /// Custom metadata
    pub metadata: HashMap<String, Value>,
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Recurrence rule for repeating tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurrenceRule {
    /// Recurrence frequency
    pub frequency: RecurrenceFrequency,
    /// Interval (every N days/weeks/months)
    pub interval: u32,
    /// Days of week (for weekly recurrence)
    pub days_of_week: Option<Vec<Weekday>>,
    /// Days of month (for monthly recurrence)
    pub days_of_month: Option<Vec<u8>>,
    /// End date for recurrence
    pub end_date: Option<NaiveDate>,
    /// Maximum occurrences
    pub max_occurrences: Option<u32>,
}

/// Checklist item for task breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    /// Unique identifier
    pub uuid: Uuid,
    /// Item text
    pub title: String,
    /// Completion status
    pub completed: bool,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// Completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
    /// Sort order
    pub index: u32,
}
```

### Enhanced Project Model

```rust
/// Enhanced project entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique identifier
    pub uuid: Uuid,
    /// Project title
    pub title: String,
    /// Optional notes
    pub notes: Option<String>,
    /// Start date
    pub start_date: Option<NaiveDate>,
    /// Deadline
    pub deadline: Option<NaiveDate>,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// Last modification timestamp
    pub modified: DateTime<Utc>,
    /// Parent area UUID
    pub area_uuid: Option<Uuid>,
    /// Associated tags
    pub tags: Vec<Tag>,
    /// Project status
    pub status: TaskStatus,
    /// Child tasks
    pub tasks: Vec<Task>,
    /// Project progress (0-100)
    pub progress: u8,
    /// Project priority
    pub priority: Priority,
    /// Project color (for UI)
    pub color: Option<Color>,
    /// Project icon (for UI)
    pub icon: Option<String>,
    /// Custom metadata
    pub metadata: HashMap<String, Value>,
}

/// Color representation for UI elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}
```

### Enhanced Area Model

```rust
/// Enhanced area entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    /// Unique identifier
    pub uuid: Uuid,
    /// Area title
    pub title: String,
    /// Optional notes
    pub notes: Option<String>,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// Last modification timestamp
    pub modified: DateTime<Utc>,
    /// Associated tags
    pub tags: Vec<Tag>,
    /// Child projects
    pub projects: Vec<Project>,
    /// Area color (for UI)
    pub color: Option<Color>,
    /// Area icon (for UI)
    pub icon: Option<String>,
    /// Visibility status
    pub visible: bool,
    /// Sort order
    pub index: i32,
    /// Custom metadata
    pub metadata: HashMap<String, Value>,
}
```

### Enhanced Tag Model

```rust
/// Enhanced tag entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// Unique identifier
    pub uuid: Uuid,
    /// Tag title
    pub title: String,
    /// Tag color (for UI)
    pub color: Option<Color>,
    /// Usage count
    pub usage_count: u32,
    /// Last used timestamp
    pub last_used: Option<DateTime<Utc>>,
    /// Parent tag (for hierarchical tags)
    pub parent_uuid: Option<Uuid>,
    /// Child tags
    pub children: Vec<Tag>,
    /// Sort order
    pub index: i32,
    /// Custom metadata
    pub metadata: HashMap<String, Value>,
}
```

## Database Access Layer Interface

### Core Database Trait

```rust
/// Core database operations trait
#[async_trait]
pub trait ThingsDatabase {
    /// Get database connection info
    fn connection_info(&self) -> &DatabaseInfo;
    
    /// Health check
    async fn health_check(&self) -> Result<HealthStatus>;
    
    /// Get database statistics
    async fn get_statistics(&self) -> Result<DatabaseStatistics>;
    
    /// Begin transaction
    async fn begin_transaction(&self) -> Result<Transaction>;
    
    /// Close database connection
    async fn close(self) -> Result<()>;
}

/// Transaction operations
#[async_trait]
pub trait Transaction {
    /// Commit transaction
    async fn commit(self) -> Result<()>;
    
    /// Rollback transaction
    async fn rollback(self) -> Result<()>;
    
    /// Get task operations
    fn tasks(&self) -> TaskOperations;
    
    /// Get project operations
    fn projects(&self) -> ProjectOperations;
    
    /// Get area operations
    fn areas(&self) -> AreaOperations;
    
    /// Get tag operations
    fn tags(&self) -> TagOperations;
}
```

### Task Operations

```rust
/// Task-specific database operations
#[async_trait]
pub trait TaskOperations {
    /// Create a new task
    async fn create(&self, task: &CreateTaskRequest) -> Result<Task>;
    
    /// Get task by UUID
    async fn get_by_uuid(&self, uuid: &Uuid) -> Result<Option<Task>>;
    
    /// Update task
    async fn update(&self, task: &UpdateTaskRequest) -> Result<Task>;
    
    /// Delete task
    async fn delete(&self, uuid: &Uuid) -> Result<()>;
    
    /// Get tasks with filters
    async fn get_filtered(&self, filters: &TaskFilters) -> Result<Vec<Task>>;
    
    /// Get inbox tasks
    async fn get_inbox(&self, limit: Option<usize>) -> Result<Vec<Task>>;
    
    /// Get today's tasks
    async fn get_today(&self, limit: Option<usize>) -> Result<Vec<Task>>;
    
    /// Search tasks
    async fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<Task>>;
    
    /// Get tasks by project
    async fn get_by_project(&self, project_uuid: &Uuid) -> Result<Vec<Task>>;
    
    /// Get tasks by area
    async fn get_by_area(&self, area_uuid: &Uuid) -> Result<Vec<Task>>;
    
    /// Get tasks by tag
    async fn get_by_tag(&self, tag_uuid: &Uuid) -> Result<Vec<Task>>;
    
    /// Bulk create tasks
    async fn bulk_create(&self, tasks: &[CreateTaskRequest]) -> Result<Vec<Task>>;
    
    /// Bulk update tasks
    async fn bulk_update(&self, updates: &[UpdateTaskRequest]) -> Result<Vec<Task>>;
    
    /// Bulk delete tasks
    async fn bulk_delete(&self, uuids: &[Uuid]) -> Result<()>;
}
```

### Project Operations

```rust
/// Project-specific database operations
#[async_trait]
pub trait ProjectOperations {
    /// Create a new project
    async fn create(&self, project: &CreateProjectRequest) -> Result<Project>;
    
    /// Get project by UUID
    async fn get_by_uuid(&self, uuid: &Uuid) -> Result<Option<Project>>;
    
    /// Update project
    async fn update(&self, project: &UpdateProjectRequest) -> Result<Project>;
    
    /// Delete project
    async fn delete(&self, uuid: &Uuid) -> Result<()>;
    
    /// Get all projects
    async fn get_all(&self) -> Result<Vec<Project>>;
    
    /// Get projects by area
    async fn get_by_area(&self, area_uuid: &Uuid) -> Result<Vec<Project>>;
    
    /// Get projects with filters
    async fn get_filtered(&self, filters: &ProjectFilters) -> Result<Vec<Project>>;
    
    /// Search projects
    async fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<Project>>;
}
```

### Area Operations

```rust
/// Area-specific database operations
#[async_trait]
pub trait AreaOperations {
    /// Create a new area
    async fn create(&self, area: &CreateAreaRequest) -> Result<Area>;
    
    /// Get area by UUID
    async fn get_by_uuid(&self, uuid: &Uuid) -> Result<Option<Area>>;
    
    /// Update area
    async fn update(&self, area: &UpdateAreaRequest) -> Result<Area>;
    
    /// Delete area
    async fn delete(&self, uuid: &Uuid) -> Result<()>;
    
    /// Get all areas
    async fn get_all(&self) -> Result<Vec<Area>>;
    
    /// Get visible areas
    async fn get_visible(&self) -> Result<Vec<Area>>;
    
    /// Search areas
    async fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<Area>>;
}
```

### Tag Operations

```rust
/// Tag-specific database operations
#[async_trait]
pub trait TagOperations {
    /// Create a new tag
    async fn create(&self, tag: &CreateTagRequest) -> Result<Tag>;
    
    /// Get tag by UUID
    async fn get_by_uuid(&self, uuid: &Uuid) -> Result<Option<Tag>>;
    
    /// Update tag
    async fn update(&self, tag: &UpdateTagRequest) -> Result<Tag>;
    
    /// Delete tag
    async fn delete(&self, uuid: &Uuid) -> Result<()>;
    
    /// Get all tags
    async fn get_all(&self) -> Result<Vec<Tag>>;
    
    /// Get popular tags
    async fn get_popular(&self, limit: Option<usize>) -> Result<Vec<Tag>>;
    
    /// Search tags
    async fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<Tag>>;
    
    /// Get tags for task
    async fn get_for_task(&self, task_uuid: &Uuid) -> Result<Vec<Tag>>;
    
    /// Add tag to task
    async fn add_to_task(&self, task_uuid: &Uuid, tag_uuid: &Uuid) -> Result<()>;
    
    /// Remove tag from task
    async fn remove_from_task(&self, task_uuid: &Uuid, tag_uuid: &Uuid) -> Result<()>;
}
```

## Error Handling Strategy

### Enhanced Error Types

```rust
/// Comprehensive error types for Things operations
#[derive(Error, Debug)]
pub enum ThingsError {
    // Database errors
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("Database connection failed: {0}")]
    DatabaseConnection(String),
    
    #[error("Database transaction failed: {0}")]
    DatabaseTransaction(String),
    
    // Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Deserialization error: {0}")]
    Deserialization(String),
    
    // IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    // Validation errors
    #[error("Validation error: {message}")]
    Validation { message: String, field: Option<String> },
    
    // Not found errors
    #[error("Task not found: {uuid}")]
    TaskNotFound { uuid: Uuid },
    
    #[error("Project not found: {uuid}")]
    ProjectNotFound { uuid: Uuid },
    
    #[error("Area not found: {uuid}")]
    AreaNotFound { uuid: Uuid },
    
    #[error("Tag not found: {uuid}")]
    TagNotFound { uuid: Uuid },
    
    // Configuration errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },
    
    // Cache errors
    #[error("Cache error: {message}")]
    Cache { message: String },
    
    // Permission errors
    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },
    
    // Rate limiting
    #[error("Rate limit exceeded: {message}")]
    RateLimitExceeded { message: String },
    
    // Unknown errors
    #[error("Unknown error: {message}")]
    Unknown { message: String },
}

impl ThingsError {
    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            ThingsError::Database(_) => ErrorSeverity::High,
            ThingsError::DatabaseConnection(_) => ErrorSeverity::High,
            ThingsError::DatabaseTransaction(_) => ErrorSeverity::High,
            ThingsError::Validation { .. } => ErrorSeverity::Medium,
            ThingsError::TaskNotFound { .. } => ErrorSeverity::Low,
            ThingsError::ProjectNotFound { .. } => ErrorSeverity::Low,
            ThingsError::AreaNotFound { .. } => ErrorSeverity::Low,
            ThingsError::TagNotFound { .. } => ErrorSeverity::Low,
            ThingsError::Cache { .. } => ErrorSeverity::Low,
            _ => ErrorSeverity::Medium,
        }
    }
    
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            ThingsError::DatabaseConnection(_) => true,
            ThingsError::Cache { .. } => true,
            ThingsError::RateLimitExceeded { .. } => true,
            _ => false,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}
```

## Caching Layer Interface

### Cache Configuration

```rust
/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum cache size in bytes
    pub max_size: usize,
    /// Time to live for cached items
    pub ttl: Duration,
    /// Time to idle for cached items
    pub tti: Duration,
    /// Cache eviction policy
    pub eviction_policy: EvictionPolicy,
    /// Enable compression
    pub compression: bool,
    /// Cache statistics collection
    pub statistics: bool,
}

/// Cache eviction policies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Least recently used
    Lru,
    /// Least frequently used
    Lfu,
    /// First in, first out
    Fifo,
    /// Random
    Random,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of entries
    pub entries: usize,
    /// Cache hit rate (0.0 to 1.0)
    pub hit_rate: f64,
    /// Cache miss rate (0.0 to 1.0)
    pub miss_rate: f64,
    /// Total hits
    pub hits: u64,
    /// Total misses
    pub misses: u64,
    /// Cache size in bytes
    pub size_bytes: usize,
    /// Eviction count
    pub evictions: u64,
}
```

### Cache Operations

```rust
/// Cache operations trait
#[async_trait]
pub trait Cache {
    /// Get value from cache
    async fn get<K, V>(&self, key: &K) -> Result<Option<V>>
    where
        K: Serialize + Send + Sync,
        V: DeserializeOwned + Send + Sync;
    
    /// Put value in cache
    async fn put<K, V>(&self, key: K, value: V) -> Result<()>
    where
        K: Serialize + Send + Sync,
        V: Serialize + Send + Sync;
    
    /// Remove value from cache
    async fn remove<K>(&self, key: &K) -> Result<()>
    where
        K: Serialize + Send + Sync;
    
    /// Clear all cache entries
    async fn clear(&self) -> Result<()>;
    
    /// Get cache statistics
    fn stats(&self) -> CacheStats;
    
    /// Invalidate cache entries matching pattern
    async fn invalidate_pattern(&self, pattern: &str) -> Result<()>;
}
```

## Serialization Patterns

### Serialization Configuration

```rust
/// Serialization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializationConfig {
    /// Default format
    pub default_format: SerializationFormat,
    /// Compression enabled
    pub compression: bool,
    /// Pretty printing for JSON
    pub pretty_print: bool,
    /// Include metadata
    pub include_metadata: bool,
    /// Version compatibility
    pub version: String,
}

/// Supported serialization formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SerializationFormat {
    Json,
    MessagePack,
    Bincode,
    Cbor,
    Yaml,
    Toml,
}
```

### Serialization Operations

```rust
/// Serialization operations trait
pub trait Serializer {
    /// Serialize to bytes
    fn serialize<T>(&self, value: &T) -> Result<Vec<u8>>
    where
        T: Serialize;
    
    /// Deserialize from bytes
    fn deserialize<T>(&self, data: &[u8]) -> Result<T>
    where
        T: DeserializeOwned;
    
    /// Serialize to string
    fn serialize_to_string<T>(&self, value: &T) -> Result<String>
    where
        T: Serialize;
    
    /// Deserialize from string
    fn deserialize_from_string<T>(&self, data: &str) -> Result<T>
    where
        T: DeserializeOwned;
}
```

## API Documentation Structure

### Documentation Organization

```
docs/
├── api/
│   ├── README.md                 # API overview
│   ├── core/                     # Core API documentation
│   │   ├── models.md            # Data models
│   │   ├── database.md          # Database operations
│   │   ├── cache.md             # Caching operations
│   │   └── errors.md            # Error handling
│   ├── examples/                 # Usage examples
│   │   ├── basic-usage.md       # Basic operations
│   │   ├── advanced-usage.md    # Advanced patterns
│   │   └── integration.md       # Integration examples
│   └── reference/                # API reference
│       ├── traits.md            # Trait documentation
│       ├── types.md             # Type documentation
│       └── functions.md         # Function documentation
├── architecture/                 # Architecture documentation
│   ├── core-api-design.md       # This document
│   ├── caching-strategy.md      # Caching design
│   └── error-handling.md        # Error handling design
└── guides/                      # User guides
    ├── getting-started.md       # Getting started
    ├── best-practices.md        # Best practices
    └── troubleshooting.md       # Troubleshooting
```

This comprehensive API design provides a solid foundation for building a robust, performant, and maintainable Things 3 integration library that follows Rust best practices and provides excellent developer experience.
