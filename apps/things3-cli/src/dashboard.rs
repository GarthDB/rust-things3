async fn dashboard_home(State(_state): State<DashboardState>) -> Html<&'static str> {
    Html(include_str!("dashboard.html"))
}

async fn get_metrics(
    State(state): State<DashboardState>,
) -> Result<Json<DashboardMetrics>, StatusCode> {
    let health = state.observability.health_status();
    let system_metrics = SystemMetrics {
        memory_usage: 1024.0,
        cpu_usage: 0.5,
        uptime: 3600,
        cache_hit_rate: 0.95,
        cache_size: 512.0,
    };
    let application_metrics = ApplicationMetrics {
        db_operations_total: 1000,
        tasks_created_total: 50,
        tasks_updated_total: 25,
        tasks_deleted_total: 5,
        tasks_completed_total: 30,
        search_operations_total: 200,
        export_operations_total: 10,
        errors_total: 2,
    };
    let log_statistics = LogStatistics {
        total_entries: 1000,
        level_counts: HashMap::new(),
        target_counts: HashMap::new(),
        recent_errors: Vec::new(),
    };
    let metrics = DashboardMetrics {
        health,
        system_metrics,
        application_metrics,
        log_statistics,
    };
    Ok(Json(metrics))
}

async fn get_health(State(state): State<DashboardState>) -> Result<Json<HealthStatus>, StatusCode> {
    let health = state.observability.health_status();
    Ok(Json(health))
}

async fn get_logs(State(_state): State<DashboardState>) -> Result<Json<Vec<LogEntry>>, StatusCode> {
    // Mock log entries - in a real implementation, these would come from log files
    let logs = vec![
        LogEntry {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            level: "INFO".to_string(),
            target: "things3_cli".to_string(),
            message: "Application started".to_string(),
        },
        LogEntry {
            timestamp: "2024-01-01T00:01:00Z".to_string(),
            level: "DEBUG".to_string(),
            target: "things3_cli::database".to_string(),
            message: "Database connection established".to_string(),
        },
        LogEntry {
            timestamp: "2024-01-01T00:02:00Z".to_string(),
            level: "WARN".to_string(),
            target: "things3_cli::metrics".to_string(),
            message: "High memory usage detected".to_string(),
        },
    ];
    Ok(Json(logs))
}

async fn search_logs(
    State(_state): State<DashboardState>,
    Json(_query): Json<LogSearchQuery>,
) -> Result<Json<Vec<LogEntry>>, StatusCode> {
    // Mock search results - in a real implementation, this would search through log files
    let logs = vec![LogEntry {
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        level: "INFO".to_string(),
        target: "things3_cli".to_string(),
        message: "Application started".to_string(),
    }];
    Ok(Json(logs))
}

async fn get_system_info(
    State(_state): State<DashboardState>,
) -> Result<Json<SystemInfo>, StatusCode> {
    // Mock system info - in a real implementation, this would come from system APIs
    let system_info = SystemInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        rust_version: std::env::var("RUSTC_SEMVER").unwrap_or_else(|_| "unknown".to_string()),
    };

    Ok(Json(system_info))
}

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use things3_core::{HealthStatus, ObservabilityManager, ThingsDatabase};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::{info, instrument};

// Struct definitions - must come after all functions to avoid items_after_statements
/// Dashboard state
#[derive(Clone)]
pub struct DashboardState {
    pub observability: Arc<ObservabilityManager>,
    pub database: Arc<ThingsDatabase>,
}

