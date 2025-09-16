//! Things CLI - Command line interface for Things 3 with integrated MCP server

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use things_core::{Result, ThingsConfig, ThingsDatabase};

#[derive(Parser)]
#[command(name = "things-cli")]
#[command(about = "Things 3 CLI with integrated MCP server")]
#[command(version)]
struct Cli {
    /// Database path (defaults to Things 3 default location)
    #[arg(long, short)]
    database: Option<PathBuf>,

    /// Fall back to default database path if specified path doesn't exist
    #[arg(long)]
    fallback_to_default: bool,

    /// Enable verbose output
    #[arg(long, short)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show inbox tasks
    Inbox {
        /// Limit number of results
        #[arg(long, short)]
        limit: Option<usize>,
    },
    /// Show today's tasks
    Today {
        /// Limit number of results
        #[arg(long, short)]
        limit: Option<usize>,
    },
    /// Show all projects
    Projects {
        /// Filter by area UUID
        #[arg(long)]
        area: Option<String>,
    },
    /// Show all areas
    Areas,
    /// Search for tasks
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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging if verbose
    if cli.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    }

    // Create configuration
    let config = if let Some(db_path) = cli.database {
        ThingsConfig::new(db_path, cli.fallback_to_default)
    } else {
        ThingsConfig::from_env()
    };

    // Create database connection
    let db = ThingsDatabase::with_config(&config)?;

    match cli.command {
        Commands::Inbox { limit } => {
            let tasks = db.get_inbox(limit)?;
            print_tasks(&tasks);
        }
        Commands::Today { limit } => {
            let tasks = db.get_today(limit)?;
            print_tasks(&tasks);
        }
        Commands::Projects { area } => {
            let area_uuid = area.and_then(|a| uuid::Uuid::parse_str(&a).ok());
            let projects = db.get_projects(area_uuid)?;
            print_projects(&projects);
        }
        Commands::Areas => {
            let areas = db.get_areas()?;
            print_areas(&areas);
        }
        Commands::Search { query, limit } => {
            let tasks = db.search_tasks(&query, limit)?;
            print_tasks(&tasks);
        }
        Commands::Mcp => {
            println!("MCP server mode not yet implemented");
            println!("This will start the MCP server for AI/LLM integration");
        }
        Commands::Health => {
            health_check(&db)?;
        }
    }

    Ok(())
}

fn print_tasks(tasks: &[things_core::Task]) {
    if tasks.is_empty() {
        println!("No tasks found");
        return;
    }

    println!("Found {} tasks:", tasks.len());
    for task in tasks {
        println!("  • {} ({:?})", task.title, task.task_type);
        if let Some(notes) = &task.notes {
            println!("    Notes: {notes}");
        }
        if let Some(deadline) = &task.deadline {
            println!("    Deadline: {deadline}");
        }
        if !task.tags.is_empty() {
            println!("    Tags: {}", task.tags.join(", "));
        }
        println!();
    }
}

fn print_projects(projects: &[things_core::Project]) {
    if projects.is_empty() {
        println!("No projects found");
        return;
    }

    println!("Found {} projects:", projects.len());
    for project in projects {
        println!("  • {} ({:?})", project.title, project.status);
        if let Some(notes) = &project.notes {
            println!("    Notes: {notes}");
        }
        if let Some(deadline) = &project.deadline {
            println!("    Deadline: {deadline}");
        }
        if !project.tags.is_empty() {
            println!("    Tags: {}", project.tags.join(", "));
        }
        println!();
    }
}

fn print_areas(areas: &[things_core::Area]) {
    if areas.is_empty() {
        println!("No areas found");
        return;
    }

    println!("Found {} areas:", areas.len());
    for area in areas {
        println!("  • {}", area.title);
        if let Some(notes) = &area.notes {
            println!("    Notes: {notes}");
        }
        if !area.tags.is_empty() {
            println!("    Tags: {}", area.tags.join(", "));
        }
        println!();
    }
}

fn health_check(db: &ThingsDatabase) -> Result<()> {
    println!("🔍 Checking Things 3 database connection...");

    // Try to get a small sample of tasks to verify connection
    let tasks = db.get_inbox(Some(1))?;

    println!("✅ Database connection successful");
    println!("📊 Found {} tasks in inbox", tasks.len());
    println!("🎯 Things CLI is ready to use!");

    Ok(())
}
