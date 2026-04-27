//! Query builder for filtering and searching tasks

use crate::models::{TaskFilters, TaskStatus, TaskType};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use uuid::Uuid;

/// Builder for constructing task queries with filters
#[derive(Debug, Clone)]
pub struct TaskQueryBuilder {
    filters: TaskFilters,
    /// OR-semantics / exclusion / count tag filters applied by `execute()` in Rust.
    /// Kept off `TaskFilters` to preserve the stable public struct surface.
    /// Only present in `advanced-queries` builds (same gate as `execute()`).
    #[cfg(feature = "advanced-queries")]
    any_tags: Option<Vec<String>>,
    #[cfg(feature = "advanced-queries")]
    exclude_tags: Option<Vec<String>>,
    #[cfg(feature = "advanced-queries")]
    tag_count_min: Option<usize>,
    #[cfg(feature = "advanced-queries")]
    fuzzy_query: Option<String>,
    #[cfg(feature = "advanced-queries")]
    fuzzy_threshold: Option<f32>,
    #[cfg(feature = "advanced-queries")]
    where_expr: Option<crate::filter_expr::FilterExpr>,
    /// Cursor for keyset pagination via `execute_paged`. Stored on the builder
    /// because `TaskFilters` is frozen public API.
    #[cfg(feature = "batch-operations")]
    after: Option<crate::cursor::Cursor>,
}

