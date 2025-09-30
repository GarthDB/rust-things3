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
}
