//! Schema inspection test to understand the real Things 3 database structure

use rusqlite::Connection;
use things_core::{Result, ThingsDatabase};

#[tokio::test]
async fn test_inspect_things_schema() -> Result<()> {
    let db_path = ThingsDatabase::default_path();
    println!("Inspecting database at: {db_path}");

    match Connection::open(&db_path) {
        Ok(conn) => {
            println!("✅ Successfully connected to Things 3 database");

            // Get table names
            let mut stmt =
                conn.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
            let tables: Vec<String> = stmt
                .query_map([], |row| row.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            println!("📋 Tables found: {tables:?}");

            // Inspect TMTask table structure
            if tables.contains(&"TMTask".to_string()) {
                let mut stmt = conn.prepare("PRAGMA table_info(TMTask)")?;
                let columns: Vec<(i32, String, String, i32, Option<String>, i32)> = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get(0)?, // cid
                            row.get(1)?, // name
                            row.get(2)?, // type
                            row.get(3)?, // notnull
                            row.get(4)?, // dflt_value
                            row.get(5)?, // pk
                        ))
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;

                println!("📊 TMTask columns:");
                for (_, name, col_type, notnull, default, pk) in columns {
                    println!("  - {name}: {col_type} (notnull: {notnull}, pk: {pk}, default: {default:?})");
                }
            }

            // Check if there are any tasks
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM TMTask")?;
            let count: i64 = stmt.query_row([], |row| row.get(0))?;
            println!("📈 Total tasks in database: {count}");

            // Sample a few UUIDs to see the format
            let mut stmt = conn.prepare("SELECT uuid FROM TMTask LIMIT 3")?;
            let uuids: Vec<String> = stmt
                .query_map([], |row| row.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            println!("🔍 Sample UUIDs: {uuids:?}");
        }
        Err(e) => {
            println!("⚠️  Could not connect to Things 3 database: {e}");
            println!("This is expected in CI environments or when Things 3 is not installed");
        }
    }

    Ok(())
}
