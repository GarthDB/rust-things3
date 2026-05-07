use crate::{
    database::{
        conversions::naive_date_to_things_timestamp, query_builders::TaskUpdateBuilder, validators,
        ThingsDatabase,
    },
    error::{Result as ThingsResult, ThingsError},
    models::ThingsId,
};
use chrono::Utc;
use tracing::{info, instrument};

impl ThingsDatabase {
    /// Create a new project
    ///
    /// Projects are tasks with type = 1 in the TMTask table
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails or the database insert fails
    #[instrument(skip(self))]
    pub async fn create_project(
        &self,
        request: crate::models::CreateProjectRequest,
    ) -> ThingsResult<ThingsId> {
        // Validate date range (deadline must be >= start_date)
        crate::database::validate_date_range(request.start_date, request.deadline)?;

        // Generate ID for new project
        let id = ThingsId::new_things_native();

        // Validate area if provided
        if let Some(area_uuid) = &request.area_uuid {
            validators::validate_area_exists(&self.pool, area_uuid).await?;
        }

        // Convert dates to Things 3 format (seconds since 2001-01-01)
        let start_date_ts = request.start_date.map(naive_date_to_things_timestamp);
        let deadline_ts = request.deadline.map(naive_date_to_things_timestamp);

        // Get current timestamp for creation/modification dates
        let now = Utc::now().timestamp() as f64;

        // Insert into TMTask table with type = 1 (project)
        sqlx::query(
            r"
            INSERT INTO TMTask (
                uuid, title, type, status, notes,
                startDate, deadline, project, area, heading,
                creationDate, userModificationDate,
                trashed
            ) VALUES (?, ?, 1, 0, ?, ?, ?, NULL, ?, NULL, ?, ?, 0)
            ",
        )
        .bind(id.as_str())
        .bind(&request.title)
        .bind(request.notes.as_ref())
        .bind(start_date_ts)
        .bind(deadline_ts)
        .bind(request.area_uuid.map(|u| u.into_string()))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to create project: {e}")))?;

        // Handle tags via TMTaskTag
        if let Some(tags) = request.tags {
            self.set_task_tags(&id, tags).await?;
        }

        info!("Created project with UUID: {}", id);
        Ok(id)
    }

