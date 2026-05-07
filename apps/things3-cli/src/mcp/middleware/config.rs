//! Middleware configuration types

use super::auth::{ApiKeyInfo, OAuthConfig};
use super::logging::LogLevel;
use super::{
    AuthenticationMiddleware, LoggingMiddleware, MiddlewareChain, PerformanceMiddleware,
    RateLimitMiddleware, ValidationMiddleware,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

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