/// Dashboard metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub health: HealthStatus,
    pub system_metrics: SystemMetrics,
    pub application_metrics: ApplicationMetrics,
    pub log_statistics: LogStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub memory_usage: f64,
    pub cpu_usage: f64,
    pub uptime: u64,
    pub cache_hit_rate: f64,
    pub cache_size: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationMetrics {
    pub db_operations_total: u64,
    pub tasks_created_total: u64,
    pub tasks_updated_total: u64,
    pub tasks_deleted_total: u64,
    pub tasks_completed_total: u64,
    pub search_operations_total: u64,
    pub export_operations_total: u64,
    pub errors_total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogStatistics {
    pub total_entries: u64,
    pub level_counts: HashMap<String, u64>,
    pub target_counts: HashMap<String, u64>,
    pub recent_errors: Vec<LogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSearchQuery {
    pub query: String,
    pub level: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub version: String,
    pub rust_version: String,
}

impl DashboardServer {
    /// Create a new dashboard server
    #[must_use]
    pub fn new(
        port: u16,
        observability: Arc<ObservabilityManager>,
        database: Arc<ThingsDatabase>,
    ) -> Self {
        Self {
            port,
            observability,
            database,
        }
    }

    /// Start the dashboard server
    ///
    /// # Errors
    /// Returns an error if the server fails to start or bind to the port
    #[instrument(skip(self))]
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let state = DashboardState {
            observability: self.observability,
            database: self.database,
        };

        let app = Router::new()
            .route("/", get(dashboard_home))
            .route("/metrics", get(get_metrics))
            .route("/health", get(get_health))
            .route("/logs", get(get_logs))
            .route("/logs/search", post(search_logs))
            .route("/system", get(get_system_info))
            .layer(CorsLayer::permissive())
            .with_state(state);

        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
        info!("Dashboard server running on port {}", self.port);

        axum::serve(listener, app).await?;
        Ok(())
    }
}

/// Dashboard server
pub struct DashboardServer {
    port: u16,
    observability: Arc<ObservabilityManager>,
    database: Arc<ThingsDatabase>,
}

/// Start the dashboard server
///
/// # Errors
/// Returns an error if the server fails to start or bind to the port
#[instrument(skip(observability, database))]
pub async fn start_dashboard_server(
    port: u16,
    observability: Arc<ObservabilityManager>,
    database: Arc<ThingsDatabase>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = DashboardServer::new(port, observability, database);
    server.start().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_dashboard_server_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        let config = things3_core::ThingsConfig::new(db_path, false);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let database = Arc::new(
            rt.block_on(async { ThingsDatabase::new(&config.database_path).await.unwrap() }),
        );

        let observability = Arc::new(
            things3_core::ObservabilityManager::new(things3_core::ObservabilityConfig::default())
                .unwrap(),
        );
        let server = DashboardServer::new(8080, observability, database);
        assert_eq!(server.port, 8080);
    }

    #[test]
    fn test_dashboard_metrics() {
        let metrics = DashboardMetrics {
            health: HealthStatus {
                status: "healthy".to_string(),
                timestamp: chrono::Utc::now(),
                uptime: std::time::Duration::from_secs(3600),
                version: env!("CARGO_PKG_VERSION").to_string(),
                checks: std::collections::HashMap::new(),
            },
            system_metrics: SystemMetrics {
                memory_usage: 1024.0,
                cpu_usage: 0.5,
                uptime: 3600,
                cache_hit_rate: 0.95,
                cache_size: 512.0,
            },
            application_metrics: ApplicationMetrics {
                db_operations_total: 1000,
                tasks_created_total: 50,
                tasks_updated_total: 25,
                tasks_deleted_total: 5,
                tasks_completed_total: 30,
                search_operations_total: 200,
                export_operations_total: 10,
                errors_total: 2,
            },
            log_statistics: LogStatistics {
                total_entries: 1000,
                level_counts: HashMap::new(),
                target_counts: HashMap::new(),
                recent_errors: Vec::new(),
            },
        };

        assert!((metrics.system_metrics.memory_usage - 1024.0).abs() < f64::EPSILON);
        assert_eq!(metrics.application_metrics.db_operations_total, 1000);
    }

    #[test]
    fn test_system_metrics_creation() {
        let system_metrics = SystemMetrics {
            memory_usage: 2048.0,
            cpu_usage: 0.75,
            uptime: 7200,
            cache_hit_rate: 0.88,
            cache_size: 1024.0,
        };

        assert!((system_metrics.memory_usage - 2048.0).abs() < f64::EPSILON);
        assert!((system_metrics.cpu_usage - 0.75).abs() < f64::EPSILON);
        assert_eq!(system_metrics.uptime, 7200);
        assert!((system_metrics.cache_hit_rate - 0.88).abs() < f64::EPSILON);
        assert!((system_metrics.cache_size - 1024.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_application_metrics_creation() {
        let app_metrics = ApplicationMetrics {
            db_operations_total: 5000,
            tasks_created_total: 100,
            tasks_updated_total: 50,
            tasks_deleted_total: 10,
            tasks_completed_total: 80,
            search_operations_total: 500,
            export_operations_total: 25,
            errors_total: 5,
        };

        assert_eq!(app_metrics.db_operations_total, 5000);
        assert_eq!(app_metrics.tasks_created_total, 100);
        assert_eq!(app_metrics.tasks_updated_total, 50);
        assert_eq!(app_metrics.tasks_deleted_total, 10);
        assert_eq!(app_metrics.tasks_completed_total, 80);
        assert_eq!(app_metrics.search_operations_total, 500);
        assert_eq!(app_metrics.export_operations_total, 25);
        assert_eq!(app_metrics.errors_total, 5);
    }

    #[test]
    fn test_log_statistics_creation() {
        let mut level_counts = HashMap::new();
        level_counts.insert("INFO".to_string(), 100);
        level_counts.insert("ERROR".to_string(), 5);
        level_counts.insert("WARN".to_string(), 10);

        let mut target_counts = HashMap::new();
        target_counts.insert("things3_cli".to_string(), 80);
        target_counts.insert("things3_cli::database".to_string(), 20);

        let recent_errors = vec![LogEntry {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            level: "ERROR".to_string(),
            target: "things3_cli".to_string(),
            message: "Database connection failed".to_string(),
        }];

        let log_stats = LogStatistics {
            total_entries: 115,
            level_counts,
            target_counts,
            recent_errors,
        };

        assert_eq!(log_stats.total_entries, 115);
        assert_eq!(log_stats.level_counts.get("INFO"), Some(&100));
        assert_eq!(log_stats.level_counts.get("ERROR"), Some(&5));
        assert_eq!(log_stats.level_counts.get("WARN"), Some(&10));
        assert_eq!(log_stats.target_counts.get("things3_cli"), Some(&80));
        assert_eq!(log_stats.recent_errors.len(), 1);
    }

    #[test]
    fn test_log_entry_creation() {
        let log_entry = LogEntry {
            timestamp: "2024-01-01T12:00:00Z".to_string(),
            level: "DEBUG".to_string(),
            target: "things3_cli::cache".to_string(),
            message: "Cache miss for key: user_123".to_string(),
        };

        assert_eq!(log_entry.timestamp, "2024-01-01T12:00:00Z");
        assert_eq!(log_entry.level, "DEBUG");
        assert_eq!(log_entry.target, "things3_cli::cache");
        assert_eq!(log_entry.message, "Cache miss for key: user_123");
    }

    #[test]
    fn test_log_search_query_creation() {
        let search_query = LogSearchQuery {
            query: "database".to_string(),
            level: Some("ERROR".to_string()),
            start_time: Some("2024-01-01T00:00:00Z".to_string()),
            end_time: Some("2024-01-01T23:59:59Z".to_string()),
        };

        assert_eq!(search_query.query, "database");
        assert_eq!(search_query.level, Some("ERROR".to_string()));
        assert_eq!(
            search_query.start_time,
            Some("2024-01-01T00:00:00Z".to_string())
        );
        assert_eq!(
            search_query.end_time,
            Some("2024-01-01T23:59:59Z".to_string())
        );
    }

    #[test]
    fn test_log_search_query_minimal() {
        let search_query = LogSearchQuery {
            query: "test".to_string(),
            level: None,
            start_time: None,
            end_time: None,
        };

        assert_eq!(search_query.query, "test");
        assert_eq!(search_query.level, None);
        assert_eq!(search_query.start_time, None);
        assert_eq!(search_query.end_time, None);
    }

    #[test]
    fn test_system_info_creation() {
        let system_info = SystemInfo {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            version: "1.0.0".to_string(),
            rust_version: "1.70.0".to_string(),
        };

        assert_eq!(system_info.os, "linux");
        assert_eq!(system_info.arch, "x86_64");
        assert_eq!(system_info.version, "1.0.0");
        assert_eq!(system_info.rust_version, "1.70.0");
    }

    #[test]
    fn test_dashboard_state_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        let config = things3_core::ThingsConfig::new(db_path, false);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let database = Arc::new(
            rt.block_on(async { ThingsDatabase::new(&config.database_path).await.unwrap() }),
        );

        let observability = Arc::new(
            things3_core::ObservabilityManager::new(things3_core::ObservabilityConfig::default())
                .unwrap(),
        );

        let state = DashboardState {
            observability: observability.clone(),
            database: database.clone(),
        };

        // Test that the state can be cloned
        let cloned_state = state.clone();
        assert!(Arc::ptr_eq(
            &cloned_state.observability,
            &state.observability
        ));
        assert!(Arc::ptr_eq(&cloned_state.database, &state.database));
    }

    #[test]
    fn test_dashboard_metrics_serialization() {
        let metrics = DashboardMetrics {
            health: HealthStatus {
                status: "healthy".to_string(),
                timestamp: chrono::Utc::now(),
                uptime: std::time::Duration::from_secs(3600),
                version: "1.0.0".to_string(),
                checks: HashMap::new(),
            },
            system_metrics: SystemMetrics {
                memory_usage: 1024.0,
                cpu_usage: 0.5,
                uptime: 3600,
                cache_hit_rate: 0.95,
                cache_size: 512.0,
            },
            application_metrics: ApplicationMetrics {
                db_operations_total: 1000,
                tasks_created_total: 50,
                tasks_updated_total: 25,
                tasks_deleted_total: 5,
                tasks_completed_total: 30,
                search_operations_total: 200,
                export_operations_total: 10,
                errors_total: 2,
            },
            log_statistics: LogStatistics {
                total_entries: 1000,
                level_counts: HashMap::new(),
                target_counts: HashMap::new(),
                recent_errors: Vec::new(),
            },
        };

        // Test serialization
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("1024.0"));
        assert!(json.contains("1000"));

        // Test deserialization
        let deserialized: DashboardMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.health.status, "healthy");
        assert!((deserialized.system_metrics.memory_usage - 1024.0).abs() < f64::EPSILON);
        assert_eq!(deserialized.application_metrics.db_operations_total, 1000);
    }

    #[test]
    fn test_system_metrics_serialization() {
        let system_metrics = SystemMetrics {
            memory_usage: 2048.0,
            cpu_usage: 0.75,
            uptime: 7200,
            cache_hit_rate: 0.88,
            cache_size: 1024.0,
        };

        let json = serde_json::to_string(&system_metrics).unwrap();
        let deserialized: SystemMetrics = serde_json::from_str(&json).unwrap();

        assert!((deserialized.memory_usage - 2048.0).abs() < f64::EPSILON);
        assert!((deserialized.cpu_usage - 0.75).abs() < f64::EPSILON);
        assert_eq!(deserialized.uptime, 7200);
        assert!((deserialized.cache_hit_rate - 0.88).abs() < f64::EPSILON);
        assert!((deserialized.cache_size - 1024.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_application_metrics_serialization() {
        let app_metrics = ApplicationMetrics {
            db_operations_total: 5000,
            tasks_created_total: 100,
            tasks_updated_total: 50,
            tasks_deleted_total: 10,
            tasks_completed_total: 80,
            search_operations_total: 500,
            export_operations_total: 25,
            errors_total: 5,
        };

        let json = serde_json::to_string(&app_metrics).unwrap();
        let deserialized: ApplicationMetrics = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.db_operations_total, 5000);
        assert_eq!(deserialized.tasks_created_total, 100);
        assert_eq!(deserialized.tasks_updated_total, 50);
        assert_eq!(deserialized.tasks_deleted_total, 10);
        assert_eq!(deserialized.tasks_completed_total, 80);
        assert_eq!(deserialized.search_operations_total, 500);
        assert_eq!(deserialized.export_operations_total, 25);
        assert_eq!(deserialized.errors_total, 5);
    }

    #[test]
    fn test_log_entry_serialization() {
        let log_entry = LogEntry {
            timestamp: "2024-01-01T12:00:00Z".to_string(),
            level: "DEBUG".to_string(),
            target: "things3_cli::cache".to_string(),
            message: "Cache miss for key: user_123".to_string(),
        };

        let json = serde_json::to_string(&log_entry).unwrap();
        let deserialized: LogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.timestamp, "2024-01-01T12:00:00Z");
        assert_eq!(deserialized.level, "DEBUG");
        assert_eq!(deserialized.target, "things3_cli::cache");
        assert_eq!(deserialized.message, "Cache miss for key: user_123");
    }

    #[test]
    fn test_log_search_query_serialization() {
        let search_query = LogSearchQuery {
            query: "database".to_string(),
            level: Some("ERROR".to_string()),
            start_time: Some("2024-01-01T00:00:00Z".to_string()),
            end_time: Some("2024-01-01T23:59:59Z".to_string()),
        };

        let json = serde_json::to_string(&search_query).unwrap();
        let deserialized: LogSearchQuery = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.query, "database");
        assert_eq!(deserialized.level, Some("ERROR".to_string()));
        assert_eq!(
            deserialized.start_time,
            Some("2024-01-01T00:00:00Z".to_string())
        );
        assert_eq!(
            deserialized.end_time,
            Some("2024-01-01T23:59:59Z".to_string())
        );
    }

    #[test]
    fn test_system_info_serialization() {
        let system_info = SystemInfo {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            version: "1.0.0".to_string(),
            rust_version: "1.70.0".to_string(),
        };

        let json = serde_json::to_string(&system_info).unwrap();
        let deserialized: SystemInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.os, "linux");
        assert_eq!(deserialized.arch, "x86_64");
        assert_eq!(deserialized.version, "1.0.0");
        assert_eq!(deserialized.rust_version, "1.70.0");
    }

    #[test]
    fn test_dashboard_metrics_debug_formatting() {
        let metrics = DashboardMetrics {
            health: HealthStatus {
                status: "healthy".to_string(),
                timestamp: chrono::Utc::now(),
                uptime: std::time::Duration::from_secs(3600),
                version: "1.0.0".to_string(),
                checks: HashMap::new(),
            },
            system_metrics: SystemMetrics {
                memory_usage: 1024.0,
                cpu_usage: 0.5,
                uptime: 3600,
                cache_hit_rate: 0.95,
                cache_size: 512.0,
            },
            application_metrics: ApplicationMetrics {
                db_operations_total: 1000,
                tasks_created_total: 50,
                tasks_updated_total: 25,
                tasks_deleted_total: 5,
                tasks_completed_total: 30,
                search_operations_total: 200,
                export_operations_total: 10,
                errors_total: 2,
            },
            log_statistics: LogStatistics {
                total_entries: 1000,
                level_counts: HashMap::new(),
                target_counts: HashMap::new(),
                recent_errors: Vec::new(),
            },
        };

        let debug_str = format!("{metrics:?}");
        assert!(debug_str.contains("DashboardMetrics"));
        assert!(debug_str.contains("SystemMetrics"));
        assert!(debug_str.contains("ApplicationMetrics"));
        assert!(debug_str.contains("LogStatistics"));
    }

    #[test]
    fn test_dashboard_metrics_clone() {
        let metrics = DashboardMetrics {
            health: HealthStatus {
                status: "healthy".to_string(),
                timestamp: chrono::Utc::now(),
                uptime: std::time::Duration::from_secs(3600),
                version: "1.0.0".to_string(),
                checks: HashMap::new(),
            },
            system_metrics: SystemMetrics {
                memory_usage: 1024.0,
                cpu_usage: 0.5,
                uptime: 3600,
                cache_hit_rate: 0.95,
                cache_size: 512.0,
            },
            application_metrics: ApplicationMetrics {
                db_operations_total: 1000,
                tasks_created_total: 50,
                tasks_updated_total: 25,
                tasks_deleted_total: 5,
                tasks_completed_total: 30,
                search_operations_total: 200,
                export_operations_total: 10,
                errors_total: 2,
            },
            log_statistics: LogStatistics {
                total_entries: 1000,
                level_counts: HashMap::new(),
                target_counts: HashMap::new(),
                recent_errors: Vec::new(),
            },
        };

        let cloned_metrics = metrics.clone();
        assert_eq!(cloned_metrics.health.status, metrics.health.status);
        assert!(
            (cloned_metrics.system_metrics.memory_usage - metrics.system_metrics.memory_usage)
                .abs()
                < f64::EPSILON
        );
        assert_eq!(
            cloned_metrics.application_metrics.db_operations_total,
            metrics.application_metrics.db_operations_total
        );
        assert_eq!(
            cloned_metrics.log_statistics.total_entries,
            metrics.log_statistics.total_entries
        );
    }
}
