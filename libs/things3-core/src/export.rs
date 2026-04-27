//! Data export functionality for Things 3 data

#[cfg(any(feature = "export-csv", feature = "export-taskpaper"))]
use crate::models::TaskType;
use crate::models::{Area, Project, Task, TaskStatus};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[cfg(feature = "export-opml")]
use std::collections::HashMap;
use std::fmt::Write;

/// Export format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    Opml,
    Markdown,
    TaskPaper,
    ICalendar,
}

impl std::str::FromStr for ExportFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            "opml" => Ok(Self::Opml),
            "markdown" | "md" => Ok(Self::Markdown),
            "taskpaper" | "tp" => Ok(Self::TaskPaper),
            "ical" | "ics" | "icalendar" => Ok(Self::ICalendar),
            _ => Err(anyhow::anyhow!("Unsupported export format: {s}")),
        }
    }
}

/// Export data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub tasks: Vec<Task>,
    pub projects: Vec<Project>,
    pub areas: Vec<Area>,
    pub exported_at: DateTime<Utc>,
    pub total_items: usize,
}

impl ExportData {
    #[must_use]
    pub fn new(tasks: Vec<Task>, projects: Vec<Project>, areas: Vec<Area>) -> Self {
        let total_items = tasks.len() + projects.len() + areas.len();
        Self {
            tasks,
            projects,
            areas,
            exported_at: Utc::now(),
            total_items,
        }
    }
}

/// Export configuration
#[derive(Debug, Clone)]
pub struct ExportConfig {
    pub include_metadata: bool,
    pub include_notes: bool,
    pub include_tags: bool,
    pub date_format: String,
    pub timezone: String,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            include_metadata: true,
            include_notes: true,
            include_tags: true,
            date_format: "%Y-%m-%d %H:%M:%S".to_string(),
            timezone: "UTC".to_string(),
        }
    }
}

/// Data exporter for Things 3 data
pub struct DataExporter {
    #[allow(dead_code)]
    config: ExportConfig,
}

