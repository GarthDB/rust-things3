# API Stability Policy

This document outlines the API stability guarantees and versioning policy for the `things3-core`, `things3-common`, and `things3-cli` crates.

## Versioning Policy

We follow [Semantic Versioning (SemVer)](https://semver.org/):

- **MAJOR** version (1.0.0): Breaking changes that require code modifications
- **MINOR** version (0.1.0): New features added in a backward-compatible manner
- **PATCH** version (0.0.1): Backward-compatible bug fixes

## Current Status

**Current Version**: 0.2.0

All APIs are considered **stable** unless explicitly marked otherwise. As we approach 1.0.0, we will:

1. Freeze the API surface
2. Document any planned breaking changes
3. Provide migration guides for major version upgrades

## What Constitutes a Breaking Change?

The following changes are considered **breaking** and require a MAJOR version bump:

1. **Removing public APIs**: Functions, structs, enums, traits, or constants
2. **Changing function signatures**: Parameter types, return types, or adding required parameters
3. **Changing struct/enum fields**: Removing fields, changing field types, or making optional fields required
4. **Changing trait definitions**: Adding required methods or changing method signatures
5. **Changing behavior**: Modifying the semantics of existing APIs in ways that break user expectations
6. **Changing error types**: Modifying error variants in ways that break pattern matching

The following changes are **non-breaking** and can be done in MINOR or PATCH versions:

1. **Adding new APIs**: New functions, structs, enums, traits, or constants
2. **Adding optional parameters**: With default values or `Option<T>` types
3. **Adding fields to structs**: With default values or `Option<T>` types
4. **Adding enum variants**: Only if exhaustive matching is not required
5. **Deprecating APIs**: Marking APIs as deprecated (removal happens in next MAJOR version)
6. **Performance improvements**: As long as behavior remains the same
7. **Bug fixes**: Correcting incorrect behavior

## Deprecation Policy

1. **Deprecation Period**: APIs will be marked as `#[deprecated]` for at least one MINOR version before removal
2. **Deprecation Notice**: Deprecated APIs will include guidance on alternatives
3. **Removal**: Deprecated APIs are removed in the next MAJOR version

### Example Deprecation

```rust
#[deprecated(note = "Use `new_function` instead. This will be removed in 2.0.0")]
pub fn old_function() {
    // ...
}
```

## Stable APIs

All public APIs in the following modules are considered stable:

### things3-core

- `ThingsDatabase` - Core database access
- `models::*` - All data models (Task, Project, Area, Tag, etc.)
- `error::ThingsError` - Error types
- `database::*` - Database operations and utilities
- `export::*` - Data export functionality
- `backup::*` - Backup and restore functionality
- `cache::*` - Caching layer
- `config::*` - Configuration management
- `observability::*` - Metrics and health checks
- `performance::*` - Performance monitoring

### things3-common

- `constants::*` - Shared constants
- `utils::*` - Utility functions

### things3-cli

- CLI interface (command-line arguments and subcommands)
- MCP server implementation

## Experimental APIs

Currently, there are **no experimental APIs**. All public APIs are considered stable.

If we introduce experimental APIs in the future, they will be:

1. Marked with `#[doc(hidden)]` or behind a feature flag
2. Clearly documented as experimental
3. Subject to change without notice until stabilized

## Feature Flags

### things3-core

- `test-utils`: Enables test utilities (for testing only, not for production use)

## Migration Guides

When breaking changes are introduced, migration guides will be provided in:

- `docs/MIGRATION_GUIDE.md` - For major version upgrades
- Release notes in `CHANGELOG.md`

## Questions or Concerns?

If you have questions about API stability or need clarification on our versioning policy, please:

1. Open an issue on GitHub
2. Check the `CHANGELOG.md` for recent changes
3. Review the documentation for the specific API

## Future Considerations

As we approach 1.0.0, we will:

1. Finalize the API surface
2. Document any planned breaking changes for 2.0.0
3. Establish a deprecation timeline for any APIs that will change
4. Create comprehensive migration guides

