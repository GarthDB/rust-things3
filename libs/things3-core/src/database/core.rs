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

    /// Search completed tasks in the logbook
    ///
    /// Returns completed tasks matching the provided filters.
    /// All filters are optional and can be combined.
    ///
    /// # Parameters
    ///
    /// - `search_text`: Search in task titles and notes (case-insensitive)
    /// - `from_date`: Start date for completion date range
    /// - `to_date`: End date for completion date range
    /// - `project_uuid`: Filter by project UUID
    /// - `area_uuid`: Filter by area UUID
    /// - `tags`: Filter by tags (all tags must match)
    /// - `limit`: Maximum number of results (default: 50)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if task data is invalid
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip(self))]
    pub async fn search_logbook(
        &self,
        search_text: Option<String>,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
        project_uuid: Option<Uuid>,
        area_uuid: Option<Uuid>,
        tags: Option<Vec<String>>,
        limit: Option<u32>,
    ) -> ThingsResult<Vec<Task>> {
        // Apply limit
        let result_limit = limit.unwrap_or(50).min(500);

        // Build and execute query based on filters
        let rows = if let Some(ref text) = search_text {
            let pattern = format!("%{text}%");
            let mut q = String::from(
                "SELECT uuid, title, status, type, startDate, deadline, stopDate, project, area, heading, notes, cachedTags, creationDate, userModificationDate FROM TMTask WHERE status = 1 AND trashed = 0 AND type = 0",
            );
            q.push_str(" AND (title LIKE ? OR notes LIKE ?)");

            if let Some(date) = from_date {
                // stopDate is stored as Unix timestamp (seconds since 1970-01-01)
                let date_time = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let timestamp = date_time.timestamp() as f64;
                q.push_str(&format!(" AND stopDate >= {}", timestamp));
            }

            if let Some(date) = to_date {
                // Include tasks completed on to_date by adding 1 day
                let end_date = date + chrono::Duration::days(1);
                let date_time = end_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let timestamp = date_time.timestamp() as f64;
                q.push_str(&format!(" AND stopDate < {}", timestamp));
            }

            if let Some(uuid) = project_uuid {
                q.push_str(&format!(" AND project = '{}'", uuid));
            }

            if let Some(uuid) = area_uuid {
                q.push_str(&format!(" AND area = '{}'", uuid));
            }

            q.push_str(&format!(" ORDER BY stopDate DESC LIMIT {result_limit}"));

            sqlx::query(&q)
                .bind(&pattern)
                .bind(&pattern)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to search logbook: {e}")))?
        } else {
            let mut q = String::from(
                "SELECT uuid, title, status, type, startDate, deadline, stopDate, project, area, heading, notes, cachedTags, creationDate, userModificationDate FROM TMTask WHERE status = 1 AND trashed = 0 AND type = 0",
            );

            if let Some(date) = from_date {
                // stopDate is stored as Unix timestamp (seconds since 1970-01-01)
                let date_time = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let timestamp = date_time.timestamp() as f64;
                q.push_str(&format!(" AND stopDate >= {}", timestamp));
            }

            if let Some(date) = to_date {
                // Include tasks completed on to_date by adding 1 day
                let end_date = date + chrono::Duration::days(1);
                let date_time = end_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let timestamp = date_time.timestamp() as f64;
                q.push_str(&format!(" AND stopDate < {}", timestamp));
            }

            if let Some(uuid) = project_uuid {
                q.push_str(&format!(" AND project = '{}'", uuid));
            }

            if let Some(uuid) = area_uuid {
                q.push_str(&format!(" AND area = '{}'", uuid));
            }

            q.push_str(&format!(" ORDER BY stopDate DESC LIMIT {result_limit}"));

            sqlx::query(&q)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to search logbook: {e}")))?
        };

        // Filter by tags if provided
        let mut tasks = rows
            .iter()
            .map(map_task_row)
            .collect::<ThingsResult<Vec<Task>>>()?;

        if let Some(ref filter_tags) = tags {
            if !filter_tags.is_empty() {
                tasks.retain(|task| {
                    // Check if task has all required tags
                    filter_tags
                        .iter()
                        .all(|filter_tag| task.tags.contains(filter_tag))
                });
            }
        }

        debug!("Found {} completed tasks in logbook", tasks.len());
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
        // Validate date range (deadline must be >= start_date)
        crate::database::validate_date_range(request.start_date, request.deadline)?;

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
        // Validate date range (deadline must be >= start_date)
        crate::database::validate_date_range(request.start_date, request.deadline)?;

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

        // Validate dates if either is being updated
        if request.start_date.is_some() || request.deadline.is_some() {
            // Get current task to merge dates
            if let Some(current_task) = self.get_task_by_uuid(&request.uuid).await? {
                let final_start = request.start_date.or(current_task.start_date);
                let final_deadline = request.deadline.or(current_task.deadline);
                crate::database::validate_date_range(final_start, final_deadline)?;
            }
        }

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

        // Validate dates if either is being updated
        if request.start_date.is_some() || request.deadline.is_some() {
            // Fetch current project to merge dates
            let current_projects = self.get_all_projects().await?;
            if let Some(current_project) = current_projects.iter().find(|p| p.uuid == request.uuid)
            {
                let final_start = request.start_date.or(current_project.start_date);
                let final_deadline = request.deadline.or(current_project.deadline);
                crate::database::validate_date_range(final_start, final_deadline)?;
            }
        }

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

    // ========================================================================
    // TAG OPERATIONS (with smart duplicate prevention)
    // ========================================================================

    /// Find a tag by normalized title (exact match, case-insensitive)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn find_tag_by_normalized_title(
        &self,
        normalized: &str,
    ) -> ThingsResult<Option<crate::models::Tag>> {
        let row = sqlx::query(
            "SELECT uuid, title, shortcut, parent, creationDate, userModificationDate, usedDate 
             FROM TMTag 
             WHERE LOWER(title) = LOWER(?)",
        )
        .bind(normalized)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to find tag by title: {e}")))?;

        if let Some(row) = row {
            let uuid_str: String = row.get("uuid");
            let uuid =
                Uuid::parse_str(&uuid_str).unwrap_or_else(|_| things_uuid_to_uuid(&uuid_str));
            let title: String = row.get("title");
            let shortcut: Option<String> = row.get("shortcut");
            let parent_str: Option<String> = row.get("parent");
            let parent_uuid =
                parent_str.map(|s| Uuid::parse_str(&s).unwrap_or_else(|_| things_uuid_to_uuid(&s)));

            let creation_ts: f64 = row.get("creationDate");
            let created = {
                let ts = safe_timestamp_convert(creation_ts);
                DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
            };

            let modification_ts: f64 = row.get("userModificationDate");
            let modified = {
                let ts = safe_timestamp_convert(modification_ts);
                DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
            };

            let used_ts: Option<f64> = row.get("usedDate");
            let last_used = used_ts.and_then(|ts| {
                let ts_i64 = safe_timestamp_convert(ts);
                DateTime::from_timestamp(ts_i64, 0)
            });

            // Count usage by querying tasks with this tag
            let usage_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM TMTask 
                 WHERE cachedTags IS NOT NULL 
                 AND json_extract(cachedTags, '$') LIKE ?
                 AND trashed = 0",
            )
            .bind(format!("%\"{}\"%", title))
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

            Ok(Some(crate::models::Tag {
                uuid,
                title,
                shortcut,
                parent_uuid,
                created,
                modified,
                usage_count: usage_count as u32,
                last_used,
            }))
        } else {
            Ok(None)
        }
    }

    /// Find tags similar to the given title using fuzzy matching
    ///
    /// Returns tags sorted by similarity score (highest first)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn find_similar_tags(
        &self,
        title: &str,
        min_similarity: f32,
    ) -> ThingsResult<Vec<crate::models::TagMatch>> {
        use crate::database::tag_utils::{calculate_similarity, get_match_type};

        // Get all tags
        let all_tags = self.get_all_tags().await?;

        // Calculate similarity for each tag
        let mut matches: Vec<crate::models::TagMatch> = all_tags
            .into_iter()
            .filter_map(|tag| {
                let similarity = calculate_similarity(title, &tag.title);
                if similarity >= min_similarity {
                    let match_type = get_match_type(title, &tag.title, min_similarity);
                    Some(crate::models::TagMatch {
                        tag,
                        similarity_score: similarity,
                        match_type,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by similarity score (highest first)
        matches.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(matches)
    }

    /// Search tags by partial title match
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn search_tags(&self, query: &str) -> ThingsResult<Vec<crate::models::Tag>> {
        let rows = sqlx::query(
            "SELECT uuid, title, shortcut, parent, creationDate, userModificationDate, usedDate 
             FROM TMTag 
             WHERE title LIKE ? 
             ORDER BY title",
        )
        .bind(format!("%{}%", query))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to search tags: {e}")))?;

        let mut tags = Vec::new();
        for row in rows {
            let uuid_str: String = row.get("uuid");
            let uuid =
                Uuid::parse_str(&uuid_str).unwrap_or_else(|_| things_uuid_to_uuid(&uuid_str));
            let title: String = row.get("title");
            let shortcut: Option<String> = row.get("shortcut");
            let parent_str: Option<String> = row.get("parent");
            let parent_uuid =
                parent_str.map(|s| Uuid::parse_str(&s).unwrap_or_else(|_| things_uuid_to_uuid(&s)));

            let creation_ts: f64 = row.get("creationDate");
            let created = {
                let ts = safe_timestamp_convert(creation_ts);
                DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
            };

            let modification_ts: f64 = row.get("userModificationDate");
            let modified = {
                let ts = safe_timestamp_convert(modification_ts);
                DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
            };

            let used_ts: Option<f64> = row.get("usedDate");
            let last_used = used_ts.and_then(|ts| {
                let ts_i64 = safe_timestamp_convert(ts);
                DateTime::from_timestamp(ts_i64, 0)
            });

            // Count usage
            let usage_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM TMTask 
                 WHERE cachedTags IS NOT NULL 
                 AND json_extract(cachedTags, '$') LIKE ?
                 AND trashed = 0",
            )
            .bind(format!("%\"{}\"%", title))
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

            tags.push(crate::models::Tag {
                uuid,
                title,
                shortcut,
                parent_uuid,
                created,
                modified,
                usage_count: usage_count as u32,
                last_used,
            });
        }

        Ok(tags)
    }

    /// Get all tags ordered by title
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn get_all_tags(&self) -> ThingsResult<Vec<crate::models::Tag>> {
        let rows = sqlx::query(
            "SELECT uuid, title, shortcut, parent, creationDate, userModificationDate, usedDate 
             FROM TMTag 
             ORDER BY title",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to get all tags: {e}")))?;

        let mut tags = Vec::new();
        for row in rows {
            let uuid_str: String = row.get("uuid");
            let uuid =
                Uuid::parse_str(&uuid_str).unwrap_or_else(|_| things_uuid_to_uuid(&uuid_str));
            let title: String = row.get("title");
            let shortcut: Option<String> = row.get("shortcut");
            let parent_str: Option<String> = row.get("parent");
            let parent_uuid =
                parent_str.map(|s| Uuid::parse_str(&s).unwrap_or_else(|_| things_uuid_to_uuid(&s)));

            let creation_ts: f64 = row.get("creationDate");
            let created = {
                let ts = safe_timestamp_convert(creation_ts);
                DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
            };

            let modification_ts: f64 = row.get("userModificationDate");
            let modified = {
                let ts = safe_timestamp_convert(modification_ts);
                DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
            };

            let used_ts: Option<f64> = row.get("usedDate");
            let last_used = used_ts.and_then(|ts| {
                let ts_i64 = safe_timestamp_convert(ts);
                DateTime::from_timestamp(ts_i64, 0)
            });

            // Count usage
            let usage_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM TMTask 
                 WHERE cachedTags IS NOT NULL 
                 AND json_extract(cachedTags, '$') LIKE ?
                 AND trashed = 0",
            )
            .bind(format!("%\"{}\"%", title))
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

            tags.push(crate::models::Tag {
                uuid,
                title,
                shortcut,
                parent_uuid,
                created,
                modified,
                usage_count: usage_count as u32,
                last_used,
            });
        }

        Ok(tags)
    }

    /// Get most frequently used tags
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn get_popular_tags(&self, limit: usize) -> ThingsResult<Vec<crate::models::Tag>> {
        let mut all_tags = self.get_all_tags().await?;

        // Sort by usage count (highest first)
        all_tags.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));

        // Take the top N
        all_tags.truncate(limit);

        Ok(all_tags)
    }

    /// Get recently used tags
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn get_recent_tags(&self, limit: usize) -> ThingsResult<Vec<crate::models::Tag>> {
        let rows = sqlx::query(
            "SELECT uuid, title, shortcut, parent, creationDate, userModificationDate, usedDate 
             FROM TMTag 
             WHERE usedDate IS NOT NULL 
             ORDER BY usedDate DESC 
             LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to get recent tags: {e}")))?;

        let mut tags = Vec::new();
        for row in rows {
            let uuid_str: String = row.get("uuid");
            let uuid =
                Uuid::parse_str(&uuid_str).unwrap_or_else(|_| things_uuid_to_uuid(&uuid_str));
            let title: String = row.get("title");
            let shortcut: Option<String> = row.get("shortcut");
            let parent_str: Option<String> = row.get("parent");
            let parent_uuid =
                parent_str.map(|s| Uuid::parse_str(&s).unwrap_or_else(|_| things_uuid_to_uuid(&s)));

            let creation_ts: f64 = row.get("creationDate");
            let created = {
                let ts = safe_timestamp_convert(creation_ts);
                DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
            };

            let modification_ts: f64 = row.get("userModificationDate");
            let modified = {
                let ts = safe_timestamp_convert(modification_ts);
                DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
            };

            let used_ts: Option<f64> = row.get("usedDate");
            let last_used = used_ts.and_then(|ts| {
                let ts_i64 = safe_timestamp_convert(ts);
                DateTime::from_timestamp(ts_i64, 0)
            });

            // Count usage
            let usage_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM TMTask 
                 WHERE cachedTags IS NOT NULL 
                 AND json_extract(cachedTags, '$') LIKE ?
                 AND trashed = 0",
            )
            .bind(format!("%\"{}\"%", title))
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

            tags.push(crate::models::Tag {
                uuid,
                title,
                shortcut,
                parent_uuid,
                created,
                modified,
                usage_count: usage_count as u32,
                last_used,
            });
        }

        Ok(tags)
    }

    /// Create a tag with smart duplicate detection
    ///
    /// Returns:
    /// - `Created`: New tag was created
    /// - `Existing`: Exact match found (case-insensitive)
    /// - `SimilarFound`: Similar tags found (user decision needed)
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    #[instrument(skip(self))]
    pub async fn create_tag_smart(
        &self,
        request: crate::models::CreateTagRequest,
    ) -> ThingsResult<crate::models::TagCreationResult> {
        use crate::database::tag_utils::normalize_tag_title;
        use crate::models::TagCreationResult;

        // 1. Normalize the title
        let normalized = normalize_tag_title(&request.title);

        // 2. Check for exact match (case-insensitive)
        if let Some(existing) = self.find_tag_by_normalized_title(&normalized).await? {
            return Ok(TagCreationResult::Existing {
                tag: existing,
                is_new: false,
            });
        }

        // 3. Find similar tags (fuzzy matching with 80% threshold)
        let similar_tags = self.find_similar_tags(&normalized, 0.8).await?;

        // 4. If similar tags found, return them for user decision
        if !similar_tags.is_empty() {
            return Ok(TagCreationResult::SimilarFound {
                similar_tags,
                requested_title: request.title,
            });
        }

        // 5. No duplicates, safe to create
        let uuid = Uuid::new_v4();
        let now = Utc::now().timestamp() as f64;

        sqlx::query(
            "INSERT INTO TMTag (uuid, title, shortcut, parent, creationDate, userModificationDate, usedDate, `index`) 
             VALUES (?, ?, ?, ?, ?, ?, NULL, 0)"
        )
        .bind(uuid.to_string())
        .bind(&request.title)
        .bind(request.shortcut.as_ref())
        .bind(request.parent_uuid.map(|u| u.to_string()))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to create tag: {e}")))?;

        info!("Created tag with UUID: {}", uuid);
        Ok(TagCreationResult::Created { uuid, is_new: true })
    }

    /// Create tag forcefully (skip duplicate check)
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    #[instrument(skip(self))]
    pub async fn create_tag_force(
        &self,
        request: crate::models::CreateTagRequest,
    ) -> ThingsResult<Uuid> {
        let uuid = Uuid::new_v4();
        let now = Utc::now().timestamp() as f64;

        sqlx::query(
            "INSERT INTO TMTag (uuid, title, shortcut, parent, creationDate, userModificationDate, usedDate, `index`) 
             VALUES (?, ?, ?, ?, ?, ?, NULL, 0)"
        )
        .bind(uuid.to_string())
        .bind(&request.title)
        .bind(request.shortcut.as_ref())
        .bind(request.parent_uuid.map(|u| u.to_string()))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to create tag: {e}")))?;

        info!("Forcefully created tag with UUID: {}", uuid);
        Ok(uuid)
    }

    /// Update a tag
    ///
    /// # Errors
    ///
    /// Returns an error if the tag doesn't exist or database operation fails
    #[instrument(skip(self))]
    pub async fn update_tag(&self, request: crate::models::UpdateTagRequest) -> ThingsResult<()> {
        use crate::database::tag_utils::normalize_tag_title;

        // Verify tag exists
        let existing = self
            .find_tag_by_normalized_title(&request.uuid.to_string())
            .await?;
        if existing.is_none() {
            // Try by UUID
            let row = sqlx::query("SELECT 1 FROM TMTag WHERE uuid = ?")
                .bind(request.uuid.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to validate tag: {e}")))?;

            if row.is_none() {
                return Err(ThingsError::unknown(format!(
                    "Tag not found: {}",
                    request.uuid
                )));
            }
        }

        // If renaming, check for duplicates with new name
        if let Some(new_title) = &request.title {
            let normalized = normalize_tag_title(new_title);
            if let Some(duplicate) = self.find_tag_by_normalized_title(&normalized).await? {
                if duplicate.uuid != request.uuid {
                    return Err(ThingsError::unknown(format!(
                        "Tag with title '{}' already exists",
                        new_title
                    )));
                }
            }
        }

        let now = Utc::now().timestamp() as f64;

        // Build dynamic UPDATE query
        let mut updates = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(title) = &request.title {
            updates.push("title = ?");
            params.push(title.clone());
        }
        if let Some(shortcut) = &request.shortcut {
            updates.push("shortcut = ?");
            params.push(shortcut.clone());
        }
        if let Some(parent_uuid) = request.parent_uuid {
            updates.push("parent = ?");
            params.push(parent_uuid.to_string());
        }

        if updates.is_empty() {
            return Ok(()); // Nothing to update
        }

        updates.push("userModificationDate = ?");
        params.push(now.to_string());

        let sql = format!("UPDATE TMTag SET {} WHERE uuid = ?", updates.join(", "));
        params.push(request.uuid.to_string());

        let mut query = sqlx::query(&sql);
        for param in params {
            query = query.bind(param);
        }

        query
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to update tag: {e}")))?;

        info!("Updated tag with UUID: {}", request.uuid);
        Ok(())
    }

    /// Delete a tag
    ///
    /// # Arguments
    ///
    /// * `uuid` - UUID of the tag to delete
    /// * `remove_from_tasks` - If true, removes tag from all tasks' cachedTags
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    #[instrument(skip(self))]
    pub async fn delete_tag(&self, uuid: &Uuid, remove_from_tasks: bool) -> ThingsResult<()> {
        // Get the tag title before deletion
        let tag = self.find_tag_by_normalized_title(&uuid.to_string()).await?;

        if tag.is_none() {
            // Try by UUID directly
            let row = sqlx::query("SELECT title FROM TMTag WHERE uuid = ?")
                .bind(uuid.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to find tag: {e}")))?;

            if row.is_none() {
                return Err(ThingsError::unknown(format!("Tag not found: {}", uuid)));
            }
        }

        if remove_from_tasks {
            // TODO: Implement updating all tasks' cachedTags to remove this tag
            // This requires parsing and re-serializing the JSON arrays
            info!("Removing tag {} from all tasks (not yet implemented)", uuid);
        }

        // Delete the tag
        sqlx::query("DELETE FROM TMTag WHERE uuid = ?")
            .bind(uuid.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to delete tag: {e}")))?;

        info!("Deleted tag with UUID: {}", uuid);
        Ok(())
    }

    /// Merge two tags (combine source into target)
    ///
    /// # Arguments
    ///
    /// * `source_uuid` - UUID of tag to merge from (will be deleted)
    /// * `target_uuid` - UUID of tag to merge into (will remain)
    ///
    /// # Errors
    ///
    /// Returns an error if either tag doesn't exist or database operation fails
    #[instrument(skip(self))]
    pub async fn merge_tags(&self, source_uuid: &Uuid, target_uuid: &Uuid) -> ThingsResult<()> {
        // Verify both tags exist
        let source_row = sqlx::query("SELECT title FROM TMTag WHERE uuid = ?")
            .bind(source_uuid.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to find source tag: {e}")))?;

        if source_row.is_none() {
            return Err(ThingsError::unknown(format!(
                "Source tag not found: {}",
                source_uuid
            )));
        }

        let target_row = sqlx::query("SELECT title FROM TMTag WHERE uuid = ?")
            .bind(target_uuid.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to find target tag: {e}")))?;

        if target_row.is_none() {
            return Err(ThingsError::unknown(format!(
                "Target tag not found: {}",
                target_uuid
            )));
        }

        // TODO: Implement updating all tasks' cachedTags to replace source tag with target tag
        // This requires parsing and re-serializing the JSON arrays
        info!(
            "Merging tag {} into {} (tag replacement in tasks not yet fully implemented)",
            source_uuid, target_uuid
        );

        // Update usedDate on target if source was used more recently
        let now = Utc::now().timestamp() as f64;
        sqlx::query("UPDATE TMTag SET userModificationDate = ?, usedDate = ? WHERE uuid = ?")
            .bind(now)
            .bind(now)
            .bind(target_uuid.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to update target tag: {e}")))?;

        // Delete source tag
        sqlx::query("DELETE FROM TMTag WHERE uuid = ?")
            .bind(source_uuid.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to delete source tag: {e}")))?;

        info!("Merged tag {} into {}", source_uuid, target_uuid);
        Ok(())
    }

    // ========================================================================
    // TAG ASSIGNMENT OPERATIONS
    // ========================================================================

    /// Add a tag to a task (with duplicate prevention)
    ///
    /// Returns:
    /// - `Assigned`: Tag was successfully assigned
    /// - `Suggestions`: Similar tags found (user decision needed)
    ///
    /// # Errors
    ///
    /// Returns an error if the task doesn't exist or database operation fails
    #[instrument(skip(self))]
    pub async fn add_tag_to_task(
        &self,
        task_uuid: &Uuid,
        tag_title: &str,
    ) -> ThingsResult<crate::models::TagAssignmentResult> {
        use crate::database::tag_utils::normalize_tag_title;
        use crate::models::TagAssignmentResult;

        // 1. Verify task exists
        validators::validate_task_exists(&self.pool, task_uuid).await?;

        // 2. Normalize and find tag
        let normalized = normalize_tag_title(tag_title);

        // 3. Check for exact match first
        let tag = if let Some(existing_tag) = self.find_tag_by_normalized_title(&normalized).await?
        {
            existing_tag
        } else {
            // 4. Find similar tags
            let similar_tags = self.find_similar_tags(&normalized, 0.8).await?;

            if !similar_tags.is_empty() {
                return Ok(TagAssignmentResult::Suggestions { similar_tags });
            }

            // 5. No existing tag found, create new one
            let request = crate::models::CreateTagRequest {
                title: tag_title.to_string(),
                shortcut: None,
                parent_uuid: None,
            };
            let _uuid = self.create_tag_force(request).await?;

            // Fetch the newly created tag
            self.find_tag_by_normalized_title(&normalized)
                .await?
                .ok_or_else(|| ThingsError::unknown("Failed to retrieve newly created tag"))?
        };

        // 6. Get current tags from task
        let row = sqlx::query("SELECT cachedTags FROM TMTask WHERE uuid = ?")
            .bind(task_uuid.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to fetch task tags: {e}")))?;

        let cached_tags_blob: Option<Vec<u8>> = row.get("cachedTags");
        let mut tags: Vec<String> = if let Some(blob) = cached_tags_blob {
            deserialize_tags_from_blob(&blob)?
        } else {
            Vec::new()
        };

        // 7. Add tag if not already present
        if !tags.contains(&tag.title) {
            tags.push(tag.title.clone());

            // 8. Serialize and update
            let cached_tags = serialize_tags_to_blob(&tags)?;
            let now = Utc::now().timestamp() as f64;

            sqlx::query(
                "UPDATE TMTask SET cachedTags = ?, userModificationDate = ? WHERE uuid = ?",
            )
            .bind(cached_tags)
            .bind(now)
            .bind(task_uuid.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to update task tags: {e}")))?;

            // 9. Update tag's usedDate
            sqlx::query("UPDATE TMTag SET usedDate = ?, userModificationDate = ? WHERE uuid = ?")
                .bind(now)
                .bind(now)
                .bind(tag.uuid.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to update tag usedDate: {e}")))?;

            info!("Added tag '{}' to task {}", tag.title, task_uuid);
        }

        Ok(TagAssignmentResult::Assigned { tag_uuid: tag.uuid })
    }

    /// Remove a tag from a task
    ///
    /// # Errors
    ///
    /// Returns an error if the task doesn't exist or database operation fails
    #[instrument(skip(self))]
    pub async fn remove_tag_from_task(
        &self,
        task_uuid: &Uuid,
        tag_title: &str,
    ) -> ThingsResult<()> {
        use crate::database::tag_utils::normalize_tag_title;

        // 1. Verify task exists
        validators::validate_task_exists(&self.pool, task_uuid).await?;

        // 2. Get current tags from task
        let row = sqlx::query("SELECT cachedTags FROM TMTask WHERE uuid = ?")
            .bind(task_uuid.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to fetch task tags: {e}")))?;

        let cached_tags_blob: Option<Vec<u8>> = row.get("cachedTags");
        let mut tags: Vec<String> = if let Some(blob) = cached_tags_blob {
            deserialize_tags_from_blob(&blob)?
        } else {
            return Ok(()); // No tags to remove
        };

        // 3. Normalize and find the tag to remove (case-insensitive)
        let normalized = normalize_tag_title(tag_title);
        let original_len = tags.len();
        tags.retain(|t| normalize_tag_title(t) != normalized);

        // 4. If tags were actually removed, update the task
        if tags.len() < original_len {
            let cached_tags = if tags.is_empty() {
                None
            } else {
                Some(serialize_tags_to_blob(&tags)?)
            };

            let now = Utc::now().timestamp() as f64;

            if let Some(cached_tags_val) = cached_tags {
                sqlx::query(
                    "UPDATE TMTask SET cachedTags = ?, userModificationDate = ? WHERE uuid = ?",
                )
                .bind(cached_tags_val)
                .bind(now)
                .bind(task_uuid.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to update task tags: {e}")))?;
            } else {
                // Set cachedTags to NULL if no tags remain
                sqlx::query(
                    "UPDATE TMTask SET cachedTags = NULL, userModificationDate = ? WHERE uuid = ?",
                )
                .bind(now)
                .bind(task_uuid.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to update task tags: {e}")))?;
            }

            info!("Removed tag '{}' from task {}", tag_title, task_uuid);
        }

        Ok(())
    }

    /// Replace all tags on a task (with duplicate prevention)
    ///
    /// Returns any tag titles that had similar matches for user confirmation
    ///
    /// # Errors
    ///
    /// Returns an error if the task doesn't exist or database operation fails
    #[instrument(skip(self))]
    pub async fn set_task_tags(
        &self,
        task_uuid: &Uuid,
        tag_titles: Vec<String>,
    ) -> ThingsResult<Vec<crate::models::TagMatch>> {
        use crate::database::tag_utils::normalize_tag_title;

        // 1. Verify task exists
        validators::validate_task_exists(&self.pool, task_uuid).await?;

        let mut resolved_tags = Vec::new();
        let mut suggestions = Vec::new();

        // 2. Resolve each tag title
        for title in tag_titles {
            let normalized = normalize_tag_title(&title);

            // Try to find exact match
            if let Some(existing_tag) = self.find_tag_by_normalized_title(&normalized).await? {
                resolved_tags.push(existing_tag.title);
            } else {
                // Check for similar tags
                let similar_tags = self.find_similar_tags(&normalized, 0.8).await?;

                if !similar_tags.is_empty() {
                    suggestions.extend(similar_tags);
                }

                // Use the requested title anyway (will create if needed)
                resolved_tags.push(title);
            }
        }

        // 3. For any tags that don't exist yet, create them
        for title in &resolved_tags {
            let normalized = normalize_tag_title(title);
            if self
                .find_tag_by_normalized_title(&normalized)
                .await?
                .is_none()
            {
                let request = crate::models::CreateTagRequest {
                    title: title.clone(),
                    shortcut: None,
                    parent_uuid: None,
                };
                self.create_tag_force(request).await?;
            }
        }

        // 4. Update task's cachedTags
        let cached_tags = if resolved_tags.is_empty() {
            None
        } else {
            Some(serialize_tags_to_blob(&resolved_tags)?)
        };

        let now = Utc::now().timestamp() as f64;

        if let Some(cached_tags_val) = cached_tags {
            sqlx::query(
                "UPDATE TMTask SET cachedTags = ?, userModificationDate = ? WHERE uuid = ?",
            )
            .bind(cached_tags_val)
            .bind(now)
            .bind(task_uuid.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to update task tags: {e}")))?;
        } else {
            sqlx::query(
                "UPDATE TMTask SET cachedTags = NULL, userModificationDate = ? WHERE uuid = ?",
            )
            .bind(now)
            .bind(task_uuid.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to update task tags: {e}")))?;
        }

        // 5. Update usedDate for all tags
        for title in &resolved_tags {
            let normalized = normalize_tag_title(title);
            if let Some(tag) = self.find_tag_by_normalized_title(&normalized).await? {
                sqlx::query(
                    "UPDATE TMTag SET usedDate = ?, userModificationDate = ? WHERE uuid = ?",
                )
                .bind(now)
                .bind(now)
                .bind(tag.uuid.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to update tag usedDate: {e}")))?;
            }
        }

        info!("Set tags on task {} to: {:?}", task_uuid, resolved_tags);
        Ok(suggestions)
    }

    // ========================================================================
    // TAG AUTO-COMPLETION & ANALYTICS
    // ========================================================================

    /// Get tag completions for partial input
    ///
    /// Returns tags sorted by:
    /// 1. Exact prefix matches (prioritized)
    /// 2. Contains matches
    /// 3. Fuzzy matches
    /// Within each category, sorted by usage frequency
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn get_tag_completions(
        &self,
        partial_input: &str,
        limit: usize,
    ) -> ThingsResult<Vec<crate::models::TagCompletion>> {
        use crate::database::tag_utils::{calculate_similarity, normalize_tag_title};

        let normalized_input = normalize_tag_title(partial_input);
        let all_tags = self.get_all_tags().await?;

        let mut completions: Vec<crate::models::TagCompletion> = all_tags
            .into_iter()
            .filter_map(|tag| {
                let normalized_tag = normalize_tag_title(&tag.title);

                // Calculate score based on match type
                let score = if normalized_tag.starts_with(&normalized_input) {
                    // Exact prefix match: highest priority
                    3.0 + (tag.usage_count as f32 / 100.0)
                } else if normalized_tag.contains(&normalized_input) {
                    // Contains match: medium priority
                    2.0 + (tag.usage_count as f32 / 100.0)
                } else {
                    // Fuzzy match: lower priority
                    let similarity = calculate_similarity(partial_input, &tag.title);
                    if similarity >= 0.6 {
                        similarity + (tag.usage_count as f32 / 1000.0)
                    } else {
                        return None; // Not similar enough
                    }
                };

                Some(crate::models::TagCompletion { tag, score })
            })
            .collect();

        // Sort by score (highest first)
        completions.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take the top N
        completions.truncate(limit);

        Ok(completions)
    }

    /// Get detailed statistics for a tag
    ///
    /// # Errors
    ///
    /// Returns an error if the tag doesn't exist or database query fails
    #[instrument(skip(self))]
    pub async fn get_tag_statistics(
        &self,
        uuid: &Uuid,
    ) -> ThingsResult<crate::models::TagStatistics> {
        // Get the tag
        let tag_row = sqlx::query("SELECT title FROM TMTag WHERE uuid = ?")
            .bind(uuid.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to find tag: {e}")))?;

        let title: String = tag_row
            .ok_or_else(|| ThingsError::unknown(format!("Tag not found: {}", uuid)))?
            .get("title");

        // Get all tasks using this tag
        // Note: We query cachedTags BLOB which should contain JSON, but handle gracefully if malformed
        let task_rows = sqlx::query(
            "SELECT uuid, cachedTags FROM TMTask 
             WHERE cachedTags IS NOT NULL 
             AND trashed = 0",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to query tasks with tag: {e}")))?;

        let mut task_uuids = Vec::new();
        for row in task_rows {
            let uuid_str: String = row.get("uuid");
            let cached_tags_blob: Option<Vec<u8>> = row.get("cachedTags");

            // Check if this task actually has the tag
            if let Some(blob) = cached_tags_blob {
                if let Ok(tags) = deserialize_tags_from_blob(&blob) {
                    if tags.iter().any(|t| t.eq_ignore_ascii_case(&title)) {
                        let task_uuid = Uuid::parse_str(&uuid_str)
                            .unwrap_or_else(|_| things_uuid_to_uuid(&uuid_str));
                        task_uuids.push(task_uuid);
                    }
                }
            }
        }

        let usage_count = task_uuids.len() as u32;

        // Find related tags (tags that frequently appear with this tag)
        let mut related_tags: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();

        for task_uuid in &task_uuids {
            let row = sqlx::query("SELECT cachedTags FROM TMTask WHERE uuid = ?")
                .bind(task_uuid.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to fetch task tags: {e}")))?;

            if let Some(row) = row {
                let cached_tags_blob: Option<Vec<u8>> = row.get("cachedTags");
                if let Some(blob) = cached_tags_blob {
                    let tags: Vec<String> = deserialize_tags_from_blob(&blob)?;
                    for tag in tags {
                        if tag != title {
                            *related_tags.entry(tag).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        // Sort related tags by co-occurrence count
        let mut related_vec: Vec<(String, u32)> = related_tags.into_iter().collect();
        related_vec.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(crate::models::TagStatistics {
            uuid: *uuid,
            title,
            usage_count,
            task_uuids,
            related_tags: related_vec,
        })
    }

    /// Find duplicate or highly similar tags
    ///
    /// Returns pairs of tags that are similar above the threshold
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn find_duplicate_tags(
        &self,
        min_similarity: f32,
    ) -> ThingsResult<Vec<crate::models::TagPair>> {
        use crate::database::tag_utils::calculate_similarity;

        let all_tags = self.get_all_tags().await?;
        let mut pairs = Vec::new();

        // Compare each tag with every other tag
        for i in 0..all_tags.len() {
            for j in (i + 1)..all_tags.len() {
                let tag1 = &all_tags[i];
                let tag2 = &all_tags[j];

                let similarity = calculate_similarity(&tag1.title, &tag2.title);

                if similarity >= min_similarity {
                    pairs.push(crate::models::TagPair {
                        tag1: tag1.clone(),
                        tag2: tag2.clone(),
                        similarity,
                    });
                }
            }
        }

        // Sort by similarity (highest first)
        pairs.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(pairs)
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
