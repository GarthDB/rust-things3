use crate::{
    database::{conversions::naive_date_to_things_timestamp, validators, ThingsDatabase},
    error::{Result as ThingsResult, ThingsError},
};
use chrono::Utc;
use sqlx::Row;
use tracing::{info, instrument};

impl ThingsDatabase {
    /// Maximum number of tasks that can be processed in a single bulk operation
    /// This prevents abuse and ensures reasonable transaction sizes
    const MAX_BULK_BATCH_SIZE: usize = 1000;

    /// Move multiple tasks to a project or area (transactional)
    ///
    /// All tasks must exist and be valid, or the entire operation will be rolled back.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Task UUIDs array is empty
    /// - Neither project_uuid nor area_uuid is specified
    /// - Target project or area doesn't exist
    /// - Any task UUID is invalid or doesn't exist
    /// - Database operation fails
    #[instrument(skip(self))]
    pub async fn bulk_move(
        &self,
        request: crate::models::BulkMoveRequest,
    ) -> ThingsResult<crate::models::BulkOperationResult> {
        // Validation
        if request.task_uuids.is_empty() {
            return Err(ThingsError::validation("Task UUIDs cannot be empty"));
        }
        if request.task_uuids.len() > Self::MAX_BULK_BATCH_SIZE {
            return Err(ThingsError::validation(format!(
                "Batch size {} exceeds maximum of {}",
                request.task_uuids.len(),
                Self::MAX_BULK_BATCH_SIZE
            )));
        }
        if request.project_uuid.is_none() && request.area_uuid.is_none() {
            return Err(ThingsError::validation(
                "Must specify either project_uuid or area_uuid",
            ));
        }

        // Validate target project/area exists
        if let Some(project_uuid) = &request.project_uuid {
            validators::validate_project_exists(&self.pool, project_uuid).await?;
        }
        if let Some(area_uuid) = &request.area_uuid {
            validators::validate_area_exists(&self.pool, area_uuid).await?;
        }

        // Begin transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to begin transaction: {e}")))?;

        // Validate all tasks exist in a single batch query (prevent N+1)
        let placeholders = request
            .task_uuids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query_str = format!(
            "SELECT uuid FROM TMTask WHERE uuid IN ({}) AND trashed = 0",
            placeholders
        );

        let mut query = sqlx::query(&query_str);
        for id in &request.task_uuids {
            query = query.bind(id.as_str());
        }

        let found_uuids: Vec<String> = query
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to validate tasks: {e}")))?
            .iter()
            .map(|row| row.get("uuid"))
            .collect();

        // Check if any UUIDs were not found
        if found_uuids.len() != request.task_uuids.len() {
            // Find the first missing UUID for error reporting
            for id in &request.task_uuids {
                if !found_uuids.contains(&id.to_string()) {
                    tx.rollback().await.ok();
                    return Err(ThingsError::TaskNotFound {
                        uuid: id.to_string(),
                    });
                }
            }
        }

        // Batch update
        let now = Utc::now().timestamp() as f64;
        let placeholders = request
            .task_uuids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query_str = format!(
            "UPDATE TMTask SET project = ?, area = ?, userModificationDate = ? WHERE uuid IN ({})",
            placeholders
        );

        let mut query = sqlx::query(&query_str)
            .bind(request.project_uuid.map(|u| u.into_string()))
            .bind(request.area_uuid.map(|u| u.into_string()))
            .bind(now);

        for id in &request.task_uuids {
            query = query.bind(id.as_str());
        }

        query
            .execute(&mut *tx)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to bulk move tasks: {e}")))?;

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to commit transaction: {e}")))?;

        info!("Bulk moved {} task(s)", request.task_uuids.len());
        Ok(crate::models::BulkOperationResult {
            success: true,
            processed_count: request.task_uuids.len(),
            message: format!("Successfully moved {} task(s)", request.task_uuids.len()),
        })
    }

