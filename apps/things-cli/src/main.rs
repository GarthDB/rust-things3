//! Things CLI - Command line interface for Things 3 with integrated MCP server

use clap::Parser;
use things_cli::{
    health_check, print_areas, print_projects, print_tasks, start_mcp_server, Cli, Commands,
};
use things_core::{Result, ThingsConfig, ThingsDatabase};

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
            print_tasks(&db, &tasks, &mut std::io::stdout())?;
        }
        Commands::Today { limit } => {
            let tasks = db.get_today(limit)?;
            print_tasks(&db, &tasks, &mut std::io::stdout())?;
        }
        Commands::Projects { area, limit } => {
            let area_uuid = area.and_then(|a| uuid::Uuid::parse_str(&a).ok());
            let projects = db.get_projects(area_uuid)?;
            let projects = if let Some(limit) = limit {
                projects.into_iter().take(limit).collect()
            } else {
                projects
            };
            print_projects(&db, &projects, &mut std::io::stdout())?;
        }
        Commands::Areas { limit } => {
            let areas = db.get_areas()?;
            let areas = if let Some(limit) = limit {
                areas.into_iter().take(limit).collect()
            } else {
                areas
            };
            print_areas(&db, &areas, &mut std::io::stdout())?;
        }
        Commands::Search { query, limit } => {
            let tasks = db.search_tasks(&query, limit)?;
            print_tasks(&db, &tasks, &mut std::io::stdout())?;
        }
        Commands::Mcp => {
            start_mcp_server(db, config)?;
        }
        Commands::Health => {
            health_check(&db)?;
        }
    }

    Ok(())
}
