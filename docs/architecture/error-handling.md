# Error Handling Design

This document outlines the comprehensive error handling strategy for the Rust Things library, designed to provide robust error management with excellent developer experience.

## Error Handling Philosophy

### Core Principles

1. **Fail Fast**: Detect and report errors as early as possible
2. **Fail Safe**: Graceful degradation when possible
3. **Context Rich**: Provide detailed error context for debugging
4. **Recoverable**: Distinguish between recoverable and non-recoverable errors
5. **User Friendly**: Provide actionable error messages for end users

### Error Categories

```rust
/// Error categories for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// System-level errors (database, network, etc.)
    System,
    /// User input validation errors
    Validation,
    /// Business logic errors
    Business,
    /// Configuration errors
    Configuration,
    /// Permission/authorization errors
    Permission,
    /// Resource not found errors
    NotFound,
    /// Rate limiting errors
    RateLimit,
    /// Cache-related errors
    Cache,
    /// Serialization/deserialization errors
    Serialization,
    /// Unknown/unexpected errors
    Unknown,
}
```

## Enhanced Error Types

### Main Error Enum

```rust
/// Comprehensive error types for Things operations
#[derive(Error, Debug)]
pub enum ThingsError {
    // Database errors
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("Database connection failed: {message}")]
    DatabaseConnection { 
        message: String,
        #[source]
        cause: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    #[error("Database transaction failed: {message}")]
    DatabaseTransaction { 
        message: String,
        #[source]
        cause: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    #[error("Database query failed: {query}")]
    DatabaseQuery { 
        query: String,
        #[source]
        cause: rusqlite::Error,
    },
    
    // Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Deserialization error: {message}")]
    Deserialization { 
        message: String,
        data: String,
        #[source]
        cause: serde_json::Error,
    },
    
    // IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("File not found: {path}")]
    FileNotFound { path: String },
    
    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },
    
    // Validation errors
    #[error("Validation error: {message}")]
    Validation { 
        message: String,
        field: Option<String>,
        value: Option<String>,
    },
    
    #[error("Invalid UUID: {uuid}")]
    InvalidUuid { uuid: String },
    
    #[error("Invalid date: {date}")]
    InvalidDate { date: String },
    
    #[error("Invalid email: {email}")]
    InvalidEmail { email: String },
    
    // Not found errors
    #[error("Task not found: {uuid}")]
    TaskNotFound { uuid: Uuid },
    
    #[error("Project not found: {uuid}")]
    ProjectNotFound { uuid: Uuid },
    
    #[error("Area not found: {uuid}")]
    AreaNotFound { uuid: Uuid },
    
    #[error("Tag not found: {uuid}")]
    TagNotFound { uuid: Uuid },
    
    #[error("User not found: {identifier}")]
    UserNotFound { identifier: String },
    
    // Configuration errors
    #[error("Configuration error: {message}")]
    Configuration { 
        message: String,
        key: Option<String>,
        value: Option<String>,
    },
    
    #[error("Missing configuration: {key}")]
    MissingConfiguration { key: String },
    
    #[error("Invalid configuration: {key} = {value}")]
    InvalidConfiguration { 
        key: String, 
        value: String,
        expected: String,
    },
    
    // Cache errors
    #[error("Cache error: {message}")]
    Cache { 
        message: String,
        operation: Option<String>,
        key: Option<String>,
    },
    
    #[error("Cache miss: {key}")]
    CacheMiss { key: String },
    
    #[error("Cache eviction failed: {key}")]
    CacheEvictionFailed { key: String },
    
    // Permission errors
    #[error("Permission denied: {message}")]
    PermissionDenied { 
        message: String,
        resource: Option<String>,
        action: Option<String>,
    },
    
    #[error("Insufficient privileges: {required}")]
    InsufficientPrivileges { required: String },
    
    // Rate limiting
    #[error("Rate limit exceeded: {message}")]
    RateLimitExceeded { 
        message: String,
        limit: u32,
        window: Duration,
        retry_after: Option<Duration>,
    },
    
    // Business logic errors
    #[error("Business rule violation: {message}")]
    BusinessRuleViolation { 
        message: String,
        rule: Option<String>,
        context: Option<HashMap<String, Value>>,
    },
    
    #[error("Operation not allowed: {message}")]
    OperationNotAllowed { 
        message: String,
        operation: String,
        reason: Option<String>,
    },
    
    // Network errors
    #[error("Network error: {message}")]
    Network { 
        message: String,
        url: Option<String>,
        status_code: Option<u16>,
    },
    
    #[error("Timeout: {operation}")]
    Timeout { 
        operation: String,
        duration: Duration,
    },
    
    // Unknown errors
    #[error("Unknown error: {message}")]
    Unknown { 
        message: String,
        context: Option<HashMap<String, Value>>,
    },
}

impl ThingsError {
    /// Get error category
    pub fn category(&self) -> ErrorCategory {
        match self {
            ThingsError::Database(_) => ErrorCategory::System,
            ThingsError::DatabaseConnection { .. } => ErrorCategory::System,
            ThingsError::DatabaseTransaction { .. } => ErrorCategory::System,
            ThingsError::DatabaseQuery { .. } => ErrorCategory::System,
            ThingsError::Serialization(_) => ErrorCategory::Serialization,
            ThingsError::Deserialization { .. } => ErrorCategory::Serialization,
            ThingsError::Io(_) => ErrorCategory::System,
            ThingsError::FileNotFound { .. } => ErrorCategory::System,
            ThingsError::PermissionDenied { .. } => ErrorCategory::Permission,
            ThingsError::Validation { .. } => ErrorCategory::Validation,
            ThingsError::InvalidUuid { .. } => ErrorCategory::Validation,
            ThingsError::InvalidDate { .. } => ErrorCategory::Validation,
            ThingsError::InvalidEmail { .. } => ErrorCategory::Validation,
            ThingsError::TaskNotFound { .. } => ErrorCategory::NotFound,
            ThingsError::ProjectNotFound { .. } => ErrorCategory::NotFound,
            ThingsError::AreaNotFound { .. } => ErrorCategory::NotFound,
            ThingsError::TagNotFound { .. } => ErrorCategory::NotFound,
            ThingsError::UserNotFound { .. } => ErrorCategory::NotFound,
            ThingsError::Configuration { .. } => ErrorCategory::Configuration,
            ThingsError::MissingConfiguration { .. } => ErrorCategory::Configuration,
            ThingsError::InvalidConfiguration { .. } => ErrorCategory::Configuration,
            ThingsError::Cache { .. } => ErrorCategory::Cache,
            ThingsError::CacheMiss { .. } => ErrorCategory::Cache,
            ThingsError::CacheEvictionFailed { .. } => ErrorCategory::Cache,
            ThingsError::InsufficientPrivileges { .. } => ErrorCategory::Permission,
            ThingsError::RateLimitExceeded { .. } => ErrorCategory::RateLimit,
            ThingsError::BusinessRuleViolation { .. } => ErrorCategory::Business,
            ThingsError::OperationNotAllowed { .. } => ErrorCategory::Business,
            ThingsError::Network { .. } => ErrorCategory::System,
            ThingsError::Timeout { .. } => ErrorCategory::System,
            ThingsError::Unknown { .. } => ErrorCategory::Unknown,
        }
    }
    
    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            ThingsError::Database(_) => ErrorSeverity::High,
            ThingsError::DatabaseConnection { .. } => ErrorSeverity::High,
            ThingsError::DatabaseTransaction { .. } => ErrorSeverity::High,
            ThingsError::DatabaseQuery { .. } => ErrorSeverity::Medium,
            ThingsError::Serialization(_) => ErrorSeverity::Medium,
            ThingsError::Deserialization { .. } => ErrorSeverity::Medium,
            ThingsError::Io(_) => ErrorSeverity::High,
            ThingsError::FileNotFound { .. } => ErrorSeverity::Medium,
            ThingsError::PermissionDenied { .. } => ErrorSeverity::High,
            ThingsError::Validation { .. } => ErrorSeverity::Low,
            ThingsError::InvalidUuid { .. } => ErrorSeverity::Low,
            ThingsError::InvalidDate { .. } => ErrorSeverity::Low,
            ThingsError::InvalidEmail { .. } => ErrorSeverity::Low,
            ThingsError::TaskNotFound { .. } => ErrorSeverity::Low,
            ThingsError::ProjectNotFound { .. } => ErrorSeverity::Low,
            ThingsError::AreaNotFound { .. } => ErrorSeverity::Low,
            ThingsError::TagNotFound { .. } => ErrorSeverity::Low,
            ThingsError::UserNotFound { .. } => ErrorSeverity::Medium,
            ThingsError::Configuration { .. } => ErrorSeverity::High,
            ThingsError::MissingConfiguration { .. } => ErrorSeverity::High,
            ThingsError::InvalidConfiguration { .. } => ErrorSeverity::High,
            ThingsError::Cache { .. } => ErrorSeverity::Low,
            ThingsError::CacheMiss { .. } => ErrorSeverity::Low,
            ThingsError::CacheEvictionFailed { .. } => ErrorSeverity::Low,
            ThingsError::InsufficientPrivileges { .. } => ErrorSeverity::High,
            ThingsError::RateLimitExceeded { .. } => ErrorSeverity::Medium,
            ThingsError::BusinessRuleViolation { .. } => ErrorSeverity::Medium,
            ThingsError::OperationNotAllowed { .. } => ErrorSeverity::Medium,
            ThingsError::Network { .. } => ErrorSeverity::High,
            ThingsError::Timeout { .. } => ErrorSeverity::Medium,
            ThingsError::Unknown { .. } => ErrorSeverity::High,
        }
    }
    
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            ThingsError::DatabaseConnection { .. } => true,
            ThingsError::Cache { .. } => true,
            ThingsError::CacheMiss { .. } => true,
            ThingsError::RateLimitExceeded { .. } => true,
            ThingsError::Network { .. } => true,
            ThingsError::Timeout { .. } => true,
            _ => false,
        }
    }
    
    /// Get retry delay for recoverable errors
    pub fn retry_delay(&self) -> Option<Duration> {
        match self {
            ThingsError::DatabaseConnection { .. } => Some(Duration::from_secs(1)),
            ThingsError::Cache { .. } => Some(Duration::from_millis(100)),
            ThingsError::RateLimitExceeded { retry_after, .. } => *retry_after,
            ThingsError::Network { .. } => Some(Duration::from_secs(2)),
            ThingsError::Timeout { .. } => Some(Duration::from_secs(1)),
            _ => None,
        }
    }
    
    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            ThingsError::TaskNotFound { uuid } => {
                format!("Task with ID {} was not found", uuid)
            }
            ThingsError::ProjectNotFound { uuid } => {
                format!("Project with ID {} was not found", uuid)
            }
            ThingsError::AreaNotFound { uuid } => {
                format!("Area with ID {} was not found", uuid)
            }
            ThingsError::TagNotFound { uuid } => {
                format!("Tag with ID {} was not found", uuid)
            }
            ThingsError::Validation { message, field, .. } => {
                if let Some(field) = field {
                    format!("Invalid {}: {}", field, message)
                } else {
                    message.clone()
                }
            }
            ThingsError::PermissionDenied { message, .. } => {
                format!("Access denied: {}", message)
            }
            ThingsError::RateLimitExceeded { message, .. } => {
                format!("Too many requests: {}", message)
            }
            _ => self.to_string(),
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}
```