    /// Update dates for multiple tasks with validation (transactional)
    ///
    /// All tasks must exist and dates must be valid, or the entire operation will be rolled back.
    /// Validates that deadline >= start_date for each task after merging with existing dates.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Task UUIDs array is empty
    /// - Any task UUID is invalid or doesn't exist
    /// - Date range validation fails (deadline before start_date)
    /// - Database operation fails
    #[instrument(skip(self))]
    pub async fn bulk_update_dates(
        &self,
        request: crate::models::BulkUpdateDatesRequest,
    ) -> ThingsResult<crate::models::BulkOperationResult> {
        use crate::database::{safe_things_date_to_naive_date, validate_date_range};

        // Validation
        if request.task_uuids.is_empty() {
            return Err(ThingsError::validation("Task UUIDs cannot be empty"));
        }
        if request.task_uuids.len() > Self::MAX_BULK_BATCH_SIZE {
            return Err(ThingsError::validation(format!(
                "Batch size {} exceeds maximum of {}",
                request.task_uuids.len(),
                Self::MAX_BULK_BATCH_SIZE
            )));
        }

        // Validate date range if both are provided
        if let (Some(start), Some(deadline)) = (request.start_date, request.deadline) {
            validate_date_range(Some(start), Some(deadline))?;
        }

        // Begin transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to begin transaction: {e}")))?;

        // Validate all tasks exist and check merged date validity in a single batch query
        let placeholders = request
            .task_uuids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query_str = format!(
            "SELECT uuid, startDate, deadline FROM TMTask WHERE uuid IN ({}) AND trashed = 0",
            placeholders
        );

        let mut query = sqlx::query(&query_str);
        for id in &request.task_uuids {
            query = query.bind(id.as_str());
        }

        let rows = query
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to validate tasks: {e}")))?;

        // Check if all UUIDs were found
        if rows.len() != request.task_uuids.len() {
            // Find the first missing UUID for error reporting
            let found_uuids: Vec<String> = rows.iter().map(|row| row.get("uuid")).collect();
            for id in &request.task_uuids {
                if !found_uuids.contains(&id.to_string()) {
                    tx.rollback().await.ok();
                    return Err(ThingsError::TaskNotFound {
                        uuid: id.to_string(),
                    });
                }
            }
        }

        // Validate merged dates for all tasks
        for row in &rows {
            let current_start: Option<i64> = row.get("startDate");
            let current_deadline: Option<i64> = row.get("deadline");

            let final_start = if request.clear_start_date {
                None
            } else if let Some(new_start) = request.start_date {
                Some(new_start)
            } else {
                current_start.and_then(|ts| safe_things_date_to_naive_date(ts).ok())
            };

            let final_deadline = if request.clear_deadline {
                None
            } else if let Some(new_deadline) = request.deadline {
                Some(new_deadline)
            } else {
                current_deadline.and_then(|ts| safe_things_date_to_naive_date(ts).ok())
            };

            validate_date_range(final_start, final_deadline)?;
        }

        // Build and execute batch update
        let now = Utc::now().timestamp() as f64;
        let placeholders = request
            .task_uuids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let start_date_value = if request.clear_start_date {
            None
        } else {
            request.start_date.map(naive_date_to_things_timestamp)
        };

        let deadline_value = if request.clear_deadline {
            None
        } else {
            request.deadline.map(naive_date_to_things_timestamp)
        };

        let query_str = format!(
            "UPDATE TMTask SET startDate = ?, deadline = ?, userModificationDate = ? WHERE uuid IN ({})",
            placeholders
        );

        let mut query = sqlx::query(&query_str)
            .bind(start_date_value)
            .bind(deadline_value)
            .bind(now);

        for id in &request.task_uuids {
            query = query.bind(id.as_str());
        }

        query
            .execute(&mut *tx)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to bulk update dates: {e}")))?;

        tx.commit()
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to commit transaction: {e}")))?;

        info!(
            "Bulk updated dates for {} task(s)",
            request.task_uuids.len()
        );
        Ok(crate::models::BulkOperationResult {
            success: true,
            processed_count: request.task_uuids.len(),
            message: format!(
                "Successfully updated dates for {} task(s)",
                request.task_uuids.len()
            ),
        })
    }

