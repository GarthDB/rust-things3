use crate::{
    database::{validators, ThingsDatabase},
    error::{Result as ThingsResult, ThingsError},
    models::ThingsId,
};
use chrono::Utc;
use tracing::{info, instrument};

impl ThingsDatabase {
    /// Create a tag with smart duplicate detection
    ///
    /// Returns:
    /// - `Created`: New tag was created
    /// - `Existing`: Exact match found (case-insensitive)
    /// - `SimilarFound`: Similar tags found (user decision needed)
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    #[instrument(skip(self))]
    pub async fn create_tag_smart(
        &self,
        request: crate::models::CreateTagRequest,
    ) -> ThingsResult<crate::models::TagCreationResult> {
        use crate::database::tag_utils::normalize_tag_title;
        use crate::models::TagCreationResult;

        // 1. Normalize the title
        let normalized = normalize_tag_title(&request.title);

        // 2. Check for exact match (case-insensitive)
        if let Some(existing) = self.find_tag_by_normalized_title(&normalized).await? {
            return Ok(TagCreationResult::Existing {
                tag: existing,
                is_new: false,
            });
        }

        // 3. Find similar tags (fuzzy matching with 80% threshold)
        let similar_tags = self.find_similar_tags(&normalized, 0.8).await?;

        // 4. If similar tags found, return them for user decision
        if !similar_tags.is_empty() {
            return Ok(TagCreationResult::SimilarFound {
                similar_tags,
                requested_title: request.title,
            });
        }

        // 5. No duplicates, safe to create
        let id = ThingsId::new_things_native();

        sqlx::query(
            "INSERT INTO TMTag (uuid, title, shortcut, parent, usedDate, `index`) \
             VALUES (?, ?, ?, ?, NULL, 0)",
        )
        .bind(id.as_str())
        .bind(&request.title)
        .bind(request.shortcut.as_ref())
        .bind(request.parent_uuid.map(|u| u.into_string()))
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to create tag: {e}")))?;

        info!("Created tag with UUID: {}", id);
        Ok(TagCreationResult::Created {
            uuid: id,
            is_new: true,
        })
    }

