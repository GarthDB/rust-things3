//! MCP Middleware system for cross-cutting concerns

use crate::mcp::{CallToolRequest, CallToolResult, McpError, McpResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Middleware execution context
#[derive(Debug, Clone)]
pub struct MiddlewareContext {
    /// Request ID for tracking
    pub request_id: String,
    /// Start time of the request
    pub start_time: Instant,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, Value>,
}

impl MiddlewareContext {
    /// Create a new middleware context
    #[must_use]
    pub fn new(request_id: String) -> Self {
        Self {
            request_id,
            start_time: Instant::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Get the elapsed time since request start
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Set metadata value
    pub fn set_metadata(&mut self, key: String, value: Value) {
        self.metadata.insert(key, value);
    }

    /// Get metadata value
    #[must_use]
    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }
}

/// Middleware execution result
#[derive(Debug)]
pub enum MiddlewareResult {
    /// Continue to next middleware or handler
    Continue,
    /// Stop execution and return this result
    Stop(CallToolResult),
    /// Stop execution with error
    Error(McpError),
}

/// MCP Middleware trait for intercepting and controlling server operations
#[async_trait::async_trait]
pub trait McpMiddleware: Send + Sync {
    /// Name of the middleware for identification
    fn name(&self) -> &str;

    /// Priority/order of execution (lower numbers execute first)
    fn priority(&self) -> i32 {
        0
    }

    /// Called before the request is processed
    async fn before_request(
        &self,
        request: &CallToolRequest,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        let _ = (request, context);
        Ok(MiddlewareResult::Continue)
    }

    /// Called after the request is processed but before response is returned
    async fn after_request(
        &self,
        request: &CallToolRequest,
        response: &mut CallToolResult,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        let _ = (request, response, context);
        Ok(MiddlewareResult::Continue)
    }

    /// Called when an error occurs during request processing
    async fn on_error(
        &self,
        request: &CallToolRequest,
        error: &McpError,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        let _ = (request, error, context);
        Ok(MiddlewareResult::Continue)
    }
}

/// Middleware chain for executing multiple middleware in order
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn McpMiddleware>>,
}

impl MiddlewareChain {
    /// Create a new middleware chain
    #[must_use]
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add middleware to the chain
    #[must_use]
    pub fn add_middleware<M: McpMiddleware + 'static>(mut self, middleware: M) -> Self {
        self.middlewares.push(Arc::new(middleware));
        self.sort_by_priority();
        self
    }

    /// Add middleware from Arc
    #[must_use]
    pub fn add_arc(mut self, middleware: Arc<dyn McpMiddleware>) -> Self {
        self.middlewares.push(middleware);
        self.sort_by_priority();
        self
    }

    /// Sort middlewares by priority (lower numbers first)
    fn sort_by_priority(&mut self) {
        self.middlewares.sort_by_key(|m| m.priority());
    }

    /// Execute the middleware chain for a request
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Any middleware in the chain returns an error
    /// - The main handler function returns an error
    /// - Any middleware fails during execution
    pub async fn execute<F, Fut>(
        &self,
        request: CallToolRequest,
        handler: F,
    ) -> McpResult<CallToolResult>
    where
        F: FnOnce(CallToolRequest) -> Fut,
        Fut: std::future::Future<Output = McpResult<CallToolResult>> + Send,
    {
        let request_id = uuid::Uuid::new_v4().to_string();
        let mut context = MiddlewareContext::new(request_id);

        // Execute before_request hooks
        for middleware in &self.middlewares {
            match middleware.before_request(&request, &mut context).await? {
                MiddlewareResult::Continue => {}
                MiddlewareResult::Stop(result) => return Ok(result),
                MiddlewareResult::Error(error) => return Err(error),
            }
        }

        // Clone request for use in after_request hooks
        let request_clone = request.clone();

        // Execute the main handler
        let mut result = match handler(request).await {
            Ok(response) => response,
            Err(error) => {
                // Execute on_error hooks
                for middleware in &self.middlewares {
                    match middleware
                        .on_error(&request_clone, &error, &mut context)
                        .await?
                    {
                        MiddlewareResult::Continue => {}
                        MiddlewareResult::Stop(result) => return Ok(result),
                        MiddlewareResult::Error(middleware_error) => return Err(middleware_error),
                    }
                }
                return Err(error);
            }
        };

        // Execute after_request hooks
        for middleware in &self.middlewares {
            match middleware
                .after_request(&request_clone, &mut result, &mut context)
                .await?
            {
                MiddlewareResult::Continue => {}
                MiddlewareResult::Stop(new_result) => return Ok(new_result),
                MiddlewareResult::Error(error) => return Err(error),
            }
        }

        Ok(result)
    }

    /// Get the number of middlewares in the chain
    #[must_use]
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    /// Check if the chain is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in logging middleware
pub struct LoggingMiddleware {
    level: LogLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LoggingMiddleware {
    /// Create a new logging middleware
    #[must_use]
    pub fn new(level: LogLevel) -> Self {
        Self { level }
    }

    /// Create with debug level
    #[must_use]
    pub fn debug() -> Self {
        Self::new(LogLevel::Debug)
    }

    /// Create with info level
    #[must_use]
    pub fn info() -> Self {
        Self::new(LogLevel::Info)
    }

    /// Create with warn level
    #[must_use]
    pub fn warn() -> Self {
        Self::new(LogLevel::Warn)
    }

    /// Create with error level
    #[must_use]
    pub fn error() -> Self {
        Self::new(LogLevel::Error)
    }

    fn should_log(&self, level: LogLevel) -> bool {
        matches!(
            (self.level, level),
            (LogLevel::Debug, _)
                | (
                    LogLevel::Info,
                    LogLevel::Info | LogLevel::Warn | LogLevel::Error
                )
                | (LogLevel::Warn, LogLevel::Warn | LogLevel::Error)
                | (LogLevel::Error, LogLevel::Error)
        )
    }

    fn log(&self, level: LogLevel, message: &str) {
        if self.should_log(level) {
            match level {
                LogLevel::Debug => println!("[DEBUG] {message}"),
                LogLevel::Info => println!("[INFO] {message}"),
                LogLevel::Warn => println!("[WARN] {message}"),
                LogLevel::Error => println!("[ERROR] {message}"),
            }
        }
    }
}

#[async_trait::async_trait]
impl McpMiddleware for LoggingMiddleware {
    fn name(&self) -> &'static str {
        "logging"
    }

    fn priority(&self) -> i32 {
        100 // Low priority to run early
    }

    async fn before_request(
        &self,
        request: &CallToolRequest,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        self.log(
            LogLevel::Info,
            &format!(
                "Request started: {} (ID: {})",
                request.name, context.request_id
            ),
        );
        Ok(MiddlewareResult::Continue)
    }

    async fn after_request(
        &self,
        request: &CallToolRequest,
        response: &mut CallToolResult,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        let elapsed = context.elapsed();
        let status = if response.is_error {
            "ERROR"
        } else {
            "SUCCESS"
        };

        self.log(
            LogLevel::Info,
            &format!(
                "Request completed: {} (ID: {}) - {} in {:?}",
                request.name, context.request_id, status, elapsed
            ),
        );
        Ok(MiddlewareResult::Continue)
    }

    async fn on_error(
        &self,
        request: &CallToolRequest,
        error: &McpError,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        self.log(
            LogLevel::Error,
            &format!(
                "Request failed: {} (ID: {}) - {}",
                request.name, context.request_id, error
            ),
        );
        Ok(MiddlewareResult::Continue)
    }
}

/// Built-in validation middleware
pub struct ValidationMiddleware {
    strict_mode: bool,
}

impl ValidationMiddleware {
    /// Create a new validation middleware
    #[must_use]
    pub fn new(strict_mode: bool) -> Self {
        Self { strict_mode }
    }

    /// Create with strict mode enabled
    #[must_use]
    pub fn strict() -> Self {
        Self::new(true)
    }

    /// Create with strict mode disabled
    #[must_use]
    pub fn lenient() -> Self {
        Self::new(false)
    }

    fn validate_request(&self, request: &CallToolRequest) -> McpResult<()> {
        // Basic validation
        if request.name.is_empty() {
            return Err(McpError::validation_error("Tool name cannot be empty"));
        }

        // Validate tool name format (alphanumeric and underscores only)
        if !request
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            return Err(McpError::validation_error(
                "Tool name must contain only alphanumeric characters and underscores",
            ));
        }

        // In strict mode, validate arguments structure
        if self.strict_mode {
            if let Some(args) = &request.arguments {
                if !args.is_object() {
                    return Err(McpError::validation_error(
                        "Arguments must be a JSON object",
                    ));
                }
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl McpMiddleware for ValidationMiddleware {
    fn name(&self) -> &'static str {
        "validation"
    }

    fn priority(&self) -> i32 {
        50 // Medium priority
    }

    async fn before_request(
        &self,
        request: &CallToolRequest,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        if let Err(error) = self.validate_request(request) {
            context.set_metadata(
                "validation_error".to_string(),
                serde_json::Value::String(error.to_string()),
            );
            return Ok(MiddlewareResult::Error(error));
        }

        context.set_metadata("validated".to_string(), serde_json::Value::Bool(true));
        Ok(MiddlewareResult::Continue)
    }
}

/// Built-in performance monitoring middleware
pub struct PerformanceMiddleware {
    slow_request_threshold: Duration,
}

impl PerformanceMiddleware {
    /// Create a new performance middleware
    #[must_use]
    pub fn new(slow_request_threshold: Duration) -> Self {
        Self {
            slow_request_threshold,
        }
    }

    /// Create with default threshold (1 second)
    #[must_use]
    pub fn create_default() -> Self {
        Self::new(Duration::from_secs(1))
    }

    /// Create with custom threshold
    #[must_use]
    pub fn with_threshold(threshold: Duration) -> Self {
        Self::new(threshold)
    }
}

#[async_trait::async_trait]
impl McpMiddleware for PerformanceMiddleware {
    fn name(&self) -> &'static str {
        "performance"
    }

    fn priority(&self) -> i32 {
        200 // High priority to run late
    }

    async fn after_request(
        &self,
        request: &CallToolRequest,
        _response: &mut CallToolResult,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        let elapsed = context.elapsed();

        // Record performance metrics
        context.set_metadata(
            "duration_ms".to_string(),
            serde_json::Value::Number(serde_json::Number::from(
                u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
            )),
        );

        context.set_metadata(
            "is_slow".to_string(),
            serde_json::Value::Bool(elapsed > self.slow_request_threshold),
        );

        // Log slow requests
        if elapsed > self.slow_request_threshold {
            println!(
                "[PERF] Slow request detected: {} took {:?} (threshold: {:?})",
                request.name, elapsed, self.slow_request_threshold
            );
        }

        Ok(MiddlewareResult::Continue)
    }
}

/// Middleware configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiddlewareConfig {
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Validation configuration
    pub validation: ValidationConfig,
    /// Performance monitoring configuration
    pub performance: PerformanceConfig,
}

/// Logging middleware configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Enable logging middleware
    pub enabled: bool,
    /// Log level for logging middleware
    pub level: String,
}

