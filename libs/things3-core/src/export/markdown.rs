use crate::models::TaskStatus;
use std::fmt::Write;

use super::ExportData;

pub(super) fn export_markdown(data: &ExportData) -> String {
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
