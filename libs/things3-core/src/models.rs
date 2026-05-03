//! Data models for Things 3 entities

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ThingsError;

/// Identifier for any Things 3 entity (task, project, area, tag, heading).
///
/// Things 3 uses two distinct identifier formats in the wild:
///
/// 1. **Native Things IDs** — 21- or 22-character base62 strings the Things
///    app itself produces (e.g. `R4t2G8Q63aGZq4epMHNeCr`). These appear on
///    every entity created via the Things UI or via `osascript`.
/// 2. **RFC-4122 UUIDs** — 36-character hyphenated hex strings that the
///    `SqlxBackend` generates for entities created through rust-things3
///    (e.g. `9d3f1e44-5c2a-4b8e-9c1f-7e2d8a4b3c5e`).
///
/// Both formats coexist in the same SQLite `uuid` column. This type stores
/// whichever format the entity was created with — never lossy conversion,
/// always round-trip-safe through `osascript`, the database, and JSON wire
/// format.
///
/// `#[serde(transparent)]` means the JSON shape is unchanged from when the
/// type was `Uuid`: a bare string field, no enum tagging.
///
/// # Construction
///
/// - [`ThingsId::new_v4`] — fresh hyphenated UUID, used by `SqlxBackend`
/// - [`ThingsId::from_str`] — strict parse, rejects anything that isn't one
///   of the two known formats; used at MCP boundaries
/// - [`From<Uuid>`] — infallible, wraps a UUID's hyphenated form
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThingsId(String);

impl ThingsId {
    /// Generate a fresh hyphenated UUID, suitable for `SqlxBackend`-created
    /// entities.
    #[must_use]
    pub fn new_v4() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Borrow the underlying string (for SQL parameter binding, AppleScript
    /// interpolation, logging, etc.).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into the owned `String`.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }

    /// Construct without validation. Reserved for trusted sources only —
    /// values read directly from the SQLite `uuid` column or returned by
    /// `osascript`. Public input must go through [`FromStr`].
    pub(crate) fn from_trusted(s: String) -> Self {
        Self(s)
    }

    /// Returns true if `s` matches the native 21–22-char base62 Things format.
    fn is_things_native(s: &str) -> bool {
        let len = s.len();
        (len == 21 || len == 22) && s.chars().all(|c| c.is_ascii_alphanumeric())
    }
}

impl fmt::Display for ThingsId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ThingsId {
    type Err = ThingsError;

    /// Strict parse. Accepts:
    /// - Hyphenated RFC-4122 UUIDs (36 chars)
    /// - Things native IDs (21 or 22 base62 chars)
    ///
    /// Anything else returns a `ThingsError::Validation` so MCP callers see a
    /// clear error before the request hits the database.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if Uuid::parse_str(s).is_ok() {
            return Ok(Self(s.to_string()));
        }
        if Self::is_things_native(s) {
            return Ok(Self(s.to_string()));
        }
        Err(ThingsError::validation(format!(
            "invalid Things 3 identifier {s:?}: expected RFC-4122 UUID \
             (36 chars, hex+hyphens) or Things native ID (21–22 base62 chars)"
        )))
    }
}

impl From<Uuid> for ThingsId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid.to_string())
    }
}

impl AsRef<str> for ThingsId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod things_id_tests {
    use super::*;

    #[test]
    fn new_v4_produces_hyphenated_uuid_string() {
        let id = ThingsId::new_v4();
        let s = id.as_str();
        assert_eq!(s.len(), 36);
        assert!(Uuid::parse_str(s).is_ok());
    }

    #[test]
    fn from_str_accepts_hyphenated_uuid() {
        let s = "9d3f1e44-5c2a-4b8e-9c1f-7e2d8a4b3c5e";
        let id: ThingsId = s.parse().unwrap();
        assert_eq!(id.as_str(), s);
    }

    #[test]
    fn from_str_accepts_22_char_native_id() {
        let s = "R4t2G8Q63aGZq4epMHNeCr";
        assert_eq!(s.len(), 22);
        let id: ThingsId = s.parse().unwrap();
        assert_eq!(id.as_str(), s);
    }

