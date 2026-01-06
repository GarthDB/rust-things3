//! Custom Middleware Example
//!
//! This example demonstrates how to create custom middleware for the Things 3
//! library to extend functionality. This is useful for:
//! - Request/response transformation
//! - Custom caching strategies
//! - Logging and monitoring
//! - Access control and authorization
//! - Data validation and enrichment
//!
//! Run this example with:
//! ```bash
//! cargo run --example custom_middleware
//! ```

use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};
use things3_core::{ThingsDatabase, ThingsConfig, Task, ThingsError};
use tracing::{info, warn};

/// Middleware trait for intercepting database operations
#[async_trait]
trait Middleware: Send + Sync {
    /// Called before a database operation
    async fn before_operation(&self, operation: &str) -> Result<(), ThingsError> {
        let _ = operation;
        Ok(())
    }

    /// Called after a successful database operation
    async fn after_operation(&self, operation: &str, _result: &dyn std::any::Any) -> Result<(), ThingsError> {
        let _ = operation;
        Ok(())
    }

    /// Called when an operation fails
    async fn on_error(&self, operation: &str, error: &ThingsError) -> Result<(), ThingsError> {
        let _ = (operation, error);
        Ok(())
    }
}

/// Example 1: Logging Middleware
/// Logs all database operations with timing information
struct LoggingMiddleware;

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn before_operation(&self, operation: &str) -> Result<(), ThingsError> {
        info!("🔍 Starting operation: {}", operation);
        Ok(())
    }

    async fn after_operation(&self, operation: &str, _result: &dyn std::any::Any) -> Result<(), ThingsError> {
        info!("✅ Completed operation: {}", operation);
        Ok(())
    }

    async fn on_error(&self, operation: &str, error: &ThingsError) -> Result<(), ThingsError> {
        warn!("❌ Operation failed: {} - {:?}", operation, error);
        Ok(())
    }
}

/// Example 2: Performance Monitoring Middleware
/// Tracks operation timing and alerts on slow operations
struct PerformanceMiddleware {
    slow_threshold: Duration,
    operation_start: std::sync::RwLock<Option<Instant>>,
}

impl PerformanceMiddleware {
    fn new(slow_threshold: Duration) -> Self {
        Self {
            slow_threshold,
            operation_start: std::sync::RwLock::new(None),
        }
    }
}

#[async_trait]
impl Middleware for PerformanceMiddleware {
    async fn before_operation(&self, _operation: &str) -> Result<(), ThingsError> {
        *self.operation_start.write().unwrap() = Some(Instant::now());
        Ok(())
    }

    async fn after_operation(&self, operation: &str, _result: &dyn std::any::Any) -> Result<(), ThingsError> {
        if let Some(start) = *self.operation_start.read().unwrap() {
            let elapsed = start.elapsed();
            
            if elapsed > self.slow_threshold {
                warn!("⚠️  SLOW OPERATION: {} took {:?}", operation, elapsed);
            } else {
                info!("⚡ {} completed in {:?}", operation, elapsed);
            }
        }
        Ok(())
    }
}

/// Example 3: Caching Middleware
/// Implements custom caching logic
struct CachingMiddleware {
    cache: dashmap::DashMap<String, (Vec<Task>, Instant)>,
    ttl: Duration,
}

impl CachingMiddleware {
    fn new(ttl: Duration) -> Self {
        Self {
            cache: dashmap::DashMap::new(),
            ttl,
        }
    }

    fn get_cached(&self, key: &str) -> Option<Vec<Task>> {
        if let Some(entry) = self.cache.get(key) {
            let (tasks, cached_at) = entry.value();
            if cached_at.elapsed() < self.ttl {
                info!("💾 Cache HIT: {}", key);
                return Some(tasks.clone());
            } else {
                info!("💨 Cache EXPIRED: {}", key);
                self.cache.remove(key);
            }
        }
        info!("❌ Cache MISS: {}", key);
        None
    }

    fn set_cached(&self, key: String, tasks: Vec<Task>) {
        self.cache.insert(key, (tasks, Instant::now()));
    }
}

/// Example 4: Validation Middleware
/// Validates data before and after operations
struct ValidationMiddleware;

#[async_trait]
impl Middleware for ValidationMiddleware {
    async fn before_operation(&self, operation: &str) -> Result<(), ThingsError> {
        // Example: Check if operation is allowed
        if operation == "dangerous_operation" {
            warn!("⚠️  Blocked dangerous operation");
            return Err(ThingsError::unknown("Operation not allowed".to_string()));
        }
        Ok(())
    }

    async fn after_operation(&self, _operation: &str, _result: &dyn std::any::Any) -> Result<(), ThingsError> {
        // Example: Validate result data
        // if let Some(tasks) = result.downcast_ref::<Vec<Task>>() {
        //     for task in tasks {
        //         // Validate task data
        //     }
        // }
        Ok(())
    }
}

/// Example 5: Rate Limiting Middleware
/// Implements simple rate limiting
struct RateLimitMiddleware {
    requests: std::sync::Arc<std::sync::Mutex<Vec<Instant>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimitMiddleware {
    fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            max_requests,
            window,
        }
    }

    fn check_rate_limit(&self) -> Result<(), ThingsError> {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();

        // Remove old requests outside the window
        requests.retain(|&req| now.duration_since(req) < self.window);

        if requests.len() >= self.max_requests {
            warn!("🚫 Rate limit exceeded");
            return Err(ThingsError::unknown("Rate limit exceeded".to_string()));
        }

        requests.push(now);
        Ok(())
    }
}

