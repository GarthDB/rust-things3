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
use crate::models::{
    BulkCompleteRequest, BulkCreateTasksRequest, BulkDeleteRequest, BulkMoveRequest,
    BulkUpdateDatesRequest, CreateAreaRequest, CreateProjectRequest, CreateTagRequest,
    CreateTaskRequest, TaskStatus, ThingsId, UpdateAreaRequest, UpdateProjectRequest,
    UpdateTagRequest, UpdateTaskRequest,
};

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

// =====================================================================
// Projects (Phase C — #135)
// =====================================================================

/// Build the `make new project` script for a [`CreateProjectRequest`].
///
/// Returns the new project's UUID via `return id of newProject`.
#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn create_project_script(req: &CreateProjectRequest) -> String {
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
        "\t\tset newProject to make new project with properties {{{}}}\n",
        props.join(", "),
    );

    if let Some(date) = req.start_date {
        body.push_str(&assign_date_var("activationDate", date));
        body.push_str("\t\tset activation date of newProject to activationDate\n");
    }
    if let Some(date) = req.deadline {
        body.push_str(&assign_date_var("dueDate", date));
        body.push_str("\t\tset due date of newProject to dueDate\n");
    }

    if let Some(uuid) = &req.area_uuid {
        body.push_str(&format!("\t\tmove newProject to area id \"{uuid}\"\n"));
    }

    body.push_str("\t\treturn id of newProject\n");
    wrap(&body)
}

/// Build the partial-update script for an [`UpdateProjectRequest`].
#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn update_project_script(req: &UpdateProjectRequest) -> String {
    let mut body = format!("\t\tset p to project id \"{}\"\n", req.uuid);

    if let Some(title) = &req.title {
        body.push_str(&format!(
            "\t\tset name of p to {}\n",
            as_applescript_string(title),
        ));
    }
    if let Some(notes) = &req.notes {
        body.push_str(&format!(
            "\t\tset notes of p to {}\n",
            as_applescript_string(notes),
        ));
    }
    if let Some(date) = req.start_date {
        body.push_str(&assign_date_var("activationDate", date));
        body.push_str("\t\tset activation date of p to activationDate\n");
    }
    if let Some(date) = req.deadline {
        body.push_str(&assign_date_var("dueDate", date));
        body.push_str("\t\tset due date of p to dueDate\n");
    }
    if let Some(uuid) = &req.area_uuid {
        body.push_str(&format!("\t\tmove p to area id \"{uuid}\"\n"));
    }
    if let Some(tags) = &req.tags {
        let joined = tags.join(", ");
        body.push_str(&format!(
            "\t\tset tag names of p to {}\n",
            as_applescript_string(&joined),
        ));
    }

    wrap(&body)
}

#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn complete_project_script(id: &ThingsId) -> String {
    wrap(&format!(
        "\t\tset status of project id \"{id}\" to completed\n"
    ))
}

#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn delete_project_script(id: &ThingsId) -> String {
    wrap(&format!("\t\tdelete project id \"{id}\"\n"))
}

/// Build a cascading complete-project script: completes every child task in
/// `child_ids`, then completes the project. Single osascript invocation.
/// Fail-fast — if any sub-statement raises, the project status is left untouched.
#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn cascade_complete_project_script(
    project_id: &ThingsId,
    child_ids: &[ThingsId],
) -> String {
    let mut body = String::new();
    for child in child_ids {
        body.push_str(&format!(
            "\t\tset status of to do id \"{child}\" to completed\n"
        ));
    }
    body.push_str(&format!(
        "\t\tset status of project id \"{project_id}\" to completed\n"
    ));
    wrap(&body)
}

/// Build a cascading delete-project script: deletes every child task in
/// `child_ids`, then deletes the project. Single osascript invocation.
/// Fail-fast — if any sub-statement raises, the project is left untouched.
#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn cascade_delete_project_script(
    project_id: &ThingsId,
    child_ids: &[ThingsId],
) -> String {
    let mut body = String::new();
    for child in child_ids {
        body.push_str(&format!("\t\tdelete to do id \"{child}\"\n"));
    }
    body.push_str(&format!("\t\tdelete project id \"{project_id}\"\n"));
    wrap(&body)
}

