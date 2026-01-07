# Security Policy

## Supported Versions

We take security seriously and strive to address vulnerabilities promptly. The following versions are currently supported with security updates:

| Version | Supported          |
| ------- | ------------------ |
| 0.5.x   | :white_check_mark: |
| 0.4.x   | :white_check_mark: |
| 0.3.x   | :x:                |
| 0.2.x   | :x:                |
| < 0.2   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability in `rust-things3`, please report it responsibly by following these steps:

### Private Disclosure

**DO NOT** create a public GitHub issue for security vulnerabilities.

Instead, please report security issues by emailing:

📧 **garthdb@gmail.com**

### What to Include

When reporting a vulnerability, please include:

1. **Description**: A clear description of the vulnerability
2. **Impact**: How the vulnerability could be exploited and what the potential impact is
3. **Steps to Reproduce**: Detailed steps to reproduce the issue
4. **Affected Versions**: Which versions of `rust-things3` are affected
5. **Suggested Fix**: If you have ideas for how to fix the issue (optional)
6. **Proof of Concept**: Code or configuration that demonstrates the issue (if applicable)

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 5 business days
- **Fix Timeline**: Depends on severity (see below)

## Severity Levels

We assess vulnerabilities using the following severity levels:

### Critical

- **Impact**: Complete system compromise, data loss, or unauthorized access to sensitive data
- **Response**: Patch within 24-48 hours
- **Examples**: Remote code execution, authentication bypass

### High

- **Impact**: Significant impact on security or functionality
- **Response**: Patch within 7 days
- **Examples**: SQL injection, privilege escalation

### Medium

- **Impact**: Moderate security risk
- **Response**: Patch within 30 days
- **Examples**: Information disclosure, weak cryptography

### Low

- **Impact**: Minor security concern
- **Response**: Patch in next regular release
- **Examples**: Minor information leaks, denial of service

## Security Best Practices

When using `rust-things3`, we recommend following these security best practices:

### 1. Database Access

```rust
// ✅ Good: Read-only access when possible
let db = ThingsDatabase::new(&path).await?;
let tasks = db.get_inbox(None).await?;

// ⚠️  Caution: Write operations should be carefully controlled
let task_uuid = db.create_task(request).await?;
```

### 2. Database Path

```bash
# ✅ Good: Use environment variable
export THINGS_DB_PATH="/path/to/things.db"

# ⚠️  Avoid: Hardcoding paths in code
```

### 3. API Endpoints

```rust
// ✅ Good: Add authentication
async fn protected_endpoint(
    auth: AuthGuard,
    State(state): State<AppState>,
) -> Result<Json<Response>, StatusCode> {
    // ...
}

// ❌ Bad: Public write endpoints
async fn public_write_endpoint(
    State(state): State<AppState>,
) -> Result<Json<Response>, StatusCode> {
    // Anyone can write!
}
```

### 4. Input Validation

```rust
// ✅ Good: Validate input
let uuid = uuid::Uuid::parse_str(&id)
    .map_err(|_| ThingsError::invalid_input("Invalid UUID"))?;

// ❌ Bad: Trust user input
let uuid = uuid::Uuid::parse_str(&id).unwrap();
```

### 5. Error Messages

```rust
// ✅ Good: Generic error messages
return Err(ThingsError::authentication_failed());

// ❌ Bad: Detailed error messages
return Err(ThingsError::unknown(format!(
    "Authentication failed for user {} with password {}",
    username, password
)));
```

## Known Security Considerations

### 1. Local Database Access

`rust-things3` operates on local SQLite databases. Access control is managed by file system permissions. Ensure:

- Database files have appropriate permissions (e.g., `600` or `640`)
- Only trusted users have access to the database file
- Database backups are stored securely

### 2. MCP Server

The MCP server listens on stdin/stdout by default (no network exposure). If you build a web-facing service:

- Implement proper authentication
- Use HTTPS in production
- Add rate limiting
- Validate all inputs

### 3. Data Exposure

Tasks and projects may contain sensitive information. When exposing data:

- Implement proper authorization
- Filter sensitive fields
- Use secure transport (HTTPS, encrypted connections)
- Log access appropriately

### 4. Dependencies

We regularly update dependencies to address security vulnerabilities. To check for updates:

```bash
cargo outdated
cargo audit
```

## Security Updates

Security updates are announced through:

1. **GitHub Security Advisories**: https://github.com/GarthDB/rust-things3/security/advisories
2. **GitHub Releases**: https://github.com/GarthDB/rust-things3/releases
3. **CHANGELOG.md**: Security fixes are noted in the changelog

## Responsible Disclosure Recognition

We appreciate security researchers who responsibly disclose vulnerabilities. With your permission, we will:

- Acknowledge your contribution in the security advisory
- Credit you in the release notes
- Add you to our CONTRIBUTORS.md file

## Security Tooling

We use the following tools to maintain security:

- **cargo-audit**: Regular dependency vulnerability scanning
- **cargo-deny**: License and security policy enforcement
- **clippy**: Linter with security-focused rules
- **GitHub Dependabot**: Automatic dependency updates

## Contact

For security-related questions or concerns:

- **Email**: garthdb@gmail.com
- **GitHub**: https://github.com/GarthDB/rust-things3/security

## Legal

By reporting a vulnerability, you agree to:

1. Give us reasonable time to fix the issue before public disclosure
2. Not exploit the vulnerability beyond what's necessary to demonstrate it
3. Not access, modify, or delete data beyond what's necessary for testing

We commit to:

1. Respond to your report promptly
2. Keep you informed of our progress
3. Credit you appropriately (if desired)
4. Not take legal action against you for responsible disclosure

## Additional Resources

- [Rust Security Advisory Database](https://rustsec.org/)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CWE Top 25](https://cwe.mitre.org/top25/)

---

**Last Updated**: January 2026  
**Version**: 1.0

