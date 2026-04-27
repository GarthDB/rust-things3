//! Boolean expression tree for task filtering.
//!
//! [`FilterExpr`] is a recursive expression of [`FilterPredicate`]s combined
//! with `And`, `Or`, and `Not`. It is evaluated in Rust against
//! [`crate::models::Task`] values returned from
//! [`crate::database::ThingsDatabase::query_tasks`], composing alongside
//! [`crate::models::TaskFilters`] (which AND-narrows the SQL fetch) and the
//! other builder-only post-filters (`any_tags`, `exclude_tags`, fuzzy).
//!
//! Vacuous semantics: `And(vec![])` evaluates to `true`,
//! `Or(vec![])` to `false`.
//!
//! Requires the `advanced-queries` feature flag.

#![cfg(feature = "advanced-queries")]

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{Task, TaskStatus, TaskType};

/// Boolean combinator over [`FilterPredicate`]s.
///
/// JSON encoding uses adjacent tagging: `{"op": "and", "args": [...]}`,
/// `{"op": "pred", "args": {"kind": "status", ...}}`. Adjacent tagging is
/// required because newtype variants over `Vec`/`Box` aren't compatible with
/// serde's internal tagging.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", content = "args", rename_all = "snake_case")]
pub enum FilterExpr {
    /// All children must match. `And(vec![])` is vacuously `true`.
    And(Vec<FilterExpr>),
    /// At least one child must match. `Or(vec![])` is vacuously `false`.
    Or(Vec<FilterExpr>),
    /// Negation.
    Not(Box<FilterExpr>),
    /// Leaf predicate.
    Pred(FilterPredicate),
}

/// Leaf predicates evaluated against a single [`Task`].
///
/// `TitleContains` and `NotesContains` are case-insensitive, matching the
/// convention used by `search_tasks` elsewhere in this crate.
///
/// JSON encoding: `{"kind": "status", "value": "incomplete"}`,
/// `{"kind": "project", "value": "<uuid>"}`. Adjacent tagging keeps the wire
/// format readable while supporting newtype variants over scalar types
/// (`Uuid`, `String`, `NaiveDate`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum FilterPredicate {
    /// Task status matches.
    Status(TaskStatus),
    /// Task type matches.
    TaskType(TaskType),
    /// Task belongs to the given project.
    Project(Uuid),
    /// Task belongs to the given area.
    Area(Uuid),
    /// Task carries the named tag (case-sensitive, matches existing tag filters).
    HasTag(String),
    /// `start_date < d`. Tasks with no start date never match.
    StartDateBefore(NaiveDate),
    /// `start_date > d`. Tasks with no start date never match.
    StartDateAfter(NaiveDate),
    /// `deadline < d`. Tasks with no deadline never match.
    DeadlineBefore(NaiveDate),
    /// `deadline > d`. Tasks with no deadline never match.
    DeadlineAfter(NaiveDate),
    /// Title contains substring (case-insensitive).
    TitleContains(String),
    /// Notes contain substring (case-insensitive). Tasks with no notes never match.
    NotesContains(String),
}

impl FilterExpr {
    /// Evaluate the expression against a task.
    #[must_use]
    pub fn matches(&self, task: &Task) -> bool {
        match self {
            FilterExpr::And(children) => children.iter().all(|c| c.matches(task)),
            FilterExpr::Or(children) => children.iter().any(|c| c.matches(task)),
            FilterExpr::Not(inner) => !inner.matches(task),
            FilterExpr::Pred(pred) => pred.matches(task),
        }
    }

    /// Wrap two expressions with AND. Right-hand side is appended if `self` is
    /// already an `And`, keeping the tree shallow.
    #[must_use]
    pub fn and(self, other: FilterExpr) -> FilterExpr {
        match self {
            FilterExpr::And(mut children) => {
                children.push(other);
                FilterExpr::And(children)
            }
            this => FilterExpr::And(vec![this, other]),
        }
    }

    /// Wrap two expressions with OR. Right-hand side is appended if `self` is
    /// already an `Or`, keeping the tree shallow.
    #[must_use]
    pub fn or(self, other: FilterExpr) -> FilterExpr {
        match self {
            FilterExpr::Or(mut children) => {
                children.push(other);
                FilterExpr::Or(children)
            }
            this => FilterExpr::Or(vec![this, other]),
        }
    }

    /// Wrap in a `Not`. Named to keep the fluent chain readable
    /// (`expr.and(other.not())`); we deliberately don't implement `std::ops::Not`
    /// because the prefix `!` form interferes with builder chaining.
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn not(self) -> FilterExpr {
        FilterExpr::Not(Box::new(self))
    }

    /// Convenience: leaf predicate from a [`TaskStatus`].
    #[must_use]
    pub fn status(status: TaskStatus) -> Self {
        FilterExpr::Pred(FilterPredicate::Status(status))
    }

