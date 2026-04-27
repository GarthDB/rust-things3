# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2026-04-27

### Added
- **Predictive cache preloading** — new `CachePreloader` trait registered on `ThingsCache` via `set_preloader`/`clear_preloader`. After every `get_*` access the cache calls `predict(key)` to enqueue follow-up keys for warming; the warming-loop tick then calls `warm(key)` for each top-priority queued entry (replacing the previous no-op stub at `start_cache_warming`). Ships a `DefaultPreloader { Weak<ThingsCache>, Arc<ThingsDatabase> }` with three hardcoded heuristics over existing keys: `inbox:all → today:all`, `today:all → inbox:all`, `areas:all → projects:all`. `CacheStats` gains `warmed_keys` and `warming_runs` counters so tests and operators can confirm the loop is doing work. Default behavior unchanged for callers that don't register a preloader.
- **Dependency-based cache invalidation** — `ThingsCache::invalidate_by_entity` / `invalidate_by_operation` now consult the `CacheDependency` list attached to each cached entry and evict only matching entries instead of nuking every cache. New `CacheDependency::matches` and `CacheDependency::matches_operation` helpers expose the matching rules. New `ThingsCacheInvalidationHandler` bridges `CacheInvalidationMiddleware` events into the cache so registering it makes `process_event(...)` actually evict dependent entries; cascade invalidation now reads `project_uuid` / `area_uuid` from `event.metadata` to populate concrete `entity_id`s on dependent events instead of `None`.

### Changed
- **⚠️ Breaking: `ThingsCache::invalidate_by_entity` and `invalidate_by_operation` are now `async fn`** — callers that invoked these as synchronous functions must `.await` them. Both now return the number of keys submitted for eviction (not a guarantee of immediate removal).
- **`batch-operations` feature flag** — gates new pagination, streaming, and batch-fetch APIs (1.2.0 milestone). Additive; default builds are unaffected.
- **Cursor-based pagination on `TaskQueryBuilder`** — new `cursor` module exposing opaque `Cursor` and `Page<T>` types, plus `TaskQueryBuilder::after(cursor)` and `execute_paged()` (gated on both `advanced-queries` and `batch-operations`). Cursor anchors on `(created, uuid)`: `created` is immutable so cursors stay valid under concurrent edits, and `uuid` provides a deterministic tiebreak. Cursor encoding is URL-safe base64 of a compact JSON payload. Default page size is 100 (overridable via `.limit()`). `execute_paged` returns `ThingsError::InvalidCursor` if `.offset()` and `.after()` are both set, or if `.fuzzy_search()` and `.after()` are both set.
- **Streaming results API on `TaskQueryBuilder`** — new `TaskQueryBuilder::execute_stream(db)` returns `Pin<Box<dyn Stream<Item = Result<Task>> + Send>>` (gated on both `advanced-queries` and `batch-operations`). Internally chunked via `execute_paged`: yields tasks in `(creationDate DESC, uuid DESC)` order, transparently fetching pages until exhausted. `.limit(n)` controls chunk size in the streaming context (default 100), not a cap on total emitted items. Validation errors (e.g. `.fuzzy_search()` + `.after()`) surface as the stream's first `Err` item. Pulls in `async-stream` and `futures-core` as optional deps under `batch-operations`.
- **Batch fetch-by-id primitives on `ThingsDatabase`** — new `batch` module with `ThingsDatabase::get_tasks_batch(uuids)` and `ThingsDatabase::get_projects_batch(uuids)` (gated behind `batch-operations`). Mirrors filtering semantics of `get_task_by_uuid` / `get_project_by_uuid` (trashed rows omitted). Empty input returns `Ok(vec![])` with no SQL roundtrip; duplicate input UUIDs are de-duplicated; results are ordered by `(creationDate DESC, uuid DESC)`. Internally chunks at 500 UUIDs per query so callers can pass arbitrarily long lists without hitting SQLite's `SQLITE_LIMIT_VARIABLE_NUMBER`.
- **`ThingsDatabase::query_tasks` ordering is now deterministic** — `ORDER BY` was tightened from `creationDate DESC` to `CAST(creationDate AS INTEGER) DESC, uuid DESC`. Previously, the order of tasks tied on truncated-second `creationDate` was unspecified; now it is well-defined. No effect on callers that already had distinct timestamps.

