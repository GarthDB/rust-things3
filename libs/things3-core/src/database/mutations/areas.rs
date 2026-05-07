use crate::{
    database::{validators, ThingsDatabase},
    error::{Result as ThingsResult, ThingsError},
    models::ThingsId,
};
use chrono::Utc;
use tracing::{info, instrument};

impl ThingsDatabase {
    /// Create a new area
    ///
    /// # Errors
    ///
    /// Returns an error if the database insert fails
    #[instrument(skip(self))]
    pub async fn create_area(
        &self,
        request: crate::models::CreateAreaRequest,
    ) -> ThingsResult<ThingsId> {
        // Generate ID for new area
        let id = ThingsId::new_things_native();

        // Get current timestamp for creation/modification dates
        let now = Utc::now().timestamp() as f64;

        // Calculate next index (max + 1)
        let max_index: Option<i64> = sqlx::query_scalar("SELECT MAX(`index`) FROM TMArea")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to get max area index: {e}")))?;

        let next_index = max_index.unwrap_or(-1) + 1;

        // Insert into TMArea table
        sqlx::query(
            r"
            INSERT INTO TMArea (
                uuid, title, visible, `index`,
                creationDate, userModificationDate
            ) VALUES (?, ?, 1, ?, ?, ?)
            ",
        )
        .bind(id.as_str())
        .bind(&request.title)
        .bind(next_index)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to create area: {e}")))?;

        info!("Created area with UUID: {}", id);
        Ok(id)
    }

    /// Update an existing area
    ///
    /// # Errors
    ///
    /// Returns an error if the area doesn't exist or if the database update fails
    #[instrument(skip(self))]
    pub async fn update_area(&self, request: crate::models::UpdateAreaRequest) -> ThingsResult<()> {
        // Verify area exists
        validators::validate_area_exists(&self.pool, &request.uuid).await?;

        let now = Utc::now().timestamp() as f64;

        sqlx::query("UPDATE TMArea SET title = ?, userModificationDate = ? WHERE uuid = ?")
            .bind(&request.title)
            .bind(now)
            .bind(request.uuid.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to update area: {e}")))?;

        info!("Updated area with UUID: {}", request.uuid);
        Ok(())
    }

    /// Delete an area
    ///
    /// Hard delete (areas don't have a trashed field)
    /// Orphans all projects in the area by setting their area to NULL
    ///
    /// # Errors
    ///
    /// Returns an error if the area doesn't exist or if the database delete fails
    #[instrument(skip(self))]
    pub async fn delete_area(&self, id: &ThingsId) -> ThingsResult<()> {
        // Verify area exists
        validators::validate_area_exists(&self.pool, id).await?;

        let now = Utc::now().timestamp() as f64;

        // Orphan all projects in this area (set area to NULL)
        sqlx::query(
            "UPDATE TMTask SET area = NULL, userModificationDate = ? WHERE area = ? AND type = 1 AND trashed = 0",
        )
        .bind(now)
        .bind(id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to orphan projects in area: {e}")))?;

        // Delete the area (hard delete)
        sqlx::query("DELETE FROM TMArea WHERE uuid = ?")
            .bind(id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to delete area: {e}")))?;

        info!("Deleted area with UUID: {}", id);
        Ok(())
    }
}
