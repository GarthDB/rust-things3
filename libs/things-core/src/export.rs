//! Data export functionality for Things 3 data

use crate::models::{Area, Project, Task, TaskStatus, TaskType};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Export format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    Opml,
    Markdown,
}

impl std::str::FromStr for ExportFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "json" => Ok(ExportFormat::Json),
            "csv" => Ok(ExportFormat::Csv),
            "opml" => Ok(ExportFormat::Opml),
            "markdown" | "md" => Ok(ExportFormat::Markdown),
            _ => Err(anyhow::anyhow!("Unsupported export format: {}", s)),
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
    pub fn new(config: ExportConfig) -> Self {
        Self { config }
    }

    pub fn new_default() -> Self {
        Self::new(ExportConfig::default())
    }

    /// Export data in the specified format
    pub fn export(&self, data: &ExportData, format: ExportFormat) -> Result<String> {
        match format {
            ExportFormat::Json => self.export_json(data),
            ExportFormat::Csv => self.export_csv(data),
            ExportFormat::Opml => self.export_opml(data),
            ExportFormat::Markdown => self.export_markdown(data),
        }
    }

    /// Export as JSON
    fn export_json(&self, data: &ExportData) -> Result<String> {
        Ok(serde_json::to_string_pretty(data)?)
    }

    /// Export as CSV
    fn export_csv(&self, data: &ExportData) -> Result<String> {
        let mut csv = String::new();

        // Export tasks
        if !data.tasks.is_empty() {
            csv.push_str("Type,Title,Status,Notes,Start Date,Deadline,Created,Modified,Project,Area,Parent\n");
            for task in &data.tasks {
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{},{},{},{},{}\n",
                    format_task_type_csv(&task.task_type),
                    escape_csv(&task.title),
                    format_task_status_csv(&task.status),
                    escape_csv(task.notes.as_deref().unwrap_or("")),
                    format_date_csv(task.start_date),
                    format_date_csv(task.deadline),
                    format_datetime_csv(task.created),
                    format_datetime_csv(task.modified),
                    task.project_uuid.map(|u| u.to_string()).unwrap_or_default(),
                    task.area_uuid.map(|u| u.to_string()).unwrap_or_default(),
                    task.parent_uuid.map(|u| u.to_string()).unwrap_or_default(),
                ));
            }
        }

        // Export projects
        if !data.projects.is_empty() {
            csv.push_str("\n\nProjects\n");
            csv.push_str("Title,Status,Notes,Start Date,Deadline,Created,Modified,Area\n");
            for project in &data.projects {
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{},{}\n",
                    escape_csv(&project.title),
                    format_task_status_csv(&project.status),
                    escape_csv(project.notes.as_deref().unwrap_or("")),
                    format_date_csv(project.start_date),
                    format_date_csv(project.deadline),
                    format_datetime_csv(project.created),
                    format_datetime_csv(project.modified),
                    project.area_uuid.map(|u| u.to_string()).unwrap_or_default(),
                ));
            }
        }

        // Export areas
        if !data.areas.is_empty() {
            csv.push_str("\n\nAreas\n");
            csv.push_str("Title,Notes,Created,Modified\n");
            for area in &data.areas {
                csv.push_str(&format!(
                    "{},{},{},{}\n",
                    escape_csv(&area.title),
                    escape_csv(area.notes.as_deref().unwrap_or("")),
                    format_datetime_csv(area.created),
                    format_datetime_csv(area.modified),
                ));
            }
        }

        Ok(csv)
    }

    /// Export as OPML
    fn export_opml(&self, data: &ExportData) -> Result<String> {
        let mut opml = String::new();
        opml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        opml.push_str("<opml version=\"2.0\">\n");
        opml.push_str("  <head>\n");
        opml.push_str(&format!(
            "    <title>Things 3 Export - {}</title>\n",
            data.exported_at.format("%Y-%m-%d %H:%M:%S")
        ));
        opml.push_str("  </head>\n");
        opml.push_str("  <body>\n");

        // Group by areas
        let mut area_map: HashMap<Option<uuid::Uuid>, Vec<&Project>> = HashMap::new();
        for project in &data.projects {
            area_map.entry(project.area_uuid).or_default().push(project);
        }

        for area in &data.areas {
            opml.push_str(&format!(
                "    <outline text=\"{}\">\n",
                escape_xml(&area.title)
            ));

            if let Some(projects) = area_map.get(&Some(area.uuid)) {
                for project in projects {
                    opml.push_str(&format!(
                        "      <outline text=\"{}\" type=\"project\">\n",
                        escape_xml(&project.title)
                    ));

                    // Add tasks for this project
                    for task in &data.tasks {
                        if task.project_uuid == Some(project.uuid) {
                            opml.push_str(&format!(
                                "        <outline text=\"{}\" type=\"task\"/>\n",
                                escape_xml(&task.title)
                            ));
                        }
                    }

                    opml.push_str("      </outline>\n");
                }
            }

            opml.push_str("    </outline>\n");
        }

        opml.push_str("  </body>\n");
        opml.push_str("</opml>\n");
        Ok(opml)
    }

    /// Export as Markdown
    fn export_markdown(&self, data: &ExportData) -> Result<String> {
        let mut md = String::new();

        md.push_str("# Things 3 Export\n\n");
        md.push_str(&format!(
            "**Exported:** {}\n",
            data.exported_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        md.push_str(&format!("**Total Items:** {}\n\n", data.total_items));

        // Export areas
        if !data.areas.is_empty() {
            md.push_str("## Areas\n\n");
            for area in &data.areas {
                md.push_str(&format!("### {}\n", area.title));
                if let Some(notes) = &area.notes {
                    md.push_str(&format!("{}\n\n", notes));
                }
            }
        }

        // Export projects
        if !data.projects.is_empty() {
            md.push_str("## Projects\n\n");
            for project in &data.projects {
                md.push_str(&format!("### {}\n", project.title));
                md.push_str(&format!("**Status:** {:?}\n", project.status));
                if let Some(notes) = &project.notes {
                    md.push_str(&format!("**Notes:** {}\n", notes));
                }
                if let Some(deadline) = &project.deadline {
                    md.push_str(&format!("**Deadline:** {}\n", deadline));
                }
                md.push('\n');
            }
        }

        // Export tasks
        if !data.tasks.is_empty() {
            md.push_str("## Tasks\n\n");
            for task in &data.tasks {
                md.push_str(&format!(
                    "- [{}] {}\n",
                    if task.status == TaskStatus::Completed {
                        "x"
                    } else {
                        " "
                    },
                    task.title
                ));
                if let Some(notes) = &task.notes {
                    md.push_str(&format!("  - {}\n", notes));
                }
                if let Some(deadline) = &task.deadline {
                    md.push_str(&format!("  - **Deadline:** {}\n", deadline));
                }
            }
        }

        Ok(md)
    }
}

/// Helper functions for CSV export
fn format_task_type_csv(task_type: &TaskType) -> &'static str {
    match task_type {
        TaskType::Todo => "Todo",
        TaskType::Project => "Project",
        TaskType::Heading => "Heading",
        TaskType::Area => "Area",
    }
}

fn format_task_status_csv(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Incomplete => "Incomplete",
        TaskStatus::Completed => "Completed",
        TaskStatus::Canceled => "Canceled",
        TaskStatus::Trashed => "Trashed",
    }
}

fn format_date_csv(date: Option<chrono::NaiveDate>) -> String {
    date.map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

fn format_datetime_csv(datetime: DateTime<Utc>) -> String {
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_json() {
        let exporter = DataExporter::new_default();
        let data = ExportData::new(vec![], vec![], vec![]);
        let result = exporter.export(&data, ExportFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_csv() {
        let exporter = DataExporter::new_default();
        let data = ExportData::new(vec![], vec![], vec![]);
        let result = exporter.export(&data, ExportFormat::Csv);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_markdown() {
        let exporter = DataExporter::new_default();
        let data = ExportData::new(vec![], vec![], vec![]);
        let result = exporter.export(&data, ExportFormat::Markdown);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_opml() {
        let exporter = DataExporter::new_default();
        let data = ExportData::new(vec![], vec![], vec![]);
        let result = exporter.export(&data, ExportFormat::Opml);
        assert!(result.is_ok());
    }
}
