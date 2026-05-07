//! MCP (Model Context Protocol) server implementation for Things 3 integration

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
#[cfg(target_os = "macos")]
use things3_core::AppleScriptBackend;
use things3_core::{
    BackupManager, DataExporter, McpServerConfig, MutationBackend, PerformanceMonitor, SqlxBackend,
    ThingsCache, ThingsConfig, ThingsDatabase, ThingsError,
};
use thiserror::Error;
use tokio::sync::Mutex;

pub mod io_wrapper;
pub mod middleware;
// pub mod performance_tests; // Temporarily disabled due to API changes
pub mod test_harness;
mod tools;

use io_wrapper::{McpIo, StdIo};
use middleware::{MiddlewareChain, MiddlewareConfig};

/// MCP-specific error types for better error handling and user experience
#[derive(Error, Debug)]
pub enum McpError {
    #[error("Tool not found: {tool_name}")]
    ToolNotFound { tool_name: String },

    #[error("Resource not found: {uri}")]
    ResourceNotFound { uri: String },

    #[error("Prompt not found: {prompt_name}")]
    PromptNotFound { prompt_name: String },

    #[error("Invalid parameter: {parameter_name} - {message}")]
    InvalidParameter {
        parameter_name: String,
        message: String,
    },

    #[error("Missing required parameter: {parameter_name}")]
    MissingParameter { parameter_name: String },

    #[error("Invalid format: {format} - supported formats: {supported}")]
    InvalidFormat { format: String, supported: String },

    #[error("Invalid data type: {data_type} - supported types: {supported}")]
    InvalidDataType {
        data_type: String,
        supported: String,
    },

    #[error("Database operation failed: {operation}")]
    DatabaseOperationFailed {
        operation: String,
        source: ThingsError,
    },

    #[error("Backup operation failed: {operation}")]
    BackupOperationFailed {
        operation: String,
        source: ThingsError,
    },

    #[error("Export operation failed: {operation}")]
    ExportOperationFailed {
        operation: String,
        source: ThingsError,
    },

    #[error("Performance monitoring failed: {operation}")]
    PerformanceMonitoringFailed {
        operation: String,
        source: ThingsError,
    },

    #[error("Cache operation failed: {operation}")]
    CacheOperationFailed {
        operation: String,
        source: ThingsError,
    },

    #[error("Serialization failed: {operation}")]
    SerializationFailed {
        operation: String,
        source: serde_json::Error,
    },

    #[error("IO operation failed: {operation}")]
    IoOperationFailed {
        operation: String,
        source: std::io::Error,
    },

    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },

    #[error("Validation error: {message}")]
    ValidationError { message: String },

    #[error("Internal error: {message}")]
    InternalError { message: String },
}

impl McpError {
    /// Create a tool not found error
    pub fn tool_not_found(tool_name: impl Into<String>) -> Self {
        Self::ToolNotFound {
            tool_name: tool_name.into(),
        }
    }

    /// Create a resource not found error
    pub fn resource_not_found(uri: impl Into<String>) -> Self {
        Self::ResourceNotFound { uri: uri.into() }
    }

    /// Create a prompt not found error
    pub fn prompt_not_found(prompt_name: impl Into<String>) -> Self {
        Self::PromptNotFound {
            prompt_name: prompt_name.into(),
        }
    }

    /// Create an invalid parameter error
    pub fn invalid_parameter(
        parameter_name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::InvalidParameter {
            parameter_name: parameter_name.into(),
            message: message.into(),
        }
    }

    /// Create a missing parameter error
    pub fn missing_parameter(parameter_name: impl Into<String>) -> Self {
        Self::MissingParameter {
            parameter_name: parameter_name.into(),
        }
    }

    /// Create an invalid format error
    pub fn invalid_format(format: impl Into<String>, supported: impl Into<String>) -> Self {
        Self::InvalidFormat {
            format: format.into(),
            supported: supported.into(),
        }
    }

    /// Create an invalid data type error
    pub fn invalid_data_type(data_type: impl Into<String>, supported: impl Into<String>) -> Self {
        Self::InvalidDataType {
            data_type: data_type.into(),
            supported: supported.into(),
        }
    }

    /// Create a database operation failed error
    pub fn database_operation_failed(operation: impl Into<String>, source: ThingsError) -> Self {
        Self::DatabaseOperationFailed {
            operation: operation.into(),
            source,
        }
    }

    /// Create a backup operation failed error
    pub fn backup_operation_failed(operation: impl Into<String>, source: ThingsError) -> Self {
        Self::BackupOperationFailed {
            operation: operation.into(),
            source,
        }
    }

    /// Create an export operation failed error
    pub fn export_operation_failed(operation: impl Into<String>, source: ThingsError) -> Self {
        Self::ExportOperationFailed {
            operation: operation.into(),
            source,
        }
    }

    /// Create a performance monitoring failed error
    pub fn performance_monitoring_failed(
        operation: impl Into<String>,
        source: ThingsError,
    ) -> Self {
        Self::PerformanceMonitoringFailed {
            operation: operation.into(),
            source,
        }
    }

    /// Create a cache operation failed error
    pub fn cache_operation_failed(operation: impl Into<String>, source: ThingsError) -> Self {
        Self::CacheOperationFailed {
            operation: operation.into(),
            source,
        }
    }

    /// Create a serialization failed error
    pub fn serialization_failed(operation: impl Into<String>, source: serde_json::Error) -> Self {
        Self::SerializationFailed {
            operation: operation.into(),
            source,
        }
    }

    /// Create an IO operation failed error
    pub fn io_operation_failed(operation: impl Into<String>, source: std::io::Error) -> Self {
        Self::IoOperationFailed {
            operation: operation.into(),
            source,
        }
    }

