//! Common test utilities and setup functions for MCP tests

use sqlx::SqlitePool;
pub(crate) use things3_cli::mcp::ThingsMcpServer;
use things3_core::{config::ThingsConfig, database::ThingsDatabase};

/// Create a test MCP server with mock database
pub(crate) async fn create_test_mcp_server() -> ThingsMcpServer {
    // Use in-memory database for testing
    let db = ThingsDatabase::from_connection_string("sqlite::memory:")
        .await
        .unwrap();

    // Create the database schema
    create_test_schema(&db).await.unwrap();

    let config = ThingsConfig::for_testing().unwrap();
    ThingsMcpServer::new(db.into(), config)
}

/// Create the test database schema
async fn create_test_schema(db: &ThingsDatabase) -> Result<(), Box<dyn std::error::Error>> {
    let pool = db.pool();

    // Create the Things 3 schema
    sqlx::query(
        r"
        -- TMTask table (main tasks table) - matches real Things 3 schema
        CREATE TABLE IF NOT EXISTS TMTask (
            uuid TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            type INTEGER NOT NULL DEFAULT 0,
            status INTEGER NOT NULL DEFAULT 0,
            notes TEXT,
            startDate INTEGER,
            deadline INTEGER,
            stopDate REAL,
            creationDate REAL NOT NULL,
            userModificationDate REAL NOT NULL,
            project TEXT,
            area TEXT,
            heading TEXT,
            trashed INTEGER NOT NULL DEFAULT 0,
            tags TEXT DEFAULT '[]',
            cachedTags BLOB,
            todayIndex INTEGER
        )
        ",
    )
    .execute(pool)
    .await?;

    // Note: Projects are stored in TMTask table with type=1, no separate TMProject table

    sqlx::query(
        r"
        -- TMArea table (areas table)
        CREATE TABLE IF NOT EXISTS TMArea (
            uuid TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            visible INTEGER NOT NULL DEFAULT 1,
            'index' INTEGER NOT NULL DEFAULT 0,
            creationDate REAL NOT NULL,
            userModificationDate REAL NOT NULL
        )
        ",
    )
    .execute(pool)
    .await?;

    // Create TMTag table
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS TMTag (
            uuid TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            shortcut TEXT,
            parent TEXT,
            creationDate REAL NOT NULL,
            userModificationDate REAL NOT NULL,
            usedDate REAL,
            'index' INTEGER NOT NULL DEFAULT 0
        )
        ",
    )
    .execute(pool)
    .await?;

    // Insert test data
    insert_test_data(pool).await?;

    Ok(())
}

/// Insert test data into the database
async fn insert_test_data(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    use chrono::Utc;
    use uuid::Uuid;

    let _now = Utc::now().to_rfc3339(); // Keep for potential future use

    // Generate valid UUIDs for test data
    let area_uuid = Uuid::new_v4().to_string();
    let project_uuid = Uuid::new_v4().to_string();
    let task_uuid = Uuid::new_v4().to_string();

    // Insert test areas
    let now = chrono::Utc::now().timestamp() as f64;
    sqlx::query("INSERT INTO TMArea (uuid, title, visible, 'index', creationDate, userModificationDate) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&area_uuid)
        .bind("Work")
        .bind(1) // Visible
        .bind(0) // Index
        .bind(now) // creationDate
        .bind(now) // userModificationDate
        .execute(pool)
        .await?;

    // Insert test projects (stored in TMTask with type=1)
    let now_timestamp = 1_700_000_000.0; // Test timestamp
    sqlx::query(
        "INSERT INTO TMTask (uuid, title, type, status, area, creationDate, userModificationDate, trashed) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&project_uuid)
    .bind("Website Redesign")
    .bind(1) // Project type
    .bind(0) // Incomplete
    .bind(&area_uuid)
    .bind(now_timestamp)
    .bind(now_timestamp)
    .bind(0) // Not trashed
    .execute(pool).await?;

    // Insert test tasks - one in inbox (no project), one in project
    let inbox_task_uuid = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO TMTask (uuid, title, type, status, project, creationDate, userModificationDate, trashed) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&inbox_task_uuid)
    .bind("Inbox Task")
    .bind(0) // Todo type
    .bind(0) // Incomplete
    .bind::<Option<String>>(None) // No project (inbox)
    .bind(now_timestamp)
    .bind(now_timestamp)
    .bind(0) // Not trashed
    .execute(pool).await?;

    sqlx::query(
        "INSERT INTO TMTask (uuid, title, type, status, project, creationDate, userModificationDate, trashed) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&task_uuid)
    .bind("Research competitors")
    .bind(0) // Todo type
    .bind(0) // Incomplete
    .bind(&project_uuid)
    .bind(now_timestamp)
    .bind(now_timestamp)
    .bind(0) // Not trashed
    .execute(pool).await?;

    Ok(())
}
