//! Integration tests for CLI functionality

use std::io::Cursor;
use tempfile::NamedTempFile;
use things3_core::{
    config::ThingsConfig, database::ThingsDatabase, test_utils::create_test_database,
};

/// Test the `print_tasks` function with various inputs
#[tokio::test]
async fn test_print_tasks_integration() {
    // Create a test database
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();

    let config = ThingsConfig::new(db_path, false);
    let db = ThingsDatabase::new(&config.database_path).await.unwrap();

    // Test with empty tasks
    let mut output = Cursor::new(Vec::new());
    things3_cli::print_tasks(&db, &[], &mut output).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    assert!(result.contains("No tasks found"));

    // Test with some tasks
    let tasks = db.get_inbox(None).await.unwrap();
    let mut output = Cursor::new(Vec::new());
    things3_cli::print_tasks(&db, &tasks, &mut output).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    assert!(!result.is_empty());
}

/// Test the `print_projects` function with various inputs
#[tokio::test]
async fn test_print_projects_integration() {
    // Create a test database
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();

    let config = ThingsConfig::new(db_path, false);
    let db = ThingsDatabase::new(&config.database_path).await.unwrap();

    // Test with empty projects
    let mut output = Cursor::new(Vec::new());
    things3_cli::print_projects(&db, &[], &mut output).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    assert!(result.contains("No projects found"));

    // Test with some projects
    let projects = db.get_projects(None).await.unwrap();
    let mut output = Cursor::new(Vec::new());
    things3_cli::print_projects(&db, &projects, &mut output).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    assert!(!result.is_empty());
}

/// Test the `print_areas` function with various inputs
#[tokio::test]
async fn test_print_areas_integration() {
    // Create a test database
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();

    let config = ThingsConfig::new(db_path, false);
    let db = ThingsDatabase::new(&config.database_path).await.unwrap();

    // Test with empty areas
    let mut output = Cursor::new(Vec::new());
    things3_cli::print_areas(&db, &[], &mut output).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    assert!(result.contains("No areas found"));

    // Test with some areas
    let areas = db.get_areas().await.unwrap();
    let mut output = Cursor::new(Vec::new());
    things3_cli::print_areas(&db, &areas, &mut output).unwrap();
    let result = String::from_utf8(output.into_inner()).unwrap();
    assert!(!result.is_empty());
}

/// Test the `health_check` function
#[tokio::test]
async fn test_health_check_integration() {
    // Create a test database
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();

    let config = ThingsConfig::new(db_path, false);
    let db = ThingsDatabase::new(&config.database_path).await.unwrap();

    // Test successful health check
    let result = things3_cli::health_check(&db).await;
    assert!(result.is_ok());

    // Test health check with invalid database
    let invalid_config = ThingsConfig::new("/nonexistent/path", false);
    let invalid_db = ThingsDatabase::new(&invalid_config.database_path).await;
    if let Ok(db) = invalid_db {
        let result = things3_cli::health_check(&db).await;
        // This might succeed or fail depending on the database state
        let _ = result; // Just ensure it doesn't panic
    }
}

/// Test MCP server creation and basic functionality
#[tokio::test]
async fn test_mcp_server_integration() {
    // Create a test database
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();
    create_test_database(db_path).await.unwrap();

    let config = ThingsConfig::new(db_path, false);
    let db = ThingsDatabase::new(&config.database_path).await.unwrap();

    // Test MCP server creation
    let server = things3_cli::mcp::ThingsMcpServer::new(db.into(), config);

    // Test tool listing
    let tools = server.list_tools().unwrap();
    assert!(!tools.tools.is_empty());
    assert!(tools.tools.iter().any(|tool| tool.name == "get_inbox"));
    assert!(tools.tools.iter().any(|tool| tool.name == "get_today"));
    assert!(tools.tools.iter().any(|tool| tool.name == "get_projects"));
}

