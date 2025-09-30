# Contributing to Rust Things

Thank you for your interest in contributing to Rust Things! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Code Style Guidelines](#code-style-guidelines)
- [Testing Guidelines](#testing-guidelines)
- [Pull Request Process](#pull-request-process)
- [Issue Reporting](#issue-reporting)
- [Release Process](#release-process)
- [Architecture Overview](#architecture-overview)

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). By participating, you are expected to uphold this code.

## Getting Started

### Prerequisites

- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs/)
- **Moon**: Install from [moonrepo.dev](https://moonrepo.dev/install)
- **Git**: For version control
- **Things 3**: For testing (macOS)

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/rust-things3.git
   cd rust-things
   ```
3. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/GarthDB/rust-things3.git
   ```

## Development Setup

### Initial Setup

```bash
# Install dependencies and setup development environment
moon run :local-dev-setup

# Run the development pipeline (format, lint, test)
moon run :dev-pipeline
```

### Project Structure

```
rust-things3/
├── apps/
│   └── things3-cli/          # CLI application with MCP server
├── libs/
│   ├── things3-core/         # Core library (database, models, etc.)
│   └── things3-common/       # Shared utilities
├── tools/
│   └── xtask/                # Development tools and scripts
├── docs/                     # Documentation
├── tests/                    # Integration tests
└── scripts/                  # Build and deployment scripts
```

### Moon Commands

```bash
# Run all tests
moon run :test-all

# Run specific project tests
moon run things3-core:test
moon run things3-cli:test

# Run development pipeline (format, lint, test)
moon run :dev-pipeline

# Run benchmarks
moon run :bench

# Generate code coverage
moon run :coverage

# Clean build artifacts
moon run :clean
```

## Code Style Guidelines

### Rust Code Style

We follow standard Rust conventions and use automated tools:

- **rustfmt**: Automatic code formatting
- **clippy**: Linting with `-D warnings` (all warnings are errors)
- **cargo check**: Compilation checks

### Code Organization

- **Modules**: Use clear, descriptive module names
- **Functions**: Keep functions focused and single-purpose
- **Error Handling**: Use `anyhow::Result` for application code, custom `ThingsError` for library code
- **Documentation**: Document all public APIs with `///` comments
- **Tests**: Place unit tests in `#[cfg(test)]` modules

### Naming Conventions

- **Crates**: `kebab-case` (e.g., `things3-core`)
- **Modules**: `snake_case`
- **Functions**: `snake_case`
- **Types**: `PascalCase`
- **Constants**: `SCREAMING_SNAKE_CASE`

### Example Code Style

```rust
/// Creates a new task in the Things 3 database.
///
/// # Arguments
/// * `title` - The task title
/// * `project_uuid` - Optional project UUID
///
/// # Errors
/// Returns `ThingsError::Database` if the database operation fails
pub async fn create_task(
    &self,
    title: String,
    project_uuid: Option<Uuid>,
) -> Result<Task> {
    // Implementation here
}
```

## Testing Guidelines

### Test Structure

- **Unit Tests**: In `#[cfg(test)]` modules within source files
- **Integration Tests**: In `tests/` directory
- **MCP Tests**: In `apps/things3-cli/tests/`
- **CI Tests**: In `libs/things3-core/tests/ci_tests.rs`

### Test Requirements

- **Coverage**: Maintain >90% code coverage
- **Naming**: Use descriptive test names (e.g., `test_create_task_with_valid_input`)
- **Isolation**: Tests should not depend on external state
- **Mocking**: Use test utilities for database operations

### Running Tests

```bash
# Run all tests
cargo test

# Run with coverage
cargo llvm-cov --html

# Run specific test
cargo test test_create_task

# Run tests for specific crate
cargo test -p things3-core
```

### Test Utilities

Use the test utilities in `things3-core::test_utils`:

```rust
use things3_core::test_utils::create_test_database;

#[tokio::test]
async fn test_my_function() {
    let db = create_test_database().await.unwrap();
    // Test implementation
}
```

## Pull Request Process

### Before Submitting

1. **Sync with upstream**:
   ```bash
   git fetch upstream
   git checkout main
   git merge upstream/main
   ```

2. **Create feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make your changes** following the code style guidelines

4. **Run the development pipeline**:
   ```bash
   moon run :dev-pipeline
   ```

5. **Add tests** for new functionality

6. **Update documentation** if needed

### PR Requirements

- **Title**: Use conventional commit format (e.g., `feat: add task filtering`)
- **Description**: Explain what changes and why
- **Tests**: All tests must pass
- **Coverage**: Maintain or improve code coverage
- **Documentation**: Update docs for new features
- **Breaking Changes**: Clearly mark and document

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] All tests pass
- [ ] Manual testing completed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] No breaking changes (or clearly marked)
```

### Review Process

1. **Automated Checks**: CI must pass
2. **Code Review**: At least one maintainer approval
3. **Testing**: Manual testing for significant changes
4. **Documentation**: Ensure docs are updated

## Issue Reporting

### Bug Reports

Use the bug report template and include:

- **Description**: Clear description of the bug
- **Steps to Reproduce**: Detailed steps
- **Expected Behavior**: What should happen
- **Actual Behavior**: What actually happens
- **Environment**: OS, Rust version, Things 3 version
- **Logs**: Relevant error messages or logs

### Feature Requests

Use the feature request template and include:

- **Description**: Clear description of the feature
- **Use Case**: Why this feature is needed
- **Proposed Solution**: How you think it should work
- **Alternatives**: Other solutions considered

### Issue Labels

- `bug`: Something isn't working
- `enhancement`: New feature or request
- `documentation`: Improvements to documentation
- `good first issue`: Good for newcomers
- `help wanted`: Extra attention needed
- `question`: Further information is requested

## Release Process

### Versioning

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Release Steps

1. **Update version** in `Cargo.toml` files
2. **Update CHANGELOG.md** with changes
3. **Create release PR** with version bump
4. **Merge to main** after review
5. **Create GitHub release** with tag
6. **Publish to crates.io** (maintainers only)
7. **Update Homebrew formula** (maintainers only)

### Changelog Format

```markdown
## [1.2.0] - 2024-01-15

### Added
- New task filtering functionality
- Performance monitoring dashboard

### Changed
- Improved error messages
- Updated dependencies

### Fixed
- Memory leak in cache system
- Database connection timeout issue
```

## Architecture Overview

### Core Components

- **things3-core**: Database operations, models, caching
- **things3-common**: Shared utilities and helpers
- **things3-cli**: CLI application and MCP server

### Key Design Principles

- **Performance**: Optimized for speed and memory usage
- **Reliability**: Comprehensive error handling and testing
- **Extensibility**: Modular design for easy extension
- **Compatibility**: Works with existing Things 3 databases

### Database Layer

- **Connection Management**: Pooled connections with fallback
- **Caching**: High-performance caching with Moka
- **Query Building**: Type-safe query construction
- **Error Handling**: Comprehensive error types

### MCP Integration

- **Tool System**: Extensible tool architecture
- **Protocol Compliance**: Full MCP specification support
- **Error Handling**: Consistent error reporting
- **Performance**: Optimized for AI/LLM usage

## Getting Help

- **Discussions**: Use GitHub Discussions for questions
- **Issues**: Use GitHub Issues for bugs and features
- **Documentation**: Check the `docs/` directory
- **Examples**: See `docs/examples/` for usage examples

## Thank You

Thank you for contributing to Rust Things! Your contributions help make this project better for everyone.

---

For questions about contributing, please open a discussion or contact the maintainers.
