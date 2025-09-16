//! Integration tests for things-core

use things_core::{ThingsDatabase, Result};

#[tokio::test]
async fn test_database_connection() -> Result<()> {
    // This test will only work if Things 3 is installed and has data
    // For now, we'll just test that we can create a database instance
    let db_path = ThingsDatabase::default_path();
    println!("Testing database connection to: {}", db_path);
    
    // Try to connect (this might fail in CI, which is expected)
    match ThingsDatabase::new(&db_path) {
        Ok(_db) => {
            println!("✅ Database connection successful");
        }
        Err(e) => {
            println!("⚠️  Database connection failed (expected in CI): {}", e);
            // Don't fail the test in CI environments
            if std::env::var("CI").is_ok() {
                println!("Skipping test in CI environment");
                return Ok(());
            }
            return Err(e);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_default_database_path() {
    let path = ThingsDatabase::default_path();
    assert!(path.ends_with("main.sqlite"));
    assert!(path.contains("Things Database.thingsdatabase"));
}
