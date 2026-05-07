//! Performance monitoring middleware

use super::{McpMiddleware, MiddlewareContext, MiddlewareResult};
use crate::mcp::{CallToolRequest, CallToolResult, McpResult};
use std::time::Duration;

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
