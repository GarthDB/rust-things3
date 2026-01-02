//! Integration tests for MCP middleware chain
//!
//! These tests verify that middleware components work correctly together
//! in realistic scenarios, complementing the unit tests in middleware.rs

use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use things3_cli::mcp::middleware::{
    AuthenticationMiddleware, LogLevel, LoggingMiddleware, MiddlewareChain, MiddlewareConfig,
    PerformanceMiddleware, RateLimitMiddleware, ValidationMiddleware,
};
use things3_cli::mcp::{CallToolRequest, CallToolResult, Content};

/// Helper to create a test handler that returns success
fn success_handler(
    _req: CallToolRequest,
) -> std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<CallToolResult, things3_cli::mcp::McpError>> + Send,
    >,
> {
    Box::pin(async move {
        Ok(CallToolResult {
            content: vec![Content::Text {
                text: "Success".to_string(),
            }],
            is_error: false,
        })
    })
}

/// Helper to create a test handler that returns an error
fn error_handler(
    _req: CallToolRequest,
) -> std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<CallToolResult, things3_cli::mcp::McpError>> + Send,
    >,
> {
    Box::pin(async move {
        Err(things3_cli::mcp::McpError::internal_error(
            "Test error".to_string(),
        ))
    })
}

// ============================================================================
// Middleware Chain Integration Tests
// ============================================================================

#[tokio::test]
async fn test_full_middleware_stack() {
    let chain = MiddlewareChain::new()
        .add_middleware(LoggingMiddleware::new(LogLevel::Debug))
        .add_middleware(ValidationMiddleware::lenient())
        .add_middleware(PerformanceMiddleware::with_threshold(Duration::from_secs(
            1,
        )));

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: Some(json!({"param": "value"})),
    };

    let result = chain.execute(request, success_handler).await;
    assert!(result.is_ok());
    assert!(!result.unwrap().is_error);
}