/// Build an orphan-then-complete-project script: detaches every child task
/// (`set project to missing value`), then completes the project.
/// Fail-fast — if any sub-statement raises, the project is left untouched.
#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn orphan_complete_project_script(
    project_id: &ThingsId,
    child_ids: &[ThingsId],
) -> String {
    let mut body = String::new();
    for child in child_ids {
        body.push_str(&format!(
            "\t\tset project of to do id \"{child}\" to missing value\n"
        ));
    }
    body.push_str(&format!(
        "\t\tset status of project id \"{project_id}\" to completed\n"
    ));
    wrap(&body)
}

/// Build an orphan-then-delete-project script: detaches every child task,
/// then deletes the project.
/// Fail-fast — if any sub-statement raises, the project is left untouched.
#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn orphan_delete_project_script(
    project_id: &ThingsId,
    child_ids: &[ThingsId],
) -> String {
    let mut body = String::new();
    for child in child_ids {
        body.push_str(&format!(
            "\t\tset project of to do id \"{child}\" to missing value\n"
        ));
    }
    body.push_str(&format!("\t\tdelete project id \"{project_id}\"\n"));
    wrap(&body)
}

// =====================================================================
// Areas (Phase C — #135)
// =====================================================================

/// Build the `make new area` script for a [`CreateAreaRequest`].
///
/// Returns the new area's UUID via `return id of newArea`.
#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn create_area_script(req: &CreateAreaRequest) -> String {
    let body = format!(
        "\t\tset newArea to make new area with properties {{name:{}}}\n\
         \t\treturn id of newArea\n",
        as_applescript_string(&req.title),
    );
    wrap(&body)
}

#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn update_area_script(req: &UpdateAreaRequest) -> String {
    wrap(&format!(
        "\t\tset name of area id \"{}\" to {}\n",
        req.uuid,
        as_applescript_string(&req.title),
    ))
}

#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn delete_area_script(id: &ThingsId) -> String {
    wrap(&format!("\t\tdelete area id \"{id}\"\n"))
}

// =====================================================================
// Bulk operations (Phase C — #135)
// =====================================================================

/// Wrap a sequence of per-item snippets in a try/on-error harness.
///
/// Each snippet runs inside its own `try ... on error ... end try` block. On
/// success, an `okCount` counter increments. On failure, the index and
/// AppleScript error message are appended to an `errorList`. The script
/// returns:
///
/// - `"OK <count>"` if no errors
/// - `"OK <count>\nitem <idx>: <msg>\nitem <idx>: <msg>\n..."` otherwise
///
/// Parsed by [`super::parse::parse_bulk_result`].
fn bulk_wrap(per_item: &[String]) -> String {
    let mut body = String::from("\t\tset okCount to 0\n\t\tset errorList to {}\n");
    for (idx, snippet) in per_item.iter().enumerate() {
        body.push_str("\t\ttry\n");
        body.push_str(snippet);
        body.push_str("\t\t\tset okCount to okCount + 1\n");
        body.push_str("\t\ton error errMsg\n");
        body.push_str(&format!(
            "\t\t\tset end of errorList to \"item {idx}: \" & errMsg\n"
        ));
        body.push_str("\t\tend try\n");
    }
    body.push_str("\t\tif (count of errorList) is 0 then\n");
    body.push_str("\t\t\treturn \"OK \" & okCount\n");
    body.push_str("\t\telse\n");
    body.push_str("\t\t\tset oldDelims to AppleScript's text item delimiters\n");
    body.push_str("\t\t\tset AppleScript's text item delimiters to linefeed\n");
    body.push_str("\t\t\tset output to \"OK \" & okCount & linefeed & (errorList as string)\n");
    body.push_str("\t\t\tset AppleScript's text item delimiters to oldDelims\n");
    body.push_str("\t\t\treturn output\n");
    body.push_str("\t\tend if\n");
    wrap(&body)
}

