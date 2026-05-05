# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking Changes

- **`ThingsId` replaces `Uuid` for all entity identifiers** (#139) — the `Uuid` type from the
  `uuid` crate no longer appears in any public API for task, project, area, tag, or heading IDs.
  All methods on `ThingsDatabase`, all `MutationBackend` trait methods, and all request/response
  model types now use `ThingsId` (from `things3_core::ThingsId`).

  **Migration guide for callers:**
  - `ThingsDatabase::create_task(...)` now returns `ThingsResult<ThingsId>` (was `ThingsResult<Uuid>`)
  - All `&Uuid` parameters become `&ThingsId`; construct with `ThingsId::from_str(s)?` at API
    boundaries or `ThingsId::new_v4()` for fresh IDs
  - `Uuid::parse_str(s)` → `ThingsId::from_str(s)` (add `use std::str::FromStr;`)
  - `.bind(uuid.to_string())` in sqlx queries → `.bind(id.as_str())`

- **`things_uuid_to_uuid` deleted** — the `pub(crate)` function in `database/core.rs` that
  hashed Things native IDs into `Uuid` values was lossy and non-deterministic (`DefaultHasher`
  is randomized per process). Any code that called it must switch to storing the raw `ThingsId`
  string directly.

- **`MutationBackend` trait** — all 21 method signatures updated to use `ThingsId` for entity
  IDs. Existing impls of `MutationBackend` must update their method signatures to match.

### Changed

- **Default mutation backend on macOS is now `AppleScriptBackend`** (#125). All MCP write
  tools route through the Things 3 app via osascript per CulturedCode's safety guidance
  (https://culturedcode.com/things/support/articles/5510170/), eliminating the
  data-corruption risk of direct SQLite writes. Linux/CI continues to use `SqlxBackend` as
  the default (no Things 3 install to corrupt). **Closes #120.**

### Deprecated

- **`SqlxBackend` (direct SQLite writes) is gated behind `--unsafe-direct-db`** /
  `THINGS_UNSAFE_DIRECT_DB=1`. Setting the flag emits a loud multi-line startup banner
  (suppressed in MCP mode for JSON-RPC purity). The flag will be removed in a future
  release; integrations writing directly to the database should migrate to AppleScript.
- **`restore_database` MCP tool now requires both `--unsafe-direct-db` AND that Things 3 is
  not running** (`pgrep -x Things3` returns nothing). The argument-parse error
  (`backup_path` missing) still fires before the gate so client error handling stays
  unchanged for that case. **Closes #126.**

### Added

- **AppleScriptBackend Phase E: live integration tests + docs** (#137) — closes #124. New
  `libs/things3-core/tests/applescript_live.rs` with `THINGS3_LIVE_TESTS=1`-gated lifecycle
  tests for tasks, projects, areas, and tags (one per domain). Each test creates uniquely-named
  entities, exercises the canonical create→update→…→delete flow through the `MutationBackend`
  trait surface, and removes them on completion. A `Drop` guard runs the deletion on a
  freshly-spawned single-threaded tokio runtime so cleanup still fires if a test panics. The
  README's `## Testing` section gains a "Running live AppleScript tests" subsection covering
  prerequisites (Things 3 install, Automation TCC grant) and the required `--test-threads=1`
  flag. The previous in-module `task_lifecycle_round_trip` was migrated verbatim into the new
  file. The production default-switch and the `--unsafe-direct-db` opt-out remain in #125.
- **AppleScriptBackend Phase D: tag operations** (#136) — implements the remaining 7
  `MutationBackend` stubs in `AppleScriptBackend`: `create_tag` (with `force` flag, smart-flow
  read via `find_tag_by_normalized_title` + `find_similar_tags` ≥0.8 before any AS write),
  `update_tag`, `delete_tag` (`remove_from_tasks=true` rewrites each affected task's `tag names`
  in a single bulk osascript invocation, then deletes the tag — a strict capability improvement
  over `SqlxBackend`, which has this branch as a TODO), `merge_tags` (replaces source with target
  in every affected task's `tag names`, then deletes source), `add_tag_to_task`,
  `remove_tag_from_task`, `set_task_tags`. Three methods are read+write hybrids: the read side
  computes similarity scores from the live DB and short-circuits with `Suggestions` /
  `SimilarFound` before any AS spawn, while only the unambiguous-write path routes through
  osascript. Things AppleScript does not expose `shortcut` or `parent` properties on `tag`, so
  `CreateTagRequest::shortcut` / `parent_uuid` and the equivalents on `UpdateTagRequest` are
  silently dropped with a `tracing::debug!` log. Closes #124 (with #137).
- **AppleScriptBackend Phase C: projects, areas, bulk operations** (#135) — implements 12 of the 16
  remaining `MutationBackend` stubs in `AppleScriptBackend`: `create_project`, `update_project`,
  `complete_project`, `delete_project` (all three `ProjectChildHandling` modes — Error, Cascade,
  Orphan); `create_area`, `update_area`, `delete_area`; and the five bulk ops
  (`bulk_create_tasks`, `bulk_delete`, `bulk_move`, `bulk_update_dates`, `bulk_complete`). Each
  bulk op runs as a single `osascript` invocation with per-item `try`/`on error` blocks; partial
  failures surface via `BulkOperationResult.message`. The 1000-item batch cap and empty-array
  validation mirror `SqlxBackend`. Tag operations remain stubbed (Phase D, #136).
- **AppleScriptBackend Phase C followups** — post-merge hardening of #142:
  - Document the fail-fast contract on the remaining cascade/orphan project
    script builders (`cascade_delete_project_script`,
    `orphan_complete_project_script`, `orphan_delete_project_script`) to
    match `cascade_complete_project_script`.
  - Cap project-cascade child counts at `MAX_BULK_BATCH_SIZE` in
    `complete_project` and `delete_project` so the generated osascript
    payload stays bounded for the same reason `bulk_*` operations cap.
  - Replace the silent `bulk_wrap(&[])` no-op fallback in `bulk_move_script`
    with `unreachable!`, asserting the destination invariant the caller
    already validates.
  - Clamp `parse_bulk_result`'s `processed` count against `total` so a
    future script-generation bug cannot report more processed items than
    were requested.
- **`ThingsId` type** (#139) — `things3_core::ThingsId` is a transparent newtype over `String`
  that accepts both RFC-4122 UUIDs (from `SqlxBackend`-created entities, e.g.
  `550e8400-e29b-41d4-a716-446655440000`) and Things native IDs (21–22-char base62 strings the
  Things 3 app itself produces, e.g. `R4t2G8Q63aGZq4epMHNeCr`). Serializes as a bare JSON string
  (`#[serde(transparent)]`). `FromStr` validates strictly at MCP/API boundaries; internal DB reads
  use `ThingsId::from_trusted` to avoid re-parsing values the DB already owns. Implements
  `Display`, `Hash`, `Eq`, `Ord`, `From<Uuid>`.

## [1.4.0] - 2026-04-28

### Added
- **Foundational `things3` agent skill** — new `skills/things3/` directory with an [agentskills.io](https://agentskills.io/specification)-compliant `SKILL.md`, plus `references/TOOLS.md` (complete 46-tool catalog sourced from `mcp.rs`, superseding the outdated 21-tool list in `docs/MCP_INTEGRATION.md`) and `references/HOSTS.md` (copy-paste MCP config snippets for Claude Desktop, Claude Code, Cursor, VS Code, and Zed). Reconciles the tool-count discrepancy in `docs/MCP_INTEGRATION.md`. Closes #111.
- **`things3-daily-review` workflow skill** — new `skills/things3-daily-review/SKILL.md` with a read-only daily-review recipe: pulls `get_today`, `get_inbox`, and `get_recent_tasks`; produces a structured Markdown summary grouped by area and project with overdue items flagged. References the foundational `things3` skill for MCP setup without duplicating install instructions. Closes #112.
- **Skills catalog README** (#115) — new `skills/README.md` listing the shipped skills with one-line descriptions and per-host install instructions (Claude Code; generic "check your host's docs" pointer for Claude Desktop / Cursor / Zed). Top-level `README.md` gains a "Use with Claude Code / your AI agent" section linking to the catalog. Tool count in the MCP Integration section corrected from 21 to 46.
- **CI: skills frontmatter validation** (#114) — new `.github/workflows/skills.yml` runs `agentskills validate` (PyPI `skills-ref==0.1.1`) on every `skills/*/` directory for PRs that touch `skills/**`. Fails the job if any skill is spec-invalid.

### Fixed
- **MCP tool errors no longer drop the connection** (#148) — when a `tools/call`, `resources/read`, or `prompts/get` handler returned an error (e.g. AppleScriptBackend rejecting a malformed task ID with `-1728`), the request loop's `?` propagation terminated the loop, surfacing to the client as `MCP error -32000: Connection closed`. Tool/resource/prompt errors now flow through the existing `*_with_fallback` variants and come back as structured `isError: true` envelopes inside the JSON-RPC `result`; any remaining handler-level error is converted to a JSON-RPC error response (-32600) instead of killing the loop. The server stays up; subsequent requests are answered.
- **MCP `Prompt.arguments` shape** (#119) — `Prompt.arguments` was a `serde_json::Value` holding a JSON Schema object, which is spec-invalid; the MCP 2025-11-25 spec requires `Vec<PromptArgument>`. Adds a `PromptArgument` struct (`name`, optional `description`, `required: bool`), rewrites the four `create_*_prompt()` helpers, and enables full `ListPromptsResult` schema validation in `test_prompts_list` (previously skipped because of this bug). Spec-strict clients like Claude Code 2.1+ now render prompt arguments correctly.

## [1.3.0] - 2026-04-27

### Added
- **TaskPaper export format** (#105 → #107) — new `export-taskpaper` feature flag adds an `ExportFormat::TaskPaper` variant on `DataExporter::export()`, plus `"taskpaper"` / `"tp"` in the `FromStr` parser. TaskPaper is a plain-text outline format consumed by Hog Bay Software's TaskPaper / Taskmator and other macOS scratch-list apps. Areas render as top-level project headers, projects nest under their area at one tab, tasks at two; status maps to `@done(stop_date)` / `@cancelled` / `@trashed`; deadlines and start dates to `@due(date)` / `@start(date)`; tags become sanitized `@tag` tokens (whitespace runs collapse to `-`, `@`, `(`, `)`, and control chars are stripped). Subtasks recurse via `task.children` or `parent_uuid`. No new external dependencies.
- **iCalendar (.ics) export format** (#106 → #108) — new `export-ical` feature flag adds an `ExportFormat::ICalendar` variant plus `"ical"` / `"ics"` / `"icalendar"` in the `FromStr` parser. All Things tasks and projects map to RFC 5545 `VTODO` components (no `VEVENT` — Things items are to-dos, not time-bounded events). Areas surface as `CATEGORIES` entries on each task/project rather than standalone components. Tasks emit `RELATED-TO:<project-uid>` for project linkage, plus an additional `RELATED-TO:<parent-uid>` for subtask hierarchy. UIDs are the Things UUID strings, so re-exports update existing entries rather than duplicate. Status maps `Incomplete → STATUS:NEEDS-ACTION`, `Completed → STATUS:COMPLETED` (+ `COMPLETED:<stop_date>` when present), `Canceled` / `Trashed → STATUS:CANCELLED`. `NaiveDate` fields use the `DATE` value type; `DateTime<Utc>` fields use `DATE-TIME` UTC. Pulls in `icalendar` 0.17 as a new optional dep.

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