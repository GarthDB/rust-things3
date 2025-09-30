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

    #[test]
    fn test_health_response_with_checks() {
        let mut checks = std::collections::HashMap::new();
        checks.insert(
            "database".to_string(),
            CheckResponse {
                status: "healthy".to_string(),
                message: Some("Connection successful".to_string()),
                duration_ms: 5,
            },
        );
        checks.insert(
            "cache".to_string(),
            CheckResponse {
                status: "unhealthy".to_string(),
                message: Some("Connection failed".to_string()),
                duration_ms: 100,
            },
        );

        let response = HealthResponse {
            status: "degraded".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            uptime: std::time::Duration::from_secs(7200),
            version: "2.0.0".to_string(),
            environment: "staging".to_string(),
            checks,
        };

        assert_eq!(response.status, "degraded");
        assert_eq!(response.version, "2.0.0");
        assert_eq!(response.environment, "staging");
        assert_eq!(response.checks.len(), 2);
        assert_eq!(response.uptime.as_secs(), 7200);
    }

    #[test]
    fn test_check_response() {
        let check = CheckResponse {
            status: "healthy".to_string(),
            message: Some("All systems operational".to_string()),
            duration_ms: 10,
        };

        assert_eq!(check.status, "healthy");
        assert_eq!(check.message, Some("All systems operational".to_string()));
        assert_eq!(check.duration_ms, 10);
    }

    #[test]
    fn test_check_response_without_message() {
        let check = CheckResponse {
            status: "unhealthy".to_string(),
            message: None,
            duration_ms: 500,
        };

        assert_eq!(check.status, "unhealthy");
        assert_eq!(check.message, None);
        assert_eq!(check.duration_ms, 500);
    }

    #[test]
    fn test_app_state_creation() {
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

        let state = AppState {
            observability: Arc::clone(&observability),
            database: Arc::clone(&database),
        };

        // Test that state can be created and cloned
        let _cloned_state = state.clone();
    }

    #[test]
    fn test_health_server_with_different_ports() {
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

        // Test with different ports
        let server1 = HealthServer::new(8080, Arc::clone(&observability), Arc::clone(&database));
        let server2 = HealthServer::new(9090, Arc::clone(&observability), Arc::clone(&database));
        let server3 = HealthServer::new(3000, Arc::clone(&observability), Arc::clone(&database));

        assert_eq!(server1.port, 8080);
        assert_eq!(server2.port, 9090);
        assert_eq!(server3.port, 3000);
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            uptime: std::time::Duration::from_secs(3600),
            version: "1.0.0".to_string(),
            environment: "test".to_string(),
            checks: std::collections::HashMap::new(),
        };

        // Test serialization
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("1.0.0"));

        // Test deserialization
        let deserialized: HealthResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, response.status);
        assert_eq!(deserialized.version, response.version);
    }

    #[test]
    fn test_check_response_serialization() {
        let check = CheckResponse {
            status: "healthy".to_string(),
            message: Some("All systems operational".to_string()),
            duration_ms: 10,
        };

        // Test serialization
        let json = serde_json::to_string(&check).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("All systems operational"));

        // Test deserialization
        let deserialized: CheckResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, check.status);
        assert_eq!(deserialized.message, check.message);
        assert_eq!(deserialized.duration_ms, check.duration_ms);
    }

    #[test]
    fn test_health_response_debug_formatting() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            uptime: std::time::Duration::from_secs(3600),
            version: "1.0.0".to_string(),
            environment: "test".to_string(),
            checks: std::collections::HashMap::new(),
        };

        let debug_str = format!("{response:?}");
        assert!(debug_str.contains("healthy"));
        assert!(debug_str.contains("1.0.0"));
    }

    #[test]
    fn test_check_response_debug_formatting() {
        let check = CheckResponse {
            status: "unhealthy".to_string(),
            message: Some("Connection failed".to_string()),
            duration_ms: 100,
        };

        let debug_str = format!("{check:?}");
        assert!(debug_str.contains("unhealthy"));
        assert!(debug_str.contains("Connection failed"));
    }

    #[test]
    fn test_health_response_clone() {
        let mut checks = std::collections::HashMap::new();
        checks.insert(
            "database".to_string(),
            CheckResponse {
                status: "healthy".to_string(),
                message: Some("OK".to_string()),
                duration_ms: 5,
            },
        );

        let response = HealthResponse {
            status: "healthy".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            uptime: std::time::Duration::from_secs(3600),
            version: "1.0.0".to_string(),
            environment: "test".to_string(),
            checks,
        };

        let cloned = response.clone();
        assert_eq!(cloned.status, response.status);
        assert_eq!(cloned.version, response.version);
        assert_eq!(cloned.checks.len(), response.checks.len());
    }

    #[test]
    fn test_check_response_clone() {
        let check = CheckResponse {
            status: "healthy".to_string(),
            message: Some("OK".to_string()),
            duration_ms: 5,
        };

        let cloned = check.clone();
        assert_eq!(cloned.status, check.status);
        assert_eq!(cloned.message, check.message);
        assert_eq!(cloned.duration_ms, check.duration_ms);
    }

    #[test]
    fn test_health_response_with_empty_checks() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            uptime: std::time::Duration::from_secs(0),
            version: "0.1.0".to_string(),
            environment: "development".to_string(),
            checks: std::collections::HashMap::new(),
        };

        assert_eq!(response.status, "healthy");
        assert_eq!(response.uptime.as_secs(), 0);
        assert_eq!(response.checks.len(), 0);
    }

    #[test]
    fn test_health_response_with_multiple_checks() {
        let mut checks = std::collections::HashMap::new();
        checks.insert(
            "database".to_string(),
            CheckResponse {
                status: "healthy".to_string(),
                message: Some("Connection OK".to_string()),
                duration_ms: 2,
            },
        );
        checks.insert(
            "redis".to_string(),
            CheckResponse {
                status: "healthy".to_string(),
                message: Some("Cache OK".to_string()),
                duration_ms: 1,
            },
        );
        checks.insert(
            "api".to_string(),
            CheckResponse {
                status: "unhealthy".to_string(),
                message: Some("Service down".to_string()),
                duration_ms: 1000,
            },
        );

        let response = HealthResponse {
            status: "degraded".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            uptime: std::time::Duration::from_secs(86400), // 24 hours
            version: "3.0.0".to_string(),
            environment: "production".to_string(),
            checks,
        };

        assert_eq!(response.status, "degraded");
        assert_eq!(response.checks.len(), 3);
        assert_eq!(response.uptime.as_secs(), 86400);
        assert_eq!(response.environment, "production");
    }
}
