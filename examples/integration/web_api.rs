//! Web API Example with Axum
//!
//! This example demonstrates how to build a REST API on top of Things 3
//! using the Axum web framework. This is useful for:
//! - Building web dashboards
//! - Creating mobile app backends
//! - Integrating with other systems via HTTP
//! - Providing team access to Things 3 data
//!
//! Run this example with:
//! ```bash
//! cargo run --example web_api
//! ```
//!
//! Then test with:
//! ```bash
//! curl http://localhost:3000/health
//! curl http://localhost:3000/api/inbox
//! curl http://localhost:3000/api/projects
//! ```

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use things3_core::{ThingsDatabase, ThingsConfig, Task, Project};
use tower_http::cors::CorsLayer;

/// Application state shared across handlers
#[derive(Clone)]
struct AppState {
    db: Arc<ThingsDatabase>,
}

/// Query parameters for list endpoints
#[derive(Deserialize)]
struct ListQuery {
    limit: Option<usize>,
    offset: Option<usize>,
}

/// Query parameters for search
#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    limit: Option<usize>,
}

/// Request body for creating tasks
#[derive(Deserialize)]
struct CreateTaskRequest {
    title: String,
    notes: Option<String>,
    project_uuid: Option<String>,
}

/// API response wrapper
#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize database
    let config = ThingsConfig::from_env();
    let db = ThingsDatabase::new(&config.database_path).await?;
    let state = AppState { db: Arc::new(db) };

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        
        // API routes
        .route("/api/inbox", get(get_inbox))
        .route("/api/today", get(get_today))
        .route("/api/projects", get(get_projects))
        .route("/api/projects/:id", get(get_project))
        .route("/api/areas", get(get_areas))
        .route("/api/search", get(search_tasks))
        .route("/api/tasks", post(create_task))
        .route("/api/tasks/:id", get(get_task))
        
        // Stats endpoint
        .route("/api/stats", get(get_stats))
        
        // Add CORS middleware
        .layer(CorsLayer::permissive())
        
        // Add state
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("🚀 Web API server running on http://127.0.0.1:3000");
    println!("\nAvailable endpoints:");
    println!("  GET  /health           - Health check");
    println!("  GET  /api/inbox        - Get inbox tasks");
    println!("  GET  /api/today        - Get today's tasks");
    println!("  GET  /api/projects     - Get all projects");
    println!("  GET  /api/projects/:id - Get specific project");
    println!("  GET  /api/areas        - Get all areas");
    println!("  GET  /api/search?q=... - Search tasks");
    println!("  GET  /api/stats        - Get statistics");
    println!("  POST /api/tasks        - Create a task");
    println!("  GET  /api/tasks/:id    - Get specific task");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(ApiResponse::success(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "things3-api"
    })))
}

