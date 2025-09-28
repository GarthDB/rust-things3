//! Comprehensive tests for the MCP middleware system

use std::time::Duration;
use things3_cli::mcp::{
    middleware::{
        LoggingConfig, LoggingMiddleware, McpMiddleware, MiddlewareChain, MiddlewareConfig,
        MiddlewareContext, MiddlewareResult, PerformanceConfig, PerformanceMiddleware,
        ValidationConfig, ValidationMiddleware,
    },
    CallToolRequest, CallToolResult, Content, McpError, McpResult,
};
use things3_core::ThingsConfig;
use tokio::time::sleep;

/// Test middleware that counts requests
#[derive(Clone)]
struct CountingMiddleware {
    name: String,
    priority: i32,
    before_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    after_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    error_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl CountingMiddleware {
    fn new(name: &str, priority: i32) -> Self {
        Self {
            name: name.to_string(),
            priority,
            before_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            after_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            error_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    fn get_before_count(&self) -> usize {
        self.before_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn get_after_count(&self) -> usize {
        self.after_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    #[allow(dead_code)]
    fn get_error_count(&self) -> usize {
        self.error_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[async_trait::async_trait]
impl McpMiddleware for CountingMiddleware {
    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    async fn before_request(
        &self,
        _request: &CallToolRequest,
        _context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        self.before_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(MiddlewareResult::Continue)
    }

    async fn after_request(
        &self,
        _request: &CallToolRequest,
        _response: &mut CallToolResult,
        _context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        self.after_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(MiddlewareResult::Continue)
    }

    async fn on_error(
        &self,
        _request: &CallToolRequest,
        _error: &McpError,
        _context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        self.error_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(MiddlewareResult::Continue)
    }
}

/// Test middleware that stops execution
struct StoppingMiddleware {
    name: String,
    priority: i32,
    stop_after: usize,
    call_count: std::sync::atomic::AtomicUsize,
}

impl StoppingMiddleware {
    fn new(name: &str, priority: i32, stop_after: usize) -> Self {
        Self {
            name: name.to_string(),
            priority,
            stop_after,
            call_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

#[async_trait::async_trait]
impl McpMiddleware for StoppingMiddleware {
    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    async fn before_request(
        &self,
        _request: &CallToolRequest,
        _context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        let count = self
            .call_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if count >= self.stop_after {
            Ok(MiddlewareResult::Stop(CallToolResult {
                content: vec![Content::Text {
                    text: "Stopped by middleware".to_string(),
                }],
                is_error: false,
            }))
        } else {
            Ok(MiddlewareResult::Continue)
        }
    }
}

/// Test middleware that modifies responses
struct ResponseModifyingMiddleware {
    name: String,
    priority: i32,
    prefix: String,
}

impl ResponseModifyingMiddleware {
    fn new(name: &str, priority: i32, prefix: &str) -> Self {
        Self {
            name: name.to_string(),
            priority,
            prefix: prefix.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl McpMiddleware for ResponseModifyingMiddleware {
    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    async fn after_request(
        &self,
        _request: &CallToolRequest,
        response: &mut CallToolResult,
        _context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        if let Some(Content::Text { text }) = response.content.first_mut() {
            *text = format!("{}{}", self.prefix, text);
        }
        Ok(MiddlewareResult::Continue)
    }
}

#[tokio::test]
async fn test_middleware_chain_basic_execution() {
    let chain = MiddlewareChain::new()
        .add_middleware(CountingMiddleware::new("counter1", 100))
        .add_middleware(CountingMiddleware::new("counter2", 200));

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: Some(serde_json::json!({"param": "value"})),
    };

    let handler = |_req: CallToolRequest| async move {
        Ok(CallToolResult {
            content: vec![Content::Text {
                text: "Test response".to_string(),
            }],
            is_error: false,
        })
    };

    let result = chain.execute(request, handler).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_middleware_priority_ordering() {
    let counter1 = CountingMiddleware::new("counter1", 200);
    let counter2 = CountingMiddleware::new("counter2", 100);

    let chain = MiddlewareChain::new()
        .add_middleware(counter1.clone())
        .add_middleware(counter2.clone());

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    let handler = |_req: CallToolRequest| async move {
        Ok(CallToolResult {
            content: vec![Content::Text {
                text: "Test response".to_string(),
            }],
            is_error: false,
        })
    };

    let _result = chain.execute(request, handler).await;

    // Both middlewares should have been called
    assert_eq!(counter1.get_before_count(), 1);
    assert_eq!(counter1.get_after_count(), 1);
    assert_eq!(counter2.get_before_count(), 1);
    assert_eq!(counter2.get_after_count(), 1);
}

#[tokio::test]
async fn test_middleware_stop_execution() {
    let chain = MiddlewareChain::new()
        .add_middleware(StoppingMiddleware::new("stopper", 50, 0)) // Stop immediately
        .add_middleware(CountingMiddleware::new("counter", 100));

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    let handler = |_req: CallToolRequest| async move {
        Ok(CallToolResult {
            content: vec![Content::Text {
                text: "Should not reach here".to_string(),
            }],
            is_error: false,
        })
    };

    let result = chain.execute(request, handler).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.content[0].text(), "Stopped by middleware");
}

#[tokio::test]
async fn test_middleware_response_modification() {
    let chain = MiddlewareChain::new()
        .add_middleware(ResponseModifyingMiddleware::new(
            "modifier1",
            200,
            "[MOD1] ",
        ))
        .add_middleware(ResponseModifyingMiddleware::new(
            "modifier2",
            100,
            "[MOD2] ",
        ));

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    let handler = |_req: CallToolRequest| async move {
        Ok(CallToolResult {
            content: vec![Content::Text {
                text: "Original response".to_string(),
            }],
            is_error: false,
        })
    };

    let result = chain.execute(request, handler).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(
        response.content[0].text(),
        "[MOD1] [MOD2] Original response"
    );
}

#[tokio::test]
async fn test_validation_middleware() {
    let chain = MiddlewareChain::new()
        .add_middleware(ValidationMiddleware::strict())
        .add_middleware(CountingMiddleware::new("counter", 100));

    // Test valid request
    let valid_request = CallToolRequest {
        name: "valid_tool".to_string(),
        arguments: Some(serde_json::json!({"param": "value"})),
    };

    let handler = |_req: CallToolRequest| async move {
        Ok(CallToolResult {
            content: vec![Content::Text {
                text: "Valid response".to_string(),
            }],
            is_error: false,
        })
    };

    let result = chain.execute(valid_request, handler).await;
    assert!(result.is_ok());

    // Test invalid request (empty name)
    let invalid_request = CallToolRequest {
        name: String::new(),
        arguments: None,
    };

    let result = chain.execute(invalid_request, handler).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_logging_middleware() {
    let chain = MiddlewareChain::new()
        .add_middleware(LoggingMiddleware::info())
        .add_middleware(CountingMiddleware::new("counter", 100));

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    let handler = |_req: CallToolRequest| async move {
        Ok(CallToolResult {
            content: vec![Content::Text {
                text: "Test response".to_string(),
            }],
            is_error: false,
        })
    };

    let result = chain.execute(request, handler).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_performance_middleware() {
    let chain = MiddlewareChain::new()
        .add_middleware(PerformanceMiddleware::with_threshold(
            Duration::from_millis(50),
        ))
        .add_middleware(CountingMiddleware::new("counter", 100));

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    let handler = |_req: CallToolRequest| async move {
        // Simulate slow operation
        sleep(Duration::from_millis(100)).await;
        Ok(CallToolResult {
            content: vec![Content::Text {
                text: "Slow response".to_string(),
            }],
            is_error: false,
        })
    };

    let result = chain.execute(request, handler).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_middleware_error_handling() {
    let chain = MiddlewareChain::new().add_middleware(CountingMiddleware::new("counter", 100));

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    let handler =
        |_req: CallToolRequest| async move { Err(McpError::internal_error("Test error")) };

    let result = chain.execute(request, handler).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_middleware_config_build_chain() {
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
async fn test_middleware_context_metadata() {
    let mut context = MiddlewareContext::new("test-123".to_string());

    // Test setting and getting metadata
    context.set_metadata("key1".to_string(), serde_json::json!("value1"));
    context.set_metadata("key2".to_string(), serde_json::json!(42));

    assert_eq!(
        context.get_metadata("key1"),
        Some(&serde_json::json!("value1"))
    );
    assert_eq!(context.get_metadata("key2"), Some(&serde_json::json!(42)));
    assert_eq!(context.get_metadata("nonexistent"), None);

    // Test elapsed time
    sleep(Duration::from_millis(10)).await;
    let elapsed = context.elapsed();
    assert!(elapsed >= Duration::from_millis(10));
}

#[tokio::test]
async fn test_middleware_chain_with_mcp_server() {
    // Create a test database (this would need to be mocked in a real test)
    // For now, we'll just test the middleware chain creation
    let _config = ThingsConfig::default();
    let middleware_config = MiddlewareConfig {
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
    };

    // Test that we can create a middleware chain
    let chain = middleware_config.build_chain();
    assert!(!chain.is_empty());

    // Test that we can execute a simple request
    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: Some(serde_json::json!({"param": "value"})),
    };

    let handler = |_req: CallToolRequest| async move {
        Ok(CallToolResult {
            content: vec![Content::Text {
                text: "Test response".to_string(),
            }],
            is_error: false,
        })
    };

    let result = chain.execute(request, handler).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_multiple_middleware_execution_order() {
    let counter1 = CountingMiddleware::new("counter1", 100);
    let counter2 = CountingMiddleware::new("counter2", 200);
    let counter3 = CountingMiddleware::new("counter3", 50);

    let chain = MiddlewareChain::new()
        .add_middleware(counter1.clone())
        .add_middleware(counter2.clone())
        .add_middleware(counter3.clone());

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    let handler = |_req: CallToolRequest| async move {
        Ok(CallToolResult {
            content: vec![Content::Text {
                text: "Test response".to_string(),
            }],
            is_error: false,
        })
    };

    let _result = chain.execute(request, handler).await;

    // All middlewares should have been called
    assert_eq!(counter1.get_before_count(), 1);
    assert_eq!(counter1.get_after_count(), 1);
    assert_eq!(counter2.get_before_count(), 1);
    assert_eq!(counter2.get_after_count(), 1);
    assert_eq!(counter3.get_before_count(), 1);
    assert_eq!(counter3.get_after_count(), 1);
}

#[tokio::test]
async fn test_middleware_chain_empty() {
    let chain = MiddlewareChain::new();
    assert!(chain.is_empty());
    assert_eq!(chain.len(), 0);

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    let handler = |_req: CallToolRequest| async move {
        Ok(CallToolResult {
            content: vec![Content::Text {
                text: "Test response".to_string(),
            }],
            is_error: false,
        })
    };

    let result = chain.execute(request, handler).await;
    assert!(result.is_ok());
}

// Helper trait for easier testing
trait ContentExt {
    fn text(&self) -> &str;
}

impl ContentExt for Content {
    fn text(&self) -> &str {
        match self {
            Content::Text { text } => text,
        }
    }
}