impl DataExporter {
    #[must_use]
    pub const fn new(config: ExportConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub fn new_default() -> Self {
        Self::new(ExportConfig::default())
    }

    /// Export data in the specified format
    ///
    /// # Errors
    ///
    /// Returns an error if the export format is not supported or if serialization fails.
    pub fn export(&self, data: &ExportData, format: ExportFormat) -> Result<String> {
        match format {
            ExportFormat::Json => Self::export_json(data),
            #[cfg(feature = "export-csv")]
            ExportFormat::Csv => Ok(Self::export_csv(data)),
            #[cfg(not(feature = "export-csv"))]
            ExportFormat::Csv => Err(anyhow::anyhow!(
                "CSV export is not enabled. Enable the 'export-csv' feature."
            )),
            #[cfg(feature = "export-opml")]
            ExportFormat::Opml => Ok(Self::export_opml(data)),
            #[cfg(not(feature = "export-opml"))]
            ExportFormat::Opml => Err(anyhow::anyhow!(
                "OPML export is not enabled. Enable the 'export-opml' feature."
            )),
            ExportFormat::Markdown => Ok(Self::export_markdown(data)),
            #[cfg(feature = "export-taskpaper")]
            ExportFormat::TaskPaper => Ok(Self::export_taskpaper(data)),
            #[cfg(not(feature = "export-taskpaper"))]
            ExportFormat::TaskPaper => Err(anyhow::anyhow!(
                "TaskPaper export is not enabled. Enable the 'export-taskpaper' feature."
            )),
            #[cfg(feature = "export-ical")]
            ExportFormat::ICalendar => Ok(export_icalendar(data)),
            #[cfg(not(feature = "export-ical"))]
            ExportFormat::ICalendar => Err(anyhow::anyhow!(
                "iCalendar export is not enabled. Enable the 'export-ical' feature."
            )),
        }
    }

    /// Export as JSON
    fn export_json(data: &ExportData) -> Result<String> {
        Ok(serde_json::to_string_pretty(data)?)
    }

    /// Export as CSV
    #[cfg(feature = "export-csv")]
    fn export_csv(data: &ExportData) -> String {
        let mut csv = String::new();

        // Export tasks
        if !data.tasks.is_empty() {
            csv.push_str("Type,Title,Status,Notes,Start Date,Deadline,Created,Modified,Project,Area,Parent\n");
            for task in &data.tasks {
                writeln!(
                    csv,
                    "{},{},{},{},{},{},{},{},{},{},{}",
                    format_task_type_csv(task.task_type),
                    escape_csv(&task.title),
                    format_task_status_csv(task.status),
                    escape_csv(task.notes.as_deref().unwrap_or("")),
                    format_date_csv(task.start_date),
                    format_date_csv(task.deadline),
                    format_datetime_csv(task.created),
                    format_datetime_csv(task.modified),
                    task.project_uuid.map(|u| u.to_string()).unwrap_or_default(),
                    task.area_uuid.map(|u| u.to_string()).unwrap_or_default(),
                    task.parent_uuid.map(|u| u.to_string()).unwrap_or_default(),
                )
                .unwrap();
            }
        }

        // Export projects
        if !data.projects.is_empty() {
            csv.push_str("\n\nProjects\n");
            csv.push_str("Title,Status,Notes,Start Date,Deadline,Created,Modified,Area\n");
            for project in &data.projects {
                writeln!(
                    csv,
                    "{},{},{},{},{},{},{},{}",
                    escape_csv(&project.title),
                    format_task_status_csv(project.status),
                    escape_csv(project.notes.as_deref().unwrap_or("")),
                    format_date_csv(project.start_date),
                    format_date_csv(project.deadline),
                    format_datetime_csv(project.created),
                    format_datetime_csv(project.modified),
                    project.area_uuid.map(|u| u.to_string()).unwrap_or_default(),
                )
                .unwrap();
            }
        }

        // Export areas
        if !data.areas.is_empty() {
            csv.push_str("\n\nAreas\n");
            csv.push_str("Title,Notes,Created,Modified\n");
            for area in &data.areas {
                writeln!(
                    csv,
                    "{},{},{},{}",
                    escape_csv(&area.title),
                    escape_csv(area.notes.as_deref().unwrap_or("")),
                    format_datetime_csv(area.created),
                    format_datetime_csv(area.modified),
                )
                .unwrap();
            }
        }

        csv
    }

    /// Export as OPML
    #[cfg(feature = "export-opml")]
    fn export_opml(data: &ExportData) -> String {
        let mut opml = String::new();
        opml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        opml.push_str("<opml version=\"2.0\">\n");
        opml.push_str("  <head>\n");
        writeln!(
            opml,
            "    <title>Things 3 Export - {}</title>",
            data.exported_at.format("%Y-%m-%d %H:%M:%S")
        )
        .unwrap();
        opml.push_str("  </head>\n");
        opml.push_str("  <body>\n");

        // Group by areas
        let mut area_map: HashMap<Option<uuid::Uuid>, Vec<&Project>> = HashMap::new();
        for project in &data.projects {
            area_map.entry(project.area_uuid).or_default().push(project);
        }

        for area in &data.areas {
            writeln!(opml, "    <outline text=\"{}\">", escape_xml(&area.title)).unwrap();

            if let Some(projects) = area_map.get(&Some(area.uuid)) {
                for project in projects {
                    writeln!(
                        opml,
                        "      <outline text=\"{}\" type=\"project\">",
                        escape_xml(&project.title)
                    )
                    .unwrap();

                    // Add tasks for this project
                    for task in &data.tasks {
                        if task.project_uuid == Some(project.uuid) {
                            writeln!(
                                opml,
                                "        <outline text=\"{}\" type=\"task\"/>",
                                escape_xml(&task.title)
                            )
                            .unwrap();
                        }
                    }

                    opml.push_str("      </outline>\n");
                }
            }

            opml.push_str("    </outline>\n");
        }

        opml.push_str("  </body>\n");
        opml.push_str("</opml>\n");
        opml
    }

    /// Export as TaskPaper
    #[cfg(feature = "export-taskpaper")]
    fn export_taskpaper(data: &ExportData) -> String {
        let mut out = String::new();

        // --- Areas → their projects → tasks ---
        for area in &data.areas {
            let area_meta =
                taskpaper_metadata(TaskStatus::Incomplete, None, None, None, &area.tags);
            writeln!(out, "{}:{area_meta}", escape_taskpaper_title(&area.title)).unwrap();

            let area_projects: Vec<&Project> = data
                .projects
                .iter()
                .filter(|p| p.area_uuid == Some(area.uuid))
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
                for task in data
                    .tasks
                    .iter()
                    .filter(|t| t.project_uuid == Some(project.uuid) && t.parent_uuid.is_none())
                {
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
            for task in data
                .tasks
                .iter()
                .filter(|t| t.project_uuid == Some(project.uuid) && t.parent_uuid.is_none())
            {
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

    /// Export as Markdown
    fn export_markdown(data: &ExportData) -> String {
        let mut md = String::new();

        md.push_str("# Things 3 Export\n\n");
        writeln!(
            md,
            "**Exported:** {}",
            data.exported_at.format("%Y-%m-%d %H:%M:%S UTC")
        )
        .unwrap();
        writeln!(md, "**Total Items:** {}\n", data.total_items).unwrap();

        // Export areas
        if !data.areas.is_empty() {
            md.push_str("## Areas\n\n");
            for area in &data.areas {
                writeln!(md, "### {}", area.title).unwrap();
                if let Some(notes) = &area.notes {
                    writeln!(md, "{notes}\n").unwrap();
                }
            }
        }

        // Export projects
        if !data.projects.is_empty() {
            md.push_str("## Projects\n\n");
            for project in &data.projects {
                writeln!(md, "### {}", project.title).unwrap();
                writeln!(md, "**Status:** {:?}", project.status).unwrap();
                if let Some(notes) = &project.notes {
                    writeln!(md, "**Notes:** {notes}").unwrap();
                }
                if let Some(deadline) = &project.deadline {
                    writeln!(md, "**Deadline:** {deadline}").unwrap();
                }
                md.push('\n');
            }
        }

        // Export tasks
        if !data.tasks.is_empty() {
            md.push_str("## Tasks\n\n");
            for task in &data.tasks {
                writeln!(
                    md,
                    "- [{}] {}",
                    if task.status == TaskStatus::Completed {
                        "x"
                    } else {
                        " "
                    },
                    task.title
                )
                .unwrap();
                if let Some(notes) = &task.notes {
                    writeln!(md, "  - {notes}").unwrap();
                }
                if let Some(deadline) = &task.deadline {
                    writeln!(md, "  - **Deadline:** {deadline}").unwrap();
                }
            }
        }

        md
    }
}

/// Helper functions for CSV export
#[cfg(feature = "export-csv")]
const fn format_task_type_csv(task_type: TaskType) -> &'static str {
    match task_type {
        TaskType::Todo => "Todo",
        TaskType::Project => "Project",
        TaskType::Heading => "Heading",
        TaskType::Area => "Area",
    }
}

#[cfg(feature = "export-csv")]
const fn format_task_status_csv(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Incomplete => "Incomplete",
        TaskStatus::Completed => "Completed",
        TaskStatus::Canceled => "Canceled",
        TaskStatus::Trashed => "Trashed",
    }
}

#[cfg(feature = "export-csv")]
fn format_date_csv(date: Option<chrono::NaiveDate>) -> String {
    date.map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

#[cfg(feature = "export-csv")]
fn format_datetime_csv(datetime: DateTime<Utc>) -> String {
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(feature = "export-csv")]
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
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
fn sanitize_taskpaper_tag(s: &str) -> String {
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
fn escape_taskpaper_title(s: &str) -> String {
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
    stop_date: Option<chrono::DateTime<Utc>>,
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
            .filter(|t| t.parent_uuid == Some(task.uuid))
        {
            write_taskpaper_task(out, child, indent + 1, all_tasks);
        }
    }
}

#[cfg(feature = "export-opml")]
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Export as iCalendar (RFC 5545) — all items map to VTODO components.
///
/// Projects export as top-level VTODOs. Tasks reference their project via
/// `RELATED-TO`. Areas are not emitted as components; they surface as entries
/// in each task/project's `CATEGORIES` list.
#[cfg(feature = "export-ical")]
fn export_icalendar(data: &ExportData) -> String {
    use icalendar::{Calendar, Component, DatePerhapsTime, EventLike, Todo};

    let mut cal = Calendar::new();
    cal.name("Things 3 Export");

    for project in &data.projects {
        let mut todo = Todo::new();
        todo.uid(&project.uuid.to_string());
        todo.summary(&project.title);

        if let Some(notes) = &project.notes {
            todo.description(notes);
        }

        todo.status(ical_todo_status(project.status));

        if let Some(d) = project.deadline {
            todo.due(DatePerhapsTime::Date(d));
        }
        if let Some(d) = project.start_date {
            todo.starts(DatePerhapsTime::Date(d));
        }

        let mut cats: Vec<&str> = project.tags.iter().map(String::as_str).collect();
        if let Some(area) = data
            .areas
            .iter()
            .find(|a| Some(a.uuid) == project.area_uuid)
        {
            cats.push(&area.title);
        }
        if !cats.is_empty() {
            todo.add_property("CATEGORIES", cats.join(","));
        }

        todo.add_property("CREATED", ical_datetime(project.created));
        todo.add_property("LAST-MODIFIED", ical_datetime(project.modified));

        cal.push(todo);
    }

    for task in &data.tasks {
        let mut todo = Todo::new();
        todo.uid(&task.uuid.to_string());
        todo.summary(&task.title);

        if let Some(notes) = &task.notes {
            todo.description(notes);
        }

        todo.status(ical_todo_status(task.status));

        if task.status == TaskStatus::Completed {
            if let Some(stop) = task.stop_date {
                todo.add_property("COMPLETED", ical_datetime(stop));
            }
        }

        if let Some(d) = task.deadline {
            todo.due(DatePerhapsTime::Date(d));
        }
        if let Some(d) = task.start_date {
            todo.starts(DatePerhapsTime::Date(d));
        }

        let mut cats: Vec<&str> = task.tags.iter().map(String::as_str).collect();
        if let Some(area) = data.areas.iter().find(|a| Some(a.uuid) == task.area_uuid) {
            cats.push(&area.title);
        }
        if !cats.is_empty() {
            todo.add_property("CATEGORIES", cats.join(","));
        }

        if let Some(proj_uuid) = task.project_uuid {
            todo.add_property("RELATED-TO", proj_uuid.to_string());
        }

        todo.add_property("CREATED", ical_datetime(task.created));
        todo.add_property("LAST-MODIFIED", ical_datetime(task.modified));

        cal.push(todo);
    }

    cal.to_string()
}

#[cfg(feature = "export-ical")]
fn ical_todo_status(status: TaskStatus) -> icalendar::TodoStatus {
    match status {
        TaskStatus::Incomplete => icalendar::TodoStatus::NeedsAction,
        TaskStatus::Completed => icalendar::TodoStatus::Completed,
        TaskStatus::Canceled | TaskStatus::Trashed => icalendar::TodoStatus::Cancelled,
    }
}

#[cfg(feature = "export-ical")]
fn ical_datetime(dt: DateTime<Utc>) -> String {
    dt.format("%Y%m%dT%H%M%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{create_mock_areas, create_mock_projects, create_mock_tasks};

    #[test]
    fn test_export_format_from_str() {
        assert_eq!("json".parse::<ExportFormat>().unwrap(), ExportFormat::Json);
        assert_eq!("JSON".parse::<ExportFormat>().unwrap(), ExportFormat::Json);
        assert_eq!("csv".parse::<ExportFormat>().unwrap(), ExportFormat::Csv);
        assert_eq!("CSV".parse::<ExportFormat>().unwrap(), ExportFormat::Csv);
        assert_eq!("opml".parse::<ExportFormat>().unwrap(), ExportFormat::Opml);
        assert_eq!("OPML".parse::<ExportFormat>().unwrap(), ExportFormat::Opml);
        assert_eq!(
            "markdown".parse::<ExportFormat>().unwrap(),
            ExportFormat::Markdown
        );
        assert_eq!(
            "Markdown".parse::<ExportFormat>().unwrap(),
            ExportFormat::Markdown
        );
        assert_eq!(
            "md".parse::<ExportFormat>().unwrap(),
            ExportFormat::Markdown
        );
        assert_eq!(
            "MD".parse::<ExportFormat>().unwrap(),
            ExportFormat::Markdown
        );

        assert!("invalid".parse::<ExportFormat>().is_err());
        assert!("".parse::<ExportFormat>().is_err());
    }

    #[test]
    fn test_export_data_new() {
        let tasks = create_mock_tasks();
        let projects = create_mock_projects();
        let areas = create_mock_areas();

        let data = ExportData::new(tasks.clone(), projects.clone(), areas.clone());

        assert_eq!(data.tasks.len(), tasks.len());
        assert_eq!(data.projects.len(), projects.len());
        assert_eq!(data.areas.len(), areas.len());
        assert_eq!(data.total_items, tasks.len() + projects.len() + areas.len());
        assert!(data.exported_at <= Utc::now());
    }

    #[test]
    fn test_export_config_default() {
        let config = ExportConfig::default();

        assert!(config.include_metadata);
        assert!(config.include_notes);
        assert!(config.include_tags);
        assert_eq!(config.date_format, "%Y-%m-%d %H:%M:%S");
        assert_eq!(config.timezone, "UTC");
    }

    #[test]
    fn test_data_exporter_new() {
        let config = ExportConfig::default();
        let _exporter = DataExporter::new(config);
        // Just test that it can be created
        // Test passes if we reach this point
    }

    #[test]
    fn test_data_exporter_new_default() {
        let _exporter = DataExporter::new_default();
        // Just test that it can be created
        // Test passes if we reach this point
    }

    #[test]
    fn test_export_json_empty() {
        let exporter = DataExporter::new_default();
        let data = ExportData::new(vec![], vec![], vec![]);
        let result = exporter.export(&data, ExportFormat::Json);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert!(json.contains("\"tasks\""));
        assert!(json.contains("\"projects\""));
        assert!(json.contains("\"areas\""));
        assert!(json.contains("\"exported_at\""));
        assert!(json.contains("\"total_items\""));
    }

    #[test]
    fn test_export_json_with_data() {
        let exporter = DataExporter::new_default();
        let tasks = create_mock_tasks();
        let projects = create_mock_projects();
        let areas = create_mock_areas();
        let data = ExportData::new(tasks, projects, areas);

        let result = exporter.export(&data, ExportFormat::Json);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert!(json.contains("\"Research competitors\""));
        assert!(json.contains("\"Website Redesign\""));
        assert!(json.contains("\"Work\""));
    }

    #[test]
    #[cfg(feature = "export-csv")]
    fn test_export_csv_empty() {
        let exporter = DataExporter::new_default();
        let data = ExportData::new(vec![], vec![], vec![]);
        let result = exporter.export(&data, ExportFormat::Csv);
        assert!(result.is_ok());

        let csv = result.unwrap();
        assert!(csv.is_empty());
    }

    #[test]
    #[cfg(feature = "export-csv")]
    fn test_export_csv_with_data() {
        let exporter = DataExporter::new_default();
        let tasks = create_mock_tasks();
        let projects = create_mock_projects();
        let areas = create_mock_areas();
        let data = ExportData::new(tasks, projects, areas);

        let result = exporter.export(&data, ExportFormat::Csv);
        assert!(result.is_ok());

        let csv = result.unwrap();
        assert!(csv.contains(
            "Type,Title,Status,Notes,Start Date,Deadline,Created,Modified,Project,Area,Parent"
        ));
        assert!(csv.contains("Research competitors"));
        assert!(csv.contains("Projects"));
        assert!(csv.contains("Website Redesign"));
        assert!(csv.contains("Areas"));
        assert!(csv.contains("Work"));
    }

    #[test]
    fn test_export_markdown_empty() {
        let exporter = DataExporter::new_default();
        let data = ExportData::new(vec![], vec![], vec![]);
        let result = exporter.export(&data, ExportFormat::Markdown);
        assert!(result.is_ok());

        let md = result.unwrap();
        assert!(md.contains("# Things 3 Export"));
        assert!(md.contains("**Total Items:** 0"));
    }

    #[test]
    fn test_export_markdown_with_data() {
        let exporter = DataExporter::new_default();
        let tasks = create_mock_tasks();
        let projects = create_mock_projects();
        let areas = create_mock_areas();
        let data = ExportData::new(tasks, projects, areas);

        let result = exporter.export(&data, ExportFormat::Markdown);
        assert!(result.is_ok());

        let md = result.unwrap();
        assert!(md.contains("# Things 3 Export"));
        assert!(md.contains("## Areas"));
        assert!(md.contains("### Work"));
        assert!(md.contains("## Projects"));
        assert!(md.contains("### Website Redesign"));
        assert!(md.contains("## Tasks"));
        assert!(md.contains("- [ ] Research competitors"));
    }

    #[test]
    #[cfg(feature = "export-opml")]
    fn test_export_opml_empty() {
        let exporter = DataExporter::new_default();
        let data = ExportData::new(vec![], vec![], vec![]);
        let result = exporter.export(&data, ExportFormat::Opml);
        assert!(result.is_ok());

        let opml = result.unwrap();
        assert!(opml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(opml.contains("<opml version=\"2.0\">"));
        assert!(opml.contains("<head>"));
        assert!(opml.contains("<body>"));
        assert!(opml.contains("</opml>"));
    }

    #[test]
    #[cfg(feature = "export-opml")]
    fn test_export_opml_with_data() {
        let exporter = DataExporter::new_default();
        let tasks = create_mock_tasks();
        let projects = create_mock_projects();
        let areas = create_mock_areas();
        let data = ExportData::new(tasks, projects, areas);

        let result = exporter.export(&data, ExportFormat::Opml);
        assert!(result.is_ok());

        let opml = result.unwrap();
        assert!(opml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(opml.contains("<opml version=\"2.0\">"));
        assert!(opml.contains("Work"));
        assert!(opml.contains("Website Redesign"));
    }

    #[test]
    #[cfg(feature = "export-csv")]
    fn test_format_task_type_csv() {
        assert_eq!(format_task_type_csv(TaskType::Todo), "Todo");
        assert_eq!(format_task_type_csv(TaskType::Project), "Project");
        assert_eq!(format_task_type_csv(TaskType::Heading), "Heading");
        assert_eq!(format_task_type_csv(TaskType::Area), "Area");
    }

    #[test]
    #[cfg(feature = "export-csv")]
    fn test_format_task_status_csv() {
        assert_eq!(format_task_status_csv(TaskStatus::Incomplete), "Incomplete");
        assert_eq!(format_task_status_csv(TaskStatus::Completed), "Completed");
        assert_eq!(format_task_status_csv(TaskStatus::Canceled), "Canceled");
        assert_eq!(format_task_status_csv(TaskStatus::Trashed), "Trashed");
    }

    #[test]
    #[cfg(feature = "export-csv")]
    fn test_format_date_csv() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap();
        assert_eq!(format_date_csv(Some(date)), "2023-12-25");
        assert_eq!(format_date_csv(None), "");
    }

    #[test]
    #[cfg(feature = "export-csv")]
    fn test_format_datetime_csv() {
        let datetime = Utc::now();
        let formatted = format_datetime_csv(datetime);
        // Check that the formatted string contains the current year
        let current_year = datetime.format("%Y").to_string();
        assert!(
            formatted.contains(&current_year),
            "Formatted datetime should contain current year: {}",
            current_year
        );
        assert!(formatted.contains('-'));
        assert!(formatted.contains(' '));
        assert!(formatted.contains(':'));
    }

    #[test]
    #[cfg(feature = "export-csv")]
    fn test_escape_csv() {
        // No special characters
        assert_eq!(escape_csv("normal text"), "normal text");

        // Contains comma
        assert_eq!(escape_csv("text,with,comma"), "\"text,with,comma\"");

        // Contains quote
        assert_eq!(escape_csv("text\"with\"quote"), "\"text\"\"with\"\"quote\"");

        // Contains newline
        assert_eq!(escape_csv("text\nwith\nnewline"), "\"text\nwith\nnewline\"");

        // Contains multiple special characters
        assert_eq!(
            escape_csv("text,\"with\",\nall"),
            "\"text,\"\"with\"\",\nall\""
        );
    }

    #[test]
    #[cfg(feature = "export-opml")]
    fn test_escape_xml() {
        assert_eq!(escape_xml("normal text"), "normal text");
        assert_eq!(
            escape_xml("text&with&ampersand"),
            "text&amp;with&amp;ampersand"
        );
        assert_eq!(escape_xml("text<with>tags"), "text&lt;with&gt;tags");
        assert_eq!(
            escape_xml("text\"with\"quotes"),
            "text&quot;with&quot;quotes"
        );
        assert_eq!(
            escape_xml("text'with'apostrophe"),
            "text&apos;with&apos;apostrophe"
        );
        assert_eq!(escape_xml("all<>&\"'"), "all&lt;&gt;&amp;&quot;&apos;");
    }

    #[test]
    fn test_export_data_serialization() {
        let tasks = create_mock_tasks();
        let projects = create_mock_projects();
        let areas = create_mock_areas();
        let data = ExportData::new(tasks, projects, areas);

        // Test that ExportData can be serialized and deserialized
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: ExportData = serde_json::from_str(&json).unwrap();

        assert_eq!(data.tasks.len(), deserialized.tasks.len());
        assert_eq!(data.projects.len(), deserialized.projects.len());
        assert_eq!(data.areas.len(), deserialized.areas.len());
        assert_eq!(data.total_items, deserialized.total_items);
    }

    #[test]
    fn test_export_config_clone() {
        let config = ExportConfig::default();
        let cloned = config.clone();

        assert_eq!(config.include_metadata, cloned.include_metadata);
        assert_eq!(config.include_notes, cloned.include_notes);
        assert_eq!(config.include_tags, cloned.include_tags);
        assert_eq!(config.date_format, cloned.date_format);
        assert_eq!(config.timezone, cloned.timezone);
    }

    #[test]
    fn test_export_format_debug() {
        let formats = vec![
            ExportFormat::Json,
            ExportFormat::Csv,
            ExportFormat::Opml,
            ExportFormat::Markdown,
        ];

        for format in formats {
            let debug_str = format!("{format:?}");
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_export_format_equality() {
        assert_eq!(ExportFormat::Json, ExportFormat::Json);
        assert_eq!(ExportFormat::Csv, ExportFormat::Csv);
        assert_eq!(ExportFormat::Opml, ExportFormat::Opml);
        assert_eq!(ExportFormat::Markdown, ExportFormat::Markdown);

        assert_ne!(ExportFormat::Json, ExportFormat::Csv);
        assert_ne!(ExportFormat::Csv, ExportFormat::Opml);
        assert_ne!(ExportFormat::Opml, ExportFormat::Markdown);
        assert_ne!(ExportFormat::Markdown, ExportFormat::Json);
    }

    #[test]
    fn test_export_data_debug() {
        let data = ExportData::new(vec![], vec![], vec![]);
        let debug_str = format!("{data:?}");
        assert!(!debug_str.is_empty());
        assert!(debug_str.contains("ExportData"));
    }

    // Not gated behind #[cfg(feature = "export-taskpaper")] intentionally:
    // ExportFormat::TaskPaper is an unconditional enum variant and FromStr
    // always matches "taskpaper" — the feature flag only controls whether the
    // actual serialization work is compiled in.
    #[test]
    fn test_export_format_from_str_taskpaper() {
        assert_eq!(
            "taskpaper".parse::<ExportFormat>().unwrap(),
            ExportFormat::TaskPaper
        );
        assert_eq!(
            "TaskPaper".parse::<ExportFormat>().unwrap(),
            ExportFormat::TaskPaper
        );
        assert_eq!(
            "TASKPAPER".parse::<ExportFormat>().unwrap(),
            ExportFormat::TaskPaper
        );
        assert_eq!(
            "tp".parse::<ExportFormat>().unwrap(),
            ExportFormat::TaskPaper
        );
        assert_eq!(
            "TP".parse::<ExportFormat>().unwrap(),
            ExportFormat::TaskPaper
        );
    }

    #[test]
    #[cfg(not(feature = "export-taskpaper"))]
    fn test_export_taskpaper_disabled() {
        let exporter = DataExporter::new_default();
        let data = ExportData::new(vec![], vec![], vec![]);
        let result = exporter.export(&data, ExportFormat::TaskPaper);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("export-taskpaper"),
            "Error should name the missing feature, got: {msg}"
        );
    }

    #[test]
    #[cfg(feature = "export-taskpaper")]
    fn test_export_taskpaper_empty() {
        let exporter = DataExporter::new_default();
        let data = ExportData::new(vec![], vec![], vec![]);
        let result = exporter.export(&data, ExportFormat::TaskPaper);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    #[cfg(feature = "export-taskpaper")]
    fn test_export_taskpaper_with_data() {
        let exporter = DataExporter::new_default();
        let tasks = create_mock_tasks();
        let projects = create_mock_projects();
        let areas = create_mock_areas();
        let data = ExportData::new(tasks, projects, areas);

        let result = exporter.export(&data, ExportFormat::TaskPaper);
        assert!(result.is_ok());
        let tp = result.unwrap();

        // Area as top-level project
        assert!(tp.contains("Work:"), "Expected 'Work:' in output:\n{tp}");
        // Project indented under area
        assert!(
            tp.contains("\tWebsite Redesign:"),
            "Expected '\\tWebsite Redesign:' in output:\n{tp}"
        );
        // Task indented under project (two levels)
        assert!(
            tp.contains("\t\t- Research competitors"),
            "Expected '\\t\\t- Research competitors' in output:\n{tp}"
        );
    }

    #[test]
    #[cfg(feature = "export-taskpaper")]
    fn test_export_taskpaper_status_tags() {
        use chrono::TimeZone;

        let base_uuid = uuid::Uuid::parse_str("aaaaaaaa-0000-0000-0000-000000000000").unwrap();
        let make_task =
            |n: u8, status: TaskStatus, stop_date: Option<chrono::DateTime<Utc>>| Task {
                uuid: uuid::Uuid::parse_str(&format!("aaaaaaaa-0000-0000-0000-{n:012}")).unwrap(),
                title: format!("Task {n}"),
                task_type: TaskType::Todo,
                status,
                notes: None,
                start_date: None,
                deadline: None,
                created: Utc::now(),
                modified: Utc::now(),
                stop_date,
                project_uuid: None,
                area_uuid: None,
                parent_uuid: None,
                tags: vec![],
                children: vec![],
            };
        let _ = base_uuid;

        let stop = Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap();
        let tasks = vec![
            make_task(1, TaskStatus::Incomplete, None),
            make_task(2, TaskStatus::Completed, Some(stop)),
            make_task(3, TaskStatus::Completed, None),
            make_task(4, TaskStatus::Canceled, None),
            make_task(5, TaskStatus::Trashed, None),
        ];
        let data = ExportData::new(tasks, vec![], vec![]);
        let exporter = DataExporter::new_default();
        let tp = exporter.export(&data, ExportFormat::TaskPaper).unwrap();

        // Incomplete: no status tag
        assert!(
            tp.contains("- Task 1\n"),
            "Incomplete task should have no status tag:\n{tp}"
        );
        // Completed with stop date
        assert!(
            tp.contains("@done(2026-01-15)"),
            "Completed task with stop_date:\n{tp}"
        );
        // Completed without stop date
        assert!(
            tp.contains("- Task 3 @done\n"),
            "Completed task without stop_date:\n{tp}"
        );
        // Canceled
        assert!(tp.contains("@cancelled"), "Cancelled task:\n{tp}");
        // Trashed
        assert!(tp.contains("@trashed"), "Trashed task:\n{tp}");
    }

    #[test]
    #[cfg(feature = "export-taskpaper")]
    fn test_export_taskpaper_dates() {
        use chrono::NaiveDate;

        let task = Task {
            uuid: uuid::Uuid::parse_str("bbbbbbbb-0000-0000-0000-000000000001").unwrap(),
            title: "Task with dates".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: None,
            start_date: Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
            deadline: Some(NaiveDate::from_ymd_opt(2026, 4, 30).unwrap()),
            created: Utc::now(),
            modified: Utc::now(),
            stop_date: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: vec![],
            children: vec![],
        };
        let data = ExportData::new(vec![task], vec![], vec![]);
        let exporter = DataExporter::new_default();
        let tp = exporter.export(&data, ExportFormat::TaskPaper).unwrap();

        assert!(tp.contains("@due(2026-04-30)"), "Expected @due date:\n{tp}");
        assert!(
            tp.contains("@start(2026-03-01)"),
            "Expected @start date:\n{tp}"
        );
    }

    #[test]
    #[cfg(feature = "export-taskpaper")]
    fn test_export_taskpaper_tags() {
        let task = Task {
            uuid: uuid::Uuid::parse_str("cccccccc-0000-0000-0000-000000000001").unwrap(),
            title: "Tagged task".to_string(),
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
            tags: vec![
                "work".to_string(),
                "high priority".to_string(),
                "@weird(name)".to_string(),
            ],
            children: vec![],
        };
        let data = ExportData::new(vec![task], vec![], vec![]);
        let exporter = DataExporter::new_default();
        let tp = exporter.export(&data, ExportFormat::TaskPaper).unwrap();

        assert!(tp.contains("@work"), "Expected @work tag:\n{tp}");
        assert!(
            tp.contains("@high-priority"),
            "Expected @high-priority tag:\n{tp}"
        );
        // @ ( ) stripped; resulting non-empty tag should appear
        assert!(tp.contains("@weirdname"), "Expected @weirdname tag:\n{tp}");
    }

    #[test]
    #[cfg(feature = "export-taskpaper")]
    fn test_export_taskpaper_notes_multiline() {
        let task = Task {
            uuid: uuid::Uuid::parse_str("dddddddd-0000-0000-0000-000000000001").unwrap(),
            title: "Task with notes".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: Some("First line\nSecond line\nThird line".to_string()),
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
        let data = ExportData::new(vec![task], vec![], vec![]);
        let exporter = DataExporter::new_default();
        let tp = exporter.export(&data, ExportFormat::TaskPaper).unwrap();

        // Task is at indent 0, so notes are at indent 1 (\t)
        assert!(
            tp.contains("\tFirst line"),
            "Expected indented first line:\n{tp}"
        );
        assert!(
            tp.contains("\tSecond line"),
            "Expected indented second line:\n{tp}"
        );
        assert!(
            tp.contains("\tThird line"),
            "Expected indented third line:\n{tp}"
        );
    }

    #[test]
    #[cfg(feature = "export-taskpaper")]
    fn test_sanitize_taskpaper_tag() {
        assert_eq!(sanitize_taskpaper_tag("work"), "work");
        assert_eq!(sanitize_taskpaper_tag("high priority"), "high-priority");
        assert_eq!(sanitize_taskpaper_tag("  leading"), "leading");
        assert_eq!(sanitize_taskpaper_tag("trailing  "), "trailing");
        assert_eq!(sanitize_taskpaper_tag("@tag(value)"), "tagvalue");
        assert_eq!(sanitize_taskpaper_tag(""), "");
        assert_eq!(sanitize_taskpaper_tag("a  b  c"), "a-b-c");
    }

    #[test]
    #[cfg(feature = "export-taskpaper")]
    fn test_escape_taskpaper_title() {
        assert_eq!(escape_taskpaper_title("Normal title"), "Normal title");
        assert_eq!(escape_taskpaper_title("Multi\nline"), "Multi line");
        assert_eq!(
            escape_taskpaper_title("Carriage\rreturn"),
            "Carriage return"
        );
        assert_eq!(escape_taskpaper_title("Tab\there"), "Tab here");
        assert_eq!(
            escape_taskpaper_title("Ends with colon:"),
            "Ends with colon: "
        );
        assert_eq!(escape_taskpaper_title("Not a colon"), "Not a colon");
    }

    // Not gated behind #[cfg(feature = "export-ical")] intentionally:
    // ExportFormat::ICalendar is an unconditional enum variant and FromStr
    // always matches "ical"/"ics"/"icalendar" — the feature flag only controls
    // whether the actual serialization work is compiled in.
    #[test]
    fn test_export_format_from_str_icalendar() {
        assert_eq!(
            "ical".parse::<ExportFormat>().unwrap(),
            ExportFormat::ICalendar
        );
        assert_eq!(
            "ICAL".parse::<ExportFormat>().unwrap(),
            ExportFormat::ICalendar
        );
        assert_eq!(
            "ics".parse::<ExportFormat>().unwrap(),
            ExportFormat::ICalendar
        );
        assert_eq!(
            "ICS".parse::<ExportFormat>().unwrap(),
            ExportFormat::ICalendar
        );
        assert_eq!(
            "icalendar".parse::<ExportFormat>().unwrap(),
            ExportFormat::ICalendar
        );
        assert_eq!(
            "iCalendar".parse::<ExportFormat>().unwrap(),
            ExportFormat::ICalendar
        );
    }

    #[test]
    #[cfg(not(feature = "export-ical"))]
    fn test_export_icalendar_disabled() {
        let exporter = DataExporter::new_default();
        let data = ExportData::new(vec![], vec![], vec![]);
        let result = exporter.export(&data, ExportFormat::ICalendar);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("export-ical"),
            "Error should name the missing feature, got: {msg}"
        );
    }

    #[test]
    #[cfg(feature = "export-ical")]
    fn test_export_icalendar_empty() {
        let exporter = DataExporter::new_default();
        let data = ExportData::new(vec![], vec![], vec![]);
        let result = exporter.export(&data, ExportFormat::ICalendar);
        assert!(result.is_ok());
        let ics = result.unwrap();
        assert!(ics.contains("BEGIN:VCALENDAR"), "Missing VCALENDAR:\n{ics}");
        assert!(
            ics.contains("END:VCALENDAR"),
            "Missing END:VCALENDAR:\n{ics}"
        );
        assert!(
            !ics.contains("BEGIN:VTODO"),
            "Empty export should have no VTODOs:\n{ics}"
        );
    }

    #[test]
    #[cfg(feature = "export-ical")]
    fn test_export_icalendar_with_data() {
        let exporter = DataExporter::new_default();
        let tasks = create_mock_tasks();
        let projects = create_mock_projects();
        let areas = create_mock_areas();
        let data = ExportData::new(tasks, projects, areas);

        let ics = exporter.export(&data, ExportFormat::ICalendar).unwrap();

        assert!(ics.contains("BEGIN:VCALENDAR"), "Missing VCALENDAR:\n{ics}");
        // Projects and tasks both become VTODOs
        assert!(ics.contains("BEGIN:VTODO"), "Missing VTODO:\n{ics}");
        assert!(
            ics.contains("SUMMARY:Website Redesign"),
            "Missing project summary:\n{ics}"
        );
        assert!(
            ics.contains("SUMMARY:Research competitors"),
            "Missing task summary:\n{ics}"
        );
    }

    #[test]
    #[cfg(feature = "export-ical")]
    fn test_export_icalendar_uid_stable() {
        use crate::models::TaskType;
        let task_uuid = uuid::Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap();
        let task = Task {
            uuid: task_uuid,
            title: "UID test".to_string(),
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
        };
        let data = ExportData::new(vec![task], vec![], vec![]);
        let ics = DataExporter::new_default()
            .export(&data, ExportFormat::ICalendar)
            .unwrap();

        assert!(
            ics.contains("UID:aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
            "UID should equal task UUID:\n{ics}"
        );
    }

    #[test]
    #[cfg(feature = "export-ical")]
    fn test_export_icalendar_status_mapping() {
        use crate::models::TaskType;
        use chrono::TimeZone;

        let make_task = |n: u8, status: TaskStatus, stop: Option<DateTime<Utc>>| Task {
            uuid: uuid::Uuid::parse_str(&format!("00000000-0000-0000-0000-{n:012}")).unwrap(),
            title: format!("Task {n}"),
            task_type: TaskType::Todo,
            status,
            notes: None,
            start_date: None,
            deadline: None,
            created: Utc::now(),
            modified: Utc::now(),
            stop_date: stop,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: vec![],
            children: vec![],
        };

        let stop = Utc.with_ymd_and_hms(2026, 1, 15, 10, 30, 0).unwrap();
        let tasks = vec![
            make_task(1, TaskStatus::Incomplete, None),
            make_task(2, TaskStatus::Completed, Some(stop)),
            make_task(3, TaskStatus::Completed, None),
            make_task(4, TaskStatus::Canceled, None),
            make_task(5, TaskStatus::Trashed, None),
        ];
        let data = ExportData::new(tasks, vec![], vec![]);
        let ics = DataExporter::new_default()
            .export(&data, ExportFormat::ICalendar)
            .unwrap();

        assert!(
            ics.contains("STATUS:NEEDS-ACTION"),
            "Incomplete → NEEDS-ACTION:\n{ics}"
        );
        assert!(
            ics.contains("STATUS:COMPLETED"),
            "Completed → COMPLETED:\n{ics}"
        );
        // Completed task with stop_date should have COMPLETED: property
        assert!(
            ics.contains("COMPLETED:20260115T103000Z"),
            "Expected COMPLETED timestamp:\n{ics}"
        );
        assert!(
            ics.contains("STATUS:CANCELLED"),
            "Canceled/Trashed → CANCELLED:\n{ics}"
        );
    }

    #[test]
    #[cfg(feature = "export-ical")]
    fn test_export_icalendar_due_date() {
        use crate::models::TaskType;
        use chrono::NaiveDate;

        let task = Task {
            uuid: uuid::Uuid::parse_str("11111111-0000-0000-0000-000000000001").unwrap(),
            title: "Task with deadline".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: None,
            start_date: Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
            deadline: Some(NaiveDate::from_ymd_opt(2026, 4, 30).unwrap()),
            created: Utc::now(),
            modified: Utc::now(),
            stop_date: None,
            project_uuid: None,
            area_uuid: None,
            parent_uuid: None,
            tags: vec![],
            children: vec![],
        };
        let data = ExportData::new(vec![task], vec![], vec![]);
        let ics = DataExporter::new_default()
            .export(&data, ExportFormat::ICalendar)
            .unwrap();

        assert!(ics.contains("20260430"), "DUE date in output:\n{ics}");
        assert!(ics.contains("20260301"), "DTSTART date in output:\n{ics}");
    }

    #[test]
    #[cfg(feature = "export-ical")]
    fn test_export_icalendar_categories() {
        use crate::models::{Area, TaskType};

        let area_uuid = uuid::Uuid::parse_str("aaaaaaaa-0000-0000-0000-000000000000").unwrap();
        let area = Area {
            uuid: area_uuid,
            title: "Work".to_string(),
            notes: None,
            created: Utc::now(),
            modified: Utc::now(),
            tags: vec![],
            projects: vec![],
        };
        let task = Task {
            uuid: uuid::Uuid::parse_str("bbbbbbbb-0000-0000-0000-000000000001").unwrap(),
            title: "Tagged task".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: None,
            start_date: None,
            deadline: None,
            created: Utc::now(),
            modified: Utc::now(),
            stop_date: None,
            project_uuid: None,
            area_uuid: Some(area_uuid),
            parent_uuid: None,
            tags: vec!["focus".to_string(), "deep-work".to_string()],
            children: vec![],
        };
        let data = ExportData::new(vec![task], vec![], vec![area]);
        let ics = DataExporter::new_default()
            .export(&data, ExportFormat::ICalendar)
            .unwrap();

        assert!(
            ics.contains("CATEGORIES:"),
            "Expected CATEGORIES property:\n{ics}"
        );
        assert!(
            ics.contains("focus"),
            "Expected 'focus' in categories:\n{ics}"
        );
        assert!(
            ics.contains("deep-work"),
            "Expected 'deep-work' in categories:\n{ics}"
        );
        assert!(
            ics.contains("Work"),
            "Expected area name in categories:\n{ics}"
        );
    }

    #[test]
    #[cfg(feature = "export-ical")]
    fn test_export_icalendar_related_to() {
        use crate::models::TaskType;
        let proj_uuid = uuid::Uuid::parse_str("12345678-0000-0000-0000-000000000000").unwrap();
        let project = crate::models::Project {
            uuid: proj_uuid,
            title: "My Project".to_string(),
            notes: None,
            start_date: None,
            deadline: None,
            created: Utc::now(),
            modified: Utc::now(),
            area_uuid: None,
            tags: vec![],
            status: TaskStatus::Incomplete,
            tasks: vec![],
        };
        let task = Task {
            uuid: uuid::Uuid::parse_str("87654321-0000-0000-0000-000000000001").unwrap(),
            title: "Child task".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: None,
            start_date: None,
            deadline: None,
            created: Utc::now(),
            modified: Utc::now(),
            stop_date: None,
            project_uuid: Some(proj_uuid),
            area_uuid: None,
            parent_uuid: None,
            tags: vec![],
            children: vec![],
        };
        let data = ExportData::new(vec![task], vec![project], vec![]);
        let ics = DataExporter::new_default()
            .export(&data, ExportFormat::ICalendar)
            .unwrap();

        assert!(
            ics.contains("RELATED-TO:12345678-0000-0000-0000-000000000000"),
            "Expected RELATED-TO with project UUID:\n{ics}"
        );
    }

    #[test]
    #[cfg(feature = "export-ical")]
    fn test_export_icalendar_notes_multiline() {
        use crate::models::TaskType;
        let task = Task {
            uuid: uuid::Uuid::parse_str("cccccccc-0000-0000-0000-000000000001").unwrap(),
            title: "Task with notes".to_string(),
            task_type: TaskType::Todo,
            status: TaskStatus::Incomplete,
            notes: Some("First line\nSecond line\nThird line".to_string()),
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
        let data = ExportData::new(vec![task], vec![], vec![]);
        let ics = DataExporter::new_default()
            .export(&data, ExportFormat::ICalendar)
            .unwrap();

        // RFC 5545 escapes newlines in DESCRIPTION as \n (literal backslash-n)
        assert!(
            ics.contains("DESCRIPTION:"),
            "Expected DESCRIPTION property:\n{ics}"
        );
        assert!(
            ics.contains("First line"),
            "Expected first line in description:\n{ics}"
        );
        assert!(
            ics.contains("Second line"),
            "Expected second line in description:\n{ics}"
        );
        assert!(
            ics.contains("Third line"),
            "Expected third line in description:\n{ics}"
        );
    }
}
