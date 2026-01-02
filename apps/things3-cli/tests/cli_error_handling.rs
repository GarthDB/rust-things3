//! CLI error handling tests
//!
//! Tests various error conditions in CLI command processing to ensure
//! robust error handling and helpful error messages.

use clap::Parser;
use things3_cli::{Cli, Commands};

/// Test parsing invalid command
#[test]
fn test_invalid_command() {
    let result = Cli::try_parse_from(["things3", "invalid_command"]);
    assert!(result.is_err(), "Should fail on invalid command");
}

/// Test parsing command with invalid arguments
#[test]
fn test_invalid_arguments() {
    // Invalid limit type
    let result = Cli::try_parse_from(["things3", "inbox", "--limit", "not_a_number"]);
    assert!(result.is_err(), "Should fail on invalid limit type");
}

/// Test parsing command with missing required arguments
#[test]
fn test_missing_required_arguments() {
    // Search requires a query argument
    let result = Cli::try_parse_from(["things3", "search"]);
    assert!(
        result.is_err(),
        "Should fail when required argument is missing"
    );
}

/// Test parsing with conflicting flags
#[test]
fn test_help_flag() {
    let result = Cli::try_parse_from(["things3", "--help"]);
    assert!(result.is_err(), "Help flag should cause early exit");
}

/// Test parsing valid commands
#[test]
fn test_valid_inbox_command() {
    let cli = Cli::try_parse_from(["things3", "inbox"]).unwrap();
    assert!(matches!(cli.command, Commands::Inbox { .. }));
}

/// Test parsing inbox with limit
#[test]
fn test_inbox_with_limit() {
    let cli = Cli::try_parse_from(["things3", "inbox", "--limit", "10"]).unwrap();
    match cli.command {
        Commands::Inbox { limit } => {
            assert_eq!(limit, Some(10));
        }
        _ => panic!("Expected inbox command"),
    }
}

/// Test parsing today command
#[test]
fn test_valid_today_command() {
    let cli = Cli::try_parse_from(["things3", "today"]).unwrap();
    assert!(matches!(cli.command, Commands::Today { .. }));
}

/// Test parsing projects command
#[test]
fn test_valid_projects_command() {
    let cli = Cli::try_parse_from(["things3", "projects"]).unwrap();
    assert!(matches!(cli.command, Commands::Projects { .. }));
}

/// Test parsing areas command
#[test]
fn test_valid_areas_command() {
    let cli = Cli::try_parse_from(["things3", "areas"]).unwrap();
    assert!(matches!(cli.command, Commands::Areas { .. }));
}

/// Test parsing search command
#[test]
fn test_valid_search_command() {
    let cli = Cli::try_parse_from(["things3", "search", "test query"]).unwrap();
    match cli.command {
        Commands::Search { query, .. } => {
            assert_eq!(query, "test query");
        }
        _ => panic!("Expected search command"),
    }
}

/// Test parsing MCP command
#[test]
fn test_valid_mcp_command() {
    let cli = Cli::try_parse_from(["things3", "mcp"]).unwrap();
    assert!(matches!(cli.command, Commands::Mcp));
}

/// Test parsing health command
#[test]
fn test_valid_health_command() {
    let cli = Cli::try_parse_from(["things3", "health"]).unwrap();
    assert!(matches!(cli.command, Commands::Health));
}

/// Test parsing with database path option
#[test]
fn test_database_path_option() {
    let cli =
        Cli::try_parse_from(["things3", "--database", "/path/to/db.sqlite", "inbox"]).unwrap();
    assert!(cli.database.is_some());
    assert_eq!(
        cli.database.unwrap().to_str().unwrap(),
        "/path/to/db.sqlite"
    );
}

/// Test parsing with verbose flag
#[test]
fn test_verbose_flag() {
    let cli = Cli::try_parse_from(["things3", "--verbose", "inbox"]).unwrap();
    assert!(cli.verbose);
}

/// Test parsing with fallback flag
#[test]
fn test_fallback_flag() {
    let cli = Cli::try_parse_from(["things3", "--fallback-to-default", "inbox"]).unwrap();
    assert!(cli.fallback_to_default);
}

/// Test parsing search with empty query
#[test]
fn test_search_with_empty_query() {
    let cli = Cli::try_parse_from(["things3", "search", ""]).unwrap();
    match cli.command {
        Commands::Search { query, .. } => {
            assert_eq!(query, "");
        }
        _ => panic!("Expected search command"),
    }
}

/// Test parsing with negative limit (should fail)
#[test]
fn test_negative_limit() {
    let result = Cli::try_parse_from(["things3", "inbox", "--limit", "-10"]);
    assert!(result.is_err(), "Should fail on negative limit");
}

/// Test parsing with zero limit
#[test]
fn test_zero_limit() {
    let cli = Cli::try_parse_from(["things3", "inbox", "--limit", "0"]).unwrap();
    match cli.command {
        Commands::Inbox { limit } => {
            assert_eq!(limit, Some(0));
        }
        _ => panic!("Expected inbox command"),
    }
}