    /// Create a configuration error
    pub fn configuration_error(message: impl Into<String>) -> Self {
        Self::ConfigurationError {
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::ValidationError {
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }

    /// Convert error to MCP call result
    #[must_use]
    pub fn to_call_result(self) -> CallToolResult {
        let error_message = match &self {
            McpError::ToolNotFound { tool_name } => {
                format!("Tool '{tool_name}' not found. Available tools can be listed using the list_tools method.")
            }
            McpError::ResourceNotFound { uri } => {
                format!("Resource '{uri}' not found. Available resources can be listed using the list_resources method.")
            }
            McpError::PromptNotFound { prompt_name } => {
                format!("Prompt '{prompt_name}' not found. Available prompts can be listed using the list_prompts method.")
            }
            McpError::InvalidParameter {
                parameter_name,
                message,
            } => {
                format!("Invalid parameter '{parameter_name}': {message}. Please check the parameter format and try again.")
            }
            McpError::MissingParameter { parameter_name } => {
                format!("Missing required parameter '{parameter_name}'. Please provide this parameter and try again.")
            }
            McpError::InvalidFormat { format, supported } => {
                format!("Invalid format '{format}'. Supported formats: {supported}. Please use one of the supported formats.")
            }
            McpError::InvalidDataType {
                data_type,
                supported,
            } => {
                format!("Invalid data type '{data_type}'. Supported types: {supported}. Please use one of the supported types.")
            }
            McpError::DatabaseOperationFailed { operation, source } => {
                format!("Database operation '{operation}' failed: {source}. Please check your database connection and try again.")
            }
            McpError::BackupOperationFailed { operation, source } => {
                format!("Backup operation '{operation}' failed: {source}. Please check backup permissions and try again.")
            }
            McpError::ExportOperationFailed { operation, source } => {
                format!("Export operation '{operation}' failed: {source}. Please check export parameters and try again.")
            }
            McpError::PerformanceMonitoringFailed { operation, source } => {
                format!("Performance monitoring '{operation}' failed: {source}. Please try again later.")
            }
            McpError::CacheOperationFailed { operation, source } => {
                format!("Cache operation '{operation}' failed: {source}. Please try again later.")
            }
            McpError::SerializationFailed { operation, source } => {
                format!("Serialization '{operation}' failed: {source}. Please check data format and try again.")
            }
            McpError::IoOperationFailed { operation, source } => {
                format!("IO operation '{operation}' failed: {source}. Please check file permissions and try again.")
            }
            McpError::ConfigurationError { message } => {
                format!("Configuration error: {message}. Please check your configuration and try again.")
            }
            McpError::ValidationError { message } => {
                format!("Validation error: {message}. Please check your input and try again.")
            }
            McpError::InternalError { message } => {
                format!("Internal error: {message}. Please try again later or contact support if the issue persists.")
            }
        };

        CallToolResult {
            content: vec![Content::Text {
                text: error_message,
            }],
            is_error: true,
        }
    }

    /// Convert error to MCP prompt result
    #[must_use]
    pub fn to_prompt_result(self) -> GetPromptResult {
        let error_message = match &self {
            McpError::PromptNotFound { prompt_name } => {
                format!("Prompt '{prompt_name}' not found. Available prompts can be listed using the list_prompts method.")
            }
            McpError::InvalidParameter {
                parameter_name,
                message,
            } => {
                format!("Invalid parameter '{parameter_name}': {message}. Please check the parameter format and try again.")
            }
            McpError::MissingParameter { parameter_name } => {
                format!("Missing required parameter '{parameter_name}'. Please provide this parameter and try again.")
            }
            McpError::DatabaseOperationFailed { operation, source } => {
                format!("Database operation '{operation}' failed: {source}. Please check your database connection and try again.")
            }
            McpError::SerializationFailed { operation, source } => {
                format!("Serialization '{operation}' failed: {source}. Please check data format and try again.")
            }
            McpError::ValidationError { message } => {
                format!("Validation error: {message}. Please check your input and try again.")
            }
            McpError::InternalError { message } => {
                format!("Internal error: {message}. Please try again later or contact support if the issue persists.")
            }
            _ => {
                format!("Error: {self}. Please try again later.")
            }
        };

        GetPromptResult {
            content: vec![Content::Text {
                text: error_message,
            }],
            is_error: true,
        }
    }

    /// Convert error to MCP resource result
    #[must_use]
    pub fn to_resource_result(self) -> ReadResourceResult {
        let error_message = match &self {
            McpError::ResourceNotFound { uri } => {
                format!("Resource '{uri}' not found. Available resources can be listed using the list_resources method.")
            }
            McpError::DatabaseOperationFailed { operation, source } => {
                format!("Database operation '{operation}' failed: {source}. Please check your database connection and try again.")
            }
            McpError::SerializationFailed { operation, source } => {
                format!("Serialization '{operation}' failed: {source}. Please check data format and try again.")
            }
            McpError::InternalError { message } => {
                format!("Internal error: {message}. Please try again later or contact support if the issue persists.")
            }
            _ => {
                format!("Error: {self}. Please try again later.")
            }
        };

        ReadResourceResult {
            contents: vec![Content::Text {
                text: error_message,
            }],
        }
    }
}

/// Result type alias for MCP operations
pub type McpResult<T> = std::result::Result<T, McpError>;

/// From trait implementations for common error types
impl From<ThingsError> for McpError {
    fn from(error: ThingsError) -> Self {
        match error {
            ThingsError::Database(e) => {
                McpError::database_operation_failed("database operation", ThingsError::Database(e))
            }
            ThingsError::Serialization(e) => McpError::serialization_failed("serialization", e),
            ThingsError::Io(e) => McpError::io_operation_failed("io operation", e),
            ThingsError::DatabaseNotFound { path } => {
                McpError::configuration_error(format!("Database not found at: {path}"))
            }
            ThingsError::InvalidUuid { uuid } => {
                McpError::validation_error(format!("Invalid UUID format: {uuid}"))
            }
            ThingsError::InvalidDate { date } => {
                McpError::validation_error(format!("Invalid date format: {date}"))
            }
            ThingsError::TaskNotFound { uuid } => {
                McpError::validation_error(format!("Task not found: {uuid}"))
            }
            ThingsError::ProjectNotFound { uuid } => {
                McpError::validation_error(format!("Project not found: {uuid}"))
            }
            ThingsError::AreaNotFound { uuid } => {
                McpError::validation_error(format!("Area not found: {uuid}"))
            }
            ThingsError::Validation { message } => McpError::validation_error(message),
            ThingsError::InvalidCursor(message) => {
                McpError::validation_error(format!("Invalid cursor: {message}"))
            }
            ThingsError::Configuration { message } => McpError::configuration_error(message),
            ThingsError::DateValidation(e) => {
                McpError::validation_error(format!("Date validation failed: {e}"))
            }
            ThingsError::DateConversion(e) => {
                McpError::validation_error(format!("Date conversion failed: {e}"))
            }
            ThingsError::AppleScript { message } => {
                McpError::internal_error(format!("AppleScript automation failed: {message}"))
            }
            ThingsError::Unknown { message } => McpError::internal_error(message),
        }
    }
}

impl From<serde_json::Error> for McpError {
    fn from(error: serde_json::Error) -> Self {
        McpError::serialization_failed("json serialization", error)
    }
}

impl From<std::io::Error> for McpError {
    fn from(error: std::io::Error) -> Self {
        McpError::io_operation_failed("file operation", error)
    }
}

/// Simplified MCP types for our implementation
#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolRequest {
    pub name: String,
    pub arguments: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<Content>,
    #[serde(rename = "isError", skip_serializing_if = "std::ops::Not::not")]
    pub is_error: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Content {
    Text { text: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
}

/// MCP Resource for data exposure
#[derive(Debug, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListResourcesResult {
    pub resources: Vec<Resource>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadResourceRequest {
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadResourceResult {
    pub contents: Vec<Content>,
}

/// Describes an argument that an MCP prompt can accept.
#[derive(Debug, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Omitted from JSON when false; `true` serializes as `"required": true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,
}

/// MCP Prompt for reusable templates
#[derive(Debug, Serialize, Deserialize)]
pub struct Prompt {
    pub name: String,
    pub description: String,
    pub arguments: Vec<PromptArgument>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListPromptsResult {
    pub prompts: Vec<Prompt>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPromptRequest {
    pub name: String,
    pub arguments: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPromptResult {
    pub content: Vec<Content>,
    pub is_error: bool,
}

/// MCP server for Things 3 integration
pub struct ThingsMcpServer {
    #[allow(dead_code)]
    pub db: Arc<ThingsDatabase>,
    /// Mutation backend used for all write operations.
    ///
    /// On macOS the default is `AppleScriptBackend` (CulturedCode-supported);
    /// `--unsafe-direct-db` falls back to `SqlxBackend`. On non-macOS the
    /// default is always `SqlxBackend` (no Things 3 install to corrupt).
    mutations: Arc<dyn MutationBackend>,
    /// Whether the user opted into the deprecated direct-DB path. Required to
    /// run `restore_database` — see `handle_restore_database` for the gate.
    unsafe_direct_db: bool,
    /// Stub-able predicate for "is Things 3 currently running?". Defaults to
    /// `is_things3_running`; tests inject a constant function instead of
    /// shelling out to `pgrep`.
    process_check: fn() -> bool,
    #[allow(dead_code)]
    cache: Arc<Mutex<ThingsCache>>,
    #[allow(dead_code)]
    performance_monitor: Arc<Mutex<PerformanceMonitor>>,
    #[allow(dead_code)]
    exporter: DataExporter,
    #[allow(dead_code)]
    backup_manager: Arc<Mutex<BackupManager>>,
    /// Middleware chain for cross-cutting concerns
    middleware_chain: MiddlewareChain,
}

/// Build a JSON-RPC error response from a `ThingsError` produced inside the
/// request loop.
///
/// This is the connection-survival path: if `handle_jsonrpc_request` returns
/// `Err`, we convert it to a structured JSON-RPC error response instead of
/// propagating with `?` (which would terminate the server loop and drop the
/// connection — see issue #148).
///
/// Returns `None` when the request is a JSON-RPC notification (no `id` field):
/// the spec forbids responses to notifications, so we silently drop the error.
///
/// Tool/resource/prompt errors are NOT routed here — those go through the
/// `*_with_fallback` variants and surface as `isError: true` envelopes inside
/// the result. Only protocol-level failures (missing method, malformed params)
/// reach this helper.
fn build_jsonrpc_error_response(
    id: Option<serde_json::Value>,
    err: &things3_core::ThingsError,
) -> Option<serde_json::Value> {
    use serde_json::json;

    // Notifications (no `id`) must not receive a response per JSON-RPC 2.0.
    let id = id?;

    // -32601 (Method Not Found) is the most precise fit for the protocol-level
    // failures that reach here: unknown method names, missing dispatch targets.
    // Use -32603 (Internal Error) only if we ever distinguish structural/parse
    // errors, which are caught earlier and already map to -32700 / -32600.
    Some(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": -32601,
            "message": err.to_string()
        }
    }))
}

#[allow(dead_code)]
/// Start the MCP server
///
/// # Errors
/// Returns an error if the server fails to start
pub async fn start_mcp_server(
    db: Arc<ThingsDatabase>,
    config: ThingsConfig,
    unsafe_direct_db: bool,
) -> things3_core::Result<()> {
    let io = StdIo::new();
    start_mcp_server_generic(db, config, io, unsafe_direct_db).await
}

/// Generic MCP server implementation that works with any I/O implementation
///
/// This function is generic over the I/O layer, allowing it to work with both
/// production stdin/stdout (via `StdIo`) and test mocks (via `MockIo`).
pub async fn start_mcp_server_generic<I: McpIo>(
    db: Arc<ThingsDatabase>,
    config: ThingsConfig,
    mut io: I,
    unsafe_direct_db: bool,
) -> things3_core::Result<()> {
    let server = Arc::new(tokio::sync::Mutex::new(ThingsMcpServer::new(
        db,
        config,
        unsafe_direct_db,
    )));

    // Read JSON-RPC requests line by line
    loop {
        // Read a line from input
        let line = io.read_line().await.map_err(|e| {
            things3_core::ThingsError::unknown(format!("Failed to read from input: {}", e))
        })?;

        // EOF reached
        let Some(line) = line else {
            break;
        };

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // Parse JSON-RPC request
        let request: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
            things3_core::ThingsError::unknown(format!("Failed to parse JSON-RPC request: {}", e))
        })?;

        // Handle the request. If the handler errors we MUST NOT propagate with
        // `?` — that terminates the loop and drops the MCP connection (#148).
        // Convert handler errors into JSON-RPC error responses instead.
        // Extract `id` before consuming `request` so we can use it in the error
        // path without cloning the entire request value on every hot-path call.
        let request_id = request.get("id").cloned();
        let server_clone = Arc::clone(&server);
        let response_opt = {
            let server = server_clone.lock().await;
            match server.handle_jsonrpc_request(request).await {
                Ok(opt) => opt,
                Err(e) => build_jsonrpc_error_response(request_id, &e),
            }
        };

        // Only write response if this is a request (not a notification)
        if let Some(response) = response_opt {
            let response_str = serde_json::to_string(&response).map_err(|e| {
                things3_core::ThingsError::unknown(format!("Failed to serialize response: {}", e))
            })?;

            io.write_line(&response_str).await.map_err(|e| {
                things3_core::ThingsError::unknown(format!("Failed to write response: {}", e))
            })?;

            io.flush().await.map_err(|e| {
                things3_core::ThingsError::unknown(format!("Failed to flush output: {}", e))
            })?;
        }
        // Notifications don't require a response, so we silently continue
    }

    Ok(())
}

/// Start the MCP server with comprehensive configuration
///
/// # Arguments
/// * `db` - Database connection
/// * `mcp_config` - MCP server configuration
///
/// # Errors
/// Returns an error if the server fails to start
pub async fn start_mcp_server_with_config(
    db: Arc<ThingsDatabase>,
    mcp_config: McpServerConfig,
    unsafe_direct_db: bool,
) -> things3_core::Result<()> {
    let io = StdIo::new();
    start_mcp_server_with_config_generic(db, mcp_config, io, unsafe_direct_db).await
}

/// Generic MCP server with config implementation that works with any I/O implementation
pub async fn start_mcp_server_with_config_generic<I: McpIo>(
    db: Arc<ThingsDatabase>,
    mcp_config: McpServerConfig,
    mut io: I,
    unsafe_direct_db: bool,
) -> things3_core::Result<()> {
    // Convert McpServerConfig to ThingsConfig for backward compatibility
    let things_config = ThingsConfig::new(
        mcp_config.database.path.clone(),
        mcp_config.database.fallback_to_default,
    );

    let server = Arc::new(tokio::sync::Mutex::new(
        ThingsMcpServer::new_with_mcp_config(db, things_config, mcp_config, unsafe_direct_db),
    ));

    // Read JSON-RPC requests line by line
    loop {
        // Read a line from input
        let line = io.read_line().await.map_err(|e| {
            things3_core::ThingsError::unknown(format!("Failed to read from input: {}", e))
        })?;

        // EOF reached
        let Some(line) = line else {
            break;
        };

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // Parse JSON-RPC request
        let request: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
            things3_core::ThingsError::unknown(format!("Failed to parse JSON-RPC request: {}", e))
        })?;

