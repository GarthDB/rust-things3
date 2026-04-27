//! Saved query storage and replay.
//!
//! [`SavedQuery`] captures the full state of a [`crate::query::TaskQueryBuilder`] —
//! both [`TaskFilters`] and the post-1.0.0 builder-only predicates (`any_tags`,
//! `exclude_tags`, `tag_count_min`, `fuzzy_query`, `fuzzy_threshold`) — so a
//! query can be persisted to disk by name and replayed later.
//!
//! [`SavedQueryStore`] is a file-backed `HashMap<String, SavedQuery>` with
//! atomic writes (write-to-temp + rename) and a permissive load that returns
//! an empty store when the file doesn't exist yet.
//!
//! Requires the `advanced-queries` feature flag.

#![cfg(feature = "advanced-queries")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Result, ThingsError};
use crate::models::TaskFilters;

/// A saved task query.
///
/// Wraps [`TaskFilters`] plus the builder-only predicates introduced after
/// 1.0.0. Construct via [`crate::query::TaskQueryBuilder::to_saved_query`]
/// and replay via [`crate::query::TaskQueryBuilder::from_saved_query`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedQuery {
    /// Display name. Acts as the primary key in [`SavedQueryStore`].
    pub name: String,

    /// Optional human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// SQL-level filters (the stable 1.0.0 surface).
    #[serde(default)]
    pub filters: TaskFilters,

    /// OR-semantics tag filter (post-filter applied in Rust).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub any_tags: Option<Vec<String>>,

    /// Tag exclusion filter (post-filter applied in Rust).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude_tags: Option<Vec<String>>,

    /// Minimum tag-count threshold (post-filter applied in Rust).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_count_min: Option<usize>,

    /// Fuzzy search query string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuzzy_query: Option<String>,

    /// Fuzzy match score threshold (clamped to `[0.0, 1.0]`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuzzy_threshold: Option<f32>,

    /// When the query was created.
    pub created: DateTime<Utc>,
}

impl SavedQuery {
    /// Build a minimal `SavedQuery` from just a name. All filters default
    /// to empty; `created` is set to `Utc::now()`. Useful for tests and as
    /// a starting point in CLI prompts.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            filters: TaskFilters::default(),
            any_tags: None,
            exclude_tags: None,
            tag_count_min: None,
            fuzzy_query: None,
            fuzzy_threshold: None,
            created: Utc::now(),
        }
    }
}

/// File-backed store for saved queries, keyed by name.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SavedQueryStore {
    queries: HashMap<String, SavedQuery>,
}