/// Test CLI argument parsing with various combinations
#[test]
fn test_cli_parsing_integration() {
    use clap::Parser;
    use things3_cli::Cli;

    // Test basic parsing
    let cli = Cli::try_parse_from(["things-cli", "inbox"]).unwrap();
    assert_eq!(cli.command, things3_cli::Commands::Inbox { limit: None });

    // Test with limit
    let cli = Cli::try_parse_from(["things-cli", "inbox", "--limit", "10"]).unwrap();
    assert_eq!(
        cli.command,
        things3_cli::Commands::Inbox { limit: Some(10) }
    );

    // Test with database path
    let cli = Cli::try_parse_from(["things-cli", "--database", "/tmp/test.db", "inbox"]).unwrap();
    assert_eq!(cli.database, Some(std::path::PathBuf::from("/tmp/test.db")));

    // Test with verbose flag
    let cli = Cli::try_parse_from(["things-cli", "--verbose", "inbox"]).unwrap();
    assert!(cli.verbose);

    // Test with fallback flag
    let cli = Cli::try_parse_from(["things-cli", "--fallback-to-default", "inbox"]).unwrap();
    assert!(cli.fallback_to_default);
}

/// Test error handling in CLI functions
#[tokio::test]
async fn test_cli_error_handling_integration() {
    // Test with nonexistent database
    let config = ThingsConfig::new("/nonexistent/path", false);
    let db_result = ThingsDatabase::new(&config.database_path).await;

    // This should fail
    assert!(db_result.is_err());

    // Test with malformed database path (invalid characters)
    let config = ThingsConfig::new("/invalid/path/with/invalid/chars/\0", false);
    let db_result = ThingsDatabase::new(&config.database_path).await;

    // This should also fail
    assert!(db_result.is_err());
}

/// Test CLI with different command types
#[test]
fn test_cli_commands_integration() {
    use clap::Parser;
    use things3_cli::Cli;

    // Test all command types
    let cli = Cli::try_parse_from(["things-cli", "inbox"]).unwrap();
    assert_eq!(cli.command, things3_cli::Commands::Inbox { limit: None });

    let cli = Cli::try_parse_from(["things-cli", "today"]).unwrap();
    assert_eq!(cli.command, things3_cli::Commands::Today { limit: None });

    let cli = Cli::try_parse_from(["things-cli", "projects"]).unwrap();
    assert_eq!(
        cli.command,
        things3_cli::Commands::Projects {
            area: None,
            limit: None
        }
    );

    let cli = Cli::try_parse_from(["things-cli", "areas"]).unwrap();
    assert_eq!(cli.command, things3_cli::Commands::Areas { limit: None });

    let cli = Cli::try_parse_from(["things-cli", "search", "test"]).unwrap();
    assert_eq!(
        cli.command,
        things3_cli::Commands::Search {
            query: "test".to_string(),
            limit: None
        }
    );

    let cli = Cli::try_parse_from(["things-cli", "health"]).unwrap();
    assert_eq!(cli.command, things3_cli::Commands::Health);

    let cli = Cli::try_parse_from(["things-cli", "mcp"]).unwrap();
    assert_eq!(cli.command, things3_cli::Commands::Mcp);
}

/// Test CLI with various flag combinations
#[test]
fn test_cli_flag_combinations_integration() {
    use clap::Parser;
    use things3_cli::Cli;

    // Test multiple flags together
    let cli = Cli::try_parse_from([
        "things-cli",
        "--verbose",
        "--fallback-to-default",
        "--database",
        "/tmp/test.db",
        "inbox",
        "--limit",
        "5",
    ])
    .unwrap();

    assert_eq!(cli.command, things3_cli::Commands::Inbox { limit: Some(5) });
    assert!(cli.verbose);
    assert!(cli.fallback_to_default);
    assert_eq!(cli.database, Some(std::path::PathBuf::from("/tmp/test.db")));
}

/// Test CLI help and version
#[test]
fn test_cli_help_version_integration() {
    use clap::Parser;
    use things3_cli::Cli;

    // Test help parsing (should not panic)
    let _ = Cli::try_parse_from(["things-cli", "--help"]);

    // Test version parsing (should not panic)
    let _ = Cli::try_parse_from(["things-cli", "--version"]);
}
