use crate::{
    database::{conversions::safe_timestamp_convert, mappers::map_project_row, ThingsDatabase},
    error::{Result as ThingsResult, ThingsError},
    models::{Project, TaskStatus, ThingsId},
};
use chrono::{DateTime, Utc};
use sqlx::Row;
use tracing::{debug, instrument};

impl ThingsDatabase {
    /// Get all projects (from `TMTask` table where type = 1)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if project data is invalid
    #[instrument]
    pub async fn get_all_projects(&self) -> ThingsResult<Vec<Project>> {
        let rows = sqlx::query(
            r"
            SELECT 
                uuid, title, status, 
                area, notes, 
                creationDate, userModificationDate,
                startDate, deadline
            FROM TMTask
            WHERE type = 1 AND trashed = 0
            ORDER BY creationDate DESC
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch projects: {e}")))?;

        let mut projects = Vec::new();
        for row in rows {
            let project = Project {
                uuid: ThingsId::from_trusted(row.get::<String, _>("uuid")),
                title: row.get("title"),
                status: TaskStatus::from_i32(row.get("status")).unwrap_or(TaskStatus::Incomplete),
                area_uuid: row
                    .get::<Option<String>, _>("area")
                    .map(ThingsId::from_trusted),
                notes: row.get("notes"),
                deadline: row
                    .get::<Option<i64>, _>("deadline")
                    .and_then(|ts| DateTime::from_timestamp(ts, 0))
                    .map(|dt| dt.date_naive()),
                start_date: row
                    .get::<Option<i64>, _>("startDate")
                    .and_then(|ts| DateTime::from_timestamp(ts, 0))
                    .map(|dt| dt.date_naive()),
                tags: Vec::new(),  // TODO: Load tags separately
                tasks: Vec::new(), // TODO: Load child tasks separately
                created: {
                    let ts_f64 = row.get::<f64, _>("creationDate");
                    let ts = safe_timestamp_convert(ts_f64);
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
                },
                modified: {
                    let ts_f64 = row.get::<f64, _>("userModificationDate");
                    let ts = safe_timestamp_convert(ts_f64);
                    DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
                },
            };
            projects.push(project);
        }

        debug!("Fetched {} projects", projects.len());
        Ok(projects)
    }

    /// Get all projects (alias for `get_all_projects` for compatibility)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if project data is invalid
    #[instrument(skip(self))]
    pub async fn get_projects(&self, limit: Option<usize>) -> ThingsResult<Vec<Project>> {
        let _ = limit; // Currently unused but kept for API compatibility
        self.get_all_projects().await
    }

    /// Get a single project by UUID
    ///
    /// Returns `None` if the project doesn't exist or is trashed
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn get_project_by_uuid(&self, id: &ThingsId) -> ThingsResult<Option<Project>> {
        let row = sqlx::query(
            r"
            SELECT
                uuid, title, status,
                area, notes,
                creationDate, userModificationDate,
                startDate, deadline,
                trashed, type
            FROM TMTask
            WHERE uuid = ? AND type = 1
            ",
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch project: {e}")))?;

        if let Some(row) = row {
            let trashed: i64 = row.get("trashed");
            if trashed == 1 {
                return Ok(None);
            }
            Ok(Some(map_project_row(&row)))
        } else {
            Ok(None)
        }
    }
}