    /// Complete multiple tasks (transactional)
    ///
    /// All tasks must exist, or the entire operation will be rolled back.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Task UUIDs array is empty
    /// - Any task UUID is invalid or doesn't exist
    /// - Database operation fails
    #[instrument(skip(self))]
    pub async fn bulk_complete(
        &self,
        request: crate::models::BulkCompleteRequest,
    ) -> ThingsResult<crate::models::BulkOperationResult> {
        // Validation
        if request.task_uuids.is_empty() {
            return Err(ThingsError::validation("Task UUIDs cannot be empty"));
        }
        if request.task_uuids.len() > Self::MAX_BULK_BATCH_SIZE {
            return Err(ThingsError::validation(format!(
                "Batch size {} exceeds maximum of {}",
                request.task_uuids.len(),
                Self::MAX_BULK_BATCH_SIZE
            )));
        }

        // Begin transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to begin transaction: {e}")))?;

        // Validate all tasks exist in a single batch query (prevent N+1)
        let placeholders = request
            .task_uuids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query_str = format!(
            "SELECT uuid FROM TMTask WHERE uuid IN ({}) AND trashed = 0",
            placeholders
        );

        let mut query = sqlx::query(&query_str);
        for id in &request.task_uuids {
            query = query.bind(id.as_str());
        }

        let found_uuids: Vec<String> = query
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to validate tasks: {e}")))?
            .iter()
            .map(|row| row.get("uuid"))
            .collect();

        // Check if any UUIDs were not found
        if found_uuids.len() != request.task_uuids.len() {
            // Find the first missing UUID for error reporting
            for id in &request.task_uuids {
                if !found_uuids.contains(&id.to_string()) {
                    tx.rollback().await.ok();
                    return Err(ThingsError::TaskNotFound {
                        uuid: id.to_string(),
                    });
                }
            }
        }

        // Batch update: mark as completed
        let now = Utc::now().timestamp() as f64;
        let placeholders = request
            .task_uuids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query_str = format!(
            "UPDATE TMTask SET status = 3, stopDate = ?, userModificationDate = ? WHERE uuid IN ({})",
            placeholders
        );

        let mut query = sqlx::query(&query_str).bind(now).bind(now);

        for id in &request.task_uuids {
            query = query.bind(id.as_str());
        }

        query
            .execute(&mut *tx)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to bulk complete tasks: {e}")))?;

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to commit transaction: {e}")))?;

        info!("Bulk completed {} task(s)", request.task_uuids.len());
        Ok(crate::models::BulkOperationResult {
            success: true,
            processed_count: request.task_uuids.len(),
            message: format!(
                "Successfully completed {} task(s)",
                request.task_uuids.len()
            ),
        })
    }

    /// Delete multiple tasks (soft delete, transactional)
    ///
    /// All tasks must exist, or the entire operation will be rolled back.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Task UUIDs array is empty
    /// - Any task UUID is invalid or doesn't exist
    /// - Database operation fails
    #[instrument(skip(self))]
    pub async fn bulk_delete(
        &self,
        request: crate::models::BulkDeleteRequest,
    ) -> ThingsResult<crate::models::BulkOperationResult> {
        // Validation
        if request.task_uuids.is_empty() {
            return Err(ThingsError::validation("Task UUIDs cannot be empty"));
        }
        if request.task_uuids.len() > Self::MAX_BULK_BATCH_SIZE {
            return Err(ThingsError::validation(format!(
                "Batch size {} exceeds maximum of {}",
                request.task_uuids.len(),
                Self::MAX_BULK_BATCH_SIZE
            )));
        }

        // Begin transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to begin transaction: {e}")))?;

        // Validate all tasks exist in a single batch query (prevent N+1)
        let placeholders = request
            .task_uuids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query_str = format!(
            "SELECT uuid FROM TMTask WHERE uuid IN ({}) AND trashed = 0",
            placeholders
        );

        let mut query = sqlx::query(&query_str);
        for id in &request.task_uuids {
            query = query.bind(id.as_str());
        }

        let found_uuids: Vec<String> = query
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to validate tasks: {e}")))?
            .iter()
            .map(|row| row.get("uuid"))
            .collect();

        // Check if any UUIDs were not found
        if found_uuids.len() != request.task_uuids.len() {
            // Find the first missing UUID for error reporting
            for id in &request.task_uuids {
                if !found_uuids.contains(&id.to_string()) {
                    tx.rollback().await.ok();
                    return Err(ThingsError::TaskNotFound {
                        uuid: id.to_string(),
                    });
                }
            }
        }

        // Batch update: soft delete
        let now = Utc::now().timestamp() as f64;
        let placeholders = request
            .task_uuids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query_str = format!(
            "UPDATE TMTask SET trashed = 1, userModificationDate = ? WHERE uuid IN ({})",
            placeholders
        );

        let mut query = sqlx::query(&query_str).bind(now);

        for id in &request.task_uuids {
            query = query.bind(id.as_str());
        }

        query
            .execute(&mut *tx)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to bulk delete tasks: {e}")))?;

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to commit transaction: {e}")))?;

        info!("Bulk deleted {} task(s)", request.task_uuids.len());
        Ok(crate::models::BulkOperationResult {
            success: true,
            processed_count: request.task_uuids.len(),
            message: format!("Successfully deleted {} task(s)", request.task_uuids.len()),
        })
    }
}
