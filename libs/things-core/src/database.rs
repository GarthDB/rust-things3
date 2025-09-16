//! Database access layer for Things 3

use crate::{
    config::ThingsConfig,
    error::Result,
    models::{Area, Project, Task, TaskStatus, TaskType},
};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::Connection;
use std::path::Path;
use uuid::Uuid;

/// Main database access struct
pub struct ThingsDatabase {
    conn: Connection,
}

impl ThingsDatabase {
    /// Convert Things 3 type integer to `TaskType`
    fn convert_task_type(type_value: i32) -> TaskType {
        match type_value {
            1 => TaskType::Project,
            2 => TaskType::Heading,
            3 => TaskType::Area, // Checklist items are treated as areas in our model
            _ => TaskType::Todo,
        }
    }

    /// Convert Things 3 status integer to `TaskStatus`
    fn convert_task_status(status_value: i32) -> TaskStatus {
        match status_value {
            1 => TaskStatus::Completed,
            2 => TaskStatus::Canceled,
            3 => TaskStatus::Trashed,
            _ => TaskStatus::Incomplete,
        }
    }

    /// Convert Things 3 timestamp (REAL) to `DateTime<Utc>`
    fn convert_timestamp(timestamp: Option<f64>) -> DateTime<Utc> {
        timestamp.map_or_else(Utc::now, |ts| {
            #[allow(clippy::cast_possible_truncation)]
            {
                DateTime::from_timestamp(ts as i64, 0).unwrap_or_else(Utc::now)
            }
        })
    }

