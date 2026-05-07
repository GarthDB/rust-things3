//! Authentication middleware

use super::{McpMiddleware, MiddlewareContext, MiddlewareResult};
use crate::mcp::{CallToolRequest, CallToolResult, McpError, McpResult};
#[allow(unused_imports)]
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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
    fn extract_api_key(request: &CallToolRequest) -> Option<String> {
        // Check if API key is in request arguments
        if let Some(args) = &request.arguments {
            if let Some(api_key) = args.get("api_key").and_then(|v| v.as_str()) {
                return Some(api_key.to_string());
            }
        }
        None
    }

    /// Extract JWT token from request headers or arguments
    fn extract_jwt_token(request: &CallToolRequest) -> Option<String> {
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
        let info = self
            .api_keys
            .get(api_key)
            .cloned()
            .ok_or_else(|| McpError::validation_error("Invalid API key"))?;
        if let Some(exp) = &info.expires_at {
            if *exp < chrono::Utc::now() {
                return Err(McpError::validation_error("API key has expired"));
            }
        }
        Ok(info)
    }

    /// Validate JWT token
    fn validate_jwt_token(&self, token: &str) -> McpResult<JwtClaims> {
        let validation = Validation::new(Algorithm::HS256);
        let key = DecodingKey::from_secret(self.jwt_secret.as_ref());

        let token_data = decode::<JwtClaims>(token, &key, &validation)
            .map_err(|_| McpError::validation_error("Invalid JWT token"))?;

        // Check if token is expired
        let now = chrono::Utc::now().timestamp().try_into().unwrap_or(0);
        if token_data.claims.exp < now {
            return Err(McpError::validation_error("JWT token has expired"));
        }

        Ok(token_data.claims)
    }

    /// Generate JWT token for testing
    ///
    /// # Panics
    /// Panics if JWT encoding fails
    #[cfg(test)]
    #[must_use]
    pub fn generate_test_jwt(&self, user_id: &str, permissions: Vec<String>) -> String {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
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
        if let Some(api_key) = Self::extract_api_key(request) {
            if let Ok(api_key_info) = self.validate_api_key(&api_key) {
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
                    serde_json::to_value(api_key_info.permissions).unwrap_or(Value::Array(vec![])),
                );
                context.set_metadata("auth_required".to_string(), Value::Bool(true));
                return Ok(MiddlewareResult::Continue);
            }
            // API key failed, try JWT
        }

        // Try JWT authentication
        if let Some(jwt_token) = Self::extract_jwt_token(request) {
            if let Ok(claims) = self.validate_jwt_token(&jwt_token) {
                context.set_metadata("auth_type".to_string(), Value::String("jwt".to_string()));
                context.set_metadata("auth_user_id".to_string(), Value::String(claims.sub));
                context.set_metadata(
                    "auth_permissions".to_string(),
                    serde_json::to_value(claims.permissions).unwrap_or(Value::Array(vec![])),
                );
                context.set_metadata("auth_required".to_string(), Value::Bool(true));
                return Ok(MiddlewareResult::Continue);
            }
            // JWT failed
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
