//! Tests for CLI feature functionality
//!
//! These tests actually execute CLI code paths to increase coverage

use clap::Parser;
use things3_cli::{Cli, Commands};

#[test]
fn test_core_command_parsing() {
    let test_cases = vec![
        vec!["things3", "inbox"],
        vec!["things3", "today"],
        vec!["things3", "projects"],
        vec!["things3", "areas"],
        vec!["things3", "search", "test"],
        vec!["things3", "health"],
    ];

    for args in test_cases {
        let cli = Cli::try_parse_from(args.clone());
        assert!(cli.is_ok(), "Failed to parse: {:?}", args);
    }
}

#[cfg(feature = "mcp-server")]
#[test]
fn test_mcp_command_structure() {
    let args = vec!["things3", "mcp"];
    let cli = Cli::try_parse_from(args).unwrap();

    assert!(matches!(cli.command, Commands::Mcp));
}

#[cfg(feature = "observability")]
#[test]
fn test_observability_commands_structure() {
    // Test health-server command
    let args = vec!["things3", "health-server", "--port", "9999"];
    let cli = Cli::try_parse_from(args).unwrap();

    if let Commands::HealthServer { port } = cli.command {
        assert_eq!(port, 9999);
    } else {
        panic!("Expected HealthServer command");
    }

    // Test dashboard command
    let args = vec!["things3", "dashboard", "--port", "3030"];
    let cli = Cli::try_parse_from(args).unwrap();

    if let Commands::Dashboard { port } = cli.command {
        assert_eq!(port, 3030);
    } else {
        panic!("Expected Dashboard command");
    }
}

#[test]
fn test_bulk_export_command() {
    let args = vec!["things3", "bulk", "export", "--format", "json"];
    let cli = Cli::try_parse_from(args);

    assert!(cli.is_ok(), "Bulk export command should parse");
}

#[cfg(feature = "export-csv")]
#[test]
fn test_csv_export_via_bulk() {
    let args = vec!["things3", "bulk", "export", "--format", "csv"];
    let cli = Cli::try_parse_from(args);

    assert!(cli.is_ok(), "CSV export should be available");
}

#[test]
fn test_cli_verbose_flag() {
    let args = vec!["things3", "--verbose", "inbox"];
    let cli = Cli::try_parse_from(args).unwrap();

    assert!(cli.verbose, "Verbose flag should be set");
}

#[test]
fn test_cli_database_path() {
    let args = vec!["things3", "--database", "/custom/path.db", "inbox"];
    let cli = Cli::try_parse_from(args).unwrap();

    assert!(cli.database.is_some());
    assert_eq!(cli.database.unwrap().to_str().unwrap(), "/custom/path.db");
}

#[test]
fn test_cli_fallback_flag() {
    let args = vec!["things3", "--fallback-to-default", "inbox"];
    let cli = Cli::try_parse_from(args).unwrap();

    assert!(cli.fallback_to_default);
}

#[test]
fn test_search_command_with_query() {
    let args = vec!["things3", "search", "meeting"];
    let cli = Cli::try_parse_from(args).unwrap();

    if let Commands::Search { query, .. } = cli.command {
        assert_eq!(query, "meeting");
    } else {
        panic!("Expected Search command");
    }
}

#[test]
fn test_projects_with_limit() {
    let args = vec!["things3", "projects", "--limit", "10"];
    let cli = Cli::try_parse_from(args).unwrap();

    if let Commands::Projects { limit, .. } = cli.command {
        assert_eq!(limit, Some(10));
    } else {
        panic!("Expected Projects command");
    }
}

#[test]
fn test_today_with_limit() {
    let args = vec!["things3", "today", "--limit", "5"];
    let cli = Cli::try_parse_from(args).unwrap();

    if let Commands::Today { limit } = cli.command {
        assert_eq!(limit, Some(5));
    } else {
        panic!("Expected Today command");
    }
}

#[test]
fn test_server_command_with_port() {
    let args = vec!["things3", "server", "--port", "8888"];
    let cli = Cli::try_parse_from(args).unwrap();

    if let Commands::Server { port } = cli.command {
        assert_eq!(port, 8888);
    } else {
        panic!("Expected Server command");
    }
}

#[test]
fn test_watch_command_with_url() {
    let args = vec!["things3", "watch", "--url", "ws://localhost:9000"];
    let cli = Cli::try_parse_from(args).unwrap();

    if let Commands::Watch { url } = cli.command {
        assert_eq!(url, "ws://localhost:9000");
    } else {
        panic!("Expected Watch command");
    }
}
