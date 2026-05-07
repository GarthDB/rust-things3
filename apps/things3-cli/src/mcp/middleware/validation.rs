//! Validation middleware

use super::{McpMiddleware, MiddlewareContext, MiddlewareResult};
use crate::mcp::{CallToolRequest, McpError, McpResult};

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
