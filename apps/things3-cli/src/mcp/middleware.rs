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

    #[tokio::test]
    async fn test_middleware_context_creation() {
        let context = MiddlewareContext::new("test-request-123".to_string());
        assert_eq!(context.request_id, "test-request-123");
        assert!(context.metadata.is_empty());
    }

    #[tokio::test]
    async fn test_middleware_context_elapsed() {
        let context = MiddlewareContext::new("test-request-123".to_string());
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = context.elapsed();
        assert!(elapsed.as_millis() >= 10);
    }

    #[tokio::test]
    async fn test_middleware_context_metadata() {
        let mut context = MiddlewareContext::new("test-request-123".to_string());

        // Test setting metadata
        context.set_metadata(
            "key1".to_string(),
            serde_json::Value::String("value1".to_string()),
        );
        context.set_metadata(
            "key2".to_string(),
            serde_json::Value::Number(serde_json::Number::from(42)),
        );

        // Test getting metadata
        assert_eq!(
            context.get_metadata("key1"),
            Some(&serde_json::Value::String("value1".to_string()))
        );
        assert_eq!(
            context.get_metadata("key2"),
            Some(&serde_json::Value::Number(serde_json::Number::from(42)))
        );
        assert_eq!(context.get_metadata("nonexistent"), None);
    }

    #[tokio::test]
    async fn test_middleware_result_variants() {
        let continue_result = MiddlewareResult::Continue;
        let stop_result = MiddlewareResult::Stop(CallToolResult {
            content: vec![Content::Text {
                text: "test".to_string(),
            }],
            is_error: false,
        });
        let error_result = MiddlewareResult::Error(McpError::tool_not_found("test error"));

        // Test that we can create all variants
        match continue_result {
            MiddlewareResult::Continue => {}
            _ => panic!("Expected Continue"),
        }

        match stop_result {
            MiddlewareResult::Stop(_) => {}
            _ => panic!("Expected Stop"),
        }

        match error_result {
            MiddlewareResult::Error(_) => {}
            _ => panic!("Expected Error"),
        }
    }

    #[tokio::test]
    async fn test_logging_middleware_different_levels() {
        let debug_middleware = LoggingMiddleware::new(LogLevel::Debug);
        let info_middleware = LoggingMiddleware::new(LogLevel::Info);
        let warn_middleware = LoggingMiddleware::new(LogLevel::Warn);
        let error_middleware = LoggingMiddleware::new(LogLevel::Error);

        assert_eq!(debug_middleware.name(), "logging");
        assert_eq!(info_middleware.name(), "logging");
        assert_eq!(warn_middleware.name(), "logging");
        assert_eq!(error_middleware.name(), "logging");
    }

    #[tokio::test]
    async fn test_logging_middleware_should_log() {
        let debug_middleware = LoggingMiddleware::new(LogLevel::Debug);
        let info_middleware = LoggingMiddleware::new(LogLevel::Info);
        let warn_middleware = LoggingMiddleware::new(LogLevel::Warn);
        let error_middleware = LoggingMiddleware::new(LogLevel::Error);

        // Debug should log everything
        assert!(debug_middleware.should_log(LogLevel::Debug));
        assert!(debug_middleware.should_log(LogLevel::Info));
        assert!(debug_middleware.should_log(LogLevel::Warn));
        assert!(debug_middleware.should_log(LogLevel::Error));

        // Info should log info, warn, error
        assert!(!info_middleware.should_log(LogLevel::Debug));
        assert!(info_middleware.should_log(LogLevel::Info));
        assert!(info_middleware.should_log(LogLevel::Warn));
        assert!(info_middleware.should_log(LogLevel::Error));

        // Warn should log warn, error
        assert!(!warn_middleware.should_log(LogLevel::Debug));
        assert!(!warn_middleware.should_log(LogLevel::Info));
        assert!(warn_middleware.should_log(LogLevel::Warn));
        assert!(warn_middleware.should_log(LogLevel::Error));

        // Error should only log error
        assert!(!error_middleware.should_log(LogLevel::Debug));
        assert!(!error_middleware.should_log(LogLevel::Info));
        assert!(!error_middleware.should_log(LogLevel::Warn));
        assert!(error_middleware.should_log(LogLevel::Error));
    }

    #[tokio::test]
    async fn test_validation_middleware_strict_mode() {
        let strict_middleware = ValidationMiddleware::strict();
        let lenient_middleware = ValidationMiddleware::lenient();

        assert_eq!(strict_middleware.name(), "validation");
        assert_eq!(lenient_middleware.name(), "validation");
    }

    #[tokio::test]
    async fn test_validation_middleware_creation() {
        let middleware1 = ValidationMiddleware::new(true);
        let middleware2 = ValidationMiddleware::new(false);

        assert_eq!(middleware1.name(), "validation");
        assert_eq!(middleware2.name(), "validation");
    }

    #[tokio::test]
    async fn test_performance_middleware_creation() {
        let middleware1 = PerformanceMiddleware::new(Duration::from_millis(100));
        let middleware2 = PerformanceMiddleware::with_threshold(Duration::from_millis(200));
        let middleware3 = PerformanceMiddleware::create_default();

        assert_eq!(middleware1.name(), "performance");
        assert_eq!(middleware2.name(), "performance");
        assert_eq!(middleware3.name(), "performance");
    }

    #[tokio::test]
    async fn test_middleware_chain_empty() {
        let chain = MiddlewareChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[tokio::test]
    async fn test_middleware_chain_add_middleware() {
        let chain = MiddlewareChain::new()
            .add_middleware(LoggingMiddleware::new(LogLevel::Info))
            .add_middleware(ValidationMiddleware::new(false));

        assert!(!chain.is_empty());
        assert_eq!(chain.len(), 2);
    }

    #[tokio::test]
    async fn test_middleware_chain_add_arc() {
        let middleware = Arc::new(LoggingMiddleware::new(LogLevel::Info)) as Arc<dyn McpMiddleware>;
        let chain = MiddlewareChain::new().add_arc(middleware);

        assert!(!chain.is_empty());
        assert_eq!(chain.len(), 1);
    }

    #[tokio::test]
    async fn test_middleware_chain_execution_with_empty_chain() {
        let chain = MiddlewareChain::new();
        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: None,
        };

        let result = chain
            .execute(request, |_| async {
                Ok(CallToolResult {
                    content: vec![Content::Text {
                        text: "success".to_string(),
                    }],
                    is_error: false,
                })
            })
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
    }

    #[tokio::test]
    async fn test_middleware_chain_execution_with_error() {
        let chain = MiddlewareChain::new().add_middleware(LoggingMiddleware::new(LogLevel::Info));
        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: None,
        };

        let result = chain
            .execute(request, |_| async {
                Err(McpError::tool_not_found("test error"))
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_middleware_chain_execution_with_stop() {
        let chain = MiddlewareChain::new().add_middleware(LoggingMiddleware::new(LogLevel::Info));
        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: None,
        };

        // Create a middleware that stops execution
        struct StopMiddleware;
        #[async_trait::async_trait]
        impl McpMiddleware for StopMiddleware {
            fn name(&self) -> &'static str {
                "stop"
            }

            async fn before_request(
                &self,
                _request: &CallToolRequest,
                _context: &mut MiddlewareContext,
            ) -> McpResult<MiddlewareResult> {
                Ok(MiddlewareResult::Stop(CallToolResult {
                    content: vec![Content::Text {
                        text: "stopped".to_string(),
                    }],
                    is_error: false,
                }))
            }

            async fn after_request(
                &self,
                _request: &CallToolRequest,
                _result: &mut CallToolResult,
                _context: &mut MiddlewareContext,
            ) -> McpResult<MiddlewareResult> {
                Ok(MiddlewareResult::Continue)
            }

            async fn on_error(
                &self,
                _request: &CallToolRequest,
                _error: &McpError,
                _context: &mut MiddlewareContext,
            ) -> McpResult<MiddlewareResult> {
                Ok(MiddlewareResult::Continue)
            }
        }

        let chain = chain.add_middleware(StopMiddleware);

        let result = chain
            .execute(request, |_| async {
                Ok(CallToolResult {
                    content: vec![Content::Text {
                        text: "should not reach here".to_string(),
                    }],
                    is_error: false,
                })
            })
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        let Content::Text { text } = &result.content[0];
        assert_eq!(text, "stopped");
    }

    #[tokio::test]
    async fn test_middleware_chain_execution_with_middleware_error() {
        let chain = MiddlewareChain::new().add_middleware(LoggingMiddleware::new(LogLevel::Info));
        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: None,
        };

        // Create a middleware that returns an error
        struct ErrorMiddleware;
        #[async_trait::async_trait]
        impl McpMiddleware for ErrorMiddleware {
            fn name(&self) -> &'static str {
                "error"
            }

            async fn before_request(
                &self,
                _request: &CallToolRequest,
                _context: &mut MiddlewareContext,
            ) -> McpResult<MiddlewareResult> {
                Err(McpError::tool_not_found("middleware error"))
            }

            async fn after_request(
                &self,
                _request: &CallToolRequest,
                _result: &mut CallToolResult,
                _context: &mut MiddlewareContext,
            ) -> McpResult<MiddlewareResult> {
                Ok(MiddlewareResult::Continue)
            }

            async fn on_error(
                &self,
                _request: &CallToolRequest,
                _error: &McpError,
                _context: &mut MiddlewareContext,
            ) -> McpResult<MiddlewareResult> {
                Ok(MiddlewareResult::Continue)
            }
        }

        let chain = chain.add_middleware(ErrorMiddleware);

        let result = chain
            .execute(request, |_| async {
                Ok(CallToolResult {
                    content: vec![Content::Text {
                        text: "should not reach here".to_string(),
                    }],
                    is_error: false,
                })
            })
            .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, McpError::ToolNotFound { tool_name: _ }));
    }

    #[tokio::test]
    async fn test_middleware_chain_execution_with_on_error() {
        let chain = MiddlewareChain::new().add_middleware(LoggingMiddleware::new(LogLevel::Info));
        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: None,
        };

        // Create a middleware that handles errors
        struct ErrorHandlerMiddleware;
        #[async_trait::async_trait]
        impl McpMiddleware for ErrorHandlerMiddleware {
            fn name(&self) -> &'static str {
                "error_handler"
            }

            async fn before_request(
                &self,
                _request: &CallToolRequest,
                _context: &mut MiddlewareContext,
            ) -> McpResult<MiddlewareResult> {
                Ok(MiddlewareResult::Continue)
            }

            async fn after_request(
                &self,
                _request: &CallToolRequest,
                _result: &mut CallToolResult,
                _context: &mut MiddlewareContext,
            ) -> McpResult<MiddlewareResult> {
                Ok(MiddlewareResult::Continue)
            }

            async fn on_error(
                &self,
                _request: &CallToolRequest,
                _error: &McpError,
                _context: &mut MiddlewareContext,
            ) -> McpResult<MiddlewareResult> {
                Ok(MiddlewareResult::Stop(CallToolResult {
                    content: vec![Content::Text {
                        text: "error handled".to_string(),
                    }],
                    is_error: false,
                }))
            }
        }

        let chain = chain.add_middleware(ErrorHandlerMiddleware);

        let result = chain
            .execute(request, |_| async {
                Err(McpError::tool_not_found("test error"))
            })
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        let Content::Text { text } = &result.content[0];
        assert_eq!(text, "error handled");
    }

    #[tokio::test]
    async fn test_config_structs_creation() {
        let logging_config = LoggingConfig {
            enabled: true,
            level: "debug".to_string(),
        };
        let validation_config = ValidationConfig {
            enabled: true,
            strict_mode: true,
        };
        let performance_config = PerformanceConfig {
            enabled: true,
            slow_request_threshold_ms: 1000,
        };

        assert!(logging_config.enabled);
        assert_eq!(logging_config.level, "debug");
        assert!(validation_config.enabled);
        assert!(validation_config.strict_mode);
        assert!(performance_config.enabled);
        assert_eq!(performance_config.slow_request_threshold_ms, 1000);
    }

    #[tokio::test]
    async fn test_config_default() {
        let config = MiddlewareConfig::default();
        assert!(config.logging.enabled);
        assert_eq!(config.logging.level, "info");
        assert!(config.validation.enabled);
        assert!(!config.validation.strict_mode);
        assert!(config.performance.enabled);
        assert_eq!(config.performance.slow_request_threshold_ms, 1000);
    }

    #[tokio::test]
    async fn test_config_build_chain_with_disabled_middleware() {
        let config = MiddlewareConfig {
            logging: LoggingConfig {
                enabled: false,
                level: "debug".to_string(),
            },
            validation: ValidationConfig {
                enabled: false,
                strict_mode: true,
            },
            performance: PerformanceConfig {
                enabled: false,
                slow_request_threshold_ms: 1000,
            },
        };

        let chain = config.build_chain();
        assert!(chain.is_empty());
    }

    #[tokio::test]
    async fn test_config_build_chain_with_partial_middleware() {
        let config = MiddlewareConfig {
            logging: LoggingConfig {
                enabled: true,
                level: "debug".to_string(),
            },
            validation: ValidationConfig {
                enabled: false,
                strict_mode: true,
            },
            performance: PerformanceConfig {
                enabled: true,
                slow_request_threshold_ms: 1000,
            },
        };

        let chain = config.build_chain();
        assert!(!chain.is_empty());
        assert_eq!(chain.len(), 2); // Only logging and performance
    }

    #[tokio::test]
    async fn test_config_build_chain_with_invalid_log_level() {
        let config = MiddlewareConfig {
            logging: LoggingConfig {
                enabled: true,
                level: "invalid".to_string(),
            },
            validation: ValidationConfig {
                enabled: true,
                strict_mode: true,
            },
            performance: PerformanceConfig {
                enabled: true,
                slow_request_threshold_ms: 1000,
            },
        };

        let chain = config.build_chain();
        assert!(!chain.is_empty());
        // Should default to info level
    }
}