    #[test]
    fn from_str_accepts_21_char_native_id() {
        // Real example pulled from a Things 3 database.
        let s = "19KLMeA2ULbixtvNbXsDK";
        assert_eq!(s.len(), 21);
        let id: ThingsId = s.parse().unwrap();
        assert_eq!(id.as_str(), s);
    }

    #[test]
    fn from_str_rejects_short_garbage() {
        let err = "abc".parse::<ThingsId>().unwrap_err();
        assert!(matches!(err, ThingsError::Validation { .. }));
    }

    #[test]
    fn from_str_rejects_long_garbage() {
        // 23 chars — wrong length for native, wrong format for UUID
        let err = "ZZZZZZZZZZZZZZZZZZZZZZZ".parse::<ThingsId>().unwrap_err();
        assert!(matches!(err, ThingsError::Validation { .. }));
    }

    #[test]
    fn from_str_rejects_native_with_special_chars() {
        // 22 chars, but contains `-` (which is fine for UUIDs but not native)
        let err = "R4t2G8Q63aGZq4epMHN-Cr".parse::<ThingsId>().unwrap_err();
        assert!(matches!(err, ThingsError::Validation { .. }));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = "".parse::<ThingsId>().unwrap_err();
        assert!(matches!(err, ThingsError::Validation { .. }));
    }

    #[test]
    fn from_str_rejects_uuid_with_extra_chars() {
        // Valid UUID prefix + extra chars
        let err = "9d3f1e44-5c2a-4b8e-9c1f-7e2d8a4b3c5e-XYZ"
            .parse::<ThingsId>()
            .unwrap_err();
        assert!(matches!(err, ThingsError::Validation { .. }));
    }

    #[test]
    fn display_is_the_inner_string() {
        let id: ThingsId = "R4t2G8Q63aGZq4epMHNeCr".parse().unwrap();
        assert_eq!(format!("{id}"), "R4t2G8Q63aGZq4epMHNeCr");
    }

    #[test]
    fn from_uuid_wraps_hyphenated_form() {
        let uuid = Uuid::parse_str("9d3f1e44-5c2a-4b8e-9c1f-7e2d8a4b3c5e").unwrap();
        let id: ThingsId = uuid.into();
        assert_eq!(id.as_str(), "9d3f1e44-5c2a-4b8e-9c1f-7e2d8a4b3c5e");
    }

    #[test]
    fn serde_roundtrips_as_bare_string() {
        let id: ThingsId = "R4t2G8Q63aGZq4epMHNeCr".parse().unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"R4t2G8Q63aGZq4epMHNeCr\"");
        let back: ThingsId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn equality_is_string_equality() {
        let a: ThingsId = "R4t2G8Q63aGZq4epMHNeCr".parse().unwrap();
        let b: ThingsId = "R4t2G8Q63aGZq4epMHNeCr".parse().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn from_trusted_skips_validation() {
        // Confirms the internal escape hatch works for DB/AS-sourced strings.
        // Deliberately weird value to prove no validation happens.
        let id = ThingsId::from_trusted("anything-goes-here".to_string());
        assert_eq!(id.as_str(), "anything-goes-here");
    }
}

/// Task status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    #[serde(rename = "incomplete")]
    Incomplete,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "canceled")]
    Canceled,
    #[serde(rename = "trashed")]
    Trashed,
}

/// Task type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    #[serde(rename = "to-do")]
    Todo,
    #[serde(rename = "project")]
    Project,
    #[serde(rename = "heading")]
    Heading,
    #[serde(rename = "area")]
    Area,
}

/// How to handle child tasks when deleting a parent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteChildHandling {
    /// Return error if task has children (default)
    #[serde(rename = "error")]
    Error,
    /// Delete parent and all children
    #[serde(rename = "cascade")]
    Cascade,
    /// Delete parent only, orphan children
    #[serde(rename = "orphan")]
    Orphan,
}

/// Main task entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier
    pub uuid: ThingsId,
    /// Task title
    pub title: String,
    /// Task type
    pub task_type: TaskType,
    /// Task status
    pub status: TaskStatus,
    /// Optional notes
    pub notes: Option<String>,
    /// Start date
    pub start_date: Option<NaiveDate>,
    /// Deadline
    pub deadline: Option<NaiveDate>,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// Last modification timestamp
    pub modified: DateTime<Utc>,
    /// Completion timestamp (when status changed to completed)
    pub stop_date: Option<DateTime<Utc>>,
    /// Parent project UUID
    pub project_uuid: Option<ThingsId>,
    /// Parent area UUID
    pub area_uuid: Option<ThingsId>,
    /// Parent task UUID
    pub parent_uuid: Option<ThingsId>,
    /// Associated tags
    pub tags: Vec<String>,
    /// Child tasks
    pub children: Vec<Task>,
}

