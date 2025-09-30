async fn health_check(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    let health_status = state.observability.health_status();

    let response = HealthResponse {
        status: health_status.status,
        timestamp: health_status.timestamp.to_string(),
        uptime: health_status.uptime,
        version: health_status.version,
        environment: "production".to_string(),
        checks: std::collections::HashMap::new(),
    };

    Ok(Json(response))
}

async fn readiness_check(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, StatusCode> {
    let health_status = state.observability.health_status();

    let response = HealthResponse {
        status: health_status.status,
        timestamp: health_status.timestamp.to_string(),
        uptime: health_status.uptime,
        version: health_status.version,
        environment: "production".to_string(),
        checks: std::collections::HashMap::new(),
    };

    Ok(Json(response))
}

async fn liveness_check(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    let health_status = state.observability.health_status();

    let response = HealthResponse {
        status: health_status.status,
        timestamp: health_status.timestamp.to_string(),
        uptime: health_status.uptime,
        version: health_status.version,
        environment: "production".to_string(),
        checks: std::collections::HashMap::new(),
    };

    Ok(Json(response))
}

async fn metrics_endpoint(State(state): State<AppState>) -> Result<String, StatusCode> {
    let health_status = state.observability.health_status();

    let metrics = format!(
        "# HELP health_status Current health status\n\
         # TYPE health_status gauge\n\
         health_status{{status=\"{}\"}} {}\n\
         # HELP uptime_seconds Current uptime in seconds\n\
         # TYPE uptime_seconds counter\n\
         uptime_seconds {}\n",
        health_status.status,
        i32::from(health_status.status == "healthy"),
        health_status.uptime.as_secs()
    );

    Ok(metrics)
}

use axum::{extract::State, http::StatusCode, response::Json, routing::get, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use things3_core::{ObservabilityManager, ThingsDatabase};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::{info, instrument};

// Struct definitions - must come after all functions to avoid items_after_statements
/// Application state
#[derive(Clone)]
pub struct AppState {
    pub observability: Arc<ObservabilityManager>,
    pub database: Arc<ThingsDatabase>,
}

/// Health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub uptime: std::time::Duration,
    pub version: String,
    pub environment: String,
    pub checks: std::collections::HashMap<String, CheckResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResponse {
    pub status: String,
    pub message: Option<String>,
    pub duration_ms: u64,
}

impl HealthServer {
    /// Create a new health check server
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

    /// Start the health check server
    ///
    /// # Errors
    /// Returns an error if the server fails to start or bind to the port
    #[instrument(skip(self))]
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let state = AppState {
            observability: self.observability,
            database: self.database,
        };

        let app = Router::new()
            .route("/health", get(health_check))
            .route("/ready", get(readiness_check))
            .route("/live", get(liveness_check))
            .route("/metrics", get(metrics_endpoint))
            .layer(CorsLayer::permissive())
            .with_state(state);

        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
        info!("Health check server running on port {}", self.port);

        axum::serve(listener, app).await?;
        Ok(())
    }
}

/// Health check server
pub struct HealthServer {
    port: u16,
    observability: Arc<ObservabilityManager>,
    database: Arc<ThingsDatabase>,
}

/// Start the health check server
///
/// # Errors
/// Returns an error if the server fails to start or bind to the port
#[instrument(skip(observability, database))]
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
    use tempfile::NamedTempFile;

    #[test]
    fn test_health_server_creation() {
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
        let server = HealthServer::new(8080, observability, database);
        assert_eq!(server.port, 8080);
    }

    #[test]
    fn test_health_response() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            uptime: std::time::Duration::from_secs(3600),
            version: "1.0.0".to_string(),
            environment: "test".to_string(),
            checks: std::collections::HashMap::new(),
        };

        assert_eq!(response.status, "healthy");
        assert_eq!(response.version, "1.0.0");
    }
}
