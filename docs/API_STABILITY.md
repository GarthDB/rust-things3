# API Stability Guarantee - Version 1.0.0

**Last Updated**: January 2026  
**Status**: Frozen for 1.x series  
**Applies To**: `things3-core` v1.0.0+

---

## Overview

This document defines the **stable API surface** for `rust-things3` version 1.0.0. All items listed here are guaranteed to remain backward compatible throughout the 1.x series, following [Semantic Versioning 2.0.0](https://semver.org/).

### Stability Guarantees

**Within 1.x series:**
- ✅ No breaking changes to public APIs listed here
- ✅ New functionality may be added
- ✅ Deprecations announced 2 minor versions ahead
- ✅ Bug fixes may change behavior if documented as bugs

**For 2.0.0:**
- ⚠️ Breaking changes allowed with migration guide
- ⚠️ Deprecated items may be removed
- ⚠️ API evolution based on community feedback

---

## Stable Public API

### Core Types (`things3-core`)

#### Database Access

```rust
pub struct ThingsDatabase { /* ... */ }

impl ThingsDatabase {
    // Constructor methods
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self>;
    pub async fn new_with_config(config: ThingsConfig) -> Result<Self>;
    pub async fn new_with_default_path() -> Result<Self>;
    
    // Query methods
    pub async fn get_inbox(&self, limit: Option<usize>) -> Result<Vec<Task>>;
    pub async fn get_today(&self, limit: Option<usize>) -> Result<Vec<Task>>;
    pub async fn get_upcoming(&self, limit: Option<usize>) -> Result<Vec<Task>>;
    pub async fn get_anytime(&self, limit: Option<usize>) -> Result<Vec<Task>>;
    pub async fn get_someday(&self, limit: Option<usize>) -> Result<Vec<Task>>;
    pub async fn get_logbook(&self, limit: Option<usize>) -> Result<Vec<Task>>;
    pub async fn get_trash(&self, limit: Option<usize>) -> Result<Vec<Task>>;
    
    pub async fn get_projects(&self, area_uuid: Option<Uuid>, limit: Option<usize>) -> Result<Vec<Project>>;
    pub async fn get_areas(&self, limit: Option<usize>) -> Result<Vec<Area>>;
    pub async fn get_tags(&self) -> Result<Vec<Tag>>;
    
    // Search methods
    pub async fn search_tasks(&self, query: &str) -> Result<Vec<Task>>;
    pub async fn search_projects(&self, query: &str) -> Result<Vec<Project>>;
    pub async fn search_areas(&self, query: &str) -> Result<Vec<Area>>;
    pub async fn search_tags(&self, query: &str) -> Result<Vec<Tag>>;
    
    // Lookup methods
    pub async fn get_task_by_uuid(&self, uuid: Uuid) -> Result<Option<Task>>;
    pub async fn get_project_by_uuid(&self, uuid: Uuid) -> Result<Option<Project>>;
    pub async fn get_area_by_uuid(&self, uuid: Uuid) -> Result<Option<Area>>;
    pub async fn get_tag_by_title(&self, title: &str) -> Result<Option<Tag>>;
    
    // Relationship methods
    pub async fn get_project_tasks(&self, project_uuid: Uuid) -> Result<Vec<Task>>;
    pub async fn get_area_projects(&self, area_uuid: Uuid) -> Result<Vec<Project>>;
    pub async fn get_tasks_by_tag(&self, tag_title: &str) -> Result<Vec<Task>>;
    
    // Statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats>;
    pub async fn get_pool_health(&self) -> Result<PoolHealthStatus>;
}
```

**Stability**: ✅ **Frozen** - No breaking changes in 1.x

---

#### Data Models

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub uuid: Uuid,
    pub title: String,
    pub notes: Option<String>,
    pub status: TaskStatus,
    pub task_type: TaskType,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub start_date: Option<NaiveDate>,
    pub deadline: Option<NaiveDate>,
    pub stop_date: Option<NaiveDate>,
    pub completion_date: Option<NaiveDate>,
    pub project_uuid: Option<Uuid>,
    pub area_uuid: Option<Uuid>,
    pub tags: Vec<String>,
    pub checklist_items: Vec<ChecklistItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub uuid: Uuid,
    pub title: String,
    pub notes: Option<String>,
    pub status: TaskStatus,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub start_date: Option<NaiveDate>,
    pub deadline: Option<NaiveDate>,
    pub stop_date: Option<NaiveDate>,
    pub completion_date: Option<NaiveDate>,
    pub area_uuid: Option<Uuid>,
    pub tags: Vec<String>,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    pub uuid: Uuid,
    pub title: String,
    pub visible: bool,
    pub projects: Vec<Project>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub uuid: Uuid,
    pub title: String,
    pub shortcut: Option<String>,
    pub parent_uuid: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Incomplete,
    Completed,
    Canceled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    #[serde(rename = "to-do")]
    Todo,
    #[serde(rename = "project")]
    Project,
    #[serde(rename = "heading")]
    Heading,
}
```

**Stability**: ✅ **Frozen** - Fields may be added but not removed or changed in 1.x

---

#### Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThingsConfig {
    pub database_path: Option<PathBuf>,
    pub fallback_to_default: bool,
    pub pool_config: Option<DatabasePoolConfig>,
    pub cache: Option<CacheConfig>,
}

impl Default for ThingsConfig;

#[derive(Debug, Clone)]
pub struct DatabasePoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_size: usize,
    pub ttl: Duration,
}
```

