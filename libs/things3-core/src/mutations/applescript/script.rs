//! Pure AppleScript builders for the task operations.
//!
//! Every function here is a referentially-transparent transform from typed
//! input to AppleScript text. They do not spawn `osascript`; that is the
//! [`super::runner`] module's job. Splitting the two means the bulk of the
//! backend's logic is unit-testable on any platform without a Things 3 install.
//!
//! ## Conventions used by every script
//!
//! - Wrapped in `tell application "Things3" … end tell` so all references
//!   resolve against Things.
//! - Wrapped in `with timeout of 600 seconds … end timeout`. The default
//!   Apple Event timeout is ~120 s; Things mutations against a busy database
//!   can stall longer than that. Cheap insurance against spurious timeouts.
//! - Every interpolated user-controlled string flows through
//!   [`super::escape::as_applescript_string`]. UUIDs come from the typed
//!   [`Uuid`] which only round-trips hex+hyphens, so they are interpolated
//!   directly without escaping.
//! - Date components are assigned individually (`day` first set to 1 to dodge
//!   the May-31 → April overflow trap) instead of via the locale-sensitive
//!   `date "…"` literal.

use chrono::{Datelike, NaiveDate};

use super::escape::as_applescript_string;
use crate::models::{CreateTaskRequest, TaskStatus, ThingsId, UpdateTaskRequest};

/// Wrap a script body in the standard `tell application` + `with timeout`
/// envelope. `body` is appended verbatim — caller controls indentation.
fn wrap(body: &str) -> String {
    format!(
        "tell application \"Things3\"\n\
         \twith timeout of 600 seconds\n\
         {body}\
         \tend timeout\n\
         end tell\n"
    )
}

/// AppleScript snippet that assigns variable `var` to a `date` object equal to
/// `date` at midnight. Caller is responsible for the surrounding `tell` block.
///
/// The `set day of … to 1` line is deliberate: setting `month` first when the
/// current `day` is 31 (and the target month has 30 or fewer days) produces
/// silent date overflow. Setting day to 1 sidesteps that.
fn assign_date_var(var: &str, date: NaiveDate) -> String {
    format!(
        "\t\tset {var} to current date\n\
         \t\tset day of {var} to 1\n\
         \t\tset year of {var} to {year}\n\
         \t\tset month of {var} to {month}\n\
         \t\tset day of {var} to {day}\n\
         \t\tset time of {var} to 0\n",
        year = date.year(),
        month = date.month(),
        day = date.day(),
    )
}

/// Map a [`TaskStatus`] to its AppleScript constant name.
///
/// Things AS exposes `open | completed | canceled` as the `status` enum.
/// `Trashed` has no direct equivalent — the proper way to trash a task is
/// `delete_task`, which calls a different script — so we map it to `canceled`
/// as the closest finished state.
fn status_as_applescript(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Incomplete => "open",
        TaskStatus::Completed => "completed",
        TaskStatus::Canceled | TaskStatus::Trashed => "canceled",
    }
}