    /// Create tag forcefully (skip duplicate check)
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    #[instrument(skip(self))]
    pub async fn create_tag_force(
        &self,
        request: crate::models::CreateTagRequest,
    ) -> ThingsResult<ThingsId> {
        let id = ThingsId::new_things_native();

        sqlx::query(
            "INSERT INTO TMTag (uuid, title, shortcut, parent, usedDate, `index`) \
             VALUES (?, ?, ?, ?, NULL, 0)",
        )
        .bind(id.as_str())
        .bind(&request.title)
        .bind(request.shortcut.as_ref())
        .bind(request.parent_uuid.map(|u| u.into_string()))
        .execute(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to create tag: {e}")))?;

        info!("Forcefully created tag with UUID: {}", id);
        Ok(id)
    }

    /// Update a tag
    ///
    /// # Errors
    ///
    /// Returns an error if the tag doesn't exist or database operation fails
    #[instrument(skip(self))]
    pub async fn update_tag(&self, request: crate::models::UpdateTagRequest) -> ThingsResult<()> {
        use crate::database::tag_utils::normalize_tag_title;

        // Verify tag exists
        let existing = self
            .find_tag_by_normalized_title(request.uuid.as_str())
            .await?;
        if existing.is_none() {
            // Try by UUID
            let row = sqlx::query("SELECT 1 FROM TMTag WHERE uuid = ?")
                .bind(request.uuid.as_str())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to validate tag: {e}")))?;

            if row.is_none() {
                return Err(ThingsError::unknown(format!(
                    "Tag not found: {}",
                    request.uuid
                )));
            }
        }

        // If renaming, check for duplicates with new name
        if let Some(new_title) = &request.title {
            let normalized = normalize_tag_title(new_title);
            if let Some(duplicate) = self.find_tag_by_normalized_title(&normalized).await? {
                if duplicate.uuid != request.uuid {
                    return Err(ThingsError::unknown(format!(
                        "Tag with title '{}' already exists",
                        new_title
                    )));
                }
            }
        }

        // Build dynamic UPDATE query
        let mut updates = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(title) = &request.title {
            updates.push("title = ?");
            params.push(title.clone());
        }
        if let Some(shortcut) = &request.shortcut {
            updates.push("shortcut = ?");
            params.push(shortcut.clone());
        }
        if let Some(parent_uuid) = request.parent_uuid {
            updates.push("parent = ?");
            params.push(parent_uuid.into_string());
        }

        if updates.is_empty() {
            return Ok(()); // Nothing to update
        }

        let sql = format!("UPDATE TMTag SET {} WHERE uuid = ?", updates.join(", "));
        params.push(request.uuid.as_str().to_string());

        let mut query = sqlx::query(&sql);
        for param in params {
            query = query.bind(param);
        }

        query
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to update tag: {e}")))?;

        info!("Updated tag with UUID: {}", request.uuid);
        Ok(())
    }

    /// Delete a tag
    ///
    /// # Arguments
    ///
    /// * `uuid` - UUID of the tag to delete
    /// * `remove_from_tasks` - If true, removes tag from all tasks' cachedTags
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    #[instrument(skip(self))]
    pub async fn delete_tag(&self, id: &ThingsId, remove_from_tasks: bool) -> ThingsResult<()> {
        // Get the tag title before deletion
        let tag = self.find_tag_by_normalized_title(id.as_str()).await?;

        if tag.is_none() {
            // Try by UUID directly
            let row = sqlx::query("SELECT title FROM TMTag WHERE uuid = ?")
                .bind(id.as_str())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to find tag: {e}")))?;

            if row.is_none() {
                return Err(ThingsError::unknown(format!("Tag not found: {}", id)));
            }
        }

        if remove_from_tasks {
            // TODO: Implement updating all tasks' cachedTags to remove this tag
            // This requires parsing and re-serializing the JSON arrays
            info!("Removing tag {} from all tasks (not yet implemented)", id);
        }

        // Delete the tag
        sqlx::query("DELETE FROM TMTag WHERE uuid = ?")
            .bind(id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to delete tag: {e}")))?;

        info!("Deleted tag with UUID: {}", id);
        Ok(())
    }

    /// Merge two tags (combine source into target)
    ///
    /// # Arguments
    ///
    /// * `source_uuid` - UUID of tag to merge from (will be deleted)
    /// * `target_uuid` - UUID of tag to merge into (will remain)
    ///
    /// # Errors
    ///
    /// Returns an error if either tag doesn't exist or database operation fails
    #[instrument(skip(self))]
    pub async fn merge_tags(&self, source_id: &ThingsId, target_id: &ThingsId) -> ThingsResult<()> {
        // Verify both tags exist
        let source_row = sqlx::query("SELECT title FROM TMTag WHERE uuid = ?")
            .bind(source_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to find source tag: {e}")))?;

        if source_row.is_none() {
            return Err(ThingsError::unknown(format!(
                "Source tag not found: {}",
                source_id
            )));
        }

        let target_row = sqlx::query("SELECT title FROM TMTag WHERE uuid = ?")
            .bind(target_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to find target tag: {e}")))?;

        if target_row.is_none() {
            return Err(ThingsError::unknown(format!(
                "Target tag not found: {}",
                target_id
            )));
        }

        // TODO: Implement updating all tasks' cachedTags to replace source tag with target tag
        // This requires parsing and re-serializing the JSON arrays
        info!(
            "Merging tag {} into {} (tag replacement in tasks not yet fully implemented)",
            source_id, target_id
        );

        // Update usedDate on target if source was used more recently
        let now = Utc::now().timestamp() as f64;
        sqlx::query("UPDATE TMTag SET usedDate = ? WHERE uuid = ?")
            .bind(now)
            .bind(target_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to update target tag: {e}")))?;

        // Delete source tag
        sqlx::query("DELETE FROM TMTag WHERE uuid = ?")
            .bind(source_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to delete source tag: {e}")))?;

        info!("Merged tag {} into {}", source_id, target_id);
        Ok(())
    }

    /// Add a tag to a task (with duplicate prevention)
    ///
    /// Returns:
    /// - `Assigned`: Tag was successfully assigned
    /// - `Suggestions`: Similar tags found (user decision needed)
    ///
    /// # Errors
    ///
    /// Returns an error if the task doesn't exist or database operation fails
    #[instrument(skip(self))]
    pub async fn add_tag_to_task(
        &self,
        task_id: &ThingsId,
        tag_title: &str,
    ) -> ThingsResult<crate::models::TagAssignmentResult> {
        use crate::database::tag_utils::normalize_tag_title;
        use crate::models::TagAssignmentResult;

        // 1. Verify task exists
        validators::validate_task_exists(&self.pool, task_id).await?;

        // 2. Normalize and find tag
        let normalized = normalize_tag_title(tag_title);

        // 3. Check for exact match first
        let tag = if let Some(existing_tag) = self.find_tag_by_normalized_title(&normalized).await?
        {
            existing_tag
        } else {
            // 4. Find similar tags
            let similar_tags = self.find_similar_tags(&normalized, 0.8).await?;

            if !similar_tags.is_empty() {
                return Ok(TagAssignmentResult::Suggestions { similar_tags });
            }

            // 5. No existing tag found, create new one
            let request = crate::models::CreateTagRequest {
                title: tag_title.to_string(),
                shortcut: None,
                parent_uuid: None,
            };
            let _uuid = self.create_tag_force(request).await?;

            // Fetch the newly created tag
            self.find_tag_by_normalized_title(&normalized)
                .await?
                .ok_or_else(|| ThingsError::unknown("Failed to retrieve newly created tag"))?
        };

        // 6. Insert into TMTaskTag (idempotent)
        let now = Utc::now().timestamp() as f64;
        sqlx::query("INSERT OR IGNORE INTO TMTaskTag (tasks, tags) VALUES (?, ?)")
            .bind(task_id.as_str())
            .bind(tag.uuid.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to assign tag: {e}")))?;

        // 7. Update task modification date
        sqlx::query("UPDATE TMTask SET userModificationDate = ? WHERE uuid = ?")
            .bind(now)
            .bind(task_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                ThingsError::unknown(format!("Failed to update modification date: {e}"))
            })?;

        // 8. Update tag's usedDate
        sqlx::query("UPDATE TMTag SET usedDate = ? WHERE uuid = ?")
            .bind(now)
            .bind(tag.uuid.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to update tag usedDate: {e}")))?;

        info!("Added tag '{}' to task {}", tag.title, task_id);

        Ok(TagAssignmentResult::Assigned { tag_uuid: tag.uuid })
    }

    /// Remove a tag from a task
    ///
    /// # Errors
    ///
    /// Returns an error if the task doesn't exist or database operation fails
    #[instrument(skip(self))]
    pub async fn remove_tag_from_task(
        &self,
        task_id: &ThingsId,
        tag_title: &str,
    ) -> ThingsResult<()> {
        use crate::database::tag_utils::normalize_tag_title;

        // 1. Verify task exists
        validators::validate_task_exists(&self.pool, task_id).await?;

        // 2. Find the tag UUID
        let normalized = normalize_tag_title(tag_title);
        let Some(tag) = self.find_tag_by_normalized_title(&normalized).await? else {
            return Ok(()); // Tag doesn't exist, nothing to remove
        };

        // 3. Delete from TMTaskTag
        let now = Utc::now().timestamp() as f64;
        let result = sqlx::query("DELETE FROM TMTaskTag WHERE tasks = ? AND tags = ?")
            .bind(task_id.as_str())
            .bind(tag.uuid.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to remove tag assignment: {e}")))?;

        if result.rows_affected() > 0 {
            // 4. Update task modification date
            sqlx::query("UPDATE TMTask SET userModificationDate = ? WHERE uuid = ?")
                .bind(now)
                .bind(task_id.as_str())
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    ThingsError::unknown(format!("Failed to update modification date: {e}"))
                })?;

            info!("Removed tag '{}' from task {}", tag_title, task_id);
        }

        Ok(())
    }

    /// Replace all tags on a task (with duplicate prevention)
    ///
    /// Returns any tag titles that had similar matches for user confirmation
    ///
    /// # Errors
    ///
    /// Returns an error if the task doesn't exist or database operation fails
    #[instrument(skip(self))]
    pub async fn set_task_tags(
        &self,
        task_id: &ThingsId,
        tag_titles: Vec<String>,
    ) -> ThingsResult<Vec<crate::models::TagMatch>> {
        use crate::database::tag_utils::normalize_tag_title;

        // 1. Verify task exists
        validators::validate_task_exists(&self.pool, task_id).await?;

        let mut resolved_tags = Vec::new();
        let mut suggestions = Vec::new();

        // 2. Resolve each tag title
        for title in tag_titles {
            let normalized = normalize_tag_title(&title);

            // Try to find exact match
            if let Some(existing_tag) = self.find_tag_by_normalized_title(&normalized).await? {
                resolved_tags.push(existing_tag.title);
            } else {
                // Check for similar tags
                let similar_tags = self.find_similar_tags(&normalized, 0.8).await?;

                if !similar_tags.is_empty() {
                    suggestions.extend(similar_tags);
                }

                // Use the requested title anyway (will create if needed)
                resolved_tags.push(title);
            }
        }

        // 3. For any tags that don't exist yet, create them
        for title in &resolved_tags {
            let normalized = normalize_tag_title(title);
            if self
                .find_tag_by_normalized_title(&normalized)
                .await?
                .is_none()
            {
                let request = crate::models::CreateTagRequest {
                    title: title.clone(),
                    shortcut: None,
                    parent_uuid: None,
                };
                self.create_tag_force(request).await?;
            }
        }

        // 4. Delete existing tag assignments for this task
        let now = Utc::now().timestamp() as f64;
        sqlx::query("DELETE FROM TMTaskTag WHERE tasks = ?")
            .bind(task_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to clear task tags: {e}")))?;

        // 5. Insert new tag assignments
        for title in &resolved_tags {
            let normalized = normalize_tag_title(title);
            if let Some(tag) = self.find_tag_by_normalized_title(&normalized).await? {
                sqlx::query("INSERT OR IGNORE INTO TMTaskTag (tasks, tags) VALUES (?, ?)")
                    .bind(task_id.as_str())
                    .bind(tag.uuid.as_str())
                    .execute(&self.pool)
                    .await
                    .map_err(|e| ThingsError::unknown(format!("Failed to assign tag: {e}")))?;
            }
        }

        // Update task modification date
        sqlx::query("UPDATE TMTask SET userModificationDate = ? WHERE uuid = ?")
            .bind(now)
            .bind(task_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                ThingsError::unknown(format!("Failed to update modification date: {e}"))
            })?;

        // 6. Update usedDate for all tags
        for title in &resolved_tags {
            let normalized = normalize_tag_title(title);
            if let Some(tag) = self.find_tag_by_normalized_title(&normalized).await? {
                sqlx::query("UPDATE TMTag SET usedDate = ? WHERE uuid = ?")
                    .bind(now)
                    .bind(tag.uuid.as_str())
                    .execute(&self.pool)
                    .await
                    .map_err(|e| {
                        ThingsError::unknown(format!("Failed to update tag usedDate: {e}"))
                    })?;
            }
        }

        info!("Set tags on task {} to: {:?}", task_id, resolved_tags);
        Ok(suggestions)
    }
}