/// Render the property-list + post-statements for a single `make new to do`
/// inside a bulk script. Indented one extra level (`\t\t\t`) to sit inside
/// the `try` block emitted by [`bulk_wrap`].
fn create_task_snippet(req: &CreateTaskRequest) -> String {
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

    let mut snippet = format!(
        "\t\t\tset newTask to make new to do with properties {{{}}}\n",
        props.join(", "),
    );

    if let Some(date) = req.start_date {
        snippet.push_str(&assign_date_var_indented("activationDate", date, 3));
        snippet.push_str("\t\t\tset activation date of newTask to activationDate\n");
    }
    if let Some(date) = req.deadline {
        snippet.push_str(&assign_date_var_indented("dueDate", date, 3));
        snippet.push_str("\t\t\tset due date of newTask to dueDate\n");
    }

    if let Some(uuid) = &req.project_uuid {
        snippet.push_str(&format!("\t\t\tmove newTask to project id \"{uuid}\"\n"));
    } else if let Some(uuid) = &req.area_uuid {
        snippet.push_str(&format!("\t\t\tmove newTask to area id \"{uuid}\"\n"));
    } else if let Some(uuid) = &req.parent_uuid {
        snippet.push_str(&format!("\t\t\tmove newTask to to do id \"{uuid}\"\n"));
    }

    if let Some(status) = req.status {
        snippet.push_str(&format!(
            "\t\t\tset status of newTask to {}\n",
            status_as_applescript(status),
        ));
    }
    snippet
}

/// Indented variant of [`assign_date_var`] for use inside bulk try-blocks.
fn assign_date_var_indented(var: &str, date: NaiveDate, level: usize) -> String {
    let tabs = "\t".repeat(level);
    format!(
        "{tabs}set {var} to current date\n\
         {tabs}set day of {var} to 1\n\
         {tabs}set year of {var} to {year}\n\
         {tabs}set month of {var} to {month}\n\
         {tabs}set day of {var} to {day}\n\
         {tabs}set time of {var} to 0\n",
        year = date.year(),
        month = date.month(),
        day = date.day(),
    )
}

#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn bulk_create_tasks_script(req: &BulkCreateTasksRequest) -> String {
    let snippets: Vec<String> = req.tasks.iter().map(create_task_snippet).collect();
    bulk_wrap(&snippets)
}

#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn bulk_delete_script(req: &BulkDeleteRequest) -> String {
    let snippets: Vec<String> = req
        .task_uuids
        .iter()
        .map(|id| format!("\t\t\tdelete to do id \"{id}\"\n"))
        .collect();
    bulk_wrap(&snippets)
}

#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn bulk_complete_script(req: &BulkCompleteRequest) -> String {
    let snippets: Vec<String> = req
        .task_uuids
        .iter()
        .map(|id| format!("\t\t\tset status of to do id \"{id}\" to completed\n"))
        .collect();
    bulk_wrap(&snippets)
}

#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn bulk_move_script(req: &BulkMoveRequest) -> String {
    let dest = if let Some(uuid) = &req.project_uuid {
        format!("project id \"{uuid}\"")
    } else if let Some(uuid) = &req.area_uuid {
        format!("area id \"{uuid}\"")
    } else {
        unreachable!(
            "bulk_move_script called without project_uuid or area_uuid; \
             bulk_move() must validate before constructing the script"
        );
    };
    let snippets: Vec<String> = req
        .task_uuids
        .iter()
        .map(|id| format!("\t\t\tmove to do id \"{id}\" to {dest}\n"))
        .collect();
    bulk_wrap(&snippets)
}

#[allow(dead_code)] // Used by AppleScriptBackend, added in #135.
pub(crate) fn bulk_update_dates_script(req: &BulkUpdateDatesRequest) -> String {
    // Build the per-item snippet once: it depends only on the request, not the id.
    // Date variables are assigned once per try-block; per-item set statements reference them.
    let snippets: Vec<String> = req
        .task_uuids
        .iter()
        .map(|id| {
            let mut snippet = format!("\t\t\tset t to to do id \"{id}\"\n");
            if let Some(date) = req.start_date {
                snippet.push_str(&assign_date_var_indented("activationDate", date, 3));
                snippet.push_str("\t\t\tset activation date of t to activationDate\n");
            } else if req.clear_start_date {
                snippet.push_str("\t\t\tset activation date of t to missing value\n");
            }
            if let Some(date) = req.deadline {
                snippet.push_str(&assign_date_var_indented("dueDate", date, 3));
                snippet.push_str("\t\t\tset due date of t to dueDate\n");
            } else if req.clear_deadline {
                snippet.push_str("\t\t\tset due date of t to missing value\n");
            }
            snippet
        })
        .collect();
    bulk_wrap(&snippets)
}