/// Project entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique identifier
    pub uuid: ThingsId,
    /// Project title
    pub title: String,
    /// Optional notes
    pub notes: Option<String>,
    /// Start date
    pub start_date: Option<NaiveDate>,
    /// Deadline
    pub deadline: Option<NaiveDate>,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// Last modification timestamp
    pub modified: DateTime<Utc>,
    /// Parent area UUID
    pub area_uuid: Option<ThingsId>,
    /// Associated tags
    pub tags: Vec<String>,
    /// Project status
    pub status: TaskStatus,
    /// Child tasks
    pub tasks: Vec<Task>,
}

/// Area entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    /// Unique identifier
    pub uuid: ThingsId,
    /// Area title
    pub title: String,
    /// Optional notes
    pub notes: Option<String>,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// Last modification timestamp
    pub modified: DateTime<Utc>,
    /// Associated tags
    pub tags: Vec<String>,
    /// Child projects
    pub projects: Vec<Project>,
}

/// Tag entity (enhanced with duplicate prevention support)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// Unique identifier
    pub uuid: ThingsId,
    /// Tag title (display form, preserves case)
    pub title: String,
    /// Keyboard shortcut
    pub shortcut: Option<String>,
    /// Parent tag UUID (for nested tags)
    pub parent_uuid: Option<ThingsId>,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// Last modification timestamp
    pub modified: DateTime<Utc>,
    /// How many tasks use this tag
    pub usage_count: u32,
    /// Last time this tag was used
    pub last_used: Option<DateTime<Utc>>,
}

/// Tag creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTagRequest {
    /// Tag title (required)
    pub title: String,
    /// Keyboard shortcut
    pub shortcut: Option<String>,
    /// Parent tag UUID (for nested tags)
    pub parent_uuid: Option<ThingsId>,
}

/// Tag update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTagRequest {
    /// Tag UUID
    pub uuid: ThingsId,
    /// New title
    pub title: Option<String>,
    /// New shortcut
    pub shortcut: Option<String>,
    /// New parent UUID
    pub parent_uuid: Option<ThingsId>,
}

/// Tag match type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagMatchType {
    /// Exact match (case-insensitive)
    #[serde(rename = "exact")]
    Exact,
    /// Same text, different case
    #[serde(rename = "case_mismatch")]
    CaseMismatch,
    /// Fuzzy match (high similarity via Levenshtein distance)
    #[serde(rename = "similar")]
    Similar,
    /// Substring/contains match
    #[serde(rename = "partial")]
    PartialMatch,
}

/// Tag search result with similarity scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagMatch {
    /// The matched tag
    pub tag: Tag,
    /// Similarity score (0.0 to 1.0, higher is better)
    pub similarity_score: f32,
    /// Type of match
    pub match_type: TagMatchType,
}

/// Result of tag creation with duplicate checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TagCreationResult {
    /// New tag was created
    #[serde(rename = "created")]
    Created {
        /// UUID of created tag
        uuid: ThingsId,
        /// Always true for this variant
        is_new: bool,
    },
    /// Existing exact match found
    #[serde(rename = "existing")]
    Existing {
        /// The existing tag
        tag: Tag,
        /// Always false for this variant
        is_new: bool,
    },
    /// Similar tags found (user decision needed)
    #[serde(rename = "similar_found")]
    SimilarFound {
        /// Tags similar to requested title
        similar_tags: Vec<TagMatch>,
        /// The title user requested
        requested_title: String,
    },
}

/// Result of tag assignment to task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TagAssignmentResult {
    /// Tag assigned successfully
    #[serde(rename = "assigned")]
    Assigned {
        /// UUID of the tag that was assigned
        tag_uuid: ThingsId,
    },
    /// Similar tags found (user decision needed)
    #[serde(rename = "suggestions")]
    Suggestions {
        /// Suggested alternative tags
        similar_tags: Vec<TagMatch>,
    },
}