/// Validation middleware configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Enable validation middleware
    pub enabled: bool,
    /// Use strict validation mode
    pub strict_mode: bool,
}

/// Performance monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable performance monitoring
    pub enabled: bool,
    /// Slow request threshold in milliseconds
    pub slow_request_threshold_ms: u64,
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            logging: LoggingConfig {
                enabled: true,
                level: "info".to_string(),
            },
            validation: ValidationConfig {
                enabled: true,
                strict_mode: false,
            },
            performance: PerformanceConfig {
                enabled: true,
                slow_request_threshold_ms: 1000,
            },
        }
    }
}

impl MiddlewareConfig {
    /// Create a new middleware configuration
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a middleware chain from this configuration
    #[must_use]
    pub fn build_chain(self) -> MiddlewareChain {
        let mut chain = MiddlewareChain::new();

        if self.logging.enabled {
            let log_level = match self.logging.level.to_lowercase().as_str() {
                "debug" => LogLevel::Debug,
                "warn" => LogLevel::Warn,
                "error" => LogLevel::Error,
                _ => LogLevel::Info,
            };
            chain = chain.add_middleware(LoggingMiddleware::new(log_level));
        }

        if self.validation.enabled {
            chain = chain.add_middleware(ValidationMiddleware::new(self.validation.strict_mode));
        }

        if self.performance.enabled {
            let threshold = Duration::from_millis(self.performance.slow_request_threshold_ms);
            chain = chain.add_middleware(PerformanceMiddleware::with_threshold(threshold));
        }

        chain
    }
}

