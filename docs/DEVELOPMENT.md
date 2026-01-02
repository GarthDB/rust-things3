# Development Guide

## Table of Contents
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Testing](#testing)
- [Debugging](#debugging)
- [Performance](#performance)
- [Common Issues](#common-issues)

## Getting Started

### Prerequisites

- **Rust**: 1.70+ (install via [rustup](https://rustup.rs/))
- **Moon**: Workspace management ([install](https://moonrepo.dev/docs/install))
- **Things 3**: For testing with real data
- **cargo-llvm-cov**: For coverage reports

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Moon
curl -fsSL https://moonrepo.dev/install | bash

# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Install LLVM tools
rustup component add llvm-tools-preview
```

### Clone and Setup

```bash
# Clone repository
git clone https://github.com/GarthDB/rust-things3
cd rust-things3

# Run setup
moon run :local-dev-setup

# Verify installation
cargo test --workspace
```

## Development Workflow

### Moon Tasks

```bash
# Development pipeline (format, lint, test)
moon run :dev-pipeline

# Run all tests
moon run :test-all

# Format code
moon run :format

# Lint code
moon run :lint

# Generate documentation
moon run :docs

# Coverage report
moon run :coverage
```

### Cargo Commands

```bash
# Build
cargo build --workspace

# Build release
cargo build --workspace --release

# Run CLI
cargo run --bin things3 -- --help

# Run specific test
cargo test --package things3-core test_name

# Watch mode (requires cargo-watch)
cargo watch -x test
```

### Git Hooks

Pre-commit hooks automatically run:
- `cargo fmt` - Format code
- `cargo clippy` - Lint code
- Commit message validation

## Testing

### Test Structure

```
tests/
├── Unit tests (#[cfg(test)] in source files)
├── Integration tests (tests/ directories)
└── Test utilities (test_utils.rs)
```

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific package
cargo test --package things3-core

# Specific test
cargo test test_get_inbox

# With output
cargo test -- --nocapture

# Single-threaded
cargo test -- --test-threads=1
```

### Test Categories

**Unit Tests** (554 total):
- Database operations (Phase 1)
- MCP I/O layer (Phase 2)
- Middleware chain (Phase 3)
- Observability (Phase 4)

**Integration Tests**:
- `mcp_io_tests.rs` - MCP server I/O
- `middleware_integration_tests.rs` - Middleware
- `observability_integration_tests.rs` - Observability
- `ci_tests.rs` - CI-friendly tests

### Writing Tests

```rust
#[tokio::test]
async fn test_database_operation() {
    let db = ThingsDatabase::new(test_db_path()).await.unwrap();
    let tasks = db.get_inbox(Some(10)).await.unwrap();
    assert!(!tasks.is_empty());
}
```

### Coverage

```bash
# Generate HTML report
cargo llvm-cov --workspace --all-features --html

# Open report
open target/llvm-cov/html/index.html

# Text summary
cargo llvm-cov --workspace --all-features --text
```

## Debugging

### Logging

```bash
# Enable debug logging
export RUST_LOG=debug
cargo run --bin things3 -- inbox

# Specific module
export RUST_LOG=things3_core::database=trace
cargo run --bin things3 -- inbox

# Multiple modules
export RUST_LOG=things3_core=debug,things3_cli=info
```

### VS Code Debugging

`.vscode/launch.json`:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug things3",
      "cargo": {
        "args": ["build", "--bin=things3"],
        "filter": {
          "name": "things3",
          "kind": "bin"
        }
      },
      "args": ["inbox"],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_LOG": "debug"
      }
    }
  ]
}
```

### Database Inspection

```bash
# Open database in SQLite
sqlite3 ~/Library/Group\ Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-*/Things\ Database.thingsdatabase/main.sqlite

# List tables
.tables

# Describe table
.schema TMTask

# Query
SELECT title, status FROM TMTask LIMIT 10;
```

## Performance

### Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Profile application
cargo flamegraph --bin things3 -- inbox

# View flamegraph.svg
open flamegraph.svg
```

### Benchmarking

```bash
# Run benchmarks
cargo bench --workspace

# Specific benchmark
cargo bench --package things3-core -- database
```

### Performance Tips

1. **Use connection pooling** for concurrent access
2. **Cache expensive queries** with ThingsCache
3. **Limit query results** with LIMIT clause
4. **Use indexes** for WHERE clauses
5. **Profile before optimizing**

## Common Issues

### Database Not Found

**Issue**: `Database file not found`

**Solution**:
```bash
# Find database
find ~/Library/Group\ Containers -name "main.sqlite" 2>/dev/null

# Set path
export THINGS_DB_PATH="/path/to/main.sqlite"
```

### Permission Denied

**Issue**: `Permission denied` when accessing database

**Solution**:
```bash
# Check permissions
ls -la ~/Library/Group\ Containers/JLMPQHK86H.com.culturedcode.ThingsMac

# Ensure Things 3 is closed
killall Things3
```

### Test Failures

**Issue**: Tests fail with "database locked"

**Solution**:
```bash
# Run tests single-threaded
cargo test -- --test-threads=1

# Close Things 3 app
killall Things3
```

### Coverage Tool Issues

**Issue**: `failed to find llvm-tools-preview`

**Solution**:
```bash
# Install LLVM tools
rustup component add llvm-tools-preview

# Verify installation
rustup component list | grep llvm
```

### Build Errors

**Issue**: Compilation errors after git pull

**Solution**:
```bash
# Clean build
cargo clean

# Update dependencies
cargo update

# Rebuild
cargo build --workspace
```

## Code Style

### Formatting

```bash
# Format all code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check
```

### Linting

```bash
# Run clippy
cargo clippy --workspace -- -D warnings

# Fix automatically
cargo clippy --workspace --fix
```

### Conventions

- **Naming**: snake_case for functions/variables, PascalCase for types
- **Error Handling**: Use `Result<T, ThingsError>`
- **Async**: Prefer async/await over futures combinators
- **Documentation**: Add doc comments for public APIs
- **Tests**: One test per behavior, descriptive names

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for:
- Code review process
- Pull request guidelines
- Issue reporting
- Commit message format

## Resources

- [Architecture](./ARCHITECTURE.md)
- [MCP Integration](./MCP_INTEGRATION.md)
- [Database Schema](./DATABASE_SCHEMA.md)
- [Coverage Analysis](./COVERAGE_ANALYSIS.md)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [SQLx Documentation](https://docs.rs/sqlx/)

