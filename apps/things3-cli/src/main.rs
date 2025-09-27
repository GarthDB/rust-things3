//! Things CLI - Command line interface for Things 3 with integrated MCP server

use clap::Parser;
use things3_cli::{
    health_check, print_areas, print_projects, print_tasks, start_mcp_server, Cli, Commands,
};
use things3_core::{Result, ThingsConfig, ThingsDatabase};

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
                projects.into_iter().take(limit).collect::<Vec<_>>()
            } else {
                projects
            };
            print_projects(&db, &projects, &mut std::io::stdout())?;
        }
        Commands::Areas { limit } => {
            let areas = db.get_areas()?;
            let areas = if let Some(limit) = limit {
                areas.into_iter().take(limit).collect::<Vec<_>>()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::NamedTempFile;
    use things3_core::test_utils::create_test_database;

    /// Test the main function with various command combinations
    #[tokio::test]
    async fn test_main_inbox_command() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::with_config(&config).unwrap();

        // Test inbox command
        let cli = Cli::try_parse_from(["things-cli", "inbox"]).unwrap();
        let result = match cli.command {
            Commands::Inbox { limit } => {
                let tasks = db.get_inbox(limit).unwrap();
                let mut output = Cursor::new(Vec::new());
                print_tasks(&db, &tasks, &mut output).unwrap();
                String::from_utf8(output.into_inner()).unwrap()
            }
            _ => panic!("Expected inbox command"),
        };
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_main_today_command() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::with_config(&config).unwrap();

        // Test today command
        let cli = Cli::try_parse_from(["things-cli", "today"]).unwrap();
        let result = match cli.command {
            Commands::Today { limit } => {
                let tasks = db.get_today(limit).unwrap();
                let mut output = Cursor::new(Vec::new());
                print_tasks(&db, &tasks, &mut output).unwrap();
                String::from_utf8(output.into_inner()).unwrap()
            }
            _ => panic!("Expected today command"),
        };
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_main_projects_command() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::with_config(&config).unwrap();

        // Test projects command
        let cli = Cli::try_parse_from(["things-cli", "projects"]).unwrap();
        let result = match cli.command {
            Commands::Projects { area, limit } => {
                let area_uuid = area.and_then(|a| uuid::Uuid::parse_str(&a).ok());
                let projects = db.get_projects(area_uuid).unwrap();
                let projects = if let Some(limit) = limit {
                    projects.into_iter().take(limit).collect::<Vec<_>>()
                } else {
                    projects
                };
                let mut output = Cursor::new(Vec::new());
                print_projects(&db, &projects, &mut output).unwrap();
                String::from_utf8(output.into_inner()).unwrap()
            }
            _ => panic!("Expected projects command"),
        };
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_main_areas_command() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::with_config(&config).unwrap();

        // Test areas command
        let cli = Cli::try_parse_from(["things-cli", "areas"]).unwrap();
        let result = match cli.command {
            Commands::Areas { limit } => {
                let areas = db.get_areas().unwrap();
                let areas = if let Some(limit) = limit {
                    areas.into_iter().take(limit).collect::<Vec<_>>()
                } else {
                    areas
                };
                let mut output = Cursor::new(Vec::new());
                print_areas(&db, &areas, &mut output).unwrap();
                String::from_utf8(output.into_inner()).unwrap()
            }
            _ => panic!("Expected areas command"),
        };
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_main_search_command() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::with_config(&config).unwrap();

        // Test search command
        let cli = Cli::try_parse_from(["things-cli", "search", "test"]).unwrap();
        let result = match cli.command {
            Commands::Search { query, limit } => {
                let tasks = db.search_tasks(&query, limit).unwrap();
                let mut output = Cursor::new(Vec::new());
                print_tasks(&db, &tasks, &mut output).unwrap();
                String::from_utf8(output.into_inner()).unwrap()
            }
            _ => panic!("Expected search command"),
        };
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_main_health_command() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::with_config(&config).unwrap();

        // Test health command
        let cli = Cli::try_parse_from(["things-cli", "health"]).unwrap();
        match cli.command {
            Commands::Health => {
                health_check(&db).unwrap();
            }
            _ => panic!("Expected health command"),
        }
    }

    #[tokio::test]
    async fn test_main_mcp_command() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::with_config(&config).unwrap();

        // Test MCP command
        let cli = Cli::try_parse_from(["things-cli", "mcp"]).unwrap();
        match cli.command {
            Commands::Mcp => {
                start_mcp_server(db, config).unwrap();
            }
            _ => panic!("Expected MCP command"),
        }
    }

    #[tokio::test]
    async fn test_main_with_verbose_flag() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::with_config(&config).unwrap();

        // Test with verbose flag
        let cli = Cli::try_parse_from(["things-cli", "--verbose", "inbox"]).unwrap();
        assert!(cli.verbose);

        match cli.command {
            Commands::Inbox { limit } => {
                let tasks = db.get_inbox(limit).unwrap();
                let mut output = Cursor::new(Vec::new());
                print_tasks(&db, &tasks, &mut output).unwrap();
                let result = String::from_utf8(output.into_inner()).unwrap();
                assert!(!result.is_empty());
            }
            _ => panic!("Expected inbox command"),
        }
    }

    #[tokio::test]
    async fn test_main_with_database_path() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        // Test with database path
        let cli = Cli::try_parse_from([
            "things-cli",
            "--database",
            db_path.to_str().unwrap(),
            "inbox",
        ])
        .unwrap();
        assert_eq!(cli.database, Some(db_path.to_path_buf()));

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::with_config(&config).unwrap();

        match cli.command {
            Commands::Inbox { limit } => {
                let tasks = db.get_inbox(limit).unwrap();
                let mut output = Cursor::new(Vec::new());
                print_tasks(&db, &tasks, &mut output).unwrap();
                let result = String::from_utf8(output.into_inner()).unwrap();
                assert!(!result.is_empty());
            }
            _ => panic!("Expected inbox command"),
        }
    }

    #[tokio::test]
    async fn test_main_with_fallback_flag() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        // Test with fallback flag
        let cli = Cli::try_parse_from(["things-cli", "--fallback-to-default", "inbox"]).unwrap();
        assert!(cli.fallback_to_default);

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::with_config(&config).unwrap();

        match cli.command {
            Commands::Inbox { limit } => {
                let tasks = db.get_inbox(limit).unwrap();
                let mut output = Cursor::new(Vec::new());
                print_tasks(&db, &tasks, &mut output).unwrap();
                let result = String::from_utf8(output.into_inner()).unwrap();
                assert!(!result.is_empty());
            }
            _ => panic!("Expected inbox command"),
        }
    }

    #[tokio::test]
    async fn test_main_with_limit() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::with_config(&config).unwrap();

        // Test with limit
        let cli = Cli::try_parse_from(["things-cli", "inbox", "--limit", "5"]).unwrap();
        match cli.command {
            Commands::Inbox { limit } => {
                assert_eq!(limit, Some(5));
                let tasks = db.get_inbox(limit).unwrap();
                let mut output = Cursor::new(Vec::new());
                print_tasks(&db, &tasks, &mut output).unwrap();
                let result = String::from_utf8(output.into_inner()).unwrap();
                assert!(!result.is_empty());
            }
            _ => panic!("Expected inbox command"),
        }
    }

    #[tokio::test]
    async fn test_main_config_creation_from_env() {
        // Test configuration creation from environment
        let cli = Cli::try_parse_from(["things-cli", "inbox"]).unwrap();

        // Test that config creation doesn't panic
        let config = if let Some(db_path) = cli.database {
            ThingsConfig::new(db_path, cli.fallback_to_default)
        } else {
            ThingsConfig::from_env()
        };

        // Just verify it creates a config (it might fail due to missing database, but that's ok)
        let _ = config;
    }

    #[tokio::test]
    async fn test_main_config_creation_with_database_path() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        // Test configuration creation with database path
        let cli = Cli::try_parse_from([
            "things-cli",
            "--database",
            db_path.to_str().unwrap(),
            "inbox",
        ])
        .unwrap();

        let config = if let Some(db_path) = cli.database {
            ThingsConfig::new(db_path, cli.fallback_to_default)
        } else {
            ThingsConfig::from_env()
        };

        // This should work since we're providing a valid path
        // Just verify it creates a config (ThingsConfig::new doesn't return a Result)
        let _ = config;
    }
}