        // Handle the request. See note in `start_mcp_server_generic` — handler
        // errors are converted to JSON-RPC error responses instead of being
        // propagated with `?`, which would terminate the loop (#148).
        let request_id = request.get("id").cloned();
        let server_clone = Arc::clone(&server);
        let response_opt = {
            let server = server_clone.lock().await;
            match server.handle_jsonrpc_request(request).await {
                Ok(opt) => opt,
                Err(e) => build_jsonrpc_error_response(request_id, &e),
            }
        };

        // Only write response if this is a request (not a notification)
        if let Some(response) = response_opt {
            let response_str = serde_json::to_string(&response).map_err(|e| {
                things3_core::ThingsError::unknown(format!("Failed to serialize response: {}", e))
            })?;

            io.write_line(&response_str).await.map_err(|e| {
                things3_core::ThingsError::unknown(format!("Failed to write response: {}", e))
            })?;

            io.flush().await.map_err(|e| {
                things3_core::ThingsError::unknown(format!("Failed to flush output: {}", e))
            })?;
        }
        // Notifications don't require a response, so we silently continue
    }

    Ok(())
}

/// Pick the default `MutationBackend` for a server invocation.
///
/// On macOS the safe default is `AppleScriptBackend`. `--unsafe-direct-db` /
/// `THINGS_UNSAFE_DIRECT_DB=1` falls back to the deprecated `SqlxBackend`.
/// On non-macOS the default is always `SqlxBackend` — there's no Things 3
/// install to corrupt, and `AppleScriptBackend` is platform-gated.
fn select_default_backend(
    db: Arc<ThingsDatabase>,
    unsafe_direct_db: bool,
) -> Arc<dyn MutationBackend> {
    #[cfg(target_os = "macos")]
    {
        if unsafe_direct_db {
            Arc::new(SqlxBackend::new(db))
        } else {
            Arc::new(AppleScriptBackend::new(db))
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = unsafe_direct_db;
        Arc::new(SqlxBackend::new(db))
    }
}

/// Returns `true` if Things 3 is currently running (macOS only).
///
/// Used as a precondition for `restore_database` — overwriting the live
/// SQLite file under a running Things 3 process is the highest-corruption
/// scenario CulturedCode warns about. On non-macOS we always return `false`
/// because there is no Things 3 process to detect.
fn is_things3_running() -> bool {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("pgrep")
            .args(["-x", "Things3"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

impl ThingsMcpServer {
    #[must_use]
    pub fn new(db: Arc<ThingsDatabase>, config: ThingsConfig, unsafe_direct_db: bool) -> Self {
        let mutations = select_default_backend(Arc::clone(&db), unsafe_direct_db);
        let mut server = Self::with_mutation_backend(db, mutations, config);
        server.unsafe_direct_db = unsafe_direct_db;
        server
    }

    /// Create a new MCP server with a caller-provided mutation backend.
    ///
    /// Use this to inject `AppleScriptBackend` (issue #124) or a test double
    /// without taking the default `SqlxBackend`. The `unsafe_direct_db` flag
    /// defaults to `false`; callers gating `restore_database` should use
    /// [`Self::new`] or set it after construction via the test-only
    /// `set_unsafe_direct_db` helper.
    #[must_use]
    pub fn with_mutation_backend(
        db: Arc<ThingsDatabase>,
        mutations: Arc<dyn MutationBackend>,
        config: ThingsConfig,
    ) -> Self {
        let cache = ThingsCache::new_default();
        let performance_monitor = PerformanceMonitor::new_default();
        let exporter = DataExporter::new_default();
        let backup_manager = BackupManager::new(config);
        // Use silent middleware config for MCP mode (no logging to stdout)
        let mut middleware_config = MiddlewareConfig::default();
        middleware_config.logging.enabled = false; // Disable logging to prevent stdout interference
        let middleware_chain = middleware_config.build_chain();

        Self {
            db,
            mutations,
            unsafe_direct_db: false,
            process_check: is_things3_running,
            cache: Arc::new(Mutex::new(cache)),
            performance_monitor: Arc::new(Mutex::new(performance_monitor)),
            exporter,
            backup_manager: Arc::new(Mutex::new(backup_manager)),
            middleware_chain,
        }
    }

    /// Create a new MCP server with custom middleware configuration
    #[must_use]
    pub fn with_middleware_config(
        db: ThingsDatabase,
        config: ThingsConfig,
        middleware_config: MiddlewareConfig,
        unsafe_direct_db: bool,
    ) -> Self {
        let db = Arc::new(db);
        let mutations = select_default_backend(Arc::clone(&db), unsafe_direct_db);
        let cache = ThingsCache::new_default();
        let performance_monitor = PerformanceMonitor::new_default();
        let exporter = DataExporter::new_default();
        let backup_manager = BackupManager::new(config);
        let middleware_chain = middleware_config.build_chain();

        Self {
            db,
            mutations,
            unsafe_direct_db,
            process_check: is_things3_running,
            cache: Arc::new(Mutex::new(cache)),
            performance_monitor: Arc::new(Mutex::new(performance_monitor)),
            exporter,
            backup_manager: Arc::new(Mutex::new(backup_manager)),
            middleware_chain,
        }
    }

    /// Create a new MCP server with comprehensive configuration
    #[must_use]
    pub fn new_with_mcp_config(
        db: Arc<ThingsDatabase>,
        config: ThingsConfig,
        mcp_config: McpServerConfig,
        unsafe_direct_db: bool,
    ) -> Self {
        let mutations = select_default_backend(Arc::clone(&db), unsafe_direct_db);
        let cache = ThingsCache::new_default();
        let performance_monitor = PerformanceMonitor::new_default();
        let exporter = DataExporter::new_default();
        let backup_manager = BackupManager::new(config);

        // Convert McpServerConfig to MiddlewareConfig
        // Always disable logging in MCP mode to prevent stdout interference with JSON-RPC
        let middleware_config = MiddlewareConfig {
            logging: middleware::LoggingConfig {
                enabled: false, // Always disabled in MCP mode for JSON-RPC compatibility
                level: mcp_config.logging.level.clone(),
            },
            validation: middleware::ValidationConfig {
                enabled: mcp_config.security.validation.enabled,
                strict_mode: mcp_config.security.validation.strict_mode,
            },
            performance: middleware::PerformanceConfig {
                enabled: mcp_config.performance.enabled,
                slow_request_threshold_ms: mcp_config.performance.slow_request_threshold_ms,
            },
            security: middleware::SecurityConfig {
                authentication: middleware::AuthenticationConfig {
                    enabled: mcp_config.security.authentication.enabled,
                    require_auth: mcp_config.security.authentication.require_auth,
                    jwt_secret: mcp_config.security.authentication.jwt_secret,
                    api_keys: mcp_config
                        .security
                        .authentication
                        .api_keys
                        .iter()
                        .map(|key| middleware::ApiKeyConfig {
                            key: key.key.clone(),
                            key_id: key.key_id.clone(),
                            permissions: key.permissions.clone(),
                            expires_at: key.expires_at.clone(),
                        })
                        .collect(),
                    oauth: mcp_config
                        .security
                        .authentication
                        .oauth
                        .as_ref()
                        .map(|oauth| middleware::OAuth2Config {
                            client_id: oauth.client_id.clone(),
                            client_secret: oauth.client_secret.clone(),
                            token_endpoint: oauth.token_endpoint.clone(),
                            scopes: oauth.scopes.clone(),
                        }),
                },
                rate_limiting: middleware::RateLimitingConfig {
                    enabled: mcp_config.security.rate_limiting.enabled,
                    requests_per_minute: mcp_config.security.rate_limiting.requests_per_minute,
                    burst_limit: mcp_config.security.rate_limiting.burst_limit,
                    custom_limits: mcp_config.security.rate_limiting.custom_limits.clone(),
                },
            },
        };

        let middleware_chain = middleware_config.build_chain();

        Self {
            db,
            mutations,
            unsafe_direct_db,
            process_check: is_things3_running,
            cache: Arc::new(Mutex::new(cache)),
            performance_monitor: Arc::new(Mutex::new(performance_monitor)),
            exporter,
            backup_manager: Arc::new(Mutex::new(backup_manager)),
            middleware_chain,
        }
    }

    /// Get the middleware chain for inspection or modification
    #[must_use]
    pub fn middleware_chain(&self) -> &MiddlewareChain {
        &self.middleware_chain
    }

    /// The mutation backend's static identifier — `"applescript"` (safe
    /// default on macOS) or `"sqlx"` (direct DB writes; deprecated). Used by
    /// tests and operators to confirm which path is active.
    #[must_use]
    pub fn backend_kind(&self) -> &'static str {
        self.mutations.kind()
    }

    /// Override the Things 3 process check used by `restore_database`.
    ///
    /// Tests use this to bypass the live `pgrep -x Things3` call. Production
    /// code should never need it — the default predicate is correct.
    pub fn set_process_check_for_test(&mut self, check: fn() -> bool) {
        self.process_check = check;
    }

    /// List available MCP tools
    ///
    /// # Errors
    /// Returns an error if tool generation fails
    pub fn list_tools(&self) -> McpResult<ListToolsResult> {
        Ok(ListToolsResult {
            tools: Self::get_available_tools(),
        })
    }

    /// Call a specific MCP tool
    ///
    /// # Errors
    /// Returns an error if tool execution fails or tool is not found
    pub async fn call_tool(&self, request: CallToolRequest) -> McpResult<CallToolResult> {
        self.middleware_chain
            .execute(
                request,
                |req| async move { self.handle_tool_call(req).await },
            )
            .await
    }

    /// Call a specific MCP tool with fallback error handling
    ///
    /// This method provides backward compatibility by converting `McpError` to `CallToolResult`
    /// for cases where the caller expects a `CallToolResult` even on error
    pub async fn call_tool_with_fallback(&self, request: CallToolRequest) -> CallToolResult {
        match self.handle_tool_call(request).await {
            Ok(result) => result,
            Err(error) => error.to_call_result(),
        }
    }

    /// List available MCP resources
    ///
    /// # Errors
    /// Returns an error if resource generation fails
    pub fn list_resources(&self) -> McpResult<ListResourcesResult> {
        Ok(ListResourcesResult {
            resources: Self::get_available_resources(),
        })
    }

    /// Read a specific MCP resource
    ///
    /// # Errors
    /// Returns an error if resource reading fails or resource is not found
    pub async fn read_resource(
        &self,
        request: ReadResourceRequest,
    ) -> McpResult<ReadResourceResult> {
        self.handle_resource_read(request).await
    }

    /// Read a specific MCP resource with fallback error handling
    ///
    /// This method provides backward compatibility by converting `McpError` to `ReadResourceResult`
    /// for cases where the caller expects a `ReadResourceResult` even on error
    pub async fn read_resource_with_fallback(
        &self,
        request: ReadResourceRequest,
    ) -> ReadResourceResult {
        match self.handle_resource_read(request).await {
            Ok(result) => result,
            Err(error) => error.to_resource_result(),
        }
    }

    /// List available MCP prompts
    ///
    /// # Errors
    /// Returns an error if prompt generation fails
    pub fn list_prompts(&self) -> McpResult<ListPromptsResult> {
        Ok(ListPromptsResult {
            prompts: Self::get_available_prompts(),
        })
    }

    /// Get a specific MCP prompt with arguments
    ///
    /// # Errors
    /// Returns an error if prompt retrieval fails or prompt is not found
    pub async fn get_prompt(&self, request: GetPromptRequest) -> McpResult<GetPromptResult> {
        self.handle_prompt_request(request).await
    }

    /// Get a specific MCP prompt with fallback error handling
    ///
    /// This method provides backward compatibility by converting `McpError` to `GetPromptResult`
    /// for cases where the caller expects a `GetPromptResult` even on error
    pub async fn get_prompt_with_fallback(&self, request: GetPromptRequest) -> GetPromptResult {
        match self.handle_prompt_request(request).await {
            Ok(result) => result,
            Err(error) => error.to_prompt_result(),
        }
    }

    /// Get available MCP tools
    fn get_available_tools() -> Vec<Tool> {
        let mut tools = Vec::new();
        tools.extend(Self::get_data_retrieval_tools());
        tools.extend(Self::get_task_management_tools());
        tools.extend(Self::get_bulk_operation_tools());
        tools.extend(Self::get_tag_management_tools());
        tools.extend(Self::get_analytics_tools());
        tools.extend(Self::get_backup_tools());
        tools.extend(Self::get_system_tools());
        tools
    }

    fn get_data_retrieval_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "get_inbox".to_string(),
                description: "Get tasks from the inbox".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of tasks to return"
                        }
                    }
                }),
            },
            Tool {
                name: "get_today".to_string(),
                description: "Get tasks scheduled for today".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of tasks to return"
                        }
                    }
                }),
            },
            Tool {
                name: "get_projects".to_string(),
                description: "Get all projects, optionally filtered by area".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "area_uuid": {
                            "type": "string",
                            "description": "Optional area UUID to filter projects"
                        }
                    }
                }),
            },
            Tool {
                name: "get_areas".to_string(),
                description: "Get all areas".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "search_tasks".to_string(),
                description: "Search for tasks by query".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of tasks to return"
                        }
                    },
                    "required": ["query"]
                }),
            },
            Tool {
                name: "get_recent_tasks".to_string(),
                description: "Get recently created or modified tasks".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of tasks to return"
                        },
                        "hours": {
                            "type": "integer",
                            "description": "Number of hours to look back"
                        }
                    }
                }),
            },
            Tool {
                name: "logbook_search".to_string(),
                description: "Search completed tasks in the Things 3 logbook. Supports text search, date ranges, and filtering by project/area/tags.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "search_text": {
                            "type": "string",
                            "description": "Search in task titles and notes (case-insensitive)"
                        },
                        "from_date": {
                            "type": "string",
                            "format": "date",
                            "description": "Start date for completion date range (YYYY-MM-DD)"
                        },
                        "to_date": {
                            "type": "string",
                            "format": "date",
                            "description": "End date for completion date range (YYYY-MM-DD)"
                        },
                        "project_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Filter by project UUID"
                        },
                        "area_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Filter by area UUID"
                        },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Filter by one or more tags (all must match)"
                        },
                        "limit": {
                            "type": "integer",
                            "default": 50,
                            "minimum": 1,
                            "maximum": 500,
                            "description": "Maximum number of results to return (default: 50, max: 500)"
                        },
                        "offset": {
                            "type": "integer",
                            "default": 0,
                            "minimum": 0,
                            "description": "Number of results to skip for pagination (default: 0). Applied at the SQL level before tag filtering."
                        }
                    }
                }),
            },
        ]
    }

    fn get_task_management_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "create_task".to_string(),
                description: "Create a new task in Things 3".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Task title (required)"
                        },
                        "task_type": {
                            "type": "string",
                            "enum": ["to-do", "project", "heading"],
                            "description": "Task type (default: to-do)"
                        },
                        "notes": {
                            "type": "string",
                            "description": "Task notes"
                        },
                        "start_date": {
                            "type": "string",
                            "format": "date",
                            "description": "Start date (YYYY-MM-DD)"
                        },
                        "deadline": {
                            "type": "string",
                            "format": "date",
                            "description": "Deadline (YYYY-MM-DD)"
                        },
                        "project_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Project UUID"
                        },
                        "area_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Area UUID"
                        },
                        "parent_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Parent task UUID (for subtasks)"
                        },
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Tag names"
                        },
                        "status": {
                            "type": "string",
                            "enum": ["incomplete", "completed", "canceled", "trashed"],
                            "description": "Initial status (default: incomplete)"
                        }
                    },
                    "required": ["title"]
                }),
            },
            Tool {
                name: "update_task".to_string(),
                description: "Update an existing task (only provided fields will be updated)"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Task UUID (required)"
                        },
                        "title": {
                            "type": "string",
                            "description": "New task title"
                        },
                        "notes": {
                            "type": "string",
                            "description": "New task notes"
                        },
                        "start_date": {
                            "type": "string",
                            "format": "date",
                            "description": "New start date (YYYY-MM-DD)"
                        },
                        "deadline": {
                            "type": "string",
                            "format": "date",
                            "description": "New deadline (YYYY-MM-DD)"
                        },
                        "status": {
                            "type": "string",
                            "enum": ["incomplete", "completed", "canceled", "trashed"],
                            "description": "New task status"
                        },
                        "project_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "New project UUID"
                        },
                        "area_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "New area UUID"
                        },
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "New tag names"
                        }
                    },
                    "required": ["uuid"]
                }),
            },
            Tool {
                name: "complete_task".to_string(),
                description: "Mark a task as completed".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "UUID of the task to complete"
                        }
                    },
                    "required": ["uuid"]
                }),
            },
            Tool {
                name: "uncomplete_task".to_string(),
                description: "Mark a completed task as incomplete".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "UUID of the task to mark incomplete"
                        }
                    },
                    "required": ["uuid"]
                }),
            },
            Tool {
                name: "delete_task".to_string(),
                description: "Soft delete a task (set trashed=1)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "UUID of the task to delete"
                        },
                        "child_handling": {
                            "type": "string",
                            "enum": ["error", "cascade", "orphan"],
                            "default": "error",
                            "description": "How to handle child tasks: error (fail if children exist), cascade (delete children too), orphan (delete parent only)"
                        }
                    },
                    "required": ["uuid"]
                }),
            },
            Tool {
                name: "bulk_create_tasks".to_string(),
                description: "Create multiple tasks at once".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "tasks": {
                            "type": "array",
                            "description": "Array of task objects to create (1–1000 items)",
                            "minItems": 1,
                            "maxItems": 1000,
                            "items": {
                                "type": "object",
                                "properties": {
                                    "title": {
                                        "type": "string",
                                        "description": "Task title (required)"
                                    },
                                    "task_type": {
                                        "type": "string",
                                        "enum": ["to-do", "project", "heading"],
                                        "description": "Task type (default: to-do)"
                                    },
                                    "notes": {
                                        "type": "string",
                                        "description": "Task notes"
                                    },
                                    "start_date": {
                                        "type": "string",
                                        "format": "date",
                                        "description": "Start date (YYYY-MM-DD)"
                                    },
                                    "deadline": {
                                        "type": "string",
                                        "format": "date",
                                        "description": "Deadline (YYYY-MM-DD)"
                                    },
                                    "project_uuid": {
                                        "type": "string",
                                        "format": "uuid",
                                        "description": "Project UUID"
                                    },
                                    "area_uuid": {
                                        "type": "string",
                                        "format": "uuid",
                                        "description": "Area UUID"
                                    },
                                    "parent_uuid": {
                                        "type": "string",
                                        "format": "uuid",
                                        "description": "Parent task UUID (for subtasks)"
                                    },
                                    "tags": {
                                        "type": "array",
                                        "items": {"type": "string"},
                                        "description": "Tag names"
                                    },
                                    "status": {
                                        "type": "string",
                                        "enum": ["incomplete", "completed", "canceled", "trashed"],
                                        "description": "Initial status (default: incomplete)"
                                    }
                                },
                                "required": ["title"]
                            }
                        }
                    },
                    "required": ["tasks"]
                }),
            },
            Tool {
                name: "create_project".to_string(),
                description: "Create a new project (a task with type=project)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Project title (required)"
                        },
                        "notes": {
                            "type": "string",
                            "description": "Project notes"
                        },
                        "area_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Area UUID"
                        },
                        "start_date": {
                            "type": "string",
                            "format": "date",
                            "description": "Start date (YYYY-MM-DD)"
                        },
                        "deadline": {
                            "type": "string",
                            "format": "date",
                            "description": "Deadline (YYYY-MM-DD)"
                        },
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Tag names"
                        }
                    },
                    "required": ["title"]
                }),
            },
            Tool {
                name: "update_project".to_string(),
                description: "Update an existing project (only provided fields will be updated)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Project UUID (required)"
                        },
                        "title": {
                            "type": "string",
                            "description": "New project title"
                        },
                        "notes": {
                            "type": "string",
                            "description": "New project notes"
                        },
                        "area_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "New area UUID"
                        },
                        "start_date": {
                            "type": "string",
                            "format": "date",
                            "description": "New start date (YYYY-MM-DD)"
                        },
                        "deadline": {
                            "type": "string",
                            "format": "date",
                            "description": "New deadline (YYYY-MM-DD)"
                        },
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "New tag names"
                        }
                    },
                    "required": ["uuid"]
                }),
            },
            Tool {
                name: "complete_project".to_string(),
                description: "Mark a project as completed, with options for handling child tasks".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "UUID of the project to complete"
                        },
                        "child_handling": {
                            "type": "string",
                            "enum": ["error", "cascade", "orphan"],
                            "default": "error",
                            "description": "How to handle child tasks: error (fail if children exist), cascade (complete children too), orphan (move children to inbox)"
                        }
                    },
                    "required": ["uuid"]
                }),
            },
            Tool {
                name: "delete_project".to_string(),
                description: "Soft delete a project (set trashed=1), with options for handling child tasks".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "UUID of the project to delete"
                        },
                        "child_handling": {
                            "type": "string",
                            "enum": ["error", "cascade", "orphan"],
                            "default": "error",
                            "description": "How to handle child tasks: error (fail if children exist), cascade (delete children too), orphan (move children to inbox)"
                        }
                    },
                    "required": ["uuid"]
                }),
            },
            Tool {
                name: "create_area".to_string(),
                description: "Create a new area".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Area title (required)"
                        }
                    },
                    "required": ["title"]
                }),
            },
            Tool {
                name: "update_area".to_string(),
                description: "Update an existing area".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Area UUID (required)"
                        },
                        "title": {
                            "type": "string",
                            "description": "New area title (required)"
                        }
                    },
                    "required": ["uuid", "title"]
                }),
            },
            Tool {
                name: "delete_area".to_string(),
                description: "Delete an area (hard delete). All projects in this area will be moved to no area.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "UUID of the area to delete"
                        }
                    },
                    "required": ["uuid"]
                }),
            },
        ]
    }

    fn get_analytics_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "get_productivity_metrics".to_string(),
                description: "Get productivity metrics and statistics".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "days": {
                            "type": "integer",
                            "description": "Number of days to look back for metrics"
                        }
                    }
                }),
            },
            Tool {
                name: "export_data".to_string(),
                description: "Export data in various formats. When output_path is provided, writes to that file and returns a short confirmation; otherwise returns the data inline. Note: CSV format does not support data_type=all.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "format": {
                            "type": "string",
                            "description": "Export format",
                            "enum": ["json", "csv", "markdown"]
                        },
                        "data_type": {
                            "type": "string",
                            "description": "Type of data to export",
                            "enum": ["tasks", "projects", "areas", "all"]
                        },
                        "output_path": {
                            "type": "string",
                            "description": "Optional absolute path to write the export file. Supports leading ~ for home directory. When provided, returns a confirmation object instead of inline data."
                        }
                    },
                    "required": ["format", "data_type"]
                }),
            },
        ]
    }

    fn get_backup_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "backup_database".to_string(),
                description: "Create a backup of the Things 3 database".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "backup_dir": {
                            "type": "string",
                            "description": "Directory to store the backup"
                        },
                        "description": {
                            "type": "string",
                            "description": "Optional description for the backup"
                        }
                    },
                    "required": ["backup_dir"]
                }),
            },
            Tool {
                name: "restore_database".to_string(),
                description: "Restore from a backup".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "backup_path": {
                            "type": "string",
                            "description": "Path to the backup file"
                        }
                    },
                    "required": ["backup_path"]
                }),
            },
            Tool {
                name: "list_backups".to_string(),
                description: "List available backups".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "backup_dir": {
                            "type": "string",
                            "description": "Directory containing backups"
                        }
                    },
                    "required": ["backup_dir"]
                }),
            },
        ]
    }

    fn get_system_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "get_performance_stats".to_string(),
                description: "Get performance statistics and metrics".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "get_system_metrics".to_string(),
                description: "Get current system resource metrics".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "get_cache_stats".to_string(),
                description: "Get cache statistics and hit rates".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }

    fn get_bulk_operation_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "bulk_move".to_string(),
                description: "Move multiple tasks to a project or area (transactional)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "task_uuids": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Array of task UUIDs to move"
                        },
                        "project_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Target project UUID (optional)"
                        },
                        "area_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Target area UUID (optional)"
                        }
                    },
                    "required": ["task_uuids"]
                }),
            },
            Tool {
                name: "bulk_update_dates".to_string(),
                description: "Update dates for multiple tasks with validation (transactional)"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "task_uuids": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Array of task UUIDs to update"
                        },
                        "start_date": {
                            "type": "string",
                            "format": "date",
                            "description": "New start date (YYYY-MM-DD, optional)"
                        },
                        "deadline": {
                            "type": "string",
                            "format": "date",
                            "description": "New deadline (YYYY-MM-DD, optional)"
                        },
                        "clear_start_date": {
                            "type": "boolean",
                            "description": "Clear start date (set to NULL, default: false)"
                        },
                        "clear_deadline": {
                            "type": "boolean",
                            "description": "Clear deadline (set to NULL, default: false)"
                        }
                    },
                    "required": ["task_uuids"]
                }),
            },
            Tool {
                name: "bulk_complete".to_string(),
                description: "Mark multiple tasks as completed (transactional)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "task_uuids": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Array of task UUIDs to complete"
                        }
                    },
                    "required": ["task_uuids"]
                }),
            },
            Tool {
                name: "bulk_delete".to_string(),
                description: "Delete multiple tasks (soft delete, transactional)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "task_uuids": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Array of task UUIDs to delete"
                        }
                    },
                    "required": ["task_uuids"]
                }),
            },
        ]
    }

    fn get_tag_management_tools() -> Vec<Tool> {
        vec![
            // Tag Discovery Tools
            Tool {
                name: "search_tags".to_string(),
                description: "Search for existing tags (finds exact and similar matches)"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query for tag titles"
                        },
                        "include_similar": {
                            "type": "boolean",
                            "description": "Include fuzzy matches (default: true)"
                        },
                        "min_similarity": {
                            "type": "number",
                            "description": "Minimum similarity score 0.0-1.0 (default: 0.7)"
                        }
                    },
                    "required": ["query"]
                }),
            },
            Tool {
                name: "get_tag_suggestions".to_string(),
                description: "Get tag suggestions for a title (prevents duplicates)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Proposed tag title"
                        }
                    },
                    "required": ["title"]
                }),
            },
            Tool {
                name: "get_popular_tags".to_string(),
                description: "Get most frequently used tags".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of tags to return (default: 20)"
                        }
                    }
                }),
            },
            Tool {
                name: "get_recent_tags".to_string(),
                description: "Get recently used tags".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of tags to return (default: 20)"
                        }
                    }
                }),
            },
            // Tag CRUD Operations
            Tool {
                name: "create_tag".to_string(),
                description: "Create a new tag (checks for duplicates first)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Tag title (required)"
                        },
                        "shortcut": {
                            "type": "string",
                            "description": "Keyboard shortcut"
                        },
                        "parent_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Parent tag UUID for nesting"
                        },
                        "force": {
                            "type": "boolean",
                            "description": "Skip duplicate check (default: false)"
                        }
                    },
                    "required": ["title"]
                }),
            },
            Tool {
                name: "update_tag".to_string(),
                description: "Update an existing tag".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Tag UUID (required)"
                        },
                        "title": {
                            "type": "string",
                            "description": "New title"
                        },
                        "shortcut": {
                            "type": "string",
                            "description": "New shortcut"
                        },
                        "parent_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "New parent UUID"
                        }
                    },
                    "required": ["uuid"]
                }),
            },
            Tool {
                name: "delete_tag".to_string(),
                description: "Delete a tag".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Tag UUID (required)"
                        },
                        "remove_from_tasks": {
                            "type": "boolean",
                            "description": "Remove tag from all tasks (default: false)"
                        }
                    },
                    "required": ["uuid"]
                }),
            },
            Tool {
                name: "merge_tags".to_string(),
                description: "Merge two tags (combine source into target)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "source_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "UUID of tag to merge from (will be deleted)"
                        },
                        "target_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "UUID of tag to merge into (will remain)"
                        }
                    },
                    "required": ["source_uuid", "target_uuid"]
                }),
            },
            // Tag Assignment Tools
            Tool {
                name: "add_tag_to_task".to_string(),
                description: "Add a tag to a task (suggests existing tags)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "task_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Task UUID (required)"
                        },
                        "tag_title": {
                            "type": "string",
                            "description": "Tag title (required)"
                        }
                    },
                    "required": ["task_uuid", "tag_title"]
                }),
            },
            Tool {
                name: "remove_tag_from_task".to_string(),
                description: "Remove a tag from a task".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "task_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Task UUID (required)"
                        },
                        "tag_title": {
                            "type": "string",
                            "description": "Tag title (required)"
                        }
                    },
                    "required": ["task_uuid", "tag_title"]
                }),
            },
            Tool {
                name: "set_task_tags".to_string(),
                description: "Replace all tags on a task".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "task_uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Task UUID (required)"
                        },
                        "tag_titles": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Array of tag titles"
                        }
                    },
                    "required": ["task_uuid", "tag_titles"]
                }),
            },
            // Tag Analytics
            Tool {
                name: "get_tag_statistics".to_string(),
                description: "Get detailed statistics for a tag".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uuid": {
                            "type": "string",
                            "format": "uuid",
                            "description": "Tag UUID (required)"
                        }
                    },
                    "required": ["uuid"]
                }),
            },
            Tool {
                name: "find_duplicate_tags".to_string(),
                description: "Find duplicate or highly similar tags".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "min_similarity": {
                            "type": "number",
                            "description": "Minimum similarity score 0.0-1.0 (default: 0.85)"
                        }
                    }
                }),
            },
            Tool {
                name: "get_tag_completions".to_string(),
                description: "Get tag auto-completions for partial input".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "prefix": {
                            "type": "string",
                            "description": "Partial tag input to complete"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum completions to return (default: 10)"
                        }
                    },
                    // "partial_input" is accepted as a hidden backward-compat alias
                    // but is not advertised here. Use "prefix" for all new callers.
                    "required": ["prefix"]
                }),
            },
        ]
    }

    /// Handle tool call
    async fn handle_tool_call(&self, request: CallToolRequest) -> McpResult<CallToolResult> {
        let tool_name = &request.name;
        let arguments = request.arguments.unwrap_or_default();

        let result = match tool_name.as_str() {
            "get_inbox" => self.handle_get_inbox(arguments).await,
            "get_today" => self.handle_get_today(arguments).await,
            "get_projects" => self.handle_get_projects(arguments).await,
            "get_areas" => self.handle_get_areas(arguments).await,
            "search_tasks" => self.handle_search_tasks(arguments).await,
            "logbook_search" => self.handle_logbook_search(arguments).await,
            "create_task" => self.handle_create_task(arguments).await,
            "update_task" => self.handle_update_task(arguments).await,
            "complete_task" => self.handle_complete_task(arguments).await,
            "uncomplete_task" => self.handle_uncomplete_task(arguments).await,
            "delete_task" => self.handle_delete_task(arguments).await,
            "bulk_move" => self.handle_bulk_move(arguments).await,
            "bulk_update_dates" => self.handle_bulk_update_dates(arguments).await,
            "bulk_complete" => self.handle_bulk_complete(arguments).await,
            "bulk_delete" => self.handle_bulk_delete(arguments).await,
            "create_project" => self.handle_create_project(arguments).await,
            "update_project" => self.handle_update_project(arguments).await,
            "complete_project" => self.handle_complete_project(arguments).await,
            "delete_project" => self.handle_delete_project(arguments).await,
            "create_area" => self.handle_create_area(arguments).await,
            "update_area" => self.handle_update_area(arguments).await,
            "delete_area" => self.handle_delete_area(arguments).await,
            "get_productivity_metrics" => self.handle_get_productivity_metrics(arguments).await,
            "export_data" => self.handle_export_data(arguments).await,
            "bulk_create_tasks" => self.handle_bulk_create_tasks(arguments).await,
            "get_recent_tasks" => self.handle_get_recent_tasks(arguments).await,
            "backup_database" => self.handle_backup_database(arguments).await,
            "restore_database" => self.handle_restore_database(arguments).await,
            "list_backups" => self.handle_list_backups(arguments).await,
            "get_performance_stats" => self.handle_get_performance_stats(arguments).await,
            "get_system_metrics" => self.handle_get_system_metrics(arguments).await,
            "get_cache_stats" => self.handle_get_cache_stats(arguments).await,
            // Tag discovery tools
            "search_tags" => self.handle_search_tags_tool(arguments).await,
            "get_tag_suggestions" => self.handle_get_tag_suggestions(arguments).await,
            "get_popular_tags" => self.handle_get_popular_tags(arguments).await,
            "get_recent_tags" => self.handle_get_recent_tags(arguments).await,
            // Tag CRUD
            "create_tag" => self.handle_create_tag(arguments).await,
            "update_tag" => self.handle_update_tag(arguments).await,
            "delete_tag" => self.handle_delete_tag(arguments).await,
            "merge_tags" => self.handle_merge_tags(arguments).await,
            // Tag assignment
            "add_tag_to_task" => self.handle_add_tag_to_task(arguments).await,
            "remove_tag_from_task" => self.handle_remove_tag_from_task(arguments).await,
            "set_task_tags" => self.handle_set_task_tags(arguments).await,
            // Tag analytics
            "get_tag_statistics" => self.handle_get_tag_statistics(arguments).await,
            "find_duplicate_tags" => self.handle_find_duplicate_tags(arguments).await,
            "get_tag_completions" => self.handle_get_tag_completions(arguments).await,
            _ => {
                return Err(McpError::tool_not_found(tool_name));
            }
        };

        result
    }

    // ============================================================================
    // Bulk Operation Handlers
    // ============================================================================

    // ========================================================================
    // TAG TOOL HANDLERS
    // ========================================================================

    /// Get available MCP prompts
    fn get_available_prompts() -> Vec<Prompt> {
        vec![
            Self::create_task_review_prompt(),
            Self::create_project_planning_prompt(),
            Self::create_productivity_analysis_prompt(),
            Self::create_backup_strategy_prompt(),
        ]
    }

    fn create_task_review_prompt() -> Prompt {
        Prompt {
            name: "task_review".to_string(),
            description: "Review task for completeness and clarity".to_string(),
            arguments: vec![
                PromptArgument {
                    name: "task_title".to_string(),
                    description: Some("The title of the task to review".to_string()),
                    required: true,
                },
                PromptArgument {
                    name: "task_notes".to_string(),
                    description: Some("Optional notes or description of the task".to_string()),
                    required: false,
                },
                PromptArgument {
                    name: "context".to_string(),
                    description: Some("Optional context about the task or project".to_string()),
                    required: false,
                },
            ],
        }
    }

    fn create_project_planning_prompt() -> Prompt {
        Prompt {
            name: "project_planning".to_string(),
            description: "Help plan projects with tasks and deadlines".to_string(),
            arguments: vec![
                PromptArgument {
                    name: "project_title".to_string(),
                    description: Some("The title of the project to plan".to_string()),
                    required: true,
                },
                PromptArgument {
                    name: "project_description".to_string(),
                    description: Some(
                        "Description of what the project aims to achieve".to_string(),
                    ),
                    required: false,
                },
                PromptArgument {
                    name: "deadline".to_string(),
                    description: Some("Optional deadline for the project".to_string()),
                    required: false,
                },
                PromptArgument {
                    name: "complexity".to_string(),
                    description: Some(
                        "Project complexity level: simple, medium, or complex".to_string(),
                    ),
                    required: false,
                },
            ],
        }
    }

    fn create_productivity_analysis_prompt() -> Prompt {
        Prompt {
            name: "productivity_analysis".to_string(),
            description: "Analyze productivity patterns".to_string(),
            arguments: vec![
                PromptArgument {
                    name: "time_period".to_string(),
                    description: Some(
                        "Time period to analyze: week, month, quarter, or year".to_string(),
                    ),
                    required: true,
                },
                PromptArgument {
                    name: "focus_area".to_string(),
                    description: Some(
                        "Specific area to focus on: completion_rate, time_management, task_distribution, or all".to_string(),
                    ),
                    required: false,
                },
                PromptArgument {
                    name: "include_recommendations".to_string(),
                    description: Some(
                        "Whether to include improvement recommendations".to_string(),
                    ),
                    required: false,
                },
            ],
        }
    }

    fn create_backup_strategy_prompt() -> Prompt {
        Prompt {
            name: "backup_strategy".to_string(),
            description: "Suggest backup strategies".to_string(),
            arguments: vec![
                PromptArgument {
                    name: "data_volume".to_string(),
                    description: Some(
                        "Estimated data volume: small, medium, or large".to_string(),
                    ),
                    required: true,
                },
                PromptArgument {
                    name: "frequency".to_string(),
                    description: Some(
                        "Desired backup frequency: daily, weekly, or monthly".to_string(),
                    ),
                    required: true,
                },
                PromptArgument {
                    name: "retention_period".to_string(),
                    description: Some(
                        "How long to keep backups: 1_month, 3_months, 6_months, 1_year, or indefinite".to_string(),
                    ),
                    required: false,
                },
                PromptArgument {
                    name: "storage_preference".to_string(),
                    description: Some(
                        "Preferred storage type: local, cloud, or hybrid".to_string(),
                    ),
                    required: false,
                },
            ],
        }
    }

    /// Get available MCP resources
    fn get_available_resources() -> Vec<Resource> {
        vec![
            Resource {
                uri: "things://inbox".to_string(),
                name: "Inbox Tasks".to_string(),
                description: "Current inbox tasks from Things 3".to_string(),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "things://projects".to_string(),
                name: "All Projects".to_string(),
                description: "All projects in Things 3".to_string(),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "things://areas".to_string(),
                name: "All Areas".to_string(),
                description: "All areas in Things 3".to_string(),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "things://today".to_string(),
                name: "Today's Tasks".to_string(),
                description: "Tasks scheduled for today".to_string(),
                mime_type: Some("application/json".to_string()),
            },
        ]
    }

    /// Handle a JSON-RPC request and return a JSON-RPC response
    ///
    /// Returns `None` for notifications (messages without `id` field) - these don't require a response
    ///
    /// # Errors
    /// Returns an error if request parsing or handling fails
    pub async fn handle_jsonrpc_request(
        &self,
        request: serde_json::Value,
    ) -> things3_core::Result<Option<serde_json::Value>> {
        use serde_json::json;

        let method = request["method"].as_str().ok_or_else(|| {
            things3_core::ThingsError::unknown("Missing method in JSON-RPC request".to_string())
        })?;
        let params = request["params"].clone();

        // Check if this is a notification (no `id` field present)
        // In JSON-RPC, notifications don't have an `id` field, so get("id") returns None
        let is_notification = request.get("id").is_none();

        // Handle notifications silently (they don't require a response)
        if is_notification {
            match method {
                "notifications/initialized" => {
                    // Silently acknowledge the initialized notification
                    return Ok(None);
                }
                _ => {
                    // Unknown notification - silently ignore
                    return Ok(None);
                }
            }
        }

        // For requests (with `id` field), we need the id for the response
        let id = request["id"].clone();

        let result = match method {
            "initialize" => {
                // Negotiate protocol version: respond with the highest version we support
                // that is <= the client's requested version. Claude Code 2.1+ uses
                // 2025-03-26 or newer; responding with 2024-11-05 causes it to silently
                // drop the server's tools from its deferred-tool catalog.
                let client_version = params
                    .get("protocolVersion")
                    .and_then(|v| v.as_str())
                    .unwrap_or("2024-11-05");
                // Supported versions (oldest → newest). When adding support for
                // a new spec version, add a branch here and update the
                // accepted_response_versions list in test_initialize_handshake_2025_11_25.
                let protocol_version = if client_version >= "2025-03-26" {
                    "2025-03-26"
                } else {
                    "2024-11-05"
                };
                json!({
                    "protocolVersion": protocol_version,
                    "capabilities": {
                        "tools": { "listChanged": false },
                        "resources": { "subscribe": false, "listChanged": false },
                        "prompts": { "listChanged": false }
                    },
                    "serverInfo": {
                        "name": "things3-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                })
            }
            "tools/list" => {
                let tools_result = self.list_tools().map_err(|e| {
                    things3_core::ThingsError::unknown(format!("Failed to list tools: {}", e))
                })?;
                json!(tools_result)
            }
            "tools/call" => {
                let tool_name = params["name"]
                    .as_str()
                    .ok_or_else(|| {
                        things3_core::ThingsError::unknown(
                            "Missing tool name in params".to_string(),
                        )
                    })?
                    .to_string();
                let arguments = params["arguments"].clone();

                let call_request = CallToolRequest {
                    name: tool_name,
                    arguments: Some(arguments),
                };

                // Use the fallback variant so tool-level failures (e.g. an
                // AppleScript backend error) come back as a structured
                // `{"isError": true, "content": [...]}` envelope inside the
                // JSON-RPC `result`, rather than propagating up as an `Err`
                // and dropping the MCP connection (#148).
                let call_result = self.call_tool_with_fallback(call_request).await;

                json!(call_result)
            }
            "resources/list" => {
                let resources_result = self.list_resources().map_err(|e| {
                    things3_core::ThingsError::unknown(format!("Failed to list resources: {}", e))
                })?;
                // Spec: result must be ListResourcesResult `{"resources":[...]}`, not a bare array.
                json!(resources_result)
            }
            "resources/read" => {
                let uri = params["uri"]
                    .as_str()
                    .ok_or_else(|| {
                        things3_core::ThingsError::unknown("Missing URI in params".to_string())
                    })?
                    .to_string();

                let read_request = ReadResourceRequest { uri };
                // Same envelope pattern as `tools/call` above (#148).
                let read_result = self.read_resource_with_fallback(read_request).await;

                json!(read_result)
            }
            "prompts/list" => {
                let prompts_result = self.list_prompts().map_err(|e| {
                    things3_core::ThingsError::unknown(format!("Failed to list prompts: {}", e))
                })?;
                // Spec: result must be ListPromptsResult `{"prompts":[...]}`, not a bare array.
                json!(prompts_result)
            }
            "prompts/get" => {
                let prompt_name = params["name"]
                    .as_str()
                    .ok_or_else(|| {
                        things3_core::ThingsError::unknown(
                            "Missing prompt name in params".to_string(),
                        )
                    })?
                    .to_string();
                let arguments = params.get("arguments").cloned();

                let get_request = GetPromptRequest {
                    name: prompt_name,
                    arguments,
                };

                // Same envelope pattern as `tools/call` above (#148).
                let get_result = self.get_prompt_with_fallback(get_request).await;

                json!(get_result)
            }
            _ => {
                return Ok(Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {}", method)
                    }
                })));
            }
        };

        Ok(Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        })))
    }
}