**Stability**: ✅ **Stable** - New optional fields may be added

---

#### Error Handling

```rust
pub type Result<T> = std::result::Result<T, ThingsError>;

#[derive(Debug, thiserror::Error)]
pub enum ThingsError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    // ... other variants
}
```

**Stability**: ✅ **Stable** - New error variants may be added

---

#### Exports (Feature-Gated)

**Feature**: `export-csv`, `export-opml`

```rust
pub struct DataExporter { /* ... */ }

impl DataExporter {
    pub fn new(config: ExportConfig) -> Self;
    pub fn export(&self, data: &ExportData, format: ExportFormat) -> Result<String>;
}

#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    #[cfg(feature = "export-csv")]
    Csv,
    #[cfg(feature = "export-opml")]
    Opml,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportData {
    pub tasks: Vec<Task>,
    pub projects: Vec<Project>,
    pub areas: Vec<Area>,
    pub tags: Vec<Tag>,
}
```

**Stability**: ✅ **Stable** - Available when features enabled

---

#### Observability (Feature-Gated)

**Feature**: `observability`

```rust
pub struct ObservabilityManager { /* ... */ }

impl ObservabilityManager {
    pub fn new(config: ObservabilityConfig) -> Result<Self>;
    pub async fn health_check(&self) -> Result<HealthStatus>;
    pub fn get_metrics(&self) -> ThingsMetrics;
}

#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    pub metrics_enabled: bool,
    pub log_level: String,
    pub health_check_interval: Duration,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub checks: Vec<CheckResult>,
}

#[derive(Debug, Clone, Default)]
pub struct ThingsMetrics {
    pub query_count: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}
```

**Stability**: ✅ **Stable** - Available when `observability` feature enabled

---

### CLI Application (`things3-cli`)

#### Commands

All CLI commands maintain backward compatibility:

```bash
things3-cli inbox [--limit N]
things3-cli today [--limit N]
things3-cli upcoming [--limit N]
things3-cli anytime [--limit N]
things3-cli someday [--limit N]
things3-cli logbook [--limit N]
things3-cli trash [--limit N]

things3-cli projects [--area UUID] [--limit N]
things3-cli areas [--limit N]
things3-cli tags

things3-cli search <query>
things3-cli get <uuid>

things3-cli health
things3-cli mcp  # Requires mcp-server feature
```

**Stability**: ✅ **Stable** - Flags may be added, existing flags unchanged

---

## Deprecation Process

### Timeline

When an API needs to change:

1. **Announce** (version N): Deprecation announced in release notes
2. **Warn** (version N): Add `#[deprecated]` attribute with alternative
3. **Wait** (versions N+1, N+2): Feature remains with warnings
4. **Remove** (version 2.0): Remove in next major version

### Example

```rust
// Version 1.2: Original API
pub fn old_function() -> Result<String> { /* ... */ }

// Version 1.3: Deprecation announced
#[deprecated(since = "1.3.0", note = "Use new_function instead")]
pub fn old_function() -> Result<String> { /* ... */ }

pub fn new_function() -> Result<String> { /* ... */ }

// Version 1.4, 1.5: Still present with deprecation warning

// Version 2.0: Removed
// old_function is no longer available
```

---

## Non-Stable APIs

### Internal Modules

The following are **internal implementation details** and may change without notice:

- `things3_core::database::core::*` (internal helpers)
- `things3_core::database::mappers::*` (internal)
- Any items not `pub` or marked `#[doc(hidden)]`

### Test Utilities

**Feature**: `test-utils`

Test utilities are **not stable**:

```rust
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {
    // Not covered by stability guarantee
}
```

These may change between minor versions.

---

## Version Compatibility

### 1.x Series

| Version | Status | Support |
|---------|--------|---------|
| 1.0.x | Current | Full support |
| 1.1.x+ | Future | Planned |

### Previous Versions

| Version | Status | Support |
|---------|--------|---------|
| 0.2.x | Legacy | Security fixes until July 2026 |
| 0.1.x | Unsupported | Upgrade recommended |

---

## Enforcement

### Automated Checks

We enforce API stability through:

1. **CI Tests**: All 1.x releases tested against 1.0 API surface
2. **Semver Checker**: Automated semver compliance checking
3. **Documentation Review**: Manual review of all public API changes

### Reporting Issues

If you discover an unintended breaking change:

1. **File an issue**: https://github.com/GarthDB/rust-things3/issues
2. **Label**: `breaking-change`, `bug`
3. **We will**: Fix in patch release or document as intended

---

## Future Evolution (2.0.0)

Potential changes being considered for 2.0:

- More granular error types
- Builder pattern for configuration
- Trait-based database interface
- Enhanced type safety with newtypes

See [POST_1.0_ROADMAP.md](POST_1.0_ROADMAP.md) for details.

---

## Summary

**1.x API Guarantee:**
- ✅ All public types, functions, and modules listed above are stable
- ✅ No breaking changes within 1.x series
- ✅ Deprecations follow 2-version warning period
- ✅ Semantic versioning strictly followed

**Trust the API:**
- Write code against 1.0 APIs with confidence
- Upgrades within 1.x are safe
- Migration guides provided for 2.0

---

## Questions?

- **GitHub Issues**: https://github.com/GarthDB/rust-things3/issues
- **API Docs**: https://docs.rs/things3-core
- **Discussions**: https://github.com/GarthDB/rust-things3/discussions
