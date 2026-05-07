use crate::{
    database::{
        pool::{
            ComprehensiveHealthStatus, DatabasePoolConfig, PoolHealthStatus, PoolMetrics,
            SqliteOptimizations,
        },
        stats::DatabaseStats,
    },
    error::{Result as ThingsResult, ThingsError},
};
use chrono::Utc;
use sqlx::{pool::PoolOptions, SqlitePool};
use std::path::Path;
use tracing::{debug, error, info, instrument};

/// SQLx-based database implementation for Things 3 data
/// This provides async, Send + Sync compatible database access
#[derive(Debug, Clone)]
pub struct ThingsDatabase {
    pub(crate) pool: SqlitePool,
    config: DatabasePoolConfig,
}

impl ThingsDatabase {
    /// Create a new database connection pool with default configuration
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use things3_core::{ThingsDatabase, ThingsError};
    /// use std::path::Path;
    ///
    /// # async fn example() -> Result<(), ThingsError> {
    /// // Connect to Things 3 database
    /// let db = ThingsDatabase::new(Path::new("/path/to/things.db")).await?;
    ///
    /// // Get inbox tasks
    /// let tasks = db.get_inbox(None).await?;
    /// println!("Found {} tasks in inbox", tasks.len());
    /// # Ok(())
    /// # }
    /// ```
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
    /// # Examples
    ///
    /// ```no_run
    /// use things3_core::{ThingsDatabase, DatabasePoolConfig, ThingsError};
    /// use std::path::Path;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> Result<(), ThingsError> {
    /// // Create custom pool configuration
    /// let config = DatabasePoolConfig {
    ///     max_connections: 10,
    ///     min_connections: 2,
    ///     connect_timeout: Duration::from_secs(5),
    ///     idle_timeout: Duration::from_secs(300),
    ///     max_lifetime: Duration::from_secs(3600),
    ///     test_before_acquire: true,
    ///     sqlite_optimizations: Default::default(),
    /// };
    ///
    /// // Connect with custom configuration
    /// let db = ThingsDatabase::new_with_config(
    ///     Path::new("/path/to/things.db"),
    ///     config,
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
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

    // ========================================================================
    // TAG OPERATIONS (with smart duplicate prevention)
    // ========================================================================

    // ========================================================================
    // TAG ASSIGNMENT OPERATIONS
    // ========================================================================

    // ========================================================================
    // TAG AUTO-COMPLETION & ANALYTICS
    // ========================================================================

