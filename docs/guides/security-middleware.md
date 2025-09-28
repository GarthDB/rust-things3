# Security Middleware Guide

This guide explains how to use the authentication and rate limiting middleware in the Things 3 MCP server.

## Overview

The security middleware provides two main components:
- **AuthenticationMiddleware**: Handles API key and JWT token authentication
- **RateLimitMiddleware**: Implements per-client rate limiting

## Authentication Middleware

### Features

- **API Key Authentication**: Simple key-based authentication
- **JWT Token Authentication**: JSON Web Token support with expiration
- **OAuth 2.0 Support**: Integration with OAuth providers
- **Permission-based Access**: Fine-grained permission control
- **Flexible Configuration**: Can be enabled/disabled per environment

### Configuration

```json
{
  "security": {
    "authentication": {
      "enabled": true,
      "require_auth": true,
      "jwt_secret": "your-secret-key",
      "api_keys": [
        {
          "key": "sk-1234567890abcdef",
          "key_id": "admin-key",
          "permissions": ["read", "write", "admin"],
          "expires_at": null
        }
      ],
      "oauth": {
        "client_id": "your-client-id",
        "client_secret": "your-client-secret",
        "token_endpoint": "https://provider.com/oauth/token",
        "scopes": ["read", "write"]
      }
    }
  }
}
```

### Usage

#### API Key Authentication

Include the API key in your request arguments:

```json
{
  "name": "get_inbox",
  "arguments": {
    "api_key": "sk-1234567890abcdef",
    "limit": 10
  }
}
```

#### JWT Token Authentication

Include the JWT token in your request arguments:

```json
{
  "name": "get_inbox",
  "arguments": {
    "jwt_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "limit": 10
  }
}
```

### JWT Token Structure

The JWT tokens use the following claims:

```json
{
  "sub": "user123",
  "exp": 1640995200,
  "iat": 1640991600,
  "permissions": ["read", "write"]
}
```

- `sub`: Subject (user ID)
- `exp`: Expiration time (Unix timestamp)
- `iat`: Issued at time (Unix timestamp)
- `permissions`: Array of permission strings

## Rate Limiting Middleware

### Features

- **Per-client Limits**: Different limits for different client types
- **Burst Protection**: Handles short bursts of requests
- **Authentication-aware**: Uses auth context for client identification
- **Configurable Limits**: Customizable per client type

### Configuration

```json
{
  "security": {
    "rate_limiting": {
      "enabled": true,
      "requests_per_minute": 60,
      "burst_limit": 10,
      "custom_limits": {
        "admin": 120,
        "readonly": 30,
        "api_key": 60,
        "jwt": 90
      }
    }
  }
}
```

### Client Identification

The rate limiter identifies clients using the following priority:

1. **API Key ID**: If authenticated with API key
2. **JWT User ID**: If authenticated with JWT
3. **Client ID**: From request arguments
4. **Request ID**: Fallback identifier

## Error Responses

### Authentication Errors

When authentication fails, the middleware returns:

```json
{
  "content": [
    {
      "text": "Authentication required. Please provide a valid API key or JWT token."
    }
  ],
  "is_error": true
}
```

### Rate Limiting Errors

When rate limits are exceeded:

```json
{
  "content": [
    {
      "text": "Rate limit exceeded. Limit: 60 requests per minute. Please try again later."
    }
  ],
  "is_error": true
}
```

## Development vs Production

### Development Mode

For easier development, you can disable authentication:

```json
{
  "security": {
    "authentication": {
      "enabled": true,
      "require_auth": false
    }
  }
}
```

### Production Mode

For production, ensure:

1. **Strong JWT Secret**: Use a cryptographically secure secret
2. **API Key Rotation**: Regularly rotate API keys
3. **Proper Rate Limits**: Set appropriate limits for your use case
4. **HTTPS Only**: Always use HTTPS in production
5. **Key Management**: Store secrets securely

## Security Best Practices

### API Keys

- Use cryptographically secure random keys
- Implement key rotation policies
- Monitor key usage
- Revoke compromised keys immediately

### JWT Tokens

- Use strong signing algorithms (HS256 or RS256)
- Set reasonable expiration times
- Implement token refresh mechanisms
- Validate all claims

### Rate Limiting

- Set appropriate limits for your use case
- Monitor rate limit violations
- Implement progressive penalties
- Consider different limits for different user types

## Testing

The middleware includes comprehensive tests. Run them with:

```bash
cargo test middleware::tests
```

### Test Examples

```rust
#[tokio::test]
async fn test_authentication_with_valid_api_key() {
    let mut api_keys = HashMap::new();
    api_keys.insert(
        "test-key".to_string(),
        ApiKeyInfo {
            key_id: "test-id".to_string(),
            permissions: vec!["read".to_string()],
            expires_at: None,
        },
    );

    let middleware = AuthenticationMiddleware::new(api_keys, "test-secret".to_string());
    // ... test implementation
}
```

## Integration Examples

### With MCP Server

```rust
use things3_cli::mcp::middleware::{MiddlewareConfig, SecurityConfig};

let config = MiddlewareConfig {
    security: SecurityConfig {
        authentication: AuthenticationConfig {
            enabled: true,
            require_auth: true,
            jwt_secret: "your-secret".to_string(),
            api_keys: vec![],
            oauth: None,
        },
        rate_limiting: RateLimitingConfig {
            enabled: true,
            requests_per_minute: 60,
            burst_limit: 10,
            custom_limits: None,
        },
    },
    // ... other config
};

let server = ThingsMcpServer::with_middleware_config(db, things_config, config);
```

### With Custom Middleware Chain

```rust
use things3_cli::mcp::middleware::*;

let mut api_keys = HashMap::new();
api_keys.insert("key1".to_string(), ApiKeyInfo {
    key_id: "admin".to_string(),
    permissions: vec!["read".to_string(), "write".to_string()],
    expires_at: None,
});

let chain = MiddlewareChain::new()
    .add_middleware(AuthenticationMiddleware::new(api_keys, "secret".to_string()))
    .add_middleware(RateLimitMiddleware::new(60, 10))
    .add_middleware(LoggingMiddleware::info());
```

## Troubleshooting

### Common Issues

1. **Authentication Always Fails**
   - Check JWT secret matches
   - Verify API key format
   - Ensure tokens are not expired

2. **Rate Limiting Too Strict**
   - Adjust `requests_per_minute` setting
   - Check burst limit configuration
   - Verify client identification

3. **Performance Issues**
   - Monitor middleware execution time
   - Consider disabling unused middleware
   - Optimize rate limiter configuration

### Debug Mode

Enable debug logging to troubleshoot issues:

```json
{
  "logging": {
    "enabled": true,
    "level": "debug"
  }
}
```

This will log detailed information about authentication and rate limiting decisions.
