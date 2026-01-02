//! Entity validation utilities for database operations
//!
//! This module provides centralized validation functions to ensure
//! referenced entities (tasks, projects, areas) exist before performing operations.

use crate::error::{Result as ThingsResult, ThingsError};
use sqlx::SqlitePool;
use tracing::instrument;
use uuid::Uuid;

/// Validate that a task exists and is not trashed
///
/// # Errors
///
/// Returns an error if the task does not exist, is trashed, or if the database query fails
#[instrument(skip(pool))]
pub async fn validate_task_exists(pool: &SqlitePool, uuid: &Uuid) -> ThingsResult<()> {
    let exists = sqlx::query("SELECT 1 FROM TMTask WHERE uuid = ? AND trashed = 0")
        .bind(uuid.to_string())
        .fetch_optional(pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to validate task: {e}")))?
        .is_some();

    if !exists {
        return Err(ThingsError::unknown(format!("Task not found: {uuid}")));
    }
    Ok(())
}

/// Validate that a project exists (project is a task with type = 1)
///
/// # Errors
///
/// Returns an error if the project does not exist, is trashed, or if the database query fails
#[instrument(skip(pool))]
pub async fn validate_project_exists(pool: &SqlitePool, uuid: &Uuid) -> ThingsResult<()> {
    let exists = sqlx::query("SELECT 1 FROM TMTask WHERE uuid = ? AND type = 1 AND trashed = 0")
        .bind(uuid.to_string())
        .fetch_optional(pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to validate project: {e}")))?
        .is_some();

    if !exists {
        return Err(ThingsError::unknown(format!("Project not found: {uuid}")));
    }
    Ok(())
}

/// Validate that an area exists
///
/// # Errors
///
/// Returns an error if the area does not exist or if the database query fails
#[instrument(skip(pool))]
pub async fn validate_area_exists(pool: &SqlitePool, uuid: &Uuid) -> ThingsResult<()> {
    let exists = sqlx::query("SELECT 1 FROM TMArea WHERE uuid = ?")
        .bind(uuid.to_string())
        .fetch_optional(pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to validate area: {e}")))?
        .is_some();

    if !exists {
        return Err(ThingsError::unknown(format!("Area not found: {uuid}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "test-utils")]
    #[tokio::test]
    async fn test_validate_nonexistent_task() {
        use crate::test_utils::create_test_database;
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).await.unwrap();

        let pool = sqlx::SqlitePool::connect(&format!("sqlite://{}", db_path.display()))
            .await
            .unwrap();

        let nonexistent_uuid = Uuid::new_v4();
        let result = validate_task_exists(&pool, &nonexistent_uuid).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Task not found"));
    }

    #[cfg(feature = "test-utils")]
    #[tokio::test]
    async fn test_validate_nonexistent_project() {
        use crate::test_utils::create_test_database;
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).await.unwrap();

        let pool = sqlx::SqlitePool::connect(&format!("sqlite://{}", db_path.display()))
            .await
            .unwrap();

        let nonexistent_uuid = Uuid::new_v4();
        let result = validate_project_exists(&pool, &nonexistent_uuid).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Project not found"));
    }

    #[cfg(feature = "test-utils")]
    #[tokio::test]
    async fn test_validate_nonexistent_area() {
        use crate::test_utils::create_test_database;
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).await.unwrap();

        let pool = sqlx::SqlitePool::connect(&format!("sqlite://{}", db_path.display()))
            .await
            .unwrap();

        let nonexistent_uuid = Uuid::new_v4();
        let result = validate_area_exists(&pool, &nonexistent_uuid).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Area not found"));
    }
}
