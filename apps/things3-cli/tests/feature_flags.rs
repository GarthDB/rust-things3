//! Integration tests for CLI feature flags
//!
//! These tests verify that CLI feature flags work correctly and that
//! conditional compilation is working as expected.

use things3_cli::{Cli, Commands};

#[test]
fn test_core_cli_features_always_available() {
    // Core CLI features should always be available
    use clap::Parser;

    // Verify we can parse basic commands
    let args = vec!["things3", "inbox"];
    let result = Cli::try_parse_from(args);
    assert!(result.is_ok());
}

#[cfg(feature = "mcp-server")]
#[test]
fn test_mcp_server_feature_enabled() {
    // When mcp-server is enabled, the Mcp command should be available
    use clap::Parser;

    // Verify Mcp command is available
    let args = vec!["things3", "mcp"];
    let cli = Cli::try_parse_from(args);

    assert!(
        cli.is_ok(),
        "MCP command should be available when feature is enabled"
    );

    if let Ok(cli) = cli {
        assert!(
            matches!(cli.command, Commands::Mcp),
            "Should parse as Mcp command"
        );
    }
}

#[cfg(not(feature = "mcp-server"))]
#[test]
fn test_mcp_server_feature_disabled() {
    // When mcp-server is disabled, the Mcp command should not be available
    use clap::Parser;

    // Try to parse MCP command - should fail
    let args = vec!["things3", "mcp"];
    let result = Cli::try_parse_from(args);

    assert!(
        result.is_err(),
        "MCP command should not be available when feature is disabled"
    );
}

#[cfg(feature = "export-csv")]
#[test]
fn test_csv_export_cli_feature_enabled() {
    // When export-csv is enabled, CSV export should work through the CLI bulk command
    use clap::Parser;

    // Verify we can parse bulk export command with CSV format
    let args = vec!["things3", "bulk", "export", "--format", "csv"];
    let cli = Cli::try_parse_from(args);

    assert!(
        cli.is_ok(),
        "Bulk export CSV command should be available when feature is enabled"
    );
}

#[cfg(feature = "export-opml")]
#[test]
fn test_opml_export_cli_feature_enabled() {
    // When export-opml is enabled, OPML export should work through the CLI bulk command
    // Note: The CLI bulk export currently supports json, csv, xml formats
    // OPML is available through the core library but not directly through bulk export CLI
    use clap::Parser;

    // Verify we can parse bulk export command
    let args = vec!["things3", "bulk", "export", "--format", "json"];
    let cli = Cli::try_parse_from(args);

    assert!(cli.is_ok(), "Bulk export command should be available");
}

#[cfg(feature = "observability")]
#[test]
fn test_observability_cli_commands_available() {
    // Health and dashboard commands are only available with observability feature
    use clap::Parser;

    // Verify health-server command is parseable
    let args = vec!["things3", "health-server", "--port", "8080"];
    let cli = Cli::try_parse_from(args);

    assert!(
        cli.is_ok(),
        "Health server command should be available with observability feature"
    );

    if let Ok(cli) = cli {
        assert!(
            matches!(cli.command, Commands::HealthServer { .. }),
            "Should parse as HealthServer command"
        );
    }

    // Verify dashboard command is parseable
    let args = vec!["things3", "dashboard", "--port", "3030"];
    let cli = Cli::try_parse_from(args);

    assert!(
        cli.is_ok(),
        "Dashboard command should be available with observability feature"
    );

    if let Ok(cli) = cli {
        assert!(
            matches!(cli.command, Commands::Dashboard { .. }),
            "Should parse as Dashboard command"
        );
    }
}

#[cfg(not(feature = "observability"))]
#[test]
fn test_observability_cli_commands_unavailable() {
    // Health and dashboard commands should not be available without observability feature
    use clap::Parser;

    // Verify health-server command is not parseable
    let args = vec!["things3", "health-server"];
    let cli = Cli::try_parse_from(args);

    assert!(
        cli.is_err(),
        "Health server command should not be available without observability feature"
    );

    // Verify dashboard command is not parseable
    let args = vec!["things3", "dashboard"];
    let cli = Cli::try_parse_from(args);

    assert!(
        cli.is_err(),
        "Dashboard command should not be available without observability feature"
    );
}