impl TaskQueryBuilder {
    /// Create a new query builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            filters: TaskFilters::default(),
            #[cfg(feature = "advanced-queries")]
            any_tags: None,
            #[cfg(feature = "advanced-queries")]
            exclude_tags: None,
            #[cfg(feature = "advanced-queries")]
            tag_count_min: None,
            #[cfg(feature = "advanced-queries")]
            fuzzy_query: None,
            #[cfg(feature = "advanced-queries")]
            fuzzy_threshold: None,
            #[cfg(feature = "advanced-queries")]
            where_expr: None,
            #[cfg(feature = "batch-operations")]
            after: None,
        }
    }

    /// Filter by status
    #[must_use]
    pub const fn status(mut self, status: TaskStatus) -> Self {
        self.filters.status = Some(status);
        self
    }

    /// Filter by task type
    #[must_use]
    pub const fn task_type(mut self, task_type: TaskType) -> Self {
        self.filters.task_type = Some(task_type);
        self
    }

    /// Filter by project UUID
    #[must_use]
    pub const fn project_uuid(mut self, project_uuid: Uuid) -> Self {
        self.filters.project_uuid = Some(project_uuid);
        self
    }

    /// Filter by area UUID
    #[must_use]
    pub const fn area_uuid(mut self, area_uuid: Uuid) -> Self {
        self.filters.area_uuid = Some(area_uuid);
        self
    }

    /// Filter by tags (AND semantics — task must contain every listed tag).
    #[must_use]
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.filters.tags = Some(tags);
        self
    }

    /// Filter to tasks containing **any** of these tags (OR semantics).
    ///
    /// Composes with `.tags()` (AND) and `.exclude_tags()` (NOT) — all
    /// active tag filters must be satisfied. Applied in Rust by `execute()`;
    /// not reflected in `build()`.
    ///
    /// Requires the `advanced-queries` feature flag.
    #[cfg(feature = "advanced-queries")]
    #[must_use]
    pub fn any_tags(mut self, tags: Vec<String>) -> Self {
        self.any_tags = Some(tags);
        self
    }

    /// Filter out tasks containing any of these tags. Applied in Rust by `execute()`;
    /// not reflected in `build()`.
    ///
    /// Requires the `advanced-queries` feature flag.
    #[cfg(feature = "advanced-queries")]
    #[must_use]
    pub fn exclude_tags(mut self, tags: Vec<String>) -> Self {
        self.exclude_tags = Some(tags);
        self
    }

    /// Filter to tasks with at least `min` tags total. Applied in Rust by `execute()`;
    /// not reflected in `build()`.
    ///
    /// Requires the `advanced-queries` feature flag.
    #[cfg(feature = "advanced-queries")]
    #[must_use]
    pub fn tag_count(mut self, min: usize) -> Self {
        self.tag_count_min = Some(min);
        self
    }

    /// Filter and rank tasks by fuzzy similarity to `query` (title and notes).
    ///
    /// Scores are computed with windowed Levenshtein; only tasks meeting the
    /// threshold (default `0.6`, tunable via `fuzzy_threshold`) are returned.
    /// If `.search()` is also set, fuzzy wins and a warning is logged.
    ///
    /// Applied in Rust by `execute()` / `execute_ranked()`; not reflected in `build()`.
    ///
    /// Requires the `advanced-queries` feature flag.
    #[cfg(feature = "advanced-queries")]
    #[must_use]
    pub fn fuzzy_search(mut self, query: &str) -> Self {
        self.fuzzy_query = Some(query.to_string());
        self
    }

    /// Override the minimum fuzzy-match score threshold (clamped to `[0.0, 1.0]`).
    /// Defaults to `0.6` when not called.
    ///
    /// Requires the `advanced-queries` feature flag.
    #[cfg(feature = "advanced-queries")]
    #[must_use]
    pub fn fuzzy_threshold(mut self, threshold: f32) -> Self {
        self.fuzzy_threshold = Some(threshold.clamp(0.0, 1.0));
        self
    }

    /// Apply a boolean expression tree as an additional filter.
    ///
    /// The expression composes via AND with the SQL pre-fetch (`TaskFilters`)
    /// and with the other post-filters (`any_tags`, `exclude_tags`,
    /// `tag_count`, fuzzy search). To express disjunction or negation across
    /// statuses or types, leave `filters.status` / `filters.task_type` unset
    /// and put the OR/NOT branches inside `expr` instead.
    ///
    /// Evaluated in Rust by `execute()` after the database returns rows; not
    /// reflected in `build()`.
    ///
    /// Requires the `advanced-queries` feature flag.
    #[cfg(feature = "advanced-queries")]
    #[must_use]
    pub fn where_expr(mut self, expr: crate::filter_expr::FilterExpr) -> Self {
        self.where_expr = Some(expr);
        self
    }

    /// Continue cursor-based pagination from a previously-returned [`crate::cursor::Cursor`].
    ///
    /// The cursor identifies the last task delivered on the previous page;
    /// [`Self::execute_paged`] will return tasks strictly after it in the
    /// `(creationDate DESC, uuid DESC)` ordering. Mutually exclusive with
    /// `.offset(...)` — `execute_paged` will return [`crate::error::ThingsError::InvalidCursor`]
    /// if both are set.
    ///
    /// Requires the `batch-operations` feature flag.
    #[cfg(feature = "batch-operations")]
    #[must_use]
    pub fn after(mut self, cursor: crate::cursor::Cursor) -> Self {
        self.after = Some(cursor);
        self
    }

    /// Filter by start date range
    #[must_use]
    pub const fn start_date_range(
        mut self,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Self {
        self.filters.start_date_from = from;
        self.filters.start_date_to = to;
        self
    }

    /// Filter by deadline range
    #[must_use]
    pub const fn deadline_range(mut self, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Self {
        self.filters.deadline_from = from;
        self.filters.deadline_to = to;
        self
    }

    /// Add search query
    #[must_use]
    pub fn search(mut self, query: &str) -> Self {
        self.filters.search_query = Some(query.to_string());
        self
    }

    /// Set limit
    #[must_use]
    pub const fn limit(mut self, limit: usize) -> Self {
        self.filters.limit = Some(limit);
        self
    }

    /// Set offset for pagination
    #[must_use]
    pub const fn offset(mut self, offset: usize) -> Self {
        self.filters.offset = Some(offset);
        self
    }

    /// Filter to tasks whose deadline is today.
    #[must_use]
    pub fn due_today(self) -> Self {
        let today = today();
        self.deadline_range(Some(today), Some(today))
    }

    /// Filter to tasks whose deadline falls between today and the upcoming Sunday
    /// (Monday-Sunday week).
    #[must_use]
    pub fn due_this_week(self) -> Self {
        let today = today();
        self.deadline_range(Some(today), Some(end_of_week(today)))
    }

    /// Filter to tasks whose deadline falls in next calendar week (next Monday
    /// through Sunday, Monday-Sunday week).
    #[must_use]
    pub fn due_next_week(self) -> Self {
        let today = today();
        let next_monday = end_of_week(today) + Duration::days(1);
        self.deadline_range(Some(next_monday), Some(end_of_week(next_monday)))
    }

    /// Filter to tasks whose deadline is between today and `days` days from now (inclusive).
    #[must_use]
    pub fn due_in(self, days: i64) -> Self {
        let today = today();
        self.deadline_range(Some(today), Some(today + Duration::days(days)))
    }

    /// Filter to overdue tasks: deadline strictly before today.
    ///
    /// If no `status` filter has already been set, this also restricts results
    /// to incomplete tasks (a completed task isn't meaningfully overdue). An
    /// explicit `.status(...)` call before this helper is preserved.
    #[must_use]
    pub fn overdue(mut self) -> Self {
        let yesterday = today() - Duration::days(1);
        self.filters.deadline_from = None;
        self.filters.deadline_to = Some(yesterday);
        if self.filters.status.is_none() {
            self.filters.status = Some(TaskStatus::Incomplete);
        }
        self
    }

    /// Filter to tasks with a start date of today.
    #[must_use]
    pub fn starting_today(self) -> Self {
        let today = today();
        self.start_date_range(Some(today), Some(today))
    }

    /// Filter to tasks with a start date between today and the upcoming Sunday
    /// (Monday-Sunday week).
    #[must_use]
    pub fn starting_this_week(self) -> Self {
        let today = today();
        self.start_date_range(Some(today), Some(end_of_week(today)))
    }

    /// Build the final filters
    #[must_use]
    pub fn build(self) -> TaskFilters {
        self.filters
    }

    /// Execute the query against a live database connection.
    ///
    /// SQL-level filters and the `tags` AND-filter are handled by `query_tasks`.
    /// Builder-only predicates (`any_tags`, `exclude_tags`, `tag_count`,
    /// `fuzzy_search`) are applied in Rust afterward; when any are active,
    /// `limit`/`offset` pagination is deferred to Rust so pages count only
    /// matching rows. When `fuzzy_search` is set, this delegates to
    /// `execute_ranked` and strips scores.
    ///
    /// Requires the `advanced-queries` feature flag.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or task data cannot be mapped.
    #[cfg(feature = "advanced-queries")]
    pub async fn execute(
        &self,
        db: &crate::database::ThingsDatabase,
    ) -> crate::error::Result<Vec<crate::models::Task>> {
        if self.fuzzy_query.is_some() {
            return self
                .execute_ranked(db)
                .await
                .map(|ranked| ranked.into_iter().map(|r| r.task).collect());
        }

        let has_tag_post_filters = self.any_tags.as_ref().is_some_and(|t| !t.is_empty())
            || self.exclude_tags.as_ref().is_some_and(|t| !t.is_empty())
            || self.tag_count_min.is_some();
        let has_where_expr = self.where_expr.is_some();

        if !has_tag_post_filters && !has_where_expr {
            return db.query_tasks(&self.filters).await;
        }

        let mut filters_no_page = self.filters.clone();
        let limit = filters_no_page.limit.take();
        let offset = filters_no_page.offset.take();

        let tasks = db.query_tasks(&filters_no_page).await?;
        let mut tasks = Self::apply_tag_filters(
            tasks,
            self.any_tags.as_deref(),
            self.exclude_tags.as_deref(),
            self.tag_count_min,
        );

        if let Some(expr) = &self.where_expr {
            tasks.retain(|task| expr.matches(task));
        }

        let offset = offset.unwrap_or(0);
        tasks = tasks.into_iter().skip(offset).collect();
        if let Some(limit) = limit {
            tasks.truncate(limit);
        }

        Ok(tasks)
    }

    #[cfg(feature = "advanced-queries")]
    fn apply_tag_filters(
        mut tasks: Vec<crate::models::Task>,
        any_tags: Option<&[String]>,
        exclude_tags: Option<&[String]>,
        tag_count_min: Option<usize>,
    ) -> Vec<crate::models::Task> {
        if let Some(any) = any_tags {
            if !any.is_empty() {
                tasks.retain(|task| any.iter().any(|f| task.tags.contains(f)));
            }
        }
        if let Some(excl) = exclude_tags {
            if !excl.is_empty() {
                tasks.retain(|task| !excl.iter().any(|f| task.tags.contains(f)));
            }
        }
        if let Some(min) = tag_count_min {
            tasks.retain(|task| task.tags.len() >= min);
        }
        tasks
    }

    /// Execute the query and return one page of results plus an optional
    /// cursor for the next page.
    ///
    /// Pagination is keyset-based: the cursor encodes `(created, uuid)` of the
    /// last-returned task and the next page admits only rows strictly older in
    /// the canonical `(creationDate DESC, uuid DESC)` ordering. Unlike
    /// `offset`, page boundaries are stable when underlying data changes
    /// between fetches.
    ///
    /// Page size is `self.filters.limit` if set, otherwise `100`. The returned
    /// `Page::next_cursor` is `Some` only if the page is full (i.e. there may
    /// be more rows); the last page always has `next_cursor: None`.
    ///
    /// Requires both the `advanced-queries` and `batch-operations` feature
    /// flags (cursor pagination is built on top of `query_tasks`).
    ///
    /// # Errors
    ///
    /// - [`crate::error::ThingsError::InvalidCursor`] if `.offset(...)` and
    ///   `.after(...)` are both set, or if `.fuzzy_search(...)` and
    ///   `.after(...)` are both set, or if the cursor itself is malformed.
    /// - Any error returned by [`crate::database::ThingsDatabase::query_tasks`].
    #[cfg(all(feature = "advanced-queries", feature = "batch-operations"))]
    pub async fn execute_paged(
        &self,
        db: &crate::database::ThingsDatabase,
    ) -> crate::error::Result<crate::cursor::Page<crate::models::Task>> {
        if self.fuzzy_query.is_some() {
            return Err(crate::error::ThingsError::InvalidCursor(
                "execute_paged and execute_stream do not support fuzzy_search; use execute_ranked instead".to_string(),
            ));
        }
        if self.filters.offset.is_some() && self.after.is_some() {
            return Err(crate::error::ThingsError::InvalidCursor(
                "offset and after are mutually exclusive".to_string(),
            ));
        }

        let after_payload = self
            .after
            .as_ref()
            .map(crate::cursor::Cursor::decode)
            .transpose()?;
        let after_anchor = after_payload.as_ref().map(|p| (p.c.timestamp(), p.u));

        let page_size = self.filters.limit.unwrap_or(DEFAULT_PAGE_SIZE);

        // Build a TaskFilters with the page size applied at the SQL layer when
        // there are no post-filters; with post-filters, defer to Rust below.
        let has_tag_post_filters = self.any_tags.as_ref().is_some_and(|t| !t.is_empty())
            || self.exclude_tags.as_ref().is_some_and(|t| !t.is_empty())
            || self.tag_count_min.is_some();
        let has_where_expr = self.where_expr.is_some();
        let has_post_filters = has_tag_post_filters || has_where_expr;

        let mut filters = self.filters.clone();
        filters.offset = None;
        if has_post_filters {
            // SQL fetches everything; Rust does the slicing.
            filters.limit = None;
        } else {
            filters.limit = Some(page_size);
        }

        let mut tasks = db.query_tasks_inner(&filters, after_anchor).await?;

        if has_tag_post_filters {
            tasks = Self::apply_tag_filters(
                tasks,
                self.any_tags.as_deref(),
                self.exclude_tags.as_deref(),
                self.tag_count_min,
            );
        }
        if let Some(expr) = &self.where_expr {
            tasks.retain(|task| expr.matches(task));
        }
        if has_post_filters {
            tasks.truncate(page_size);
        }

        let next_cursor = if tasks.len() == page_size {
            tasks
                .last()
                .map(|last| {
                    let payload = crate::cursor::CursorPayload {
                        c: last.created,
                        u: last.uuid,
                    };
                    crate::cursor::Cursor::encode(&payload)
                })
                .transpose()?
        } else {
            None
        };

        Ok(crate::cursor::Page {
            items: tasks,
            next_cursor,
        })
    }

    /// Execute the query as a [`futures_core::Stream`] of tasks, internally
    /// chunked via cursor pagination.
    ///
    /// Yields tasks one at a time in `(creationDate DESC, uuid DESC)` order,
    /// transparently fetching the next page when the current one is exhausted.
    /// The stream completes when the underlying query has no more rows.
    ///
    /// `self.filters.limit` (overridable via [`limit`](Self::limit)) sets the
    /// **chunk size** in this context, not a cap on total emitted items —
    /// defaults to `100` if unset. Pre-filters and post-filters
    /// (`status`, `any_tags`, `where_expr`, etc.) compose with streaming
    /// exactly as they do with [`execute_paged`](Self::execute_paged).
    ///
    /// The first item is `Err(ThingsError)` if the underlying `execute_paged`
    /// call rejects the query (e.g. `.offset()` and `.after()` both set, or
    /// `.fuzzy_search()` and `.after()` both set). After any error, the
    /// stream terminates.
    ///
    /// Requires both the `advanced-queries` and `batch-operations` feature
    /// flags.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use futures_util::StreamExt;
    /// let mut stream = TaskQueryBuilder::new()
    ///     .status(TaskStatus::Incomplete)
    ///     .limit(50) // chunk size, not total cap
    ///     .execute_stream(&db);
    /// while let Some(task) = stream.next().await {
    ///     let task = task?;
    ///     // process task
    /// }
    /// # Ok::<(), things3_core::ThingsError>(())
    /// ```
    #[cfg(all(feature = "advanced-queries", feature = "batch-operations"))]
    pub fn execute_stream<'a>(
        mut self,
        db: &'a crate::database::ThingsDatabase,
    ) -> std::pin::Pin<
        Box<dyn futures_core::Stream<Item = crate::error::Result<crate::models::Task>> + Send + 'a>,
    >
    where
        Self: Send + 'a,
    {
        Box::pin(async_stream::try_stream! {
            loop {
                let page = self.execute_paged(db).await?;
                let next = page.next_cursor;
                for task in page.items {
                    yield task;
                }
                match next {
                    Some(c) => self.after = Some(c),
                    None => break,
                }
            }
        })
    }

    /// Execute the query and return tasks paired with their fuzzy-match scores,
    /// sorted by score descending (ties broken by UUID for determinism).
    ///
    /// Requires `.fuzzy_search(query)` to be set — returns a validation error
    /// otherwise (it is a programming error to ask for ranked results with no
    /// fuzzy predicate).
    ///
    /// Requires the `advanced-queries` feature flag.
    ///
    /// # Errors
    ///
    /// Returns an error if `fuzzy_search` is not set, or if the database query fails.
    #[cfg(feature = "advanced-queries")]
    pub async fn execute_ranked(
        &self,
        db: &crate::database::ThingsDatabase,
    ) -> crate::error::Result<Vec<crate::models::RankedTask>> {
        let query = self.fuzzy_query.as_deref().ok_or_else(|| {
            crate::error::ThingsError::validation(
                "execute_ranked requires fuzzy_search() to be set",
            )
        })?;

        let query_lc = query.to_lowercase();
        let threshold = self.fuzzy_threshold.unwrap_or(DEFAULT_FUZZY_THRESHOLD);

        let mut filters_no_page = self.filters.clone();
        let limit = filters_no_page.limit.take();
        let offset = filters_no_page.offset.take();

        if filters_no_page.search_query.is_some() {
            tracing::warn!(
                "fuzzy_search and search both set; fuzzy takes precedence, ignoring substring search"
            );
            filters_no_page.search_query = None;
        }

        let tasks = db.query_tasks(&filters_no_page).await?;
        let mut tasks = Self::apply_tag_filters(
            tasks,
            self.any_tags.as_deref(),
            self.exclude_tags.as_deref(),
            self.tag_count_min,
        );

        if let Some(expr) = &self.where_expr {
            tasks.retain(|task| expr.matches(task));
        }

        let mut scored: Vec<crate::models::RankedTask> = tasks
            .into_iter()
            .filter_map(|task| {
                let score = task_fuzzy_score(&query_lc, &task);
                if score >= threshold {
                    Some(crate::models::RankedTask { task, score })
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.task.uuid.cmp(&b.task.uuid))
        });

        let offset = offset.unwrap_or(0);
        scored = scored.into_iter().skip(offset).collect();
        if let Some(limit) = limit {
            scored.truncate(limit);
        }

        Ok(scored)
    }

    /// Snapshot the full builder state — both [`TaskFilters`] and the
    /// builder-only post-1.0.0 predicates — into a [`crate::saved_queries::SavedQuery`].
    ///
    /// Combine with [`crate::saved_queries::SavedQueryStore`] to persist queries
    /// to disk and replay them later via [`Self::from_saved_query`].
    ///
    /// Requires the `advanced-queries` feature flag.
    #[cfg(feature = "advanced-queries")]
    #[must_use]
    pub fn to_saved_query(&self, name: impl Into<String>) -> crate::saved_queries::SavedQuery {
        crate::saved_queries::SavedQuery {
            name: name.into(),
            description: None,
            filters: self.filters.clone(),
            any_tags: self.any_tags.clone(),
            exclude_tags: self.exclude_tags.clone(),
            tag_count_min: self.tag_count_min,
            fuzzy_query: self.fuzzy_query.clone(),
            fuzzy_threshold: self.fuzzy_threshold,
            where_expr: self.where_expr.clone(),
            saved_at: chrono::Utc::now(),
        }
    }

    /// Reconstruct a builder from a previously-saved [`crate::saved_queries::SavedQuery`].
    /// The returned builder can be executed directly via [`Self::execute`] or
    /// [`Self::execute_ranked`].
    ///
    /// Requires the `advanced-queries` feature flag.
    #[cfg(feature = "advanced-queries")]
    #[must_use]
    pub fn from_saved_query(query: &crate::saved_queries::SavedQuery) -> Self {
        Self {
            filters: query.filters.clone(),
            any_tags: query.any_tags.clone(),
            exclude_tags: query.exclude_tags.clone(),
            tag_count_min: query.tag_count_min,
            fuzzy_query: query.fuzzy_query.clone(),
            fuzzy_threshold: query.fuzzy_threshold.map(|t| t.clamp(0.0, 1.0)),
            where_expr: query.where_expr.clone(),
            // Cursors are ephemeral and not part of saved-query state.
            #[cfg(feature = "batch-operations")]
            after: None,
        }
    }
}

