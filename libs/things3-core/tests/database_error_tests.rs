//! Database error handling tests
//!
//! Tests various error scenarios for database operations to ensure
//! proper error handling and graceful degradation.

use std::path::PathBuf;
use tempfile::{NamedTempFile, TempDir};
use things3_core::ThingsDatabase;

/// Test that connecting to a non-existent database fails gracefully
#[tokio::test]
async fn test_database_not_found() {
    let nonexistent_path = PathBuf::from("/nonexistent/path/to/database.db");

    let result = ThingsDatabase::new(&nonexistent_path).await;

    assert!(result.is_err(), "Should fail when database doesn't exist");

    // Verify error message is helpful
    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("Failed to connect") || err_msg.contains("database"),
        "Error message should mention connection or database: {}",
        err_msg
    );
}

/// Test that connecting to a directory (not a file) fails gracefully
#[tokio::test]
async fn test_database_path_is_directory() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    let result = ThingsDatabase::new(dir_path).await;

    assert!(result.is_err(), "Should fail when path is a directory");
}

/// Test that connecting to an empty file fails gracefully
#[tokio::test]
async fn test_database_empty_file() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    // Empty file exists but is not a valid SQLite database
    let result = ThingsDatabase::new(db_path).await;

    // Connection might succeed (SQLite can initialize), but queries should fail
    match result {
        Ok(db) => {
            // If connection succeeds, test that queries fail gracefully
            let inbox_result = db.get_inbox(Some(10)).await;
            assert!(
                inbox_result.is_err(),
                "Queries should fail on invalid/empty database"
            );
        }
        Err(_) => {
            // Connection failure is also acceptable
        }
    }
}

/// Test that a corrupted database file is handled gracefully
#[tokio::test]
async fn test_database_corrupted_file() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    // Write garbage data to simulate corruption
    std::fs::write(db_path, b"This is not a valid SQLite database file!").unwrap();

    let result = ThingsDatabase::new(db_path).await;

    // Should fail to connect or queries should fail
    match result {
        Ok(db) => {
            let inbox_result = db.get_inbox(Some(10)).await;
            assert!(
                inbox_result.is_err(),
                "Queries should fail on corrupted database"
            );
        }
        Err(_) => {
            // Connection failure is expected for corrupted files
        }
    }
}

/// Test that invalid database path characters are handled
#[tokio::test]
async fn test_database_invalid_path_characters() {
    // Test with null bytes (invalid in paths)
    let invalid_path = PathBuf::from("invalid\0path.db");

    let result = ThingsDatabase::new(&invalid_path).await;

    assert!(result.is_err(), "Should fail with invalid path characters");
}

/// Test that queries fail gracefully when database schema is wrong
#[tokio::test]
async fn test_database_wrong_schema() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    // Create a valid SQLite database but with wrong schema
    let pool = sqlx::sqlite::SqlitePool::connect(&format!("sqlite:{}", db_path.display()))
        .await
        .unwrap();

    // Create tables with wrong schema
    sqlx::query("CREATE TABLE wrong_table (id INTEGER PRIMARY KEY, data TEXT)")
        .execute(&pool)
        .await
        .unwrap();

    pool.close().await;

    // Now try to use it with ThingsDatabase
    let result = ThingsDatabase::new(db_path).await;

    match result {
        Ok(db) => {
            // Connection succeeds, but queries should fail
            let inbox_result = db.get_inbox(Some(10)).await;
            assert!(
                inbox_result.is_err(),
                "Queries should fail when schema is wrong"
            );

            let today_result = db.get_today(Some(10)).await;
            assert!(
                today_result.is_err(),
                "Today query should fail when schema is wrong"
            );

            let projects_result = db.get_projects(Some(10)).await;
            assert!(
                projects_result.is_err(),
                "Projects query should fail when schema is wrong"
            );
        }
        Err(_) => {
            // Connection failure is also acceptable
        }
    }
}

/// Test that database connection with invalid connection string fails
#[tokio::test]
async fn test_database_invalid_connection_string() {
    let invalid_conn_str = "invalid://connection/string";

    let result = ThingsDatabase::from_connection_string(invalid_conn_str).await;

    assert!(
        result.is_err(),
        "Should fail with invalid connection string"
    );
}

/// Test error handling when database file becomes inaccessible during operation
#[tokio::test]
async fn test_database_file_removed_during_operation() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_path_buf();

    // Create a valid (empty) database first
    {
        let pool = sqlx::sqlite::SqlitePool::connect(&format!("sqlite:{}", db_path.display()))
            .await
            .unwrap();

        // Create minimal schema
        sqlx::query("CREATE TABLE IF NOT EXISTS TMTask (uuid TEXT PRIMARY KEY, title TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        pool.close().await;
    }

    // Connect to database
    let db = ThingsDatabase::new(&db_path).await.unwrap();

    // Remove the file while connection is open
    std::fs::remove_file(&db_path).unwrap();

    // Queries should fail gracefully
    let result = db.get_inbox(Some(10)).await;
    assert!(result.is_err(), "Should fail when database file is removed");
}

/// Test that connection health check works correctly
#[tokio::test]
async fn test_database_health_check_on_invalid_db() {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    // Create minimal valid database
    let pool = sqlx::sqlite::SqlitePool::connect(&format!("sqlite:{}", db_path.display()))
        .await
        .unwrap();
    pool.close().await;

    let db = ThingsDatabase::new(db_path).await.unwrap();

    // Health check should work even on minimal database
    let is_connected = db.is_connected().await;
    assert!(
        is_connected,
        "Health check should succeed on valid connection"
    );
}

/// Test error handling for extremely long database paths
#[tokio::test]
async fn test_database_extremely_long_path() {
    // Create a path that exceeds typical filesystem limits
    let long_component = "a".repeat(300);
    let long_path = PathBuf::from(format!("/tmp/{}/{}", long_component, long_component));

    let result = ThingsDatabase::new(&long_path).await;

    assert!(result.is_err(), "Should fail with extremely long path");
}