    /// Update an existing project
    ///
    /// Only updates fields that are provided (Some(_))
    /// Validates existence and that the entity is a project (type = 1)
    ///
    /// # Errors
    ///
    /// Returns an error if the project doesn't exist, validation fails, or the database update fails
    #[instrument(skip(self))]
    pub async fn update_project(
        &self,
        request: crate::models::UpdateProjectRequest,
    ) -> ThingsResult<()> {
        // Verify project exists (type = 1, trashed = 0)
        validators::validate_project_exists(&self.pool, &request.uuid).await?;

        // Validate dates if either is being updated
        if request.start_date.is_some() || request.deadline.is_some() {
            // Fetch current project to merge dates
            if let Some(current_project) = self.get_project_by_uuid(&request.uuid).await? {
                let final_start = request.start_date.or(current_project.start_date);
                let final_deadline = request.deadline.or(current_project.deadline);
                crate::database::validate_date_range(final_start, final_deadline)?;
            }
        }

        // Validate area if being updated
        if let Some(area_uuid) = &request.area_uuid {
            validators::validate_area_exists(&self.pool, area_uuid).await?;
        }

        // Build dynamic query using TaskUpdateBuilder
        let mut builder = TaskUpdateBuilder::new();

        // Add fields to update
        if request.title.is_some() {
            builder = builder.add_field("title");
        }
        if request.notes.is_some() {
            builder = builder.add_field("notes");
        }
        if request.start_date.is_some() {
            builder = builder.add_field("startDate");
        }
        if request.deadline.is_some() {
            builder = builder.add_field("deadline");
        }
        if request.area_uuid.is_some() {
            builder = builder.add_field("area");
        }

        // If nothing to update (tags-only changes still need to proceed for TMTaskTag)
        let has_db_fields = !builder.is_empty();

        if has_db_fields {
            // Build query string
            let query_str = builder.build_query_string();
            let mut q = sqlx::query(&query_str);

            // Bind values in the same order they were added to the builder
            if let Some(ref title) = request.title {
                q = q.bind(title);
            }
            if let Some(ref notes) = request.notes {
                q = q.bind(notes);
            }
            if let Some(start_date) = request.start_date {
                q = q.bind(naive_date_to_things_timestamp(start_date));
            }
            if let Some(deadline) = request.deadline {
                q = q.bind(naive_date_to_things_timestamp(deadline));
            }
            if let Some(area_uuid) = request.area_uuid {
                q = q.bind(area_uuid.into_string());
            }

            // Bind modification date and UUID (always added by builder)
            let now = Utc::now().timestamp() as f64;
            q = q.bind(now).bind(request.uuid.as_str());

            q.execute(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to update project: {e}")))?;
        } else if request.tags.is_none() {
            // Nothing to update at all
            return Ok(());
        }

        // Handle tags via TMTaskTag (separate from the UPDATE query)
        if let Some(tags) = request.tags {
            self.set_task_tags(&request.uuid, tags).await?;
        }

        info!("Updated project with UUID: {}", request.uuid);
        Ok(())
    }

    /// Complete a project and optionally handle its child tasks
    ///
    /// # Errors
    ///
    /// Returns an error if the project doesn't exist or if the database update fails
    #[instrument(skip(self))]
    pub async fn complete_project(
        &self,
        id: &ThingsId,
        child_handling: crate::models::ProjectChildHandling,
    ) -> ThingsResult<()> {
        // Verify project exists
        validators::validate_project_exists(&self.pool, id).await?;

        let now = Utc::now().timestamp() as f64;

        // Handle child tasks based on the handling mode
        match child_handling {
            crate::models::ProjectChildHandling::Error => {
                // Check if project has children
                let child_count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM TMTask WHERE project = ? AND trashed = 0",
                )
                .bind(id.as_str())
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    ThingsError::unknown(format!("Failed to check for child tasks: {e}"))
                })?;

                if child_count > 0 {
                    return Err(ThingsError::unknown(format!(
                        "Project {} has {} child task(s). Use cascade or orphan mode to complete.",
                        id, child_count
                    )));
                }
            }
            crate::models::ProjectChildHandling::Cascade => {
                // Complete all child tasks
                sqlx::query(
                    "UPDATE TMTask SET status = 3, stopDate = ?, userModificationDate = ? WHERE project = ? AND trashed = 0",
                )
                .bind(now)
                .bind(now)
                .bind(id.as_str())
                .execute(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to complete child tasks: {e}")))?;
            }
            crate::models::ProjectChildHandling::Orphan => {
                // Move child tasks to inbox (set project to NULL)
                sqlx::query(
                    "UPDATE TMTask SET project = NULL, userModificationDate = ? WHERE project = ? AND trashed = 0",
                )
                .bind(now)
                .bind(id.as_str())
                .execute(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to orphan child tasks: {e}")))?;
            }
        }

        // Complete the project
        sqlx::query(
            "UPDATE TMTask SET status = 3, stopDate = ?, userModificationDate = ? WHERE uuid = ?",
        )
        .bind(now)
        .bind(now)
        .bind(id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to complete project: {e}")))?;

        info!("Completed project with UUID: {}", id);
        Ok(())
    }

    /// Soft delete a project and handle its child tasks
    ///
    /// # Errors
    ///
    /// Returns an error if the project doesn't exist, if child handling fails, or if the database update fails
    #[instrument(skip(self))]
    pub async fn delete_project(
        &self,
        id: &ThingsId,
        child_handling: crate::models::ProjectChildHandling,
    ) -> ThingsResult<()> {
        // Verify project exists
        validators::validate_project_exists(&self.pool, id).await?;

        let now = Utc::now().timestamp() as f64;

        // Handle child tasks based on the handling mode
        match child_handling {
            crate::models::ProjectChildHandling::Error => {
                // Check if project has children
                let child_count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM TMTask WHERE project = ? AND trashed = 0",
                )
                .bind(id.as_str())
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    ThingsError::unknown(format!("Failed to check for child tasks: {e}"))
                })?;

                if child_count > 0 {
                    return Err(ThingsError::unknown(format!(
                        "Project {} has {} child task(s). Use cascade or orphan mode to delete.",
                        id, child_count
                    )));
                }
            }
            crate::models::ProjectChildHandling::Cascade => {
                // Delete all child tasks
                sqlx::query(
                    "UPDATE TMTask SET trashed = 1, userModificationDate = ? WHERE project = ? AND trashed = 0",
                )
                .bind(now)
                .bind(id.as_str())
                .execute(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to delete child tasks: {e}")))?;
            }
            crate::models::ProjectChildHandling::Orphan => {
                // Move child tasks to inbox (set project to NULL)
                sqlx::query(
                    "UPDATE TMTask SET project = NULL, userModificationDate = ? WHERE project = ? AND trashed = 0",
                )
                .bind(now)
                .bind(id.as_str())
                .execute(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to orphan child tasks: {e}")))?;
            }
        }

        // Delete the project
        sqlx::query("UPDATE TMTask SET trashed = 1, userModificationDate = ? WHERE uuid = ?")
            .bind(now)
            .bind(id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to delete project: {e}")))?;

        info!("Deleted project with UUID: {}", id);
        Ok(())
    }
}