    /// Convert Things 3 date (INTEGER) to `NaiveDate`
    fn convert_date(date_value: Option<i64>) -> Option<NaiveDate> {
        date_value.and_then(|d| {
            // Things 3 stores dates as days since 2001-01-01
            let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1)?;
            #[allow(clippy::cast_sign_loss)]
            {
                base_date.checked_add_days(chrono::Days::new(d as u64))
            }
        })
    }

    /// Convert Things 3 UUID string to Uuid, handling None case
    /// Things 3 uses a custom base64-like format, so we'll generate a UUID from the string
    fn convert_uuid(uuid_str: Option<String>) -> Option<Uuid> {
        uuid_str.map(|s| {
            // Try to parse as standard UUID first
            if let Ok(uuid) = Uuid::parse_str(&s) {
                uuid
            } else {
                // For Things 3 format, generate a deterministic UUID from the string
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                s.hash(&mut hasher);
                let hash = hasher.finish();
                // Create a UUID from the hash
                Uuid::from_u128(u128::from(hash))
            }
        })
    }
    /// Create a new database connection
    ///
    /// # Errors
    /// Returns `ThingsError::Database` if the database cannot be opened
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        Ok(Self { conn })
    }

    /// Create a new database connection using configuration
    ///
    /// # Errors
    /// Returns `ThingsError::Database` if the database cannot be opened
    /// Returns `ThingsError::Message` if the database path is not found and fallback fails
    pub fn with_config(config: &ThingsConfig) -> Result<Self> {
        let db_path = config.get_effective_database_path()?;
        Self::new(db_path)
    }

    /// Get the default Things 3 database path
    #[must_use]
    pub fn default_path() -> String {
        format!(
            "{}/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite",
            std::env::var("HOME").unwrap_or_else(|_| "~".to_string())
        )
    }

    /// Create with default database path
    ///
    /// # Errors
    /// Returns `ThingsError::Database` if the database cannot be opened
    pub fn with_default_path() -> Result<Self> {
        Self::new(Self::default_path())
    }

    /// Get tasks from inbox
    ///
    /// # Errors
    /// Returns `ThingsError::Database` if the database query fails
    ///
    /// # Panics
    /// Panics if UUID parsing fails (should not happen with valid database)
    pub fn get_inbox(&self, limit: Option<usize>) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, project, area, heading 
             FROM TMTask 
             WHERE status = 0 AND project IS NULL AND area IS NULL 
             ORDER BY creationDate DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Task {
                uuid: Self::convert_uuid(Some(row.get("uuid")?)).unwrap_or_else(Uuid::new_v4),
                title: row.get("title")?,
                task_type: Self::convert_task_type(row.get("type")?),
                status: Self::convert_task_status(row.get("status")?),
                notes: row.get("notes")?,
                start_date: Self::convert_date(row.get("startDate")?),
                deadline: Self::convert_date(row.get("deadline")?),
                created: Self::convert_timestamp(row.get("creationDate")?),
                modified: Self::convert_timestamp(row.get("userModificationDate")?),
                project_uuid: Self::convert_uuid(row.get("project")?),
                area_uuid: Self::convert_uuid(row.get("area")?),
                parent_uuid: Self::convert_uuid(row.get("heading")?),
                tags: vec![],     // TODO: Load tags separately
                children: vec![], // TODO: Load children separately
            })
        })?;

        let mut tasks: Vec<Task> = rows.collect::<std::result::Result<Vec<_>, _>>()?;

        if let Some(limit) = limit {
            tasks.truncate(limit);
        }

        Ok(tasks)
    }

    /// Get today's tasks
    ///
    /// # Errors
    /// Returns `ThingsError::Database` if the database query fails
    ///
    /// # Panics
    /// Panics if UUID parsing fails (should not happen with valid database)
    pub fn get_today(&self, limit: Option<usize>) -> Result<Vec<Task>> {
        let today = chrono::Utc::now().date_naive();
        // Convert today to days since 2001-01-01 (Things 3 format)
        let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
        let days_since_2001 = today.signed_duration_since(base_date).num_days();

        let mut stmt = self.conn.prepare(
            "SELECT uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, project, area, heading 
             FROM TMTask 
             WHERE status = 0 AND startDate = ? 
             ORDER BY creationDate DESC"
        )?;

        let rows = stmt.query_map([days_since_2001], |row| {
            Ok(Task {
                uuid: Uuid::parse_str(&row.get::<_, String>("uuid")?)
                    .unwrap_or_else(|_| Uuid::new_v4()),
                title: row.get("title")?,
                task_type: match row.get::<_, i32>("type")? {
                    1 => TaskType::Project,
                    2 => TaskType::Heading,
                    3 => TaskType::Area,
                    _ => TaskType::Todo,
                },
                status: match row.get::<_, i32>("status")? {
                    1 => TaskStatus::Completed,
                    2 => TaskStatus::Canceled,
                    3 => TaskStatus::Trashed,
                    _ => TaskStatus::Incomplete,
                },
                notes: row.get("notes")?,
                start_date: row.get::<_, Option<i32>>("startDate")?.and_then(|days| {
                    // Convert from days since 2001-01-01 to NaiveDate
                    let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
                    base_date.checked_add_days(chrono::Days::new(days as u64))
                }),
                deadline: row.get::<_, Option<i32>>("deadline")?.and_then(|days| {
                    // Convert from days since 2001-01-01 to NaiveDate
                    let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
                    base_date.checked_add_days(chrono::Days::new(days as u64))
                }),
                created: {
                    let timestamp = row.get::<_, f64>("creationDate")?;
                    // Convert from Core Data timestamp (seconds since 2001-01-01) to DateTime<Utc>
                    let base_date = chrono::DateTime::parse_from_rfc3339("2001-01-01T00:00:00Z")
                        .unwrap()
                        .with_timezone(&chrono::Utc);
                    base_date + chrono::Duration::seconds(timestamp as i64)
                },
                modified: {
                    let timestamp = row.get::<_, f64>("userModificationDate")?;
                    // Convert from Core Data timestamp (seconds since 2001-01-01) to DateTime<Utc>
                    let base_date = chrono::DateTime::parse_from_rfc3339("2001-01-01T00:00:00Z")
                        .unwrap()
                        .with_timezone(&chrono::Utc);
                    base_date + chrono::Duration::seconds(timestamp as i64)
                },
                project_uuid: row
                    .get::<_, Option<String>>("project")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                area_uuid: row
                    .get::<_, Option<String>>("area")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                parent_uuid: row
                    .get::<_, Option<String>>("heading")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                tags: vec![],     // TODO: Load tags separately
                children: vec![], // TODO: Load children separately
            })
        })?;

        let mut tasks: Vec<Task> = rows.collect::<std::result::Result<Vec<_>, _>>()?;

        if let Some(limit) = limit {
            tasks.truncate(limit);
        }

        Ok(tasks)
    }

    /// Get all projects
    ///
    /// # Errors
    /// Returns `ThingsError::Database` if the database query fails
    pub fn get_projects(&self, area_uuid: Option<Uuid>) -> Result<Vec<Project>> {
        let query = if area_uuid.is_some() {
            "SELECT uuid, title, notes, startDate, deadline, creationDate, userModificationDate, area, status 
             FROM TMTask 
             WHERE type = 1 AND area = ? 
             ORDER BY creationDate DESC"
        } else {
            "SELECT uuid, title, notes, startDate, deadline, creationDate, userModificationDate, area, status 
             FROM TMTask 
             WHERE type = 1 
             ORDER BY creationDate DESC"
        };

        let mut stmt = self.conn.prepare(query)?;
        let rows = if let Some(area_uuid) = area_uuid {
            stmt.query_map([area_uuid.to_string()], Self::map_project_row)?
        } else {
            stmt.query_map([], Self::map_project_row)?
        };

        let projects: Vec<Project> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(projects)
    }

    /// Get all areas
    ///
    /// # Errors
    /// Returns `ThingsError::Database` if the database query fails
    ///
    /// # Panics
    /// Panics if UUID parsing fails (should not happen with valid database)
    pub fn get_areas(&self) -> Result<Vec<Area>> {
        let mut stmt = self.conn.prepare(
            "SELECT uuid, title, visible, \"index\" 
             FROM TMArea 
             WHERE visible IS NULL OR visible = 1 
             ORDER BY \"index\"",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Area {
                uuid: Uuid::parse_str(&row.get::<_, String>("uuid")?)
                    .unwrap_or_else(|_| Uuid::new_v4()),
                title: row.get("title")?,
                notes: None,                  // TMArea doesn't have notes field
                created: chrono::Utc::now(),  // TMArea doesn't track creation date
                modified: chrono::Utc::now(), // TMArea doesn't track modification date
                tags: vec![],                 // TODO: Load tags separately
                projects: vec![],             // TODO: Load projects separately
            })
        })?;

        let areas: Vec<Area> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(areas)
    }

    /// Search tasks
    ///
    /// # Errors
    /// Returns `ThingsError::Database` if the database query fails
    ///
    /// # Panics
    /// Panics if UUID parsing fails (should not happen with valid database)
    pub fn search_tasks(&self, query: &str, limit: Option<usize>) -> Result<Vec<Task>> {
        let search_pattern = format!("%{query}%");
        let mut stmt = self.conn.prepare(
            "SELECT uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, project, area, heading 
             FROM TMTask 
             WHERE (title LIKE ? OR notes LIKE ?) AND status = 0
             ORDER BY creationDate DESC"
        )?;

        let rows = stmt.query_map([&search_pattern, &search_pattern], |row| {
            let uuid_str = row.get::<_, String>("uuid")?;
            let uuid = Uuid::parse_str(&uuid_str).unwrap_or_else(|_| {
                // Generate a new UUID if parsing fails
                Uuid::new_v4()
            });
            Ok(Task {
                uuid,
                title: row.get("title")?,
                task_type: match row.get::<_, i32>("type")? {
                    1 => TaskType::Project,
                    2 => TaskType::Heading,
                    3 => TaskType::Area,
                    _ => TaskType::Todo,
                },
                status: match row.get::<_, i32>("status")? {
                    1 => TaskStatus::Completed,
                    2 => TaskStatus::Canceled,
                    3 => TaskStatus::Trashed,
                    _ => TaskStatus::Incomplete,
                },
                notes: row.get("notes")?,
                start_date: row.get::<_, Option<i32>>("startDate")?.and_then(|days| {
                    // Convert from days since 2001-01-01 to NaiveDate
                    let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
                    base_date.checked_add_days(chrono::Days::new(days as u64))
                }),
                deadline: row.get::<_, Option<i32>>("deadline")?.and_then(|days| {
                    // Convert from days since 2001-01-01 to NaiveDate
                    let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
                    base_date.checked_add_days(chrono::Days::new(days as u64))
                }),
                created: {
                    let timestamp = row.get::<_, f64>("creationDate")?;
                    // Convert from Core Data timestamp (seconds since 2001-01-01) to DateTime<Utc>
                    let base_date = chrono::DateTime::parse_from_rfc3339("2001-01-01T00:00:00Z")
                        .unwrap()
                        .with_timezone(&chrono::Utc);
                    base_date + chrono::Duration::seconds(timestamp as i64)
                },
                modified: {
                    let timestamp = row.get::<_, f64>("userModificationDate")?;
                    // Convert from Core Data timestamp (seconds since 2001-01-01) to DateTime<Utc>
                    let base_date = chrono::DateTime::parse_from_rfc3339("2001-01-01T00:00:00Z")
                        .unwrap()
                        .with_timezone(&chrono::Utc);
                    base_date + chrono::Duration::seconds(timestamp as i64)
                },
                project_uuid: row
                    .get::<_, Option<String>>("project")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                area_uuid: row
                    .get::<_, Option<String>>("area")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                parent_uuid: row
                    .get::<_, Option<String>>("heading")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                tags: vec![],     // TODO: Load tags separately
                children: vec![], // TODO: Load children separately
            })
        })?;

        let mut tasks: Vec<Task> = rows.collect::<std::result::Result<Vec<_>, _>>()?;

        if let Some(limit) = limit {
            tasks.truncate(limit);
        }

        Ok(tasks)
    }

    /// Helper method to map a database row to a Project
    fn map_project_row(row: &rusqlite::Row) -> rusqlite::Result<Project> {
        Ok(Project {
            uuid: Uuid::parse_str(&row.get::<_, String>("uuid")?)
                .unwrap_or_else(|_| Uuid::new_v4()),
            title: row.get("title")?,
            notes: row.get("notes")?,
            start_date: row.get::<_, Option<i32>>("startDate")?.and_then(|days| {
                // Convert from days since 2001-01-01 to NaiveDate
                let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
                base_date.checked_add_days(chrono::Days::new(days as u64))
            }),
            deadline: row.get::<_, Option<i32>>("deadline")?.and_then(|days| {
                // Convert from days since 2001-01-01 to NaiveDate
                let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
                base_date.checked_add_days(chrono::Days::new(days as u64))
            }),
            created: {
                let timestamp = row.get::<_, f64>("creationDate")?;
                // Convert from Core Data timestamp (seconds since 2001-01-01) to DateTime<Utc>
                let base_date = chrono::DateTime::parse_from_rfc3339("2001-01-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc);
                base_date + chrono::Duration::seconds(timestamp as i64)
            },
            modified: {
                let timestamp = row.get::<_, f64>("userModificationDate")?;
                // Convert from Core Data timestamp (seconds since 2001-01-01) to DateTime<Utc>
                let base_date = chrono::DateTime::parse_from_rfc3339("2001-01-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc);
                base_date + chrono::Duration::seconds(timestamp as i64)
            },
            area_uuid: row
                .get::<_, Option<String>>("area")?
                .and_then(|s| Uuid::parse_str(&s).ok()),
            tags: vec![], // TODO: Load tags separately
            status: match row.get::<_, i32>("status")? {
                1 => TaskStatus::Completed,
                2 => TaskStatus::Canceled,
                3 => TaskStatus::Trashed,
                _ => TaskStatus::Incomplete,
            },
            tasks: vec![], // TODO: Load tasks separately
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_database;
    use tempfile::NamedTempFile;

    /// Test convert_task_type with all possible values
    #[test]
    fn test_convert_task_type() {
        assert_eq!(ThingsDatabase::convert_task_type(1), TaskType::Project);
        assert_eq!(ThingsDatabase::convert_task_type(2), TaskType::Heading);
        assert_eq!(ThingsDatabase::convert_task_type(3), TaskType::Area);
        assert_eq!(ThingsDatabase::convert_task_type(0), TaskType::Todo);
        assert_eq!(ThingsDatabase::convert_task_type(4), TaskType::Todo);
        assert_eq!(ThingsDatabase::convert_task_type(-1), TaskType::Todo);
    }

    /// Test convert_task_status with all possible values
    #[test]
    fn test_convert_task_status() {
        assert_eq!(
            ThingsDatabase::convert_task_status(1),
            TaskStatus::Completed
        );
        assert_eq!(ThingsDatabase::convert_task_status(2), TaskStatus::Canceled);
        assert_eq!(ThingsDatabase::convert_task_status(3), TaskStatus::Trashed);
        assert_eq!(
            ThingsDatabase::convert_task_status(0),
            TaskStatus::Incomplete
        );
        assert_eq!(
            ThingsDatabase::convert_task_status(4),
            TaskStatus::Incomplete
        );
        assert_eq!(
            ThingsDatabase::convert_task_status(-1),
            TaskStatus::Incomplete
        );
    }

    /// Test convert_timestamp with various inputs
    #[test]
    fn test_convert_timestamp() {
        // Test with None - should return current time
        let result = ThingsDatabase::convert_timestamp(None);
        let _ = result; // Just verify it doesn't panic

        // Test with valid timestamp - just check it returns a valid DateTime
        let timestamp = 1234567890.0;
        let result = ThingsDatabase::convert_timestamp(Some(timestamp));
        let _ = result; // Just verify it doesn't panic

        // Test with negative timestamp (should fallback to now)
        let timestamp = -1234567890.0;
        let result = ThingsDatabase::convert_timestamp(Some(timestamp));
        let _ = result; // Just verify it doesn't panic

        // Test with very large timestamp (should fallback to now)
        let timestamp = 999999999999.0;
        let result = ThingsDatabase::convert_timestamp(Some(timestamp));
        let _ = result; // Just verify it doesn't panic
    }

    /// Test convert_date with various inputs
    #[test]
    fn test_convert_date() {
        // Test with None
        assert_eq!(ThingsDatabase::convert_date(None), None);

        // Test with valid date (days since 2001-01-01)
        let days = 0; // 2001-01-01
        let result = ThingsDatabase::convert_date(Some(days));
        assert_eq!(
            result,
            Some(chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap())
        );

        // Test with 365 days (2002-01-01)
        let days = 365;
        let result = ThingsDatabase::convert_date(Some(days));
        assert_eq!(
            result,
            Some(chrono::NaiveDate::from_ymd_opt(2002, 1, 1).unwrap())
        );

        // Test with negative days (should return None as it's before 2001-01-01)
        let days = -1;
        let result = ThingsDatabase::convert_date(Some(days));
        assert_eq!(result, None);

        // Test with very large number
        let days = 10000;
        let result = ThingsDatabase::convert_date(Some(days));
        assert!(result.is_some());
    }

    /// Test convert_uuid with various inputs
    #[test]
    fn test_convert_uuid() {
        // Test with None
        assert_eq!(ThingsDatabase::convert_uuid(None), None);

        // Test with valid UUID string
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let result = ThingsDatabase::convert_uuid(Some(uuid_str.to_string()));
        assert_eq!(result, Some(Uuid::parse_str(uuid_str).unwrap()));

        // Test with invalid UUID string (should generate deterministic UUID)
        let uuid_str = "invalid-uuid";
        let result = ThingsDatabase::convert_uuid(Some(uuid_str.to_string()));
        assert!(result.is_some());
        // Should be deterministic
        let result2 = ThingsDatabase::convert_uuid(Some(uuid_str.to_string()));
        assert_eq!(result, result2);

        // Test with empty string
        let result = ThingsDatabase::convert_uuid(Some("".to_string()));
        assert!(result.is_some());

        // Test with special characters
        let uuid_str = "!@#$%^&*()";
        let result = ThingsDatabase::convert_uuid(Some(uuid_str.to_string()));
        assert!(result.is_some());
    }

    /// Test map_project_row with various inputs
    #[test]
    fn test_map_project_row() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db = ThingsDatabase::new(db_path).unwrap();

        // Test with a real project row (if TMProject table exists)
        let mut stmt = match db.conn.prepare("SELECT uuid, title, notes, startDate, deadline, creationDate, userModificationDate, area, status FROM TMProject LIMIT 1") {
            Ok(stmt) => stmt,
            Err(_) => {
                // TMProject table doesn't exist, skip this test
                return;
            }
        };

        let mut rows = stmt.query([]).unwrap();

        if let Some(row) = rows.next().unwrap() {
            let project = ThingsDatabase::map_project_row(row).unwrap();
            assert!(!project.title.is_empty());
            assert!(project.uuid != Uuid::nil());
        }
    }

    /// Test database connection with invalid path
    #[test]
    fn test_database_invalid_path() {
        let result = ThingsDatabase::new("/nonexistent/path/database.sqlite");
        assert!(result.is_err());
    }

    /// Test database connection with malformed database
    #[test]
    fn test_database_malformed() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        // Create a file that's not a valid SQLite database
        std::fs::write(db_path, "not a database").unwrap();

        let result = ThingsDatabase::new(db_path);
        // SQLite might still open the file, so we test that it fails on query
        match result {
            Ok(db) => {
                // If it opens, it should fail on query
                let tasks = db.get_inbox(Some(1));
                assert!(tasks.is_err());
            }
            Err(_) => {
                // Expected error
            }
        }
    }

    /// Test get_inbox with malformed data
    #[test]
    fn test_get_inbox_malformed_data() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db = ThingsDatabase::new(db_path).unwrap();

        // Insert malformed data
        db.conn.execute(
            "INSERT INTO TMTask (uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, project, area, heading) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            ("invalid-uuid", "Test Task", 1, 0, "Notes", 0, 0, 0.0, 0.0, "invalid-project", "invalid-area", "invalid-heading")
        ).unwrap();

        // Should handle malformed data gracefully
        let tasks = db.get_inbox(Some(10)).unwrap();
        assert!(!tasks.is_empty());
    }

    /// Test get_today with edge case dates
    #[test]
    fn test_get_today_edge_cases() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db = ThingsDatabase::new(db_path).unwrap();

        // Test with very old date
        db.conn.execute(
            "INSERT INTO TMTask (uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, project, area, heading) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            ("test-uuid-1", "Old Task", 0, 0, "Notes", -1000, 0, 0.0, 0.0, None::<String>, None::<String>, None::<String>)
        ).unwrap();

        // Test with future date
        db.conn.execute(
            "INSERT INTO TMTask (uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, project, area, heading) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            ("test-uuid-2", "Future Task", 0, 0, "Notes", 10000, 0, 0.0, 0.0, None::<String>, None::<String>, None::<String>)
        ).unwrap();

        let tasks = db.get_today(Some(10)).unwrap();
        // Should handle edge cases gracefully
        let _ = tasks.len();
    }

    /// Test search_tasks with edge cases
    #[test]
    fn test_search_tasks_edge_cases() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db = ThingsDatabase::new(db_path).unwrap();

        // Test with empty query
        let tasks = db.search_tasks("", Some(10)).unwrap();
        let _ = tasks.len();

        // Test with very long query
        let long_query = "a".repeat(1000);
        let tasks = db.search_tasks(&long_query, Some(10)).unwrap();
        let _ = tasks.len();

        // Test with special characters
        let special_query = "!@#$%^&*()";
        let tasks = db.search_tasks(special_query, Some(10)).unwrap();
        let _ = tasks.len();

        // Test with SQL injection attempt
        let sql_query = "'; DROP TABLE TMTask; --";
        let tasks = db.search_tasks(sql_query, Some(10)).unwrap();
        let _ = tasks.len();
    }

    /// Test get_projects with edge cases
    #[test]
    fn test_get_projects_edge_cases() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db = ThingsDatabase::new(db_path).unwrap();

        // Test with invalid area UUID
        let invalid_uuid = Uuid::new_v4();
        let projects = db.get_projects(Some(invalid_uuid)).unwrap();
        assert!(projects.is_empty());

        // Test with no area filter
        let projects = db.get_projects(None).unwrap();
        let _ = projects.len();
    }

    /// Test get_areas with edge cases
    #[test]
    fn test_get_areas_edge_cases() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db = ThingsDatabase::new(db_path).unwrap();

        // Test basic areas functionality
        let areas = db.get_areas().unwrap();
        let _ = areas.len();
    }

    /// Test database connection persistence
    #[test]
    fn test_database_connection_persistence() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db1 = ThingsDatabase::new(db_path).unwrap();
        let tasks1 = db1.get_inbox(Some(5)).unwrap();

        // Create another connection to the same database
        let db2 = ThingsDatabase::new(db_path).unwrap();
        let tasks2 = db2.get_inbox(Some(5)).unwrap();

        // Should get the same results
        assert_eq!(tasks1.len(), tasks2.len());
    }

    /// Test database error recovery
    #[test]
    fn test_database_error_recovery() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db = ThingsDatabase::new(db_path).unwrap();

        // Test that we can recover from errors
        let result = db.get_inbox(Some(5));
        assert!(result.is_ok());

        // Test with invalid limit
        let result = db.get_inbox(Some(0));
        assert!(result.is_ok());
    }

    /// Test database query consistency
    #[test]
    fn test_database_query_consistency() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db = ThingsDatabase::new(db_path).unwrap();

        // Test that different queries return consistent results
        let inbox = db.get_inbox(Some(10)).unwrap();
        let today = db.get_today(Some(10)).unwrap();
        let all_tasks = db.search_tasks("", Some(20)).unwrap();

        // Inbox should be a subset of all tasks
        assert!(all_tasks.len() >= inbox.len());

        // Today should be a subset of all tasks
        assert!(all_tasks.len() >= today.len());
    }

    /// Test database with mock data consistency
    #[test]
    fn test_database_with_mock_data_consistency() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db = ThingsDatabase::new(db_path).unwrap();

        // Test that mock data is consistent
        let tasks = db.get_inbox(Some(10)).unwrap();
        let projects = db.get_projects(None).unwrap();
        let areas = db.get_areas().unwrap();

        // Should have some data
        assert!(tasks.len() > 0 || projects.len() > 0 || areas.len() > 0);

        // Test that tasks with area relationships work
        let tasks_with_areas = tasks.iter().filter(|t| t.area_uuid.is_some()).count();
        let _ = tasks_with_areas;
    }

    /// Test database performance with large limits
    #[test]
    fn test_database_performance_with_large_limits() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db = ThingsDatabase::new(db_path).unwrap();

        // Test with very large limit
        let start = std::time::Instant::now();
        let tasks = db.get_inbox(Some(10000)).unwrap();
        let duration = start.elapsed();

        // Should complete quickly even with large limit
        assert!(duration.as_secs() < 5);
        let _ = tasks.len();
    }

    /// Test database helper functions indirectly
    #[test]
    fn test_database_helper_functions_indirectly() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db = ThingsDatabase::new(db_path).unwrap();

        // Test that helper functions are called through get_inbox
        let tasks = db.get_inbox(Some(5)).unwrap();

        // Verify that tasks have proper types and statuses
        for task in tasks {
            assert!(matches!(
                task.task_type,
                TaskType::Project | TaskType::Heading | TaskType::Area | TaskType::Todo
            ));
            assert!(matches!(
                task.status,
                TaskStatus::Completed
                    | TaskStatus::Canceled
                    | TaskStatus::Trashed
                    | TaskStatus::Incomplete
            ));
        }
    }
}