/// Build the `make new to do` script for a [`CreateTaskRequest`].
///
/// Returns the new task's UUID via `return id of newTask`.
#[allow(dead_code)] // Used by AppleScriptBackend, added in #134.
pub(crate) fn create_task_script(req: &CreateTaskRequest) -> String {
    let mut props = vec![format!("name:{}", as_applescript_string(&req.title))];
    if let Some(notes) = &req.notes {
        props.push(format!("notes:{}", as_applescript_string(notes)));
    }
    if let Some(tags) = &req.tags {
        if !tags.is_empty() {
            let joined = tags.join(", ");
            props.push(format!("tag names:{}", as_applescript_string(&joined)));
        }
    }

    let mut body = format!(
        "\t\tset newTask to make new to do with properties {{{}}}\n",
        props.join(", "),
    );

    if let Some(date) = req.start_date {
        body.push_str(&assign_date_var("activationDate", date));
        body.push_str("\t\tset activation date of newTask to activationDate\n");
    }
    if let Some(date) = req.deadline {
        body.push_str(&assign_date_var("dueDate", date));
        body.push_str("\t\tset due date of newTask to dueDate\n");
    }

    // Container precedence matches the sqlx backend: project > area > parent.
    if let Some(uuid) = &req.project_uuid {
        body.push_str(&format!("\t\tmove newTask to project id \"{uuid}\"\n"));
    } else if let Some(uuid) = &req.area_uuid {
        body.push_str(&format!("\t\tmove newTask to area id \"{uuid}\"\n"));
    } else if let Some(uuid) = &req.parent_uuid {
        body.push_str(&format!("\t\tmove newTask to to do id \"{uuid}\"\n"));
    }

    if let Some(status) = req.status {
        body.push_str(&format!(
            "\t\tset status of newTask to {}\n",
            status_as_applescript(status),
        ));
    }

    body.push_str("\t\treturn id of newTask\n");
    wrap(&body)
}

/// Build the partial-update script for an [`UpdateTaskRequest`].
///
/// Only emits `set` lines for fields where the corresponding `Option` is
/// `Some` — `None` means "leave unchanged".
#[allow(dead_code)] // Used by AppleScriptBackend, added in #134.
pub(crate) fn update_task_script(req: &UpdateTaskRequest) -> String {
    let mut body = format!("\t\tset t to to do id \"{}\"\n", req.uuid);

    if let Some(title) = &req.title {
        body.push_str(&format!(
            "\t\tset name of t to {}\n",
            as_applescript_string(title),
        ));
    }
    if let Some(notes) = &req.notes {
        body.push_str(&format!(
            "\t\tset notes of t to {}\n",
            as_applescript_string(notes),
        ));
    }
    if let Some(date) = req.start_date {
        body.push_str(&assign_date_var("activationDate", date));
        body.push_str("\t\tset activation date of t to activationDate\n");
    }
    if let Some(date) = req.deadline {
        body.push_str(&assign_date_var("dueDate", date));
        body.push_str("\t\tset due date of t to dueDate\n");
    }
    if let Some(status) = req.status {
        body.push_str(&format!(
            "\t\tset status of t to {}\n",
            status_as_applescript(status),
        ));
    }
    if let Some(uuid) = &req.project_uuid {
        body.push_str(&format!("\t\tmove t to project id \"{uuid}\"\n"));
    } else if let Some(uuid) = &req.area_uuid {
        body.push_str(&format!("\t\tmove t to area id \"{uuid}\"\n"));
    }
    if let Some(tags) = &req.tags {
        let joined = tags.join(", ");
        body.push_str(&format!(
            "\t\tset tag names of t to {}\n",
            as_applescript_string(&joined),
        ));
    }

    wrap(&body)
}

#[allow(dead_code)] // Used by AppleScriptBackend, added in #134.
pub(crate) fn complete_task_script(id: &ThingsId) -> String {
    wrap(&format!(
        "\t\tset status of to do id \"{id}\" to completed\n"
    ))
}

#[allow(dead_code)] // Used by AppleScriptBackend, added in #134.
pub(crate) fn uncomplete_task_script(id: &ThingsId) -> String {
    wrap(&format!("\t\tset status of to do id \"{id}\" to open\n"))
}