#[tokio::test]
async fn test_middleware_chain_execution_with_empty_middleware() {
    let chain = MiddlewareChain::new();
    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: Some(serde_json::json!({"param": "value"})),
    };

    let result = chain
        .execute(request, |_| async {
            Ok(CallToolResult {
                content: vec![Content::Text {
                    text: "Test response".to_string(),
                }],
                is_error: false,
            })
        })
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
}

#[tokio::test]
async fn test_middleware_chain_execution_with_multiple_middleware() {
    let chain = MiddlewareChain::new()
        .add_middleware(LoggingMiddleware::new(LogLevel::Info))
        .add_middleware(ValidationMiddleware::new(false))
        .add_middleware(PerformanceMiddleware::new(Duration::from_millis(100)));

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: Some(serde_json::json!({"param": "value"})),
    };

    let result = chain
        .execute(request, |_| async {
            Ok(CallToolResult {
                content: vec![Content::Text {
                    text: "Test response".to_string(),
                }],
                is_error: false,
            })
        })
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
}

#[tokio::test]
async fn test_middleware_chain_execution_with_middleware_stop() {
    struct StopMiddleware;
    #[async_trait::async_trait]
    impl McpMiddleware for StopMiddleware {
        fn name(&self) -> &'static str {
            "stop_middleware"
        }

        fn priority(&self) -> i32 {
            100
        }

        async fn before_request(
            &self,
            _request: &CallToolRequest,
            _context: &mut MiddlewareContext,
        ) -> McpResult<MiddlewareResult> {
            Ok(MiddlewareResult::Stop(CallToolResult {
                content: vec![Content::Text {
                    text: "Stopped by middleware".to_string(),
                }],
                is_error: false,
            }))
        }

        async fn after_request(
            &self,
            _request: &CallToolRequest,
            _result: &mut CallToolResult,
            _context: &mut MiddlewareContext,
        ) -> McpResult<MiddlewareResult> {
            Ok(MiddlewareResult::Continue)
        }

        async fn on_error(
            &self,
            _request: &CallToolRequest,
            _error: &McpError,
            _context: &mut MiddlewareContext,
        ) -> McpResult<MiddlewareResult> {
            Ok(MiddlewareResult::Continue)
        }
    }

    let chain = MiddlewareChain::new().add_middleware(StopMiddleware);

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    let result = chain
        .execute(request, |_| async {
            Ok(CallToolResult {
                content: vec![Content::Text {
                    text: "Should not reach here".to_string(),
                }],
                is_error: false,
            })
        })
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.is_error);
    let Content::Text { text } = &result.content[0];
    assert_eq!(text, "Stopped by middleware");
}

