//! Health check endpoints and monitoring
//!
//! This module provides health check endpoints and monitoring capabilities
//! for the Things 3 CLI application.

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use things3_core::{ObservabilityManager, ThingsDatabase};
use crate::thread_safe_db::ThreadSafeDatabase;
use tokio::net::TcpListener;
// Removed unused import
use tower_http::cors::CorsLayer;
use tracing::{info, instrument};

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub uptime: u64,
    pub checks: std::collections::HashMap<String, CheckResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResponse {
    pub status: String,
    pub message: Option<String>,
    pub duration_ms: u64,
}

/// Application state for health checks
#[derive(Clone)]
pub struct AppState {
    pub observability: Arc<ObservabilityManager>,
    pub database: ThreadSafeDatabase,
}

/// Health check server
pub struct HealthServer {
    port: u16,
    state: AppState,
}

impl HealthServer {
    /// Create a new health check server
    pub fn new(port: u16, observability: Arc<ObservabilityManager>, database: Arc<ThingsDatabase>) -> Self {
        let state = AppState {
            observability,
            database: ThreadSafeDatabase::new(database),
        };
        
        Self { port, state }
    }
    
    /// Start the health check server
    #[instrument(skip(self))]
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let port = self.port;
        let app = self.create_app();
        
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        info!("Health check server started on port {}", port);
        
        axum::serve(listener, app).await?;
        Ok(())
    }
    
    /// Create the Axum application
    fn create_app(self) -> Router {
        Router::new()
            .route("/health", get(health_check))
            .route("/health/ready", get(readiness_check))
            .route("/health/live", get(liveness_check))
            .route("/metrics", get(metrics_endpoint))
            .with_state(self.state)
            .layer(CorsLayer::permissive())
    }
}

/// Health check endpoint
#[axum::debug_handler]
#[instrument(skip(state))]
async fn health_check(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    let health_status = state.observability.health_status();
    
    let response = HealthResponse {
        status: health_status.status,
        timestamp: health_status.timestamp,
        version: health_status.version,
        uptime: health_status.uptime.as_secs(),
        checks: health_status
            .checks
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    CheckResponse {
                        status: v.status,
                        message: v.message,
                        duration_ms: v.duration_ms,
                    },
                )
            })
            .collect(),
    };
    
    Ok(Json(response))
}

/// Readiness check endpoint
#[axum::debug_handler]
#[instrument(skip(state))]
async fn readiness_check(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    // Check if the application is ready to serve requests
    let mut checks = std::collections::HashMap::new();
    
    // Database readiness check
    let db_start = std::time::Instant::now();
    let db_ready = state.database.is_connected().await;
    let db_duration = db_start.elapsed();
    
    checks.insert("database".to_string(), CheckResponse {
        status: if db_ready { "ready".to_string() } else { "not_ready".to_string() },
        message: Some(if db_ready { "Database is ready" } else { "Database is not ready" }.to_string()),
        duration_ms: db_duration.as_millis() as u64,
    });
    
    let health_status = state.observability.health_status();
    let overall_status = if db_ready { "ready" } else { "not_ready" };
    
    let response = HealthResponse {
        status: overall_status.to_string(),
        timestamp: health_status.timestamp,
        version: health_status.version,
        uptime: health_status.uptime.as_secs(),
        checks,
    };
    
    Ok(Json(response))
}

/// Liveness check endpoint
#[axum::debug_handler]
#[instrument(skip(state))]
async fn liveness_check(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    // Check if the application is alive (basic health check)
    let health_status = state.observability.health_status();
    
    let response = HealthResponse {
        status: "alive".to_string(),
        timestamp: health_status.timestamp,
        version: health_status.version,
        uptime: health_status.uptime.as_secs(),
        checks: std::collections::HashMap::new(),
    };
    
    Ok(Json(response))
}

/// Metrics endpoint
#[axum::debug_handler]
#[instrument(skip(state))]
async fn metrics_endpoint(State(state): State<AppState>) -> Result<String, StatusCode> {
    // This would typically return Prometheus-formatted metrics
    // For now, return a simple JSON response
    let health_status = state.observability.health_status();
    
    let metrics = format!(
        "# HELP things3_uptime_seconds Total uptime in seconds\n\
         # TYPE things3_uptime_seconds counter\n\
         things3_uptime_seconds {{}} {}\n\
         \n\
         # HELP things3_version_info Version information\n\
         # TYPE things3_version_info gauge\n\
         things3_version_info {{version=\"{}\"}} 1\n",
        health_status.uptime.as_secs(),
        health_status.version
    );
    
    Ok(metrics)
}

/// Start health check server in background
pub async fn start_health_server(
    port: u16,
    observability: Arc<ObservabilityManager>,
    database: Arc<ThingsDatabase>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = HealthServer::new(port, observability, database);
    server.start().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use things3_core::{ObservabilityConfig, ThingsConfig};
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_health_response_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        
        let config = ThingsConfig::new(db_path, false);
        let database = Arc::new(ThingsDatabase::with_config(&config).unwrap());
        
        let obs_config = ObservabilityConfig::default();
        let observability = Arc::new(ObservabilityManager::new(obs_config).unwrap());
        
        let state = AppState {
            observability,
            database,
        };
        
        let response = health_check(State(state)).await.unwrap();
        assert_eq!(response.status, "healthy");
    }
    
    #[test]
    fn test_health_server_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        
        let config = ThingsConfig::new(db_path, false);
        let database = Arc::new(ThingsDatabase::with_config(&config).unwrap());
        
        let obs_config = ObservabilityConfig::default();
        let observability = Arc::new(ObservabilityManager::new(obs_config).unwrap());
        
        let server = HealthServer::new(8080, observability, database);
        assert_eq!(server.port, 8080);
    }
}
