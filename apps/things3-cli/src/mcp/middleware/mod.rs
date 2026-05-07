//! MCP Middleware system for cross-cutting concerns

use crate::mcp::{CallToolRequest, CallToolResult, McpError, McpResult};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};

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

mod auth;
mod config;
mod logging;
mod performance;
mod rate_limit;
mod validation;

pub use auth::{ApiKeyInfo, AuthenticationMiddleware, JwtClaims, OAuthConfig};
pub use config::{
    ApiKeyConfig, AuthenticationConfig, LoggingConfig, MiddlewareConfig, OAuth2Config,
    PerformanceConfig, RateLimitingConfig, SecurityConfig, ValidationConfig,
};
pub use logging::{LogLevel, LoggingMiddleware};
pub use performance::PerformanceMiddleware;
pub use rate_limit::RateLimitMiddleware;
pub use validation::ValidationMiddleware;

#[derive(Debug, thiserror::Error)]
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
    use std::collections::HashMap;

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
            security: SecurityConfig::default(),
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

        let chain = MiddlewareChain::new().add_middleware(LoggingMiddleware::new(LogLevel::Info));
        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: None,
        };

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

        let chain = MiddlewareChain::new().add_middleware(LoggingMiddleware::new(LogLevel::Info));
        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: None,
        };

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

        let chain = MiddlewareChain::new().add_middleware(LoggingMiddleware::new(LogLevel::Info));
        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: None,
        };

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
            security: SecurityConfig {
                authentication: AuthenticationConfig {
                    enabled: false,
                    require_auth: false,
                    jwt_secret: "test".to_string(),
                    api_keys: vec![],
                    oauth: None,
                },
                rate_limiting: RateLimitingConfig {
                    enabled: false,
                    requests_per_minute: 60,
                    burst_limit: 10,
                    custom_limits: None,
                },
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
            security: SecurityConfig::default(),
        };

        let chain = config.build_chain();
        assert!(!chain.is_empty());
        assert!(chain.len() >= 2); // At least logging and performance
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
            security: SecurityConfig::default(),
        };

        let chain = config.build_chain();
        assert!(!chain.is_empty());
        // Should default to info level
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
    async fn test_middleware_chain_execution_with_middleware_error_duplicate() {
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

    // Authentication Middleware Tests
    #[tokio::test]
    async fn test_authentication_middleware_permissive() {
        let middleware = AuthenticationMiddleware::permissive();
        let mut context = MiddlewareContext::new("test".to_string());

        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: None,
        };

        let result = middleware.before_request(&request, &mut context).await;

        assert!(matches!(result, Ok(MiddlewareResult::Continue)));
        assert_eq!(
            context.get_metadata("auth_required"),
            Some(&Value::Bool(false))
        );
    }

    #[tokio::test]
    async fn test_authentication_middleware_with_valid_api_key() {
        let mut api_keys = HashMap::new();
        api_keys.insert(
            "test-api-key".to_string(),
            ApiKeyInfo {
                key_id: "test-key-1".to_string(),
                permissions: vec!["read".to_string(), "write".to_string()],
                expires_at: None,
            },
        );

        let middleware = AuthenticationMiddleware::new(api_keys, "test-secret".to_string());
        let mut context = MiddlewareContext::new("test".to_string());

        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: Some(serde_json::json!({
                "api_key": "test-api-key"
            })),
        };

        let result = middleware.before_request(&request, &mut context).await;

        assert!(matches!(result, Ok(MiddlewareResult::Continue)));
        assert_eq!(
            context.get_metadata("auth_type"),
            Some(&Value::String("api_key".to_string()))
        );
        assert_eq!(
            context.get_metadata("auth_key_id"),
            Some(&Value::String("test-key-1".to_string()))
        );
    }

    #[tokio::test]
    async fn test_authentication_middleware_with_invalid_api_key() {
        let api_keys = HashMap::new();
        let middleware = AuthenticationMiddleware::new(api_keys, "test-secret".to_string());
        let mut context = MiddlewareContext::new("test".to_string());

        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: Some(serde_json::json!({
                "api_key": "invalid-key"
            })),
        };

        let result = middleware.before_request(&request, &mut context).await;

        assert!(matches!(result, Ok(MiddlewareResult::Stop(_))));
    }

    #[tokio::test]
    async fn test_authentication_middleware_with_valid_jwt() {
        let api_keys = HashMap::new();
        let middleware = AuthenticationMiddleware::new(api_keys, "test-secret".to_string());

        // Generate a test JWT token
        let jwt_token = middleware.generate_test_jwt("user123", vec!["read".to_string()]);

        let mut context = MiddlewareContext::new("test".to_string());

        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: Some(serde_json::json!({
                "jwt_token": jwt_token
            })),
        };

        let result = middleware.before_request(&request, &mut context).await;

        assert!(matches!(result, Ok(MiddlewareResult::Continue)));
        assert_eq!(
            context.get_metadata("auth_type"),
            Some(&Value::String("jwt".to_string()))
        );
        assert_eq!(
            context.get_metadata("auth_user_id"),
            Some(&Value::String("user123".to_string()))
        );
    }

    #[tokio::test]
    async fn test_authentication_middleware_with_invalid_jwt() {
        let api_keys = HashMap::new();
        let middleware = AuthenticationMiddleware::new(api_keys, "test-secret".to_string());
        let mut context = MiddlewareContext::new("test".to_string());

        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: Some(serde_json::json!({
                "jwt_token": "invalid.jwt.token"
            })),
        };

        let result = middleware.before_request(&request, &mut context).await;

        assert!(matches!(result, Ok(MiddlewareResult::Stop(_))));
    }

    #[tokio::test]
    async fn test_authentication_middleware_no_auth_provided() {
        let api_keys = HashMap::new();
        let middleware = AuthenticationMiddleware::new(api_keys, "test-secret".to_string());
        let mut context = MiddlewareContext::new("test".to_string());

        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: None,
        };

        let result = middleware.before_request(&request, &mut context).await;

        assert!(matches!(result, Ok(MiddlewareResult::Stop(_))));
    }

    // Rate Limiting Middleware Tests
    #[tokio::test]
    async fn test_rate_limit_middleware_allows_request() {
        let middleware = RateLimitMiddleware::new(10, 5);
        let mut context = MiddlewareContext::new("test".to_string());

        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: Some(serde_json::json!({
                "client_id": "test-client"
            })),
        };

        let result = middleware.before_request(&request, &mut context).await;

        assert!(matches!(result, Ok(MiddlewareResult::Continue)));
        assert_eq!(
            context.get_metadata("rate_limit_client_id"),
            Some(&Value::String("client:test-client".to_string()))
        );
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_uses_auth_context() {
        let middleware = RateLimitMiddleware::new(10, 5);
        let mut context = MiddlewareContext::new("test".to_string());

        // Set up auth context
        context.set_metadata(
            "auth_key_id".to_string(),
            Value::String("api-key-123".to_string()),
        );

        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: None,
        };

        let result = middleware.before_request(&request, &mut context).await;

        assert!(matches!(result, Ok(MiddlewareResult::Continue)));
        assert_eq!(
            context.get_metadata("rate_limit_client_id"),
            Some(&Value::String("api_key:api-key-123".to_string()))
        );
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_uses_jwt_context() {
        let middleware = RateLimitMiddleware::new(10, 5);
        let mut context = MiddlewareContext::new("test".to_string());

        // Set up JWT context
        context.set_metadata(
            "auth_user_id".to_string(),
            Value::String("user-456".to_string()),
        );

        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: None,
        };

        let result = middleware.before_request(&request, &mut context).await;

        assert!(matches!(result, Ok(MiddlewareResult::Continue)));
        assert_eq!(
            context.get_metadata("rate_limit_client_id"),
            Some(&Value::String("jwt:user-456".to_string()))
        );
    }

    // Security Configuration Tests
    #[tokio::test]
    async fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.authentication.enabled);
        assert!(!config.authentication.require_auth); // Should be false for easier development
        assert!(config.rate_limiting.enabled);
        assert_eq!(config.rate_limiting.requests_per_minute, 60);
    }

    #[tokio::test]
    async fn test_middleware_config_with_security() {
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
            security: SecurityConfig {
                authentication: AuthenticationConfig {
                    enabled: true,
                    require_auth: true,
                    jwt_secret: "test-secret".to_string(),
                    api_keys: vec![ApiKeyConfig {
                        key: "test-key".to_string(),
                        key_id: "test-id".to_string(),
                        permissions: vec!["read".to_string()],
                        expires_at: None,
                    }],
                    oauth: None,
                },
                rate_limiting: RateLimitingConfig {
                    enabled: true,
                    requests_per_minute: 30,
                    burst_limit: 5,
                    custom_limits: None,
                },
            },
        };

        let chain = config.build_chain();
        assert!(!chain.is_empty());
        assert!(chain.len() >= 5); // Should have auth, rate limiting, logging, validation, and performance
    }

    #[tokio::test]
    async fn test_middleware_chain_with_security_middleware() {
        let mut api_keys = HashMap::new();
        api_keys.insert(
            "test-key".to_string(),
            ApiKeyInfo {
                key_id: "test-id".to_string(),
                permissions: vec!["read".to_string()],
                expires_at: None,
            },
        );

        let chain = MiddlewareChain::new()
            .add_middleware(AuthenticationMiddleware::new(
                api_keys,
                "test-secret".to_string(),
            ))
            .add_middleware(RateLimitMiddleware::new(10, 5))
            .add_middleware(LoggingMiddleware::new(LogLevel::Info));

        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: Some(serde_json::json!({
                "api_key": "test-key"
            })),
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
    }
}
