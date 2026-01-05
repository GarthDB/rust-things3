# Migration Guide

This guide helps you migrate between versions of `things3-core`.

## Current Version: 0.2.0

This PR represents Phase 2 work toward version 0.3.0.

## Version 0.2.0 → 0.3.0 (In Progress)

### No Breaking Changes

Version 0.3.0 will maintain full backward compatibility with 0.2.0. No code changes will be required.

### New Features

- Enhanced error messages with recovery suggestions
- Additional examples in documentation
- Improved API documentation with examples

### Recommended Updates

While not required, consider:

1. **Update error handling** to use new error recovery suggestions
2. **Review examples** for improved patterns
3. **Check documentation** for new best practices

## Future Breaking Changes

When breaking changes are introduced, they will be documented here with:

- **Deprecation Notice**: When the change is announced
- **Migration Steps**: How to update your code
- **Timeline**: When the change will take effect

### Deprecation Policy

1. APIs will be marked with `#[deprecated]` for at least one MINOR version
2. Deprecation notices will include migration guidance
3. Deprecated APIs will be removed in the next MAJOR version

## Example Migration Patterns

### Pattern 1: Function Signature Change

**Before (deprecated):**
```rust
fn old_function(param: String) -> Result<()>
```

**After:**
```rust
fn new_function(param: &str) -> Result<()>
```

**Migration:**
```rust
// Old code
old_function(string_value.clone())?;

// New code
new_function(&string_value)?;
```

### Pattern 2: Struct Field Change

**Before (deprecated):**
```rust
struct Request {
    name: String,
    value: i32,
}
```

**After:**
```rust
struct Request {
    name: String,
    value: Option<i32>, // Now optional
}
```

**Migration:**
```rust
// Old code
Request {
    name: "test".to_string(),
    value: 42,
}

// New code
Request {
    name: "test".to_string(),
    value: Some(42),
}
```

### Pattern 3: Enum Variant Change

**Before (deprecated):**
```rust
enum Status {
    Active,
    Inactive,
}
```

**After:**
```rust
enum Status {
    Active,
    Inactive,
    Pending, // New variant
}
```

**Migration:**
```rust
// Old code - no changes needed, but consider handling new variant
match status {
    Status::Active => {},
    Status::Inactive => {},
    // Handle new variant if needed
    Status::Pending => {},
}
```

## Reporting Migration Issues

If you encounter issues migrating between versions:

1. Check this guide for known issues
2. Review the [CHANGELOG.md](../CHANGELOG.md)
3. Open an issue on GitHub with:
   - Version you're migrating from/to
   - Code example showing the issue
   - Error messages (if any)

## Version History

- **0.2.0**: Initial release
- **0.3.0**: Documentation improvements (no breaking changes)

---

For questions or assistance with migration, please open an issue on GitHub.

