//! Discover the active Things 3 database path on disk.
//!
//! Things 3 stores its SQLite file under `Library/Group Containers/.../ThingsData-XXXXX/...`,
//! where the 4-character suffix varies per install (App Store vs. direct purchase,
//! possibly tied to iCloud account). This module scans the group container at
//! runtime and picks the most-recently-modified candidate, falling back to the
//! historical literal `ThingsData-0Z0Z2` path when nothing is found.

use std::path::PathBuf;

/// Things 3 group container directory under the user's `Library`.
const THINGS_GROUP_CONTAINER: &str =
    "Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac";

/// Path inside a `ThingsData-XXXXX` directory that points at the SQLite file.
const THINGS_DB_RELATIVE: &str = "Things Database.thingsdatabase/main.sqlite";

/// Get the default Things 3 database path.
///
/// The 4-character suffix on `ThingsData-XXXXX` varies per install (App Store
/// vs. direct purchase, possibly tied to iCloud account), so this function
/// scans the group container for any `ThingsData-*` directory containing a
/// real database file. If multiple candidates exist, the one whose
/// `main.sqlite` was modified most recently wins. When no candidate is found
/// the function falls back to the historical literal `ThingsData-0Z0Z2` path
/// — callers downstream surface a clean "file not found" error in that case.
///
/// # Examples
///
/// ```
/// use things3_core::get_default_database_path;
///
/// let path = get_default_database_path();
/// assert!(!path.to_string_lossy().is_empty());
/// assert!(path.to_string_lossy().contains("Library"));
/// ```
#[must_use]
pub fn get_default_database_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
    let group_container = PathBuf::from(&home).join(THINGS_GROUP_CONTAINER);

    if let Some(found) = discover_things_database(&group_container) {
        return found;
    }

    group_container
        .join("ThingsData-0Z0Z2")
        .join(THINGS_DB_RELATIVE)
}

/// Scan `group_container` for `ThingsData-*/Things Database.thingsdatabase/main.sqlite`
/// and return the most-recently-modified candidate, if any.
fn discover_things_database(group_container: &std::path::Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(group_container).ok()?;

    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if !name_str.starts_with("ThingsData-") {
            continue;
        }

        let candidate = entry.path().join(THINGS_DB_RELATIVE);
        let Ok(meta) = std::fs::metadata(&candidate) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);

        match &best {
            Some((_, best_mtime)) if mtime <= *best_mtime => {}
            _ => best = Some((candidate, mtime)),
        }
    }

    best.map(|(path, _)| path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_default_database_path_format() {
        let path = get_default_database_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("Things Database.thingsdatabase"));
        assert!(path_str.contains("main.sqlite"));
        assert!(path_str.contains("Library/Group Containers"));
    }

    #[test]
    fn test_discover_things_database_picks_non_default_suffix() {
        let group_container = TempDir::new().unwrap();
        let things_dir = group_container.path().join("ThingsData-01AEF");
        let db_dir = things_dir.join("Things Database.thingsdatabase");
        std::fs::create_dir_all(&db_dir).unwrap();
        let db_path = db_dir.join("main.sqlite");
        std::fs::write(&db_path, b"").unwrap();

        let found = discover_things_database(group_container.path()).unwrap();
        assert_eq!(found, db_path);
    }

    #[test]
    fn test_discover_things_database_prefers_most_recent() {
        let group_container = TempDir::new().unwrap();

        let make = |suffix: &str| {
            let dir = group_container
                .path()
                .join(format!("ThingsData-{suffix}"))
                .join("Things Database.thingsdatabase");
            std::fs::create_dir_all(&dir).unwrap();
            let db = dir.join("main.sqlite");
            std::fs::write(&db, b"").unwrap();
            db
        };

        let _older = make("OLDER");
        // 10ms is well above any reasonable filesystem mtime resolution, so
        // the second file is guaranteed to have a strictly later mtime.
        std::thread::sleep(std::time::Duration::from_millis(10));
        let newer = make("NEWER");

        let found = discover_things_database(group_container.path()).unwrap();
        assert_eq!(found, newer);
    }

    #[test]
    fn test_discover_things_database_returns_none_when_empty() {
        let group_container = TempDir::new().unwrap();
        assert!(discover_things_database(group_container.path()).is_none());
    }

    #[test]
    fn test_discover_things_database_skips_non_matching_dirs() {
        let group_container = TempDir::new().unwrap();
        std::fs::create_dir_all(group_container.path().join("SomethingElse")).unwrap();
        std::fs::create_dir_all(
            group_container.path().join("ThingsData-EMPTY"), // no main.sqlite inside
        )
        .unwrap();
        assert!(discover_things_database(group_container.path()).is_none());
    }
}
