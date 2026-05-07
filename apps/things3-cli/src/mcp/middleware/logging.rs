//! Logging middleware

use super::{McpMiddleware, MiddlewareContext, MiddlewareResult};
use crate::mcp::{CallToolRequest, CallToolResult, McpError, McpResult};

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

    pub(super) fn should_log(&self, level: LogLevel) -> bool {
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
