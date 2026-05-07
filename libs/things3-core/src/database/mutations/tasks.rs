use crate::{
    database::{
        conversions::naive_date_to_things_timestamp, query_builders::TaskUpdateBuilder, validators,
        ThingsDatabase,
    },
    error::{Result as ThingsResult, ThingsError},
    models::{
        CreateTaskRequest, DeleteChildHandling, TaskStatus, TaskType, ThingsId, UpdateTaskRequest,
    },
};
use chrono::Utc;
use sqlx::Row;
use tracing::{info, instrument};

impl ThingsDatabase {
    /// Create a new task in the database
    ///
    /// Validates:
    /// - Project UUID exists if provided
    /// - Area UUID exists if provided
    /// - Parent task UUID exists if provided
    /// - Date range (deadline >= start_date)
    ///
    /// Returns the UUID of the created task
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use things3_core::{ThingsDatabase, CreateTaskRequest, ThingsError};
    /// use std::path::Path;
    /// use chrono::NaiveDate;
    ///
    /// # async fn example() -> Result<(), ThingsError> {
    /// let db = ThingsDatabase::new(Path::new("/path/to/things.db")).await?;
    ///
    /// // Create a simple task
    /// let request = CreateTaskRequest {
    ///     title: "Buy groceries".to_string(),
    ///     notes: Some("Milk, eggs, bread".to_string()),
    ///     deadline: Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
    ///     start_date: None,
    ///     project_uuid: None,
    ///     area_uuid: None,
    ///     parent_uuid: None,
    ///     tags: None,
    ///     task_type: None,
    ///     status: None,
    /// };
    ///
    /// let task_uuid = db.create_task(request).await?;
    /// println!("Created task with UUID: {}", task_uuid);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails or if the database insert fails
    #[instrument(skip(self))]
    pub async fn create_task(&self, request: CreateTaskRequest) -> ThingsResult<ThingsId> {
        // Validate date range (deadline must be >= start_date)
        crate::database::validate_date_range(request.start_date, request.deadline)?;

        // Generate ID for new task
        let id = ThingsId::new_things_native();

        // Validate referenced entities
        if let Some(project_uuid) = &request.project_uuid {
            validators::validate_project_exists(&self.pool, project_uuid).await?;
        }

        if let Some(area_uuid) = &request.area_uuid {
            validators::validate_area_exists(&self.pool, area_uuid).await?;
        }

        if let Some(parent_uuid) = &request.parent_uuid {
            validators::validate_task_exists(&self.pool, parent_uuid).await?;
        }

        // Convert dates to Things 3 format (seconds since 2001-01-01)
        let start_date_ts = request.start_date.map(naive_date_to_things_timestamp);
        let deadline_ts = request.deadline.map(naive_date_to_things_timestamp);

        // Get current timestamp for creation/modification dates
        let now = Utc::now().timestamp() as f64;

        // Insert into TMTask table
        sqlx::query(
            r"
            INSERT INTO TMTask (
                uuid, title, type, status, notes,
                startDate, deadline, project, area, heading,
                creationDate, userModificationDate,
                trashed
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(id.as_str())
        .bind(&request.title)
        .bind(request.task_type.unwrap_or(TaskType::Todo) as i32)
        .bind(request.status.unwrap_or(TaskStatus::Incomplete) as i32)
        .bind(request.notes.as_ref())
        .bind(start_date_ts)
        .bind(deadline_ts)
        .bind(request.project_uuid.map(|u| u.into_string()))
        .bind(request.area_uuid.map(|u| u.into_string()))
        .bind(request.parent_uuid.map(|u| u.into_string()))
        .bind(now)
        .bind(now)
        .bind(0) // not trashed
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to create task: {e}")))?;

        // Handle tags via TMTaskTag
        if let Some(tags) = request.tags {
            self.set_task_tags(&id, tags).await?;
        }

        info!("Created task with UUID: {}", id);
        Ok(id)
    }

    /// Update an existing task
    ///
    /// Only updates fields that are provided (Some(_))
    /// Validates existence of referenced entities
    ///
    /// # Errors
    ///
    /// Returns an error if the task doesn't exist, validation fails, or the database update fails
    #[instrument(skip(self))]
    pub async fn update_task(&self, request: UpdateTaskRequest) -> ThingsResult<()> {
        // Verify task exists
        validators::validate_task_exists(&self.pool, &request.uuid).await?;

        // Validate dates if either is being updated
        if request.start_date.is_some() || request.deadline.is_some() {
            // Get current task to merge dates
            if let Some(current_task) = self.get_task_by_uuid(&request.uuid).await? {
                let final_start = request.start_date.or(current_task.start_date);
                let final_deadline = request.deadline.or(current_task.deadline);
                crate::database::validate_date_range(final_start, final_deadline)?;
            }
        }

        // Validate referenced entities if being updated
        if let Some(project_uuid) = &request.project_uuid {
            validators::validate_project_exists(&self.pool, project_uuid).await?;
        }

        if let Some(area_uuid) = &request.area_uuid {
            validators::validate_area_exists(&self.pool, area_uuid).await?;
        }

        // Use the TaskUpdateBuilder to construct the query
        let builder = TaskUpdateBuilder::from_request(&request);

        // If no fields to update, just return (modification date will still be updated)
        if builder.is_empty() {
            return Ok(());
        }

        let query_string = builder.build_query_string();
        let mut q = sqlx::query(&query_string);

        // Bind values in the same order as the builder added fields
        if let Some(title) = &request.title {
            q = q.bind(title);
        }

        if let Some(notes) = &request.notes {
            q = q.bind(notes);
        }

        if let Some(start_date) = request.start_date {
            q = q.bind(naive_date_to_things_timestamp(start_date));
        }

        if let Some(deadline) = request.deadline {
            q = q.bind(naive_date_to_things_timestamp(deadline));
        }

        if let Some(status) = request.status {
            q = q.bind(status as i32);
        }

        if let Some(project_uuid) = request.project_uuid {
            q = q.bind(project_uuid.into_string());
        }

        if let Some(area_uuid) = request.area_uuid {
            q = q.bind(area_uuid.into_string());
        }

        // Bind modification date and UUID (always added by builder)
        let now = Utc::now().timestamp() as f64;
        q = q.bind(now).bind(request.uuid.as_str());

        q.execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to update task: {e}")))?;

        // Handle tags via TMTaskTag (separate from the UPDATE query)
        if let Some(tags) = request.tags {
            self.set_task_tags(&request.uuid, tags).await?;
        }

        info!("Updated task with UUID: {}", request.uuid);
        Ok(())
    }

    /// Mark a task as completed
    ///
    /// # Errors
    ///
    /// Returns an error if the task does not exist or if the database update fails
    #[instrument(skip(self))]
    pub async fn complete_task(&self, id: &ThingsId) -> ThingsResult<()> {
        // Verify task exists
        validators::validate_task_exists(&self.pool, id).await?;

        let now = Utc::now().timestamp() as f64;

        sqlx::query(
            "UPDATE TMTask SET status = 3, stopDate = ?, userModificationDate = ? WHERE uuid = ?",
        )
        .bind(now)
        .bind(now)
        .bind(id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to complete task: {e}")))?;

        info!("Completed task with UUID: {}", id);
        Ok(())
    }

    /// Mark a completed task as incomplete
    ///
    /// # Errors
    ///
    /// Returns an error if the task does not exist or if the database update fails
    #[instrument(skip(self))]
    pub async fn uncomplete_task(&self, id: &ThingsId) -> ThingsResult<()> {
        // Verify task exists
        validators::validate_task_exists(&self.pool, id).await?;

        let now = Utc::now().timestamp() as f64;

        sqlx::query(
            "UPDATE TMTask SET status = 0, stopDate = NULL, userModificationDate = ? WHERE uuid = ?",
        )
        .bind(now)
        .bind(id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to uncomplete task: {e}")))?;

        info!("Uncompleted task with UUID: {}", id);
        Ok(())
    }

    /// Soft delete a task (set trashed flag)
    ///
    /// # Errors
    ///
    /// Returns an error if the task does not exist, if child handling fails, or if the database update fails
    #[instrument(skip(self))]
    pub async fn delete_task(
        &self,
        id: &ThingsId,
        child_handling: DeleteChildHandling,
    ) -> ThingsResult<()> {
        // Verify task exists
        validators::validate_task_exists(&self.pool, id).await?;

        // Check for child tasks
        let children = sqlx::query("SELECT uuid FROM TMTask WHERE heading = ? AND trashed = 0")
            .bind(id.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to query child tasks: {e}")))?;

        let has_children = !children.is_empty();

        if has_children {
            match child_handling {
                DeleteChildHandling::Error => {
                    return Err(ThingsError::unknown(format!(
                        "Task {} has {} child task(s). Use cascade or orphan mode to delete.",
                        id,
                        children.len()
                    )));
                }
                DeleteChildHandling::Cascade => {
                    // Delete all children
                    let now = Utc::now().timestamp() as f64;
                    for child_row in &children {
                        let child_uuid: String = child_row.get("uuid");
                        sqlx::query(
                            "UPDATE TMTask SET trashed = 1, userModificationDate = ? WHERE uuid = ?",
                        )
                        .bind(now)
                        .bind(&child_uuid)
                        .execute(&self.pool)
                        .await
                        .map_err(|e| {
                            ThingsError::unknown(format!("Failed to delete child task: {e}"))
                        })?;
                    }
                    info!("Cascade deleted {} child task(s)", children.len());
                }
                DeleteChildHandling::Orphan => {
                    // Clear parent reference for children
                    let now = Utc::now().timestamp() as f64;
                    for child_row in &children {
                        let child_uuid: String = child_row.get("uuid");
                        sqlx::query(
                            "UPDATE TMTask SET heading = NULL, userModificationDate = ? WHERE uuid = ?",
                        )
                        .bind(now)
                        .bind(&child_uuid)
                        .execute(&self.pool)
                        .await
                        .map_err(|e| {
                            ThingsError::unknown(format!("Failed to orphan child task: {e}"))
                        })?;
                    }
                    info!("Orphaned {} child task(s)", children.len());
                }
            }
        }

        // Delete the parent task
        let now = Utc::now().timestamp() as f64;
        sqlx::query("UPDATE TMTask SET trashed = 1, userModificationDate = ? WHERE uuid = ?")
            .bind(now)
            .bind(id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to delete task: {e}")))?;

        info!("Deleted task with UUID: {}", id);
        Ok(())
    }
}