## Error Context and Tracing

### Error Context

```rust
/// Error context for additional information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Error ID for tracking
    pub error_id: Uuid,
    /// Timestamp when error occurred
    pub timestamp: DateTime<Utc>,
    /// User ID (if available)
    pub user_id: Option<Uuid>,
    /// Session ID (if available)
    pub session_id: Option<String>,
    /// Request ID (if available)
    pub request_id: Option<String>,
    /// Stack trace
    pub stack_trace: Option<String>,
    /// Additional context data
    pub context_data: HashMap<String, Value>,
    /// Error chain
    pub error_chain: Vec<ErrorLink>,
}

/// Error link in the error chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorLink {
    /// Error message
    pub message: String,
    /// Source location
    pub location: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Error context builder
pub struct ErrorContextBuilder {
    context: ErrorContext,
}

impl ErrorContextBuilder {
    /// Create new error context builder
    pub fn new() -> Self {
        Self {
            context: ErrorContext {
                error_id: Uuid::new_v4(),
                timestamp: Utc::now(),
                user_id: None,
                session_id: None,
                request_id: None,
                stack_trace: None,
                context_data: HashMap::new(),
                error_chain: Vec::new(),
            },
        }
    }
    
    /// Set user ID
    pub fn user_id(mut self, user_id: Uuid) -> Self {
        self.context.user_id = Some(user_id);
        self
    }
    
    /// Set session ID
    pub fn session_id(mut self, session_id: String) -> Self {
        self.context.session_id = Some(session_id);
        self
    }
    
    /// Set request ID
    pub fn request_id(mut self, request_id: String) -> Self {
        self.context.request_id = Some(request_id);
        self
    }
    
    /// Add context data
    pub fn context_data(mut self, key: String, value: Value) -> Self {
        self.context.context_data.insert(key, value);
        self
    }
    
    /// Add error to chain
    pub fn add_error(mut self, message: String, location: Option<String>) -> Self {
        self.context.error_chain.push(ErrorLink {
            message,
            location,
            timestamp: Utc::now(),
        });
        self
    }
    
    /// Build error context
    pub fn build(self) -> ErrorContext {
        self.context
    }
}
```

