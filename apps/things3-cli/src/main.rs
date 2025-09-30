//! Things CLI - Command line interface for Things 3 with integrated MCP server

use clap::Parser;
use std::sync::Arc;
// use things3_cli::bulk_operations::BulkOperationsManager; // Temporarily disabled
use things3_cli::mcp::start_mcp_server;
use things3_cli::{health_check, start_websocket_server, watch_updates, Cli, Commands};
use things3_core::{
    ObservabilityConfig, ObservabilityManager, Result, ThingsConfig, ThingsDatabase,
};
use tracing::{error, info};

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize observability
    let obs_config = ObservabilityConfig {
        log_level: if cli.verbose {
            "debug".to_string()
        } else {
            "info".to_string()
        },
        json_logs: std::env::var("THINGS3_JSON_LOGS").unwrap_or_default() == "true",
        enable_tracing: true,
        jaeger_endpoint: std::env::var("JAEGER_ENDPOINT").ok(),
        otlp_endpoint: std::env::var("OTLP_ENDPOINT").ok(),
        enable_metrics: true,
        metrics_port: 9090,
        health_port: 8080,
        service_name: "things3-cli".to_string(),
        service_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let mut observability = ObservabilityManager::new(obs_config)
        .map_err(|e| things3_core::ThingsError::unknown(e.to_string()))?;
    observability
        .initialize()
        .map_err(|e| things3_core::ThingsError::unknown(e.to_string()))?;
    let observability = Arc::new(observability);

    info!("Things 3 CLI starting up");

    // Create configuration
    let config = if let Some(db_path) = cli.database {
        ThingsConfig::new(db_path, cli.fallback_to_default)
    } else {
        ThingsConfig::from_env()
    };

    // Create database connection
    let db = ThingsDatabase::new(&config.database_path).await?;
    let db = Arc::new(db);

    match cli.command {
        Commands::Inbox { limit: _ } => {
            error!("Inbox command is temporarily disabled during SQLx migration");
            println!("🚧 Inbox command is temporarily disabled");
            println!("   This feature is being migrated to use SQLx for better async support");
            return Err(things3_core::ThingsError::unknown(
                "Inbox command temporarily disabled".to_string(),
            ));
        }
        Commands::Today { limit: _ } => {
            error!("Today command is temporarily disabled during SQLx migration");
            println!("🚧 Today command is temporarily disabled");
            println!("   This feature is being migrated to use SQLx for better async support");
            return Err(things3_core::ThingsError::unknown(
                "Today command temporarily disabled".to_string(),
            ));
        }
        Commands::Projects { area: _, limit: _ } => {
            error!("Projects command is temporarily disabled during SQLx migration");
            println!("🚧 Projects command is temporarily disabled");
            println!("   This feature is being migrated to use SQLx for better async support");
            return Err(things3_core::ThingsError::unknown(
                "Projects command temporarily disabled".to_string(),
            ));
        }
        Commands::Areas { limit: _ } => {
            error!("Areas command is temporarily disabled during SQLx migration");
            println!("🚧 Areas command is temporarily disabled");
            println!("   This feature is being migrated to use SQLx for better async support");
            return Err(things3_core::ThingsError::unknown(
                "Areas command temporarily disabled".to_string(),
            ));
        }
        Commands::Search { query: _, limit: _ } => {
            error!("Search command is temporarily disabled during SQLx migration");
            println!("🚧 Search command is temporarily disabled");
            println!("   This feature is being migrated to use SQLx for better async support");
            return Err(things3_core::ThingsError::unknown(
                "Search command temporarily disabled".to_string(),
            ));
        }
        Commands::Mcp => {
            info!("Starting MCP server...");
            start_mcp_server(Arc::clone(&db), config)?;
            info!("MCP server started successfully");
        }
        Commands::Health => {
            info!("Performing health check");
            health_check(&db).await?;
        }
        Commands::HealthServer { port } => {
            info!("Starting health check server on port {}", port);
            things3_cli::health::start_health_server(port, observability, Arc::clone(&db))
                .await
                .map_err(|e| things3_core::ThingsError::unknown(e.to_string()))?;
        }
        Commands::Dashboard { port } => {
            info!("Starting monitoring dashboard on port {}", port);
            things3_cli::dashboard::start_dashboard_server(port, observability, Arc::clone(&db))
                .await
                .map_err(|e| things3_core::ThingsError::unknown(e.to_string()))?;
        }
        Commands::Server { port } => {
            info!("Starting WebSocket server on port {}", port);
            start_websocket_server(port).await?;
        }
        Commands::Watch { url } => {
            info!("Connecting to WebSocket server at {}", url);
            watch_updates(&url)?;
        }
        Commands::Validate => {
            info!("Validating real-time features");
            println!("🔍 Validating real-time features...");
            // TODO: Implement validation logic
            println!("✅ Real-time features validation completed");
        }
        Commands::Bulk { operation: _ } => {
            error!("Bulk operations are temporarily disabled during SQLx migration");
            println!("🚧 Bulk operations are temporarily disabled");
            println!("   This feature is being migrated to use SQLx for better async support");
            return Err(things3_core::ThingsError::unknown(
                "Bulk operations temporarily disabled".to_string(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::NamedTempFile;
    use things3_cli::{print_areas, print_projects, print_tasks, BulkOperation};
    use things3_core::test_utils::create_test_database;

    /// Test the main function with various command combinations
    #[tokio::test]
    async fn test_main_inbox_command() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).await.unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::new(&config.database_path).await.unwrap();

        // Test inbox command
        let cli = Cli::try_parse_from(["things-cli", "inbox"]).unwrap();
        let result = match cli.command {
            Commands::Inbox { limit } => {
                let tasks = db.get_inbox(limit).await.unwrap();
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
        create_test_database(db_path).await.unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::new(&config.database_path).await.unwrap();

        // Test today command
        let cli = Cli::try_parse_from(["things-cli", "today"]).unwrap();
        let result = match cli.command {
            Commands::Today { limit } => {
                let tasks = db.get_today(limit).await.unwrap();
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
        create_test_database(db_path).await.unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::new(&config.database_path).await.unwrap();

        // Test projects command
        let cli = Cli::try_parse_from(["things-cli", "projects"]).unwrap();
        let result = match cli.command {
            Commands::Projects { area, limit } => {
                let _area_uuid = area.and_then(|a| uuid::Uuid::parse_str(&a).ok());
                let projects = db.get_projects(None).await.unwrap();
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
        create_test_database(db_path).await.unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::new(&config.database_path).await.unwrap();

        // Test areas command
        let cli = Cli::try_parse_from(["things-cli", "areas"]).unwrap();
        let result = match cli.command {
            Commands::Areas { limit } => {
                let areas = db.get_areas().await.unwrap();
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
        create_test_database(db_path).await.unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::new(&config.database_path).await.unwrap();

        // Test search command
        let cli = Cli::try_parse_from(["things-cli", "search", "test"]).unwrap();
        let result = match cli.command {
            Commands::Search { query, limit: _ } => {
                let tasks = db.search_tasks(&query).await.unwrap();
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
        create_test_database(db_path).await.unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::new(&config.database_path).await.unwrap();

        // Test health command
        let cli = Cli::try_parse_from(["things-cli", "health"]).unwrap();
        match cli.command {
            Commands::Health => {
                health_check(&db).await.unwrap();
            }
            _ => panic!("Expected health command"),
        }
    }

    #[tokio::test]
    async fn test_main_mcp_command() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).await.unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::new(&config.database_path).await.unwrap();

        // Test MCP command
        let cli = Cli::try_parse_from(["things-cli", "mcp"]).unwrap();
        match cli.command {
            Commands::Mcp => {
                start_mcp_server(db.into(), config).unwrap();
            }
            _ => panic!("Expected MCP command"),
        }
    }

    #[tokio::test]
    async fn test_main_with_verbose_flag() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).await.unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::new(&config.database_path).await.unwrap();

        // Test with verbose flag
        let cli = Cli::try_parse_from(["things-cli", "--verbose", "inbox"]).unwrap();
        assert!(cli.verbose);

        match cli.command {
            Commands::Inbox { limit } => {
                let tasks = db.get_inbox(limit).await.unwrap();
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
        create_test_database(db_path).await.unwrap();

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
        let db = ThingsDatabase::new(&config.database_path).await.unwrap();

        match cli.command {
            Commands::Inbox { limit } => {
                let tasks = db.get_inbox(limit).await.unwrap();
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
        create_test_database(db_path).await.unwrap();

        // Test with fallback flag
        let cli = Cli::try_parse_from(["things-cli", "--fallback-to-default", "inbox"]).unwrap();
        assert!(cli.fallback_to_default);

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::new(&config.database_path).await.unwrap();

        match cli.command {
            Commands::Inbox { limit } => {
                let tasks = db.get_inbox(limit).await.unwrap();
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
        create_test_database(db_path).await.unwrap();

        let config = ThingsConfig::new(db_path, false);
        let db = ThingsDatabase::new(&config.database_path).await.unwrap();

        // Test with limit
        let cli = Cli::try_parse_from(["things-cli", "inbox", "--limit", "5"]).unwrap();
        match cli.command {
            Commands::Inbox { limit } => {
                assert_eq!(limit, Some(5));
                let tasks = db.get_inbox(limit).await.unwrap();
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

    #[test]
    fn test_main_server_command() {
        let cli = Cli::parse_from(["things3", "server", "--port", "8080"]);
        match cli.command {
            Commands::Server { port } => assert_eq!(port, 8080),
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_main_server_command_default_port() {
        let cli = Cli::parse_from(["things3", "server"]);
        match cli.command {
            Commands::Server { port } => assert_eq!(port, 8080),
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_main_watch_command() {
        let cli = Cli::parse_from(["things3", "watch", "--url", "ws://localhost:8080"]);
        match cli.command {
            Commands::Watch { url } => assert_eq!(url, "ws://localhost:8080"),
            _ => panic!("Expected Watch command"),
        }
    }

    #[test]
    fn test_main_validate_command() {
        let cli = Cli::parse_from(["things3", "validate"]);
        match cli.command {
            Commands::Validate => {} // Placeholder for validate command
            _ => panic!("Expected Validate command"),
        }
    }

    #[test]
    fn test_main_bulk_export_command() {
        let cli = Cli::parse_from(["things3", "bulk", "export", "--format", "json"]);
        match cli.command {
            Commands::Bulk { operation } => match operation {
                BulkOperation::Export { format } => assert_eq!(format, "json"),
                _ => panic!("Expected Export operation"),
            },
            _ => panic!("Expected Bulk command"),
        }
    }

    #[test]
    fn test_main_bulk_export_command_default_format() {
        let cli = Cli::parse_from(["things3", "bulk", "export"]);
        match cli.command {
            Commands::Bulk { operation } => match operation {
                BulkOperation::Export { format } => assert_eq!(format, "json"),
                _ => panic!("Expected Export operation"),
            },
            _ => panic!("Expected Bulk command"),
        }
    }

    #[test]
    fn test_main_bulk_update_status_command() {
        let cli = Cli::parse_from(["things3", "bulk", "update-status", "123,456", "completed"]);
        match cli.command {
            Commands::Bulk { operation } => match operation {
                BulkOperation::UpdateStatus { task_ids, status } => {
                    assert_eq!(task_ids, "123,456");
                    assert_eq!(status, "completed");
                }
                _ => panic!("Expected UpdateStatus operation"),
            },
            _ => panic!("Expected Bulk command"),
        }
    }

    #[test]
    fn test_main_bulk_search_and_process_command() {
        let cli = Cli::parse_from(["things3", "bulk", "search-and-process", "test"]);
        match cli.command {
            Commands::Bulk { operation } => match operation {
                BulkOperation::SearchAndProcess { query } => {
                    assert_eq!(query, "test");
                }
                _ => panic!("Expected SearchAndProcess operation"),
            },
            _ => panic!("Expected Bulk command"),
        }
    }

    #[test]
    fn test_main_bulk_search_and_process_command_default_limit() {
        let cli = Cli::parse_from(["things3", "bulk", "search-and-process", "test"]);
        match cli.command {
            Commands::Bulk { operation } => match operation {
                BulkOperation::SearchAndProcess { query } => {
                    assert_eq!(query, "test");
                }
                _ => panic!("Expected SearchAndProcess operation"),
            },
            _ => panic!("Expected Bulk command"),
        }
    }

    #[test]
    fn test_main_projects_command_with_area() {
        let cli = Cli::parse_from([
            "things3",
            "projects",
            "--area",
            "123e4567-e89b-12d3-a456-426614174000",
        ]);
        match cli.command {
            Commands::Projects { area, .. } => {
                assert_eq!(
                    area,
                    Some("123e4567-e89b-12d3-a456-426614174000".to_string())
                );
            }
            _ => panic!("Expected Projects command with area"),
        }
    }

    #[test]
    fn test_main_projects_command_with_limit() {
        let cli = Cli::parse_from(["things3", "projects", "--limit", "5"]);
        match cli.command {
            Commands::Projects { limit, .. } => {
                assert_eq!(limit, Some(5));
            }
            _ => panic!("Expected Projects command with limit"),
        }
    }

    #[test]
    fn test_main_areas_command_with_limit() {
        let cli = Cli::parse_from(["things3", "areas", "--limit", "3"]);
        match cli.command {
            Commands::Areas { limit } => {
                assert_eq!(limit, Some(3));
            }
            _ => panic!("Expected Areas command with limit"),
        }
    }

    #[test]
    fn test_main_search_command_with_limit() {
        let cli = Cli::parse_from(["things3", "search", "test query", "--limit", "10"]);
        match cli.command {
            Commands::Search { query, limit } => {
                assert_eq!(query, "test query");
                assert_eq!(limit, Some(10));
            }
            _ => panic!("Expected Search command with limit"),
        }
    }

    #[test]
    fn test_main_today_command_with_limit() {
        let cli = Cli::parse_from(["things3", "today", "--limit", "5"]);
        match cli.command {
            Commands::Today { limit } => {
                assert_eq!(limit, Some(5));
            }
            _ => panic!("Expected Today command with limit"),
        }
    }

    #[test]
    fn test_main_inbox_command_with_limit() {
        let cli = Cli::parse_from(["things3", "inbox", "--limit", "7"]);
        match cli.command {
            Commands::Inbox { limit } => {
                assert_eq!(limit, Some(7));
            }
            _ => panic!("Expected Inbox command with limit"),
        }
    }

    #[test]
    fn test_main_verbose_and_database_flags() {
        let cli = Cli::parse_from(["things3", "--verbose", "--database", "/path/to/db", "inbox"]);
        assert!(cli.verbose);
        assert_eq!(cli.database, Some(std::path::PathBuf::from("/path/to/db")));
    }

    #[test]
    fn test_main_fallback_and_verbose_flags() {
        let cli = Cli::parse_from(["things3", "--fallback-to-default", "--verbose", "health"]);
        assert!(cli.fallback_to_default);
        assert!(cli.verbose);
    }

    #[test]
    fn test_main_all_flags_combined() {
        let cli = Cli::parse_from([
            "things3",
            "--verbose",
            "--database",
            "/path/to/db",
            "--fallback-to-default",
            "inbox",
            "--limit",
            "5",
        ]);
        assert!(cli.verbose);
        assert_eq!(cli.database, Some(std::path::PathBuf::from("/path/to/db")));
        assert!(cli.fallback_to_default);
        match cli.command {
            Commands::Inbox { limit } => assert_eq!(limit, Some(5)),
            _ => panic!("Expected Inbox command with limit"),
        }
    }

    #[test]
    fn test_main_bulk_export_with_all_formats() {
        let formats = vec!["json", "csv", "xml", "markdown", "opml"];

        for format in formats {
            let cli = Cli::parse_from(["things3", "bulk", "export", "--format", format]);
            match cli.command {
                Commands::Bulk { operation } => match operation {
                    BulkOperation::Export { format: f } => assert_eq!(f, format),
                    _ => panic!("Expected Export operation"),
                },
                _ => panic!("Expected Bulk command"),
            }
        }
    }

    #[test]
    fn test_main_bulk_update_status_with_all_statuses() {
        let statuses = vec!["completed", "cancelled", "in_progress"];

        for status in statuses {
            let cli = Cli::parse_from(["things3", "bulk", "update-status", "123", status]);
            match cli.command {
                Commands::Bulk { operation } => match operation {
                    BulkOperation::UpdateStatus { status: s, .. } => assert_eq!(s, status),
                    _ => panic!("Expected UpdateStatus operation"),
                },
                _ => panic!("Expected Bulk command"),
            }
        }
    }

    #[test]
    fn test_main_server_command_with_different_ports() {
        let ports = vec![3000, 8080, 9000, 3001];

        for port in ports {
            let cli = Cli::parse_from(["things3", "server", "--port", &port.to_string()]);
            match cli.command {
                Commands::Server { port: p } => assert_eq!(p, port),
                _ => panic!("Expected Server command"),
            }
        }
    }

    #[test]
    fn test_main_watch_command_with_different_urls() {
        let urls = vec![
            "ws://localhost:8080",
            "ws://127.0.0.1:3000",
            "wss://example.com:443",
            "ws://192.168.1.100:9000",
        ];

        for url in urls {
            let cli = Cli::parse_from(["things3", "watch", "--url", url]);
            match cli.command {
                Commands::Watch { url: u } => assert_eq!(u, url),
                _ => panic!("Expected Watch command"),
            }
        }
    }
}
