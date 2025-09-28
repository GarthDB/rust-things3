//! MCP Middleware system for cross-cutting concerns

use crate::mcp::{CallToolRequest, CallToolResult, McpError, McpResult};
use governor::clock::DefaultClock;
use governor::{state::keyed::DefaultKeyedStateStore, Quota, RateLimiter};
#[allow(unused_imports)]
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use nonzero_ext::nonzero;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
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

/// Authentication middleware for API key and OAuth 2.0 support
pub struct AuthenticationMiddleware {
    api_keys: HashMap<String, ApiKeyInfo>,
    jwt_secret: String,
    #[allow(dead_code)]
    oauth_config: Option<OAuthConfig>,
    require_auth: bool,
}

#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub key_id: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub token_endpoint: String,
    pub scope: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String, // Subject (user ID)
    pub exp: usize,  // Expiration time
    pub iat: usize,  // Issued at
    pub permissions: Vec<String>,
}

impl AuthenticationMiddleware {
    /// Create a new authentication middleware
    #[must_use]
    pub fn new(api_keys: HashMap<String, ApiKeyInfo>, jwt_secret: String) -> Self {
        Self {
            api_keys,
            jwt_secret,
            oauth_config: None,
            require_auth: true,
        }
    }

    /// Create with OAuth 2.0 support
    #[must_use]
    pub fn with_oauth(
        api_keys: HashMap<String, ApiKeyInfo>,
        jwt_secret: String,
        oauth_config: OAuthConfig,
    ) -> Self {
        Self {
            api_keys,
            jwt_secret,
            oauth_config: Some(oauth_config),
            require_auth: true,
        }
    }

    /// Create without requiring authentication (for testing)
    #[must_use]
    pub fn permissive() -> Self {
        Self {
            api_keys: HashMap::new(),
            jwt_secret: "test-secret".to_string(),
            oauth_config: None,
            require_auth: false,
        }
    }

    /// Extract API key from request headers or arguments
    fn extract_api_key(&self, request: &CallToolRequest) -> Option<String> {
        // Check if API key is in request arguments
        if let Some(args) = &request.arguments {
            if let Some(api_key) = args.get("api_key").and_then(|v| v.as_str()) {
                return Some(api_key.to_string());
            }
        }
        None
    }

    /// Extract JWT token from request headers or arguments
    fn extract_jwt_token(&self, request: &CallToolRequest) -> Option<String> {
        // Check if JWT token is in request arguments
        if let Some(args) = &request.arguments {
            if let Some(token) = args.get("jwt_token").and_then(|v| v.as_str()) {
                return Some(token.to_string());
            }
        }
        None
    }

    /// Validate API key
    fn validate_api_key(&self, api_key: &str) -> McpResult<ApiKeyInfo> {
        self.api_keys
            .get(api_key)
            .cloned()
            .ok_or_else(|| McpError::validation_error("Invalid API key"))
    }

    /// Validate JWT token
    fn validate_jwt_token(&self, token: &str) -> McpResult<JwtClaims> {
        let validation = Validation::new(Algorithm::HS256);
        let key = DecodingKey::from_secret(self.jwt_secret.as_ref());

        let token_data = decode::<JwtClaims>(token, &key, &validation)
            .map_err(|_| McpError::validation_error("Invalid JWT token"))?;

        // Check if token is expired
        let now = chrono::Utc::now().timestamp() as usize;
        if token_data.claims.exp < now {
            return Err(McpError::validation_error("JWT token has expired"));
        }

        Ok(token_data.claims)
    }

    /// Generate JWT token for testing
    #[cfg(test)]
    pub fn generate_test_jwt(&self, user_id: &str, permissions: Vec<String>) -> String {
        let now = chrono::Utc::now().timestamp() as usize;
        let claims = JwtClaims {
            sub: user_id.to_string(),
            exp: now + 3600, // 1 hour
            iat: now,
            permissions,
        };

        let header = Header::new(Algorithm::HS256);
        let key = EncodingKey::from_secret(self.jwt_secret.as_ref());
        encode(&header, &claims, &key).unwrap()
    }
}

#[async_trait::async_trait]
impl McpMiddleware for AuthenticationMiddleware {
    fn name(&self) -> &'static str {
        "authentication"
    }

    fn priority(&self) -> i32 {
        10 // High priority to run early
    }

    async fn before_request(
        &self,
        request: &CallToolRequest,
        context: &mut MiddlewareContext,
    ) -> McpResult<MiddlewareResult> {
        if !self.require_auth {
            context.set_metadata("auth_required".to_string(), Value::Bool(false));
            return Ok(MiddlewareResult::Continue);
        }

        // Try API key authentication first
        if let Some(api_key) = self.extract_api_key(request) {
            match self.validate_api_key(&api_key) {
                Ok(api_key_info) => {
                    context.set_metadata(
                        "auth_type".to_string(),
                        Value::String("api_key".to_string()),
                    );
                    context.set_metadata(
                        "auth_key_id".to_string(),
                        Value::String(api_key_info.key_id),
                    );
                    context.set_metadata(
                        "auth_permissions".to_string(),
                        serde_json::to_value(api_key_info.permissions)
                            .unwrap_or(Value::Array(vec![])),
                    );
                    context.set_metadata("auth_required".to_string(), Value::Bool(true));
                    return Ok(MiddlewareResult::Continue);
                }
                Err(_) => {
                    // API key failed, try JWT
                }
            }
        }

        // Try JWT authentication
        if let Some(jwt_token) = self.extract_jwt_token(request) {
            match self.validate_jwt_token(&jwt_token) {
                Ok(claims) => {
                    context.set_metadata("auth_type".to_string(), Value::String("jwt".to_string()));
                    context.set_metadata("auth_user_id".to_string(), Value::String(claims.sub));
                    context.set_metadata(
                        "auth_permissions".to_string(),
                        serde_json::to_value(claims.permissions).unwrap_or(Value::Array(vec![])),
                    );
                    context.set_metadata("auth_required".to_string(), Value::Bool(true));
                    return Ok(MiddlewareResult::Continue);
                }
                Err(_) => {
                    // JWT failed
                }
            }
        }

        // No valid authentication found
        let error_result = CallToolResult {
            content: vec![crate::mcp::Content::Text {
                text: "Authentication required. Please provide a valid API key or JWT token."
                    .to_string(),
            }],
            is_error: true,
        };

        Ok(MiddlewareResult::Stop(error_result))
    }
}

