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
        let mut stmt = self.conn.prepare(
            "SELECT uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, project, area, heading 
             FROM TMTask 
             WHERE status = 0 AND startDate = ? 
             ORDER BY creationDate DESC"
        )?;

        let rows = stmt.query_map([today.format("%Y-%m-%d").to_string()], |row| {
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