impl SavedQueryStore {
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            queries: HashMap::new(),
        }
    }

    /// Default storage path: `~/.config/things3/saved-queries.json` on
    /// Unix-likes, `%USERPROFILE%\AppData\Roaming\things3\saved-queries.json`
    /// on Windows. Falls back to `./saved-queries.json` if neither env var
    /// is set.
    ///
    /// Mirrors `ConfigLoader::get_user_config_dir` but uses `things3/` rather
    /// than `things3-mcp/` since saved queries are a core-library feature,
    /// not server-specific.
    #[must_use]
    pub fn default_path() -> PathBuf {
        let dir = if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config").join("things3")
        } else if let Ok(userprofile) = std::env::var("USERPROFILE") {
            PathBuf::from(userprofile)
                .join("AppData")
                .join("Roaming")
                .join("things3")
        } else {
            PathBuf::from(".")
        };
        dir.join("saved-queries.json")
    }

    /// Load a store from disk. **Returns an empty store if the file does not
    /// exist** (first-run UX). Returns an error if the file exists but cannot
    /// be parsed.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or contains
    /// invalid JSON.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let content = std::fs::read_to_string(path).map_err(|e| {
            ThingsError::Io(std::io::Error::other(format!(
                "Failed to read saved queries from {}: {}",
                path.display(),
                e
            )))
        })?;
        serde_json::from_str(&content).map_err(|e| {
            ThingsError::configuration(format!(
                "Failed to parse saved queries at {}: {e}",
                path.display()
            ))
        })
    }

    /// Save the store to disk atomically (write to temp file, then rename).
    /// Creates the parent directory if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the parent directory cannot be created, the temp
    /// file cannot be written, or the rename fails.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ThingsError::Io(std::io::Error::other(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    )))
                })?;
            }
        }

        let content = serde_json::to_string_pretty(self).map_err(|e| {
            ThingsError::configuration(format!("Failed to serialize saved queries: {e}"))
        })?;

        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, content).map_err(|e| {
            ThingsError::Io(std::io::Error::other(format!(
                "Failed to write temp file {}: {}",
                tmp.display(),
                e
            )))
        })?;

        std::fs::rename(&tmp, path).map_err(|e| {
            ThingsError::Io(std::io::Error::other(format!(
                "Failed to rename {} to {}: {}",
                tmp.display(),
                path.display(),
                e
            )))
        })?;

        Ok(())
    }

    /// Insert a query, replacing any existing entry with the same name.
    pub fn insert(&mut self, query: SavedQuery) {
        self.queries.insert(query.name.clone(), query);
    }

    /// Look up a query by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&SavedQuery> {
        self.queries.get(name)
    }

    /// Remove and return a query by name.
    pub fn remove(&mut self, name: &str) -> Option<SavedQuery> {
        self.queries.remove(name)
    }

    /// Iterate over all saved queries (order is unspecified).
    pub fn list(&self) -> impl Iterator<Item = &SavedQuery> {
        self.queries.values()
    }

    /// Number of saved queries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.queries.len()
    }

    /// Whether the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{TaskStatus, TaskType};
    use chrono::NaiveDate;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn fully_populated_query(name: &str) -> SavedQuery {
        SavedQuery {
            name: name.to_string(),
            description: Some("populated for tests".to_string()),
            filters: TaskFilters {
                status: Some(TaskStatus::Incomplete),
                task_type: Some(TaskType::Todo),
                project_uuid: Some(Uuid::nil()),
                area_uuid: None,
                tags: Some(vec!["work".to_string()]),
                start_date_from: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
                start_date_to: Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
                deadline_from: None,
                deadline_to: Some(NaiveDate::from_ymd_opt(2026, 6, 30).unwrap()),
                search_query: Some("budget".to_string()),
                limit: Some(20),
                offset: Some(5),
            },
            any_tags: Some(vec!["urgent".to_string(), "important".to_string()]),
            exclude_tags: Some(vec!["archived".to_string()]),
            tag_count_min: Some(2),
            fuzzy_query: Some("agenda".to_string()),
            fuzzy_threshold: Some(0.7),
            created: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_saved_query_full_roundtrip() {
        let original = fully_populated_query("everything");
        let json = serde_json::to_string(&original).unwrap();
        let parsed: SavedQuery = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, original.name);
        assert_eq!(parsed.description, original.description);
        assert_eq!(parsed.filters.status, original.filters.status);
        assert_eq!(parsed.filters.tags, original.filters.tags);
        assert_eq!(parsed.filters.search_query, original.filters.search_query);
        assert_eq!(parsed.filters.limit, original.filters.limit);
        assert_eq!(parsed.any_tags, original.any_tags);
        assert_eq!(parsed.exclude_tags, original.exclude_tags);
        assert_eq!(parsed.tag_count_min, original.tag_count_min);
        assert_eq!(parsed.fuzzy_query, original.fuzzy_query);
        assert_eq!(parsed.fuzzy_threshold, original.fuzzy_threshold);
    }

    #[test]
    fn test_saved_query_minimal_serialize_omits_empty_options() {
        let q = SavedQuery::new("minimal");
        let json = serde_json::to_string(&q).unwrap();

        // skip_serializing_if = Option::is_none should drop these from output
        assert!(!json.contains("\"description\""));
        assert!(!json.contains("\"any_tags\""));
        assert!(!json.contains("\"exclude_tags\""));
        assert!(!json.contains("\"tag_count_min\""));
        assert!(!json.contains("\"fuzzy_query\""));
        assert!(!json.contains("\"fuzzy_threshold\""));

        // Required fields stay
        assert!(json.contains("\"name\":\"minimal\""));
        assert!(json.contains("\"filters\""));
        assert!(json.contains("\"created\""));
    }

    #[test]
    fn test_saved_query_deserializes_with_missing_optional_fields() {
        // Forward-compat: only name + filters + created should be enough
        let json = r#"{
            "name": "old-format",
            "filters": {},
            "created": "2026-01-01T00:00:00Z"
        }"#;
        let q: SavedQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.name, "old-format");
        assert!(q.description.is_none());
        assert!(q.any_tags.is_none());
        assert!(q.fuzzy_query.is_none());
    }

    #[test]
    fn test_store_insert_get_remove() {
        let mut store = SavedQueryStore::new();
        assert!(store.is_empty());

        store.insert(SavedQuery::new("a"));
        store.insert(SavedQuery::new("b"));
        assert_eq!(store.len(), 2);
        assert!(store.get("a").is_some());
        assert!(store.get("missing").is_none());

        let removed = store.remove("a").unwrap();
        assert_eq!(removed.name, "a");
        assert_eq!(store.len(), 1);
        assert!(store.get("a").is_none());
    }

    #[test]
    fn test_store_insert_replaces_same_name() {
        let mut store = SavedQueryStore::new();
        let mut q1 = SavedQuery::new("dup");
        q1.description = Some("first".to_string());
        store.insert(q1);

        let mut q2 = SavedQuery::new("dup");
        q2.description = Some("second".to_string());
        store.insert(q2);

        assert_eq!(store.len(), 1);
        assert_eq!(
            store.get("dup").unwrap().description.as_deref(),
            Some("second")
        );
    }

    #[test]
    fn test_store_list_iteration() {
        let mut store = SavedQueryStore::new();
        store.insert(SavedQuery::new("a"));
        store.insert(SavedQuery::new("b"));
        store.insert(SavedQuery::new("c"));

        let mut names: Vec<&str> = store.list().map(|q| q.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_store_load_missing_file_returns_empty() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("does-not-exist.json");
        let store = SavedQueryStore::load(&path).unwrap();
        assert!(store.is_empty());
    }

    #[test]
    fn test_store_save_then_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("queries.json");

        let mut original = SavedQueryStore::new();
        original.insert(fully_populated_query("everything"));
        original.insert(SavedQuery::new("simple"));
        original.save(&path).unwrap();

        let loaded = SavedQueryStore::load(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert!(loaded.get("everything").is_some());
        assert!(loaded.get("simple").is_some());

        let everything = loaded.get("everything").unwrap();
        assert_eq!(everything.fuzzy_threshold, Some(0.7));
        assert_eq!(everything.tag_count_min, Some(2));
    }

    #[test]
    fn test_store_save_replaces_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("queries.json");

        let mut store = SavedQueryStore::new();
        store.insert(SavedQuery::new("first"));
        store.save(&path).unwrap();

        store.remove("first");
        store.insert(SavedQuery::new("second"));
        store.save(&path).unwrap();

        let loaded = SavedQueryStore::load(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(loaded.get("first").is_none());
        assert!(loaded.get("second").is_some());
    }

    #[test]
    fn test_store_save_creates_parent_dir() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("nested").join("more").join("queries.json");

        let mut store = SavedQueryStore::new();
        store.insert(SavedQuery::new("a"));
        store.save(&nested).unwrap();

        assert!(nested.exists());
        let loaded = SavedQueryStore::load(&nested).unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_store_load_invalid_json_errors() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, "{ not valid json").unwrap();

        let result = SavedQueryStore::load(&path);
        assert!(result.is_err(), "expected parse error");
    }

    #[test]
    fn test_default_path_ends_with_saved_queries_json() {
        let path = SavedQueryStore::default_path();
        assert_eq!(
            path.file_name().and_then(|s| s.to_str()),
            Some("saved-queries.json")
        );
        assert!(
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                == Some("things3"),
            "expected parent dir to be 'things3', got {path:?}"
        );
    }
}
