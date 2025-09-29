//! Things CLI library
//! This module provides real-time updates and progress tracking capabilities

pub mod bulk_operations;
pub mod events;
pub mod mcp;
pub mod monitoring;
pub mod progress;
pub mod websocket;

use crate::events::EventBroadcaster;
use crate::websocket::WebSocketServer;
use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use things3_core::{Result, ThingsConfig, ThingsDatabase};

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
    Mcp,
    /// Health check
    Health,
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
/// # Errors
/// Returns an error if the database is not accessible
pub fn health_check(db: &ThingsDatabase) -> Result<()> {
    println!("🔍 Checking Things 3 database connection...");

    // Try to get a small sample of tasks to verify connection
    let tasks = db.get_inbox(Some(1))?;
    println!("✅ Database connection successful!");
    println!("   Found {} tasks in inbox", tasks.len());

    // Try to get projects
    let projects = db.get_projects(None)?;
    println!("   Found {} projects", projects.len());

    // Try to get areas
    let areas = db.get_areas()?;
    println!("   Found {} areas", areas.len());

    println!("🎉 All systems operational!");
    Ok(())
}

/// Start the MCP server
///
/// # Errors
/// Returns an error if the server fails to start
pub fn start_mcp_server(db: ThingsDatabase, config: ThingsConfig) -> Result<()> {
    println!("🚀 Starting MCP server...");

    let _server = mcp::ThingsMcpServer::new(db, config);

    // In a real implementation, this would start the MCP server
    // For now, we'll just print that it would start
    println!("✅ MCP server would start here");
    println!("   (This is a placeholder - actual MCP server implementation would go here)");

    Ok(())
}

/// Start the WebSocket server for real-time updates
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

    #[test]
    fn test_health_check() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        let _conn = create_test_database(db_path).unwrap();
        let db = ThingsDatabase::new(db_path).unwrap();
        let result = health_check(&db);
        assert!(result.is_ok());
    }

    #[test]
    fn test_start_mcp_server() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        let _conn = create_test_database(db_path).unwrap();
        let db = ThingsDatabase::new(db_path).unwrap();
        let config = ThingsConfig::default();
        let result = start_mcp_server(db, config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_start_websocket_server_function_exists() {
        // Test that the function exists and can be referenced
        // We don't actually call it as it would hang
        let _function_ref = start_websocket_server;
        // Function reference test passed if we get here
    }

    #[test]
    fn test_watch_updates() {
        let result = watch_updates("ws://127.0.0.1:8080");
        assert!(result.is_ok());
    }
}