    // ============================================================================
    // Bulk Operations
    // ============================================================================
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "advanced-queries")]
    use crate::models::{TaskStatus, ThingsId};
    use std::time::Duration;
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

    #[cfg(feature = "advanced-queries")]
    mod query_tasks_tests {
        use super::*;
        use crate::models::TaskFilters;
        use crate::query::TaskQueryBuilder;
        use tempfile::NamedTempFile;

        async fn open_test_db() -> (ThingsDatabase, NamedTempFile) {
            let f = NamedTempFile::new().unwrap();
            crate::test_utils::create_test_database(f.path())
                .await
                .unwrap();
            let db = ThingsDatabase::new(f.path()).await.unwrap();
            (db, f)
        }

        #[tokio::test]
        async fn test_query_tasks_no_filters() {
            let (db, _f) = open_test_db().await;
            let result = db.query_tasks(&TaskFilters::default()).await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_query_tasks_status_filter() {
            let (db, _f) = open_test_db().await;
            let filters = TaskFilters {
                status: Some(TaskStatus::Completed),
                ..TaskFilters::default()
            };
            let tasks = db.query_tasks(&filters).await.unwrap();
            assert!(tasks.iter().all(|t| t.status == TaskStatus::Completed));
        }

        #[tokio::test]
        async fn test_query_tasks_limit() {
            let (db, _f) = open_test_db().await;
            let filters = TaskFilters {
                limit: Some(1),
                ..TaskFilters::default()
            };
            let tasks = db.query_tasks(&filters).await.unwrap();
            assert!(tasks.len() <= 1);
        }

        #[tokio::test]
        async fn test_query_tasks_tag_filter_and_semantics() {
            let (db, _f) = open_test_db().await;
            let filters = TaskFilters {
                tags: Some(vec!["nonexistent-tag-xyz".to_string()]),
                ..TaskFilters::default()
            };
            let tasks = db.query_tasks(&filters).await.unwrap();
            assert!(tasks.is_empty());
        }

        #[tokio::test]
        async fn test_query_tasks_search_query() {
            let (db, _f) = open_test_db().await;
            let filters = TaskFilters {
                search_query: Some("zzznomatch".to_string()),
                ..TaskFilters::default()
            };
            let tasks = db.query_tasks(&filters).await.unwrap();
            assert!(tasks.is_empty());
        }

        #[tokio::test]
        async fn test_query_tasks_trashed_status() {
            use sqlx::SqlitePool;
            use uuid::Uuid;

            // Create a DB, insert one soft-deleted row (trashed = 1), then verify:
            // - default query (trashed = 0) does NOT return it
            // - TaskStatus::Trashed filter DOES return it
            let f = NamedTempFile::new().unwrap();
            crate::test_utils::create_test_database(f.path())
                .await
                .unwrap();
            let pool = SqlitePool::connect(&format!("sqlite:{}", f.path().display()))
                .await
                .unwrap();
            let trashed_uuid = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO TMTask \
                 (uuid, title, type, status, trashed, creationDate, userModificationDate) \
                 VALUES (?, ?, 0, 0, 1, 0, 0)",
            )
            .bind(&trashed_uuid)
            .bind("Trashed Task")
            .execute(&pool)
            .await
            .unwrap();
            pool.close().await;

            let db = ThingsDatabase::new(f.path()).await.unwrap();

            // Default query must not surface the trashed row
            let active = db.query_tasks(&TaskFilters::default()).await.unwrap();
            assert!(active.iter().all(|t| t.uuid.to_string() != trashed_uuid));

            // Trashed filter must surface it
            let trashed = db
                .query_tasks(&TaskFilters {
                    status: Some(TaskStatus::Trashed),
                    ..TaskFilters::default()
                })
                .await
                .unwrap();
            assert!(
                trashed.iter().any(|t| t.uuid.to_string() == trashed_uuid),
                "expected trashed row to be returned by TaskStatus::Trashed filter"
            );
        }

        #[tokio::test]
        async fn test_query_tasks_offset_without_limit() {
            // Bug fix: offset must not be silently ignored when limit is absent
            let (db, _f) = open_test_db().await;
            let all = db.query_tasks(&TaskFilters::default()).await.unwrap();
            if all.len() < 2 {
                return; // not enough rows to test pagination
            }
            let filters = TaskFilters {
                offset: Some(1),
                ..TaskFilters::default()
            };
            let offset_tasks = db.query_tasks(&filters).await.unwrap();
            assert_eq!(offset_tasks.len(), all.len() - 1);
            assert_eq!(offset_tasks[0].uuid, all[1].uuid);
        }

        #[tokio::test]
        async fn test_query_tasks_pagination_with_post_filter() {
            // Bug fix: LIMIT/OFFSET must count post-filter matches, not raw SQL rows
            let (db, _f) = open_test_db().await;
            // Fetch all tasks matching search (may be 0 in empty test DB — that's fine)
            let all_matching = db
                .query_tasks(&TaskFilters {
                    search_query: Some(String::new()),
                    ..TaskFilters::default()
                })
                .await
                .unwrap();
            if all_matching.len() < 2 {
                return;
            }
            let page0 = db
                .query_tasks(&TaskFilters {
                    search_query: Some(String::new()),
                    limit: Some(1),
                    offset: Some(0),
                    ..TaskFilters::default()
                })
                .await
                .unwrap();
            let page1 = db
                .query_tasks(&TaskFilters {
                    search_query: Some(String::new()),
                    limit: Some(1),
                    offset: Some(1),
                    ..TaskFilters::default()
                })
                .await
                .unwrap();
            assert_eq!(page0.len(), 1);
            assert_eq!(page1.len(), 1);
            assert_ne!(page0[0].uuid, page1[0].uuid);
        }

        /// Insert a TMTask row with optional notes and tags.
        /// Used to seed tests; bypasses create_test_database which inserts only untagged rows.
        async fn insert_task(
            db: &ThingsDatabase,
            title: &str,
            notes: Option<&str>,
            tags: &[&str],
        ) -> ThingsId {
            let raw_uuid = uuid::Uuid::new_v4();
            sqlx::query(
                "INSERT INTO TMTask \
                 (uuid, title, notes, type, status, trashed, creationDate, userModificationDate) \
                 VALUES (?, ?, ?, 0, 0, 0, 0, 0)",
            )
            .bind(raw_uuid.to_string())
            .bind(title)
            .bind(notes)
            .execute(&db.pool)
            .await
            .unwrap();
            let task_id = ThingsId::from_trusted(raw_uuid.to_string());

            // Insert tags via TMTaskTag
            for tag_title in tags {
                // Find or create the tag
                let normalized = crate::database::tag_utils::normalize_tag_title(tag_title);
                let tag = if let Some(existing) =
                    db.find_tag_by_normalized_title(&normalized).await.unwrap()
                {
                    existing
                } else {
                    let request = crate::models::CreateTagRequest {
                        title: (*tag_title).to_string(),
                        shortcut: None,
                        parent_uuid: None,
                    };
                    let uuid = db.create_tag_force(request).await.unwrap();
                    db.find_tag_by_normalized_title(&normalized)
                        .await
                        .unwrap()
                        .unwrap_or_else(|| crate::models::Tag {
                            uuid,
                            title: (*tag_title).to_string(),
                            shortcut: None,
                            parent_uuid: None,
                            usage_count: 0,
                            last_used: None,
                        })
                };
                sqlx::query("INSERT OR IGNORE INTO TMTaskTag (tasks, tags) VALUES (?, ?)")
                    .bind(task_id.as_str())
                    .bind(tag.uuid.as_str())
                    .execute(&db.pool)
                    .await
                    .unwrap();
            }

            task_id
        }

        async fn insert_task_with_tags(
            db: &ThingsDatabase,
            title: &str,
            tags: &[&str],
        ) -> ThingsId {
            insert_task(db, title, None, tags).await
        }

        async fn open_db_with_tagged_rows(
        ) -> (ThingsDatabase, NamedTempFile, ThingsId, ThingsId, ThingsId) {
            let (db, f) = open_test_db().await;
            let a = insert_task_with_tags(&db, "task-a", &["a"]).await;
            let b = insert_task_with_tags(&db, "task-b", &["b"]).await;
            let c = insert_task_with_tags(&db, "task-c", &["c"]).await;
            (db, f, a, b, c)
        }

        #[tokio::test]
        async fn test_query_tasks_any_tags_or_semantics() {
            let (db, _f, a, b, c) = open_db_with_tagged_rows().await;
            let tasks = TaskQueryBuilder::new()
                .any_tags(vec!["a".to_string(), "b".to_string()])
                .execute(&db)
                .await
                .unwrap();
            let uuids: std::collections::HashSet<_> =
                tasks.iter().map(|t| t.uuid.clone()).collect();
            assert!(uuids.contains(&a));
            assert!(uuids.contains(&b));
            assert!(!uuids.contains(&c));
        }

        #[tokio::test]
        async fn test_query_tasks_exclude_tags() {
            let (db, _f, a, b, c) = open_db_with_tagged_rows().await;
            let tasks = TaskQueryBuilder::new()
                .exclude_tags(vec!["b".to_string()])
                .execute(&db)
                .await
                .unwrap();
            let uuids: std::collections::HashSet<_> =
                tasks.iter().map(|t| t.uuid.clone()).collect();
            assert!(uuids.contains(&a));
            assert!(!uuids.contains(&b));
            assert!(uuids.contains(&c));
        }

        #[tokio::test]
        async fn test_query_tasks_tag_count_min() {
            let (db, _f) = open_test_db().await;
            insert_task_with_tags(&db, "zero-tags", &[]).await;
            insert_task_with_tags(&db, "one-tag", &["x"]).await;
            let two = insert_task_with_tags(&db, "two-tags", &["x", "y"]).await;
            let tasks = TaskQueryBuilder::new()
                .tag_count(2)
                .execute(&db)
                .await
                .unwrap();
            let uuids: Vec<ThingsId> = tasks.iter().map(|t| t.uuid.clone()).collect();
            assert_eq!(uuids, vec![two]);
        }

        #[tokio::test]
        async fn test_query_tasks_combined_tag_filters() {
            let (db, _f) = open_test_db().await;
            let target = insert_task_with_tags(&db, "target", &["a", "x"]).await;
            let _wrong_required = insert_task_with_tags(&db, "no-a", &["x"]).await;
            let _excluded = insert_task_with_tags(&db, "has-z", &["a", "x", "z"]).await;
            let _no_any = insert_task_with_tags(&db, "no-x", &["a"]).await;

            let tasks = TaskQueryBuilder::new()
                .tags(vec!["a".to_string()])
                .any_tags(vec!["x".to_string(), "y".to_string()])
                .exclude_tags(vec!["z".to_string()])
                .execute(&db)
                .await
                .unwrap();
            let uuids: Vec<ThingsId> = tasks.iter().map(|t| t.uuid.clone()).collect();
            assert_eq!(uuids, vec![target]);
        }

        #[tokio::test]
        async fn test_query_tasks_pagination_with_any_tags() {
            // execute() must defer LIMIT/OFFSET to Rust when any_tags is set so
            // pages count only matching rows.
            let (db, _f) = open_test_db().await;
            insert_task_with_tags(&db, "a1", &["a"]).await;
            insert_task_with_tags(&db, "a2", &["a"]).await;
            insert_task_with_tags(&db, "a3", &["a"]).await;
            let page0 = TaskQueryBuilder::new()
                .any_tags(vec!["a".to_string()])
                .limit(1)
                .offset(0)
                .execute(&db)
                .await
                .unwrap();
            let page1 = TaskQueryBuilder::new()
                .any_tags(vec!["a".to_string()])
                .limit(1)
                .offset(1)
                .execute(&db)
                .await
                .unwrap();
            assert_eq!(page0.len(), 1);
            assert_eq!(page1.len(), 1);
            assert_ne!(page0[0].uuid, page1[0].uuid);
        }

        #[tokio::test]
        async fn test_execute_fuzzy_typo_match() {
            let (db, _f) = open_test_db().await;
            let groceries = insert_task(&db, "Buy groceries", None, &[]).await;
            let tasks = TaskQueryBuilder::new()
                .fuzzy_search("grocries")
                .execute(&db)
                .await
                .unwrap();
            let uuids: Vec<ThingsId> = tasks.iter().map(|t| t.uuid.clone()).collect();
            assert!(
                uuids.contains(&groceries),
                "typo 'grocries' should match 'Buy groceries'"
            );
        }

        #[tokio::test]
        async fn test_execute_fuzzy_below_threshold_excluded() {
            let (db, _f) = open_test_db().await;
            insert_task(&db, "Buy groceries", None, &[]).await;
            let tasks = TaskQueryBuilder::new()
                .fuzzy_search("xyz")
                .fuzzy_threshold(0.95)
                .execute(&db)
                .await
                .unwrap();
            assert!(
                tasks.is_empty(),
                "completely unrelated query should return nothing at 0.95 threshold"
            );
        }

        #[tokio::test]
        async fn test_execute_ranked_score_ordering() {
            let (db, _f) = open_test_db().await;
            insert_task(&db, "urgent task", None, &[]).await;
            insert_task(&db, "urgntt task", None, &[]).await; // typo
            insert_task(&db, "completely unrelated xyz abc", None, &[]).await;
            let ranked = TaskQueryBuilder::new()
                .fuzzy_search("urgent")
                .fuzzy_threshold(0.5)
                .execute_ranked(&db)
                .await
                .unwrap();
            // Verify scores are non-increasing
            for pair in ranked.windows(2) {
                assert!(
                    pair[0].score >= pair[1].score,
                    "results must be sorted by score desc: {} < {}",
                    pair[0].score,
                    pair[1].score
                );
            }
            assert!(!ranked.is_empty(), "at least 'urgent task' should match");
        }

        #[tokio::test]
        async fn test_execute_ranked_pagination() {
            let (db, _f) = open_test_db().await;
            for i in 0..5 {
                insert_task(&db, &format!("meeting agenda item {i}"), None, &[]).await;
            }
            let all = TaskQueryBuilder::new()
                .fuzzy_search("agenda")
                .execute_ranked(&db)
                .await
                .unwrap();
            let page = TaskQueryBuilder::new()
                .fuzzy_search("agenda")
                .limit(2)
                .offset(1)
                .execute_ranked(&db)
                .await
                .unwrap();
            assert_eq!(page.len(), 2);
            assert_eq!(page[0].task.uuid, all[1].task.uuid);
            assert_eq!(page[1].task.uuid, all[2].task.uuid);
        }

        #[tokio::test]
        async fn test_execute_fuzzy_with_search_collision() {
            // If substring search were applied, "zzznomatch" would filter out the
            // target row and tasks would be empty — proving fuzzy suppressed it.
            let (db, _f) = open_test_db().await;
            let target = insert_task(&db, "meeting agenda", None, &[]).await;
            let tasks = TaskQueryBuilder::new()
                .search("zzznomatch")
                .fuzzy_search("agenda")
                .execute(&db)
                .await
                .unwrap();
            assert_eq!(
                tasks.len(),
                1,
                "only the 'meeting agenda' row should match; substring filter must be suppressed"
            );
            assert_eq!(
                tasks[0].uuid, target,
                "fuzzy should win over substring search"
            );
        }

        #[tokio::test]
        async fn test_execute_ranked_errors_without_fuzzy_query() {
            let (db, _f) = open_test_db().await;
            let result = TaskQueryBuilder::new().execute_ranked(&db).await;
            assert!(
                result.is_err(),
                "execute_ranked without fuzzy_search should error"
            );
        }

        #[tokio::test]
        async fn test_execute_fuzzy_searches_notes() {
            let (db, _f) = open_test_db().await;
            let target = insert_task(&db, "Weekly sync", Some("meeting agenda for Q2"), &[]).await;
            let tasks = TaskQueryBuilder::new()
                .fuzzy_search("agenda")
                .execute(&db)
                .await
                .unwrap();
            let uuids: Vec<ThingsId> = tasks.iter().map(|t| t.uuid.clone()).collect();
            assert!(uuids.contains(&target), "fuzzy should match text in notes");
        }

        async fn insert_task_with_status(
            db: &ThingsDatabase,
            title: &str,
            status: TaskStatus,
        ) -> ThingsId {
            let raw_uuid = uuid::Uuid::new_v4();
            let status_n: i64 = match status {
                TaskStatus::Incomplete => 0,
                TaskStatus::Canceled => 2,
                TaskStatus::Completed => 3,
                TaskStatus::Trashed => 0,
            };
            sqlx::query(
                "INSERT INTO TMTask \
                 (uuid, title, notes, type, status, trashed, creationDate, userModificationDate) \
                 VALUES (?, ?, NULL, 0, ?, 0, 0, 0)",
            )
            .bind(raw_uuid.to_string())
            .bind(title)
            .bind(status_n)
            .execute(&db.pool)
            .await
            .unwrap();
            ThingsId::from_trusted(raw_uuid.to_string())
        }

        async fn insert_task_with_type(
            db: &ThingsDatabase,
            title: &str,
            task_type: crate::models::TaskType,
        ) -> ThingsId {
            let raw_uuid = uuid::Uuid::new_v4();
            let type_n: i64 = match task_type {
                crate::models::TaskType::Todo => 0,
                crate::models::TaskType::Project => 1,
                crate::models::TaskType::Heading => 2,
                crate::models::TaskType::Area => 3,
            };
            sqlx::query(
                "INSERT INTO TMTask \
                 (uuid, title, notes, type, status, trashed, creationDate, userModificationDate) \
                 VALUES (?, ?, NULL, ?, 0, 0, 0, 0)",
            )
            .bind(raw_uuid.to_string())
            .bind(title)
            .bind(type_n)
            .execute(&db.pool)
            .await
            .unwrap();
            ThingsId::from_trusted(raw_uuid.to_string())
        }

        #[tokio::test]
        async fn test_execute_with_where_expr_or_status() {
            use crate::filter_expr::FilterExpr;
            let (db, _f) = open_test_db().await;
            let inc = insert_task_with_status(&db, "inc", TaskStatus::Incomplete).await;
            let comp = insert_task_with_status(&db, "comp", TaskStatus::Completed).await;
            let canc = insert_task_with_status(&db, "canc", TaskStatus::Canceled).await;

            let tasks = TaskQueryBuilder::new()
                .where_expr(
                    FilterExpr::status(TaskStatus::Incomplete)
                        .or(FilterExpr::status(TaskStatus::Completed)),
                )
                .execute(&db)
                .await
                .unwrap();

            let uuids: std::collections::HashSet<_> =
                tasks.iter().map(|t| t.uuid.clone()).collect();
            assert!(uuids.contains(&inc));
            assert!(uuids.contains(&comp));
            assert!(!uuids.contains(&canc));
        }

        #[tokio::test]
        async fn test_execute_with_where_expr_not_type() {
            use crate::filter_expr::FilterExpr;
            use crate::models::TaskType;
            let (db, _f) = open_test_db().await;
            let todo = insert_task_with_type(&db, "todo", TaskType::Todo).await;
            let project = insert_task_with_type(&db, "project", TaskType::Project).await;

            let tasks = TaskQueryBuilder::new()
                .where_expr(FilterExpr::task_type(TaskType::Project).not())
                .execute(&db)
                .await
                .unwrap();

            let uuids: std::collections::HashSet<_> =
                tasks.iter().map(|t| t.uuid.clone()).collect();
            assert!(uuids.contains(&todo));
            assert!(!uuids.contains(&project));
        }

        #[tokio::test]
        async fn test_execute_pagination_defers_to_rust_when_where_expr_set() {
            // Mirror of test_query_tasks_pagination_with_any_tags. With a
            // where_expr, limit/offset must count post-filter matches.
            use crate::filter_expr::FilterExpr;
            let (db, _f) = open_test_db().await;
            insert_task_with_status(&db, "inc-1", TaskStatus::Incomplete).await;
            insert_task_with_status(&db, "inc-2", TaskStatus::Incomplete).await;
            insert_task_with_status(&db, "inc-3", TaskStatus::Incomplete).await;
            insert_task_with_status(&db, "comp", TaskStatus::Completed).await;

            let page0 = TaskQueryBuilder::new()
                .where_expr(FilterExpr::status(TaskStatus::Incomplete))
                .limit(1)
                .offset(0)
                .execute(&db)
                .await
                .unwrap();
            let page1 = TaskQueryBuilder::new()
                .where_expr(FilterExpr::status(TaskStatus::Incomplete))
                .limit(1)
                .offset(1)
                .execute(&db)
                .await
                .unwrap();
            assert_eq!(page0.len(), 1);
            assert_eq!(page1.len(), 1);
            assert_ne!(page0[0].uuid, page1[0].uuid);
            assert_eq!(page0[0].status, TaskStatus::Incomplete);
            assert_eq!(page1[0].status, TaskStatus::Incomplete);
        }

        #[tokio::test]
        async fn test_execute_combines_where_expr_with_filters_status() {
            // filters.status (SQL) AND-combines with where_expr (Rust).
            use crate::filter_expr::FilterExpr;
            let (db, _f) = open_test_db().await;
            let target = insert_task(&db, "needle", None, &["work"]).await;
            insert_task(&db, "decoy", None, &["work"]).await;

            let tasks = TaskQueryBuilder::new()
                .status(TaskStatus::Incomplete)
                .where_expr(FilterExpr::title_contains("needle"))
                .execute(&db)
                .await
                .unwrap();

            let uuids: std::collections::HashSet<_> =
                tasks.iter().map(|t| t.uuid.clone()).collect();
            assert!(uuids.contains(&target));
            assert_eq!(tasks.len(), 1);
        }

        #[tokio::test]
        async fn test_execute_combines_where_expr_with_any_tags() {
            // Both Rust-side post-filters apply. Tag filter narrows by tag,
            // expr further narrows by title.
            use crate::filter_expr::FilterExpr;
            let (db, _f) = open_test_db().await;
            let target = insert_task(&db, "needle-task", None, &["work"]).await;
            insert_task(&db, "decoy-task", None, &["work"]).await;
            insert_task(&db, "needle-but-wrong-tag", None, &["personal"]).await;

            let tasks = TaskQueryBuilder::new()
                .any_tags(vec!["work".to_string()])
                .where_expr(FilterExpr::title_contains("needle"))
                .execute(&db)
                .await
                .unwrap();

            let uuids: std::collections::HashSet<_> =
                tasks.iter().map(|t| t.uuid.clone()).collect();
            assert!(uuids.contains(&target));
            assert_eq!(tasks.len(), 1);
        }

        #[cfg(feature = "batch-operations")]
        mod cursor_pagination_tests {
            use super::*;

            #[tokio::test]
            async fn test_execute_paged_walks_through_all_tasks() {
                let (db, _f) = open_test_db().await;
                let mut inserted = vec![];
                for i in 0..5 {
                    inserted.push(insert_task(&db, &format!("task-{i}"), None, &[]).await);
                }

                let mut all_collected: Vec<ThingsId> = vec![];
                let mut cursor = None;
                let mut page_count = 0;
                loop {
                    let mut builder = TaskQueryBuilder::new().limit(2);
                    if let Some(c) = cursor.take() {
                        builder = builder.after(c);
                    }
                    let page = builder.execute_paged(&db).await.unwrap();
                    page_count += 1;
                    all_collected.extend(page.items.iter().map(|t| t.uuid.clone()));
                    if let Some(next) = page.next_cursor {
                        cursor = Some(next);
                    } else {
                        break;
                    }
                    assert!(page_count < 10, "runaway pagination loop");
                }

                let inserted_set: std::collections::HashSet<_> = inserted.iter().cloned().collect();
                let collected_set: std::collections::HashSet<_> =
                    all_collected.iter().cloned().collect();
                for uuid in &inserted_set {
                    assert!(collected_set.contains(uuid), "missing inserted uuid {uuid}");
                }
                // Verify no duplicates: use HashSet size comparison.
                assert_eq!(
                    all_collected.len(),
                    collected_set.len(),
                    "duplicates in pages"
                );
            }

            #[tokio::test]
            async fn test_execute_paged_last_page_has_no_next_cursor() {
                let (db, _f) = open_test_db().await;
                insert_task(&db, "only-task", None, &[]).await;
                let page = TaskQueryBuilder::new()
                    .status(TaskStatus::Incomplete)
                    .limit(100)
                    .execute_paged(&db)
                    .await
                    .unwrap();
                assert!(
                    page.next_cursor.is_none(),
                    "non-full page should not have a next cursor"
                );
            }

            #[tokio::test]
            async fn test_execute_paged_with_status_filter() {
                let (db, _f) = open_test_db().await;
                let target = insert_task(&db, "incomplete-task", None, &[]).await;
                let page = TaskQueryBuilder::new()
                    .status(TaskStatus::Incomplete)
                    .limit(50)
                    .execute_paged(&db)
                    .await
                    .unwrap();
                let uuids: std::collections::HashSet<_> =
                    page.items.iter().map(|t| t.uuid.clone()).collect();
                assert!(uuids.contains(&target));
                for task in &page.items {
                    assert_eq!(task.status, TaskStatus::Incomplete);
                }
            }

            #[tokio::test]
            async fn test_execute_paged_with_post_filter_any_tags() {
                let (db, _f) = open_test_db().await;
                let a1 = insert_task_with_tags(&db, "a1", &["a"]).await;
                let a2 = insert_task_with_tags(&db, "a2", &["a"]).await;
                let a3 = insert_task_with_tags(&db, "a3", &["a"]).await;
                let _b = insert_task_with_tags(&db, "b1", &["b"]).await;

                let mut all: Vec<ThingsId> = vec![];
                let mut cursor = None;
                loop {
                    let mut builder = TaskQueryBuilder::new()
                        .any_tags(vec!["a".to_string()])
                        .limit(2);
                    if let Some(c) = cursor.take() {
                        builder = builder.after(c);
                    }
                    let page = builder.execute_paged(&db).await.unwrap();
                    all.extend(page.items.iter().map(|t| t.uuid.clone()));
                    if let Some(n) = page.next_cursor {
                        cursor = Some(n);
                    } else {
                        break;
                    }
                }

                let collected: std::collections::HashSet<_> = all.iter().cloned().collect();
                assert!(collected.contains(&a1));
                assert!(collected.contains(&a2));
                assert!(collected.contains(&a3));
                assert_eq!(collected.len(), 3, "should contain only a-tagged tasks");
            }

            #[tokio::test]
            async fn test_execute_paged_default_page_size_when_no_limit() {
                let (db, _f) = open_test_db().await;
                // Confirm a builder without `.limit()` doesn't error and returns a Page.
                // Default page size is 100 — with the empty test DB, all tasks fit and
                // we expect no next_cursor.
                let page = TaskQueryBuilder::new()
                    .status(TaskStatus::Incomplete)
                    .execute_paged(&db)
                    .await
                    .unwrap();
                assert!(page.items.len() <= 100);
                assert!(page.next_cursor.is_none());
            }

            #[tokio::test]
            async fn test_query_tasks_order_is_deterministic_by_uuid_tiebreak() {
                let (db, _f) = open_test_db().await;
                // insert_task hardcodes creationDate = 0, so all tasks tie. ORDER BY
                // uuid DESC gives a deterministic tiebreak.
                for i in 0..3 {
                    insert_task(&db, &format!("dup-time-{i}"), None, &[]).await;
                }
                let first = db.query_tasks(&TaskFilters::default()).await.unwrap();
                let second = db.query_tasks(&TaskFilters::default()).await.unwrap();
                let first_uuids: Vec<_> = first.iter().map(|t| t.uuid.clone()).collect();
                let second_uuids: Vec<_> = second.iter().map(|t| t.uuid.clone()).collect();
                assert_eq!(
                    first_uuids, second_uuids,
                    "tied-creationDate ordering should be deterministic"
                );
            }
        }

        #[cfg(feature = "batch-operations")]
        mod cursor_streaming_tests {
            use super::*;
            use futures_util::{StreamExt, TryStreamExt};

            #[tokio::test]
            async fn test_execute_stream_yields_all_tasks() {
                let (db, _f) = open_test_db().await;
                let mut inserted = vec![];
                for i in 0..5 {
                    inserted.push(insert_task(&db, &format!("task-{i}"), None, &[]).await);
                }

                let collected: Vec<_> = TaskQueryBuilder::new()
                    .limit(2)
                    .execute_stream(&db)
                    .try_collect::<Vec<_>>()
                    .await
                    .unwrap();

                let inserted_set: std::collections::HashSet<_> = inserted.iter().cloned().collect();
                let collected_set: std::collections::HashSet<_> =
                    collected.iter().map(|t| t.uuid.clone()).collect();
                for uuid in &inserted_set {
                    assert!(
                        collected_set.contains(uuid),
                        "stream missing inserted uuid {uuid}"
                    );
                }
                assert_eq!(
                    collected.len(),
                    collected_set.len(),
                    "stream yielded duplicates"
                );
            }

            #[tokio::test]
            async fn test_execute_stream_with_status_filter() {
                let (db, _f) = open_test_db().await;
                let target = insert_task(&db, "incomplete-task", None, &[]).await;

                let tasks = TaskQueryBuilder::new()
                    .status(TaskStatus::Incomplete)
                    .limit(50)
                    .execute_stream(&db)
                    .try_collect::<Vec<_>>()
                    .await
                    .unwrap();

                let uuids: std::collections::HashSet<_> =
                    tasks.iter().map(|t| t.uuid.clone()).collect();
                assert!(uuids.contains(&target));
                for task in &tasks {
                    assert_eq!(task.status, TaskStatus::Incomplete);
                }
            }

            #[tokio::test]
            async fn test_execute_stream_with_any_tags_post_filter() {
                let (db, _f) = open_test_db().await;
                let a1 = insert_task_with_tags(&db, "a1", &["a"]).await;
                let a2 = insert_task_with_tags(&db, "a2", &["a"]).await;
                let a3 = insert_task_with_tags(&db, "a3", &["a"]).await;
                let _b = insert_task_with_tags(&db, "b1", &["b"]).await;

                let tasks = TaskQueryBuilder::new()
                    .any_tags(vec!["a".to_string()])
                    .limit(2)
                    .execute_stream(&db)
                    .try_collect::<Vec<_>>()
                    .await
                    .unwrap();

                let uuids: std::collections::HashSet<_> =
                    tasks.iter().map(|t| t.uuid.clone()).collect();
                assert!(uuids.contains(&a1));
                assert!(uuids.contains(&a2));
                assert!(uuids.contains(&a3));
                assert_eq!(uuids.len(), 3, "should yield only a-tagged tasks");
            }

            #[tokio::test]
            async fn test_execute_stream_empty_result() {
                let (db, _f) = open_test_db().await;
                // Filter by a project UUID that doesn't exist → no matches.
                let tasks = TaskQueryBuilder::new()
                    .project_uuid(ThingsId::new_v4())
                    .execute_stream(&db)
                    .try_collect::<Vec<_>>()
                    .await
                    .unwrap();
                assert!(tasks.is_empty());
            }

            #[tokio::test]
            async fn test_execute_stream_rejects_fuzzy_search() {
                let (db, _f) = open_test_db().await;
                // fuzzy_search alone (no explicit .after()) must reject immediately.
                // Previously execute_paged only guarded fuzzy_query && after.is_some(),
                // so page 1 would silently return un-scored results and only page 2
                // would error. This test catches that regression.
                let mut stream = TaskQueryBuilder::new()
                    .fuzzy_search("anything")
                    .execute_stream(&db);
                match stream.next().await {
                    Some(Err(crate::error::ThingsError::InvalidCursor(msg))) => {
                        assert!(msg.contains("fuzzy"), "msg: {msg}");
                    }
                    other => panic!("expected first item to be InvalidCursor, got {other:?}"),
                }
                assert!(stream.next().await.is_none());
            }

            #[tokio::test]
            async fn test_execute_stream_cross_page_ordering() {
                let (db, _f) = open_test_db().await;
                for i in 0..5 {
                    insert_task(&db, &format!("task-{i}"), None, &[]).await;
                }

                // Stream with chunk size 2 — cursor advances across 3 pages.
                let stream_uuids: Vec<ThingsId> = TaskQueryBuilder::new()
                    .limit(2)
                    .execute_stream(&db)
                    .try_collect::<Vec<_>>()
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|t| t.uuid)
                    .collect();

                // Single full query — same ORDER BY, no pagination.
                let full_uuids: Vec<ThingsId> = db
                    .query_tasks(&TaskFilters::default())
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|t| t.uuid)
                    .collect();

                // Every streamed UUID must appear in the full query result, and
                // their relative order must be the same.
                let stream_set: std::collections::HashSet<_> =
                    stream_uuids.iter().cloned().collect();
                let filtered_full: Vec<ThingsId> = full_uuids
                    .into_iter()
                    .filter(|u| stream_set.contains(u))
                    .collect();
                assert_eq!(
                    stream_uuids, filtered_full,
                    "stream ordering should agree with full-query (creationDate DESC, uuid DESC)"
                );
            }
        }
    }
}