## [1.1.0] - 2026-04-26

### Added
- **`advanced-queries` feature flag** — gates new query execution APIs so existing builds are unaffected.
- **`ThingsDatabase::query_tasks(filters: &TaskFilters)`** — executes a dynamic SQL query driven by all `TaskFilters` fields: status, type, project, area, start date range, deadline range, limit, and offset. Tag and search-query filters are applied in Rust after the database fetch.
- **`TaskQueryBuilder::execute(&ThingsDatabase)`** — end-to-end shorthand that calls `.build()` and `query_tasks()` in one step.
- **Natural-language date helpers on `TaskQueryBuilder`** — `due_today`, `due_this_week`, `due_next_week`, `due_in(days)`, `overdue`, `starting_today`, `starting_this_week`. Pure builder sugar that delegates to existing `deadline_range` / `start_date_range` setters. Weeks are Monday-Sunday. `overdue()` also implies `status = Incomplete` when no status filter has been set.
- **Flexible tag operations on `TaskQueryBuilder`** — `any_tags(tags)` (OR semantics), `exclude_tags(tags)` (NOT IN), `tag_count(min)` (minimum tag-count threshold). These are builder-only predicates gated behind `advanced-queries`; they are applied in Rust inside `execute()` and are not reflected in `build()` / `TaskFilters`. Pagination correctly defers to Rust when any of these are active. Tag matching is case-sensitive, matching the existing `tags` (AND) filter.
- **Fuzzy search and ranked results on `TaskQueryBuilder`** — `fuzzy_search(query)` and `fuzzy_threshold(f32)` builder methods (gated behind `advanced-queries`). Scores are computed with windowed Levenshtein similarity over task title and notes; only tasks meeting the threshold (default `0.6`) are returned. `execute()` returns `Vec<Task>` sorted by score; `execute_ranked()` returns `Vec<RankedTask>` (each `RankedTask` carries the score). `execute_ranked()` errors if `fuzzy_search` is not set. When both `fuzzy_search` and `search` are active, fuzzy wins and a warning is logged.
- **Saved queries** — new `saved_queries` module with `SavedQuery` and `SavedQueryStore` (gated behind `advanced-queries`). `SavedQuery` captures full builder state including the post-1.0.0 builder-only predicates (`any_tags`, `exclude_tags`, `tag_count_min`, `fuzzy_query`, `fuzzy_threshold`). `SavedQueryStore` is a JSON-backed `HashMap<String, SavedQuery>` with atomic save (write-temp + rename) and permissive load (missing file returns empty store). New `TaskQueryBuilder::to_saved_query(name)` and `TaskQueryBuilder::from_saved_query(&saved)` round-trip a builder through a saved query.
- **Boolean expressions on `TaskQueryBuilder`** — new `filter_expr` module with `FilterExpr` (recursive `And` / `Or` / `Not` / `Pred`) and `FilterPredicate` (`Status`, `TaskType`, `Project`, `Area`, `HasTag`, `StartDateBefore` / `After`, `DeadlineBefore` / `After`, `TitleContains`, `NotesContains`). `FilterExpr` is a Rust-side post-filter applied by `execute()` after the SQL fetch and after `apply_tag_filters`; pagination correctly defers when `where_expr` is set. Vacuous semantics: `And(vec![])` → `true`, `Or(vec![])` → `false`. JSON encoding uses adjacent tagging (`{"op": ..., "args": ...}`, `{"kind": ..., "value": ...}`) so saved-query files stay human-readable. New `TaskQueryBuilder::where_expr(expr)` builder method, gated behind `advanced-queries`. `SavedQuery` gains a corresponding `where_expr: Option<FilterExpr>` field, additive and forward-compatible.

