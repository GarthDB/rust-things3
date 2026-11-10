# Compatibility Matrix

This document describes the compatibility requirements and test matrix for `rust-things3`.

## Minimum Supported Rust Version (MSRV)

**MSRV: Rust 1.89.0**

The library is tested and guaranteed to work with Rust 1.89.0 and later. This MSRV is documented in `Cargo.toml` via the `rust-version` field.

### MSRV Policy

- **Stability**: We aim to keep the MSRV stable for at least 6 months
- **Updates**: MSRV may be bumped in minor version releases (0.x.0)
- **Notice**: MSRV changes will be clearly documented in the changelog
- **Testing**: CI tests against MSRV, stable, and beta Rust versions

### Why Rust 1.89?

Rust 1.89.0 is required by `serde-saphyr`, the pure-Rust YAML library that replaced the unmaintained `serde_yaml`/`unsafe-libyaml`.

## Supported Platforms

### Important Note: Things 3 Availability

**Things 3 is an Apple ecosystem exclusive app**, available only on:
- macOS
- iOS (iPhone & iPad)  
- Apple Watch
- Apple Vision Pro

However, this library is useful beyond just macOS for:
- **Server-side processing** of exported Things 3 databases
- **CI/CD pipelines** (GitHub Actions, etc.)
- **Cross-platform tools** that process Things 3 data

### Tier 1: Fully Supported

These platforms are tested in CI and guaranteed to work:

| Platform | Architecture | Status | Notes |
|----------|-------------|--------|-------|
| macOS 14+ | x86_64 (Intel) | ✅ Supported | Primary platform - where Things 3 runs |
| macOS 14+ | ARM64 (Apple Silicon) | ✅ Supported | Native ARM support, tested on latest |
| Linux (Ubuntu) | x86_64 | ✅ Supported | For CI/CD and server-side processing |

### Tier 2: Should Work

These platforms should work but are not regularly tested:

| Platform | Architecture | Status | Notes |
|----------|-------------|--------|-------|
| Debian 11+ | x86_64 | ⚠️ Untested | Similar to Ubuntu |
| Fedora 38+ | x86_64 | ⚠️ Untested | Should work |
| Arch Linux | x86_64 | ⚠️ Untested | Should work |

### Tier 3: Not Supported

These platforms are not supported:

| Platform | Status | Reason |
|----------|--------|--------|
| Windows | ❌ Not Supported | Things 3 doesn't run on Windows; library not tested |
| 32-bit systems | ❌ Not Supported | Not tested, may have issues |
| BSD variants | ❌ Not Supported | Not tested |

## SQLite Compatibility

### Bundled SQLite (Default)

By default, the library uses **bundled SQLite** via `rusqlite` with the `bundled` feature:

- **Version**: SQLite 3.43.0+ (bundled with rusqlite)
- **Advantages**:
  - Consistent behavior across platforms
  - No system dependencies
  - Known-good configuration
- **Recommended**: Yes, for most users

### System SQLite

The library can also use system-installed SQLite:

- **Minimum Version**: SQLite 3.35.0+
- **Tested Versions**: 3.35.0, 3.40.0, 3.43.0+
- **Configuration**: Remove `bundled` feature from `rusqlite`

#### System SQLite Requirements

- **macOS**: Built-in SQLite (3.39.5+ on macOS 13+)
- **Ubuntu**: `libsqlite3-dev` package
- **Fedora**: `sqlite-devel` package

### SQLite Features Used

The library requires these SQLite features:
- JSON functions (`json_extract`, `json_each`)
- Common Table Expressions (CTEs)
- Window functions
- Foreign key support
- WAL mode (optional, for better concurrency)

## Dependency Compatibility

### Core Dependencies

| Dependency | Minimum Version | Notes |
|------------|----------------|-------|
| `tokio` | 1.0 | Async runtime |
| `sqlx` | 0.8 | Database access |
| `rusqlite` | 0.31 | Bundled SQLite |
| `serde` | 1.0 | Serialization |
| `chrono` | 0.4 | Date/time handling |
| `uuid` | 1.0 | UUID generation |

### Optional Dependencies

| Dependency | Minimum Version | Feature | Notes |
|------------|----------------|---------|-------|
| `criterion` | 0.5 | Benchmarks | Dev dependency |
| `proptest` | 1.0 | Property testing | Dev dependency |
| `tempfile` | 3.0 | Testing | Dev dependency |

## Things 3 Database Compatibility

### Supported Things 3 Versions

| Things 3 Version | Database Schema | Status |
|-----------------|-----------------|--------|
| 3.20+ | Current | ✅ Fully Supported |
| 3.15-3.19 | Legacy | ⚠️ Mostly Compatible |
| < 3.15 | Old | ❌ Untested |