impl Default for TaskQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "advanced-queries")]
const DEFAULT_FUZZY_THRESHOLD: f32 = 0.6;

#[cfg(all(feature = "advanced-queries", feature = "batch-operations"))]
const DEFAULT_PAGE_SIZE: usize = 100;

#[cfg(feature = "advanced-queries")]
fn task_fuzzy_score(query_lc: &str, task: &crate::models::Task) -> f32 {
    let title_score = fuzzy_field_score(query_lc, &task.title);
    let notes_score = task
        .notes
        .as_deref()
        .map(|n| fuzzy_field_score(query_lc, n))
        .unwrap_or(0.0);
    title_score.max(notes_score)
}

#[cfg(feature = "advanced-queries")]
fn fuzzy_field_score(query_lc: &str, field: &str) -> f32 {
    let field_lc = field.to_lowercase();
    if !query_lc.is_empty() && field_lc.contains(query_lc) {
        return 1.0;
    }
    best_window_score(query_lc, &field_lc)
}

#[cfg(feature = "advanced-queries")]
fn best_window_score(query: &str, field: &str) -> f32 {
    if query.is_empty() || field.is_empty() {
        return 0.0;
    }
    let window_len = (2 * query.len()).min(field.len());
    let step = 1;
    let chars: Vec<char> = field.chars().collect();
    let n = chars.len();
    let mut best = 0.0f32;
    let mut i = 0;
    loop {
        let end = (i + window_len).min(n);
        let slice: String = chars[i..end].iter().collect();
        let score = strsim::normalized_levenshtein(query, &slice) as f32;
        if score > best {
            best = score;
        }
        if end >= n {
            break;
        }
        i += step;
    }
    best
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

fn end_of_week(d: NaiveDate) -> NaiveDate {
    let days_from_monday = i64::from(d.weekday().num_days_from_monday());
    d + Duration::days(6 - days_from_monday)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use uuid::Uuid;

    #[test]
    fn test_task_query_builder_new() {
        let builder = TaskQueryBuilder::new();
        let filters = builder.build();

        assert!(filters.status.is_none());
        assert!(filters.task_type.is_none());
        assert!(filters.project_uuid.is_none());
        assert!(filters.area_uuid.is_none());
        assert!(filters.tags.is_none());
        assert!(filters.start_date_from.is_none());
        assert!(filters.start_date_to.is_none());
        assert!(filters.deadline_from.is_none());
        assert!(filters.deadline_to.is_none());
        assert!(filters.search_query.is_none());
        assert!(filters.limit.is_none());
        assert!(filters.offset.is_none());
    }

    #[test]
    fn test_task_query_builder_default() {
        let builder = TaskQueryBuilder::default();
        let filters = builder.build();

        assert!(filters.status.is_none());
        assert!(filters.task_type.is_none());
    }

    #[test]
    fn test_task_query_builder_status() {
        let builder = TaskQueryBuilder::new().status(TaskStatus::Completed);
        let filters = builder.build();

        assert_eq!(filters.status, Some(TaskStatus::Completed));
    }

    #[test]
    fn test_task_query_builder_task_type() {
        let builder = TaskQueryBuilder::new().task_type(TaskType::Project);
        let filters = builder.build();

        assert_eq!(filters.task_type, Some(TaskType::Project));
    }

    #[test]
    fn test_task_query_builder_project_uuid() {
        let uuid = Uuid::new_v4();
        let builder = TaskQueryBuilder::new().project_uuid(uuid);
        let filters = builder.build();

        assert_eq!(filters.project_uuid, Some(uuid));
    }

    #[test]
    fn test_task_query_builder_area_uuid() {
        let uuid = Uuid::new_v4();
        let builder = TaskQueryBuilder::new().area_uuid(uuid);
        let filters = builder.build();

        assert_eq!(filters.area_uuid, Some(uuid));
    }

    #[test]
    fn test_task_query_builder_tags() {
        let tags = vec!["urgent".to_string(), "important".to_string()];
        let builder = TaskQueryBuilder::new().tags(tags.clone());
        let filters = builder.build();

        assert_eq!(filters.tags, Some(tags));
    }

    #[cfg(feature = "advanced-queries")]
    #[test]
    fn test_task_query_builder_any_tags() {
        let tags = vec!["a".to_string(), "b".to_string()];
        let builder = TaskQueryBuilder::new().any_tags(tags.clone());
        assert_eq!(builder.any_tags, Some(tags));
    }

    #[cfg(feature = "advanced-queries")]
    #[test]
    fn test_task_query_builder_exclude_tags() {
        let tags = vec!["archived".to_string()];
        let builder = TaskQueryBuilder::new().exclude_tags(tags.clone());
        assert_eq!(builder.exclude_tags, Some(tags));
    }

    #[cfg(feature = "advanced-queries")]
    #[test]
    fn test_task_query_builder_tag_count() {
        let builder = TaskQueryBuilder::new().tag_count(2);
        assert_eq!(builder.tag_count_min, Some(2));
    }

    #[cfg(feature = "advanced-queries")]
    #[test]
    fn test_task_query_builder_where_expr_setter() {
        use crate::filter_expr::FilterExpr;
        let expr = FilterExpr::status(TaskStatus::Incomplete);
        let builder = TaskQueryBuilder::new().where_expr(expr.clone());
        assert_eq!(builder.where_expr, Some(expr));
    }

    #[cfg(feature = "batch-operations")]
    mod cursor_builder_tests {
        use super::*;
        use crate::cursor::{Cursor, CursorPayload};
        use chrono::Utc;
        use uuid::Uuid;

        fn sample_cursor() -> Cursor {
            Cursor::encode(&CursorPayload {
                c: Utc::now(),
                u: Uuid::new_v4(),
            })
            .unwrap()
        }

        #[test]
        fn test_after_setter_stores_cursor() {
            let c = sample_cursor();
            let builder = TaskQueryBuilder::new().after(c.clone());
            assert_eq!(builder.after, Some(c));
        }

        #[cfg(feature = "advanced-queries")]
        #[tokio::test]
        async fn test_execute_paged_rejects_offset_and_after() {
            use tempfile::NamedTempFile;
            let f = NamedTempFile::new().unwrap();
            crate::test_utils::create_test_database(f.path())
                .await
                .unwrap();
            let db = crate::database::ThingsDatabase::new(f.path())
                .await
                .unwrap();

            let result = TaskQueryBuilder::new()
                .offset(5)
                .after(sample_cursor())
                .execute_paged(&db)
                .await;
            match result {
                Err(crate::error::ThingsError::InvalidCursor(msg)) => {
                    assert!(msg.contains("offset and after"), "msg: {msg}");
                }
                other => panic!("expected InvalidCursor, got {other:?}"),
            }
        }

        #[cfg(feature = "advanced-queries")]
        #[tokio::test]
        async fn test_execute_paged_rejects_fuzzy_search() {
            use tempfile::NamedTempFile;
            let f = NamedTempFile::new().unwrap();
            crate::test_utils::create_test_database(f.path())
                .await
                .unwrap();
            let db = crate::database::ThingsDatabase::new(f.path())
                .await
                .unwrap();

            // fuzzy_search alone must be rejected — no .after() needed to trigger.
            let result = TaskQueryBuilder::new()
                .fuzzy_search("anything")
                .execute_paged(&db)
                .await;
            match result {
                Err(crate::error::ThingsError::InvalidCursor(msg)) => {
                    assert!(msg.contains("fuzzy"), "msg: {msg}");
                }
                other => panic!("expected InvalidCursor, got {other:?}"),
            }
        }
    }

    #[cfg(feature = "advanced-queries")]
    #[test]
    fn test_task_query_builder_chaining_tag_methods() {
        let builder = TaskQueryBuilder::new()
            .tags(vec!["a".to_string()])
            .any_tags(vec!["b".to_string(), "c".to_string()])
            .exclude_tags(vec!["d".to_string()])
            .tag_count(1);
        assert_eq!(builder.filters.tags, Some(vec!["a".to_string()]));
        assert_eq!(
            builder.any_tags,
            Some(vec!["b".to_string(), "c".to_string()])
        );
        assert_eq!(builder.exclude_tags, Some(vec!["d".to_string()]));
        assert_eq!(builder.tag_count_min, Some(1));
    }

    #[cfg(feature = "advanced-queries")]
    #[test]
    fn test_fuzzy_search_sets_field() {
        let builder = TaskQueryBuilder::new().fuzzy_search("meeting");
        assert_eq!(builder.fuzzy_query, Some("meeting".to_string()));
    }

    #[cfg(feature = "advanced-queries")]
    #[test]
    fn test_fuzzy_threshold_clamps_low() {
        let builder = TaskQueryBuilder::new().fuzzy_threshold(-0.5);
        assert_eq!(builder.fuzzy_threshold, Some(0.0));
    }

    #[cfg(feature = "advanced-queries")]
    #[test]
    fn test_fuzzy_threshold_clamps_high() {
        let builder = TaskQueryBuilder::new().fuzzy_threshold(1.5);
        assert_eq!(builder.fuzzy_threshold, Some(1.0));
    }

    #[cfg(feature = "advanced-queries")]
    #[test]
    fn test_fuzzy_search_chains_with_other_filters() {
        let builder = TaskQueryBuilder::new()
            .status(TaskStatus::Incomplete)
            .fuzzy_search("agenda")
            .fuzzy_threshold(0.7);
        assert_eq!(builder.fuzzy_query, Some("agenda".to_string()));
        assert_eq!(builder.fuzzy_threshold, Some(0.7));
        assert_eq!(builder.filters.status, Some(TaskStatus::Incomplete));
    }

    #[cfg(feature = "advanced-queries")]
    mod fuzzy_score_tests {
        use super::*;

        #[test]
        fn test_fuzzy_score_substring_short_circuit() {
            assert_eq!(fuzzy_field_score("foo", "blah foo bar"), 1.0);
        }

        #[test]
        fn test_fuzzy_score_typo_above_threshold() {
            let score = fuzzy_field_score("urgent", "urgnt");
            assert!(score >= 0.6, "expected score >= 0.6, got {score}");
        }

        #[test]
        fn test_best_window_score_long_field() {
            let long_field = "alexander needs to buy eggs and milk from the store today";
            let whole = strsim::normalized_levenshtein("alex", long_field) as f32;
            let windowed = best_window_score("alex", long_field);
            assert!(
                windowed > whole,
                "windowed ({windowed}) should beat whole-field ({whole})"
            );
        }

        #[test]
        fn test_task_fuzzy_score_uses_max_of_title_notes() {
            use chrono::Utc;
            use uuid::Uuid;
            let task = crate::models::Task {
                uuid: Uuid::new_v4(),
                title: "unrelated title xyz".to_string(),
                notes: Some("meeting agenda important".to_string()),
                task_type: crate::models::TaskType::Todo,
                status: crate::models::TaskStatus::Incomplete,
                start_date: None,
                deadline: None,
                created: Utc::now(),
                modified: Utc::now(),
                stop_date: None,
                project_uuid: None,
                area_uuid: None,
                parent_uuid: None,
                tags: vec![],
                children: vec![],
            };
            let score = task_fuzzy_score("agenda", &task);
            assert_eq!(score, 1.0, "notes contains 'agenda', score should be 1.0");
        }
    }

    #[cfg(feature = "advanced-queries")]
    mod saved_query_conversion_tests {
        use super::*;

        #[test]
        fn test_to_saved_query_captures_all_state() {
            let from = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
            let to = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
            let project = Uuid::new_v4();

            let builder = TaskQueryBuilder::new()
                .status(TaskStatus::Incomplete)
                .task_type(TaskType::Todo)
                .project_uuid(project)
                .tags(vec!["work".to_string()])
                .any_tags(vec!["urgent".to_string(), "p0".to_string()])
                .exclude_tags(vec!["archived".to_string()])
                .tag_count(2)
                .fuzzy_search("budget")
                .fuzzy_threshold(0.75)
                .start_date_range(Some(from), Some(to))
                .limit(10)
                .offset(5);

            let saved = builder.to_saved_query("everything");
            assert_eq!(saved.name, "everything");
            assert_eq!(saved.filters.status, Some(TaskStatus::Incomplete));
            assert_eq!(saved.filters.task_type, Some(TaskType::Todo));
            assert_eq!(saved.filters.project_uuid, Some(project));
            assert_eq!(saved.filters.tags, Some(vec!["work".to_string()]));
            assert_eq!(saved.filters.start_date_from, Some(from));
            assert_eq!(saved.filters.limit, Some(10));
            assert_eq!(saved.filters.offset, Some(5));
            assert_eq!(
                saved.any_tags,
                Some(vec!["urgent".to_string(), "p0".to_string()])
            );
            assert_eq!(saved.exclude_tags, Some(vec!["archived".to_string()]));
            assert_eq!(saved.tag_count_min, Some(2));
            assert_eq!(saved.fuzzy_query, Some("budget".to_string()));
            assert_eq!(saved.fuzzy_threshold, Some(0.75));
            assert!(saved.where_expr.is_none());
        }

        #[test]
        fn test_to_saved_query_captures_where_expr() {
            use crate::filter_expr::FilterExpr;
            let expr = FilterExpr::status(TaskStatus::Incomplete)
                .or(FilterExpr::status(TaskStatus::Completed));
            let saved = TaskQueryBuilder::new()
                .where_expr(expr.clone())
                .to_saved_query("with-expr");
            assert_eq!(saved.where_expr, Some(expr));
        }

        #[test]
        fn test_from_saved_query_restores_all_state() {
            let original = TaskQueryBuilder::new()
                .status(TaskStatus::Completed)
                .any_tags(vec!["a".to_string()])
                .fuzzy_search("hello")
                .fuzzy_threshold(0.9)
                .limit(7);
            let saved = original.to_saved_query("test");
            let rebuilt = TaskQueryBuilder::from_saved_query(&saved);

            assert_eq!(rebuilt.filters.status, Some(TaskStatus::Completed));
            assert_eq!(rebuilt.filters.limit, Some(7));
            assert_eq!(rebuilt.any_tags, Some(vec!["a".to_string()]));
            assert_eq!(rebuilt.fuzzy_query, Some("hello".to_string()));
            assert_eq!(rebuilt.fuzzy_threshold, Some(0.9));
        }

        #[test]
        fn test_saved_query_roundtrip_through_json() {
            let original = TaskQueryBuilder::new()
                .status(TaskStatus::Incomplete)
                .any_tags(vec!["x".to_string()])
                .fuzzy_search("foo")
                .fuzzy_threshold(0.5);

            let saved = original.to_saved_query("rt");
            let json = serde_json::to_string(&saved).unwrap();
            let restored: crate::saved_queries::SavedQuery = serde_json::from_str(&json).unwrap();
            let rebuilt = TaskQueryBuilder::from_saved_query(&restored);

            assert_eq!(rebuilt.filters.status, Some(TaskStatus::Incomplete));
            assert_eq!(rebuilt.any_tags, Some(vec!["x".to_string()]));
            assert_eq!(rebuilt.fuzzy_query, Some("foo".to_string()));
            assert_eq!(rebuilt.fuzzy_threshold, Some(0.5));
        }

        #[test]
        fn test_from_saved_query_restores_where_expr_through_json() {
            use crate::filter_expr::FilterExpr;
            let expr = FilterExpr::Or(vec![
                FilterExpr::status(TaskStatus::Incomplete),
                FilterExpr::status(TaskStatus::Completed),
            ])
            .and(FilterExpr::task_type(TaskType::Project).not());

            let saved = TaskQueryBuilder::new()
                .where_expr(expr.clone())
                .to_saved_query("expr-rt");
            let json = serde_json::to_string(&saved).unwrap();
            let restored: crate::saved_queries::SavedQuery = serde_json::from_str(&json).unwrap();
            let rebuilt = TaskQueryBuilder::from_saved_query(&restored);
            assert_eq!(rebuilt.where_expr, Some(expr));
        }
    }

    #[test]
    fn test_task_query_builder_start_date_range() {
        let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let builder = TaskQueryBuilder::new().start_date_range(Some(from), Some(to));
        let filters = builder.build();

        assert_eq!(filters.start_date_from, Some(from));
        assert_eq!(filters.start_date_to, Some(to));
    }

    #[test]
    fn test_task_query_builder_deadline_range() {
        let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let builder = TaskQueryBuilder::new().deadline_range(Some(from), Some(to));
        let filters = builder.build();

        assert_eq!(filters.deadline_from, Some(from));
        assert_eq!(filters.deadline_to, Some(to));
    }

    #[test]
    fn test_task_query_builder_search() {
        let query = "test search";
        let builder = TaskQueryBuilder::new().search(query);
        let filters = builder.build();

        assert_eq!(filters.search_query, Some(query.to_string()));
    }

    #[test]
    fn test_task_query_builder_limit() {
        let builder = TaskQueryBuilder::new().limit(50);
        let filters = builder.build();

        assert_eq!(filters.limit, Some(50));
    }

    #[test]
    fn test_task_query_builder_offset() {
        let builder = TaskQueryBuilder::new().offset(10);
        let filters = builder.build();

        assert_eq!(filters.offset, Some(10));
    }

    #[cfg(feature = "advanced-queries")]
    mod execute_tests {
        use super::*;
        use tempfile::NamedTempFile;

        #[tokio::test]
        async fn test_execute_empty_builder() {
            let f = NamedTempFile::new().unwrap();
            crate::test_utils::create_test_database(f.path())
                .await
                .unwrap();
            let db = crate::database::ThingsDatabase::new(f.path())
                .await
                .unwrap();
            let result = TaskQueryBuilder::new().execute(&db).await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_execute_with_status_filter() {
            let f = NamedTempFile::new().unwrap();
            crate::test_utils::create_test_database(f.path())
                .await
                .unwrap();
            let db = crate::database::ThingsDatabase::new(f.path())
                .await
                .unwrap();
            let result = TaskQueryBuilder::new()
                .status(TaskStatus::Incomplete)
                .execute(&db)
                .await;
            assert!(result.is_ok());
            assert!(result
                .unwrap()
                .iter()
                .all(|t| t.status == TaskStatus::Incomplete));
        }
    }

    #[test]
    fn test_task_query_builder_chaining() {
        let uuid = Uuid::new_v4();
        let tags = vec!["urgent".to_string()];
        let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

        let builder = TaskQueryBuilder::new()
            .status(TaskStatus::Incomplete)
            .task_type(TaskType::Todo)
            .project_uuid(uuid)
            .tags(tags.clone())
            .start_date_range(Some(from), Some(to))
            .search("test")
            .limit(25)
            .offset(5);

        let filters = builder.build();

        assert_eq!(filters.status, Some(TaskStatus::Incomplete));
        assert_eq!(filters.task_type, Some(TaskType::Todo));
        assert_eq!(filters.project_uuid, Some(uuid));
        assert_eq!(filters.tags, Some(tags));
        assert_eq!(filters.start_date_from, Some(from));
        assert_eq!(filters.start_date_to, Some(to));
        assert_eq!(filters.search_query, Some("test".to_string()));
        assert_eq!(filters.limit, Some(25));
        assert_eq!(filters.offset, Some(5));
    }

    mod date_helper_tests {
        use super::*;

        #[test]
        fn test_due_today_sets_deadline_range_to_today() {
            let filters = TaskQueryBuilder::new().due_today().build();
            let today = today();
            assert_eq!(filters.deadline_from, Some(today));
            assert_eq!(filters.deadline_to, Some(today));
        }

        #[test]
        fn test_due_this_week_ends_on_sunday() {
            let filters = TaskQueryBuilder::new().due_this_week().build();
            let today = today();
            assert_eq!(filters.deadline_from, Some(today));
            let to = filters.deadline_to.unwrap();
            assert_eq!(to.weekday(), chrono::Weekday::Sun);
            assert!(to >= today);
        }

        #[test]
        fn test_due_next_week_spans_monday_to_sunday() {
            let filters = TaskQueryBuilder::new().due_next_week().build();
            let from = filters.deadline_from.unwrap();
            let to = filters.deadline_to.unwrap();
            assert_eq!(from.weekday(), chrono::Weekday::Mon);
            assert_eq!(to.weekday(), chrono::Weekday::Sun);
            assert_eq!(to - from, Duration::days(6));
            assert!(from > today());
        }

        #[test]
        fn test_due_in_n_days() {
            let filters = TaskQueryBuilder::new().due_in(7).build();
            let today = today();
            assert_eq!(filters.deadline_from, Some(today));
            assert_eq!(filters.deadline_to, Some(today + Duration::days(7)));
        }

        #[test]
        fn test_due_in_zero_days_is_today() {
            let filters = TaskQueryBuilder::new().due_in(0).build();
            let today = today();
            assert_eq!(filters.deadline_from, Some(today));
            assert_eq!(filters.deadline_to, Some(today));
        }

        #[test]
        fn test_overdue_sets_deadline_to_yesterday_with_no_lower_bound() {
            let filters = TaskQueryBuilder::new().overdue().build();
            let yesterday = today() - Duration::days(1);
            assert_eq!(filters.deadline_from, None);
            assert_eq!(filters.deadline_to, Some(yesterday));
        }

        #[test]
        fn test_overdue_implicitly_sets_status_incomplete_when_unset() {
            let filters = TaskQueryBuilder::new().overdue().build();
            assert_eq!(filters.status, Some(TaskStatus::Incomplete));
        }

        #[test]
        fn test_overdue_does_not_override_explicit_status() {
            let filters = TaskQueryBuilder::new()
                .status(TaskStatus::Canceled)
                .overdue()
                .build();
            assert_eq!(filters.status, Some(TaskStatus::Canceled));
        }

        #[test]
        fn test_starting_today_sets_start_date_range() {
            let filters = TaskQueryBuilder::new().starting_today().build();
            let today = today();
            assert_eq!(filters.start_date_from, Some(today));
            assert_eq!(filters.start_date_to, Some(today));
        }

        #[test]
        fn test_starting_this_week_ends_on_sunday() {
            let filters = TaskQueryBuilder::new().starting_this_week().build();
            let today = today();
            assert_eq!(filters.start_date_from, Some(today));
            let to = filters.start_date_to.unwrap();
            assert_eq!(to.weekday(), chrono::Weekday::Sun);
            assert!(to >= today);
        }

        #[test]
        fn test_end_of_week_on_monday_returns_following_sunday() {
            let monday = NaiveDate::from_ymd_opt(2026, 4, 27).unwrap();
            assert_eq!(monday.weekday(), chrono::Weekday::Mon);
            let eow = end_of_week(monday);
            assert_eq!(eow, NaiveDate::from_ymd_opt(2026, 5, 3).unwrap());
            assert_eq!(eow.weekday(), chrono::Weekday::Sun);
        }

        #[test]
        fn test_end_of_week_on_sunday_returns_same_day() {
            let sunday = NaiveDate::from_ymd_opt(2026, 5, 3).unwrap();
            assert_eq!(sunday.weekday(), chrono::Weekday::Sun);
            assert_eq!(end_of_week(sunday), sunday);
        }
    }
}