## [1.0.1] - 2026-04-22

### Fixed
- **`THINGS_DB_PATH` environment variable is now respected** (#76). The documented
  env var name was silently ignored because the code only read an undocumented
  `THINGS_DATABASE_PATH`. `ThingsConfig::from_env()` now reads `THINGS_DB_PATH`
  first; `THINGS_DATABASE_PATH` still works but logs a deprecation warning. The
  `--database` CLI flag continues to take precedence over both.

## [1.0.0] - 2026-01-07

### 🎉 Stable Release - Production Ready!

This is the first stable 1.0.0 release of `rust-things3`. All APIs are now frozen and follow semantic versioning guarantees. This release represents months of development, testing, and refinement across 5 major phases.

### Added - Phase 4: Ecosystem Integration

#### Feature Flags (Modular Compilation)
- **Feature flags** for optional functionality:
  - `export-csv`: CSV export support
  - `export-opml`: OPML export support
  - `observability`: Metrics, tracing, and health checks
  - `mcp-server`: MCP server functionality (CLI only)
  - `full`: Enable all features
  - `test-utils`: Testing utilities
- **Binary size optimization**: 24% reduction with minimal build
- **CI feature matrix**: 10 feature combinations tested
- **Comprehensive docs**: Feature compatibility matrix and usage examples

#### Integration Examples
- **5 comprehensive examples** demonstrating real-world usage:
  - `mcp_client`: Custom MCP client implementation
  - `cli_extension`: CLI extension with custom commands
  - `web_api`: REST API with Axum web framework
  - `background_service`: Long-running service with graceful shutdown
  - `custom_middleware`: Custom middleware implementation
- All examples include full documentation and runnable code

#### Community Resources
- **CODE_OF_CONDUCT.md**: Contributor Covenant v2.1
- **SECURITY.md**: Vulnerability reporting and security policy
- **GitHub issue templates**: 4 templates (bug, feature, docs, question)
- **Pull request template**: Comprehensive checklist
- **Enhanced CONTRIBUTING.md**: Architecture overview, testing guidelines, conventions

### Added - Phase 3: Performance & Reliability

#### Performance Enhancements
- **Query performance tracking**: Detailed metrics and optimization suggestions
- **Connection pooling**: Configurable pool with health monitoring
- **Multi-layer caching**: In-memory (L1) + disk (L2) with TTL support
- **SQLite optimizations**: WAL mode, memory mapping, query optimization
- **Comprehensive performance API**: Metrics, summaries, and analysis tools

#### Reliability Features
- **Git hooks**: Pre-commit (fmt + clippy) and commit-msg validation
- **Configuration hot reload**: Live config updates without restarts
- **Backup management**: Automated backups with metadata and verification
- **Health checks**: Database, memory, and overall system health
- **Comprehensive documentation**: RELIABILITY.md guide

### Improved

#### API Stability
- **Frozen stable APIs**: All public APIs follow semantic versioning
- **Consistent error handling**: Unified `Result<T, ThingsError>` pattern
- **Comprehensive examples**: All public APIs include usage examples
- **Type safety**: Strong typing throughout with zero unsafe code

#### Performance
- **90%+ code coverage**: Extensive test suite with edge cases
- **Optimized queries**: Indexed access and efficient SQL patterns
- **Memory efficiency**: Smart caching with automatic cleanup
- **Async/await**: Full async support with tokio runtime

#### Documentation
- **100% public API documentation**: Every public item documented
- **Integration guides**: Real-world usage patterns
- **Feature flag docs**: Clear guidance on modular compilation
- **Security audit**: Comprehensive security documentation

### Fixed

#### Security
- **RUSTSEC-2024-0363**: Upgraded sqlx from 0.8.0 to 0.8.6 (binary protocol vulnerability)
- **Dependency updates**: rusqlite 0.31 → 0.32, eliminated unmaintained `paste` crate
- **Security audit**: Comprehensive audit with documented accepted risks

#### Stability
- **Flaky tests**: Fixed timing-dependent test failures
- **Resource cleanup**: Proper cleanup in all code paths
- **Error handling**: Better error messages and recovery strategies
- **CI robustness**: More reliable CI pipeline with feature matrix

### Technical Details

#### Dependencies
- **sqlx**: 0.8.6 (was 0.8.0) - Security fix
- **rusqlite**: 0.32 (was 0.31) - Compatibility update
- **Total dependencies**: 449 crates
- **Security status**: 1 accepted low-risk warning (unused code path)

#### Test Coverage
- **Unit tests**: 443 passing
- **Integration tests**: 48 passing
- **Doc tests**: 23 passing
- **Coverage**: 80%+ (with 90% goal documented)
- **Feature combinations**: 10 tested in CI

#### Performance Benchmarks
- **Minimal build size**: 24% smaller than full build
- **Query performance**: < 1ms for indexed queries
- **Cache hit rate**: > 95% in typical usage
- **Memory footprint**: ~10MB for core + ~5MB per cached dataset

### Migration Guide

See [MIGRATION.md](docs/MIGRATION.md) for detailed upgrade instructions from 0.x to 1.0.0.

#### Breaking Changes
- **Default features**: Changed from all features to minimal (use `full` feature for old behavior)
- **API cleanup**: Some internal APIs moved or renamed (public API stable)
- **Configuration format**: Minor changes to config file structure (backward compatible)

#### Recommended Actions
1. Review feature flags and enable only what you need
2. Update `Cargo.toml` to use `features = ["full"]` for full compatibility
3. Run security audit: `cargo audit`
4. Review new examples for best practices

### Contributors
Thank you to everyone who contributed to this release through code, documentation, testing, and feedback!

---

## [0.2.0] - 2024-01-03

### Added
- **Configuration Management**: Comprehensive configuration system for MCP server with YAML/JSON support
- **Real-time Updates**: WebSocket-based real-time updates and progress tracking
- **Authentication & Rate Limiting**: JWT and API key authentication with configurable rate limiting
- **MCP Middleware System**: Extensible middleware framework for cross-cutting concerns
- **Enhanced Error Handling**: MCP-specific error types with detailed error context
- **MCP Prompts Support**: Reusable template system for MCP prompts
- **MCP Resources Pattern**: Structured data exposure through MCP resources
- **Structured Logging**: Comprehensive logging and metrics collection system
- **Performance Infrastructure**: Caching and performance optimization with monitoring
- **Comprehensive Test Coverage**: Significantly improved test coverage (60% → 80%+)

### Improved
- **Database Schema**: Aligned with real Things3 database structure
- **Code Quality**: Resolved all clippy warnings and linting issues
- **CI/CD Pipeline**: Enhanced reliability and coverage reporting
- **Test Reliability**: Fixed race conditions and environment-specific test issues
- **Documentation**: Added comprehensive guides and API documentation

### Fixed
- **Security Vulnerabilities**: Updated dependencies to resolve security issues
- **Environment Variable Parsing**: Improved configuration parsing reliability
- **Test Isolation**: Resolved test interference and race conditions
- **Database I/O**: Fixed database access issues in MCP tests
- **Memory Management**: Optimized memory usage in caching systems

### Technical Improvements
- **Code Coverage**: Extensive test coverage improvements across all modules
- **Performance Monitoring**: Added metrics collection and performance tracking
- **Error Recovery**: Enhanced error handling and recovery mechanisms
- **Concurrent Access**: Improved thread safety and concurrent operations
- **Resource Management**: Better resource cleanup and lifecycle management

## [0.1.0] - 2024-XX-XX

### Added
- Initial release with basic Things 3 database access
- Core data models and CLI interface
- Basic MCP server functionality
- SQLite database integration
- Export functionality (JSON, CSV, Markdown, OPML)