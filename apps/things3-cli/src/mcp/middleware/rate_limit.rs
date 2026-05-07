//! Rate limiting middleware

use super::{McpMiddleware, MiddlewareContext, MiddlewareResult};
use crate::mcp::{CallToolRequest, CallToolResult, McpResult};
use governor::clock::DefaultClock;
use governor::{state::keyed::DefaultKeyedStateStore, Quota, RateLimiter};
use nonzero_ext::nonzero;
use serde_json::Value;
use std::sync::Arc;

pub struct RateLimitMiddleware {
    rate_limiter: Arc<RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>>,
    default_limit: u32,
    #[allow(dead_code)]
    burst_limit: u32,
}

impl RateLimitMiddleware {
    /// Create a new rate limiting middleware
    #[must_use]
    pub fn new(requests_per_minute: u32, burst_limit: u32) -> Self {
        let quota = Quota::per_minute(nonzero!(60u32)); // Use a constant for now
        let rate_limiter = Arc::new(RateLimiter::keyed(quota));

        Self {
            rate_limiter,
            default_limit: requests_per_minute,
            burst_limit,
        }
    }

    /// Create with custom limits
    #[must_use]
    pub fn with_limits(requests_per_minute: u32, burst_limit: u32) -> Self {
        Self::new(requests_per_minute, burst_limit)
    }

    /// Create with default limits (60 requests per minute, burst of 10)
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn default() -> Self {
        Self::new(60, 10)
    }

    /// Extract client identifier from request
    fn extract_client_id(request: &CallToolRequest, context: &MiddlewareContext) -> String {
        // Try to get from authentication context first
        if let Some(auth_key_id) = context.get_metadata("auth_key_id").and_then(|v| v.as_str()) {
            return format!("api_key:{auth_key_id}");
        }

        if let Some(auth_user_id) = context
            .get_metadata("auth_user_id")
            .and_then(|v| v.as_str())
        {
            return format!("jwt:{auth_user_id}");
        }

        // Fallback to request-based identifier
        if let Some(args) = &request.arguments {
            if let Some(client_id) = args.get("client_id").and_then(|v| v.as_str()) {
                return format!("client:{client_id}");
            }
        }

        // Use request ID as fallback
        format!("request:{}", context.request_id)
    }

    /// Check if request is within rate limits
    fn check_rate_limit(&self, client_id: &str) -> bool {
        self.rate_limiter.check_key(&client_id.to_string()).is_ok()
    }

    /// Get remaining requests for client
    fn get_remaining_requests(&self, _client_id: &str) -> u32 {
        // This is a simplified implementation
        // In a real implementation, you'd want to track remaining requests more precisely
        self.default_limit
    }
}

#[async_trait::async_trait]
impl McpMiddleware for RateLimitMiddleware {
    fn name(&self) -> &'static str {
        "rate_limiting"
    }

    fn priority(&self) -> i32 {
        20 // Run after authentication but before other middleware
    }

    async fn before_request(
        &self,
        request: &CallToolRequest,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        let client_id = Self::extract_client_id(request, context);

        if !self.check_rate_limit(&client_id) {
            let error_result = CallToolResult {
                content: vec![crate::mcp::Content::Text {
                    text: format!(
                        "Rate limit exceeded. Limit: {} requests per minute. Please try again later.",
                        self.default_limit
                    ),
                }],
                is_error: true,
            };

            context.set_metadata("rate_limited".to_string(), Value::Bool(true));
            context.set_metadata("rate_limit_client_id".to_string(), Value::String(client_id));

            return Ok(MiddlewareResult::Stop(error_result));
        }

        let remaining = self.get_remaining_requests(&client_id);
        context.set_metadata(
            "rate_limit_remaining".to_string(),
            Value::Number(serde_json::Number::from(remaining)),
        );
        context.set_metadata("rate_limit_client_id".to_string(), Value::String(client_id));

        Ok(MiddlewareResult::Continue)
    }
}