/// Tag auto-completion suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCompletion {
    /// The tag
    pub tag: Tag,
    /// Priority score (based on usage, recency, match quality)
    pub score: f32,
}

/// Tag statistics for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagStatistics {
    /// Tag UUID
    pub uuid: ThingsId,
    /// Tag title
    pub title: String,
    /// Total usage count
    pub usage_count: u32,
    /// Task UUIDs using this tag
    pub task_uuids: Vec<ThingsId>,
    /// Related tags (frequently used together)
    pub related_tags: Vec<(String, u32)>, // (tag_title, co_occurrence_count)
}

/// Pair of similar tags (for duplicate detection)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagPair {
    /// First tag
    pub tag1: Tag,
    /// Second tag
    pub tag2: Tag,
    /// Similarity score between them
    pub similarity: f32,
}

/// Task creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    /// Task title (required)
    pub title: String,
    /// Task type (defaults to Todo)
    pub task_type: Option<TaskType>,
    /// Optional notes
    pub notes: Option<String>,
    /// Start date
    pub start_date: Option<NaiveDate>,
    /// Deadline
    pub deadline: Option<NaiveDate>,
    /// Project UUID (validated if provided)
    pub project_uuid: Option<ThingsId>,
    /// Area UUID (validated if provided)
    pub area_uuid: Option<ThingsId>,
    /// Parent task UUID (for subtasks)
    pub parent_uuid: Option<ThingsId>,
    /// Tags (as string names)
    pub tags: Option<Vec<String>>,
    /// Initial status (defaults to Incomplete)
    pub status: Option<TaskStatus>,
}

/// Task update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskRequest {
    /// Task UUID
    pub uuid: ThingsId,
    /// New title
    pub title: Option<String>,
    /// New notes
    pub notes: Option<String>,
    /// New start date
    pub start_date: Option<NaiveDate>,
    /// New deadline
    pub deadline: Option<NaiveDate>,
    /// New status
    pub status: Option<TaskStatus>,
    /// New project UUID
    pub project_uuid: Option<ThingsId>,
    /// New area UUID
    pub area_uuid: Option<ThingsId>,
    /// New tags
    pub tags: Option<Vec<String>>,
}

/// Task filters for queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskFilters {
    /// Filter by status
    pub status: Option<TaskStatus>,
    /// Filter by task type
    pub task_type: Option<TaskType>,
    /// Filter by project UUID
    pub project_uuid: Option<ThingsId>,
    /// Filter by area UUID
    pub area_uuid: Option<ThingsId>,
    /// Filter by tags (AND semantics — task must contain every listed tag).
    pub tags: Option<Vec<String>>,
    /// Filter by start date range
    pub start_date_from: Option<NaiveDate>,
    pub start_date_to: Option<NaiveDate>,
    /// Filter by deadline range
    pub deadline_from: Option<NaiveDate>,
    pub deadline_to: Option<NaiveDate>,
    /// Search query
    pub search_query: Option<String>,
    /// Limit results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// A task paired with its fuzzy-match relevance score.
///
/// Returned by [`crate::query::TaskQueryBuilder::execute_ranked`].
///
/// Requires the `advanced-queries` feature flag.
#[cfg(feature = "advanced-queries")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedTask {
    /// The matched task.
    pub task: Task,
    /// Relevance score in `[0.0, 1.0]`; higher is a better match.
    pub score: f32,
}

/// Project creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    /// Project title (required)
    pub title: String,
    /// Optional notes
    pub notes: Option<String>,
    /// Area UUID (validated if provided)
    pub area_uuid: Option<ThingsId>,
    /// Start date
    pub start_date: Option<NaiveDate>,
    /// Deadline
    pub deadline: Option<NaiveDate>,
    /// Tags (as string names)
    pub tags: Option<Vec<String>>,
}

/// Project update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectRequest {
    /// Project UUID
    pub uuid: ThingsId,
    /// New title
    pub title: Option<String>,
    /// New notes
    pub notes: Option<String>,
    /// New area UUID
    pub area_uuid: Option<ThingsId>,
    /// New start date
    pub start_date: Option<NaiveDate>,
    /// New deadline
    pub deadline: Option<NaiveDate>,
    /// New tags
    pub tags: Option<Vec<String>>,
}

