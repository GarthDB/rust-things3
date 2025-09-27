//! Things CLI library

pub mod mcp;

use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;
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
