#[cfg(feature = "export-taskpaper")]
use crate::models::{Task, TaskStatus, TaskType};
#[cfg(feature = "export-taskpaper")]
use std::fmt::Write;

#[cfg(feature = "export-taskpaper")]
use super::ExportData;

#[cfg(feature = "export-taskpaper")]
pub(super) fn export_taskpaper(data: &ExportData) -> String {
    let mut out = String::new();

    // --- Areas → their projects → tasks ---
    for area in &data.areas {
        let area_meta =
            taskpaper_metadata(TaskStatus::Incomplete, None, None, None, &area.tags);
        writeln!(out, "{}:{area_meta}", escape_taskpaper_title(&area.title)).unwrap();

        let area_projects: Vec<&crate::models::Project> = data
            .projects
            .iter()
            .filter(|p| p.area_uuid.as_ref() == Some(&area.uuid))
            .collect();

        for project in &area_projects {
            let meta = taskpaper_metadata(
                project.status,
                None,
                project.deadline,
                project.start_date,
                &project.tags,
            );
            writeln!(out, "\t{}:{meta}", escape_taskpaper_title(&project.title)).unwrap();
            if let Some(notes) = &project.notes {
                write_taskpaper_notes(&mut out, notes, 2);
            }
            for task in data.tasks.iter().filter(|t| {
                t.project_uuid.as_ref() == Some(&project.uuid) && t.parent_uuid.is_none()
            }) {
                write_taskpaper_task(&mut out, task, 2, &data.tasks);
            }
        }
        writeln!(out).unwrap();
    }

    // --- Orphan projects (no area) ---
    for project in data.projects.iter().filter(|p| p.area_uuid.is_none()) {
        let meta = taskpaper_metadata(
            project.status,
            None,
            project.deadline,
            project.start_date,
            &project.tags,
        );
        writeln!(out, "{}:{meta}", escape_taskpaper_title(&project.title)).unwrap();
        if let Some(notes) = &project.notes {
            write_taskpaper_notes(&mut out, notes, 1);
        }
        for task in data.tasks.iter().filter(|t| {
            t.project_uuid.as_ref() == Some(&project.uuid) && t.parent_uuid.is_none()
        }) {
            write_taskpaper_task(&mut out, task, 1, &data.tasks);
        }
        writeln!(out).unwrap();
    }

    // --- Orphan tasks (no project, no area, no parent) ---
    for task in data.tasks.iter().filter(|t| {
        t.project_uuid.is_none() && t.area_uuid.is_none() && t.parent_uuid.is_none()
    }) {
        write_taskpaper_task(&mut out, task, 0, &data.tasks);
    }

    out
}

/// Sanitize a tag name for TaskPaper syntax.
///
/// TaskPaper tags are `@word` tokens — no spaces, parens, or `@`.
/// Whitespace runs become `-`; `@`, `(`, `)`, and control characters are stripped.
/// Note: paren content is not treated as a separate value — characters on both
/// sides of `(…)` are concatenated directly (e.g. `weird(name)` → `weirdname`).
/// This is intentional: Things tag names containing parens are rare enough that
/// discarding the distinction is preferable to a more complex parse.
#[cfg(feature = "export-taskpaper")]
pub(super) fn sanitize_taskpaper_tag(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_was_space = false;
    for ch in s.chars() {
        match ch {
            '@' | '(' | ')' => {}
            c if c.is_control() => {}
            c if c.is_whitespace() => {
                if !prev_was_space && !result.is_empty() {
                    result.push('-');
                }
                prev_was_space = true;
                continue;
            }
            c => result.push(c),
        }
        prev_was_space = false;
    }
    // Strip trailing dashes that result from trailing whitespace
    result.trim_end_matches('-').to_string()
}