#[async_trait]
impl Middleware for RateLimitMiddleware {
    async fn before_operation(&self, operation: &str) -> Result<(), ThingsError> {
        self.check_rate_limit().map_err(|e| {
            warn!("Rate limit check failed for operation: {}", operation);
            e
        })
    }
}

/// Database wrapper with middleware support
struct MiddlewareDatabase {
    db: Arc<ThingsDatabase>,
    middleware: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareDatabase {
    fn new(db: Arc<ThingsDatabase>) -> Self {
        Self {
            db,
            middleware: Vec::new(),
        }
    }

    fn add_middleware(mut self, middleware: Arc<dyn Middleware>) -> Self {
        self.middleware.push(middleware);
        self
    }

    async fn get_inbox_with_middleware(&self, limit: Option<usize>) -> Result<Vec<Task>, ThingsError> {
        let operation = "get_inbox";

        // Before middleware
        for mw in &self.middleware {
            mw.before_operation(operation).await?;
        }

        // Actual operation
        let result = match self.db.get_inbox(limit).await {
            Ok(tasks) => {
                // After middleware
                for mw in &self.middleware {
                    mw.after_operation(operation, &tasks).await?;
                }
                Ok(tasks)
            }
            Err(e) => {
                // Error middleware
                for mw in &self.middleware {
                    mw.on_error(operation, &e).await?;
                }
                Err(e)
            }
        };

        result
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    info!("🚀 Custom Middleware Example\n");

    // Initialize database
    let config = ThingsConfig::from_env();
    let db = ThingsDatabase::new(&config.database_path).await?;
    let db = Arc::new(db);

    // Create middleware stack
    let middleware_db = MiddlewareDatabase::new(Arc::clone(&db))
        .add_middleware(Arc::new(LoggingMiddleware))
        .add_middleware(Arc::new(PerformanceMiddleware::new(Duration::from_millis(100))))
        .add_middleware(Arc::new(ValidationMiddleware))
        .add_middleware(Arc::new(RateLimitMiddleware::new(10, Duration::from_secs(60))));

    info!("Middleware stack configured with:");
    info!("  1. Logging");
    info!("  2. Performance Monitoring");
    info!("  3. Validation");
    info!("  4. Rate Limiting\n");

    // Example 1: Get inbox with middleware
    info!("=== Example 1: Get Inbox ===");
    match middleware_db.get_inbox_with_middleware(Some(5)).await {
        Ok(tasks) => {
            info!("Retrieved {} tasks", tasks.len());
            for task in &tasks {
                info!("  • {}", task.title);
            }
        }
        Err(e) => {
            warn!("Failed to get inbox: {:?}", e);
        }
    }

    println!("\n");

    // Example 2: Demonstrate caching middleware
    info!("=== Example 2: Caching Middleware ===");
    let caching_mw = CachingMiddleware::new(Duration::from_secs(60));
    
    let cache_key = "inbox_5";
    
    // First call - cache miss
    if let Some(cached) = caching_mw.get_cached(cache_key) {
        info!("Using cached data: {} tasks", cached.len());
    } else {
        info!("Cache miss, fetching from database");
        let tasks = db.get_inbox(Some(5)).await?;
        caching_mw.set_cached(cache_key.to_string(), tasks.clone());
        info!("Cached {} tasks", tasks.len());
    }

    // Second call - cache hit
    if let Some(cached) = caching_mw.get_cached(cache_key) {
        info!("Using cached data: {} tasks", cached.len());
    }

    println!("\n");

    // Example 3: Rate limiting
    info!("=== Example 3: Rate Limiting ===");
    let rate_limiter = RateLimitMiddleware::new(3, Duration::from_secs(10));
    
    for i in 1..=5 {
        if rate_limiter.check_rate_limit().is_ok() {
            info!("Request {} allowed", i);
        } else {
            warn!("Request {} rate limited", i);
        }
    }

    println!("\n");
    info!("✅ Middleware example complete!");

    Ok(())
}

/*
 * Advanced Middleware Patterns:
 * 
 * 1. Composable Middleware: Chain multiple middleware
 * 2. Async Middleware: Support async operations in middleware
 * 3. Error Recovery: Implement retry logic in middleware
 * 4. Circuit Breaker: Add circuit breaker pattern
 * 5. Request Transformation: Modify requests before execution
 * 6. Response Transformation: Modify responses after execution
 * 7. Authorization: Check permissions in middleware
 * 8. Auditing: Log all operations for compliance
 * 9. Metrics Collection: Gather detailed metrics
 * 10. Distributed Tracing: Add OpenTelemetry spans
 */

/*
 * Production Middleware Examples:
 * 
 * // Authentication Middleware
 * struct AuthMiddleware {
 *     required_role: String,
 * }
 * 
 * // Encryption Middleware
 * struct EncryptionMiddleware {
 *     key: Vec<u8>,
 * }
 * 
 * // Compression Middleware
 * struct CompressionMiddleware {
 *     min_size: usize,
 * }
 * 
 * // Retry Middleware
 * struct RetryMiddleware {
 *     max_retries: u32,
 *     backoff: Duration,
 * }
 * 
 * // Deduplication Middleware
 * struct DeduplicationMiddleware {
 *     request_cache: Cache<String, ()>,
 * }
 */

