use crate::{
    database::{conversions::safe_timestamp_convert, ThingsDatabase},
    error::{Result as ThingsResult, ThingsError},
    models::ThingsId,
};
use chrono::DateTime;
use sqlx::Row;
use tracing::instrument;

impl ThingsDatabase {
    /// Find a tag by normalized title (exact match, case-insensitive)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn find_tag_by_normalized_title(
        &self,
        normalized: &str,
    ) -> ThingsResult<Option<crate::models::Tag>> {
        let row = sqlx::query(
            "SELECT uuid, title, shortcut, parent, usedDate
             FROM TMTag
             WHERE LOWER(title) = LOWER(?)",
        )
        .bind(normalized)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to find tag by title: {e}")))?;

        if let Some(row) = row {
            let uuid_str: String = row.get("uuid");
            let title: String = row.get("title");
            let shortcut: Option<String> = row.get("shortcut");
            let parent_str: Option<String> = row.get("parent");
            let parent_uuid = parent_str.map(ThingsId::from_trusted);

            let used_ts: Option<f64> = row.get("usedDate");
            let last_used = used_ts.and_then(|ts| {
                let ts_i64 = safe_timestamp_convert(ts);
                DateTime::from_timestamp(ts_i64, 0)
            });

            // Count usage by querying the TMTaskTag join table
            let usage_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM TMTask
                 WHERE trashed = 0
                 AND EXISTS (
                     SELECT 1 FROM TMTaskTag tt
                     WHERE tt.tasks = TMTask.uuid AND tt.tags = ?
                 )",
            )
            .bind(uuid_str.as_str())
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

            Ok(Some(crate::models::Tag {
                uuid: ThingsId::from_trusted(uuid_str),
                title,
                shortcut,
                parent_uuid,
                usage_count: usage_count as u32,
                last_used,
            }))
        } else {
            Ok(None)
        }
    }

    /// Find tags similar to the given title using fuzzy matching
    ///
    /// Returns tags sorted by similarity score (highest first)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn find_similar_tags(
        &self,
        title: &str,
        min_similarity: f32,
    ) -> ThingsResult<Vec<crate::models::TagMatch>> {
        use crate::database::tag_utils::{calculate_similarity, get_match_type};

        // Get all tags
        let all_tags = self.get_all_tags().await?;

        // Calculate similarity for each tag
        let mut matches: Vec<crate::models::TagMatch> = all_tags
            .into_iter()
            .filter_map(|tag| {
                let similarity = calculate_similarity(title, &tag.title);
                if similarity >= min_similarity {
                    let match_type = get_match_type(title, &tag.title, min_similarity);
                    Some(crate::models::TagMatch {
                        tag,
                        similarity_score: similarity,
                        match_type,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by similarity score (highest first)
        matches.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(matches)
    }

    /// Search tags by partial title match
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn search_tags(&self, query: &str) -> ThingsResult<Vec<crate::models::Tag>> {
        let rows = sqlx::query(
            "SELECT uuid, title, shortcut, parent, usedDate
             FROM TMTag
             WHERE title LIKE ?
             ORDER BY title",
        )
        .bind(format!("%{}%", query))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to search tags: {e}")))?;

        let mut tags = Vec::new();
        for row in rows {
            let uuid_str: String = row.get("uuid");
            let title: String = row.get("title");
            let shortcut: Option<String> = row.get("shortcut");
            let parent_str: Option<String> = row.get("parent");
            let parent_uuid = parent_str.map(ThingsId::from_trusted);

            let used_ts: Option<f64> = row.get("usedDate");
            let last_used = used_ts.and_then(|ts| {
                let ts_i64 = safe_timestamp_convert(ts);
                DateTime::from_timestamp(ts_i64, 0)
            });

            // Count usage via TMTaskTag join table
            let usage_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM TMTask
                 WHERE trashed = 0
                 AND EXISTS (
                     SELECT 1 FROM TMTaskTag tt
                     WHERE tt.tasks = TMTask.uuid AND tt.tags = ?
                 )",
            )
            .bind(uuid_str.as_str())
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

            tags.push(crate::models::Tag {
                uuid: ThingsId::from_trusted(uuid_str),
                title,
                shortcut,
                parent_uuid,
                usage_count: usage_count as u32,
                last_used,
            });
        }

        Ok(tags)
    }

    /// Get all tags ordered by title
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn get_all_tags(&self) -> ThingsResult<Vec<crate::models::Tag>> {
        let rows = sqlx::query(
            "SELECT uuid, title, shortcut, parent, usedDate
             FROM TMTag
             ORDER BY title",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to get all tags: {e}")))?;

        let mut tags = Vec::new();
        for row in rows {
            let uuid_str: String = row.get("uuid");
            let title: String = row.get("title");
            let shortcut: Option<String> = row.get("shortcut");
            let parent_str: Option<String> = row.get("parent");
            let parent_uuid = parent_str.map(ThingsId::from_trusted);

            let used_ts: Option<f64> = row.get("usedDate");
            let last_used = used_ts.and_then(|ts| {
                let ts_i64 = safe_timestamp_convert(ts);
                DateTime::from_timestamp(ts_i64, 0)
            });

            // Count usage via TMTaskTag join table
            let usage_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM TMTask
                 WHERE trashed = 0
                 AND EXISTS (
                     SELECT 1 FROM TMTaskTag tt
                     WHERE tt.tasks = TMTask.uuid AND tt.tags = ?
                 )",
            )
            .bind(uuid_str.as_str())
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

            tags.push(crate::models::Tag {
                uuid: ThingsId::from_trusted(uuid_str),
                title,
                shortcut,
                parent_uuid,
                usage_count: usage_count as u32,
                last_used,
            });
        }

        Ok(tags)
    }

    /// Get most frequently used tags
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn get_popular_tags(&self, limit: usize) -> ThingsResult<Vec<crate::models::Tag>> {
        let mut all_tags = self.get_all_tags().await?;

        // Sort by usage count (highest first)
        all_tags.sort_by_key(|t| std::cmp::Reverse(t.usage_count));

        // Take the top N
        all_tags.truncate(limit);

        Ok(all_tags)
    }

    /// Get recently used tags
    ///
    /// Recency is determined by `MAX(t.userModificationDate)` across non-trashed tasks
    /// referencing the tag — this reflects when those tasks were last touched rather than
    /// when they were created. Tags with no non-trashed task associations are excluded
    /// entirely (INNER JOIN); previously the old `usedDate`-based query would have returned
    /// them if `usedDate` were populated (it never is in practice).
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn get_recent_tags(&self, limit: usize) -> ThingsResult<Vec<crate::models::Tag>> {
        // Things 3 never populates `usedDate` for tags created via its own UI or
        // AppleScript. Instead, order by the most recent `userModificationDate` of any
        // non-trashed task that references the tag via the TMTaskTag join table.
        let rows = sqlx::query(
            "SELECT tg.uuid, tg.title, tg.shortcut, tg.parent,
                    COUNT(t.uuid) AS usage_count,
                    MAX(t.userModificationDate) AS most_recent
             FROM TMTag tg
             JOIN TMTaskTag tt ON tt.tags = tg.uuid
             JOIN TMTask t ON t.uuid = tt.tasks
             WHERE t.trashed = 0
             GROUP BY tg.uuid, tg.title, tg.shortcut, tg.parent
             ORDER BY most_recent DESC
             LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to get recent tags: {e}")))?;

        let mut tags = Vec::new();
        for row in rows {
            let uuid_str: String = row.get("uuid");
            let title: String = row.get("title");
            let shortcut: Option<String> = row.get("shortcut");
            let parent_str: Option<String> = row.get("parent");
            let parent_uuid = parent_str.map(ThingsId::from_trusted);
            let usage_count: i64 = row.get("usage_count");

            let most_recent_ts: Option<f64> = row.get("most_recent");
            let last_used = most_recent_ts.and_then(|ts| {
                let ts_i64 = safe_timestamp_convert(ts);
                DateTime::from_timestamp(ts_i64, 0)
            });

            tags.push(crate::models::Tag {
                uuid: ThingsId::from_trusted(uuid_str),
                title,
                shortcut,
                parent_uuid,
                usage_count: usage_count as u32,
                last_used,
            });
        }

        Ok(tags)
    }

    /// Get tag completions for partial input
    ///
    /// Returns tags sorted by:
    /// 1. Exact prefix matches (prioritized)
    /// 2. Contains matches
    /// 3. Fuzzy matches
    /// Within each category, sorted by usage frequency
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn get_tag_completions(
        &self,
        partial_input: &str,
        limit: usize,
    ) -> ThingsResult<Vec<crate::models::TagCompletion>> {
        use crate::database::tag_utils::{calculate_similarity, normalize_tag_title};

        let normalized_input = normalize_tag_title(partial_input);
        let all_tags = self.get_all_tags().await?;

        let mut completions: Vec<crate::models::TagCompletion> = all_tags
            .into_iter()
            .filter_map(|tag| {
                let normalized_tag = normalize_tag_title(&tag.title);

                // Calculate score based on match type
                let score = if normalized_tag.starts_with(&normalized_input) {
                    // Exact prefix match: highest priority
                    3.0 + (tag.usage_count as f32 / 100.0)
                } else if normalized_tag.contains(&normalized_input) {
                    // Contains match: medium priority
                    2.0 + (tag.usage_count as f32 / 100.0)
                } else {
                    // Fuzzy match: lower priority
                    let similarity = calculate_similarity(partial_input, &tag.title);
                    if similarity >= 0.6 {
                        similarity + (tag.usage_count as f32 / 1000.0)
                    } else {
                        return None; // Not similar enough
                    }
                };

                Some(crate::models::TagCompletion { tag, score })
            })
            .collect();

        // Sort by score (highest first)
        completions.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take the top N
        completions.truncate(limit);

        Ok(completions)
    }

    /// Get detailed statistics for a tag
    ///
    /// # Errors
    ///
    /// Returns an error if the tag doesn't exist or database query fails
    #[instrument(skip(self))]
    pub async fn get_tag_statistics(
        &self,
        id: &ThingsId,
    ) -> ThingsResult<crate::models::TagStatistics> {
        // Get the tag
        let tag_row = sqlx::query("SELECT title FROM TMTag WHERE uuid = ?")
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to find tag: {e}")))?;

        let title: String = tag_row
            .ok_or_else(|| ThingsError::unknown(format!("Tag not found: {}", id)))?
            .get("title");

        // Get all tasks using this tag via TMTaskTag join table
        let task_rows = sqlx::query(
            "SELECT tt.tasks AS uuid FROM TMTaskTag tt
             JOIN TMTask t ON t.uuid = tt.tasks
             WHERE tt.tags = ? AND t.trashed = 0",
        )
        .bind(id.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to query tasks with tag: {e}")))?;

        let task_uuids: Vec<ThingsId> = task_rows
            .iter()
            .map(|row| ThingsId::from_trusted(row.get::<String, _>("uuid")))
            .collect();

        let usage_count = task_uuids.len() as u32;

        // Find related tags (tags that frequently appear with this tag)
        let mut related_tags: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();

        for task_uuid in &task_uuids {
            let rows = sqlx::query(
                "SELECT DISTINCT tg.title FROM TMTaskTag tt
                 JOIN TMTag tg ON tg.uuid = tt.tags
                 WHERE tt.tasks = ?",
            )
            .bind(task_uuid.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to fetch task tags: {e}")))?;

            for row in rows {
                let tag_title: String = row.get("title");
                if tag_title != title {
                    *related_tags.entry(tag_title).or_insert(0) += 1;
                }
            }
        }

        // Sort related tags by co-occurrence count
        let mut related_vec: Vec<(String, u32)> = related_tags.into_iter().collect();
        related_vec.sort_by_key(|r| std::cmp::Reverse(r.1));

        Ok(crate::models::TagStatistics {
            uuid: id.clone(),
            title,
            usage_count,
            task_uuids,
            related_tags: related_vec,
        })
    }

    /// Find duplicate or highly similar tags
    ///
    /// Returns pairs of tags that are similar above the threshold
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    #[instrument(skip(self))]
    pub async fn find_duplicate_tags(
        &self,
        min_similarity: f32,
    ) -> ThingsResult<Vec<crate::models::TagPair>> {
        use crate::database::tag_utils::calculate_similarity;

        let all_tags = self.get_all_tags().await?;
        let mut pairs = Vec::new();

        // Compare each tag with every other tag
        for i in 0..all_tags.len() {
            for j in (i + 1)..all_tags.len() {
                let tag1 = &all_tags[i];
                let tag2 = &all_tags[j];

                let similarity = calculate_similarity(&tag1.title, &tag2.title);

                if similarity >= min_similarity {
                    pairs.push(crate::models::TagPair {
                        tag1: tag1.clone(),
                        tag2: tag2.clone(),
                        similarity,
                    });
                }
            }
        }

        // Sort by similarity (highest first)
        pairs.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(pairs)
    }
}