/// Escape a task/project title for TaskPaper.
///
/// Titles must be single-line: `\n`, `\r`, and `\t` are replaced with spaces.
/// (`\t` would corrupt indent-based parsing if emitted inside a title.)
/// A trailing `:` is padded with a trailing space so the line is not
/// misread as a project header.
#[cfg(feature = "export-taskpaper")]
pub(super) fn escape_taskpaper_title(s: &str) -> String {
    let single_line = s.replace(['\n', '\r', '\t'], " ");
    if single_line.ends_with(':') {
        format!("{single_line} ")
    } else {
        single_line
    }
}

/// Build the inline metadata suffix for a task/project line.
///
/// Returns the `@tag(value) @tag …` string (with a leading space when non-empty).
#[cfg(feature = "export-taskpaper")]
fn taskpaper_metadata(
    status: TaskStatus,
    stop_date: Option<chrono::DateTime<chrono::Utc>>,
    deadline: Option<chrono::NaiveDate>,
    start_date: Option<chrono::NaiveDate>,
    tags: &[String],
) -> String {
    let mut parts: Vec<String> = Vec::new();

    match status {
        TaskStatus::Completed => {
            if let Some(dt) = stop_date {
                parts.push(format!("@done({})", dt.format("%Y-%m-%d")));
            } else {
                parts.push("@done".to_string());
            }
        }
        TaskStatus::Canceled => parts.push("@cancelled".to_string()),
        TaskStatus::Trashed => parts.push("@trashed".to_string()),
        TaskStatus::Incomplete => {}
    }

    if let Some(d) = deadline {
        parts.push(format!("@due({})", d.format("%Y-%m-%d")));
    }
    if let Some(d) = start_date {
        parts.push(format!("@start({})", d.format("%Y-%m-%d")));
    }

    for tag in tags {
        let sanitized = sanitize_taskpaper_tag(tag);
        if !sanitized.is_empty() {
            parts.push(format!("@{sanitized}"));
        }
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!(" {}", parts.join(" "))
    }
}

/// Write notes indented at `indent` tabs, one output line per note line.
#[cfg(feature = "export-taskpaper")]
fn write_taskpaper_notes(out: &mut String, notes: &str, indent: usize) {
    let prefix = "\t".repeat(indent);
    for line in notes.lines() {
        writeln!(out, "{prefix}{line}").unwrap();
    }
}

/// Write a single task (and its children) at the given indent depth.
#[cfg(feature = "export-taskpaper")]
fn write_taskpaper_task(out: &mut String, task: &Task, indent: usize, all_tasks: &[Task]) {
    let tabs = "\t".repeat(indent);

    if task.task_type == TaskType::Heading {
        // Headings are section dividers — render as a nested project-style header
        let meta = taskpaper_metadata(
            task.status,
            task.stop_date,
            task.deadline,
            task.start_date,
            &task.tags,
        );
        writeln!(out, "{tabs}{}:{meta}", escape_taskpaper_title(&task.title)).unwrap();
    } else {
        let meta = taskpaper_metadata(
            task.status,
            task.stop_date,
            task.deadline,
            task.start_date,
            &task.tags,
        );
        writeln!(out, "{tabs}- {}{meta}", escape_taskpaper_title(&task.title)).unwrap();
    }

    if let Some(notes) = &task.notes {
        write_taskpaper_notes(out, notes, indent + 1);
    }

    // Two sources of children depending on how the caller built ExportData:
    // - task.children populated (nested model): recurse with &[] so each child
    //   uses its own .children rather than re-scanning the flat list.
    // - flat all_tasks list (flat model): scan for tasks whose parent_uuid
    //   matches this task's uuid.
    if !task.children.is_empty() {
        for child in &task.children {
            write_taskpaper_task(out, child, indent + 1, &[]);
        }
    } else {
        for child in all_tasks
            .iter()
            .filter(|t| t.parent_uuid.as_ref() == Some(&task.uuid))
        {
            write_taskpaper_task(out, child, indent + 1, all_tasks);
        }
    }
}