/// Area creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAreaRequest {
    /// Area title (required)
    pub title: String,
}

/// Area update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAreaRequest {
    /// Area UUID
    pub uuid: ThingsId,
    /// New title
    pub title: String,
}

/// How to handle child tasks when completing/deleting a project
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProjectChildHandling {
    /// Return error if project has child tasks (default, safest)
    #[serde(rename = "error")]
    #[default]
    Error,
    /// Complete/delete all child tasks
    #[serde(rename = "cascade")]
    Cascade,
    /// Move child tasks to inbox (orphan them)
    #[serde(rename = "orphan")]
    Orphan,
}

// ============================================================================
// Bulk Operation Models
// ============================================================================

/// Request to move multiple tasks to a project or area
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkMoveRequest {
    /// Task UUIDs to move
    pub task_uuids: Vec<ThingsId>,
    /// Target project UUID (optional)
    pub project_uuid: Option<ThingsId>,
    /// Target area UUID (optional)
    pub area_uuid: Option<ThingsId>,
}

/// Request to update dates for multiple tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkUpdateDatesRequest {
    /// Task UUIDs to update
    pub task_uuids: Vec<ThingsId>,
    /// New start date (None means don't update)
    pub start_date: Option<NaiveDate>,
    /// New deadline (None means don't update)
    pub deadline: Option<NaiveDate>,
    /// Clear start date (set to NULL)
    pub clear_start_date: bool,
    /// Clear deadline (set to NULL)
    pub clear_deadline: bool,
}

/// Request to complete multiple tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkCompleteRequest {
    /// Task UUIDs to complete
    pub task_uuids: Vec<ThingsId>,
}

/// Request to delete multiple tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkDeleteRequest {
    /// Task UUIDs to delete
    pub task_uuids: Vec<ThingsId>,
}

/// Request to create multiple tasks in one call.
///
/// Bulk creation is best-effort and non-atomic — each task is attempted
/// independently and per-item failures are surfaced via `BulkOperationResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkCreateTasksRequest {
    /// Tasks to create
    pub tasks: Vec<CreateTaskRequest>,
}

