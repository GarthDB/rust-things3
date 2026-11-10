#![allow(deprecated)]

use crate::{
    database::pool::{apply_sqlite_optimizations, DatabasePoolConfig},
    error::{Result as ThingsResult, ThingsError},
};
use sqlx::{pool::PoolOptions, SqlitePool};
use std::path::Path;
use tracing::{info, instrument};

/// SQLite-backed implementation of [`ThingsDatabase`](crate::database::ThingsDatabase).
///
/// This is the production database implementation.  Use
/// [`ThingsDatabase`](crate::database::ThingsDatabase) as the trait type when
/// you need to accept any backend (e.g. the in-memory test fixture).
#[derive(Debug, Clone)]
pub struct SqliteThingsDatabase {
    pub(crate) pool: SqlitePool,
    pub(crate) config: DatabasePoolConfig,
}

impl SqliteThingsDatabase {
    /// Create a new database connection pool with default configuration
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use things3_core::{SqliteThingsDatabase, ThingsError};
    /// use std::path::Path;
    ///
    /// # async fn example() -> Result<(), ThingsError> {
    /// // Connect to Things 3 database
    /// let db = SqliteThingsDatabase::new(Path::new("/path/to/things.db")).await?;
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
    /// use things3_core::{SqliteThingsDatabase, DatabasePoolConfig, ThingsError};
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
    /// let db = SqliteThingsDatabase::new_with_config(
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

        apply_sqlite_optimizations(&pool, &config.sqlite_optimizations).await?;

        info!(
            "Database connection pool established successfully with {} max connections",
            config.max_connections
        );

        Ok(Self { pool, config })
    }

    /// Create a connection pool from a URL string with default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the database connection fails or if `SQLite` configuration fails
    #[instrument]
    pub async fn from_connection_string(database_url: &str) -> ThingsResult<Self> {
        Self::from_connection_string_with_config(database_url, DatabasePoolConfig::default()).await
    }

    /// Create a connection pool from a URL string with custom configuration.
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

        apply_sqlite_optimizations(&pool, &config.sqlite_optimizations).await?;

        info!(
            "Database connection pool established successfully with {} max connections",
            config.max_connections
        );

        Ok(Self { pool, config })
    }

    /// Get the underlying SQLite connection pool.
    ///
    /// This is an inherent method specific to `SqliteThingsDatabase` and is not
    /// part of the [`ThingsDatabase`](crate::database::ThingsDatabase) trait.
    #[must_use]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