// =====================================================================
// Tags (Phase D — #136)
// =====================================================================

/// Build the `make new tag` script for a [`CreateTagRequest`].
///
/// Returns the new tag's UUID via `return id of newTag`.
///
/// **Note:** Things AppleScript does not expose `shortcut` or `parent`
/// properties on `tag`, so [`CreateTagRequest::shortcut`] and
/// [`CreateTagRequest::parent_uuid`] are silently dropped here. The caller
/// is responsible for `tracing::debug!`-logging that drop. This is a known
/// divergence from `SqlxBackend` — see Phase D PR notes (#136).
#[allow(dead_code)] // Used by AppleScriptBackend, added in #136.
pub(crate) fn create_tag_script(req: &CreateTagRequest) -> String {
    let body = format!(
        "\t\tset newTag to make new tag with properties {{name:{}}}\n\
         \t\treturn id of newTag\n",
        as_applescript_string(&req.title),
    );
    wrap(&body)
}

/// Build the partial-update script for an [`UpdateTagRequest`].
///
/// Same `shortcut` / `parent_uuid` caveat as [`create_tag_script`] — those
/// fields are silently ignored.
#[allow(dead_code)] // Used by AppleScriptBackend, added in #136.
pub(crate) fn update_tag_script(req: &UpdateTagRequest) -> String {
    let mut body = format!("\t\tset t to tag id \"{}\"\n", req.uuid);
    if let Some(title) = &req.title {
        body.push_str(&format!(
            "\t\tset name of t to {}\n",
            as_applescript_string(title),
        ));
    }
    wrap(&body)
}

#[allow(dead_code)] // Used by AppleScriptBackend, added in #136.
pub(crate) fn delete_tag_script(id: &ThingsId) -> String {
    wrap(&format!("\t\tdelete tag id \"{id}\"\n"))
}

/// Set a task's `tag names` property to the provided comma-joined string.
///
/// Things AS treats `tag names` as a single text value: a comma-separated
/// list that Things parses internally. Caller is responsible for joining
/// titles with `", "` and for any deduplication.
#[allow(dead_code)] // Used by AppleScriptBackend, added in #136.
pub(crate) fn set_task_tag_names_script(task_id: &ThingsId, joined: &str) -> String {
    wrap(&format!(
        "\t\tset tag names of to do id \"{task_id}\" to {}\n",
        as_applescript_string(joined),
    ))
}