/// Middleware-specific errors
#[derive(Error, Debug)]
pub enum MiddlewareError {
    #[error("Middleware execution failed: {message}")]
    ExecutionFailed { message: String },

    #[error("Middleware configuration error: {message}")]
    ConfigurationError { message: String },

    #[error("Middleware chain error: {message}")]
    ChainError { message: String },
}

impl From<MiddlewareError> for McpError {
    fn from(error: MiddlewareError) -> Self {
        McpError::internal_error(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::Content;

    struct TestMiddleware {
        priority: i32,
    }

    #[async_trait::async_trait]
    impl McpMiddleware for TestMiddleware {
        fn name(&self) -> &'static str {
            "test_middleware"
        }

        fn priority(&self) -> i32 {
            self.priority
        }
    }

    #[tokio::test]
    async fn test_middleware_chain_creation() {
        let chain = MiddlewareChain::new()
            .add_middleware(TestMiddleware { priority: 100 })
            .add_middleware(TestMiddleware { priority: 50 });

        assert_eq!(chain.len(), 2);
        assert!(!chain.is_empty());
    }

    #[tokio::test]
    async fn test_middleware_priority_ordering() {
        let chain = MiddlewareChain::new()
            .add_middleware(TestMiddleware { priority: 10 })
            .add_middleware(TestMiddleware { priority: 100 });

        // The chain should be sorted by priority
        assert_eq!(chain.len(), 2);
    }

    #[tokio::test]
    async fn test_middleware_execution() {
        let chain = MiddlewareChain::new()
            .add_middleware(LoggingMiddleware::info())
            .add_middleware(ValidationMiddleware::lenient());

        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: Some(serde_json::json!({"param": "value"})),
        };

        let handler = |_req: CallToolRequest| {
            Box::pin(async move {
                Ok(CallToolResult {
                    content: vec![Content::Text {
                        text: "Test response".to_string(),
                    }],
                    is_error: false,
                })
            })
        };

        let result = chain.execute(request, handler).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validation_middleware() {
        let middleware = ValidationMiddleware::strict();
        let mut context = MiddlewareContext::new("test".to_string());

        // Valid request
        let valid_request = CallToolRequest {
            name: "valid_tool".to_string(),
            arguments: Some(serde_json::json!({"param": "value"})),
        };

        let result = middleware
            .before_request(&valid_request, &mut context)
            .await;
        assert!(matches!(result, Ok(MiddlewareResult::Continue)));

        // Invalid request (empty name)
        let invalid_request = CallToolRequest {
            name: String::new(),
            arguments: None,
        };

        let result = middleware
            .before_request(&invalid_request, &mut context)
            .await;
        assert!(matches!(result, Ok(MiddlewareResult::Error(_))));
    }

    #[tokio::test]
    async fn test_performance_middleware() {
        let middleware = PerformanceMiddleware::with_threshold(Duration::from_millis(100));
        let mut context = MiddlewareContext::new("test".to_string());

        // Simulate a slow request
        tokio::time::sleep(Duration::from_millis(150)).await;

        let mut response = CallToolResult {
            content: vec![Content::Text {
                text: "Test".to_string(),
            }],
            is_error: false,
        };

        let request = CallToolRequest {
            name: "test".to_string(),
            arguments: None,
        };

        let result = middleware
            .after_request(&request, &mut response, &mut context)
            .await;
        assert!(matches!(result, Ok(MiddlewareResult::Continue)));

        // Check that performance metadata was set
        assert!(context.get_metadata("duration_ms").is_some());
        assert!(context.get_metadata("is_slow").is_some());
    }

    #[tokio::test]
    async fn test_middleware_config() {
        let config = MiddlewareConfig {
            logging: LoggingConfig {
                enabled: true,
                level: "debug".to_string(),
            },
            validation: ValidationConfig {
                enabled: true,
                strict_mode: true,
            },
            performance: PerformanceConfig {
                enabled: true,
                slow_request_threshold_ms: 500,
            },
        };

        let chain = config.build_chain();
        assert!(!chain.is_empty());
        assert!(chain.len() >= 3); // Should have logging, validation, and performance
    }
}