pub(in crate::mcp) fn expand_tilde(path: &str) -> McpResult<std::path::PathBuf> {
    if path == "~" || path.starts_with("~/") {
        let home = std::env::var("HOME").map_err(|_| {
            McpError::invalid_parameter(
                "output_path",
                "cannot expand ~: HOME environment variable is not set",
            )
        })?;
        Ok(std::path::PathBuf::from(format!("{}{}", home, &path[1..])))
    } else if path.starts_with('~') {
        Err(McpError::invalid_parameter(
            "output_path",
            "~user expansion is not supported; use an absolute path or ~/...",
        ))
    } else {
        Ok(std::path::PathBuf::from(path))
    }
}

#[cfg(test)]
mod backend_selection_tests {
    use super::*;

    /// Build a server with a fresh temp DB and the given `unsafe_direct_db` flag,
    /// routing through `ThingsMcpServer::new` so platform-aware backend selection runs.
    fn build_server(unsafe_direct_db: bool) -> (ThingsMcpServer, tempfile::NamedTempFile) {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();
        let db_path_clone = db_path.clone();

        let db = std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { ThingsDatabase::new(&db_path_clone).await.unwrap() })
        })
        .join()
        .unwrap();

        let config = ThingsConfig::new(&db_path, false);
        let server = ThingsMcpServer::new(Arc::new(db), config, unsafe_direct_db);
        (server, temp_file)
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn defaults_to_applescript_on_macos() {
        let (server, _tmp) = build_server(false);
        assert_eq!(server.backend_kind(), "applescript");
    }

    #[tokio::test]
    async fn unsafe_flag_selects_sqlx() {
        let (server, _tmp) = build_server(true);
        assert_eq!(server.backend_kind(), "sqlx");
    }

    #[tokio::test]
    async fn restore_database_refuses_without_flag() {
        let (server, _tmp) = build_server(false);
        let err = server
            .handle_restore_database(serde_json::json!({"backup_path": "/tmp/x"}))
            .await
            .expect_err("must refuse when --unsafe-direct-db is not set");
        let msg = err.to_string();
        assert!(
            msg.contains("--unsafe-direct-db"),
            "error should name the flag, got: {msg}"
        );
    }

    #[tokio::test]
    async fn restore_database_refuses_when_things3_running() {
        let (mut server, _tmp) = build_server(true);
        server.set_process_check_for_test(|| true);
        let err = server
            .handle_restore_database(serde_json::json!({"backup_path": "/tmp/x"}))
            .await
            .expect_err("must refuse while Things 3 is running");
        let msg = err.to_string();
        assert!(
            msg.contains("Things 3"),
            "error should mention Things 3, got: {msg}"
        );
    }
}
