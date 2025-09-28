# MCP Middleware System Usage Examples

This document demonstrates how to use the MCP middleware system for cross-cutting concerns in the Things 3 CLI.

## Overview

The MCP middleware system allows you to intercept and control server operations without modifying core protocol code. It provides hooks for request processing, response handling, and error management.

## Basic Usage

### Creating a Custom Middleware

```rust
use things3_cli::mcp::middleware::{McpMiddleware, MiddlewareContext, MiddlewareResult};
use things3_cli::mcp::{CallToolRequest, CallToolResult, McpError, McpResult};

struct CustomMiddleware {
    name: String,
}

#[async_trait::async_trait]
impl McpMiddleware for CustomMiddleware {
    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> i32 {
        50 // Lower numbers execute first
    }

    async fn before_request(
        &self,
        request: &CallToolRequest,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        println!("Processing request: {}", request.name);
        context.set_metadata("custom_data".to_string(), serde_json::json!("value"));
        Ok(MiddlewareResult::Continue)
    }

    async fn after_request(
        &self,
        request: &CallToolRequest,
        response: &mut CallToolResult,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        println!("Completed request: {}", request.name);
        Ok(MiddlewareResult::Continue)
    }

    async fn on_error(
        &self,
        request: &CallToolRequest,
        error: &McpError,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        println!("Error in request {}: {}", request.name, error);
        Ok(MiddlewareResult::Continue)
    }
}
```

### Using Built-in Middleware

```rust
use things3_cli::mcp::middleware::{
    LoggingMiddleware, LogLevel, ValidationMiddleware, PerformanceMiddleware,
    MiddlewareChain, MiddlewareConfig
};

// Create a middleware chain with built-in middleware
let chain = MiddlewareChain::new()
    .add(LoggingMiddleware::info())
    .add(ValidationMiddleware::strict())
    .add(PerformanceMiddleware::with_threshold(Duration::from_millis(1000)));

// Or use configuration
let config = MiddlewareConfig {
    enable_logging: true,
    log_level: "debug".to_string(),
    enable_validation: true,
    strict_validation: true,
    enable_performance: true,
    slow_request_threshold_ms: 500,
    ..Default::default()
};

let chain = config.build_chain();
```

### Creating an MCP Server with Middleware

```rust
use things3_cli::mcp::{ThingsMcpServer, MiddlewareConfig};
use things3_core::{ThingsDatabase, ThingsConfig};

// Create server with default middleware
let db = ThingsDatabase::new("path/to/database.sqlite")?;
let config = ThingsConfig::default();
let server = ThingsMcpServer::new(db, config);

// Create server with custom middleware configuration
let middleware_config = MiddlewareConfig {
    enable_logging: true,
    log_level: "info".to_string(),
    enable_validation: true,
    strict_validation: false,
    enable_performance: true,
    slow_request_threshold_ms: 1000,
    ..Default::default()
};

let server = ThingsMcpServer::with_middleware_config(db, config, middleware_config);
```

## Middleware Execution Flow

1. **before_request**: Called before the main handler executes
2. **Main Handler**: The actual tool/resource/prompt handler
3. **after_request**: Called after the main handler (if successful)
4. **on_error**: Called if an error occurs during processing

## Middleware Results

Middleware can return three types of results:

- `MiddlewareResult::Continue`: Continue to the next middleware or handler
- `MiddlewareResult::Stop(result)`: Stop execution and return this result
- `MiddlewareResult::Error(error)`: Stop execution with this error

## Built-in Middleware

### LoggingMiddleware

Provides request/response logging with configurable log levels.

```rust
let logging = LoggingMiddleware::debug(); // or info(), warn(), error()
```

### ValidationMiddleware

Validates incoming requests with optional strict mode.

```rust
let validation = ValidationMiddleware::strict(); // or lenient()
```

### PerformanceMiddleware

Monitors request performance and logs slow requests.

```rust
let performance = PerformanceMiddleware::with_threshold(Duration::from_millis(500));
```

## Advanced Examples

### Custom Authentication Middleware

```rust
struct AuthMiddleware {
    api_key: String,
}

#[async_trait::async_trait]
impl McpMiddleware for AuthMiddleware {
    fn name(&self) -> &str {
        "auth"
    }

    fn priority(&self) -> i32 {
        10 // High priority (low number)
    }

    async fn before_request(
        &self,
        request: &CallToolRequest,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        // Check for API key in request arguments
        if let Some(args) = &request.arguments {
            if let Some(key) = args.get("api_key").and_then(|v| v.as_str()) {
                if key == self.api_key {
                    return Ok(MiddlewareResult::Continue);
                }
            }
        }
        
        Ok(MiddlewareResult::Error(McpError::validation_error("Invalid API key")))
    }
}
```

### Rate Limiting Middleware

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

struct RateLimitMiddleware {
    requests_per_minute: usize,
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

#[async_trait::async_trait]
impl McpMiddleware for RateLimitMiddleware {
    fn name(&self) -> &str {
        "rate_limit"
    }

    fn priority(&self) -> i32 {
        20
    }

    async fn before_request(
        &self,
        request: &CallToolRequest,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        let client_id = context.request_id.clone(); // Use request ID as client identifier
        let now = Instant::now();
        
        let mut requests = self.requests.lock().await;
        let client_requests = requests.entry(client_id).or_insert_with(Vec::new);
        
        // Remove requests older than 1 minute
        client_requests.retain(|&time| now.duration_since(time).as_secs() < 60);
        
        if client_requests.len() >= self.requests_per_minute {
            return Ok(MiddlewareResult::Error(
                McpError::validation_error("Rate limit exceeded")
            ));
        }
        
        client_requests.push(now);
        Ok(MiddlewareResult::Continue)
    }
}
```

## Testing Middleware

```rust
#[tokio::test]
async fn test_custom_middleware() {
    let middleware = CustomMiddleware {
        name: "test".to_string(),
    };
    
    let chain = MiddlewareChain::new().add(middleware);
    
    let request = CallToolRequest {
        name: "test_tool".to_string(),
        arguments: None,
    };
    
    let handler = |req: CallToolRequest| async move {
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
```

## Configuration

The middleware system can be configured through the `MiddlewareConfig` struct:

```rust
let config = MiddlewareConfig {
    enable_logging: true,
    log_level: "info".to_string(),
    enable_validation: true,
    strict_validation: false,
    enable_performance: true,
    slow_request_threshold_ms: 1000,
};
```

This configuration will create a middleware chain with:
- Logging middleware at INFO level
- Validation middleware in lenient mode
- Performance monitoring with 1-second threshold

## Best Practices

1. **Priority Ordering**: Use lower numbers for higher priority middleware (e.g., authentication should be 10, logging should be 100)

2. **Error Handling**: Always handle errors gracefully in middleware

3. **Performance**: Keep middleware lightweight to avoid impacting request performance

4. **Context Usage**: Use the middleware context to pass data between middleware

5. **Testing**: Write comprehensive tests for custom middleware

6. **Documentation**: Document the purpose and behavior of custom middleware

## Integration with MCP Server

The middleware system is automatically integrated into the MCP server's `call_tool` method. All tool calls will go through the configured middleware chain, providing consistent cross-cutting functionality across all MCP operations.