### Database Schema Requirements

The library expects the following tables:
- `TMTask` - Tasks and projects
- `TMArea` - Areas
- `TMTag` - Tags
- `TMTaskTag` - Task-tag relationships

### Schema Compatibility Notes

- **Forward Compatible**: New Things 3 columns are ignored
- **Backward Compatible**: Missing optional columns are handled gracefully
- **Breaking Changes**: Major schema changes may require library updates

## Testing Matrix

### CI/CD Testing

Our CI pipeline tests the following combinations:

#### Rust Versions
- ✅ 1.89.0 (MSRV)
- ✅ stable
- ✅ beta

#### Operating Systems
- ✅ Ubuntu 22.04 LTS
- ✅ Ubuntu 24.04 LTS
- ✅ macOS 13 (Intel)
- ✅ macOS 14 (Apple Silicon)
- ✅ macOS latest

#### SQLite Versions
- ✅ Bundled (rusqlite default)
- ✅ System (Ubuntu)
- ✅ System (macOS)

### Test Coverage

| Test Type | Coverage | Notes |
|-----------|----------|-------|
| Unit Tests | 85%+ | Core functionality |
| Integration Tests | 80%+ | End-to-end scenarios |
| Benchmarks | N/A | Performance regression |
| Reliability Tests | 100% | Concurrency, edge cases |

## Known Limitations

### Platform-Specific

#### macOS
- ✅ Full support
- ✅ Native Things 3 database access
- ✅ Apple Silicon optimized

#### Linux
- ✅ Full library support
- ⚠️ Things 3 not available natively
- ℹ️ Useful for server-side processing

#### Windows
- ⚠️ Untested
- ⚠️ Path handling may differ
- ⚠️ Things 3 not available
- 💡 Consider WSL2 for Windows users

### SQLite Limitations

- **Concurrency**: Write operations are serialized by SQLite
- **File Locking**: May have issues with network filesystems
- **WAL Mode**: Requires SQLite 3.7.0+ (we require 3.35.0+)

### Performance Considerations

| Dataset Size | Performance | Notes |
|--------------|-------------|-------|
| < 1,000 tasks | Excellent | < 5ms queries |
| 1,000-10,000 tasks | Good | 5-50ms queries |
| 10,000-100,000 tasks | Fair | 50-500ms queries |
| > 100,000 tasks | Slow | Consider pagination |

## Compatibility Testing

### Running Compatibility Tests Locally

```bash
# Test with MSRV
rustup install 1.89.0
cargo +1.89.0 test --workspace --features test-utils

# Test with stable
cargo test --workspace --features test-utils

# Test with beta
rustup install beta
cargo +beta test --workspace --features test-utils

# Test with system SQLite (Linux)
sudo apt-get install libsqlite3-dev
cargo test --workspace --features test-utils

# Run benchmarks
cargo bench --workspace --features test-utils
```

### Continuous Integration

Compatibility tests run automatically:
- On every push to `main`
- On every pull request
- Weekly (scheduled)
- On demand (workflow_dispatch)

See `.github/workflows/compatibility.yml` for details.

## Reporting Compatibility Issues

If you encounter compatibility issues:

1. **Check this document** for known limitations
2. **Search existing issues** on GitHub
3. **Open a new issue** with:
   - Rust version (`rustc --version`)
   - Platform (OS, architecture)
   - SQLite version (if using system SQLite)
   - Things 3 version (if applicable)
   - Minimal reproduction steps

## Future Compatibility Plans

### Planned Support

- [ ] Windows native support (0.4.0)
- [ ] FreeBSD testing (0.5.0)
- [ ] WASM target (0.6.0)
- [ ] Async runtime alternatives (0.7.0)

### MSRV Updates

Potential MSRV updates:
- Raised to 1.89.0 for `serde-saphyr` (unreleased)

Updates will be announced in advance and documented in the changelog.

## Version History

| Version | MSRV | Notable Changes |
|---------|------|-----------------|
| Unreleased | 1.89.0 | Raised for `serde-saphyr` |
| 0.2.0 | 1.70.0 | Initial MSRV declaration |
| 0.1.0 | N/A | Pre-release |

## Resources

- [Rust Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
- [SQLite Version History](https://www.sqlite.org/changes.html)
- [Things 3 Release Notes](https://culturedcode.com/things/support/articles/2803573/)
- [CI/CD Workflows](../.github/workflows/)

## Questions?

For compatibility questions:
- Open an issue on GitHub
- Check the FAQ in README.md
- Review existing compatibility issues