    /// Convenience: leaf predicate from a [`TaskType`].
    #[must_use]
    pub fn task_type(task_type: TaskType) -> Self {
        FilterExpr::Pred(FilterPredicate::TaskType(task_type))
    }

    /// Convenience: leaf predicate filtering to a project.
    #[must_use]
    pub fn project(uuid: Uuid) -> Self {
        FilterExpr::Pred(FilterPredicate::Project(uuid))
    }

    /// Convenience: leaf predicate filtering to an area.
    #[must_use]
    pub fn area(uuid: Uuid) -> Self {
        FilterExpr::Pred(FilterPredicate::Area(uuid))
    }

    /// Convenience: leaf predicate matching a single tag.
    #[must_use]
    pub fn has_tag(tag: impl Into<String>) -> Self {
        FilterExpr::Pred(FilterPredicate::HasTag(tag.into()))
    }

    /// Convenience: case-insensitive title-contains predicate.
    #[must_use]
    pub fn title_contains(needle: impl Into<String>) -> Self {
        FilterExpr::Pred(FilterPredicate::TitleContains(needle.into()))
    }

    /// Convenience: case-insensitive notes-contains predicate.
    #[must_use]
    pub fn notes_contains(needle: impl Into<String>) -> Self {
        FilterExpr::Pred(FilterPredicate::NotesContains(needle.into()))
    }
}

