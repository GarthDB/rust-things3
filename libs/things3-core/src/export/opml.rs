#[cfg(feature = "export-opml")]
use std::collections::HashMap;
#[cfg(feature = "export-opml")]
use std::fmt::Write;

#[cfg(feature = "export-opml")]
use super::ExportData;

#[cfg(feature = "export-opml")]
pub(super) fn export_opml(data: &ExportData) -> String {
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
    let mut area_map: HashMap<Option<String>, Vec<&crate::models::Project>> = HashMap::new();
    for project in &data.projects {
        area_map
            .entry(project.area_uuid.as_ref().map(|u| u.to_string()))
            .or_default()
            .push(project);
    }

    for area in &data.areas {
        writeln!(opml, "    <outline text=\"{}\">", escape_xml(&area.title)).unwrap();

        if let Some(projects) = area_map.get(&Some(area.uuid.to_string())) {
            for project in projects {
                writeln!(
                    opml,
                    "      <outline text=\"{}\" type=\"project\">",
                    escape_xml(&project.title)
                )
                .unwrap();

                // Add tasks for this project
                for task in &data.tasks {
                    if task.project_uuid.as_ref() == Some(&project.uuid) {
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

#[cfg(feature = "export-opml")]
pub(super) fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
