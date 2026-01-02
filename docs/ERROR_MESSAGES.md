# Error Message Style Guide

This document defines the standard error message formatting for the rust-things3 codebase.

## Core Principles

1. **Consistency**: All error messages should follow the same format
2. **Clarity**: Messages should clearly indicate what operation failed
3. **Context**: Include the underlying error when available
4. **Actionability**: Help users understand what went wrong

## Standard Formats

### Operation Failures

When an operation fails, use the format: **"Failed to {operation}: {error}"**

```rust
// ✅ Good
.map_err(|e| ThingsError::unknown(format!("Failed to connect to database: {e}")))?

.map_err(|e| ThingsError::unknown(format!("Failed to create task: {e}")))?

.map_err(|e| ThingsError::unknown(format!("Failed to update project: {e}")))?
```

```rust
// ❌ Avoid
.map_err(|e| ThingsError::unknown(format!("Database connection failed: {e}")))?

.map_err(|e| ThingsError::unknown(format!("Could not create task: {e}")))?

.map_err(|e| ThingsError::unknown(format!("Error updating project: {e}")))?
```

### Validation Failures

When validation fails, use the format: **"{Entity} not found: {identifier}"**

```rust
// ✅ Good  
return Err(ThingsError::unknown(format!("Task not found: {uuid}")));

return Err(ThingsError::unknown(format!("Project not found: {uuid}")));

return Err(ThingsError::unknown(format!("Area not found: {uuid}")));
```

```rust
// ❌ Avoid
return Err(ThingsError::unknown(format!("Could not find task: {uuid}")));

return Err(ThingsError::unknown(format!("Task {uuid} does not exist")));
```

### Invalid Input

When input validation fails, use the format: **"Invalid {field}: {details}"**

```rust
// ✅ Good
.map_err(|e| ThingsError::unknown(format!("Invalid task UUID: {e}")))?

return Err(ThingsError::invalid_parameter("uuid", format!("Invalid UUID format: {e}")));
```

### Missing Parameters

When required parameters are missing, use specific error types:

```rust
// ✅ Good
return Err(McpError::MissingParameter { 
    parameter_name: "query".to_string() 
});

.ok_or_else(|| ThingsError::unknown(format!("Missing required field: {field_name}")))?
```

## Error Type Guidelines

### `ThingsError::unknown()`

Use for unexpected errors or when no more specific error type applies:

```rust
.map_err(|e| ThingsError::unknown(format!("Failed to serialize tags: {e}")))?
```

### `ThingsError::Database()`

Use for database-specific errors in test utilities:

```rust
.map_err(|e| ThingsError::Database(format!("Failed to create test schema: {e}")))?
```

### `McpError::{specific_variant}()`

Use MCP-specific error types when available:

```rust
McpError::MissingParameter { parameter_name }
McpError::InvalidFormat { format, supported }
McpError::ToolNotFound { tool_name }
```

## Pattern Examples

### Database Operations

```rust
// Query execution
sqlx::query("SELECT * FROM TMTask")
    .fetch_all(&pool)
    .await
    .map_err(|e| ThingsError::unknown(format!("Failed to fetch tasks: {e}")))?;

// Connection
ThingsDatabase::new(path)
    .await
    .map_err(|e| ThingsError::unknown(format!("Failed to connect to database: {e}")))?;

// Validation
validate_task_exists(&pool, &uuid)
    .await
    .map_err(|e| ThingsError::unknown(format!("Failed to validate task: {e}")))?;
```

### MCP Tool Operations

```rust
// Tool call
server.call_tool(request)
    .await
    .map_err(|e| McpError::ToolExecutionFailed { 
        tool_name: "get_inbox".to_string(),
        reason: format!("Failed to execute tool: {e}")
    })?;

// Parameter extraction
arguments
    .get("query")
    .and_then(|v| v.as_str())
    .ok_or_else(|| McpError::MissingParameter {
        parameter_name: "query".to_string()
    })?;
```

## Anti-Patterns to Avoid

### ❌ Inconsistent Verb Forms

```rust
// Don't mix "failed", "could not", "unable to", etc.
"Connection failed: {e}"        // Inconsistent
"Could not connect: {e}"        // Inconsistent
"Unable to connect: {e}"        // Inconsistent

// Use
"Failed to connect: {e}"        // ✅ Consistent
```

### ❌ Missing Context

```rust
// Don't omit what operation failed
"Operation failed: {e}"         // Too vague
"Error: {e}"                    // No context

// Use
"Failed to create task: {e}"    // ✅ Clear
"Failed to update project: {e}" // ✅ Specific
```

### ❌ Redundant Information

```rust
// Don't repeat "error" or "failed" in the error type and message
ThingsError::DatabaseError(format!("Database error: {e}"))  // Redundant

// Use
ThingsError::Database(format!("Failed to connect: {e}"))    // ✅ Clear
```

## Testing Error Messages

When writing tests for error handling, verify:

1. **Message format** follows the standard
2. **Error type** is appropriate
3. **Context** is included in the message

```rust
#[test]
fn test_error_message_format() {
    let result = validate_task_exists(&pool, &invalid_uuid).await;
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    
    // Verify format
    assert!(err.to_string().starts_with("Task not found:"));
    
    // Verify context
    assert!(err.to_string().contains(&invalid_uuid.to_string()));
}
```

## Migration Checklist

When updating existing error messages:

- [ ] Check current format
- [ ] Apply appropriate standard format
- [ ] Verify error type is correct
- [ ] Ensure context is preserved
- [ ] Update any related tests
- [ ] Run full test suite

## Future Improvements

### Localization Support

Error messages should be designed with future localization in mind:

```rust
// Good: Simple format, easy to localize
format!("Failed to connect to database: {e}")

// Avoid: Complex formatting that's hard to translate
format!("The database connection couldn't be established because {e}")
```

### Structured Error Data

Consider adding structured data to errors for programmatic handling:

```rust
pub struct OperationError {
    operation: String,
    entity: Option<String>,
    cause: Box<dyn Error>,
}
```

## Summary

**Standard format**: `"Failed to {operation}: {error}"`

**Key points**:
- Use "Failed to" prefix consistently
- Include the underlying error
- Be specific about what operation failed
- Use appropriate error types
- Keep messages concise and actionable