### Error Tracing

```rust
/// Error tracing and logging
pub struct ErrorTracer {
    logger: Arc<dyn ErrorLogger>,
    config: TracingConfig,
}

/// Error logger trait
pub trait ErrorLogger: Send + Sync {
    /// Log error
    fn log_error(&self, error: &ThingsError, context: &ErrorContext);
    
    /// Log error with custom level
    fn log_error_with_level(&self, error: &ThingsError, context: &ErrorContext, level: log::Level);
    
    /// Log error statistics
    fn log_error_stats(&self, stats: &ErrorStatistics);
}

/// Error statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStatistics {
    /// Total error count
    pub total_errors: u64,
    /// Errors by category
    pub errors_by_category: HashMap<ErrorCategory, u64>,
    /// Errors by severity
    pub errors_by_severity: HashMap<ErrorSeverity, u64>,
    /// Most common errors
    pub common_errors: Vec<CommonError>,
    /// Error rate over time
    pub error_rate: Vec<ErrorRatePoint>,
    /// Recovery rate
    pub recovery_rate: f64,
}

/// Common error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonError {
    /// Error type
    pub error_type: String,
    /// Count
    pub count: u64,
    /// Percentage
    pub percentage: f64,
    /// Last occurrence
    pub last_occurrence: DateTime<Utc>,
}

/// Error rate point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRatePoint {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Error count
    pub count: u64,
    /// Error rate (errors per second)
    pub rate: f64,
}

impl ErrorTracer {
    /// Trace error with context
    pub fn trace_error(&self, error: &ThingsError, context: &ErrorContext) {
        self.logger.log_error(error, context);
        
        // Update statistics
        self.update_statistics(error, context);
        
        // Check for alerts
        self.check_alerts(error, context);
    }
    
    /// Update error statistics
    fn update_statistics(&self, error: &ThingsError, context: &ErrorContext) {
        // Implementation for updating statistics
    }
    
    /// Check for error alerts
    fn check_alerts(&self, error: &ThingsError, context: &ErrorContext) {
        // Implementation for checking alerts
    }
}
```