/// Bulk variant of [`set_task_tag_names_script`] — one osascript invocation
/// rewrites tag names for many tasks. Each item runs in its own try block
/// via [`bulk_wrap`]. Used by `merge_tags` and `delete_tag(remove_from_tasks=true)`.
#[allow(dead_code)] // Used by AppleScriptBackend, added in #136.
pub(crate) fn bulk_set_task_tag_names_script(items: &[(ThingsId, String)]) -> String {
    let snippets: Vec<String> = items
        .iter()
        .map(|(task_id, joined)| {
            format!(
                "\t\t\tset tag names of to do id \"{task_id}\" to {}\n",
                as_applescript_string(joined),
            )
        })
        .collect();
    bulk_wrap(&snippets)
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

    // -----------------------------------------------------------------
    // Phase C — Projects
    // -----------------------------------------------------------------

    #[test]
    fn create_project_minimal() {
        let req = CreateProjectRequest {
            title: "Launch".into(),
            notes: None,
            area_uuid: None,
            start_date: None,
            deadline: None,
            tags: None,
        };
        let script = create_project_script(&req);
        assert!(script.contains("make new project with properties {name:\"Launch\"}"));
        assert!(script.contains("return id of newProject"));
        assert!(!script.contains("move newProject"));
    }

    #[test]
    fn create_project_with_area_emits_move() {
        let req = CreateProjectRequest {
            title: "x".into(),
            notes: Some("notes\nwith newline".into()),
            area_uuid: Some(project_uuid()),
            start_date: Some(date(2026, 7, 4)),
            deadline: None,
            tags: Some(vec!["ops".into(), "urgent".into()]),
        };
        let script = create_project_script(&req);
        assert!(script.contains("notes:\"notes\\nwith newline\""));
        assert!(script.contains("tag names:\"ops, urgent\""));
        assert!(script.contains(&format!(
            "move newProject to area id \"{}\"",
            project_uuid()
        )));
        assert!(script.contains("set activation date of newProject to activationDate"));
    }

    #[test]
    fn update_project_emits_only_specified_fields() {
        let req = UpdateProjectRequest {
            uuid: sample_uuid(),
            title: Some("renamed".into()),
            notes: None,
            area_uuid: None,
            start_date: None,
            deadline: Some(date(2026, 12, 31)),
            tags: None,
        };
        let script = update_project_script(&req);
        assert!(script.contains(&format!("set p to project id \"{}\"", sample_uuid())));
        assert!(script.contains("set name of p to \"renamed\""));
        assert!(script.contains("set due date of p to dueDate"));
        assert!(!script.contains("set notes"));
        assert!(!script.contains("set tag names"));
    }

    #[test]
    fn complete_project_script_shape() {
        let script = complete_project_script(&sample_uuid());
        assert!(script.contains(&format!(
            "set status of project id \"{}\" to completed",
            sample_uuid()
        )));
    }

    #[test]
    fn delete_project_script_shape() {
        let script = delete_project_script(&sample_uuid());
        assert!(script.contains(&format!("delete project id \"{}\"", sample_uuid())));
    }

    #[test]
    fn cascade_complete_project_includes_each_child_and_parent() {
        let project = sample_uuid();
        let children = vec![project_uuid(), ThingsId::from_trusted("abc-123".into())];
        let script = cascade_complete_project_script(&project, &children);
        for child in &children {
            assert!(script.contains(&format!("set status of to do id \"{child}\" to completed")));
        }
        assert!(script.contains(&format!(
            "set status of project id \"{project}\" to completed"
        )));
    }

    #[test]
    fn cascade_delete_project_includes_each_child_and_parent() {
        let project = sample_uuid();
        let children = vec![project_uuid()];
        let script = cascade_delete_project_script(&project, &children);
        assert!(script.contains(&format!("delete to do id \"{}\"", project_uuid())));
        assert!(script.contains(&format!("delete project id \"{project}\"")));
        // Order matters: children deleted before parent.
        let child_pos = script.find("delete to do id").unwrap();
        let parent_pos = script.find("delete project id").unwrap();
        assert!(child_pos < parent_pos);
    }

    #[test]
    fn orphan_complete_project_uses_missing_value_for_children() {
        let project = sample_uuid();
        let children = vec![project_uuid()];
        let script = orphan_complete_project_script(&project, &children);
        assert!(script.contains(&format!(
            "set project of to do id \"{}\" to missing value",
            project_uuid()
        )));
        assert!(script.contains(&format!(
            "set status of project id \"{project}\" to completed"
        )));
    }

    #[test]
    fn orphan_delete_project_uses_missing_value_for_children() {
        let project = sample_uuid();
        let children = vec![project_uuid()];
        let script = orphan_delete_project_script(&project, &children);
        assert!(script.contains(&format!(
            "set project of to do id \"{}\" to missing value",
            project_uuid()
        )));
        assert!(script.contains(&format!("delete project id \"{project}\"")));
    }

    // -----------------------------------------------------------------
    // Phase C — Areas
    // -----------------------------------------------------------------

    #[test]
    fn create_area_script_returns_id() {
        let script = create_area_script(&CreateAreaRequest {
            title: "Personal \"life\"".into(),
        });
        assert!(script.contains("make new area with properties {name:\"Personal \\\"life\\\"\"}"));
        assert!(script.contains("return id of newArea"));
    }

    #[test]
    fn update_area_renames() {
        let script = update_area_script(&UpdateAreaRequest {
            uuid: sample_uuid(),
            title: "New name".into(),
        });
        assert!(script.contains(&format!(
            "set name of area id \"{}\" to \"New name\"",
            sample_uuid()
        )));
    }

    #[test]
    fn delete_area_script_shape() {
        let script = delete_area_script(&sample_uuid());
        assert!(script.contains(&format!("delete area id \"{}\"", sample_uuid())));
    }

    // -----------------------------------------------------------------
    // Phase C — Bulk operations
    // -----------------------------------------------------------------

    fn task(title: &str) -> CreateTaskRequest {
        CreateTaskRequest {
            title: title.into(),
            task_type: None,
            notes: None,
            start_date: None,
            deadline: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: None,
            status: None,
        }
    }

    #[test]
    fn bulk_create_tasks_wraps_each_in_try_block() {
        let req = BulkCreateTasksRequest {
            tasks: vec![task("a"), task("b")],
        };
        let script = bulk_create_tasks_script(&req);
        // Each item gets its own try block. Match "\t\ttry\n" specifically so we
        // don't double-count the "try\n" suffix of "end try\n".
        assert_eq!(script.matches("\t\ttry\n").count(), 2);
        assert_eq!(script.matches("on error errMsg").count(), 2);
        assert_eq!(script.matches("end try").count(), 2);
        // Per-item error tagging.
        assert!(script.contains("\"item 0: \" & errMsg"));
        assert!(script.contains("\"item 1: \" & errMsg"));
        // Counter + result formatting.
        assert!(script.contains("set okCount to 0"));
        assert!(script.contains("set errorList to {}"));
        assert!(script.contains("return \"OK \" & okCount"));
        // Both task names present.
        assert!(script.contains("name:\"a\""));
        assert!(script.contains("name:\"b\""));
    }

    #[test]
    fn bulk_delete_one_per_item() {
        let req = BulkDeleteRequest {
            task_uuids: vec![sample_uuid(), project_uuid()],
        };
        let script = bulk_delete_script(&req);
        assert!(script.contains(&format!("delete to do id \"{}\"", sample_uuid())));
        assert!(script.contains(&format!("delete to do id \"{}\"", project_uuid())));
        assert_eq!(script.matches("\t\ttry\n").count(), 2);
        assert_eq!(script.matches("on error errMsg").count(), 2);
    }

    #[test]
    fn bulk_complete_one_per_item() {
        let req = BulkCompleteRequest {
            task_uuids: vec![sample_uuid()],
        };
        let script = bulk_complete_script(&req);
        assert!(script.contains(&format!(
            "set status of to do id \"{}\" to completed",
            sample_uuid()
        )));
    }

    #[test]
    fn bulk_move_emits_project_destination() {
        let req = BulkMoveRequest {
            task_uuids: vec![sample_uuid()],
            project_uuid: Some(project_uuid()),
            area_uuid: None,
        };
        let script = bulk_move_script(&req);
        assert!(script.contains(&format!(
            "move to do id \"{}\" to project id \"{}\"",
            sample_uuid(),
            project_uuid()
        )));
    }

    #[test]
    fn bulk_move_prefers_project_over_area_when_both_set() {
        let req = BulkMoveRequest {
            task_uuids: vec![sample_uuid()],
            project_uuid: Some(project_uuid()),
            area_uuid: Some(sample_uuid()),
        };
        let script = bulk_move_script(&req);
        assert!(script.contains("to project id"));
        assert!(!script.contains("to area id"));
    }

    #[test]
    fn bulk_move_emits_area_destination_when_project_unset() {
        let req = BulkMoveRequest {
            task_uuids: vec![sample_uuid()],
            project_uuid: None,
            area_uuid: Some(project_uuid()),
        };
        let script = bulk_move_script(&req);
        assert!(script.contains(&format!(
            "move to do id \"{}\" to area id \"{}\"",
            sample_uuid(),
            project_uuid()
        )));
    }

    #[test]
    #[should_panic(expected = "bulk_move_script called without project_uuid or area_uuid")]
    fn bulk_move_without_destination_panics() {
        let req = BulkMoveRequest {
            task_uuids: vec![sample_uuid()],
            project_uuid: None,
            area_uuid: None,
        };
        let _ = bulk_move_script(&req);
    }

    #[test]
    fn bulk_update_dates_with_clears() {
        let req = BulkUpdateDatesRequest {
            task_uuids: vec![sample_uuid()],
            start_date: None,
            deadline: None,
            clear_start_date: true,
            clear_deadline: true,
        };
        let script = bulk_update_dates_script(&req);
        assert!(script.contains("set activation date of t to missing value"));
        assert!(script.contains("set due date of t to missing value"));
    }

    #[test]
    fn bulk_update_dates_with_values() {
        let req = BulkUpdateDatesRequest {
            task_uuids: vec![sample_uuid()],
            start_date: Some(date(2026, 6, 1)),
            deadline: Some(date(2026, 7, 1)),
            clear_start_date: false,
            clear_deadline: false,
        };
        let script = bulk_update_dates_script(&req);
        assert!(script.contains("set activation date of t to activationDate"));
        assert!(script.contains("set due date of t to dueDate"));
    }

    // -----------------------------------------------------------------
    // Phase D — Tags
    // -----------------------------------------------------------------

    #[test]
    fn create_tag_emits_make_new_with_name_only() {
        let req = CreateTagRequest {
            title: "Work".into(),
            shortcut: Some("w".into()),
            parent_uuid: Some(sample_uuid()),
        };
        let script = create_tag_script(&req);
        assert!(script.contains("make new tag with properties {name:\"Work\"}"));
        assert!(script.contains("return id of newTag"));
        // shortcut and parent are intentionally dropped — Things AS does not
        // expose them. The Rust backend logs the drop at debug level.
        assert!(!script.contains("shortcut"));
        assert!(!script.contains("parent"));
    }

    #[test]
    fn create_tag_escapes_title() {
        let req = CreateTagRequest {
            title: "Has \"quotes\" and\nnewline".into(),
            shortcut: None,
            parent_uuid: None,
        };
        let script = create_tag_script(&req);
        assert!(script.contains("name:\"Has \\\"quotes\\\" and\\nnewline\""));
    }

    #[test]
    fn update_tag_no_fields_only_resolves_target() {
        let req = UpdateTagRequest {
            uuid: sample_uuid(),
            title: None,
            shortcut: Some("w".into()),
            parent_uuid: Some(project_uuid()),
        };
        let script = update_tag_script(&req);
        assert!(script.contains(&format!("set t to tag id \"{}\"", sample_uuid())));
        assert!(!script.contains("set name"));
        // shortcut/parent silently dropped
        assert!(!script.contains("shortcut"));
        assert!(!script.contains("parent"));
    }

    #[test]
    fn update_tag_renames_when_title_set() {
        let req = UpdateTagRequest {
            uuid: sample_uuid(),
            title: Some("Renamed".into()),
            shortcut: None,
            parent_uuid: None,
        };
        let script = update_tag_script(&req);
        assert!(script.contains("set name of t to \"Renamed\""));
    }

    #[test]
    fn delete_tag_script_shape() {
        let script = delete_tag_script(&sample_uuid());
        assert!(script.contains(&format!("delete tag id \"{}\"", sample_uuid())));
    }

    #[test]
    fn set_task_tag_names_emits_set_with_joined_string() {
        let script = set_task_tag_names_script(&sample_uuid(), "work, urgent");
        assert!(script.contains(&format!(
            "set tag names of to do id \"{}\" to \"work, urgent\"",
            sample_uuid()
        )));
    }

    #[test]
    fn set_task_tag_names_escapes_joined_string() {
        let script = set_task_tag_names_script(&sample_uuid(), "has \"quote\"");
        assert!(script.contains("\"has \\\"quote\\\"\""));
    }

    #[test]
    fn bulk_set_task_tag_names_wraps_each_in_try_block() {
        let items = vec![
            (sample_uuid(), "a, b".to_string()),
            (project_uuid(), "c".to_string()),
        ];
        let script = bulk_set_task_tag_names_script(&items);
        // Each item gets its own try block.
        assert_eq!(script.matches("\t\ttry\n").count(), 2);
        assert_eq!(script.matches("on error errMsg").count(), 2);
        assert_eq!(script.matches("end try").count(), 2);
        assert!(script.contains(&format!(
            "set tag names of to do id \"{}\" to \"a, b\"",
            sample_uuid()
        )));
        assert!(script.contains(&format!(
            "set tag names of to do id \"{}\" to \"c\"",
            project_uuid()
        )));
        // Per-item error tagging.
        assert!(script.contains("\"item 0: \" & errMsg"));
        assert!(script.contains("\"item 1: \" & errMsg"));
    }

    #[test]
    fn bulk_set_task_tag_names_empty_items_still_returns_ok() {
        let script = bulk_set_task_tag_names_script(&[]);
        assert!(script.contains("set okCount to 0"));
        assert!(script.contains("return \"OK \" & okCount"));
        assert_eq!(script.matches("\t\ttry\n").count(), 0);
    }
}
