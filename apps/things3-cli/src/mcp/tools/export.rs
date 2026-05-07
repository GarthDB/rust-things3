use crate::mcp::{expand_tilde, CallToolResult, Content, McpError, McpResult, ThingsMcpServer};
use serde_json::Value;
use things3_core::{DataExporter, ExportData, ExportFormat};

impl ThingsMcpServer {
    pub(in crate::mcp) async fn handle_export_data(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let format = args
            .get("format")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_parameter("format"))?;
        let data_type = args
            .get("data_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_parameter("data_type"))?;
        let output_path = args.get("output_path").and_then(|v| v.as_str());

        if !matches!(data_type, "tasks" | "projects" | "areas" | "all") {
            return Err(McpError::invalid_data_type(
                data_type,
                "tasks, projects, areas, all",
            ));
        }

        if format == "csv" && data_type == "all" {
            return Err(McpError::invalid_parameter(
                "data_type",
                "CSV format does not support data_type=all. Use tasks, projects, or areas individually.",
            ));
        }

        let db = &self.db;
        let need_tasks = matches!(data_type, "tasks" | "all");
        let need_projects = matches!(data_type, "projects" | "all");
        let need_areas = matches!(data_type, "areas" | "all");

        let inbox = if need_tasks {
            db.get_inbox(None)
                .await
                .map_err(|e| McpError::database_operation_failed("get_inbox for export", e))?
        } else {
            vec![]
        };
        let today = if need_tasks {
            db.get_today(None)
                .await
                .map_err(|e| McpError::database_operation_failed("get_today for export", e))?
        } else {
            vec![]
        };
        let projects = if need_projects {
            db.get_projects(None)
                .await
                .map_err(|e| McpError::database_operation_failed("get_projects for export", e))?
        } else {
            vec![]
        };
        let areas = if need_areas {
            db.get_areas()
                .await
                .map_err(|e| McpError::database_operation_failed("get_areas for export", e))?
        } else {
            vec![]
        };

        let mut counts = serde_json::Map::new();
        if need_tasks {
            counts.insert("inbox".to_string(), inbox.len().into());
            counts.insert("today".to_string(), today.len().into());
        }
        if need_projects {
            counts.insert("projects".to_string(), projects.len().into());
        }
        if need_areas {
            counts.insert("areas".to_string(), areas.len().into());
        }

        let formatted = match format {
            "json" => {
                let json_val = match data_type {
                    "tasks" => serde_json::json!({ "inbox": &inbox, "today": &today }),
                    "projects" => serde_json::json!({ "projects": &projects }),
                    "areas" => serde_json::json!({ "areas": &areas }),
                    _ => serde_json::json!({
                        "inbox": &inbox,
                        "today": &today,
                        "projects": &projects,
                        "areas": &areas
                    }),
                };
                serde_json::to_string_pretty(&json_val)
                    .map_err(|e| McpError::serialization_failed("export_data json", e))?
            }
            "csv" => {
                let mut all_tasks = inbox;
                all_tasks.extend(today);
                let export_data = ExportData::new(all_tasks, projects, areas);
                DataExporter::new_default()
                    .export(&export_data, ExportFormat::Csv)
                    .map_err(|e| McpError::invalid_parameter("format", e.to_string()))?
            }
            "markdown" => {
                let mut all_tasks = inbox;
                all_tasks.extend(today);
                let export_data = ExportData::new(all_tasks, projects, areas);
                DataExporter::new_default()
                    .export(&export_data, ExportFormat::Markdown)
                    .map_err(|e| McpError::invalid_parameter("format", e.to_string()))?
            }
            _ => return Err(McpError::invalid_format(format, "json, csv, markdown")),
        };

        if let Some(raw_path) = output_path {
            let path = expand_tilde(raw_path)?;
            let bytes = formatted.as_bytes();
            std::fs::write(&path, bytes)
                .map_err(|e| McpError::io_operation_failed("export_data write", e))?;
            let confirmation = serde_json::json!({
                "path": path.to_string_lossy().as_ref(),
                "format": format,
                "data_type": data_type,
                "bytes_written": bytes.len(),
                "counts": serde_json::Value::Object(counts)
            });
            Ok(CallToolResult {
                content: vec![Content::Text {
                    text: serde_json::to_string_pretty(&confirmation).map_err(|e| {
                        McpError::serialization_failed("export_data confirmation", e)
                    })?,
                }],
                is_error: false,
            })
        } else {
            Ok(CallToolResult {
                content: vec![Content::Text { text: formatted }],
                is_error: false,
            })
        }
    }

    pub(in crate::mcp) async fn handle_backup_database(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let backup_dir = args
            .get("backup_dir")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_parameter("backup_dir"))?;
        let description = args.get("description").and_then(|v| v.as_str());

        let backup_path = std::path::Path::new(backup_dir);
        let metadata = self
            .backup_manager
            .lock()
            .await
            .create_backup(backup_path, description)
            .map_err(|e| {
                McpError::backup_operation_failed(
                    "create_backup",
                    things3_core::ThingsError::unknown(e.to_string()),
                )
            })?;

        let response = serde_json::json!({
            "message": "Backup created successfully",
            "backup_path": metadata.backup_path,
            "file_size": metadata.file_size,
            "created_at": metadata.created_at
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("backup_database response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_restore_database(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        // Argument validation runs first so missing-parameter errors keep
        // their existing semantics regardless of the safety gate below.
        let backup_path = args
            .get("backup_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_parameter("backup_path"))?;

        // restore_database overwrites the live Things 3 SQLite file directly —
        // CulturedCode-unsupported and the highest-corruption scenario rust-things3
        // exposes. Decision per #126: keep the tool but gate it on (1) explicit
        // --unsafe-direct-db opt-in AND (2) Things 3 must not be running.
        if !self.unsafe_direct_db {
            return Err(McpError::validation_error(
                "restore_database is gated. Re-launch with --unsafe-direct-db \
                 (or THINGS_UNSAFE_DIRECT_DB=1). It overwrites the live Things 3 \
                 database directly — see https://culturedcode.com/things/support/articles/5510170/",
            ));
        }
        if (self.process_check)() {
            return Err(McpError::validation_error(
                "restore_database refuses to run while Things 3 is open: \
                 overwriting the database file under a running app corrupts it. \
                 Quit Things 3 (Cmd-Q) and retry.",
            ));
        }

        let backup_file = std::path::Path::new(backup_path);
        self.backup_manager
            .lock()
            .await
            .restore_backup(backup_file)
            .map_err(|e| {
                McpError::backup_operation_failed(
                    "restore_backup",
                    things3_core::ThingsError::unknown(e.to_string()),
                )
            })?;

        let response = serde_json::json!({
            "message": "Database restored successfully",
            "backup_path": backup_path
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("restore_database response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_list_backups(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let backup_dir = args
            .get("backup_dir")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_parameter("backup_dir"))?;

        let backup_path = std::path::Path::new(backup_dir);
        let backups = self
            .backup_manager
            .lock()
            .await
            .list_backups(backup_path)
            .map_err(|e| {
                McpError::backup_operation_failed(
                    "list_backups",
                    things3_core::ThingsError::unknown(e.to_string()),
                )
            })?;

        let response = serde_json::json!({
            "backups": backups,
            "count": backups.len()
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("list_backups response", e))?,
            }],
            is_error: false,
        })
    }
}