## Error Recovery Strategies

### Retry Logic

```rust
/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Initial retry delay
    pub initial_delay: Duration,
    /// Maximum retry delay
    pub max_delay: Duration,
    /// Retry delay multiplier
    pub delay_multiplier: f64,
    /// Jitter for retry delays
    pub jitter: bool,
    /// Retryable error types
    pub retryable_errors: Vec<ErrorCategory>,
}

/// Retry manager
pub struct RetryManager {
    config: RetryConfig,
    statistics: Arc<RwLock<RetryStatistics>>,
}

/// Retry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStatistics {
    /// Total retry attempts
    pub total_attempts: u64,
    /// Successful retries
    pub successful_retries: u64,
    /// Failed retries
    pub failed_retries: u64,
    /// Average retry delay
    pub avg_retry_delay: Duration,
    /// Retry success rate
    pub success_rate: f64,
}

impl RetryManager {
    /// Execute operation with retry logic
    pub async fn execute_with_retry<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Result<T> + Send + Sync,
    {
        let mut attempt = 0;
        let mut delay = self.config.initial_delay;
        
        loop {
            match operation() {
                Ok(result) => {
                    self.record_success(attempt);
                    return Ok(result);
                }
                Err(error) => {
                    if !self.should_retry(&error, attempt) {
                        self.record_failure(attempt);
                        return Err(error);
                    }
                    
                    attempt += 1;
                    if attempt >= self.config.max_retries {
                        self.record_failure(attempt);
                        return Err(error);
                    }
                    
                    // Wait before retry
                    tokio::time::sleep(delay).await;
                    
                    // Calculate next delay
                    delay = self.calculate_next_delay(delay);
                }
            }
        }
    }
    
    /// Check if error should be retried
    fn should_retry(&self, error: &ThingsError, attempt: u32) -> bool {
        if attempt >= self.config.max_retries {
            return false;
        }
        
        if !error.is_recoverable() {
            return false;
        }
        
        if let Some(retry_delay) = error.retry_delay() {
            return retry_delay > Duration::from_secs(0);
        }
        
        self.config.retryable_errors.contains(&error.category())
    }
    
    /// Calculate next retry delay
    fn calculate_next_delay(&self, current_delay: Duration) -> Duration {
        let mut next_delay = current_delay.mul_f64(self.config.delay_multiplier);
        
        if next_delay > self.config.max_delay {
            next_delay = self.config.max_delay;
        }
        
        if self.config.jitter {
            let jitter = Duration::from_millis(
                (next_delay.as_millis() as f64 * 0.1 * fastrand::f64()) as u64
            );
            next_delay = next_delay + jitter;
        }
        
        next_delay
    }
}
```