impl FilterPredicate {
    fn matches(&self, task: &Task) -> bool {
        match self {
            FilterPredicate::Status(s) => task.status == *s,
            FilterPredicate::TaskType(t) => task.task_type == *t,
            FilterPredicate::Project(uuid) => task.project_uuid == Some(*uuid),
            FilterPredicate::Area(uuid) => task.area_uuid == Some(*uuid),
            FilterPredicate::HasTag(tag) => task.tags.iter().any(|t| t == tag),
            FilterPredicate::StartDateBefore(d) => task.start_date.is_some_and(|sd| sd < *d),
            FilterPredicate::StartDateAfter(d) => task.start_date.is_some_and(|sd| sd > *d),
            FilterPredicate::DeadlineBefore(d) => task.deadline.is_some_and(|dl| dl < *d),
            FilterPredicate::DeadlineAfter(d) => task.deadline.is_some_and(|dl| dl > *d),
            FilterPredicate::TitleContains(needle) => {
                task.title.to_lowercase().contains(&needle.to_lowercase())
            }
            FilterPredicate::NotesContains(needle) => task
                .notes
                .as_deref()
                .is_some_and(|n| n.to_lowercase().contains(&needle.to_lowercase())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn task(title: &str) -> Task {
        Task {
            uuid: Uuid::new_v4(),
            title: title.to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: None,
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
        }
    }

    #[test]
    fn test_pred_status_matches() {
        let mut t = task("a");
        t.status = TaskStatus::Completed;
        assert!(FilterExpr::status(TaskStatus::Completed).matches(&t));
    }

    #[test]
    fn test_pred_status_does_not_match() {
        let t = task("a"); // Incomplete by default
        assert!(!FilterExpr::status(TaskStatus::Completed).matches(&t));
    }

    #[test]
    fn test_pred_task_type_matches() {
        let mut t = task("a");
        t.task_type = TaskType::Project;
        assert!(FilterExpr::task_type(TaskType::Project).matches(&t));
        assert!(!FilterExpr::task_type(TaskType::Todo).matches(&t));
    }

    #[test]
    fn test_pred_project_matches_only_when_set() {
        let project = Uuid::new_v4();
        let mut t = task("a");
        assert!(!FilterExpr::project(project).matches(&t));
        t.project_uuid = Some(project);
        assert!(FilterExpr::project(project).matches(&t));
    }

    #[test]
    fn test_pred_has_tag_matches() {
        let mut t = task("a");
        t.tags = vec!["work".to_string(), "urgent".to_string()];
        assert!(FilterExpr::has_tag("work").matches(&t));
        assert!(!FilterExpr::has_tag("home").matches(&t));
    }

    #[test]
    fn test_pred_title_contains_is_case_insensitive() {
        let t = task("Quarterly REVIEW Meeting");
        assert!(FilterExpr::title_contains("review").matches(&t));
        assert!(FilterExpr::title_contains("QUARTERLY").matches(&t));
        assert!(!FilterExpr::title_contains("budget").matches(&t));
    }

    #[test]
    fn test_pred_notes_contains_no_notes_never_matches() {
        let mut t = task("a");
        assert!(!FilterExpr::notes_contains("anything").matches(&t));
        t.notes = Some("Discuss BUDGET allocation".to_string());
        assert!(FilterExpr::notes_contains("budget").matches(&t));
    }

    #[test]
    fn test_pred_date_before_after_no_date_never_matches() {
        let mut t = task("a");
        let d = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        // No deadline set → neither before nor after matches.
        assert!(!FilterExpr::Pred(FilterPredicate::DeadlineBefore(d)).matches(&t));
        assert!(!FilterExpr::Pred(FilterPredicate::DeadlineAfter(d)).matches(&t));

        t.deadline = Some(NaiveDate::from_ymd_opt(2026, 4, 1).unwrap());
        assert!(FilterExpr::Pred(FilterPredicate::DeadlineBefore(d)).matches(&t));
        assert!(!FilterExpr::Pred(FilterPredicate::DeadlineAfter(d)).matches(&t));
    }

    #[test]
    fn test_and_all_match() {
        let mut t = task("Review");
        t.status = TaskStatus::Incomplete;
        let expr = FilterExpr::And(vec![
            FilterExpr::status(TaskStatus::Incomplete),
            FilterExpr::title_contains("review"),
        ]);
        assert!(expr.matches(&t));
    }

    #[test]
    fn test_and_one_fails_returns_false() {
        let t = task("Review");
        let expr = FilterExpr::And(vec![
            FilterExpr::status(TaskStatus::Completed),
            FilterExpr::title_contains("review"),
        ]);
        assert!(!expr.matches(&t));
    }

    #[test]
    fn test_or_first_match_returns_true() {
        let t = task("Review");
        let expr = FilterExpr::Or(vec![
            FilterExpr::status(TaskStatus::Completed),
            FilterExpr::title_contains("review"),
        ]);
        assert!(expr.matches(&t));
    }

    #[test]
    fn test_or_all_fail_returns_false() {
        let t = task("Review");
        let expr = FilterExpr::Or(vec![
            FilterExpr::status(TaskStatus::Completed),
            FilterExpr::title_contains("budget"),
        ]);
        assert!(!expr.matches(&t));
    }

    #[test]
    fn test_not_inverts() {
        let t = task("Review");
        let expr = FilterExpr::status(TaskStatus::Completed).not();
        assert!(expr.matches(&t));
        let expr = FilterExpr::status(TaskStatus::Incomplete).not();
        assert!(!expr.matches(&t));
    }

    #[test]
    fn test_and_empty_is_true() {
        let t = task("a");
        assert!(FilterExpr::And(vec![]).matches(&t));
    }

    #[test]
    fn test_or_empty_is_false() {
        let t = task("a");
        assert!(!FilterExpr::Or(vec![]).matches(&t));
    }

    #[test]
    fn test_nested_and_or_not() {
        // (status=Incomplete OR status=Completed) AND NOT type=Project
        let mut t = task("a");
        t.status = TaskStatus::Completed;
        t.task_type = TaskType::Todo;
        let expr = FilterExpr::Or(vec![
            FilterExpr::status(TaskStatus::Incomplete),
            FilterExpr::status(TaskStatus::Completed),
        ])
        .and(FilterExpr::task_type(TaskType::Project).not());
        assert!(expr.matches(&t));

        t.task_type = TaskType::Project;
        assert!(!expr.matches(&t));
    }

    #[test]
    fn test_fluent_and_appends_to_existing_and() {
        // Demonstrates the shallow-tree optimization: chaining `.and(...)` on an
        // existing `And` appends rather than nesting.
        let expr = FilterExpr::status(TaskStatus::Incomplete)
            .and(FilterExpr::title_contains("a"))
            .and(FilterExpr::title_contains("b"));
        match expr {
            FilterExpr::And(ref children) => assert_eq!(children.len(), 3),
            _ => panic!("expected flat And"),
        }
    }

    #[test]
    fn test_fluent_or_appends_to_existing_or() {
        let expr = FilterExpr::status(TaskStatus::Incomplete)
            .or(FilterExpr::status(TaskStatus::Completed))
            .or(FilterExpr::status(TaskStatus::Canceled));
        match expr {
            FilterExpr::Or(ref children) => assert_eq!(children.len(), 3),
            _ => panic!("expected flat Or"),
        }
    }

    #[test]
    fn test_serde_roundtrip_full_tree() {
        let project = Uuid::new_v4();
        let original = FilterExpr::And(vec![
            FilterExpr::Or(vec![
                FilterExpr::status(TaskStatus::Incomplete),
                FilterExpr::status(TaskStatus::Completed),
            ]),
            FilterExpr::task_type(TaskType::Project).not(),
            FilterExpr::project(project),
            FilterExpr::title_contains("review"),
        ]);
        let json = serde_json::to_string(&original).unwrap();
        let parsed: FilterExpr = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_serde_tag_layout_uses_op_and_args() {
        // Adjacent tagging on both FilterExpr ("op"+"args") and FilterPredicate
        // ("kind"+"value").
        let expr = FilterExpr::status(TaskStatus::Incomplete);
        let json = serde_json::to_string(&expr).unwrap();
        assert!(json.contains("\"op\":\"pred\""), "missing op tag: {json}");
        assert!(json.contains("\"args\":"), "missing args field: {json}");
        assert!(
            json.contains("\"kind\":\"status\""),
            "missing kind tag: {json}"
        );
        assert!(json.contains("\"value\":"), "missing value field: {json}");
    }
}
