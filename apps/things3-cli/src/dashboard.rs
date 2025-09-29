//! Monitoring dashboard for Things 3 CLI
//!
//! This module provides a web-based monitoring dashboard for viewing
//! metrics, logs, and health status of the Things 3 CLI application.

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
use things3_core::{HealthStatus, ObservabilityManager, SqlxThingsDatabase};
use tokio::net::TcpListener;
// Removed unused import
use tower_http::cors::CorsLayer;
use tracing::{info, instrument};

/// Dashboard state
#[derive(Clone)]
pub struct DashboardState {
    pub observability: Arc<ObservabilityManager>,
    pub database: Arc<SqlxThingsDatabase>,
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
    pub total_entries: usize,
    pub level_counts: HashMap<String, usize>,
    pub target_counts: HashMap<String, usize>,
    pub recent_errors: Vec<LogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

/// Dashboard server
pub struct DashboardServer {
    port: u16,
    state: DashboardState,
}

impl DashboardServer {
    /// Create a new dashboard server
    pub fn new(port: u16, observability: Arc<ObservabilityManager>, database: Arc<SqlxThingsDatabase>) -> Self {
        let state = DashboardState {
            observability,
            database,
        };
        
        Self { port, state }
    }
    
    /// Start the dashboard server
    #[instrument(skip(self))]
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let port = self.port;
        let app = self.create_app();
        
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        info!("Dashboard server started on port {}", port);
        
        axum::serve(listener, app).await?;
        Ok(())
    }
    
    /// Create the Axum application
    fn create_app(self) -> Router {
        Router::new()
            .route("/", get(dashboard_home))
            .route("/api/metrics", get(get_metrics))
            .route("/api/health", get(get_health))
            .route("/api/logs", get(get_logs))
            .route("/api/logs/search", post(search_logs))
            .route("/api/system", get(get_system_info))
            .with_state(self.state)
            .layer(CorsLayer::permissive())
    }
}

/// Dashboard home page
#[axum::debug_handler]
#[instrument(skip(_state))]
async fn dashboard_home(State(_state): State<DashboardState>) -> Html<&'static str> {
    Html(include_str!("dashboard.html"))
}

/// Get metrics endpoint
#[axum::debug_handler]
#[instrument(skip(state))]
async fn get_metrics(State(state): State<DashboardState>) -> Result<Json<DashboardMetrics>, StatusCode> {
    let health = state.observability.health_status();
    
    // Get system metrics (placeholder values for now)
    let system_metrics = SystemMetrics {
        memory_usage: 0.0, // Would get from system monitoring
        cpu_usage: 0.0,    // Would get from system monitoring
        uptime: health.uptime.as_secs(),
        cache_hit_rate: 0.85, // Would get from cache metrics
        cache_size: 1024.0,   // Would get from cache metrics
    };
    
    // Get application metrics (placeholder values for now)
    let application_metrics = ApplicationMetrics {
        db_operations_total: 0,    // Would get from metrics
        tasks_created_total: 0,    // Would get from metrics
        tasks_updated_total: 0,    // Would get from metrics
        tasks_deleted_total: 0,    // Would get from metrics
        tasks_completed_total: 0,  // Would get from metrics
        search_operations_total: 0, // Would get from metrics
        export_operations_total: 0, // Would get from metrics
        errors_total: 0,           // Would get from metrics
    };
    
    // Get log statistics (placeholder values for now)
    let log_statistics = LogStatistics {
        total_entries: 0,
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

/// Get health endpoint
#[axum::debug_handler]
#[instrument(skip(state))]
async fn get_health(State(state): State<DashboardState>) -> Result<Json<HealthStatus>, StatusCode> {
    let health = state.observability.health_status();
    Ok(Json(health))
}

/// Get logs endpoint
#[axum::debug_handler]
#[instrument(skip(_state))]
async fn get_logs(State(_state): State<DashboardState>) -> Result<Json<Vec<LogEntry>>, StatusCode> {
    // Placeholder implementation - would integrate with log aggregator
    let logs = vec![
        LogEntry {
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            level: "INFO".to_string(),
            target: "things3_cli".to_string(),
            message: "Application started".to_string(),
        },
    ];
    
    Ok(Json(logs))
}

/// Search logs endpoint
#[axum::debug_handler]
#[instrument(skip(_state))]
async fn search_logs(
    State(_state): State<DashboardState>,
    Json(query): Json<LogSearchQuery>,
) -> Result<Json<Vec<LogEntry>>, StatusCode> {
    // Placeholder implementation - would integrate with log searcher
    let logs = vec![
        LogEntry {
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            level: "INFO".to_string(),
            target: "things3_cli".to_string(),
            message: format!("Search result for: {}", query.query),
        },
    ];
    
    Ok(Json(logs))
}

/// Get system info endpoint
#[axum::debug_handler]
#[instrument(skip(_state))]
async fn get_system_info(State(_state): State<DashboardState>) -> Result<Json<SystemInfo>, StatusCode> {
    let system_info = SystemInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        rust_version: std::env::var("RUSTC_SEMVER").unwrap_or_else(|_| "unknown".to_string()),
    };
    
    Ok(Json(system_info))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSearchQuery {
    pub query: String,
    pub level: Option<String>,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub version: String,
    pub rust_version: String,
}

/// Start dashboard server in background
pub async fn start_dashboard_server(
    port: u16,
    observability: Arc<ObservabilityManager>,
    database: Arc<SqlxThingsDatabase>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = DashboardServer::new(port, observability, database);
    server.start().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use things3_core::{ObservabilityConfig, ThingsConfig};
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_dashboard_metrics_creation() {
        let metrics = DashboardMetrics {
            health: HealthStatus {
                status: "healthy".to_string(),
                timestamp: chrono::Utc::now(),
                version: "1.0.0".to_string(),
                uptime: std::time::Duration::from_secs(3600),
                checks: HashMap::new(),
            },
            system_metrics: SystemMetrics {
                memory_usage: 1024.0,
                cpu_usage: 50.0,
                uptime: 3600,
                cache_hit_rate: 0.85,
                cache_size: 1024.0,
            },
            application_metrics: ApplicationMetrics {
                db_operations_total: 100,
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
        
        assert_eq!(metrics.health.status, "healthy");
        assert_eq!(metrics.system_metrics.memory_usage, 1024.0);
    }
    
    #[test]
    fn test_dashboard_server_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        
        let config = ThingsConfig::new(db_path, false);
        let database = Arc::new(ThingsDatabase::with_config(&config).unwrap());
        
        let obs_config = ObservabilityConfig::default();
        let observability = Arc::new(ObservabilityManager::new(obs_config).unwrap());
        
        let server = DashboardServer::new(8080, observability, database);
        assert_eq!(server.port, 8080);
    }
}
