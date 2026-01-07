# Post-1.0.0 Roadmap

**Last Updated**: January 2026  
**Status**: Planning  
**Target Audience**: Contributors, users, maintainers

---

## Overview

This document outlines the planned evolution of `rust-things3` after the 1.0.0 stable release. The roadmap is organized into three horizons:

1. **1.x Series** (2026): Minor releases with backward-compatible enhancements
2. **2.0 Release** (2027): Major release with breaking changes  
3. **Long-term Vision** (2027+): Future direction and possibilities

---

## Release Philosophy

### Semantic Versioning Commitment

We strictly follow [Semantic Versioning 2.0.0](https://semver.org/):

- **1.x.y** (Patch): Bug fixes only, no new features
- **1.x.0** (Minor): New features, backward compatible
- **2.0.0** (Major): Breaking changes, API evolution

### Stability Guarantees

**1.x series**:
- ✅ No breaking changes to public APIs
- ✅ Deprecations announced 2 minor versions ahead
- ✅ Security updates backported
- ✅ Bug fixes for critical issues

**2.0 and beyond**:
- Migration guides provided
- Deprecation period honored
- Upgrade path documented

---

## 1.x Series Roadmap

### 1.1.0 (Q1 2026) - Enhanced Exports

**Theme**: Additional export formats and options

#### Planned Features
- [ ] **Markdown Export Enhancements**
  - Customizable templates (Obsidian, Notion, etc.)
  - Frontmatter support (YAML, TOML)
  - Wiki-link generation
  - Tag and link resolution

- [ ] **New Export Formats**
  - iCalendar (`.ics`) for calendar integration
  - TaskPaper format
  - HTML export with templates
  - Org-mode format

- [ ] **Export Options**
  - Filter by date range
  - Include/exclude completed tasks
  - Custom field selection
  - Batch export of multiple formats

#### Implementation Status
- **Status**: Planning
- **Est. Effort**: Medium (2-3 weeks)
- **Breaking**: No
- **Feature Flag**: `export-markdown-enhanced`, `export-ical`, etc.

---

### 1.2.0 (Q2 2026) - Query Enhancements

**Theme**: More powerful querying and filtering

#### Planned Features
- [ ] **Advanced Filters**
  - Complex boolean expressions (AND, OR, NOT)
  - Date range queries (last week, next month, etc.)
  - Tag combinations and exclusions
  - Custom field queries

- [ ] **Query Builder API**
  - Fluent interface for building queries
  - Type-safe query construction
  - Query optimization hints
  - Explain query plans

- [ ] **Saved Queries**
  - Store and reuse common queries
  - Query templates
  - Parameterized queries
  - Query performance tracking

- [ ] **Full-Text Search Improvements**
  - Fuzzy matching
  - Stemming support
  - Relevance scoring
  - Search result highlighting

#### Implementation Status
- **Status**: Exploration
- **Est. Effort**: Large (4-6 weeks)
- **Breaking**: No
- **Feature Flag**: `advanced-queries`

---

### 1.3.0 (Q3 2026) - Performance & Scale

**Theme**: Handling larger databases efficiently

#### Planned Features
- [ ] **Pagination Support**
  - Cursor-based pagination
  - Offset/limit pagination
  - Streaming results
  - Lazy loading

- [ ] **Batch Operations**
  - Bulk updates with transactions
  - Batch exports
  - Parallel query execution
  - Progress reporting

- [ ] **Caching Improvements**
  - Smarter cache invalidation
  - Predictive preloading
  - Cache warming strategies
  - Memory-mapped cache option

- [ ] **Database Optimization**
  - Automatic VACUUM scheduling
  - Index optimization suggestions
  - Query plan analysis
  - Performance profiling tools

#### Implementation Status
- **Status**: Exploration
- **Est. Effort**: Large (5-7 weeks)
- **Breaking**: No
- **Feature Flag**: Various

---

### 1.4.0 (Q4 2026) - Integration & Ecosystem

**Theme**: Better integration with other tools

#### Planned Features
- [ ] **Webhook Support**
  - Task change notifications
  - Custom webhook handlers
  - Retry logic
  - Webhook verification

- [ ] **External Sync**
  - Export to external systems (Todoist, Trello, etc.)
  - Two-way sync (cautious approach)
  - Conflict resolution
  - Sync status tracking

- [ ] **Plugin System**
  - Loadable plugins (via dynamic linking or WASM)
  - Plugin API stabilization
  - Plugin marketplace (future)
  - Example plugins

- [ ] **More Integration Examples**
  - GitHub Actions integration
  - CI/CD pipelines
  - Notification systems (Slack, Discord, etc.)
  - Data visualization tools

#### Implementation Status
- **Status**: Research
- **Est. Effort**: X-Large (8-10 weeks)
- **Breaking**: No
- **Feature Flag**: `webhooks`, `plugins`, etc.

---

## 2.0.0 Roadmap (2027)

### Vision

Version 2.0 will be a major evolution, incorporating lessons learned from 1.x usage and community feedback. It will include breaking changes to improve ergonomics, performance, and type safety.

### Tentative Breaking Changes

#### API Evolution

**1. More Granular Error Types**
```rust
// Current (1.x)
pub type Result<T> = std::result::Result<T, ThingsError>;

// Proposed (2.0)
pub enum ThingsDatabaseError { /* ... */ }
pub enum ThingsExportError { /* ... */ }
pub enum ThingsQueryError { /* ... */ }
```

**Benefit**: Better error handling, more specific error context

---

**2. Builder Pattern for Configuration**
```rust
// Current (1.x)
let config = ThingsConfig {
    database_path: Some(path),
    fallback_to_default: true,
    ..Default::default()
};

// Proposed (2.0)
let config = ThingsConfig::builder()
    .database_path(path)
    .fallback_to_default(true)
    .cache(CacheConfig::default())
    .build()?;
```

**Benefit**: More discoverable, type-safe, validated configuration

---

**3. Async Traits**
```rust
// Current (1.x) - Concrete type
pub struct ThingsDatabase { /* ... */ }

// Proposed (2.0) - Trait-based
#[async_trait]
pub trait ThingsDatabase {
    async fn get_task(&self, uuid: Uuid) -> Result<Option<Task>>;
    // ...
}

pub struct SqliteThingsDatabase { /* ... */ }
```

**Benefit**: Testability, mock implementations, alternative backends

---

**4. Improved Type Safety**
```rust
// Current (1.x)
pub fn get_task(&self, uuid: &str) -> Result<Option<Task>>;

// Proposed (2.0)
pub fn get_task(&self, id: TaskId) -> Result<Task>;

pub struct TaskId(Uuid);
pub struct ProjectId(Uuid);
```

**Benefit**: Compile-time prevention of mixing IDs, clearer intent

---

#### New Features (2.0)

- [ ] **Write Support** (Experimental, opt-in)
  - Create tasks (with safety guarantees)
  - Update task properties
  - Complete tasks
  - Move tasks between projects/areas
  - **Note**: Read-only remains default, write requires explicit opt-in

- [ ] **Alternative Backends**
  - PostgreSQL support (for server deployments)
  - MySQL support
  - In-memory backend (for testing)
  - Cloud storage backends

- [ ] **GraphQL API** (Optional)
  - Query language for complex data needs
  - Subscription support
  - Schema introspection
  - Playground UI

- [ ] **Enhanced Type System**
  - Phantom types for compile-time guarantees
  - Builder pattern validation
  - State machines for task lifecycles

#### Migration Path

- **Timeline**: 6-month deprecation period in 1.x
- **Tools**: Automated migration tool (`things3-migrate`)
- **Documentation**: Comprehensive 2.0 migration guide
- **Support**: 1.x LTS maintained for 12 months after 2.0 release

---

## Long-term Vision (2027+)

### Possible Features

These are ideas for exploration, not commitments:

#### 3.0+ Possibilities

- **Multi-database Support**: Query across multiple Things databases
- **Time Series Analysis**: Track productivity trends, task velocity
- **AI Integration**: Smart task categorization, priority suggestions
- **Collaboration Features**: Shared tasks, team views (if Things adds support)
- **Mobile Bindings**: Kotlin/Swift wrappers for mobile development
- **Desktop UI**: Tauri-based desktop application
- **Cloud Sync**: Optional cloud backup and sync

#### Ecosystem Growth

- **Community Plugins**: Marketplace for community-contributed plugins
- **Integration Gallery**: Showcase of third-party integrations
- **Educational Content**: Tutorials, courses, workshops
- **Enterprise Support**: Commercial support options for businesses

---

## How to Contribute

### Providing Feedback

We welcome feedback on this roadmap!

- **Feature Requests**: [GitHub Issues](https://github.com/GarthDB/rust-things3/issues)
- **Discussions**: [GitHub Discussions](https://github.com/GarthDB/rust-things3/discussions)
- **Pull Requests**: Implementation proposals welcome

### Prioritization

Features are prioritized based on:

1. **Community Demand**: Highly requested features rise in priority
2. **Impact**: Features benefiting many users prioritized
3. **Complexity**: Quick wins shipped sooner
4. **Breaking Changes**: Batched into major releases

### Helping Out

Want to contribute? Check out:

- **[CONTRIBUTING.md](../CONTRIBUTING.md)**: Contribution guidelines
- **[Good First Issues](https://github.com/GarthDB/rust-things3/labels/good%20first%20issue)**: Beginner-friendly tasks
- **[Help Wanted](https://github.com/GarthDB/rust-things3/labels/help%20wanted)**: Issues needing help

---

## Release Cadence

### Planned Schedule

- **Patch releases** (1.x.y): As needed (bug fixes, security)
- **Minor releases** (1.x.0): Quarterly (Q1, Q2, Q3, Q4)
- **Major releases** (2.0.0): Annually or when significant breaking changes accumulate

### Support Policy

- **Current major version** (1.x): Full support, bug fixes, new features
- **Previous major version** (0.x): Security fixes only, 6 months after 1.0.0
- **Older versions**: No support (upgrade recommended)

---

## Deprecation Policy

### How We Deprecate

1. **Announce**: Deprecation announced in release notes
2. **Warn**: Compiler warnings added (`#[deprecated]`)
3. **Document**: Alternative provided in documentation
4. **Wait**: 2 minor versions (6 months) before removal
5. **Remove**: Removed in next major version

### Example Timeline

```
1.2.0: Feature X announced as deprecated
1.3.0: Feature X still present (warnings)
1.4.0: Feature X still present (warnings)
2.0.0: Feature X removed
```

---

## Communication

### Stay Informed

- **GitHub Releases**: https://github.com/GarthDB/rust-things3/releases
- **CHANGELOG.md**: Detailed change log
- **Blog**: (Future) Development blog
- **Social Media**: (Future) Twitter/Mastodon updates

### Quarterly Updates

We'll provide quarterly roadmap updates:

- **Q1**: January roadmap review
- **Q2**: April roadmap review
- **Q3**: July roadmap review
- **Q4**: October roadmap review

---

## Questions?

- **GitHub Issues**: https://github.com/GarthDB/rust-things3/issues
- **GitHub Discussions**: https://github.com/GarthDB/rust-things3/discussions
- **Email**: (If you set up a project email)

---

## Summary

**Near-term** (1.x):
- Enhanced exports and query capabilities
- Performance improvements
- Better ecosystem integration
- All backward compatible

**Mid-term** (2.0):
- API evolution with breaking changes
- Write support (opt-in)
- Alternative backends
- Enhanced type safety

**Long-term** (3.0+):
- Advanced features (AI, collaboration, etc.)
- Ecosystem maturity
- Enterprise features

**We're excited for the future of `rust-things3`!** 🚀