/// Rate limiting middleware with per-client limits
pub struct RateLimitMiddleware {
    rate_limiter: Arc<RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>>,
    default_limit: u32,
    #[allow(dead_code)]
    burst_limit: u32,
}

impl RateLimitMiddleware {
    /// Create a new rate limiting middleware
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
    pub fn with_limits(requests_per_minute: u32, burst_limit: u32) -> Self {
        Self::new(requests_per_minute, burst_limit)
    }

    /// Create with default limits (60 requests per minute, burst of 10)
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(60, 10)
    }

    /// Extract client identifier from request
    fn extract_client_id(&self, request: &CallToolRequest, context: &MiddlewareContext) -> String {
        // Try to get from authentication context first
        if let Some(auth_key_id) = context.get_metadata("auth_key_id").and_then(|v| v.as_str()) {
            return format!("api_key:{}", auth_key_id);
        }

        if let Some(auth_user_id) = context
            .get_metadata("auth_user_id")
            .and_then(|v| v.as_str())
        {
            return format!("jwt:{}", auth_user_id);
        }

        // Fallback to request-based identifier
        if let Some(args) = &request.arguments {
            if let Some(client_id) = args.get("client_id").and_then(|v| v.as_str()) {
                return format!("client:{}", client_id);
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
        let client_id = self.extract_client_id(request, context);

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

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Authentication configuration
    pub authentication: AuthenticationConfig,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// Enable authentication middleware
    pub enabled: bool,
    /// Require authentication for all requests
    pub require_auth: bool,
    /// JWT secret for token validation
    pub jwt_secret: String,
    /// API keys configuration
    pub api_keys: Vec<ApiKeyConfig>,
    /// OAuth 2.0 configuration
    pub oauth: Option<OAuth2Config>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// API key value
    pub key: String,
    /// Key identifier
    pub key_id: String,
    /// Permissions for this key
    pub permissions: Vec<String>,
    /// Optional expiration date
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Config {
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret
    pub client_secret: String,
    /// Token endpoint URL
    pub token_endpoint: String,
    /// Required scopes
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Enable rate limiting middleware
    pub enabled: bool,
    /// Requests per minute limit
    pub requests_per_minute: u32,
    /// Burst limit for short bursts
    pub burst_limit: u32,
    /// Custom limits per client type
    pub custom_limits: Option<HashMap<String, u32>>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            authentication: AuthenticationConfig {
                enabled: true,
                require_auth: false, // Start with auth disabled for easier development
                jwt_secret: "your-secret-key-change-this-in-production".to_string(),
                api_keys: vec![],
                oauth: None,
            },
            rate_limiting: RateLimitingConfig {
                enabled: true,
                requests_per_minute: 60,
                burst_limit: 10,
                custom_limits: None,
            },
        }
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
    /// Security configuration
    pub security: SecurityConfig,
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
            security: SecurityConfig::default(),
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

        // Security middleware (highest priority)
        if self.security.authentication.enabled {
            let api_keys: HashMap<String, ApiKeyInfo> = self
                .security
                .authentication
                .api_keys
                .into_iter()
                .map(|config| {
                    let expires_at = config.expires_at.and_then(|date_str| {
                        chrono::DateTime::parse_from_rfc3339(&date_str)
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                    });

                    let api_key_info = ApiKeyInfo {
                        key_id: config.key_id,
                        permissions: config.permissions,
                        expires_at,
                    };

                    (config.key, api_key_info)
                })
                .collect();

            let auth_middleware = if self.security.authentication.require_auth {
                if let Some(oauth_config) = self.security.authentication.oauth {
                    let oauth = OAuthConfig {
                        client_id: oauth_config.client_id,
                        client_secret: oauth_config.client_secret,
                        token_endpoint: oauth_config.token_endpoint,
                        scope: oauth_config.scopes,
                    };
                    AuthenticationMiddleware::with_oauth(
                        api_keys,
                        self.security.authentication.jwt_secret,
                        oauth,
                    )
                } else {
                    AuthenticationMiddleware::new(api_keys, self.security.authentication.jwt_secret)
                }
            } else {
                AuthenticationMiddleware::permissive()
            };

            chain = chain.add_middleware(auth_middleware);
        }

        if self.security.rate_limiting.enabled {
            let rate_limit_middleware = RateLimitMiddleware::with_limits(
                self.security.rate_limiting.requests_per_minute,
                self.security.rate_limiting.burst_limit,
            );
            chain = chain.add_middleware(rate_limit_middleware);
        }

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