#[allow(dead_code)] // Used by AppleScriptBackend, added in #134.
pub(crate) fn delete_task_script(id: &ThingsId) -> String {
    wrap(&format!("\t\tdelete to do id \"{id}\"\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_uuid() -> ThingsId {
        ThingsId::from_trusted("9d3f1e44-5c2a-4b8e-9c1f-7e2d8a4b3c5e".to_string())
    }

    fn project_uuid() -> ThingsId {
        ThingsId::from_trusted("11111111-2222-3333-4444-555555555555".to_string())
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn create_task_minimal_title_only() {
        let req = CreateTaskRequest {
            title: "Buy milk".into(),
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
        let script = create_task_script(&req);
        assert!(script.starts_with("tell application \"Things3\""));
        assert!(script.contains("with timeout of 600 seconds"));
        assert!(script.contains("make new to do with properties {name:\"Buy milk\"}"));
        assert!(script.contains("return id of newTask"));
        assert!(script.ends_with("end tell\n"));
        // No date / move / status lines.
        assert!(!script.contains("activation date"));
        assert!(!script.contains("due date"));
        assert!(!script.contains("move newTask"));
        assert!(!script.contains("set status"));
    }

    #[test]
    fn create_task_escapes_title_and_notes() {
        let req = CreateTaskRequest {
            title: "Buy \"organic\" milk".into(),
            task_type: None,
            notes: Some("Has\nnewline and \\ backslash".into()),
            start_date: None,
            deadline: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: None,
            status: None,
        };
        let script = create_task_script(&req);
        assert!(script.contains("name:\"Buy \\\"organic\\\" milk\""));
        assert!(script.contains("notes:\"Has\\nnewline and \\\\ backslash\""));
    }

    #[test]
    fn create_task_with_tags_joins_with_comma() {
        let req = CreateTaskRequest {
            title: "x".into(),
            task_type: None,
            notes: None,
            start_date: None,
            deadline: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: Some(vec!["work".into(), "urgent".into()]),
            status: None,
        };
        let script = create_task_script(&req);
        assert!(script.contains("tag names:\"work, urgent\""));
    }

    #[test]
    fn create_task_with_empty_tags_omits_property() {
        let req = CreateTaskRequest {
            title: "x".into(),
            task_type: None,
            notes: None,
            start_date: None,
            deadline: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: Some(vec![]),
            status: None,
        };
        let script = create_task_script(&req);
        assert!(!script.contains("tag names"));
    }

    #[test]
    fn create_task_with_dates_sets_components_locale_independently() {
        let req = CreateTaskRequest {
            title: "x".into(),
            task_type: None,
            notes: None,
            start_date: Some(date(2026, 4, 15)),
            deadline: Some(date(2026, 5, 1)),
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: None,
            status: None,
        };
        let script = create_task_script(&req);
        // activation date setup
        assert!(script.contains("set activationDate to current date"));
        assert!(script.contains("set day of activationDate to 1"));
        assert!(script.contains("set year of activationDate to 2026"));
        assert!(script.contains("set month of activationDate to 4"));
        assert!(script.contains("set day of activationDate to 15"));
        assert!(script.contains("set activation date of newTask to activationDate"));
        // due date setup
        assert!(script.contains("set dueDate to current date"));
        assert!(script.contains("set month of dueDate to 5"));
        assert!(script.contains("set day of dueDate to 1"));
        assert!(script.contains("set due date of newTask to dueDate"));
    }

    #[test]
    fn create_task_with_project_emits_move() {
        let req = CreateTaskRequest {
            title: "x".into(),
            task_type: None,
            notes: None,
            start_date: None,
            deadline: None,
            project_uuid: Some(project_uuid()),
            area_uuid: None,
            parent_uuid: None,
            tags: None,
            status: None,
        };
        let script = create_task_script(&req);
        assert!(script.contains(&format!(
            "move newTask to project id \"{}\"",
            project_uuid()
        )));
    }

    #[test]
    fn create_task_project_takes_precedence_over_area() {
        let req = CreateTaskRequest {
            title: "x".into(),
            task_type: None,
            notes: None,
            start_date: None,
            deadline: None,
            project_uuid: Some(project_uuid()),
            area_uuid: Some(sample_uuid()),
            parent_uuid: None,
            tags: None,
            status: None,
        };
        let script = create_task_script(&req);
        assert!(script.contains("move newTask to project id"));
        assert!(!script.contains("move newTask to area id"));
    }

    #[test]
    fn create_task_with_status_emits_set_status() {
        let req = CreateTaskRequest {
            title: "x".into(),
            task_type: None,
            notes: None,
            start_date: None,
            deadline: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: None,
            status: Some(TaskStatus::Completed),
        };
        let script = create_task_script(&req);
        assert!(script.contains("set status of newTask to completed"));
    }

    #[test]
    fn update_task_no_fields_only_resolves_target() {
        let req = UpdateTaskRequest {
            uuid: sample_uuid(),
            title: None,
            notes: None,
            start_date: None,
            deadline: None,
            status: None,
            project_uuid: None,
            area_uuid: None,
            tags: None,
        };
        let script = update_task_script(&req);
        assert!(script.contains(&format!("set t to to do id \"{}\"", sample_uuid())));
        assert!(!script.contains("set name"));
        assert!(!script.contains("set notes"));
        assert!(!script.contains("set status"));
        assert!(!script.contains("move t"));
        assert!(!script.contains("set tag names"));
    }

    #[test]
    fn update_task_emits_only_specified_fields() {
        let req = UpdateTaskRequest {
            uuid: sample_uuid(),
            title: Some("renamed".into()),
            notes: None,
            start_date: None,
            deadline: None,
            status: Some(TaskStatus::Canceled),
            project_uuid: None,
            area_uuid: None,
            tags: Some(vec!["a".into(), "b".into()]),
        };
        let script = update_task_script(&req);
        assert!(script.contains("set name of t to \"renamed\""));
        assert!(script.contains("set status of t to canceled"));
        assert!(script.contains("set tag names of t to \"a, b\""));
        assert!(!script.contains("set notes"));
        assert!(!script.contains("activation date"));
        assert!(!script.contains("due date"));
    }

    #[test]
    fn update_task_trashed_status_maps_to_canceled() {
        let req = UpdateTaskRequest {
            uuid: sample_uuid(),
            title: None,
            notes: None,
            start_date: None,
            deadline: None,
            status: Some(TaskStatus::Trashed),
            project_uuid: None,
            area_uuid: None,
            tags: None,
        };
        let script = update_task_script(&req);
        assert!(script.contains("set status of t to canceled"));
    }

    #[test]
    fn complete_task_script_shape() {
        let script = complete_task_script(&sample_uuid());
        assert!(script.contains(&format!(
            "set status of to do id \"{}\" to completed",
            sample_uuid()
        )));
    }

    #[test]
    fn uncomplete_task_script_shape() {
        let script = uncomplete_task_script(&sample_uuid());
        assert!(script.contains(&format!(
            "set status of to do id \"{}\" to open",
            sample_uuid()
        )));
    }

    #[test]
    fn delete_task_script_shape() {
        let script = delete_task_script(&sample_uuid());
        assert!(script.contains(&format!("delete to do id \"{}\"", sample_uuid())));
    }

    #[test]
    fn all_scripts_wrapped_in_timeout() {
        let id = sample_uuid();
        for script in [
            complete_task_script(&id),
            uncomplete_task_script(&id),
            delete_task_script(&id),
        ] {
            assert!(
                script.contains("with timeout of 600 seconds"),
                "script was: {script}"
            );
            assert!(script.contains("end timeout"), "script was: {script}");
            assert!(script.starts_with("tell application \"Things3\""));
            assert!(script.ends_with("end tell\n"));
        }
    }

    #[test]
    fn assign_date_var_sets_day_to_1_first_to_avoid_overflow() {
        let snippet = assign_date_var("d", date(2026, 4, 15));
        let day1_pos = snippet.find("set day of d to 1").unwrap();
        let month_pos = snippet.find("set month of d to 4").unwrap();
        let final_day_pos = snippet.find("set day of d to 15").unwrap();
        assert!(day1_pos < month_pos, "day=1 must precede month assignment");
        assert!(
            month_pos < final_day_pos,
            "final day assignment must come last"
        );
    }
}