#[tokio::test]
async fn test_middleware_chain_execution_with_middleware_error() {
    struct ErrorMiddleware;
    #[async_trait::async_trait]
    impl McpMiddleware for ErrorMiddleware {
        fn name(&self) -> &'static str {
            "error_middleware"
        }

        fn priority(&self) -> i32 {
            100
        }

        async fn before_request(
            &self,
            _request: &CallToolRequest,
            _context: &mut MiddlewareContext,
        ) -> McpResult<MiddlewareResult> {
            Err(McpError::internal_error("Middleware error"))
        }

        async fn after_request(
            &self,
            _request: &CallToolRequest,
            _result: &mut CallToolResult,
            _context: &mut MiddlewareContext,
        ) -> McpResult<MiddlewareResult> {
            Ok(MiddlewareResult::Continue)
        }

        async fn on_error(
            &self,
            _request: &CallToolRequest,
            _error: &McpError,
            _context: &mut MiddlewareContext,
        ) -> McpResult<MiddlewareResult> {
            Ok(MiddlewareResult::Continue)
        }
    }

    let chain = MiddlewareChain::new().add_middleware(ErrorMiddleware);

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    let result = chain
        .execute(request, |_| async {
            Ok(CallToolResult {
                content: vec![Content::Text {
                    text: "Should not reach here".to_string(),
                }],
                is_error: false,
            })
        })
        .await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, McpError::InternalError { .. }));
}
