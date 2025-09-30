//! Data export functionality for Things 3 data

use crate::models::{Area, Project, Task, TaskStatus, TaskType};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;

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
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            "opml" => Ok(Self::Opml),
            "markdown" | "md" => Ok(Self::Markdown),
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
            ExportFormat::Csv => Ok(Self::export_csv(data)),
            ExportFormat::Opml => Ok(Self::export_opml(data)),
            ExportFormat::Markdown => Ok(Self::export_markdown(data)),
        }
    }

    /// Export as JSON
    fn export_json(data: &ExportData) -> Result<String> {
        Ok(serde_json::to_string_pretty(data)?)
    }

    /// Export as CSV
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
const fn format_task_type_csv(task_type: TaskType) -> &'static str {
    match task_type {
        TaskType::Todo => "Todo",
        TaskType::Project => "Project",
        TaskType::Heading => "Heading",
        TaskType::Area => "Area",
    }
}

const fn format_task_status_csv(status: TaskStatus) -> &'static str {
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
    fn test_export_csv_empty() {
        let exporter = DataExporter::new_default();
        let data = ExportData::new(vec![], vec![], vec![]);
        let result = exporter.export(&data, ExportFormat::Csv);
        assert!(result.is_ok());

        let csv = result.unwrap();
        assert!(csv.is_empty());
    }

    #[test]
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
    fn test_format_task_type_csv() {
        assert_eq!(format_task_type_csv(TaskType::Todo), "Todo");
        assert_eq!(format_task_type_csv(TaskType::Project), "Project");
        assert_eq!(format_task_type_csv(TaskType::Heading), "Heading");
        assert_eq!(format_task_type_csv(TaskType::Area), "Area");
    }

    #[test]
    fn test_format_task_status_csv() {
        assert_eq!(format_task_status_csv(TaskStatus::Incomplete), "Incomplete");
        assert_eq!(format_task_status_csv(TaskStatus::Completed), "Completed");
        assert_eq!(format_task_status_csv(TaskStatus::Canceled), "Canceled");
        assert_eq!(format_task_status_csv(TaskStatus::Trashed), "Trashed");
    }

    #[test]
    fn test_format_date_csv() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap();
        assert_eq!(format_date_csv(Some(date)), "2023-12-25");
        assert_eq!(format_date_csv(None), "");
    }

    #[test]
    fn test_format_datetime_csv() {
        let datetime = Utc::now();
        let formatted = format_datetime_csv(datetime);
        assert!(
            formatted.contains("2023") || formatted.contains("2024") || formatted.contains("2025")
        );
        assert!(formatted.contains('-'));
        assert!(formatted.contains(' '));
        assert!(formatted.contains(':'));
    }

    #[test]
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
}
