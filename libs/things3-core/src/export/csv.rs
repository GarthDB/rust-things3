#[cfg(feature = "export-csv")]
use crate::models::{TaskStatus, TaskType};
#[cfg(feature = "export-csv")]
use chrono::{DateTime, Utc};
#[cfg(feature = "export-csv")]
use std::fmt::Write;

#[cfg(feature = "export-csv")]
use super::ExportData;

#[cfg(feature = "export-csv")]
pub(super) fn export_csv(data: &ExportData) -> String {
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
                task.project_uuid
                    .as_ref()
                    .map(|u| u.to_string())
                    .unwrap_or_default(),
                task.area_uuid
                    .as_ref()
                    .map(|u| u.to_string())
                    .unwrap_or_default(),
                task.parent_uuid
                    .as_ref()
                    .map(|u| u.to_string())
                    .unwrap_or_default(),
            )
            .unwrap();
        }
    }

    // Export projects
    if !data.projects.is_empty() {
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
                project
                    .area_uuid
                    .as_ref()
                    .map(|u| u.to_string())
                    .unwrap_or_default(),
            )
            .unwrap();
        }
    }

    // Export areas
    if !data.areas.is_empty() {
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

#[cfg(feature = "export-csv")]
pub(super) const fn format_task_type_csv(task_type: TaskType) -> &'static str {
    match task_type {
        TaskType::Todo => "Todo",
        TaskType::Project => "Project",
        TaskType::Heading => "Heading",
        TaskType::Area => "Area",
    }
}

#[cfg(feature = "export-csv")]
pub(super) const fn format_task_status_csv(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Incomplete => "Incomplete",
        TaskStatus::Completed => "Completed",
        TaskStatus::Canceled => "Canceled",
        TaskStatus::Trashed => "Trashed",
    }
}

#[cfg(feature = "export-csv")]
pub(super) fn format_date_csv(date: Option<chrono::NaiveDate>) -> String {
    date.map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

#[cfg(feature = "export-csv")]
pub(super) fn format_datetime_csv(datetime: DateTime<Utc>) -> String {
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(feature = "export-csv")]
pub(super) fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
