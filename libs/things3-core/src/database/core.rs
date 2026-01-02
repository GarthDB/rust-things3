use crate::{
    database::{mappers::map_task_row, query_builders::TaskUpdateBuilder, validators},
    error::{Result as ThingsResult, ThingsError},
    models::{
        Area, CreateTaskRequest, DeleteChildHandling, Project, Task, TaskStatus, TaskType,
        UpdateTaskRequest,
    },
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolOptions, Row, SqlitePool};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

/// Convert f64 timestamp to i64 safely
pub(crate) fn safe_timestamp_convert(ts_f64: f64) -> i64 {
    // Use try_from to avoid clippy warnings about casting
    if ts_f64.is_finite() && ts_f64 >= 0.0 {
        // Use a reasonable upper bound for timestamps (year 2100)
        let max_timestamp = 4_102_444_800_f64; // 2100-01-01 00:00:00 UTC
        if ts_f64 <= max_timestamp {
            // Convert via string to avoid precision loss warnings
            let ts_str = format!("{:.0}", ts_f64.trunc());
            ts_str.parse::<i64>().unwrap_or(0)
        } else {
            0 // Use epoch if too large
        }
    } else {
        0 // Use epoch if invalid
    }
}

/// Convert Things 3 date value (seconds since 2001-01-01) to NaiveDate
pub(crate) fn things_date_to_naive_date(seconds_since_2001: i64) -> Option<chrono::NaiveDate> {
    use chrono::{TimeZone, Utc};

    if seconds_since_2001 <= 0 {
        return None;
    }

    // Base date: 2001-01-01 00:00:00 UTC
    let base_date = Utc.with_ymd_and_hms(2001, 1, 1, 0, 0, 0).single().unwrap();

    // Add seconds to get the actual date
    let date_time = base_date + chrono::Duration::seconds(seconds_since_2001);

    Some(date_time.date_naive())
}

/// Convert NaiveDate to Things 3 timestamp (seconds since 2001-01-01)
pub fn naive_date_to_things_timestamp(date: NaiveDate) -> i64 {
    use chrono::{NaiveTime, TimeZone, Utc};

    // Base date: 2001-01-01 00:00:00 UTC
    let base_date = Utc.with_ymd_and_hms(2001, 1, 1, 0, 0, 0).single().unwrap();

    // Convert NaiveDate to DateTime at midnight UTC
    let date_time = date
        .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .and_local_timezone(Utc)
        .single()
        .unwrap();

    // Calculate seconds difference
    date_time.timestamp() - base_date.timestamp()
}

/// Serialize tags to Things 3 binary format
/// Note: This is a simplified implementation using JSON
/// The actual Things 3 binary format is proprietary
pub fn serialize_tags_to_blob(tags: &[String]) -> ThingsResult<Vec<u8>> {
    serde_json::to_vec(tags)
        .map_err(|e| ThingsError::unknown(format!("Failed to serialize tags: {e}")))
}

/// Deserialize tags from Things 3 binary format
pub fn deserialize_tags_from_blob(blob: &[u8]) -> ThingsResult<Vec<String>> {
    serde_json::from_slice(blob)
        .map_err(|e| ThingsError::unknown(format!("Failed to deserialize tags: {e}")))
}

/// Convert Things 3 UUID format to standard UUID
/// Things 3 uses base64-like strings, we'll generate a UUID from the hash
pub(crate) fn things_uuid_to_uuid(things_uuid: &str) -> Uuid {
    // For now, create a deterministic UUID from the Things 3 ID
    // This ensures consistent mapping between Things 3 IDs and UUIDs
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    things_uuid.hash(&mut hasher);
    let hash = hasher.finish();

    // Create a UUID from the hash (not cryptographically secure, but consistent)
    // Use proper byte extraction without truncation warnings
    let bytes = [
        ((hash >> 56) & 0xFF) as u8,
        ((hash >> 48) & 0xFF) as u8,
        ((hash >> 40) & 0xFF) as u8,
        ((hash >> 32) & 0xFF) as u8,
        ((hash >> 24) & 0xFF) as u8,
        ((hash >> 16) & 0xFF) as u8,
        ((hash >> 8) & 0xFF) as u8,
        (hash & 0xFF) as u8,
        // Fill remaining bytes with a pattern based on the string
        u8::try_from(things_uuid.len().min(255)).unwrap_or(255),
        things_uuid.chars().next().unwrap_or('0') as u8,
        things_uuid.chars().nth(1).unwrap_or('0') as u8,
        things_uuid.chars().nth(2).unwrap_or('0') as u8,
        things_uuid.chars().nth(3).unwrap_or('0') as u8,
        things_uuid.chars().nth(4).unwrap_or('0') as u8,
        things_uuid.chars().nth(5).unwrap_or('0') as u8,
        things_uuid.chars().nth(6).unwrap_or('0') as u8,
    ];

    Uuid::from_bytes(bytes)
}

impl TaskStatus {
    fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(TaskStatus::Incomplete),
            1 => Some(TaskStatus::Completed),
            2 => Some(TaskStatus::Canceled),
            3 => Some(TaskStatus::Trashed),
            _ => None,
        }
    }
}

impl TaskType {
    fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(TaskType::Todo),
            1 => Some(TaskType::Project),
            2 => Some(TaskType::Heading),
            3 => Some(TaskType::Area),
            _ => None,
        }
    }
}

/// Database connection pool configuration for optimal performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabasePoolConfig {
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections in the pool
    pub min_connections: u32,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Idle timeout for connections
    pub idle_timeout: Duration,
    /// Maximum lifetime of a connection
    pub max_lifetime: Duration,
    /// Test connections before use
    pub test_before_acquire: bool,
    /// SQLite-specific optimizations
    pub sqlite_optimizations: SqliteOptimizations,
}

/// SQLite-specific optimization settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteOptimizations {
    /// Enable WAL mode for better concurrency
    pub enable_wal_mode: bool,
    /// Set synchronous mode (NORMAL, FULL, OFF)
    pub synchronous_mode: String,
    /// Cache size in pages (negative = KB)
    pub cache_size: i32,
    /// Enable foreign key constraints
    pub enable_foreign_keys: bool,
    /// Set journal mode
    pub journal_mode: String,
    /// Set temp store (MEMORY, FILE, DEFAULT)
    pub temp_store: String,
    /// Set mmap size for better performance
    pub mmap_size: i64,
    /// Enable query planner optimizations
    pub enable_query_planner: bool,
}

impl Default for DatabasePoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600), // 10 minutes
            max_lifetime: Duration::from_secs(1800), // 30 minutes
            test_before_acquire: true,
            sqlite_optimizations: SqliteOptimizations::default(),
        }
    }
}

impl Default for SqliteOptimizations {
    fn default() -> Self {
        Self {
            enable_wal_mode: true,
            synchronous_mode: "NORMAL".to_string(),
            cache_size: -20000, // 20MB cache
            enable_foreign_keys: true,
            journal_mode: "WAL".to_string(),
            temp_store: "MEMORY".to_string(),
            mmap_size: 268_435_456, // 256MB
            enable_query_planner: true,
        }
    }
}

/// Connection pool health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolHealthStatus {
    pub is_healthy: bool,
    pub pool_size: u32,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
}

/// Detailed connection pool metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetrics {
    pub pool_size: u32,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub max_connections: u32,
    pub min_connections: u32,
    pub utilization_percentage: f64,
    pub is_healthy: bool,
    pub response_time_ms: u64,
    pub connection_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
}

/// Comprehensive health status including pool and database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveHealthStatus {
    pub overall_healthy: bool,
    pub pool_health: PoolHealthStatus,
    pub pool_metrics: PoolMetrics,
    pub database_stats: DatabaseStats,
    pub timestamp: DateTime<Utc>,
}

/// SQLx-based database implementation for Things 3 data
/// This provides async, Send + Sync compatible database access
#[derive(Debug, Clone)]
pub struct ThingsDatabase {
    pool: SqlitePool,
    config: DatabasePoolConfig,
}

impl ThingsDatabase {
    /// Create a new database connection pool with default configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the database connection fails or if `SQLite` configuration fails
    #[instrument]
    pub async fn new(database_path: &Path) -> ThingsResult<Self> {
        Self::new_with_config(database_path, DatabasePoolConfig::default()).await
    }