#[tokio::test]
async fn test_middleware_chain_with_validation_strict() {
    let chain = MiddlewareChain::new()
        .add_middleware(ValidationMiddleware::strict())
        .add_middleware(LoggingMiddleware::info());

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: Some(json!({"param": "value"})),
    };

    let result = chain.execute(request, success_handler).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_middleware_chain_priority_ordering() {
    // Create middlewares with different priorities
    let logging = LoggingMiddleware::new(LogLevel::Info); // priority 100
    let validation = ValidationMiddleware::lenient(); // priority 10
    let performance = PerformanceMiddleware::with_threshold(Duration::from_millis(100)); // priority 50

    let chain = MiddlewareChain::new()
        .add_middleware(logging)
        .add_middleware(validation)
        .add_middleware(performance);

    // Chain should have 3 middlewares sorted by priority
    assert_eq!(chain.len(), 3);

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    let result = chain.execute(request, success_handler).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_middleware_chain_handles_handler_error() {
    let chain = MiddlewareChain::new()
        .add_middleware(LoggingMiddleware::info())
        .add_middleware(ValidationMiddleware::lenient());

    let request = CallToolRequest {
        name: "failing_tool".to_string(),
        arguments: None,
    };

    let result = chain.execute(request, error_handler).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_middleware_chain_empty_arguments() {
    let chain = MiddlewareChain::new()
        .add_middleware(ValidationMiddleware::lenient())
        .add_middleware(LoggingMiddleware::debug());

    let request = CallToolRequest {
        name: "tool_without_args".to_string(),
        arguments: None,
    };

    let result = chain.execute(request, success_handler).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_middleware_chain_complex_arguments() {
    let chain = MiddlewareChain::new()
        .add_middleware(ValidationMiddleware::strict())
        .add_middleware(LoggingMiddleware::info());

    let request = CallToolRequest {
        name: "complex_tool".to_string(),
        arguments: Some(json!({
            "nested": {
                "array": [1, 2, 3],
                "object": {"key": "value"}
            },
            "string": "test",
            "number": 42,
            "boolean": true
        })),
    };

    let result = chain.execute(request, success_handler).await;
    assert!(result.is_ok());
}

// ============================================================================
// Performance Middleware Integration Tests
// ============================================================================

#[tokio::test]
async fn test_performance_middleware_fast_request() {
    let chain = MiddlewareChain::new().add_middleware(PerformanceMiddleware::with_threshold(
        Duration::from_millis(100),
    ));

    let request = CallToolRequest {
        name: "fast_tool".to_string(),
        arguments: None,
    };

    let result = chain.execute(request, success_handler).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_performance_middleware_with_slow_handler() {
    let chain = MiddlewareChain::new().add_middleware(PerformanceMiddleware::with_threshold(
        Duration::from_millis(10),
    ));

    let slow_handler = |_req: CallToolRequest| {
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok(CallToolResult {
                content: vec![Content::Text {
                    text: "Slow response".to_string(),
                }],
                is_error: false,
            })
        })
    };

    let request = CallToolRequest {
        name: "slow_tool".to_string(),
        arguments: None,
    };

    let result = chain.execute(request, slow_handler).await;
    assert!(result.is_ok());
    // Performance middleware should log but not fail the request
}

// ============================================================================
// Logging Middleware Integration Tests
// ============================================================================

#[tokio::test]
async fn test_logging_middleware_all_levels() {
    let levels = vec![
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
    ];

    for level in levels {
        let chain = MiddlewareChain::new().add_middleware(LoggingMiddleware::new(level));

        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: Some(json!({"test": "data"})),
        };

        let result = chain.execute(request, success_handler).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_logging_middleware_with_error() {
    let chain = MiddlewareChain::new().add_middleware(LoggingMiddleware::error());

    let request = CallToolRequest {
        name: "failing_tool".to_string(),
        arguments: None,
    };

    let result = chain.execute(request, error_handler).await;
    assert!(result.is_err());
    // Logging middleware should log the error but not suppress it
}

// ============================================================================
// Validation Middleware Integration Tests
// ============================================================================

#[tokio::test]
async fn test_validation_middleware_lenient_mode() {
    let chain = MiddlewareChain::new().add_middleware(ValidationMiddleware::lenient());

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: Some(json!({"any": "data"})),
    };

    let result = chain.execute(request, success_handler).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validation_middleware_strict_mode() {
    let chain = MiddlewareChain::new().add_middleware(ValidationMiddleware::strict());

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: Some(json!({"param": "value"})),
    };

    let result = chain.execute(request, success_handler).await;
    assert!(result.is_ok());
}

// ============================================================================
// Security Middleware Integration Tests
// ============================================================================

#[tokio::test]
async fn test_authentication_middleware_permissive() {
    let chain = MiddlewareChain::new().add_middleware(AuthenticationMiddleware::permissive());

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    let result = chain.execute(request, success_handler).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rate_limit_middleware_basic() {
    let chain = MiddlewareChain::new().add_middleware(RateLimitMiddleware::new(100, 10)); // 100 req/min, burst 10

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    // Should allow first request
    let result = chain.execute(request.clone(), success_handler).await;
    assert!(result.is_ok());

    // Should allow subsequent requests within limit
    for _ in 0..5 {
        let result = chain.execute(request.clone(), success_handler).await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Middleware Config Integration Tests
// ============================================================================

#[tokio::test]
async fn test_middleware_config_default() {
    let config = MiddlewareConfig::default();
    let chain = config.build_chain();

    // Default config should enable some middleware
    assert!(!chain.is_empty());

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    let result = chain.execute(request, success_handler).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_middleware_config_all_enabled() {
    let config = MiddlewareConfig {
        logging: things3_cli::mcp::middleware::LoggingConfig {
            enabled: true,
            level: "debug".to_string(),
        },
        validation: things3_cli::mcp::middleware::ValidationConfig {
            enabled: true,
            strict_mode: false,
        },
        performance: things3_cli::mcp::middleware::PerformanceConfig {
            enabled: true,
            slow_request_threshold_ms: 1000,
        },
        security: things3_cli::mcp::middleware::SecurityConfig::default(),
    };

    let chain = config.build_chain();
    assert!(!chain.is_empty());

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: Some(json!({"test": "data"})),
    };

    let result = chain.execute(request, success_handler).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_middleware_config_all_disabled() {
    let config = MiddlewareConfig {
        logging: things3_cli::mcp::middleware::LoggingConfig {
            enabled: false,
            level: "info".to_string(),
        },
        validation: things3_cli::mcp::middleware::ValidationConfig {
            enabled: false,
            strict_mode: false,
        },
        performance: things3_cli::mcp::middleware::PerformanceConfig {
            enabled: false,
            slow_request_threshold_ms: 1000,
        },
        security: things3_cli::mcp::middleware::SecurityConfig {
            authentication: things3_cli::mcp::middleware::AuthenticationConfig {
                enabled: false,
                require_auth: false,
                jwt_secret: "test".to_string(),
                api_keys: vec![],
                oauth: None,
            },
            rate_limiting: things3_cli::mcp::middleware::RateLimitingConfig {
                enabled: false,
                requests_per_minute: 60,
                burst_limit: 10,
                custom_limits: None,
            },
        },
    };

    let chain = config.build_chain();
    assert!(chain.is_empty());

    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };

    let result = chain.execute(request, success_handler).await;
    assert!(result.is_ok());
}

// ============================================================================
// Complex Scenario Tests
// ============================================================================

#[tokio::test]
async fn test_middleware_chain_with_multiple_requests() {
    let chain = MiddlewareChain::new()
        .add_middleware(LoggingMiddleware::info())
        .add_middleware(ValidationMiddleware::lenient())
        .add_middleware(PerformanceMiddleware::with_threshold(
            Duration::from_millis(100),
        ));

    // Execute multiple requests through the same chain
    for i in 0..10 {
        let request = CallToolRequest {
            name: format!("tool_{}", i),
            arguments: Some(json!({"index": i})),
        };

        let result = chain.execute(request, success_handler).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_middleware_chain_concurrent_requests() {
    let chain = Arc::new(
        MiddlewareChain::new()
            .add_middleware(LoggingMiddleware::info())
            .add_middleware(ValidationMiddleware::lenient()),
    );

    let mut handles = vec![];

    // Spawn multiple concurrent requests
    for i in 0..5 {
        let chain_clone = Arc::clone(&chain);
        let handle = tokio::spawn(async move {
            let request = CallToolRequest {
                name: format!("concurrent_tool_{}", i),
                arguments: Some(json!({"index": i})),
            };

            chain_clone.execute(request, success_handler).await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_middleware_chain_mixed_success_and_error() {
    let chain = MiddlewareChain::new()
        .add_middleware(LoggingMiddleware::info())
        .add_middleware(ValidationMiddleware::lenient());

    // Success request
    let success_request = CallToolRequest {
        name: "success_tool".to_string(),
        arguments: None,
    };

    let result = chain.execute(success_request, success_handler).await;
    assert!(result.is_ok());

    // Error request
    let error_request = CallToolRequest {
        name: "error_tool".to_string(),
        arguments: None,
    };

    let result = chain.execute(error_request, error_handler).await;
    assert!(result.is_err());
}
