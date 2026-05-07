#[cfg(feature = "advanced-queries")]
use crate::database::conversions::naive_date_to_things_timestamp;
#[cfg(feature = "advanced-queries")]
use crate::models::TaskFilters;
use crate::{
    database::{mappers::map_task_row, ThingsDatabase},
    error::{Result as ThingsResult, ThingsError},
    models::{Task, TaskStatus, TaskType, ThingsId},
};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::Row;
use tracing::{debug, instrument};
#[cfg(any(feature = "advanced-queries", feature = "batch-operations"))]
use uuid::Uuid;

impl ThingsDatabase {
    /// Get all tasks from the database
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use things3_core::{ThingsDatabase, ThingsError};
    /// use std::path::Path;
    ///
    /// # async fn example() -> Result<(), ThingsError> {
    /// let db = ThingsDatabase::new(Path::new("/path/to/things.db")).await?;
    ///
    /// // Get all tasks
    /// let tasks = db.get_all_tasks().await?;
    /// println!("Found {} total tasks", tasks.len());
    ///
    /// // Filter tasks by status
    /// let incomplete: Vec<_> = tasks.iter()
    ///     .filter(|t| t.status == things3_core::TaskStatus::Incomplete)
    ///     .collect();
    /// println!("Found {} incomplete tasks", incomplete.len());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if task data is invalid
    #[instrument]
    pub async fn get_all_tasks(&self) -> ThingsResult<Vec<Task>> {
        let rows = sqlx::query(
            r"
            SELECT 
                uuid, title, status, type, 
                start_date, due_date, 
                project_uuid, area_uuid, 
                notes, tags, 
                created, modified
            FROM TMTask
            ORDER BY created DESC
            ",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch tasks: {e}")))?;

        let mut tasks = Vec::new();
        for row in rows {
            let task = Task {
                uuid: ThingsId::from_trusted(row.get::<String, _>("uuid")),
                title: row.get("title"),
                status: TaskStatus::from_i32(row.get("status")).unwrap_or(TaskStatus::Incomplete),
                task_type: TaskType::from_i32(row.get("type")).unwrap_or(TaskType::Todo),
                start_date: row
                    .get::<Option<String>, _>("start_date")
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                deadline: row
                    .get::<Option<String>, _>("due_date")
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                project_uuid: row
                    .get::<Option<String>, _>("project_uuid")
                    .map(ThingsId::from_trusted),
                area_uuid: row
                    .get::<Option<String>, _>("area_uuid")
                    .map(ThingsId::from_trusted),
                parent_uuid: None, // Not available in this query
                notes: row.get("notes"),
                tags: row
                    .get::<Option<String>, _>("tags")
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
                children: Vec::new(), // Not available in this query
                created: DateTime::parse_from_rfc3339(&row.get::<String, _>("created"))
                    .ok()
                    .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc)),
                modified: DateTime::parse_from_rfc3339(&row.get::<String, _>("modified"))
                    .ok()
                    .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc)),
                stop_date: None, // Not available in this query context
            };
            tasks.push(task);
        }

        debug!("Fetched {} tasks", tasks.len());
        Ok(tasks)
    }

    /// Get tasks by status
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if task data is invalid
    #[instrument]
    pub async fn get_tasks_by_status(&self, status: TaskStatus) -> ThingsResult<Vec<Task>> {
        let status_value = status as i32;
        let rows = sqlx::query(
            r"
            SELECT 
                uuid, title, status, type, 
                start_date, due_date, 
                project_uuid, area_uuid, 
                notes, tags, 
                created, modified
             FROM TMTask 
            WHERE status = ?
            ORDER BY created DESC
            ",
        )
        .bind(status_value)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch tasks by status: {e}")))?;

        let mut tasks = Vec::new();
        for row in rows {
            let task = Task {
                uuid: ThingsId::from_trusted(row.get::<String, _>("uuid")),
                title: row.get("title"),
                status: TaskStatus::from_i32(row.get("status")).unwrap_or(TaskStatus::Incomplete),
                task_type: TaskType::from_i32(row.get("type")).unwrap_or(TaskType::Todo),
                start_date: row
                    .get::<Option<String>, _>("start_date")
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                deadline: row
                    .get::<Option<String>, _>("due_date")
                    .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
                project_uuid: row
                    .get::<Option<String>, _>("project_uuid")
                    .map(ThingsId::from_trusted),
                area_uuid: row
                    .get::<Option<String>, _>("area_uuid")
                    .map(ThingsId::from_trusted),
                parent_uuid: None, // Not available in this query
                notes: row.get("notes"),
                tags: row
                    .get::<Option<String>, _>("tags")
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
                children: Vec::new(), // Not available in this query
                created: DateTime::parse_from_rfc3339(&row.get::<String, _>("created"))
                    .ok()
                    .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc)),
                modified: DateTime::parse_from_rfc3339(&row.get::<String, _>("modified"))
                    .ok()
                    .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc)),
                stop_date: None, // Not available in this query context
            };
            tasks.push(task);
        }

        debug!("Fetched {} tasks with status {:?}", tasks.len(), status);
        Ok(tasks)
    }

    /// Search tasks by title or notes
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if task data is invalid
    #[instrument]
    pub async fn search_tasks(&self, query: &str) -> ThingsResult<Vec<Task>> {
        let search_pattern = format!("%{query}%");
        let rows = sqlx::query(
            r"
            SELECT
                uuid, title, status, type,
                startDate, deadline, stopDate,
                project, area, heading,
                notes,
                (SELECT GROUP_CONCAT(tg.title, char(31))
                   FROM TMTaskTag tt
                   JOIN TMTag tg ON tg.uuid = tt.tags
                  WHERE tt.tasks = TMTask.uuid) AS tags_csv,
                creationDate, userModificationDate
            FROM TMTask
            WHERE (title LIKE ? OR notes LIKE ?) AND type IN (0, 2) AND trashed = 0
            ORDER BY creationDate DESC
            ",
        )
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to search tasks: {e}")))?;

        let tasks = rows
            .iter()
            .map(map_task_row)
            .collect::<ThingsResult<Vec<Task>>>()?;

        debug!("Found {} tasks matching query: {}", tasks.len(), query);
        Ok(tasks)
    }

    /// Query tasks using a [`TaskFilters`] struct produced by [`crate::query::TaskQueryBuilder`].
    ///
    /// All filter fields are optional and combined with AND semantics in SQL.
    /// Tag and search-query filters are applied in Rust after the SQL query returns
    /// (Things 3 stores tags as a BLOB). When those post-filters are active,
    /// `LIMIT`/`OFFSET` is also applied in Rust so pagination counts only
    /// matching rows; without post-filters it is applied in SQL for efficiency.
    ///
    /// Tag matching via `filters.tags` is case-sensitive.
    ///
    /// Filtering by [`TaskStatus::Trashed`] queries rows where `trashed = 1`
    /// rather than adding a `status` condition, matching Things 3's soft-delete
    /// semantics (trashed rows keep their original status value).
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or task data cannot be mapped.
    #[cfg(any(feature = "advanced-queries", feature = "batch-operations"))]
    pub async fn query_tasks(&self, filters: &TaskFilters) -> ThingsResult<Vec<Task>> {
        self.query_tasks_inner(filters, None).await
    }

    /// Internal query path that optionally applies a cursor WHERE clause.
    ///
    /// `after` is `(seconds_since_unix_epoch, uuid)` of the last-returned
    /// task. When `Some`, an additional `WHERE` clause restricts results to
    /// rows strictly older than that anchor in the canonical
    /// `CAST(creationDate AS INTEGER) DESC, uuid DESC` ordering.
    ///
    /// Gated on either `advanced-queries` or `batch-operations` because both
    /// public surfaces (`query_tasks` and `execute_paged`) share this engine.
    #[cfg(any(feature = "advanced-queries", feature = "batch-operations"))]
    pub(crate) async fn query_tasks_inner(
        &self,
        filters: &TaskFilters,
        after: Option<(i64, Uuid)>,
    ) -> ThingsResult<Vec<Task>> {
        const COLS: &str = "uuid, title, type, status, notes, startDate, deadline, stopDate, \
                            creationDate, userModificationDate, project, area, heading, \
                            (SELECT GROUP_CONCAT(tg.title, char(31)) \
                               FROM TMTaskTag tt \
                               JOIN TMTag tg ON tg.uuid = tt.tags \
                              WHERE tt.tasks = TMTask.uuid) AS tags_csv";

        // Things 3 soft-deletes by setting trashed = 1; the status column is unchanged.
        // Requesting Trashed means "show trashed rows", not a status = 3 predicate.
        let trashed_val = i32::from(matches!(filters.status, Some(TaskStatus::Trashed)));
        let mut conditions: Vec<String> = vec![format!("trashed = {trashed_val}")];

        if let Some(status) = filters.status {
            let n = match status {
                TaskStatus::Incomplete => Some(0),
                TaskStatus::Canceled => Some(2),
                TaskStatus::Completed => Some(3),
                TaskStatus::Trashed => None, // handled via trashed = 1 above
            };
            if let Some(n) = n {
                conditions.push(format!("status = {n}"));
            }
        }

        if let Some(task_type) = filters.task_type {
            let n = match task_type {
                TaskType::Todo => 0,
                TaskType::Project => 1,
                TaskType::Heading => 2,
                TaskType::Area => 3,
            };
            conditions.push(format!("type = {n}"));
        }

        if let Some(ref uuid) = filters.project_uuid {
            conditions.push(format!("project = '{uuid}'"));
        }

        if let Some(ref uuid) = filters.area_uuid {
            conditions.push(format!("area = '{uuid}'"));
        }

        if let Some(from) = filters.start_date_from {
            conditions.push(format!(
                "startDate >= {}",
                naive_date_to_things_timestamp(from)
            ));
        }
        if let Some(to) = filters.start_date_to {
            conditions.push(format!(
                "startDate <= {}",
                naive_date_to_things_timestamp(to)
            ));
        }

        if let Some(from) = filters.deadline_from {
            conditions.push(format!(
                "deadline >= {}",
                naive_date_to_things_timestamp(from)
            ));
        }
        if let Some(to) = filters.deadline_to {
            conditions.push(format!(
                "deadline <= {}",
                naive_date_to_things_timestamp(to)
            ));
        }

        if let Some((after_seconds, _)) = after {
            // Strictly less than the cursor in (truncated_seconds DESC, uuid DESC)
            // ordering — i.e. older second, or same second with smaller uuid.
            // Casting to INTEGER matches the precision of `Task::created`,
            // which is reconstructed at second precision when reading rows.
            // UUID is bound as a parameter (?) rather than interpolated for
            // consistency with the rest of the codebase's query practices.
            conditions.push(format!(
                "(CAST(creationDate AS INTEGER) < {after_seconds} \
                 OR (CAST(creationDate AS INTEGER) = {after_seconds} AND uuid < ?))"
            ));
        }

        let where_clause = conditions.join(" AND ");
        // ORDER BY uses the truncated-second value so it agrees with the
        // cursor pagination logic (which compares at second precision because
        // `Task::created` is reconstructed at second precision). `uuid DESC` is
        // a deterministic tiebreak within the same second.
        let mut sql = format!(
            "SELECT {COLS} FROM TMTask WHERE {where_clause} \
             ORDER BY CAST(creationDate AS INTEGER) DESC, uuid DESC"
        );

        // When tags or search_query are active, LIMIT/OFFSET must be applied in Rust
        // after post-filtering, because SQL LIMIT would count non-matching rows.
        let has_post_filters =
            filters.tags.as_ref().is_some_and(|t| !t.is_empty()) || filters.search_query.is_some();

        if !has_post_filters {
            match (filters.limit, filters.offset) {
                (Some(limit), Some(offset)) => {
                    sql.push_str(&format!(" LIMIT {limit} OFFSET {offset}"));
                }
                (Some(limit), None) => {
                    sql.push_str(&format!(" LIMIT {limit}"));
                }
                (None, Some(offset)) => {
                    // SQLite requires LIMIT when OFFSET is used; -1 means unlimited
                    sql.push_str(&format!(" LIMIT -1 OFFSET {offset}"));
                }
                (None, None) => {}
            }
        }

        let rows = if let Some((_, after_uuid)) = after {
            sqlx::query(&sql)
                .bind(after_uuid.to_string())
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query(&sql).fetch_all(&self.pool).await
        }
        .map_err(|e| ThingsError::unknown(format!("Failed to query tasks: {e}")))?;

        let mut tasks = rows
            .iter()
            .map(map_task_row)
            .collect::<ThingsResult<Vec<Task>>>()?;

        if let Some(ref filter_tags) = filters.tags {
            if !filter_tags.is_empty() {
                tasks.retain(|task| filter_tags.iter().all(|f| task.tags.contains(f)));
            }
        }

        if let Some(ref q) = filters.search_query {
            let q_lower = q.to_lowercase();
            tasks.retain(|task| {
                task.title.to_lowercase().contains(&q_lower)
                    || task
                        .notes
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q_lower)
            });
        }

        if has_post_filters {
            let offset = filters.offset.unwrap_or(0);
            tasks = tasks.into_iter().skip(offset).collect();
            if let Some(limit) = filters.limit {
                tasks.truncate(limit);
            }
        }

        Ok(tasks)
    }

    /// Search completed tasks in the logbook
    ///
    /// Returns completed tasks matching the provided filters.
    /// All filters are optional and can be combined.
    ///
    /// # Parameters
    ///
    /// - `search_text`: Search in task titles and notes (case-insensitive)
    /// - `from_date`: Start date for completion date range
    /// - `to_date`: End date for completion date range
    /// - `project_uuid`: Filter by project UUID
    /// - `area_uuid`: Filter by area UUID
    /// - `tags`: Filter by tags (all tags must match)
    /// - `limit`: Maximum number of results (default: 50)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if task data is invalid
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip(self))]
    pub async fn search_logbook(
        &self,
        search_text: Option<String>,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
        project_uuid: Option<ThingsId>,
        area_uuid: Option<ThingsId>,
        tags: Option<Vec<String>>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> ThingsResult<Vec<Task>> {
        // Apply limit and offset
        let result_limit = limit.unwrap_or(50).min(500);
        let result_offset = offset.unwrap_or(0);

        // Build and execute query based on filters
        // type = 0 (Todo) is intentional here: headings (type=2) have no stopDate and
        // cannot appear in a stop-date-ordered logbook.
        let rows = if let Some(ref text) = search_text {
            let pattern = format!("%{text}%");
            let mut q = String::from(
                "SELECT uuid, title, status, type, startDate, deadline, stopDate, project, area, heading, notes, (SELECT GROUP_CONCAT(tg.title, char(31)) FROM TMTaskTag tt JOIN TMTag tg ON tg.uuid = tt.tags WHERE tt.tasks = TMTask.uuid) AS tags_csv, creationDate, userModificationDate FROM TMTask WHERE status = 3 AND trashed = 0 AND type = 0",
            );
            q.push_str(" AND (title LIKE ? OR notes LIKE ?)");

            if let Some(date) = from_date {
                // stopDate is stored as Unix timestamp (seconds since 1970-01-01)
                let date_time = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let timestamp = date_time.timestamp() as f64;
                q.push_str(&format!(" AND stopDate >= {}", timestamp));
            }

            if let Some(date) = to_date {
                // Include tasks completed on to_date by adding 1 day
                let end_date = date + chrono::Duration::days(1);
                let date_time = end_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let timestamp = date_time.timestamp() as f64;
                q.push_str(&format!(" AND stopDate < {}", timestamp));
            }

            if let Some(ref id) = project_uuid {
                q.push_str(&format!(" AND project = '{}'", id));
            }

            if let Some(ref id) = area_uuid {
                q.push_str(&format!(" AND area = '{}'", id));
            }

            q.push_str(&format!(
                " ORDER BY stopDate DESC LIMIT {result_limit} OFFSET {result_offset}"
            ));

            sqlx::query(&q)
                .bind(&pattern)
                .bind(&pattern)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to search logbook: {e}")))?
        } else {
            let mut q = String::from(
                "SELECT uuid, title, status, type, startDate, deadline, stopDate, project, area, heading, notes, (SELECT GROUP_CONCAT(tg.title, char(31)) FROM TMTaskTag tt JOIN TMTag tg ON tg.uuid = tt.tags WHERE tt.tasks = TMTask.uuid) AS tags_csv, creationDate, userModificationDate FROM TMTask WHERE status = 3 AND trashed = 0 AND type = 0",
            );

            if let Some(date) = from_date {
                // stopDate is stored as Unix timestamp (seconds since 1970-01-01)
                let date_time = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let timestamp = date_time.timestamp() as f64;
                q.push_str(&format!(" AND stopDate >= {}", timestamp));
            }

            if let Some(date) = to_date {
                // Include tasks completed on to_date by adding 1 day
                let end_date = date + chrono::Duration::days(1);
                let date_time = end_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let timestamp = date_time.timestamp() as f64;
                q.push_str(&format!(" AND stopDate < {}", timestamp));
            }

            if let Some(ref id) = project_uuid {
                q.push_str(&format!(" AND project = '{}'", id));
            }

            if let Some(ref id) = area_uuid {
                q.push_str(&format!(" AND area = '{}'", id));
            }

            q.push_str(&format!(
                " ORDER BY stopDate DESC LIMIT {result_limit} OFFSET {result_offset}"
            ));

            sqlx::query(&q)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ThingsError::unknown(format!("Failed to search logbook: {e}")))?
        };

        // Filter by tags if provided
        let mut tasks = rows
            .iter()
            .map(map_task_row)
            .collect::<ThingsResult<Vec<Task>>>()?;

        if let Some(ref filter_tags) = tags {
            if !filter_tags.is_empty() {
                tasks.retain(|task| {
                    // Check if task has all required tags
                    filter_tags
                        .iter()
                        .all(|filter_tag| task.tags.contains(filter_tag))
                });
            }
        }

        debug!("Found {} completed tasks in logbook", tasks.len());
        Ok(tasks)
    }

    /// Get inbox tasks (incomplete tasks without project)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if task data is invalid
    #[instrument(skip(self))]
    pub async fn get_inbox(&self, limit: Option<usize>) -> ThingsResult<Vec<Task>> {
        let query = if let Some(limit) = limit {
            format!("SELECT uuid, title, type, status, notes, startDate, deadline, stopDate, creationDate, userModificationDate, project, area, heading, (SELECT GROUP_CONCAT(tg.title, char(31)) FROM TMTaskTag tt JOIN TMTag tg ON tg.uuid = tt.tags WHERE tt.tasks = TMTask.uuid) AS tags_csv FROM TMTask WHERE type IN (0, 2) AND status = 0 AND project IS NULL AND trashed = 0 ORDER BY creationDate DESC LIMIT {limit}")
        } else {
            "SELECT uuid, title, type, status, notes, startDate, deadline, stopDate, creationDate, userModificationDate, project, area, heading, (SELECT GROUP_CONCAT(tg.title, char(31)) FROM TMTaskTag tt JOIN TMTag tg ON tg.uuid = tt.tags WHERE tt.tasks = TMTask.uuid) AS tags_csv FROM TMTask WHERE type IN (0, 2) AND status = 0 AND project IS NULL AND trashed = 0 ORDER BY creationDate DESC"
                .to_string()
        };

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to fetch inbox tasks: {e}")))?;

        let tasks = rows
            .iter()
            .map(map_task_row)
            .collect::<ThingsResult<Vec<Task>>>()?;

        Ok(tasks)
    }

    /// Get today's tasks (incomplete tasks due today or started today)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or if task data is invalid
    ///
    /// # Panics
    ///
    /// Panics if the current date cannot be converted to a valid time with hours, minutes, and seconds
    #[instrument(skip(self))]
    pub async fn get_today(&self, limit: Option<usize>) -> ThingsResult<Vec<Task>> {
        // Things 3 uses the `todayIndex` column to mark tasks that appear in "Today"
        // A task is in "Today" if todayIndex IS NOT NULL AND todayIndex != 0
        let query = if let Some(limit) = limit {
            format!(
                "SELECT uuid, title, type, status, notes, startDate, deadline, stopDate, creationDate, userModificationDate, project, area, heading, (SELECT GROUP_CONCAT(tg.title, char(31)) FROM TMTaskTag tt JOIN TMTag tg ON tg.uuid = tt.tags WHERE tt.tasks = TMTask.uuid) AS tags_csv FROM TMTask WHERE status = 0 AND todayIndex IS NOT NULL AND todayIndex != 0 AND trashed = 0 ORDER BY todayIndex ASC LIMIT {limit}"
            )
        } else {
            "SELECT uuid, title, type, status, notes, startDate, deadline, stopDate, creationDate, userModificationDate, project, area, heading, (SELECT GROUP_CONCAT(tg.title, char(31)) FROM TMTaskTag tt JOIN TMTag tg ON tg.uuid = tt.tags WHERE tt.tasks = TMTask.uuid) AS tags_csv FROM TMTask WHERE status = 0 AND todayIndex IS NOT NULL AND todayIndex != 0 AND trashed = 0 ORDER BY todayIndex ASC".to_string()
        };

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ThingsError::unknown(format!("Failed to fetch today's tasks: {e}")))?;

        let tasks = rows
            .iter()
            .map(map_task_row)
            .collect::<ThingsResult<Vec<Task>>>()?;

        Ok(tasks)
    }

    /// Get a task by its UUID
    ///
    /// # Errors
    ///
    /// Returns an error if the task does not exist or if the database query fails
    #[instrument(skip(self))]
    pub async fn get_task_by_uuid(&self, id: &ThingsId) -> ThingsResult<Option<Task>> {
        let row = sqlx::query(
            r"
            SELECT
                uuid, title, status, type,
                startDate, deadline, stopDate,
                project, area, heading,
                notes, (SELECT GROUP_CONCAT(tg.title, char(31))
                          FROM TMTaskTag tt
                          JOIN TMTag tg ON tg.uuid = tt.tags
                         WHERE tt.tasks = TMTask.uuid) AS tags_csv,
                creationDate, userModificationDate,
                trashed
            FROM TMTask
            WHERE uuid = ?
            ",
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ThingsError::unknown(format!("Failed to fetch task: {e}")))?;

        if let Some(row) = row {
            // Check if trashed
            let trashed: i64 = row.get("trashed");
            if trashed == 1 {
                return Ok(None); // Return None for trashed tasks
            }

            // Use the centralized mapper
            let task = map_task_row(&row)?;
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }
}