    /// Create a new database connection pool with custom configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the database connection fails or if `SQLite` configuration fails
    #[instrument]
    pub async fn new_with_config(
        database_path: &Path,
        config: DatabasePoolConfig,
    ) -> ThingsResult<Self> {
        let database_url = format!("sqlite:{}", database_path.display());

        info!(
            "Connecting to SQLite database at: {} with optimized pool",
            database_url
        );

        // Create optimized connection pool
        let pool = PoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.connect_timeout)
            .idle_timeout(Some(config.idle_timeout))
            .max_lifetime(Some(config.max_lifetime))
            .test_before_acquire(config.test_before_acquire)
            .connect(&database_url)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to connect to database: {e}")))?;

        // Apply SQLite optimizations
        Self::apply_sqlite_optimizations(&pool, &config.sqlite_optimizations).await?;

        info!(
            "Database connection pool established successfully with {} max connections",
            config.max_connections
        );

        Ok(Self { pool, config })
    }

    /// Apply SQLite-specific optimizations
    async fn apply_sqlite_optimizations(
        pool: &SqlitePool,
        optimizations: &SqliteOptimizations,
    ) -> ThingsResult<()> {
        // Set journal mode
        sqlx::query(&format!(
            "PRAGMA journal_mode = {}",
            optimizations.journal_mode
        ))
        .execute(pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to set journal mode: {e}")))?;

        // Set synchronous mode
        sqlx::query(&format!(
            "PRAGMA synchronous = {}",
            optimizations.synchronous_mode
        ))
        .execute(pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to set synchronous mode: {e}")))?;

        // Set cache size
        sqlx::query(&format!("PRAGMA cache_size = {}", optimizations.cache_size))
            .execute(pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to set cache size: {e}")))?;

        // Set foreign keys
        let fk_setting = if optimizations.enable_foreign_keys {
            "ON"
        } else {
            "OFF"
        };
        sqlx::query(&format!("PRAGMA foreign_keys = {fk_setting}"))
            .execute(pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to set foreign keys: {e}")))?;

        // Set temp store
        sqlx::query(&format!("PRAGMA temp_store = {}", optimizations.temp_store))
            .execute(pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to set temp store: {e}")))?;

        // Set mmap size
        sqlx::query(&format!("PRAGMA mmap_size = {}", optimizations.mmap_size))
            .execute(pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to set mmap size: {e}")))?;

        // Enable query planner optimizations
        if optimizations.enable_query_planner {
            sqlx::query("PRAGMA optimize")
                .execute(pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to optimize database: {e}")))?;
        }

        debug!(
            "Applied SQLite optimizations: WAL={}, sync={}, cache={}KB, fk={}, temp={}, mmap={}MB",
            optimizations.enable_wal_mode,
            optimizations.synchronous_mode,
            optimizations.cache_size.abs() / 1024,
            optimizations.enable_foreign_keys,
            optimizations.temp_store,
            optimizations.mmap_size / 1024 / 1024
        );

        Ok(())
    }

    /// Create a new database connection pool from a connection string with default configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the database connection fails or if `SQLite` configuration fails
    #[instrument]
    pub async fn from_connection_string(database_url: &str) -> ThingsResult<Self> {
        Self::from_connection_string_with_config(database_url, DatabasePoolConfig::default()).await
    }

    /// Create a new database connection pool from a connection string with custom configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the database connection fails or if `SQLite` configuration fails
    #[instrument]
    pub async fn from_connection_string_with_config(
        database_url: &str,
        config: DatabasePoolConfig,
    ) -> ThingsResult<Self> {
        info!(
            "Connecting to SQLite database: {} with optimized pool",
            database_url
        );

        // Create optimized connection pool
        let pool = PoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.connect_timeout)
            .idle_timeout(Some(config.idle_timeout))
            .max_lifetime(Some(config.max_lifetime))
            .test_before_acquire(config.test_before_acquire)
            .connect(database_url)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to connect to database: {e}")))?;

        // Apply SQLite optimizations
        Self::apply_sqlite_optimizations(&pool, &config.sqlite_optimizations).await?;

        info!(
            "Database connection pool established successfully with {} max connections",
            config.max_connections
        );

        Ok(Self { pool, config })
    }

    /// Get the underlying connection pool
    #[must_use]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Check if the database is connected
    #[instrument]
    pub async fn is_connected(&self) -> bool {
        match sqlx::query("SELECT 1").fetch_one(&self.pool).await {
            Ok(_) => {
                debug!("Database connection is healthy");
                true
            }
            Err(e) => {
                error!("Database connection check failed: {}", e);
                false
            }
        }
    }

    /// Get connection pool health status
    ///
    /// # Errors
    ///
    /// Returns an error if the health check fails
    #[instrument]
    pub async fn get_pool_health(&self) -> ThingsResult<PoolHealthStatus> {
        let pool_size = self.pool.size();
        let idle_connections = self.pool.num_idle();
        let active_connections = pool_size - u32::try_from(idle_connections).unwrap_or(0);

        // Test a simple query to verify connection health
        let is_healthy = self.is_connected().await;

        Ok(PoolHealthStatus {
            is_healthy,
            pool_size,
            active_connections,
            idle_connections: u32::try_from(idle_connections).unwrap_or(0),
            max_connections: self.config.max_connections,
            min_connections: self.config.min_connections,
            connection_timeout: self.config.connect_timeout,
            idle_timeout: Some(self.config.idle_timeout),
            max_lifetime: Some(self.config.max_lifetime),
        })
    }

    /// Get detailed connection pool metrics
    ///
    /// # Errors
    ///
    /// Returns an error if the metrics collection fails
    #[instrument]
    pub async fn get_pool_metrics(&self) -> ThingsResult<PoolMetrics> {
        let pool_size = self.pool.size();
        let idle_connections = self.pool.num_idle();
        let active_connections = pool_size - u32::try_from(idle_connections).unwrap_or(0);

        // Calculate utilization percentage
        let max_connections = self.config.max_connections;
        let utilization_percentage = if max_connections > 0 {
            (f64::from(active_connections) / f64::from(max_connections)) * 100.0
        } else {
            0.0
        };

        // Test connection response time
        let start_time = std::time::Instant::now();
        let is_connected = self.is_connected().await;
        let response_time_ms = u64::try_from(start_time.elapsed().as_millis()).unwrap_or(0);

        Ok(PoolMetrics {
            pool_size,
            active_connections,
            idle_connections: u32::try_from(idle_connections).unwrap_or(0),
            max_connections,
            min_connections: self.config.min_connections,
            utilization_percentage,
            is_healthy: is_connected,
            response_time_ms,
            connection_timeout: self.config.connect_timeout,
            idle_timeout: Some(self.config.idle_timeout),
            max_lifetime: Some(self.config.max_lifetime),
        })
    }

    /// Perform a comprehensive health check including pool and database
    ///
    /// # Errors
    ///
    /// Returns an error if the health check fails
    #[instrument]
    pub async fn comprehensive_health_check(&self) -> ThingsResult<ComprehensiveHealthStatus> {
        let pool_health = self.get_pool_health().await?;
        let pool_metrics = self.get_pool_metrics().await?;
        let db_stats = self.get_stats().await?;

        let overall_healthy = pool_health.is_healthy && pool_metrics.is_healthy;

        Ok(ComprehensiveHealthStatus {
            overall_healthy,
            pool_health,
            pool_metrics,
            database_stats: db_stats,
            timestamp: Utc::now(),
        })
    }

    /// Get database statistics
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument]
    pub async fn get_stats(&self) -> ThingsResult<DatabaseStats> {
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM TMTask")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to get task count: {e}")))?;

        let project_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM TMTask WHERE type = 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to get project count: {e}")))?;

        let area_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM TMArea")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to get area count: {e}")))?;

        Ok(DatabaseStats {
            task_count: task_count.try_into().unwrap_or(0),
            project_count: project_count.try_into().unwrap_or(0),
            area_count: area_count.try_into().unwrap_or(0),
        })
    }

    /// Get all tasks
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if task data is invalid
    #[instrument]
    pub async fn get_all_tasks(&self) -> ThingsResult<Vec<Task>> {
        let rows = sqlx::query(
            r"
            SELECT 
                uuid, title, status, type, 
                start_date, due_date, 
                project_uuid, area_uuid, 
                notes, tags, 
                created, modified
            FROM TMTask
            ORDER BY created DESC
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch tasks: {e}")))?;

        let mut tasks = Vec::new();
        for row in rows {
            let task = Task {
                uuid: Uuid::parse_str(&row.get::<String, _>("uuid"))
                    .map_err(|e| ThingsError::unknown(format!("Invalid task UUID: {e}")))?,
                title: row.get("title"),
                status: TaskStatus::from_i32(row.get("status")).unwrap_or(TaskStatus::Incomplete),
                task_type: TaskType::from_i32(row.get("type")).unwrap_or(TaskType::Todo),
                start_date: row
                    .get::<Option<String>, _>("start_date")
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                deadline: row
                    .get::<Option<String>, _>("due_date")
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                project_uuid: row
                    .get::<Option<String>, _>("project_uuid")
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                area_uuid: row
                    .get::<Option<String>, _>("area_uuid")
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                parent_uuid: None, // Not available in this query
                notes: row.get("notes"),
                tags: row
                    .get::<Option<String>, _>("tags")
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
                children: Vec::new(), // Not available in this query
                created: DateTime::parse_from_rfc3339(&row.get::<String, _>("created"))
                    .ok()
                    .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc)),
                modified: DateTime::parse_from_rfc3339(&row.get::<String, _>("modified"))
                    .ok()
                    .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc)),
                stop_date: None, // Not available in this query context
            };
            tasks.push(task);
        }

        debug!("Fetched {} tasks", tasks.len());
        Ok(tasks)
    }

    /// Get all projects (from `TMTask` table where type = 1)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if project data is invalid
    #[instrument]
    pub async fn get_all_projects(&self) -> ThingsResult<Vec<Project>> {
        let rows = sqlx::query(
            r"
            SELECT 
                uuid, title, status, 
                area, notes, 
                creationDate, userModificationDate,
                startDate, deadline
            FROM TMTask
            WHERE type = 1 AND trashed = 0
            ORDER BY creationDate DESC
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch projects: {e}")))?;

        let mut projects = Vec::new();
        for row in rows {
            let project = Project {
                uuid: things_uuid_to_uuid(&row.get::<String, _>("uuid")),
                title: row.get("title"),
                status: TaskStatus::from_i32(row.get("status")).unwrap_or(TaskStatus::Incomplete),
                area_uuid: row
                    .get::<Option<String>, _>("area")
                    .map(|s| things_uuid_to_uuid(&s)),
                notes: row.get("notes"),
                deadline: row
                    .get::<Option<i64>, _>("deadline")
                    .and_then(|ts| DateTime::from_timestamp(ts, 0))
                    .map(|dt| dt.date_naive()),
                start_date: row
                    .get::<Option<i64>, _>("startDate")
                    .and_then(|ts| DateTime::from_timestamp(ts, 0))
                    .map(|dt| dt.date_naive()),
                tags: Vec::new(),  // TODO: Load tags separately
                tasks: Vec::new(), // TODO: Load child tasks separately
                created: {
                    let ts_f64 = row.get::<f64, _>("creationDate");
                    let ts = safe_timestamp_convert(ts_f64);
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
                },
                modified: {
                    let ts_f64 = row.get::<f64, _>("userModificationDate");
                    let ts = safe_timestamp_convert(ts_f64);
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
                },
            };
            projects.push(project);
        }

        debug!("Fetched {} projects", projects.len());
        Ok(projects)
    }

    /// Get all areas
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if area data is invalid
    #[instrument]
    pub async fn get_all_areas(&self) -> ThingsResult<Vec<Area>> {
        // Get all areas, not just visible ones (MCP clients may want to see all)
        let rows = sqlx::query(
            r"
            SELECT 
                uuid, title, visible, `index`
             FROM TMArea 
            ORDER BY `index` ASC
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch areas: {e}")))?;

        let mut areas = Vec::new();
        for row in rows {
            let uuid_str: String = row.get("uuid");
            // Try standard UUID first, then fall back to Things UUID format
            let uuid =
                Uuid::parse_str(&uuid_str).unwrap_or_else(|_| things_uuid_to_uuid(&uuid_str));

            let area = Area {
                uuid,
                title: row.get("title"),
                notes: None,          // Notes not stored in TMArea table
                projects: Vec::new(), // TODO: Load projects separately
                tags: Vec::new(),     // TODO: Load tags separately
                created: Utc::now(),  // Creation date not available in TMArea
                modified: Utc::now(), // Modification date not available in TMArea
            };
            areas.push(area);
        }

        debug!("Fetched {} areas", areas.len());
        Ok(areas)
    }

    /// Get tasks by status
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if task data is invalid
    #[instrument]
    pub async fn get_tasks_by_status(&self, status: TaskStatus) -> ThingsResult<Vec<Task>> {
        let status_value = status as i32;
        let rows = sqlx::query(
            r"
            SELECT 
                uuid, title, status, type, 
                start_date, due_date, 
                project_uuid, area_uuid, 
                notes, tags, 
                created, modified
             FROM TMTask 
            WHERE status = ?
            ORDER BY created DESC
            ",
        )
        .bind(status_value)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch tasks by status: {e}")))?;

        let mut tasks = Vec::new();
        for row in rows {
            let task = Task {
                uuid: Uuid::parse_str(&row.get::<String, _>("uuid"))
                    .map_err(|e| ThingsError::unknown(format!("Invalid task UUID: {e}")))?,
                title: row.get("title"),
                status: TaskStatus::from_i32(row.get("status")).unwrap_or(TaskStatus::Incomplete),
                task_type: TaskType::from_i32(row.get("type")).unwrap_or(TaskType::Todo),
                start_date: row
                    .get::<Option<String>, _>("start_date")
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                deadline: row
                    .get::<Option<String>, _>("due_date")
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                project_uuid: row
                    .get::<Option<String>, _>("project_uuid")
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                area_uuid: row
                    .get::<Option<String>, _>("area_uuid")
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                parent_uuid: None, // Not available in this query
                notes: row.get("notes"),
                tags: row
                    .get::<Option<String>, _>("tags")
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
                children: Vec::new(), // Not available in this query
                created: DateTime::parse_from_rfc3339(&row.get::<String, _>("created"))
                    .ok()
                    .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc)),
                modified: DateTime::parse_from_rfc3339(&row.get::<String, _>("modified"))
                    .ok()
                    .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc)),
                stop_date: None, // Not available in this query context
            };
            tasks.push(task);
        }

        debug!("Fetched {} tasks with status {:?}", tasks.len(), status);
        Ok(tasks)
    }

    /// Search tasks by title or notes
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if task data is invalid
    #[instrument]
    pub async fn search_tasks(&self, query: &str) -> ThingsResult<Vec<Task>> {
        let search_pattern = format!("%{query}%");
        let rows = sqlx::query(
            r"
            SELECT 
                uuid, title, status, type, 
                startDate, deadline, stopDate,
                project, area, heading,
                notes, cachedTags, 
                creationDate, userModificationDate
            FROM TMTask
            WHERE (title LIKE ? OR notes LIKE ?) AND trashed = 0 AND type = 0
            ORDER BY creationDate DESC
            ",
        )
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to search tasks: {e}")))?;

        let tasks = rows
            .iter()
            .map(map_task_row)
            .collect::<ThingsResult<Vec<Task>>>()?;

        debug!("Found {} tasks matching query: {}", tasks.len(), query);
        Ok(tasks)
    }

    /// Get inbox tasks (incomplete tasks without project)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if task data is invalid
    #[instrument(skip(self))]
    pub async fn get_inbox(&self, limit: Option<usize>) -> ThingsResult<Vec<Task>> {
        let query = if let Some(limit) = limit {
            format!("SELECT uuid, title, type, status, notes, startDate, deadline, stopDate, creationDate, userModificationDate, project, area, heading, cachedTags FROM TMTask WHERE type = 0 AND status = 0 AND project IS NULL AND trashed = 0 ORDER BY creationDate DESC LIMIT {limit}")
        } else {
            "SELECT uuid, title, type, status, notes, startDate, deadline, stopDate, creationDate, userModificationDate, project, area, heading, cachedTags FROM TMTask WHERE type = 0 AND status = 0 AND project IS NULL AND trashed = 0 ORDER BY creationDate DESC"
                .to_string()
        };

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to fetch inbox tasks: {e}")))?;

        let tasks = rows
            .iter()
            .map(map_task_row)
            .collect::<ThingsResult<Vec<Task>>>()?;

        Ok(tasks)
    }

    /// Get today's tasks (incomplete tasks due today or started today)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if task data is invalid
    ///
    /// # Panics
    ///
    /// Panics if the current date cannot be converted to a valid time with hours, minutes, and seconds
    #[instrument(skip(self))]
    pub async fn get_today(&self, limit: Option<usize>) -> ThingsResult<Vec<Task>> {
        // Things 3 uses the `todayIndex` column to mark tasks that appear in "Today"
        // A task is in "Today" if todayIndex IS NOT NULL AND todayIndex != 0
        let query = if let Some(limit) = limit {
            format!(
                "SELECT uuid, title, type, status, notes, startDate, deadline, stopDate, creationDate, userModificationDate, project, area, heading, cachedTags FROM TMTask WHERE status = 0 AND todayIndex IS NOT NULL AND todayIndex != 0 AND trashed = 0 ORDER BY todayIndex ASC LIMIT {limit}"
            )
        } else {
            "SELECT uuid, title, type, status, notes, startDate, deadline, stopDate, creationDate, userModificationDate, project, area, heading, cachedTags FROM TMTask WHERE status = 0 AND todayIndex IS NOT NULL AND todayIndex != 0 AND trashed = 0 ORDER BY todayIndex ASC".to_string()
        };

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to fetch today's tasks: {e}")))?;

        let tasks = rows
            .iter()
            .map(map_task_row)
            .collect::<ThingsResult<Vec<Task>>>()?;

        Ok(tasks)
    }

    /// Get all projects (alias for `get_all_projects` for compatibility)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if project data is invalid
    #[instrument(skip(self))]
    pub async fn get_projects(&self, limit: Option<usize>) -> ThingsResult<Vec<Project>> {
        let _ = limit; // Currently unused but kept for API compatibility
        self.get_all_projects().await
    }

    /// Get all areas (alias for `get_all_areas` for compatibility)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if area data is invalid
    #[instrument(skip(self))]
    pub async fn get_areas(&self) -> ThingsResult<Vec<Area>> {
        self.get_all_areas().await
    }

    /// Create a new task in the database
    ///
    /// Validates:
    /// - Project UUID exists if provided
    /// - Area UUID exists if provided
    /// - Parent task UUID exists if provided
    ///
    /// Returns the UUID of the created task
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails or if the database insert fails
    #[instrument(skip(self))]
    pub async fn create_task(&self, request: CreateTaskRequest) -> ThingsResult<Uuid> {
        // Generate UUID for new task
        let uuid = Uuid::new_v4();
        let uuid_str = uuid.to_string();

        // Validate referenced entities
        if let Some(project_uuid) = &request.project_uuid {
            validators::validate_project_exists(&self.pool, project_uuid).await?;
        }

        if let Some(area_uuid) = &request.area_uuid {
            validators::validate_area_exists(&self.pool, area_uuid).await?;
        }

        if let Some(parent_uuid) = &request.parent_uuid {
            validators::validate_task_exists(&self.pool, parent_uuid).await?;
        }

        // Convert dates to Things 3 format (seconds since 2001-01-01)
        let start_date_ts = request.start_date.map(naive_date_to_things_timestamp);
        let deadline_ts = request.deadline.map(naive_date_to_things_timestamp);

        // Get current timestamp for creation/modification dates
        let now = Utc::now().timestamp() as f64;

        // Serialize tags to binary format (if provided)
        let cached_tags = request
            .tags
            .as_ref()
            .map(|tags| serialize_tags_to_blob(tags))
            .transpose()?;

        // Insert into TMTask table
        sqlx::query(
            r"
            INSERT INTO TMTask (
                uuid, title, type, status, notes,
                startDate, deadline, project, area, heading,
                cachedTags, creationDate, userModificationDate,
                trashed
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&uuid_str)
        .bind(&request.title)
        .bind(request.task_type.unwrap_or(TaskType::Todo) as i32)
        .bind(request.status.unwrap_or(TaskStatus::Incomplete) as i32)
        .bind(request.notes.as_ref())
        .bind(start_date_ts)
        .bind(deadline_ts)
        .bind(request.project_uuid.map(|u| u.to_string()))
        .bind(request.area_uuid.map(|u| u.to_string()))
        .bind(request.parent_uuid.map(|u| u.to_string()))
        .bind(cached_tags)
        .bind(now)
        .bind(now)
        .bind(0) // not trashed
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to create task: {e}")))?;

        info!("Created task with UUID: {}", uuid);
        Ok(uuid)
    }

    /// Create a new project
    ///
    /// Projects are tasks with type = 1 in the TMTask table
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails or the database insert fails
    #[instrument(skip(self))]
    pub async fn create_project(
        &self,
        request: crate::models::CreateProjectRequest,
    ) -> ThingsResult<Uuid> {
        // Generate UUID for new project
        let uuid = Uuid::new_v4();
        let uuid_str = uuid.to_string();

        // Validate area if provided
        if let Some(area_uuid) = &request.area_uuid {
            validators::validate_area_exists(&self.pool, area_uuid).await?;
        }

        // Convert dates to Things 3 format (seconds since 2001-01-01)
        let start_date_ts = request.start_date.map(naive_date_to_things_timestamp);
        let deadline_ts = request.deadline.map(naive_date_to_things_timestamp);

        // Get current timestamp for creation/modification dates
        let now = Utc::now().timestamp() as f64;

        // Serialize tags to binary format (if provided)
        let cached_tags = request
            .tags
            .as_ref()
            .map(|tags| serialize_tags_to_blob(tags))
            .transpose()?;

        // Insert into TMTask table with type = 1 (project)
        sqlx::query(
            r"
            INSERT INTO TMTask (
                uuid, title, type, status, notes,
                startDate, deadline, project, area, heading,
                cachedTags, creationDate, userModificationDate,
                trashed
            ) VALUES (?, ?, 1, 0, ?, ?, ?, NULL, ?, NULL, ?, ?, ?, 0)
            ",
        )
        .bind(&uuid_str)
        .bind(&request.title)
        .bind(request.notes.as_ref())
        .bind(start_date_ts)
        .bind(deadline_ts)
        .bind(request.area_uuid.map(|u| u.to_string()))
        .bind(cached_tags)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to create project: {e}")))?;

        info!("Created project with UUID: {}", uuid);
        Ok(uuid)
    }

    /// Update an existing task
    ///
    /// Only updates fields that are provided (Some(_))
    /// Validates existence of referenced entities
    ///
    /// # Errors
    ///
    /// Returns an error if the task doesn't exist, validation fails, or the database update fails
    #[instrument(skip(self))]
    pub async fn update_task(&self, request: UpdateTaskRequest) -> ThingsResult<()> {
        // Verify task exists
        validators::validate_task_exists(&self.pool, &request.uuid).await?;

        // Validate referenced entities if being updated
        if let Some(project_uuid) = &request.project_uuid {
            validators::validate_project_exists(&self.pool, project_uuid).await?;
        }

        if let Some(area_uuid) = &request.area_uuid {
            validators::validate_area_exists(&self.pool, area_uuid).await?;
        }

        // Use the TaskUpdateBuilder to construct the query
        let builder = TaskUpdateBuilder::from_request(&request);

        // If no fields to update, just return (modification date will still be updated)
        if builder.is_empty() {
            return Ok(());
        }

        let query_string = builder.build_query_string();
        let mut q = sqlx::query(&query_string);

        // Bind values in the same order as the builder added fields
        if let Some(title) = &request.title {
            q = q.bind(title);
        }

        if let Some(notes) = &request.notes {
            q = q.bind(notes);
        }

        if let Some(start_date) = request.start_date {
            q = q.bind(naive_date_to_things_timestamp(start_date));
        }

        if let Some(deadline) = request.deadline {
            q = q.bind(naive_date_to_things_timestamp(deadline));
        }

        if let Some(status) = request.status {
            q = q.bind(status as i32);
        }

        if let Some(project_uuid) = request.project_uuid {
            q = q.bind(project_uuid.to_string());
        }

        if let Some(area_uuid) = request.area_uuid {
            q = q.bind(area_uuid.to_string());
        }

        if let Some(tags) = &request.tags {
            let cached_tags = serialize_tags_to_blob(tags)?;
            q = q.bind(cached_tags);
        }

        // Bind modification date and UUID (always added by builder)
        let now = Utc::now().timestamp() as f64;
        q = q.bind(now).bind(request.uuid.to_string());

        q.execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to update task: {e}")))?;

        info!("Updated task with UUID: {}", request.uuid);
        Ok(())
    }

    /// Update an existing project
    ///
    /// Only updates fields that are provided (Some(_))
    /// Validates existence and that the entity is a project (type = 1)
    ///
    /// # Errors
    ///
    /// Returns an error if the project doesn't exist, validation fails, or the database update fails
    #[instrument(skip(self))]
    pub async fn update_project(
        &self,
        request: crate::models::UpdateProjectRequest,
    ) -> ThingsResult<()> {
        // Verify project exists (type = 1, trashed = 0)
        validators::validate_project_exists(&self.pool, &request.uuid).await?;

        // Validate area if being updated
        if let Some(area_uuid) = &request.area_uuid {
            validators::validate_area_exists(&self.pool, area_uuid).await?;
        }

        // Build dynamic query using TaskUpdateBuilder
        let mut builder = TaskUpdateBuilder::new();

        // Add fields to update
        if request.title.is_some() {
            builder = builder.add_field("title");
        }
        if request.notes.is_some() {
            builder = builder.add_field("notes");
        }
        if request.start_date.is_some() {
            builder = builder.add_field("startDate");
        }
        if request.deadline.is_some() {
            builder = builder.add_field("deadline");
        }
        if request.area_uuid.is_some() {
            builder = builder.add_field("area");
        }
        if request.tags.is_some() {
            builder = builder.add_field("cachedTags");
        }

        // If nothing to update, return early
        if builder.is_empty() {
            return Ok(());
        }

        // Build query string
        let query_str = builder.build_query_string();
        let mut q = sqlx::query(&query_str);

        // Bind values in the same order they were added to the builder
        if let Some(ref title) = request.title {
            q = q.bind(title);
        }
        if let Some(ref notes) = request.notes {
            q = q.bind(notes);
        }
        if let Some(start_date) = request.start_date {
            q = q.bind(naive_date_to_things_timestamp(start_date));
        }
        if let Some(deadline) = request.deadline {
            q = q.bind(naive_date_to_things_timestamp(deadline));
        }
        if let Some(area_uuid) = request.area_uuid {
            q = q.bind(area_uuid.to_string());
        }
        if let Some(tags) = &request.tags {
            let cached_tags = serialize_tags_to_blob(tags)?;
            q = q.bind(cached_tags);
        }

        // Bind modification date and UUID (always added by builder)
        let now = Utc::now().timestamp() as f64;
        q = q.bind(now).bind(request.uuid.to_string());

        q.execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to update project: {e}")))?;

        info!("Updated project with UUID: {}", request.uuid);
        Ok(())
    }

    /// Get a task by its UUID
    ///
    /// # Errors
    ///
    /// Returns an error if the task does not exist or if the database query fails
    #[instrument(skip(self))]
    pub async fn get_task_by_uuid(&self, uuid: &Uuid) -> ThingsResult<Option<Task>> {
        let row = sqlx::query(
            r"
            SELECT 
                uuid, title, status, type, 
                startDate, deadline, stopDate,
                project, area, heading,
                notes, cachedTags, 
                creationDate, userModificationDate,
                trashed
            FROM TMTask
            WHERE uuid = ?
            ",
        )
        .bind(uuid.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch task: {e}")))?;

        if let Some(row) = row {
            // Check if trashed
            let trashed: i64 = row.get("trashed");
            if trashed == 1 {
                return Ok(None); // Return None for trashed tasks
            }

            // Use the centralized mapper
            let task = map_task_row(&row)?;
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    /// Mark a task as completed
    ///
    /// # Errors
    ///
    /// Returns an error if the task does not exist or if the database update fails
    #[instrument(skip(self))]
    pub async fn complete_task(&self, uuid: &Uuid) -> ThingsResult<()> {
        // Verify task exists
        validators::validate_task_exists(&self.pool, uuid).await?;

        let now = Utc::now().timestamp() as f64;

        sqlx::query(
            "UPDATE TMTask SET status = 1, stopDate = ?, userModificationDate = ? WHERE uuid = ?",
        )
        .bind(now)
        .bind(now)
        .bind(uuid.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to complete task: {e}")))?;

        info!("Completed task with UUID: {}", uuid);
        Ok(())
    }

    /// Mark a completed task as incomplete
    ///
    /// # Errors
    ///
    /// Returns an error if the task does not exist or if the database update fails
    #[instrument(skip(self))]
    pub async fn uncomplete_task(&self, uuid: &Uuid) -> ThingsResult<()> {
        // Verify task exists
        validators::validate_task_exists(&self.pool, uuid).await?;

        let now = Utc::now().timestamp() as f64;

        sqlx::query(
            "UPDATE TMTask SET status = 0, stopDate = NULL, userModificationDate = ? WHERE uuid = ?",
        )
        .bind(now)
        .bind(uuid.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to uncomplete task: {e}")))?;

        info!("Uncompleted task with UUID: {}", uuid);
        Ok(())
    }

    /// Complete a project and optionally handle its child tasks
    ///
    /// # Errors
    ///
    /// Returns an error if the project doesn't exist or if the database update fails
    #[instrument(skip(self))]
    pub async fn complete_project(
        &self,
        uuid: &Uuid,
        child_handling: crate::models::ProjectChildHandling,
    ) -> ThingsResult<()> {
        // Verify project exists
        validators::validate_project_exists(&self.pool, uuid).await?;

        let now = Utc::now().timestamp() as f64;

        // Handle child tasks based on the handling mode
        match child_handling {
            crate::models::ProjectChildHandling::Error => {
                // Check if project has children
                let child_count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM TMTask WHERE project = ? AND trashed = 0",
                )
                .bind(uuid.to_string())
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    ThingsError::unknown(format!("Failed to check for child tasks: {e}"))
                })?;

                if child_count > 0 {
                    return Err(ThingsError::unknown(format!(
                        "Project {} has {} child task(s). Use cascade or orphan mode to complete.",
                        uuid, child_count
                    )));
                }
            }
            crate::models::ProjectChildHandling::Cascade => {
                // Complete all child tasks
                sqlx::query(
                    "UPDATE TMTask SET status = 1, stopDate = ?, userModificationDate = ? WHERE project = ? AND trashed = 0",
                )
                .bind(now)
                .bind(now)
                .bind(uuid.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to complete child tasks: {e}")))?;
            }
            crate::models::ProjectChildHandling::Orphan => {
                // Move child tasks to inbox (set project to NULL)
                sqlx::query(
                    "UPDATE TMTask SET project = NULL, userModificationDate = ? WHERE project = ? AND trashed = 0",
                )
                .bind(now)
                .bind(uuid.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to orphan child tasks: {e}")))?;
            }
        }

        // Complete the project
        sqlx::query(
            "UPDATE TMTask SET status = 1, stopDate = ?, userModificationDate = ? WHERE uuid = ?",
        )
        .bind(now)
        .bind(now)
        .bind(uuid.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to complete project: {e}")))?;

        info!("Completed project with UUID: {}", uuid);
        Ok(())
    }

    /// Soft delete a task (set trashed flag)
    ///
    /// # Errors
    ///
    /// Returns an error if the task does not exist, if child handling fails, or if the database update fails
    #[instrument(skip(self))]
    pub async fn delete_task(
        &self,
        uuid: &Uuid,
        child_handling: DeleteChildHandling,
    ) -> ThingsResult<()> {
        // Verify task exists
        validators::validate_task_exists(&self.pool, uuid).await?;

        // Check for child tasks
        let children = sqlx::query("SELECT uuid FROM TMTask WHERE heading = ? AND trashed = 0")
            .bind(uuid.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to query child tasks: {e}")))?;

        let has_children = !children.is_empty();

        if has_children {
            match child_handling {
                DeleteChildHandling::Error => {
                    return Err(ThingsError::unknown(format!(
                        "Task {} has {} child task(s). Use cascade or orphan mode to delete.",
                        uuid,
                        children.len()
                    )));
                }
                DeleteChildHandling::Cascade => {
                    // Delete all children
                    let now = Utc::now().timestamp() as f64;
                    for child_row in &children {
                        let child_uuid: String = child_row.get("uuid");
                        sqlx::query(
                            "UPDATE TMTask SET trashed = 1, userModificationDate = ? WHERE uuid = ?",
                        )
                        .bind(now)
                        .bind(&child_uuid)
                        .execute(&self.pool)
                        .await
                        .map_err(|e| {
                            ThingsError::unknown(format!("Failed to delete child task: {e}"))
                        })?;
                    }
                    info!("Cascade deleted {} child task(s)", children.len());
                }
                DeleteChildHandling::Orphan => {
                    // Clear parent reference for children
                    let now = Utc::now().timestamp() as f64;
                    for child_row in &children {
                        let child_uuid: String = child_row.get("uuid");
                        sqlx::query(
                            "UPDATE TMTask SET heading = NULL, userModificationDate = ? WHERE uuid = ?",
                        )
                        .bind(now)
                        .bind(&child_uuid)
                        .execute(&self.pool)
                        .await
                        .map_err(|e| {
                            ThingsError::unknown(format!("Failed to orphan child task: {e}"))
                        })?;
                    }
                    info!("Orphaned {} child task(s)", children.len());
                }
            }
        }

        // Delete the parent task
        let now = Utc::now().timestamp() as f64;
        sqlx::query("UPDATE TMTask SET trashed = 1, userModificationDate = ? WHERE uuid = ?")
            .bind(now)
            .bind(uuid.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to delete task: {e}")))?;

        info!("Deleted task with UUID: {}", uuid);
        Ok(())
    }

    /// Soft delete a project and handle its child tasks
    ///
    /// # Errors
    ///
    /// Returns an error if the project doesn't exist, if child handling fails, or if the database update fails
    #[instrument(skip(self))]
    pub async fn delete_project(
        &self,
        uuid: &Uuid,
        child_handling: crate::models::ProjectChildHandling,
    ) -> ThingsResult<()> {
        // Verify project exists
        validators::validate_project_exists(&self.pool, uuid).await?;

        let now = Utc::now().timestamp() as f64;

        // Handle child tasks based on the handling mode
        match child_handling {
            crate::models::ProjectChildHandling::Error => {
                // Check if project has children
                let child_count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM TMTask WHERE project = ? AND trashed = 0",
                )
                .bind(uuid.to_string())
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    ThingsError::unknown(format!("Failed to check for child tasks: {e}"))
                })?;

                if child_count > 0 {
                    return Err(ThingsError::unknown(format!(
                        "Project {} has {} child task(s). Use cascade or orphan mode to delete.",
                        uuid, child_count
                    )));
                }
            }
            crate::models::ProjectChildHandling::Cascade => {
                // Delete all child tasks
                sqlx::query(
                    "UPDATE TMTask SET trashed = 1, userModificationDate = ? WHERE project = ? AND trashed = 0",
                )
                .bind(now)
                .bind(uuid.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to delete child tasks: {e}")))?;
            }
            crate::models::ProjectChildHandling::Orphan => {
                // Move child tasks to inbox (set project to NULL)
                sqlx::query(
                    "UPDATE TMTask SET project = NULL, userModificationDate = ? WHERE project = ? AND trashed = 0",
                )
                .bind(now)
                .bind(uuid.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to orphan child tasks: {e}")))?;
            }
        }

        // Delete the project
        sqlx::query("UPDATE TMTask SET trashed = 1, userModificationDate = ? WHERE uuid = ?")
            .bind(now)
            .bind(uuid.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to delete project: {e}")))?;

        info!("Deleted project with UUID: {}", uuid);
        Ok(())
    }

    /// Create a new area
    ///
    /// # Errors
    ///
    /// Returns an error if the database insert fails
    #[instrument(skip(self))]
    pub async fn create_area(
        &self,
        request: crate::models::CreateAreaRequest,
    ) -> ThingsResult<Uuid> {
        // Generate UUID for new area
        let uuid = Uuid::new_v4();
        let uuid_str = uuid.to_string();

        // Get current timestamp for creation/modification dates
        let now = Utc::now().timestamp() as f64;

        // Calculate next index (max + 1)
        let max_index: Option<i64> = sqlx::query_scalar("SELECT MAX(`index`) FROM TMArea")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to get max area index: {e}")))?;

        let next_index = max_index.unwrap_or(-1) + 1;

        // Insert into TMArea table
        sqlx::query(
            r"
            INSERT INTO TMArea (
                uuid, title, visible, `index`,
                creationDate, userModificationDate
            ) VALUES (?, ?, 1, ?, ?, ?)
            ",
        )
        .bind(&uuid_str)
        .bind(&request.title)
        .bind(next_index)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to create area: {e}")))?;

        info!("Created area with UUID: {}", uuid);
        Ok(uuid)
    }

    /// Update an existing area
    ///
    /// # Errors
    ///
    /// Returns an error if the area doesn't exist or if the database update fails
    #[instrument(skip(self))]
    pub async fn update_area(&self, request: crate::models::UpdateAreaRequest) -> ThingsResult<()> {
        // Verify area exists
        validators::validate_area_exists(&self.pool, &request.uuid).await?;

        let now = Utc::now().timestamp() as f64;

        sqlx::query("UPDATE TMArea SET title = ?, userModificationDate = ? WHERE uuid = ?")
            .bind(&request.title)
            .bind(now)
            .bind(request.uuid.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to update area: {e}")))?;

        info!("Updated area with UUID: {}", request.uuid);
        Ok(())
    }

    /// Delete an area
    ///
    /// Hard delete (areas don't have a trashed field)
    /// Orphans all projects in the area by setting their area to NULL
    ///
    /// # Errors
    ///
    /// Returns an error if the area doesn't exist or if the database delete fails
    #[instrument(skip(self))]
    pub async fn delete_area(&self, uuid: &Uuid) -> ThingsResult<()> {
        // Verify area exists
        validators::validate_area_exists(&self.pool, uuid).await?;

        let now = Utc::now().timestamp() as f64;

        // Orphan all projects in this area (set area to NULL)
        sqlx::query(
            "UPDATE TMTask SET area = NULL, userModificationDate = ? WHERE area = ? AND type = 1 AND trashed = 0",
        )
        .bind(now)
        .bind(uuid.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to orphan projects in area: {e}")))?;

        // Delete the area (hard delete)
        sqlx::query("DELETE FROM TMArea WHERE uuid = ?")
            .bind(uuid.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to delete area: {e}")))?;

        info!("Deleted area with UUID: {}", uuid);
        Ok(())
    }
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub task_count: u64,
    pub project_count: u64,
    pub area_count: u64,
}

impl DatabaseStats {
    #[must_use]
    pub fn total_items(&self) -> u64 {
        self.task_count + self.project_count + self.area_count
    }
}

/// Get the default Things 3 database path
///
/// # Examples
///
/// ```
/// use things3_core::get_default_database_path;
///
/// let path = get_default_database_path();
/// assert!(!path.to_string_lossy().is_empty());
/// assert!(path.to_string_lossy().contains("Library"));
/// ```
#[must_use]
pub fn get_default_database_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
    PathBuf::from(format!(
        "{home}/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{NamedTempFile, TempDir};

    #[tokio::test]
    async fn test_database_connection() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // This will fail because the database doesn't exist yet
        // In a real implementation, we'd need to create the schema first
        let result = super::ThingsDatabase::new(&db_path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connection_string() {
        let result = super::ThingsDatabase::from_connection_string("sqlite::memory:").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_task_status_from_i32() {
        assert_eq!(TaskStatus::from_i32(0), Some(TaskStatus::Incomplete));
        assert_eq!(TaskStatus::from_i32(1), Some(TaskStatus::Completed));
        assert_eq!(TaskStatus::from_i32(2), Some(TaskStatus::Canceled));
        assert_eq!(TaskStatus::from_i32(3), Some(TaskStatus::Trashed));
        assert_eq!(TaskStatus::from_i32(4), None);
        assert_eq!(TaskStatus::from_i32(-1), None);
    }

    #[test]
    fn test_task_type_from_i32() {
        assert_eq!(TaskType::from_i32(0), Some(TaskType::Todo));
        assert_eq!(TaskType::from_i32(1), Some(TaskType::Project));
        assert_eq!(TaskType::from_i32(2), Some(TaskType::Heading));
        assert_eq!(TaskType::from_i32(3), Some(TaskType::Area));
        assert_eq!(TaskType::from_i32(4), None);
        assert_eq!(TaskType::from_i32(-1), None);
    }

    #[test]
    fn test_database_stats_total_items() {
        let stats = DatabaseStats {
            task_count: 10,
            project_count: 5,
            area_count: 3,
        };
        assert_eq!(stats.total_items(), 18);

        let empty_stats = DatabaseStats {
            task_count: 0,
            project_count: 0,
            area_count: 0,
        };
        assert_eq!(empty_stats.total_items(), 0);
    }

    #[test]
    fn test_database_pool_config_default() {
        let config = DatabasePoolConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Duration::from_secs(600));
        assert_eq!(config.max_lifetime, Duration::from_secs(1800));
        assert!(config.test_before_acquire);
    }

    #[test]
    fn test_sqlite_optimizations_default() {
        let opts = SqliteOptimizations::default();
        assert!(opts.enable_wal_mode);
        assert_eq!(opts.cache_size, -20000);
        assert_eq!(opts.synchronous_mode, "NORMAL".to_string());
        assert_eq!(opts.temp_store, "MEMORY".to_string());
        assert_eq!(opts.journal_mode, "WAL".to_string());
        assert_eq!(opts.mmap_size, 268_435_456);
        assert!(opts.enable_foreign_keys);
        assert!(opts.enable_query_planner);
    }

    #[test]
    fn test_pool_health_status_creation() {
        let status = PoolHealthStatus {
            is_healthy: true,
            pool_size: 8,
            active_connections: 5,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };
        assert!(status.is_healthy);
        assert_eq!(status.active_connections, 5);
        assert_eq!(status.idle_connections, 3);
        assert_eq!(status.pool_size, 8);
    }

    #[test]
    fn test_pool_metrics_creation() {
        let metrics = PoolMetrics {
            pool_size: 8,
            active_connections: 5,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            utilization_percentage: 80.0,
            is_healthy: true,
            response_time_ms: 50,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };
        assert!(metrics.is_healthy);
        assert_eq!(metrics.pool_size, 8);
        assert_eq!(metrics.active_connections, 5);
        assert_eq!(metrics.idle_connections, 3);
        assert!((metrics.utilization_percentage - 80.0).abs() < f64::EPSILON);
        assert_eq!(metrics.response_time_ms, 50);
    }

    #[test]
    fn test_comprehensive_health_status_creation() {
        let pool_health = PoolHealthStatus {
            is_healthy: true,
            pool_size: 8,
            active_connections: 5,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };

        let pool_metrics = PoolMetrics {
            pool_size: 8,
            active_connections: 5,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            utilization_percentage: 80.0,
            is_healthy: true,
            response_time_ms: 50,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };

        let db_stats = DatabaseStats {
            task_count: 50,
            project_count: 10,
            area_count: 5,
        };

        let health_status = ComprehensiveHealthStatus {
            overall_healthy: true,
            pool_health,
            pool_metrics,
            database_stats: db_stats,
            timestamp: Utc::now(),
        };

        assert!(health_status.overall_healthy);
        assert_eq!(health_status.database_stats.total_items(), 65);
    }

    #[test]
    fn test_safe_timestamp_convert_edge_cases() {
        // Test normal timestamp
        assert_eq!(safe_timestamp_convert(1_609_459_200.0), 1_609_459_200); // 2021-01-01

        // Test zero
        assert_eq!(safe_timestamp_convert(0.0), 0);

        // Test negative (should return 0)
        assert_eq!(safe_timestamp_convert(-1.0), 0);

        // Test infinity (should return 0)
        assert_eq!(safe_timestamp_convert(f64::INFINITY), 0);

        // Test NaN (should return 0)
        assert_eq!(safe_timestamp_convert(f64::NAN), 0);

        // Test very large timestamp (should return 0)
        assert_eq!(safe_timestamp_convert(5_000_000_000.0), 0);

        // Test max valid timestamp
        let max_timestamp = 4_102_444_800_f64; // 2100-01-01
        assert_eq!(safe_timestamp_convert(max_timestamp), 4_102_444_800);
    }

    #[test]
    fn test_things_uuid_to_uuid_consistency() {
        // Test consistent UUID generation
        let things_id = "test-id-123";
        let uuid1 = things_uuid_to_uuid(things_id);
        let uuid2 = things_uuid_to_uuid(things_id);
        assert_eq!(uuid1, uuid2, "UUIDs should be consistent for same input");

        // Test different inputs produce different UUIDs
        let uuid3 = things_uuid_to_uuid("different-id");
        assert_ne!(
            uuid1, uuid3,
            "Different inputs should produce different UUIDs"
        );

        // Test empty string
        let uuid_empty = things_uuid_to_uuid("");
        assert!(!uuid_empty.to_string().is_empty());

        // Test very long string
        let long_string = "a".repeat(1000);
        let uuid_long = things_uuid_to_uuid(&long_string);
        assert!(!uuid_long.to_string().is_empty());
    }

    #[test]
    fn test_task_status_from_i32_all_variants() {
        assert_eq!(TaskStatus::from_i32(0), Some(TaskStatus::Incomplete));
        assert_eq!(TaskStatus::from_i32(1), Some(TaskStatus::Completed));
        assert_eq!(TaskStatus::from_i32(2), Some(TaskStatus::Canceled));
        assert_eq!(TaskStatus::from_i32(3), Some(TaskStatus::Trashed));
        assert_eq!(TaskStatus::from_i32(999), None);
        assert_eq!(TaskStatus::from_i32(-1), None);
    }

    #[test]
    fn test_task_type_from_i32_all_variants() {
        assert_eq!(TaskType::from_i32(0), Some(TaskType::Todo));
        assert_eq!(TaskType::from_i32(1), Some(TaskType::Project));
        assert_eq!(TaskType::from_i32(2), Some(TaskType::Heading));
        assert_eq!(TaskType::from_i32(3), Some(TaskType::Area));
        assert_eq!(TaskType::from_i32(999), None);
        assert_eq!(TaskType::from_i32(-1), None);
    }

    #[test]
    fn test_database_pool_config_default_values() {
        let config = DatabasePoolConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Duration::from_secs(600));
        assert_eq!(config.max_lifetime, Duration::from_secs(1800));
        assert!(config.test_before_acquire);
    }

    #[test]
    fn test_database_stats_total_items_calculation() {
        let stats = DatabaseStats {
            task_count: 10,
            project_count: 5,
            area_count: 3,
        };
        assert_eq!(stats.total_items(), 18); // 10 + 5 + 3

        // Test with zero values
        let empty_stats = DatabaseStats {
            task_count: 0,
            project_count: 0,
            area_count: 0,
        };
        assert_eq!(empty_stats.total_items(), 0);
    }

    #[test]
    fn test_pool_health_status_creation_comprehensive() {
        let status = PoolHealthStatus {
            is_healthy: true,
            pool_size: 8,
            active_connections: 2,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };
        assert!(status.is_healthy);
        assert_eq!(status.pool_size, 8);
        assert_eq!(status.max_connections, 10);
    }

    #[test]
    fn test_pool_metrics_creation_comprehensive() {
        let metrics = PoolMetrics {
            pool_size: 8,
            active_connections: 5,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            utilization_percentage: 80.0,
            is_healthy: true,
            response_time_ms: 50,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };
        assert_eq!(metrics.pool_size, 8);
        assert_eq!(metrics.response_time_ms, 50);
        assert!(metrics.is_healthy);
    }

    #[test]
    fn test_comprehensive_health_status_creation_full() {
        let pool_health = PoolHealthStatus {
            is_healthy: true,
            pool_size: 8,
            active_connections: 2,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };

        let pool_metrics = PoolMetrics {
            pool_size: 8,
            active_connections: 5,
            idle_connections: 3,
            max_connections: 10,
            min_connections: 1,
            utilization_percentage: 80.0,
            is_healthy: true,
            response_time_ms: 50,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };

        let database_stats = DatabaseStats {
            task_count: 100,
            project_count: 20,
            area_count: 5,
        };

        let status = ComprehensiveHealthStatus {
            overall_healthy: true,
            pool_health,
            pool_metrics,
            database_stats,
            timestamp: Utc::now(),
        };

        assert!(status.overall_healthy);
        assert_eq!(status.database_stats.total_items(), 125);
    }

    #[test]
    fn test_sqlite_optimizations_default_values() {
        let opts = SqliteOptimizations::default();
        assert!(opts.enable_wal_mode);
        assert!(opts.enable_foreign_keys);
        assert_eq!(opts.cache_size, -20000);
        assert_eq!(opts.temp_store, "MEMORY");
        assert_eq!(opts.mmap_size, 268_435_456);
        assert_eq!(opts.synchronous_mode, "NORMAL");
        assert_eq!(opts.journal_mode, "WAL");
    }

    #[test]
    fn test_get_default_database_path_format() {
        let path = get_default_database_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("Things Database.thingsdatabase"));
        assert!(path_str.contains("main.sqlite"));
        assert!(path_str.contains("Library/Group Containers"));
    }

    #[tokio::test]
    async fn test_database_new_with_config() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        crate::test_utils::create_test_database(db_path)
            .await
            .unwrap();

        let config = DatabasePoolConfig {
            max_connections: 5,
            min_connections: 1,
            connect_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(900),
            test_before_acquire: true,
            sqlite_optimizations: SqliteOptimizations::default(),
        };

        let database = ThingsDatabase::new_with_config(db_path, config)
            .await
            .unwrap();
        let pool = database.pool();
        assert!(!pool.is_closed());
    }

    #[tokio::test]
    async fn test_database_error_handling_invalid_path() {
        // Test with non-existent database path
        let result = ThingsDatabase::new(Path::new("/non/existent/path.db")).await;
        assert!(result.is_err(), "Should fail with non-existent path");
    }

    #[tokio::test]
    async fn test_database_get_stats() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        crate::test_utils::create_test_database(db_path)
            .await
            .unwrap();
        let database = ThingsDatabase::new(db_path).await.unwrap();

        let stats = database.get_stats().await.unwrap();
        assert!(stats.task_count > 0, "Should have test tasks");
        assert!(stats.area_count > 0, "Should have test areas");
        assert!(stats.total_items() > 0, "Should have total items");
    }

    #[tokio::test]
    async fn test_database_comprehensive_health_check() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        crate::test_utils::create_test_database(db_path)
            .await
            .unwrap();
        let database = ThingsDatabase::new(db_path).await.unwrap();

        let health = database.comprehensive_health_check().await.unwrap();
        assert!(health.overall_healthy, "Database should be healthy");
        assert!(health.pool_health.is_healthy, "Pool should be healthy");
        assert!(
            health.pool_metrics.is_healthy,
            "Pool metrics should be healthy"
        );
    }

    // ============================================================================
    // Date Conversion Tests - Edge Cases
    // ============================================================================

    #[test]
    fn test_things_date_negative_returns_none() {
        // Negative values should return None
        assert_eq!(things_date_to_naive_date(-1), None);
        assert_eq!(things_date_to_naive_date(-100), None);
        assert_eq!(things_date_to_naive_date(i64::MIN), None);
    }

    #[test]
    fn test_things_date_zero_returns_none() {
        // Zero should return None (no date set)
        assert_eq!(things_date_to_naive_date(0), None);
    }

    #[test]
    fn test_things_date_boundary_2001() {
        use chrono::Datelike;
        // 1 second after 2001-01-01 00:00:00 should be 2001-01-01
        let result = things_date_to_naive_date(1);
        assert!(result.is_some());

        let date = result.unwrap();
        assert_eq!(date.year(), 2001);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 1);
    }

    #[test]
    fn test_things_date_one_day() {
        use chrono::Datelike;
        // 86400 seconds = 1 day (60 * 60 * 24), should be 2001-01-02
        let seconds_per_day = 86400i64;
        let result = things_date_to_naive_date(seconds_per_day);
        assert!(result.is_some());

        let date = result.unwrap();
        assert_eq!(date.year(), 2001);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 2);
    }

    #[test]
    fn test_things_date_one_year() {
        use chrono::Datelike;
        // ~365 days should be around 2002-01-01 (365 days * 86400 seconds/day)
        let seconds_per_year = 365 * 86400i64;
        let result = things_date_to_naive_date(seconds_per_year);
        assert!(result.is_some());

        let date = result.unwrap();
        assert_eq!(date.year(), 2002);
    }

    #[test]
    fn test_things_date_current_era() {
        use chrono::Datelike;
        // Test a date in the current era (2024)
        // Days from 2001-01-01 to 2024-01-01 = ~8401 days
        // Calculation: (2024-2001) * 365 + leap days (2004, 2008, 2012, 2016, 2020) = 23 * 365 + 5 = 8400
        let days_to_2024 = 8401i64;
        let seconds_to_2024 = days_to_2024 * 86400;

        let result = things_date_to_naive_date(seconds_to_2024);
        assert!(result.is_some());

        let date = result.unwrap();
        assert_eq!(date.year(), 2024);
    }

    #[test]
    fn test_things_date_leap_year() {
        use chrono::{Datelike, TimeZone, Utc};
        // Test Feb 29, 2004 (leap year)
        // Days from 2001-01-01 to 2004-02-29
        let base_date = Utc.with_ymd_and_hms(2001, 1, 1, 0, 0, 0).single().unwrap();
        let target_date = Utc.with_ymd_and_hms(2004, 2, 29, 0, 0, 0).single().unwrap();
        let seconds_diff = (target_date - base_date).num_seconds();

        let result = things_date_to_naive_date(seconds_diff);
        assert!(result.is_some());

        let date = result.unwrap();
        assert_eq!(date.year(), 2004);
        assert_eq!(date.month(), 2);
        assert_eq!(date.day(), 29);
    }

    // ============================================================================
    // UUID Conversion Tests
    // ============================================================================

    #[test]
    fn test_uuid_conversion_consistency() {
        // Same input should always produce same UUID
        let input = "ABC123";
        let uuid1 = things_uuid_to_uuid(input);
        let uuid2 = things_uuid_to_uuid(input);

        assert_eq!(uuid1, uuid2);
    }

    #[test]
    fn test_uuid_conversion_uniqueness() {
        // Different inputs should produce different UUIDs
        let uuid1 = things_uuid_to_uuid("ABC123");
        let uuid2 = things_uuid_to_uuid("ABC124");
        let uuid3 = things_uuid_to_uuid("XYZ789");

        assert_ne!(uuid1, uuid2);
        assert_ne!(uuid1, uuid3);
        assert_ne!(uuid2, uuid3);
    }

    #[test]
    fn test_uuid_conversion_empty_string() {
        // Empty string should still produce a valid UUID
        let uuid = things_uuid_to_uuid("");
        assert!(!uuid.to_string().is_empty());
    }

    #[test]
    fn test_uuid_conversion_special_characters() {
        // Special characters should be handled
        let uuid1 = things_uuid_to_uuid("test-with-dashes");
        let uuid2 = things_uuid_to_uuid("test_with_underscores");
        let uuid3 = things_uuid_to_uuid("test.with.dots");

        // All should be valid and different
        assert_ne!(uuid1, uuid2);
        assert_ne!(uuid1, uuid3);
        assert_ne!(uuid2, uuid3);
    }

    // ============================================================================
    // Timestamp Conversion Tests
    // ============================================================================

    #[test]
    fn test_safe_timestamp_convert_normal_values() {
        // Normal timestamp values should convert correctly
        let ts = 1_700_000_000.0; // Around 2023
        let result = safe_timestamp_convert(ts);
        assert_eq!(result, 1_700_000_000);
    }

    #[test]
    fn test_safe_timestamp_convert_zero() {
        // Zero should return zero
        assert_eq!(safe_timestamp_convert(0.0), 0);
    }

    #[test]
    fn test_safe_timestamp_convert_negative() {
        // Negative values should return zero (safe fallback)
        assert_eq!(safe_timestamp_convert(-1.0), 0);
        assert_eq!(safe_timestamp_convert(-1000.0), 0);
    }

    #[test]
    fn test_safe_timestamp_convert_infinity() {
        // Infinity should return zero (safe fallback)
        assert_eq!(safe_timestamp_convert(f64::INFINITY), 0);
        assert_eq!(safe_timestamp_convert(f64::NEG_INFINITY), 0);
    }

    #[test]
    fn test_safe_timestamp_convert_nan() {
        // NaN should return zero (safe fallback)
        assert_eq!(safe_timestamp_convert(f64::NAN), 0);
    }

    #[test]
    fn test_date_roundtrip_known_dates() {
        use chrono::{Datelike, TimeZone, Utc};
        // Test roundtrip conversion for known dates
        // Note: Starting from 2001-01-02 because 2001-01-01 is the base date (0 seconds)
        // and things_date_to_naive_date returns None for values <= 0
        let test_cases = vec![
            (2001, 1, 2), // Start from day 2 since day 1 is the base (0 seconds)
            (2010, 6, 15),
            (2020, 12, 31),
            (2024, 2, 29), // Leap year
            (2025, 7, 4),
        ];

        for (year, month, day) in test_cases {
            let base_date = Utc.with_ymd_and_hms(2001, 1, 1, 0, 0, 0).single().unwrap();
            let target_date = Utc
                .with_ymd_and_hms(year, month, day, 0, 0, 0)
                .single()
                .unwrap();
            let seconds = (target_date - base_date).num_seconds();

            let converted = things_date_to_naive_date(seconds);
            assert!(
                converted.is_some(),
                "Failed to convert {}-{:02}-{:02}",
                year,
                month,
                day
            );

            let result_date = converted.unwrap();
            assert_eq!(
                result_date.year(),
                year,
                "Year mismatch for {}-{:02}-{:02}",
                year,
                month,
                day
            );
            assert_eq!(
                result_date.month(),
                month,
                "Month mismatch for {}-{:02}-{:02}",
                year,
                month,
                day
            );
            assert_eq!(
                result_date.day(),
                day,
                "Day mismatch for {}-{:02}-{:02}",
                year,
                month,
                day
            );
        }
    }
}