### Circuit Breaker

```rust
/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failure threshold
    pub failure_threshold: u32,
    /// Success threshold (for half-open state)
    pub success_threshold: u32,
    /// Timeout for open state
    pub timeout: Duration,
    /// Window for counting failures
    pub window: Duration,
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitBreakerState>>,
    config: CircuitBreakerConfig,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl CircuitBreaker {
    /// Execute operation through circuit breaker
    pub async fn execute<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Result<T> + Send + Sync,
    {
        let state = *self.state.read().unwrap();
        
        match state {
            CircuitBreakerState::Closed => {
                match operation() {
                    Ok(result) => {
                        self.reset_success_count();
                        Ok(result)
                    }
                    Err(error) => {
                        self.record_failure();
                        Err(error)
                    }
                }
            }
            CircuitBreakerState::Open => {
                if self.should_attempt_reset() {
                    self.set_state(CircuitBreakerState::HalfOpen);
                    self.execute(operation).await
                } else {
                    Err(ThingsError::OperationNotAllowed {
                        message: "Circuit breaker is open".to_string(),
                        operation: "execute".to_string(),
                        reason: Some("Too many failures".to_string()),
                    })
                }
            }
            CircuitBreakerState::HalfOpen => {
                match operation() {
                    Ok(result) => {
                        self.record_success();
                        if self.success_count.read().unwrap() >= &self.config.success_threshold {
                            self.set_state(CircuitBreakerState::Closed);
                        }
                        Ok(result)
                    }
                    Err(error) => {
                        self.set_state(CircuitBreakerState::Open);
                        Err(error)
                    }
                }
            }
        }
    }
}
```

## Error Handling Best Practices

### Error Handling Patterns

```rust
/// Error handling patterns and utilities
pub struct ErrorHandler {
    tracer: ErrorTracer,
    retry_manager: RetryManager,
    circuit_breaker: CircuitBreaker,
}

impl ErrorHandler {
    /// Handle error with appropriate strategy
    pub async fn handle_error<T>(&self, error: ThingsError, context: ErrorContext) -> Result<T> {
        // Trace the error
        self.tracer.trace_error(&error, &context);
        
        // Determine handling strategy based on error type
        match error.category() {
            ErrorCategory::System => {
                if error.is_recoverable() {
                    // Try to recover using retry logic
                    self.retry_manager.execute_with_retry(|| Err(error)).await
                } else {
                    Err(error)
                }
            }
            ErrorCategory::Validation => {
                // Validation errors are not recoverable
                Err(error)
            }
            ErrorCategory::Business => {
                // Business errors may be recoverable with different input
                Err(error)
            }
            ErrorCategory::Configuration => {
                // Configuration errors require manual intervention
                Err(error)
            }
            ErrorCategory::Permission => {
                // Permission errors are not recoverable
                Err(error)
            }
            ErrorCategory::NotFound => {
                // Not found errors are not recoverable
                Err(error)
            }
            ErrorCategory::RateLimit => {
                // Rate limit errors are recoverable after delay
                if let Some(delay) = error.retry_delay() {
                    tokio::time::sleep(delay).await;
                    Err(error)
                } else {
                    Err(error)
                }
            }
            ErrorCategory::Cache => {
                // Cache errors are recoverable
                if error.is_recoverable() {
                    self.retry_manager.execute_with_retry(|| Err(error)).await
                } else {
                    Err(error)
                }
            }
            ErrorCategory::Serialization => {
                // Serialization errors are usually not recoverable
                Err(error)
            }
            ErrorCategory::Unknown => {
                // Unknown errors should be logged and reported
                Err(error)
            }
        }
    }
}
```

This comprehensive error handling design provides a robust foundation for managing errors in the Rust Things library, with proper categorization, recovery strategies, and monitoring capabilities.
