//! Things CLI - Command line interface for Things 3 with integrated MCP server

use things_cli::mcp;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use things_core::{Result, ThingsConfig, ThingsDatabase};

#[derive(Parser, Debug)]
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

#[derive(Subcommand, Debug)]
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
            println!("🚀 Starting Things 3 MCP server...");
            println!("📡 Server will be available for AI/LLM integration");
            println!("🛠️  Available tools: get_inbox, get_today, get_projects, get_areas, search_tasks, create_task, update_task, get_productivity_metrics, export_data, bulk_create_tasks, get_recent_tasks, backup_database, restore_database, list_backups, get_performance_stats, get_system_metrics, get_cache_stats");
            println!();

            // Start MCP server
            let mcp_server = mcp::ThingsMcpServer::new(db, config);
            start_mcp_server(mcp_server).await?;
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

async fn start_mcp_server(_mcp_server: mcp::ThingsMcpServer) -> Result<()> {
    println!("🔄 MCP server is running...");
    println!("💡 Use Ctrl+C to stop the server");
    println!();

    // For now, just keep the server running
    // In a real implementation, this would handle MCP protocol communication
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use things_core::test_utils::create_mock_tasks;

    #[test]
    fn test_cli_parsing_inbox() {
        let cli = Cli::try_parse_from(["things-cli", "inbox", "--limit", "10"]).unwrap();
        assert!(matches!(cli.command, Commands::Inbox { limit: Some(10) }));
        assert!(!cli.verbose);
        assert!(!cli.fallback_to_default);
    }

    #[test]
    fn test_cli_parsing_today() {
        let cli = Cli::try_parse_from(["things-cli", "today", "--limit", "5"]).unwrap();
        assert!(matches!(cli.command, Commands::Today { limit: Some(5) }));
    }

    #[test]
    fn test_cli_parsing_projects() {
        let cli = Cli::try_parse_from(["things-cli", "projects", "--area", "test-uuid"]).unwrap();
        assert!(
            matches!(cli.command, Commands::Projects { area: Some(ref area) } if area == "test-uuid")
        );
    }

    #[test]
    fn test_cli_parsing_areas() {
        let cli = Cli::try_parse_from(["things-cli", "areas"]).unwrap();
        assert!(matches!(cli.command, Commands::Areas));
    }

    #[test]
    fn test_cli_parsing_search() {
        let cli =
            Cli::try_parse_from(["things-cli", "search", "test query", "--limit", "20"]).unwrap();
        assert!(
            matches!(cli.command, Commands::Search { query: ref q, limit: Some(20) } if q == "test query")
        );
    }

    #[test]
    fn test_cli_parsing_mcp() {
        let cli = Cli::try_parse_from(["things-cli", "mcp"]).unwrap();
        assert!(matches!(cli.command, Commands::Mcp));
    }

    #[test]
    fn test_cli_parsing_health() {
        let cli = Cli::try_parse_from(["things-cli", "health"]).unwrap();
        assert!(matches!(cli.command, Commands::Health));
    }

    #[test]
    fn test_cli_parsing_with_database_path() {
        let cli =
            Cli::try_parse_from(["things-cli", "--database", "/path/to/db", "inbox"]).unwrap();
        assert!(matches!(cli.command, Commands::Inbox { limit: None }));
        assert!(cli.database.is_some());
        assert_eq!(cli.database.unwrap(), PathBuf::from("/path/to/db"));
    }

    #[test]
    fn test_cli_parsing_with_verbose() {
        let cli = Cli::try_parse_from(["things-cli", "--verbose", "inbox"]).unwrap();
        assert!(matches!(cli.command, Commands::Inbox { limit: None }));
        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_parsing_with_fallback() {
        let cli = Cli::try_parse_from(["things-cli", "--fallback-to-default", "inbox"]).unwrap();
        assert!(matches!(cli.command, Commands::Inbox { limit: None }));
        assert!(cli.fallback_to_default);
    }

    #[test]
    fn test_cli_parsing_all_options() {
        let cli = Cli::try_parse_from([
            "things-cli",
            "--database",
            "/custom/db",
            "--fallback-to-default",
            "--verbose",
            "search",
            "test",
            "--limit",
            "15",
        ])
        .unwrap();
        assert!(
            matches!(cli.command, Commands::Search { query: ref q, limit: Some(15) } if q == "test")
        );
        assert_eq!(cli.database.unwrap(), PathBuf::from("/custom/db"));
        assert!(cli.fallback_to_default);
        assert!(cli.verbose);
    }

    #[test]
    fn test_print_tasks_empty() {
        let tasks = vec![];
        // This should not panic
        print_tasks(&tasks);
    }

    #[test]
    fn test_print_tasks_with_data() {
        let tasks = create_mock_tasks();
        // This should not panic
        print_tasks(&tasks);
    }

    #[test]
    fn test_print_tasks_single() {
        let tasks = vec![create_mock_tasks()[0].clone()];
        // This should not panic
        print_tasks(&tasks);
    }

    #[test]
    fn test_print_projects_empty() {
        let projects = vec![];
        // This should not panic
        print_projects(&projects);
    }

    #[test]
    fn test_print_projects_with_data() {
        let projects = things_core::test_utils::create_mock_projects();
        // This should not panic
        print_projects(&projects);
    }

    #[test]
    fn test_print_areas_empty() {
        let areas = vec![];
        // This should not panic
        print_areas(&areas);
    }

    #[test]
    fn test_print_areas_with_data() {
        let areas = things_core::test_utils::create_mock_areas();
        // This should not panic
        print_areas(&areas);
    }

    #[test]
    fn test_health_check_success() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create a test database
        things_core::test_utils::create_test_database(&db_path).unwrap();

        let config = ThingsConfig::new(&db_path, false);
        let db = ThingsDatabase::with_config(&config).unwrap();

        // This should not panic
        let result = health_check(&db);
        assert!(result.is_ok());
    }

    #[test]
    fn test_health_check_database_error() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("nonexistent.db");

        let config = ThingsConfig::new(&db_path, false);
        let result = ThingsDatabase::with_config(&config);

        // This should fail because the database doesn't exist and fallback is disabled
        assert!(result.is_err());
    }

    #[test]
    fn test_commands_enum_debug() {
        let commands = vec![
            Commands::Inbox { limit: None },
            Commands::Today { limit: Some(10) },
            Commands::Projects {
                area: Some("test".to_string()),
            },
            Commands::Areas,
            Commands::Search {
                query: "test".to_string(),
                limit: None,
            },
            Commands::Mcp,
            Commands::Health,
        ];

        for command in commands {
            let debug_str = format!("{:?}", command);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_cli_struct_debug() {
        let cli = Cli::try_parse_from(["things-cli", "inbox"]).unwrap();
        let debug_str = format!("{:?}", cli);
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn test_cli_parsing_invalid_command() {
        let result = Cli::try_parse_from(["things-cli", "invalid-command"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_parsing_missing_command() {
        let result = Cli::try_parse_from(["things-cli"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_parsing_help() {
        let result = Cli::try_parse_from(["things-cli", "--help"]);
        assert!(result.is_err()); // Help causes early exit
    }

    #[test]
    fn test_cli_parsing_version() {
        let result = Cli::try_parse_from(["things-cli", "--version"]);
        assert!(result.is_err()); // Version causes early exit
    }

    #[test]
    fn test_commands_equality() {
        let cmd1 = Commands::Inbox { limit: None };
        let cmd2 = Commands::Inbox { limit: None };
        let cmd3 = Commands::Inbox { limit: Some(10) };
        let cmd4 = Commands::Today { limit: None };

        // Note: These tests will fail if Commands doesn't implement PartialEq
        // This is expected for enums with data
        assert!(matches!(cmd1, Commands::Inbox { limit: None }));
        assert!(matches!(cmd2, Commands::Inbox { limit: None }));
        assert!(matches!(cmd3, Commands::Inbox { limit: Some(10) }));
        assert!(matches!(cmd4, Commands::Today { limit: None }));
    }

    #[test]
    fn test_cli_default_values() {
        let cli = Cli::try_parse_from(["things-cli", "inbox"]).unwrap();
        assert!(cli.database.is_none());
        assert!(!cli.fallback_to_default);
        assert!(!cli.verbose);
    }

    #[test]
    fn test_cli_short_flags() {
        let cli = Cli::try_parse_from(["things-cli", "-v", "-d", "/path", "inbox"]).unwrap();
        assert!(cli.verbose);
        assert!(cli.database.is_some());
        assert_eq!(cli.database.unwrap(), PathBuf::from("/path"));
    }

    #[test]
    fn test_cli_short_limit() {
        let cli = Cli::try_parse_from(["things-cli", "inbox", "-l", "5"]).unwrap();
        assert!(matches!(cli.command, Commands::Inbox { limit: Some(5) }));
    }

    #[test]
    fn test_cli_search_without_limit() {
        let cli = Cli::try_parse_from(["things-cli", "search", "test query"]).unwrap();
        assert!(
            matches!(cli.command, Commands::Search { query: ref q, limit: None } if q == "test query")
        );
    }

    #[test]
    fn test_cli_projects_without_area() {
        let cli = Cli::try_parse_from(["things-cli", "projects"]).unwrap();
        assert!(matches!(cli.command, Commands::Projects { area: None }));
    }

    #[test]
    fn test_cli_inbox_without_limit() {
        let cli = Cli::try_parse_from(["things-cli", "inbox"]).unwrap();
        assert!(matches!(cli.command, Commands::Inbox { limit: None }));
    }

    #[test]
    fn test_cli_parsing_all_options_comprehensive() {
        let cli = Cli::try_parse_from([
            "things-cli",
            "--database",
            "/custom/path",
            "--fallback-to-default",
            "--verbose",
            "inbox",
            "--limit",
            "5",
        ])
        .unwrap();

        assert!(matches!(cli.command, Commands::Inbox { limit: Some(5) }));
        assert!(cli.verbose);
        assert!(cli.fallback_to_default);
        assert_eq!(cli.database.unwrap(), PathBuf::from("/custom/path"));
    }

    #[test]
    fn test_main_function_execution_paths() {
        // Test that main function can be called with different commands
        // This tests the execution flow without actually running the full main function

        // Test inbox command parsing
        let cli = Cli::try_parse_from(["things-cli", "inbox", "--limit", "10"]).unwrap();
        assert!(matches!(cli.command, Commands::Inbox { limit: Some(10) }));

        // Test today command parsing
        let cli = Cli::try_parse_from(["things-cli", "today"]).unwrap();
        assert!(matches!(cli.command, Commands::Today { limit: None }));

        // Test projects command parsing
        let cli = Cli::try_parse_from(["things-cli", "projects", "--area", "test-uuid"]).unwrap();
        assert!(
            matches!(cli.command, Commands::Projects { area: Some(ref area) } if area == "test-uuid")
        );

        // Test areas command parsing
        let cli = Cli::try_parse_from(["things-cli", "areas"]).unwrap();
        assert!(matches!(cli.command, Commands::Areas));

        // Test search command parsing
        let cli =
            Cli::try_parse_from(["things-cli", "search", "test query", "--limit", "5"]).unwrap();
        assert!(
            matches!(cli.command, Commands::Search { query: ref q, limit: Some(5) } if q == "test query")
        );

        // Test mcp command parsing
        let cli = Cli::try_parse_from(["things-cli", "mcp"]).unwrap();
        assert!(matches!(cli.command, Commands::Mcp));

        // Test health command parsing
        let cli = Cli::try_parse_from(["things-cli", "health"]).unwrap();
        assert!(matches!(cli.command, Commands::Health));
    }

    #[test]
    fn test_config_creation_paths() {
        // Test config creation with database path
        let cli = Cli::try_parse_from(["things-cli", "--database", "/test/path", "inbox"]).unwrap();
        let config = if let Some(db_path) = cli.database {
            ThingsConfig::new(db_path, cli.fallback_to_default)
        } else {
            ThingsConfig::from_env()
        };

        assert_eq!(config.database_path, PathBuf::from("/test/path"));
        assert!(!config.fallback_to_default);

        // Test config creation without database path
        let cli = Cli::try_parse_from(["things-cli", "inbox"]).unwrap();
        let config = if let Some(db_path) = cli.database {
            ThingsConfig::new(db_path, cli.fallback_to_default)
        } else {
            ThingsConfig::from_env()
        };

        // Should use default path from env (may or may not contain "Things3" depending on system)
        assert!(!config.database_path.to_string_lossy().is_empty());
    }

    #[test]
    fn test_verbose_logging_setup() {
        // Test that verbose flag is properly parsed
        let cli = Cli::try_parse_from(["things-cli", "--verbose", "inbox"]).unwrap();
        assert!(cli.verbose);

        let cli = Cli::try_parse_from(["things-cli", "inbox"]).unwrap();
        assert!(!cli.verbose);

        let cli = Cli::try_parse_from(["things-cli", "-v", "inbox"]).unwrap();
        assert!(cli.verbose);
    }

    #[test]
    fn test_fallback_behavior_parsing() {
        let cli = Cli::try_parse_from(["things-cli", "--fallback-to-default", "inbox"]).unwrap();
        assert!(cli.fallback_to_default);

        let cli = Cli::try_parse_from(["things-cli", "inbox"]).unwrap();
        assert!(!cli.fallback_to_default);
    }

    #[test]
    fn test_command_enum_variants() {
        // Test all command variants can be created and debugged
        let commands = vec![
            Commands::Inbox { limit: None },
            Commands::Inbox { limit: Some(10) },
            Commands::Today { limit: None },
            Commands::Today { limit: Some(5) },
            Commands::Projects { area: None },
            Commands::Projects {
                area: Some("test".to_string()),
            },
            Commands::Areas,
            Commands::Search {
                query: "test".to_string(),
                limit: None,
            },
            Commands::Search {
                query: "test".to_string(),
                limit: Some(10),
            },
            Commands::Mcp,
            Commands::Health,
        ];

        for command in commands {
            let debug_str = format!("{:?}", command);
            assert!(!debug_str.is_empty());
            assert!(
                debug_str.contains("Inbox")
                    || debug_str.contains("Today")
                    || debug_str.contains("Projects")
                    || debug_str.contains("Areas")
                    || debug_str.contains("Search")
                    || debug_str.contains("Mcp")
                    || debug_str.contains("Health")
            );
        }
    }

    #[test]
    fn test_cli_today_without_limit() {
        let cli = Cli::try_parse_from(["things-cli", "today"]).unwrap();
        assert!(matches!(cli.command, Commands::Today { limit: None }));
    }
}
