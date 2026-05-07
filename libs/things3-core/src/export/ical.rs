#[cfg(feature = "export-ical")]
use crate::models::TaskStatus;

#[cfg(feature = "export-ical")]
use super::ExportData;

/// Export as iCalendar (RFC 5545) — all items map to VTODO components.
///
/// Projects export as top-level VTODOs. Tasks with a project emit
/// `RELATED-TO:<project-uid>`; tasks with a parent task also emit
/// `RELATED-TO:<parent-uid>` (RELTYPE defaults to PARENT per RFC 5545).
/// Areas surface as `CATEGORIES` entries rather than standalone components.
#[cfg(feature = "export-ical")]
pub(super) fn export_icalendar(data: &ExportData) -> String {
    use icalendar::{Calendar, Component, DatePerhapsTime, EventLike, Todo};

    let mut cal = Calendar::new();
    cal.name("Things 3 Export");

    for project in &data.projects {
        let mut todo = Todo::new();
        todo.uid(project.uuid.as_ref());
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

        let area_cat = data
            .areas
            .iter()
            .find(|a| Some(&a.uuid) == project.area_uuid.as_ref());
        for cat in project
            .tags
            .iter()
            .map(String::as_str)
            .chain(area_cat.map(|a| a.title.as_str()))
        {
            todo.add_multi_property("CATEGORIES", cat);
        }

        todo.created(project.created);
        todo.last_modified(project.modified);

        cal.push(todo);
    }

    for task in &data.tasks {
        let mut todo = Todo::new();
        todo.uid(task.uuid.as_ref());
        todo.summary(&task.title);

        if let Some(notes) = &task.notes {
            todo.description(notes);
        }

        todo.status(ical_todo_status(task.status));

        if task.status == TaskStatus::Completed {
            if let Some(stop) = task.stop_date {
                todo.completed(stop);
            }
        }

        if let Some(d) = task.deadline {
            todo.due(DatePerhapsTime::Date(d));
        }
        if let Some(d) = task.start_date {
            todo.starts(DatePerhapsTime::Date(d));
        }

        let area_cat = data
            .areas
            .iter()
            .find(|a| Some(&a.uuid) == task.area_uuid.as_ref());
        for cat in task
            .tags
            .iter()
            .map(String::as_str)
            .chain(area_cat.map(|a| a.title.as_str()))
        {
            todo.add_multi_property("CATEGORIES", cat);
        }

        // RELATED-TO links project and parent task (RELTYPE defaults to PARENT per RFC 5545)
        if let Some(proj_uuid) = &task.project_uuid {
            todo.add_multi_property("RELATED-TO", proj_uuid.as_str());
        }
        if let Some(parent_uuid) = &task.parent_uuid {
            todo.add_multi_property("RELATED-TO", parent_uuid.as_str());
        }

        todo.created(task.created);
        todo.last_modified(task.modified);

        cal.push(todo);
    }

    cal.to_string()
}

#[cfg(feature = "export-ical")]
pub(super) fn ical_todo_status(status: TaskStatus) -> icalendar::TodoStatus {
    match status {
        TaskStatus::Incomplete => icalendar::TodoStatus::NeedsAction,
        TaskStatus::Completed => icalendar::TodoStatus::Completed,
        TaskStatus::Canceled | TaskStatus::Trashed => icalendar::TodoStatus::Cancelled,
    }
}