/// Result of a bulk operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Number of tasks processed
    pub processed_count: usize,
    /// Result message
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_task_status_serialization() {
        let status = TaskStatus::Incomplete;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"incomplete\"");

        let status = TaskStatus::Completed;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"completed\"");

        let status = TaskStatus::Canceled;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"canceled\"");

        let status = TaskStatus::Trashed;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"trashed\"");
    }

    #[test]
    fn test_task_status_deserialization() {
        let deserialized: TaskStatus = serde_json::from_str("\"incomplete\"").unwrap();
        assert_eq!(deserialized, TaskStatus::Incomplete);

        let deserialized: TaskStatus = serde_json::from_str("\"completed\"").unwrap();
        assert_eq!(deserialized, TaskStatus::Completed);

        let deserialized: TaskStatus = serde_json::from_str("\"canceled\"").unwrap();
        assert_eq!(deserialized, TaskStatus::Canceled);

        let deserialized: TaskStatus = serde_json::from_str("\"trashed\"").unwrap();
        assert_eq!(deserialized, TaskStatus::Trashed);
    }

    #[test]
    fn test_task_type_serialization() {
        let task_type = TaskType::Todo;
        let serialized = serde_json::to_string(&task_type).unwrap();
        assert_eq!(serialized, "\"to-do\"");

        let task_type = TaskType::Project;
        let serialized = serde_json::to_string(&task_type).unwrap();
        assert_eq!(serialized, "\"project\"");

        let task_type = TaskType::Heading;
        let serialized = serde_json::to_string(&task_type).unwrap();
        assert_eq!(serialized, "\"heading\"");

        let task_type = TaskType::Area;
        let serialized = serde_json::to_string(&task_type).unwrap();
        assert_eq!(serialized, "\"area\"");
    }

    #[test]
    fn test_task_type_deserialization() {
        let deserialized: TaskType = serde_json::from_str("\"to-do\"").unwrap();
        assert_eq!(deserialized, TaskType::Todo);

        let deserialized: TaskType = serde_json::from_str("\"project\"").unwrap();
        assert_eq!(deserialized, TaskType::Project);

        let deserialized: TaskType = serde_json::from_str("\"heading\"").unwrap();
        assert_eq!(deserialized, TaskType::Heading);

        let deserialized: TaskType = serde_json::from_str("\"area\"").unwrap();
        assert_eq!(deserialized, TaskType::Area);
    }

    #[test]
    fn test_task_creation() {
        let uuid = ThingsId::new_v4();
        let now = Utc::now();
        let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let deadline = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

        let task = Task {
            uuid: uuid.clone(),
            title: "Test Task".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: Some("Test notes".to_string()),
            start_date: Some(start_date),
            deadline: Some(deadline),
            created: now,
            modified: now,
            stop_date: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: vec!["work".to_string(), "urgent".to_string()],
            children: vec![],
        };

        assert_eq!(task.uuid, uuid);
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.task_type, TaskType::Todo);
        assert_eq!(task.status, TaskStatus::Incomplete);
        assert_eq!(task.notes, Some("Test notes".to_string()));
        assert_eq!(task.start_date, Some(start_date));
        assert_eq!(task.deadline, Some(deadline));
        assert_eq!(task.tags.len(), 2);
        assert!(task.tags.contains(&"work".to_string()));
        assert!(task.tags.contains(&"urgent".to_string()));
    }

    #[test]
    fn test_task_serialization() {
        let uuid = ThingsId::new_v4();
        let now = Utc::now();

        let task = Task {
            uuid: uuid.clone(),
            title: "Test Task".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: None,
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            stop_date: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: vec![],
            children: vec![],
        };

        let serialized = serde_json::to_string(&task).unwrap();
        let deserialized: Task = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.uuid, task.uuid);
        assert_eq!(deserialized.title, task.title);
        assert_eq!(deserialized.task_type, task.task_type);
        assert_eq!(deserialized.status, task.status);
    }

    #[test]
    fn test_project_creation() {
        let uuid = ThingsId::new_v4();
        let area_uuid = ThingsId::new_v4();
        let now = Utc::now();

        let project = Project {
            uuid: uuid.clone(),
            title: "Test Project".to_string(),
            notes: Some("Project notes".to_string()),
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            area_uuid: Some(area_uuid.clone()),
            tags: vec!["project".to_string()],
            status: TaskStatus::Incomplete,
            tasks: vec![],
        };

        assert_eq!(project.uuid, uuid);
        assert_eq!(project.title, "Test Project");
        assert_eq!(project.area_uuid, Some(area_uuid));
        assert_eq!(project.status, TaskStatus::Incomplete);
        assert_eq!(project.tags.len(), 1);
    }

    #[test]
    fn test_project_serialization() {
        let uuid = ThingsId::new_v4();
        let now = Utc::now();

        let project = Project {
            uuid: uuid.clone(),
            title: "Test Project".to_string(),
            notes: None,
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            area_uuid: None,
            tags: vec![],
            status: TaskStatus::Incomplete,
            tasks: vec![],
        };

        let serialized = serde_json::to_string(&project).unwrap();
        let deserialized: Project = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.uuid, project.uuid);
        assert_eq!(deserialized.title, project.title);
        assert_eq!(deserialized.status, project.status);
    }

    #[test]
    fn test_area_creation() {
        let uuid = ThingsId::new_v4();
        let now = Utc::now();

        let area = Area {
            uuid: uuid.clone(),
            title: "Test Area".to_string(),
            notes: Some("Area notes".to_string()),
            created: now,
            modified: now,
            tags: vec!["area".to_string()],
            projects: vec![],
        };

        assert_eq!(area.uuid, uuid);
        assert_eq!(area.title, "Test Area");
        assert_eq!(area.notes, Some("Area notes".to_string()));
        assert_eq!(area.tags.len(), 1);
    }

    #[test]
    fn test_area_serialization() {
        let uuid = ThingsId::new_v4();
        let now = Utc::now();

        let area = Area {
            uuid: uuid.clone(),
            title: "Test Area".to_string(),
            notes: None,
            created: now,
            modified: now,
            tags: vec![],
            projects: vec![],
        };

        let serialized = serde_json::to_string(&area).unwrap();
        let deserialized: Area = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.uuid, area.uuid);
        assert_eq!(deserialized.title, area.title);
    }

    #[test]
    fn test_tag_creation() {
        let uuid = ThingsId::new_v4();
        let parent_uuid = ThingsId::new_v4();
        let now = Utc::now();

        let tag = Tag {
            uuid: uuid.clone(),
            title: "work".to_string(),
            shortcut: Some("w".to_string()),
            parent_uuid: Some(parent_uuid.clone()),
            created: now,
            modified: now,
            usage_count: 5,
            last_used: Some(now),
        };

        assert_eq!(tag.uuid, uuid);
        assert_eq!(tag.title, "work");
        assert_eq!(tag.shortcut, Some("w".to_string()));
        assert_eq!(tag.parent_uuid, Some(parent_uuid));
        assert_eq!(tag.usage_count, 5);
        assert_eq!(tag.last_used, Some(now));
    }

    #[test]
    fn test_tag_serialization() {
        let uuid = ThingsId::new_v4();
        let now = Utc::now();

        let tag = Tag {
            uuid: uuid.clone(),
            title: "test".to_string(),
            shortcut: None,
            parent_uuid: None,
            created: now,
            modified: now,
            usage_count: 0,
            last_used: None,
        };

        let serialized = serde_json::to_string(&tag).unwrap();
        let deserialized: Tag = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.uuid, tag.uuid);
        assert_eq!(deserialized.title, tag.title);
        assert_eq!(deserialized.usage_count, tag.usage_count);
    }

    #[test]
    fn test_create_task_request() {
        let project_uuid = ThingsId::new_v4();
        let area_uuid = ThingsId::new_v4();
        let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let request = CreateTaskRequest {
            title: "New Task".to_string(),
            task_type: None,
            notes: Some("Task notes".to_string()),
            start_date: Some(start_date),
            deadline: None,
            project_uuid: Some(project_uuid.clone()),
            area_uuid: Some(area_uuid.clone()),
            parent_uuid: None,
            tags: Some(vec!["new".to_string()]),
            status: None,
        };

        assert_eq!(request.title, "New Task");
        assert_eq!(request.project_uuid, Some(project_uuid));
        assert_eq!(request.area_uuid, Some(area_uuid));
        assert_eq!(request.start_date, Some(start_date));
        assert_eq!(request.tags.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_create_task_request_serialization() {
        let request = CreateTaskRequest {
            title: "Test".to_string(),
            task_type: None,
            notes: None,
            start_date: None,
            deadline: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: None,
            status: None,
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: CreateTaskRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.title, request.title);
    }

    #[test]
    fn test_update_task_request() {
        let uuid = ThingsId::new_v4();

        let request = UpdateTaskRequest {
            uuid: uuid.clone(),
            title: Some("Updated Title".to_string()),
            notes: Some("Updated notes".to_string()),
            start_date: None,
            deadline: None,
            status: Some(TaskStatus::Completed),
            project_uuid: None,
            area_uuid: None,
            tags: Some(vec!["updated".to_string()]),
        };

        assert_eq!(request.uuid, uuid);
        assert_eq!(request.title, Some("Updated Title".to_string()));
        assert_eq!(request.status, Some(TaskStatus::Completed));
        assert_eq!(request.tags, Some(vec!["updated".to_string()]));
    }

    #[test]
    fn test_update_task_request_serialization() {
        let uuid = ThingsId::new_v4();

        let request = UpdateTaskRequest {
            uuid: uuid.clone(),
            title: None,
            notes: None,
            start_date: None,
            deadline: None,
            status: None,
            project_uuid: None,
            area_uuid: None,
            tags: None,
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: UpdateTaskRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.uuid, request.uuid);
    }

    #[test]
    fn test_task_filters_default() {
        let filters = TaskFilters::default();

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
    fn test_task_filters_creation() {
        let project_uuid = ThingsId::new_v4();
        let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let filters = TaskFilters {
            status: Some(TaskStatus::Incomplete),
            task_type: Some(TaskType::Todo),
            project_uuid: Some(project_uuid.clone()),
            area_uuid: None,
            tags: Some(vec!["work".to_string()]),
            start_date_from: Some(start_date),
            start_date_to: None,
            deadline_from: None,
            deadline_to: None,
            search_query: Some("test".to_string()),
            limit: Some(10),
            offset: Some(0),
        };

        assert_eq!(filters.status, Some(TaskStatus::Incomplete));
        assert_eq!(filters.task_type, Some(TaskType::Todo));
        assert_eq!(filters.project_uuid, Some(project_uuid));
        assert_eq!(filters.search_query, Some("test".to_string()));
        assert_eq!(filters.limit, Some(10));
        assert_eq!(filters.offset, Some(0));
    }

    #[test]
    fn test_task_filters_serialization() {
        let filters = TaskFilters {
            status: Some(TaskStatus::Completed),
            task_type: Some(TaskType::Project),
            project_uuid: None,
            area_uuid: None,
            tags: None,
            start_date_from: None,
            start_date_to: None,
            deadline_from: None,
            deadline_to: None,
            search_query: None,
            limit: None,
            offset: None,
        };

        let serialized = serde_json::to_string(&filters).unwrap();
        let deserialized: TaskFilters = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.status, filters.status);
        assert_eq!(deserialized.task_type, filters.task_type);
    }

    #[test]
    fn test_task_status_equality() {
        assert_eq!(TaskStatus::Incomplete, TaskStatus::Incomplete);
        assert_ne!(TaskStatus::Incomplete, TaskStatus::Completed);
        assert_ne!(TaskStatus::Completed, TaskStatus::Canceled);
        assert_ne!(TaskStatus::Canceled, TaskStatus::Trashed);
    }

    #[test]
    fn test_task_type_equality() {
        assert_eq!(TaskType::Todo, TaskType::Todo);
        assert_ne!(TaskType::Todo, TaskType::Project);
        assert_ne!(TaskType::Project, TaskType::Heading);
        assert_ne!(TaskType::Heading, TaskType::Area);
    }

    #[test]
    fn test_task_with_children() {
        let parent_uuid = ThingsId::new_v4();
        let child_uuid = ThingsId::new_v4();
        let now = Utc::now();

        let child_task = Task {
            uuid: child_uuid,
            title: "Child Task".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: None,
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            stop_date: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: Some(parent_uuid.clone()),
            tags: vec![],
            children: vec![],
        };

        let parent_task = Task {
            uuid: parent_uuid.clone(),
            title: "Parent Task".to_string(),
            task_type: TaskType::Heading,
            status: TaskStatus::Incomplete,
            notes: None,
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            stop_date: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: vec![],
            children: vec![child_task],
        };

        assert_eq!(parent_task.children.len(), 1);
        assert_eq!(parent_task.children[0].parent_uuid, Some(parent_uuid));
        assert_eq!(parent_task.children[0].title, "Child Task");
    }

    #[test]
    fn test_project_with_tasks() {
        let project_uuid = ThingsId::new_v4();
        let task_uuid = ThingsId::new_v4();
        let now = Utc::now();

        let task = Task {
            uuid: task_uuid,
            title: "Project Task".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: None,
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            stop_date: None,
            project_uuid: Some(project_uuid.clone()),
            area_uuid: None,
            parent_uuid: None,
            tags: vec![],
            children: vec![],
        };

        let project = Project {
            uuid: project_uuid.clone(),
            title: "Test Project".to_string(),
            notes: None,
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            area_uuid: None,
            tags: vec![],
            status: TaskStatus::Incomplete,
            tasks: vec![task],
        };

        assert_eq!(project.tasks.len(), 1);
        assert_eq!(project.tasks[0].project_uuid, Some(project_uuid));
        assert_eq!(project.tasks[0].title, "Project Task");
    }

    #[test]
    fn test_area_with_projects() {
        let area_uuid = ThingsId::new_v4();
        let project_uuid = ThingsId::new_v4();
        let now = Utc::now();

        let project = Project {
            uuid: project_uuid,
            title: "Area Project".to_string(),
            notes: None,
            start_date: None,
            deadline: None,
            created: now,
            modified: now,
            area_uuid: Some(area_uuid.clone()),
            tags: vec![],
            status: TaskStatus::Incomplete,
            tasks: vec![],
        };

        let area = Area {
            uuid: area_uuid.clone(),
            title: "Test Area".to_string(),
            notes: None,
            created: now,
            modified: now,
            tags: vec![],
            projects: vec![project],
        };

        assert_eq!(area.projects.len(), 1);
        assert_eq!(area.projects[0].area_uuid, Some(area_uuid));
        assert_eq!(area.projects[0].title, "Area Project");
    }
}
