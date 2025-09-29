use crate::{
    error::{ThingsError, Result as ThingsResult},
    models::{Area, Project, Task, TaskStatus, TaskType},
};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{SqlitePool, Row};
use std::path::Path;
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

impl TaskStatus {
    fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(TaskStatus::Incomplete),
            1 => Some(TaskStatus::Completed),
            2 => Some(TaskStatus::Canceled),
            3 => Some(TaskStatus::Trashed),
            _ => None,
        }
    }
}

impl TaskType {
    fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(TaskType::Todo),
            1 => Some(TaskType::Project),
            2 => Some(TaskType::Heading),
            3 => Some(TaskType::Area),
            _ => None,
        }
    }
}

/// SQLx-based database implementation for Things 3 data
/// This provides async, Send + Sync compatible database access
#[derive(Debug, Clone)]
pub struct SqlxThingsDatabase {
    pool: SqlitePool,
}

impl SqlxThingsDatabase {
    /// Create a new database connection pool
    #[instrument]
    pub async fn new(database_path: &Path) -> ThingsResult<Self> {
        let database_url = format!("sqlite:{}", database_path.display());
        
        info!("Connecting to SQLite database at: {}", database_url);
        
        let pool = SqlitePool::connect(&database_url)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to connect to database: {}", e)))?;

        // Configure SQLite for better performance
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to set WAL mode: {}", e)))?;

        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to set synchronous mode: {}", e)))?;

        sqlx::query("PRAGMA cache_size = -20000")
            .execute(&pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to set cache size: {}", e)))?;

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to enable foreign keys: {}", e)))?;

        info!("Database connection established successfully");
        
        Ok(Self { pool })
    }

    /// Create a new database connection pool from a connection string
    #[instrument]
    pub async fn from_connection_string(database_url: &str) -> ThingsResult<Self> {
        info!("Connecting to SQLite database: {}", database_url);
        
        let pool = SqlitePool::connect(database_url)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to connect to database: {}", e)))?;

        // Configure SQLite for better performance
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to set WAL mode: {}", e)))?;

        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to set synchronous mode: {}", e)))?;

        sqlx::query("PRAGMA cache_size = -20000")
            .execute(&pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to set cache size: {}", e)))?;

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to enable foreign keys: {}", e)))?;

        info!("Database connection established successfully");
        
        Ok(Self { pool })
    }

    /// Get the underlying connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Check if the database is connected
    #[instrument]
    pub async fn is_connected(&self) -> bool {
        match sqlx::query("SELECT 1").fetch_one(&self.pool).await {
            Ok(_) => {
                debug!("Database connection is healthy");
                true
            }
            Err(e) => {
                error!("Database connection check failed: {}", e);
                false
            }
        }
    }

    /// Get database statistics
    #[instrument]
    pub async fn get_stats(&self) -> ThingsResult<DatabaseStats> {
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM TMTask")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to get task count: {}", e)))?;