/// Get inbox tasks
async fn get_inbox(
    State(state): State<AppState>,
    Query(params): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<Task>>>, StatusCode> {
    match state.db.get_inbox(params.limit).await {
        Ok(tasks) => {
            let tasks = apply_pagination(tasks, params.offset);
            Ok(Json(ApiResponse::success(tasks)))
        }
        Err(e) => {
            tracing::error!("Failed to get inbox: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get today's tasks
async fn get_today(
    State(state): State<AppState>,
    Query(params): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<Task>>>, StatusCode> {
    match state.db.get_all_tasks().await {
        Ok(tasks) => {
            // Filter for today (simplified - in production, use proper today filtering)
            let tasks = apply_pagination(tasks, params.offset);
            Ok(Json(ApiResponse::success(tasks)))
        }
        Err(e) => {
            tracing::error!("Failed to get today tasks: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get all projects
async fn get_projects(
    State(state): State<AppState>,
    Query(params): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<Project>>>, StatusCode> {
    match state.db.get_projects(params.limit).await {
        Ok(projects) => {
            let projects = apply_pagination(projects, params.offset);
            Ok(Json(ApiResponse::success(projects)))
        }
        Err(e) => {
            tracing::error!("Failed to get projects: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get specific project
async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Project>>, StatusCode> {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    match state.db.get_project_by_uuid(&uuid).await {
        Ok(Some(project)) => Ok(Json(ApiResponse::success(project))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get all areas
async fn get_areas(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<things3_core::Area>>>, StatusCode> {
    match state.db.get_areas().await {
        Ok(areas) => Ok(Json(ApiResponse::success(areas))),
        Err(e) => {
            tracing::error!("Failed to get areas: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Search tasks
async fn search_tasks(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<ApiResponse<Vec<Task>>>, StatusCode> {
    match state.db.search_tasks(&params.q).await {
        Ok(tasks) => {
            let tasks = if let Some(limit) = params.limit {
                tasks.into_iter().take(limit).collect()
            } else {
                tasks
            };
            Ok(Json(ApiResponse::success(tasks)))
        }
        Err(e) => {
            tracing::error!("Failed to search tasks: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get specific task
async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Task>>, StatusCode> {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    match state.db.get_task_by_uuid(&uuid).await {
        Ok(Some(task)) => Ok(Json(ApiResponse::success(task))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Create a new task
async fn create_task(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let project_uuid = if let Some(uuid_str) = payload.project_uuid {
        match uuid::Uuid::parse_str(&uuid_str) {
            Ok(uuid) => Some(uuid),
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        }
    } else {
        None
    };

    let request = things3_core::CreateTaskRequest {
        title: payload.title,
        task_type: None,
        notes: payload.notes,
        start_date: None,
        deadline: None,
        project_uuid,
        area_uuid: None,
        parent_uuid: None,
        tags: None,
        status: None,
    };

    match state.db.create_task(request).await {
        Ok(task_uuid) => Ok(Json(ApiResponse::success(task_uuid.to_string()))),
        Err(e) => {
            tracing::error!("Failed to create task: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get statistics
async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let tasks = state.db.get_all_tasks().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let projects = state.db.get_all_projects().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let areas = state.db.get_areas().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let completed_tasks = tasks.iter().filter(|t| {
        t.status == things3_core::TaskStatus::Completed
    }).count();

    let active_tasks = tasks.iter().filter(|t| {
        t.status == things3_core::TaskStatus::Incomplete
    }).count();

    let stats = serde_json::json!({
        "tasks": {
            "total": tasks.len(),
            "active": active_tasks,
            "completed": completed_tasks,
        },
        "projects": {
            "total": projects.len(),
            "active": projects.iter().filter(|p| p.status == things3_core::TaskStatus::Incomplete).count(),
        },
        "areas": {
            "total": areas.len(),
        },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    Ok(Json(ApiResponse::success(stats)))
}

/// Helper: Apply pagination
fn apply_pagination<T>(items: Vec<T>, offset: Option<usize>) -> Vec<T> {
    if let Some(offset) = offset {
        items.into_iter().skip(offset).collect()
    } else {
        items
    }
}

/*
 * Production Enhancements:
 * 
 * 1. Authentication: Add JWT or API key authentication
 * 2. Rate Limiting: Implement rate limiting per client
 * 3. Caching: Add response caching with Redis
 * 4. Pagination: Implement cursor-based pagination
 * 5. Filtering: Add query parameter filtering
 * 6. Validation: Add request validation middleware
 * 7. Error Handling: Implement custom error types
 * 8. Logging: Add structured logging with tracing
 * 9. Metrics: Add Prometheus metrics
 * 10. OpenAPI: Generate OpenAPI/Swagger docs
 */

/*
 * Example client usage:
 * 
 * // Get inbox
 * curl http://localhost:3000/api/inbox?limit=5
 * 
 * // Search tasks
 * curl "http://localhost:3000/api/search?q=meeting&limit=10"
 * 
 * // Create task
 * curl -X POST http://localhost:3000/api/tasks \
 *   -H "Content-Type: application/json" \
 *   -d '{"title": "New task", "notes": "Task notes"}'
 * 
 * // Get stats
 * curl http://localhost:3000/api/stats
 */

