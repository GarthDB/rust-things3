//! Database access layer for Things 3

use crate::{error::Result, models::*};
use rusqlite::Connection;
use std::path::Path;
use uuid::Uuid;

/// Main database access struct
pub struct ThingsDatabase {
    conn: Connection,
}

impl ThingsDatabase {
    /// Create a new database connection
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        Ok(Self { conn })
    }

    /// Get the default Things 3 database path
    pub fn default_path() -> String {
        format!(
            "{}/Library/Group Containers/JLMPQHK8H4.com.culturedcode.Things3/Things Database.thingsdatabase/main.sqlite",
            std::env::var("HOME").unwrap_or_else(|_| "~".to_string())
        )
    }

    /// Create with default database path
    pub fn with_default_path() -> Result<Self> {
        Self::new(Self::default_path())
    }

    /// Get tasks from inbox
    pub async fn get_inbox(&self, limit: Option<usize>) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT uuid, title, type, status, notes, start_date, deadline, created, modified, project_uuid, area_uuid, parent_uuid 
             FROM TMTask 
             WHERE status = 'incomplete' AND project_uuid IS NULL AND area_uuid IS NULL 
             ORDER BY created DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Task {
                uuid: Uuid::parse_str(&row.get::<_, String>("uuid")?).unwrap(),
                title: row.get("title")?,
                task_type: match row.get::<_, String>("type")?.as_str() {
                    "to-do" => TaskType::Todo,
                    "project" => TaskType::Project,
                    "heading" => TaskType::Heading,
                    "area" => TaskType::Area,
                    _ => TaskType::Todo,
                },
                status: match row.get::<_, String>("status")?.as_str() {
                    "incomplete" => TaskStatus::Incomplete,
                    "completed" => TaskStatus::Completed,
                    "canceled" => TaskStatus::Canceled,
                    "trashed" => TaskStatus::Trashed,
                    _ => TaskStatus::Incomplete,
                },
                notes: row.get("notes")?,
                start_date: row.get::<_, Option<String>>("start_date")?
                    .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                deadline: row.get::<_, Option<String>>("deadline")?
                    .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                created: row.get::<_, String>("created")?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
                modified: row.get::<_, String>("modified")?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
                project_uuid: row.get::<_, Option<String>>("project_uuid")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                area_uuid: row.get::<_, Option<String>>("area_uuid")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                parent_uuid: row.get::<_, Option<String>>("parent_uuid")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                tags: vec![], // TODO: Load tags separately
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
    pub async fn get_today(&self, limit: Option<usize>) -> Result<Vec<Task>> {
        let today = chrono::Utc::now().date_naive();
        let mut stmt = self.conn.prepare(
            "SELECT uuid, title, type, status, notes, start_date, deadline, created, modified, project_uuid, area_uuid, parent_uuid 
             FROM TMTask 
             WHERE status = 'incomplete' AND start_date = ? 
             ORDER BY created DESC"
        )?;

        let rows = stmt.query_map([today.format("%Y-%m-%d").to_string()], |row| {
            Ok(Task {
                uuid: Uuid::parse_str(&row.get::<_, String>("uuid")?).unwrap(),
                title: row.get("title")?,
                task_type: match row.get::<_, String>("type")?.as_str() {
                    "to-do" => TaskType::Todo,
                    "project" => TaskType::Project,
                    "heading" => TaskType::Heading,
                    "area" => TaskType::Area,
                    _ => TaskType::Todo,
                },
                status: match row.get::<_, String>("status")?.as_str() {
                    "incomplete" => TaskStatus::Incomplete,
                    "completed" => TaskStatus::Completed,
                    "canceled" => TaskStatus::Canceled,
                    "trashed" => TaskStatus::Trashed,
                    _ => TaskStatus::Incomplete,
                },
                notes: row.get("notes")?,
                start_date: row.get::<_, Option<String>>("start_date")?
                    .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                deadline: row.get::<_, Option<String>>("deadline")?
                    .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                created: row.get::<_, String>("created")?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
                modified: row.get::<_, String>("modified")?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
                project_uuid: row.get::<_, Option<String>>("project_uuid")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                area_uuid: row.get::<_, Option<String>>("area_uuid")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                parent_uuid: row.get::<_, Option<String>>("parent_uuid")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                tags: vec![], // TODO: Load tags separately
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
    pub async fn get_projects(&self, area_uuid: Option<Uuid>) -> Result<Vec<Project>> {
        let query = if area_uuid.is_some() {
            "SELECT uuid, title, notes, start_date, deadline, created, modified, area_uuid, status 
             FROM TMTask 
             WHERE type = 'project' AND area_uuid = ? 
             ORDER BY created DESC"
        } else {
            "SELECT uuid, title, notes, start_date, deadline, created, modified, area_uuid, status 
             FROM TMTask 
             WHERE type = 'project' 
             ORDER BY created DESC"
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
    pub async fn get_areas(&self) -> Result<Vec<Area>> {
        let mut stmt = self.conn.prepare(
            "SELECT uuid, title, notes, created, modified 
             FROM TMTask 
             WHERE type = 'area' 
             ORDER BY created DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Area {
                uuid: Uuid::parse_str(&row.get::<_, String>("uuid")?).unwrap(),
                title: row.get("title")?,
                notes: row.get("notes")?,
                created: row.get::<_, String>("created")?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
                modified: row.get::<_, String>("modified")?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
                tags: vec![], // TODO: Load tags separately
                projects: vec![], // TODO: Load projects separately
            })
        })?;

        let areas: Vec<Area> = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(areas)
    }

    /// Search tasks
    pub async fn search_tasks(&self, query: &str, limit: Option<usize>) -> Result<Vec<Task>> {
        let search_pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT uuid, title, type, status, notes, start_date, deadline, created, modified, project_uuid, area_uuid, parent_uuid 
             FROM TMTask 
             WHERE (title LIKE ? OR notes LIKE ?) AND status = 'incomplete'
             ORDER BY created DESC"
        )?;

        let rows = stmt.query_map([&search_pattern, &search_pattern], |row| {
            Ok(Task {
                uuid: Uuid::parse_str(&row.get::<_, String>("uuid")?).unwrap(),
                title: row.get("title")?,
                task_type: match row.get::<_, String>("type")?.as_str() {
                    "to-do" => TaskType::Todo,
                    "project" => TaskType::Project,
                    "heading" => TaskType::Heading,
                    "area" => TaskType::Area,
                    _ => TaskType::Todo,
                },
                status: match row.get::<_, String>("status")?.as_str() {
                    "incomplete" => TaskStatus::Incomplete,
                    "completed" => TaskStatus::Completed,
                    "canceled" => TaskStatus::Canceled,
                    "trashed" => TaskStatus::Trashed,
                    _ => TaskStatus::Incomplete,
                },
                notes: row.get("notes")?,
                start_date: row.get::<_, Option<String>>("start_date")?
                    .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                deadline: row.get::<_, Option<String>>("deadline")?
                    .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                created: row.get::<_, String>("created")?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
                modified: row.get::<_, String>("modified")?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
                project_uuid: row.get::<_, Option<String>>("project_uuid")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                area_uuid: row.get::<_, Option<String>>("area_uuid")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                parent_uuid: row.get::<_, Option<String>>("parent_uuid")?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                tags: vec![], // TODO: Load tags separately
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
            uuid: Uuid::parse_str(&row.get::<_, String>("uuid")?).unwrap(),
            title: row.get("title")?,
            notes: row.get("notes")?,
            start_date: row.get::<_, Option<String>>("start_date")?
                .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            deadline: row.get::<_, Option<String>>("deadline")?
                .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            created: row.get::<_, String>("created")?
                .parse::<chrono::DateTime<chrono::Utc>>()
                .unwrap_or_else(|_| chrono::Utc::now()),
            modified: row.get::<_, String>("modified")?
                .parse::<chrono::DateTime<chrono::Utc>>()
                .unwrap_or_else(|_| chrono::Utc::now()),
            area_uuid: row.get::<_, Option<String>>("area_uuid")?
                .and_then(|s| Uuid::parse_str(&s).ok()),
            tags: vec![], // TODO: Load tags separately
            status: match row.get::<_, String>("status")?.as_str() {
                "incomplete" => TaskStatus::Incomplete,
                "completed" => TaskStatus::Completed,
                "canceled" => TaskStatus::Canceled,
                "trashed" => TaskStatus::Trashed,
                _ => TaskStatus::Incomplete,
            },
            tasks: vec![], // TODO: Load tasks separately
        })
    }
}
