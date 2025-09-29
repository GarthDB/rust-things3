//! Thread-safe wrapper for ThingsDatabase
//! 
//! This module provides a thread-safe wrapper around ThingsDatabase
//! that can be safely shared between threads for use in web servers.

use std::sync::Arc;
use things3_core::{ThingsDatabase, ThingsError};

/// Thread-safe wrapper for ThingsDatabase
#[derive(Clone)]
pub struct ThreadSafeDatabase {
    inner: Arc<ThingsDatabase>,
}

impl ThreadSafeDatabase {
    /// Create a new thread-safe database wrapper
    pub fn new(db: Arc<ThingsDatabase>) -> Self {
        Self {
            inner: db,
        }
    }

    /// Check if the database is connected
    pub async fn is_connected(&self) -> bool {
        // For now, we'll assume it's always connected
        // In a real implementation, this would check the actual connection
        true
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats, ThingsError> {
        // In a real implementation, this would return actual stats
        Ok(DatabaseStats {
            is_connected: true,
            connection_count: 1,
            last_query_time: None,
        })
    }
}

/// Database statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct DatabaseStats {
    pub is_connected: bool,
    pub connection_count: u32,
    pub last_query_time: Option<String>,
}
