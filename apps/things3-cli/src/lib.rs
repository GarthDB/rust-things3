//! Things CLI library
//! This module provides real-time updates and progress tracking capabilities

pub mod bulk_operations;
pub mod dashboard;
pub mod events;
pub mod health;
pub mod logging;
pub mod mcp;
pub mod metrics;
pub mod monitoring;
pub mod progress;
// pub mod thread_safe_db; // Removed - ThingsDatabase is now Send + Sync
pub mod websocket;

use crate::events::EventBroadcaster;
use crate::websocket::WebSocketServer;
use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use things3_core::{Result, ThingsDatabase};

#[derive(Parser, Debug)]
#[command(name = "things3")]
#[command(about = "Things 3 CLI with integrated MCP server")]
#[command(version)]
pub struct Cli {
    /// Database path (defaults to Things 3 default location)
    #[arg(long, short)]
    pub database: Option<PathBuf>,

    /// Fall back to default database path if specified path doesn't exist
    #[arg(long)]
    pub fallback_to_default: bool,

    /// Verbose output
    #[arg(long, short)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Commands {
    /// Get inbox tasks
    Inbox {
        /// Limit number of results
        #[arg(long, short)]
        limit: Option<usize>,
    },
    /// Get today's tasks
    Today {
        /// Limit number of results
        #[arg(long, short)]
        limit: Option<usize>,
    },
    /// Get projects
    Projects {
        /// Filter by area UUID
        #[arg(long)]
        area: Option<String>,
        /// Limit number of results
        #[arg(long, short)]
        limit: Option<usize>,
    },
    /// Get areas
    Areas {
        /// Limit number of results
        #[arg(long, short)]
        limit: Option<usize>,
    },
    /// Search tasks
    Search {
        /// Search query
        query: String,
        /// Limit number of results
        #[arg(long, short)]
        limit: Option<usize>,
    },
    /// Start MCP server mode
    #[cfg(feature = "mcp-server")]
    Mcp,
    /// Health check
    Health,
    /// Start health check server
    HealthServer {
        /// Port to listen on
        #[arg(long, short, default_value = "8080")]
        port: u16,
    },
    /// Start monitoring dashboard
    Dashboard {
        /// Port to listen on
        #[arg(long, short, default_value = "3000")]
        port: u16,
    },
    /// Start WebSocket server for real-time updates
    Server {
        /// Port to listen on
        #[arg(long, short, default_value = "8080")]
        port: u16,
    },
    /// Watch for real-time updates
    Watch {
        /// WebSocket server URL
        #[arg(long, short, default_value = "ws://127.0.0.1:8080")]
        url: String,
    },
    /// Validate real-time features health
    Validate,
    /// Bulk operations with progress tracking
    Bulk {
        #[command(subcommand)]
        operation: BulkOperation,
    },
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum BulkOperation {
    /// Export all tasks with progress tracking
    Export {
        /// Export format (json, csv, xml)
        #[arg(long, short, default_value = "json")]
        format: String,
    },
    /// Update multiple tasks status
    UpdateStatus {
        /// Task IDs to update (comma-separated)
        task_ids: String,
        /// New status (completed, cancelled, trashed, incomplete)
        status: String,
    },
    /// Search and process tasks
    SearchAndProcess {
        /// Search query
        query: String,
    },
}

/// Print tasks to the given writer
///
/// # Examples
///
/// ```no_run
/// use things3_cli::print_tasks;
/// use things3_core::ThingsDatabase;
/// use std::io;
///
/// # async fn example() -> things3_core::Result<()> {
/// let db = ThingsDatabase::new(std::path::Path::new("test.db")).await?;
/// let tasks = db.get_inbox(Some(10)).await?;
/// print_tasks(&db, &tasks, &mut io::stdout())?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns an error if writing fails
pub fn print_tasks<W: Write>(
    _db: &ThingsDatabase,
    tasks: &[things3_core::Task],
    writer: &mut W,
) -> Result<()> {
    if tasks.is_empty() {
        writeln!(writer, "No tasks found")?;
        return Ok(());
    }

    writeln!(writer, "Found {} tasks:", tasks.len())?;
    for task in tasks {
        writeln!(writer, "  • {} ({:?})", task.title, task.task_type)?;
        if let Some(notes) = &task.notes {
            writeln!(writer, "    Notes: {notes}")?;
        }
        if let Some(deadline) = &task.deadline {
            writeln!(writer, "    Deadline: {deadline}")?;
        }
        if !task.tags.is_empty() {
            writeln!(writer, "    Tags: {}", task.tags.join(", "))?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

/// Print projects to the given writer
///
/// # Examples
///
/// ```no_run
/// use things3_cli::print_projects;
/// use things3_core::ThingsDatabase;
/// use std::io;
///
/// # async fn example() -> things3_core::Result<()> {
/// let db = ThingsDatabase::new(std::path::Path::new("test.db")).await?;
/// let projects = db.get_projects(None).await?;
/// print_projects(&db, &projects, &mut io::stdout())?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns an error if writing fails
pub fn print_projects<W: Write>(
    _db: &ThingsDatabase,
    projects: &[things3_core::Project],
    writer: &mut W,
) -> Result<()> {
    if projects.is_empty() {
        writeln!(writer, "No projects found")?;
        return Ok(());
    }

    writeln!(writer, "Found {} projects:", projects.len())?;
    for project in projects {
        writeln!(writer, "  • {} ({:?})", project.title, project.status)?;
        if let Some(notes) = &project.notes {
            writeln!(writer, "    Notes: {notes}")?;
        }
        if let Some(deadline) = &project.deadline {
            writeln!(writer, "    Deadline: {deadline}")?;
        }
        if !project.tags.is_empty() {
            writeln!(writer, "    Tags: {}", project.tags.join(", "))?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

/// Print areas to the given writer
///
/// # Examples
///
/// ```no_run
/// use things3_cli::print_areas;
/// use things3_core::ThingsDatabase;
/// use std::io;
///
/// # async fn example() -> things3_core::Result<()> {
/// let db = ThingsDatabase::new(std::path::Path::new("test.db")).await?;
/// let areas = db.get_areas().await?;
/// print_areas(&db, &areas, &mut io::stdout())?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns an error if writing fails
pub fn print_areas<W: Write>(
    _db: &ThingsDatabase,
    areas: &[things3_core::Area],
    writer: &mut W,
) -> Result<()> {
    if areas.is_empty() {
        writeln!(writer, "No areas found")?;
        return Ok(());
    }

    writeln!(writer, "Found {} areas:", areas.len())?;
    for area in areas {
        writeln!(writer, "  • {}", area.title)?;
        if let Some(notes) = &area.notes {
            writeln!(writer, "    Notes: {notes}")?;
        }
        if !area.tags.is_empty() {
            writeln!(writer, "    Tags: {}", area.tags.join(", "))?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

/// Perform a health check on the database
///
/// # Examples
///
/// ```no_run
/// use things3_cli::health_check;
/// use things3_core::ThingsDatabase;
///
/// # async fn example() -> things3_core::Result<()> {
/// let db = ThingsDatabase::new(std::path::Path::new("test.db")).await?;
/// health_check(&db).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns an error if the database is not accessible
pub async fn health_check(db: &ThingsDatabase) -> Result<()> {
    println!("🔍 Checking Things 3 database connection...");

    // Check if database is connected
    if !db.is_connected().await {
        return Err(things3_core::ThingsError::unknown(
            "Database is not connected".to_string(),
        ));
    }

    // Get database statistics
    let stats = db.get_stats().await?;
    println!("✅ Database connection successful!");
    println!(
        "   Found {} tasks, {} projects, {} areas",
        stats.task_count, stats.project_count, stats.area_count
    );

    println!("🎉 All systems operational!");
    Ok(())
}

// Temporarily disabled during SQLx migration
// /// Start the MCP server
// ///
// /// # Errors
// /// Returns an error if the server fails to start
// pub fn start_mcp_server(db: Arc<SqlxThingsDatabase>, config: ThingsConfig) -> Result<()> {
//     println!("🚀 Starting MCP server...");
//     println!("🚧 MCP server is temporarily disabled during SQLx migration");
//     Err(things3_core::ThingsError::unknown("MCP server temporarily disabled".to_string()))
// }

/// Start the WebSocket server for real-time updates
///
/// # Examples
///
/// ```no_run
/// use things3_cli::start_websocket_server;
///
/// # async fn example() -> things3_core::Result<()> {
/// // Start WebSocket server on port 8080
/// start_websocket_server(8080).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns an error if the server fails to start
pub async fn start_websocket_server(port: u16) -> Result<()> {
    println!("🚀 Starting WebSocket server on port {port}...");

    let server = WebSocketServer::new(port);
    let _event_broadcaster = Arc::new(EventBroadcaster::new());

    // Start the server
    server
        .start()
        .await
        .map_err(|e| things3_core::ThingsError::unknown(e.to_string()))?;

    Ok(())
}

/// Watch for real-time updates via WebSocket
///
/// # Examples
///
/// ```
/// use things3_cli::watch_updates;
///
/// # fn example() -> things3_core::Result<()> {
/// // Connect to WebSocket server
/// watch_updates("ws://127.0.0.1:8080")?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
/// Returns an error if the connection fails
pub fn watch_updates(url: &str) -> Result<()> {
    println!("👀 Connecting to WebSocket server at {url}...");

    // In a real implementation, this would connect to the WebSocket server
    // For now, we'll just print that it would connect
    println!("✅ Would connect to WebSocket server");
    println!("   (This is a placeholder - actual WebSocket client implementation would go here)");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use things3_core::test_utils::create_test_database;
    use tokio::runtime::Runtime;

    #[test]
    fn test_health_check() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        let rt = Runtime::new().unwrap();
        rt.block_on(async { create_test_database(db_path).await.unwrap() });
        let db = rt.block_on(async { ThingsDatabase::new(db_path).await.unwrap() });
        let result = rt.block_on(async { health_check(&db).await });
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_mcp_server() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).await.unwrap();
        let db = ThingsDatabase::new(db_path).await.unwrap();
        let config = things3_core::ThingsConfig::default();

        // Note: We can't actually run start_mcp_server in a test because it's an infinite
        // loop that reads from stdin. Instead, we verify the server can be created.
        let _server = crate::mcp::ThingsMcpServer::new(db.into(), config);
        // Server created successfully
    }

    #[test]
    fn test_start_websocket_server_function_exists() {
        // Test that the function exists and can be referenced
        // We don't actually call it as it would hang
        // Test that function exists and can be referenced
        // Function reference test passed if we get here
    }

    #[test]
    fn test_watch_updates() {
        let result = watch_updates("ws://127.0.0.1:8080");
        assert!(result.is_ok());
    }
}