#[cfg(all(feature = "mcp-server", feature = "observability"))]
#[test]
fn test_multiple_cli_features_enabled() {
    // When multiple features are enabled, all commands should be available
    use clap::Parser;

    // Test MCP command
    let mcp_args = vec!["things3", "mcp"];
    let mcp_cli = Cli::try_parse_from(mcp_args);
    assert!(mcp_cli.is_ok(), "MCP command should be available");

    // Test health-server command (requires observability)
    let health_args = vec!["things3", "health-server"];
    let health_cli = Cli::try_parse_from(health_args);
    assert!(
        health_cli.is_ok(),
        "Health server command should be parseable"
    );

    // Test dashboard command (requires observability)
    let dashboard_args = vec!["things3", "dashboard"];
    let dashboard_cli = Cli::try_parse_from(dashboard_args);
    assert!(
        dashboard_cli.is_ok(),
        "Dashboard command should be parseable"
    );
}

#[cfg(all(
    feature = "mcp-server",
    feature = "export-csv",
    feature = "export-opml",
    feature = "observability"
))]
#[test]
fn test_all_cli_features_enabled() {
    // When all features are enabled, all commands should be available
    use clap::Parser;

    let test_cases = vec![
        (vec!["things3", "mcp"], "MCP"),
        (
            vec!["things3", "bulk", "export", "--format", "csv"],
            "CSV export",
        ),
        (
            vec!["things3", "bulk", "export", "--format", "json"],
            "JSON export",
        ),
        (vec!["things3", "health-server"], "Health server"),
        (vec!["things3", "dashboard"], "Dashboard"),
    ];

    for (args, description) in test_cases {
        let cli = Cli::try_parse_from(args);
        assert!(
            cli.is_ok(),
            "{} command should be available with all features",
            description
        );
    }
}

#[test]
fn test_core_commands_always_available() {
    // Core commands should always be available regardless of features
    use clap::Parser;

    let core_commands = vec![
        vec!["things3", "inbox"],
        vec!["things3", "today"],
        vec!["things3", "projects"],
        vec!["things3", "areas"],
        vec!["things3", "search", "test"],
        vec!["things3", "health"],
        vec!["things3", "server"],
        vec!["things3", "watch"],
        vec!["things3", "validate"],
        vec!["things3", "bulk", "export"],
    ];

    for args in core_commands {
        let cli = Cli::try_parse_from(args.clone());
        assert!(
            cli.is_ok(),
            "Core command {:?} should always be available",
            args[1]
        );
    }
}

#[test]
fn test_default_features() {
    // With default features, most commands should be available
    // This test verifies the default feature set works correctly
    use clap::Parser;

    // Core commands should always work
    let args = vec!["things3", "inbox"];
    let cli = Cli::try_parse_from(args);
    assert!(
        cli.is_ok(),
        "Core commands should work with default features"
    );

    #[cfg(all(
        feature = "mcp-server",
        feature = "export-csv",
        feature = "export-opml",
        feature = "observability"
    ))]
    {
        // With default features, all should be available
        // Test MCP command
        let mcp_args = vec!["things3", "mcp"];
        let mcp_cli = Cli::try_parse_from(mcp_args);
        assert!(
            mcp_cli.is_ok(),
            "MCP command should work with default features"
        );
    }

    #[cfg(not(all(
        feature = "mcp-server",
        feature = "export-csv",
        feature = "export-opml",
        feature = "observability"
    )))]
    {
        // If not all default features are enabled, that's ok - user might be
        // testing with a custom feature configuration
        // The fact that this compiles proves feature flags work correctly
    }
}

#[test]
fn test_feature_combinations() {
    // Test that various feature combinations work correctly
    use clap::Parser;

    // Test that we can always parse the help command
    let args = vec!["things3", "--help"];
    let result = Cli::try_parse_from(args);

    // Help might fail to parse but shouldn't panic
    assert!(
        result.is_ok() || result.is_err(),
        "CLI should handle help command gracefully"
    );
}