        let project_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM TMProject")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to get project count: {}", e)))?;

        let area_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM TMArea")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to get area count: {}", e)))?;

        Ok(DatabaseStats {
            task_count: task_count as u64,
            project_count: project_count as u64,
            area_count: area_count as u64,
        })
    }

    /// Get all tasks
    #[instrument]
    pub async fn get_all_tasks(&self) -> ThingsResult<Vec<Task>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                uuid, title, status, type, 
                start_date, due_date, 
                project_uuid, area_uuid, 
                notes, tags, 
                created, modified
            FROM TMTask
            ORDER BY created DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch tasks: {}", e)))?;

        let mut tasks = Vec::new();
        for row in rows {
            let task = Task {
                uuid: Uuid::parse_str(&row.get::<String, _>("uuid"))
                    .map_err(|e| ThingsError::unknown(format!("Invalid task UUID: {}", e)))?,
                title: row.get("title"),
                status: TaskStatus::from_i32(row.get("status"))
                    .unwrap_or(TaskStatus::Incomplete),
                task_type: TaskType::from_i32(row.get("type"))
                    .unwrap_or(TaskType::Todo),
                start_date: row.get::<Option<String>, _>("start_date")
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                deadline: row.get::<Option<String>, _>("due_date")
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                project_uuid: row.get::<Option<String>, _>("project_uuid")
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                area_uuid: row.get::<Option<String>, _>("area_uuid")
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                parent_uuid: None, // Not available in this query
                notes: row.get("notes"),
                tags: row.get::<Option<String>, _>("tags")
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
                children: Vec::new(), // Not available in this query
                created: DateTime::parse_from_rfc3339(&row.get::<String, _>("created"))
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now),
                modified: DateTime::parse_from_rfc3339(&row.get::<String, _>("modified"))
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now),
            };
            tasks.push(task);
        }

        debug!("Fetched {} tasks", tasks.len());
        Ok(tasks)
    }

    /// Get all projects
    #[instrument]
    pub async fn get_all_projects(&self) -> ThingsResult<Vec<Project>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                uuid, title, status, 
                area_uuid, notes, 
                created, modified
            FROM TMProject
            ORDER BY created DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch projects: {}", e)))?;

        let mut projects = Vec::new();
        for row in rows {
            let project = Project {
                uuid: Uuid::parse_str(&row.get::<String, _>("uuid"))
                    .map_err(|e| ThingsError::unknown(format!("Invalid project UUID: {}", e)))?,
                title: row.get("title"),
                status: TaskStatus::from_i32(row.get("status"))
                    .unwrap_or(TaskStatus::Incomplete),
                area_uuid: row.get::<Option<String>, _>("area_uuid")
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                notes: row.get("notes"),
                deadline: None, // Not available in this query
                start_date: None, // Not available in this query
                tags: Vec::new(), // Not available in this query
                tasks: Vec::new(), // Not available in this query
                created: DateTime::parse_from_rfc3339(&row.get::<String, _>("created"))
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now),
                modified: DateTime::parse_from_rfc3339(&row.get::<String, _>("modified"))
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now),
            };
            projects.push(project);
        }

        debug!("Fetched {} projects", projects.len());
        Ok(projects)
    }

    /// Get all areas
    #[instrument]
    pub async fn get_all_areas(&self) -> ThingsResult<Vec<Area>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                uuid, title, 
                notes, 
                created, modified
            FROM TMArea
            ORDER BY created DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch areas: {}", e)))?;

        let mut areas = Vec::new();
        for row in rows {
            let area = Area {
                uuid: Uuid::parse_str(&row.get::<String, _>("uuid"))
                    .map_err(|e| ThingsError::unknown(format!("Invalid area UUID: {}", e)))?,
                title: row.get("title"),
                notes: row.get("notes"),
                projects: Vec::new(), // Not available in this query
                tags: Vec::new(), // Not available in this query
                created: DateTime::parse_from_rfc3339(&row.get::<String, _>("created"))
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now),
                modified: DateTime::parse_from_rfc3339(&row.get::<String, _>("modified"))
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now),
            };
            areas.push(area);
        }

        debug!("Fetched {} areas", areas.len());
        Ok(areas)
    }

    /// Get tasks by status
    #[instrument]
    pub async fn get_tasks_by_status(&self, status: TaskStatus) -> ThingsResult<Vec<Task>> {
        let status_value = status as i32;
        let rows = sqlx::query(
            r#"
            SELECT 
                uuid, title, status, type, 
                start_date, due_date, 
                project_uuid, area_uuid, 
                notes, tags, 
                created, modified
            FROM TMTask
            WHERE status = ?
            ORDER BY created DESC
            "#
        )
        .bind(status_value)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch tasks by status: {}", e)))?;

        let mut tasks = Vec::new();
        for row in rows {
            let task = Task {
                uuid: Uuid::parse_str(&row.get::<String, _>("uuid"))
                    .map_err(|e| ThingsError::unknown(format!("Invalid task UUID: {}", e)))?,
                title: row.get("title"),
                status: TaskStatus::from_i32(row.get("status"))
                    .unwrap_or(TaskStatus::Incomplete),
                task_type: TaskType::from_i32(row.get("type"))
                    .unwrap_or(TaskType::Todo),
                start_date: row.get::<Option<String>, _>("start_date")
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                deadline: row.get::<Option<String>, _>("due_date")
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                project_uuid: row.get::<Option<String>, _>("project_uuid")
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                area_uuid: row.get::<Option<String>, _>("area_uuid")
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                parent_uuid: None, // Not available in this query
                notes: row.get("notes"),
                tags: row.get::<Option<String>, _>("tags")
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
                children: Vec::new(), // Not available in this query
                created: DateTime::parse_from_rfc3339(&row.get::<String, _>("created"))
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now),
                modified: DateTime::parse_from_rfc3339(&row.get::<String, _>("modified"))
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now),
            };
            tasks.push(task);
        }

        debug!("Fetched {} tasks with status {:?}", tasks.len(), status);
        Ok(tasks)
    }

    /// Search tasks by title or notes
    #[instrument]
    pub async fn search_tasks(&self, query: &str) -> ThingsResult<Vec<Task>> {
        let search_pattern = format!("%{}%", query);
        let rows = sqlx::query(
            r#"
            SELECT 
                uuid, title, status, type, 
                start_date, due_date, 
                project_uuid, area_uuid, 
                notes, tags, 
                created, modified
            FROM TMTask
            WHERE title LIKE ? OR notes LIKE ?
            ORDER BY created DESC
            "#
        )
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to search tasks: {}", e)))?;

        let mut tasks = Vec::new();
        for row in rows {
            let task = Task {
                uuid: Uuid::parse_str(&row.get::<String, _>("uuid"))
                    .map_err(|e| ThingsError::unknown(format!("Invalid task UUID: {}", e)))?,
                title: row.get("title"),
                status: TaskStatus::from_i32(row.get("status"))
                    .unwrap_or(TaskStatus::Incomplete),
                task_type: TaskType::from_i32(row.get("type"))
                    .unwrap_or(TaskType::Todo),
                start_date: row.get::<Option<String>, _>("start_date")
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                deadline: row.get::<Option<String>, _>("due_date")
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                project_uuid: row.get::<Option<String>, _>("project_uuid")
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                area_uuid: row.get::<Option<String>, _>("area_uuid")
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                parent_uuid: None, // Not available in this query
                notes: row.get("notes"),
                tags: row.get::<Option<String>, _>("tags")
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
                children: Vec::new(), // Not available in this query
                created: DateTime::parse_from_rfc3339(&row.get::<String, _>("created"))
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now),
                modified: DateTime::parse_from_rfc3339(&row.get::<String, _>("modified"))
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now),
            };
            tasks.push(task);
        }

        debug!("Found {} tasks matching query: {}", tasks.len(), query);
        Ok(tasks)
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub task_count: u64,
    pub project_count: u64,
    pub area_count: u64,
}

impl DatabaseStats {
    pub fn total_items(&self) -> u64 {
        self.task_count + self.project_count + self.area_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_database_connection() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        // This will fail because the database doesn't exist yet
        // In a real implementation, we'd need to create the schema first
        let result = SqlxThingsDatabase::new(&db_path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connection_string() {
        let result = SqlxThingsDatabase::from_connection_string("sqlite::memory:").await;
        assert!(result.is_ok());
    }
}