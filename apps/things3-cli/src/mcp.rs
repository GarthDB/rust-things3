//! MCP (Model Context Protocol) server implementation for Things 3 integration

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use things3_core::{
    BackupManager, DataExporter, PerformanceMonitor, ThingsCache, ThingsConfig, ThingsDatabase,
};
use tokio::sync::Mutex;

/// Simplified MCP types for our implementation
#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallToolRequest {
    pub name: String,
    pub arguments: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<Content>,
    pub is_error: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Content {
    Text { text: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
}

/// MCP Resource for data exposure
#[derive(Debug, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListResourcesResult {
    pub resources: Vec<Resource>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadResourceRequest {
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadResourceResult {
    pub contents: Vec<Content>,
}

/// MCP server for Things 3 integration
pub struct ThingsMcpServer {
    #[allow(dead_code)]
    db: Arc<Mutex<ThingsDatabase>>,
    #[allow(dead_code)]
    cache: Arc<Mutex<ThingsCache>>,
    #[allow(dead_code)]
    performance_monitor: Arc<Mutex<PerformanceMonitor>>,
    #[allow(dead_code)]
    exporter: DataExporter,
    #[allow(dead_code)]
    backup_manager: Arc<Mutex<BackupManager>>,
}

#[allow(dead_code)]
impl ThingsMcpServer {
    pub fn new(db: ThingsDatabase, config: ThingsConfig) -> Self {
        let cache = ThingsCache::new_default();
        let performance_monitor = PerformanceMonitor::new_default();
        let exporter = DataExporter::new_default();
        let backup_manager = BackupManager::new(config);

        Self {
            db: Arc::new(Mutex::new(db)),
            cache: Arc::new(Mutex::new(cache)),
            performance_monitor: Arc::new(Mutex::new(performance_monitor)),
            exporter,
            backup_manager: Arc::new(Mutex::new(backup_manager)),
        }
    }

    /// List available MCP tools
    ///
    /// # Errors
    /// Returns an error if tool generation fails
    pub fn list_tools(&self) -> Result<ListToolsResult> {
        Ok(ListToolsResult {
            tools: Self::get_available_tools(),
        })
    }

    /// Call a specific MCP tool
    ///
    /// # Errors
    /// Returns an error if tool execution fails or tool is not found
    pub async fn call_tool(&self, request: CallToolRequest) -> Result<CallToolResult> {
        self.handle_tool_call(request).await
    }

    /// List available MCP resources
    ///
    /// # Errors
    /// Returns an error if resource generation fails
    pub fn list_resources(&self) -> Result<ListResourcesResult> {
        Ok(ListResourcesResult {
            resources: Self::get_available_resources(),
        })
    }

    /// Read a specific MCP resource
    ///
    /// # Errors
    /// Returns an error if resource reading fails or resource is not found
    pub async fn read_resource(&self, request: ReadResourceRequest) -> Result<ReadResourceResult> {
        self.handle_resource_read(request).await
    }

    /// Get available MCP tools
    fn get_available_tools() -> Vec<Tool> {
        let mut tools = Vec::new();
        tools.extend(Self::get_data_retrieval_tools());
        tools.extend(Self::get_task_management_tools());
        tools.extend(Self::get_analytics_tools());
        tools.extend(Self::get_backup_tools());
        tools.extend(Self::get_system_tools());
        tools
    }

    fn get_data_retrieval_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "get_inbox".to_string(),
                description: "Get tasks from the inbox".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of tasks to return"
                        }
                    }
                }),
            },
            Tool {
                name: "get_today".to_string(),
                description: "Get tasks scheduled for today".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of tasks to return"
                        }
                    }
                }),
            },
            Tool {
                name: "get_projects".to_string(),
                description: "Get all projects, optionally filtered by area".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "area_uuid": {
                            "type": "string",
                            "description": "Optional area UUID to filter projects"
                        }
                    }
                }),
            },
            Tool {
                name: "get_areas".to_string(),
                description: "Get all areas".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "search_tasks".to_string(),
                description: "Search for tasks by query".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of tasks to return"
                        }
                    },
                    "required": ["query"]
                }),
            },
            Tool {
                name: "get_recent_tasks".to_string(),
                description: "Get recently created or modified tasks".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of tasks to return"
                        },
                        "hours": {
                            "type": "integer",
                            "description": "Number of hours to look back"
                        }
                    }
                }),
            },
        ]
    }

    fn get_task_management_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "create_task".to_string(),
                description: "Create a new task".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Task title"
                        },
                        "notes": {
                            "type": "string",
                            "description": "Optional task notes"
                        },
                        "project_uuid": {
                            "type": "string",
                            "description": "Optional project UUID"
                        },
                        "area_uuid": {
                            "type": "string",
                            "description": "Optional area UUID"
                        }
                    },
                    "required": ["title"]
                }),
            },
            Tool {
                name: "update_task".to_string(),
                description: "Update an existing task".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uuid": {
                            "type": "string",
                            "description": "Task UUID"
                        },
                        "title": {
                            "type": "string",
                            "description": "New task title"
                        },
                        "notes": {
                            "type": "string",
                            "description": "New task notes"
                        },
                        "status": {
                            "type": "string",
                            "description": "New task status",
                            "enum": ["incomplete", "completed", "canceled", "trashed"]
                        }
                    },
                    "required": ["uuid"]
                }),
            },
            Tool {
                name: "bulk_create_tasks".to_string(),
                description: "Create multiple tasks at once".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "tasks": {
                            "type": "array",
                            "description": "Array of task objects to create",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "title": {"type": "string"},
                                    "notes": {"type": "string"},
                                    "project_uuid": {"type": "string"},
                                    "area_uuid": {"type": "string"}
                                },
                                "required": ["title"]
                            }
                        }
                    },
                    "required": ["tasks"]
                }),
            },
        ]
    }

    fn get_analytics_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "get_productivity_metrics".to_string(),
                description: "Get productivity metrics and statistics".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "days": {
                            "type": "integer",
                            "description": "Number of days to look back for metrics"
                        }
                    }
                }),
            },
            Tool {
                name: "export_data".to_string(),
                description: "Export data in various formats".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "format": {
                            "type": "string",
                            "description": "Export format",
                            "enum": ["json", "csv", "markdown"]
                        },
                        "data_type": {
                            "type": "string",
                            "description": "Type of data to export",
                            "enum": ["tasks", "projects", "areas", "all"]
                        }
                    },
                    "required": ["format", "data_type"]
                }),
            },
        ]
    }

    fn get_backup_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "backup_database".to_string(),
                description: "Create a backup of the Things 3 database".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "backup_dir": {
                            "type": "string",
                            "description": "Directory to store the backup"
                        },
                        "description": {
                            "type": "string",
                            "description": "Optional description for the backup"
                        }
                    },
                    "required": ["backup_dir"]
                }),
            },
            Tool {
                name: "restore_database".to_string(),
                description: "Restore from a backup".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "backup_path": {
                            "type": "string",
                            "description": "Path to the backup file"
                        }
                    },
                    "required": ["backup_path"]
                }),
            },
            Tool {
                name: "list_backups".to_string(),
                description: "List available backups".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "backup_dir": {
                            "type": "string",
                            "description": "Directory containing backups"
                        }
                    },
                    "required": ["backup_dir"]
                }),
            },
        ]
    }

    fn get_system_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "get_performance_stats".to_string(),
                description: "Get performance statistics and metrics".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "get_system_metrics".to_string(),
                description: "Get current system resource metrics".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "get_cache_stats".to_string(),
                description: "Get cache statistics and hit rates".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }

    /// Handle tool call
    async fn handle_tool_call(&self, request: CallToolRequest) -> Result<CallToolResult> {
        let tool_name = &request.name;
        let arguments = request.arguments.unwrap_or_default();

        let result = match tool_name.as_str() {
            "get_inbox" => self.handle_get_inbox(arguments).await,
            "get_today" => self.handle_get_today(arguments).await,
            "get_projects" => self.handle_get_projects(arguments).await,
            "get_areas" => self.handle_get_areas(arguments).await,
            "search_tasks" => self.handle_search_tasks(arguments).await,
            "create_task" => Self::handle_create_task(&arguments),
            "update_task" => Self::handle_update_task(&arguments),
            "get_productivity_metrics" => self.handle_get_productivity_metrics(arguments).await,
            "export_data" => self.handle_export_data(arguments).await,
            "bulk_create_tasks" => Self::handle_bulk_create_tasks(&arguments),
            "get_recent_tasks" => self.handle_get_recent_tasks(arguments).await,
            "backup_database" => self.handle_backup_database(arguments).await,
            "restore_database" => self.handle_restore_database(arguments).await,
            "list_backups" => self.handle_list_backups(arguments).await,
            "get_performance_stats" => self.handle_get_performance_stats(arguments).await,
            "get_system_metrics" => self.handle_get_system_metrics(arguments).await,
            "get_cache_stats" => self.handle_get_cache_stats(arguments).await,
            _ => {
                return Ok(CallToolResult {
                    content: vec![Content::Text {
                        text: format!("Unknown tool: {tool_name}"),
                    }],
                    is_error: true,
                });
            }
        };

        match result {
            Ok(call_result) => Ok(call_result),
            Err(e) => Ok(CallToolResult {
                content: vec![Content::Text {
                    text: format!("Error: {e}"),
                }],
                is_error: true,
            }),
        }
    }

    async fn handle_get_inbox(&self, args: Value) -> Result<CallToolResult> {
        let limit = args
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .map(|v| usize::try_from(v).unwrap_or(usize::MAX));
        let tasks = self.db.lock().await.get_inbox(limit)?;
        let json = serde_json::to_string_pretty(&tasks)?;
        Ok(CallToolResult {
            content: vec![Content::Text { text: json }],
            is_error: false,
        })
    }

    async fn handle_get_today(&self, args: Value) -> Result<CallToolResult> {
        let limit = args
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .map(|v| usize::try_from(v).unwrap_or(usize::MAX));
        let tasks = self.db.lock().await.get_today(limit)?;
        let json = serde_json::to_string_pretty(&tasks)?;
        Ok(CallToolResult {
            content: vec![Content::Text { text: json }],
            is_error: false,
        })
    }

    async fn handle_get_projects(&self, args: Value) -> Result<CallToolResult> {
        let area_uuid = args
            .get("area_uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok());
        let projects = self.db.lock().await.get_projects(area_uuid)?;
        let json = serde_json::to_string_pretty(&projects)?;
        Ok(CallToolResult {
            content: vec![Content::Text { text: json }],
            is_error: false,
        })
    }

    async fn handle_get_areas(&self, _args: Value) -> Result<CallToolResult> {
        let areas = self.db.lock().await.get_areas()?;
        let json = serde_json::to_string_pretty(&areas)?;
        Ok(CallToolResult {
            content: vec![Content::Text { text: json }],
            is_error: false,
        })
    }

    async fn handle_search_tasks(&self, args: Value) -> Result<CallToolResult> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: query"))?;
        let limit = args
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .map(|v| usize::try_from(v).unwrap_or(usize::MAX));
        let tasks = self.db.lock().await.search_tasks(query, limit)?;
        let json = serde_json::to_string_pretty(&tasks)?;
        Ok(CallToolResult {
            content: vec![Content::Text { text: json }],
            is_error: false,
        })
    }

    fn handle_create_task(args: &Value) -> Result<CallToolResult> {
        // Note: This is a placeholder - actual task creation would need to be implemented
        // in the things-core library
        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: title"))?;

        let response = serde_json::json!({
            "message": "Task creation not yet implemented",
            "title": title,
            "status": "placeholder"
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)?,
            }],
            is_error: false,
        })
    }

    fn handle_update_task(args: &Value) -> Result<CallToolResult> {
        // Note: This is a placeholder - actual task updating would need to be implemented
        // in the things-core library
        let uuid = args
            .get("uuid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: uuid"))?;

        let response = serde_json::json!({
            "message": "Task updating not yet implemented",
            "uuid": uuid,
            "status": "placeholder"
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)?,
            }],
            is_error: false,
        })
    }

    async fn handle_get_productivity_metrics(&self, args: Value) -> Result<CallToolResult> {
        let days = usize::try_from(
            args.get("days")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(7),
        )
        .unwrap_or(7);

        // Get various metrics
        let db = self.db.lock().await;
        let inbox_tasks = db.get_inbox(None)?;
        let today_tasks = db.get_today(None)?;
        let projects = db.get_projects(None)?;
        let areas = db.get_areas()?;
        drop(db);

        let metrics = serde_json::json!({
            "period_days": days,
            "inbox_tasks_count": inbox_tasks.len(),
            "today_tasks_count": today_tasks.len(),
            "projects_count": projects.len(),
            "areas_count": areas.len(),
            "completed_tasks": projects.iter().filter(|p| p.status == things3_core::TaskStatus::Completed).count(),
            "incomplete_tasks": projects.iter().filter(|p| p.status == things3_core::TaskStatus::Incomplete).count(),
            "timestamp": chrono::Utc::now()
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&metrics)?,
            }],
            is_error: false,
        })
    }

    async fn handle_export_data(&self, args: Value) -> Result<CallToolResult> {
        let format = args
            .get("format")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: format"))?;
        let data_type = args
            .get("data_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: data_type"))?;

        let db = self.db.lock().await;
        let export_data = match data_type {
            "tasks" => {
                let inbox = db.get_inbox(None)?;
                let today = db.get_today(None)?;
                serde_json::json!({
                    "inbox": inbox,
                    "today": today
                })
            }
            "projects" => {
                let projects = db.get_projects(None)?;
                serde_json::json!({ "projects": projects })
            }
            "areas" => {
                let areas = db.get_areas()?;
                serde_json::json!({ "areas": areas })
            }
            "all" => {
                let inbox = db.get_inbox(None)?;
                let today = db.get_today(None)?;
                let projects = db.get_projects(None)?;
                let areas = db.get_areas()?;
                drop(db);
                serde_json::json!({
                    "inbox": inbox,
                    "today": today,
                    "projects": projects,
                    "areas": areas
                })
            }
            _ => return Err(anyhow::anyhow!("Invalid data_type: {data_type}")),
        };

        let result = match format {
            "json" => serde_json::to_string_pretty(&export_data)?,
            "csv" => "CSV export not yet implemented".to_string(),
            "markdown" => "Markdown export not yet implemented".to_string(),
            _ => return Err(anyhow::anyhow!("Invalid format: {}", format)),
        };

        Ok(CallToolResult {
            content: vec![Content::Text { text: result }],
            is_error: false,
        })
    }

    fn handle_bulk_create_tasks(args: &Value) -> Result<CallToolResult> {
        let tasks = args
            .get("tasks")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: tasks"))?;

        let response = serde_json::json!({
            "message": "Bulk task creation not yet implemented",
            "tasks_count": tasks.len(),
            "status": "placeholder"
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)?,
            }],
            is_error: false,
        })
    }

    async fn handle_get_recent_tasks(&self, args: Value) -> Result<CallToolResult> {
        let limit = args
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .map(|v| usize::try_from(v).unwrap_or(usize::MAX));
        let hours = i64::try_from(
            args.get("hours")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(24),
        )
        .unwrap_or(24);

        // For now, return inbox tasks as a proxy for recent tasks
        // In a real implementation, this would query by creation/modification date
        let tasks = self.db.lock().await.get_inbox(limit)?;

        let response = serde_json::json!({
            "message": "Recent tasks (using inbox as proxy)",
            "hours_lookback": hours,
            "tasks": tasks
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)?,
            }],
            is_error: false,
        })
    }

    async fn handle_backup_database(&self, args: Value) -> Result<CallToolResult> {
        let backup_dir = args
            .get("backup_dir")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: backup_dir"))?;
        let description = args.get("description").and_then(|v| v.as_str());

        let backup_path = std::path::Path::new(backup_dir);
        let metadata = self
            .backup_manager
            .lock()
            .await
            .create_backup(backup_path, description)
            .await?;

        let response = serde_json::json!({
            "message": "Backup created successfully",
            "backup_path": metadata.backup_path,
            "file_size": metadata.file_size,
            "created_at": metadata.created_at
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)?,
            }],
            is_error: false,
        })
    }

    async fn handle_restore_database(&self, args: Value) -> Result<CallToolResult> {
        let backup_path = args
            .get("backup_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: backup_path"))?;

        let backup_file = std::path::Path::new(backup_path);
        self.backup_manager
            .lock()
            .await
            .restore_backup(backup_file)
            .await?;

        let response = serde_json::json!({
            "message": "Database restored successfully",
            "backup_path": backup_path
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)?,
            }],
            is_error: false,
        })
    }

    async fn handle_list_backups(&self, args: Value) -> Result<CallToolResult> {
        let backup_dir = args
            .get("backup_dir")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: backup_dir"))?;

        let backup_path = std::path::Path::new(backup_dir);
        let backups = self.backup_manager.lock().await.list_backups(backup_path)?;

        let response = serde_json::json!({
            "backups": backups,
            "count": backups.len()
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)?,
            }],
            is_error: false,
        })
    }

    async fn handle_get_performance_stats(&self, _args: Value) -> Result<CallToolResult> {
        let monitor = self.performance_monitor.lock().await;
        let stats = monitor.get_all_stats();
        let summary = monitor.get_summary();
        drop(monitor);

        let response = serde_json::json!({
            "summary": summary,
            "operation_stats": stats
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)?,
            }],
            is_error: false,
        })
    }

    async fn handle_get_system_metrics(&self, _args: Value) -> Result<CallToolResult> {
        let metrics = self.performance_monitor.lock().await.get_system_metrics()?;

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&metrics)?,
            }],
            is_error: false,
        })
    }

    async fn handle_get_cache_stats(&self, _args: Value) -> Result<CallToolResult> {
        let stats = self.cache.lock().await.get_stats();

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&stats)?,
            }],
            is_error: false,
        })
    }

    /// Get available MCP resources
    fn get_available_resources() -> Vec<Resource> {
        vec![
            Resource {
                uri: "things://inbox".to_string(),
                name: "Inbox Tasks".to_string(),
                description: "Current inbox tasks from Things 3".to_string(),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "things://projects".to_string(),
                name: "All Projects".to_string(),
                description: "All projects in Things 3".to_string(),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "things://areas".to_string(),
                name: "All Areas".to_string(),
                description: "All areas in Things 3".to_string(),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "things://today".to_string(),
                name: "Today's Tasks".to_string(),
                description: "Tasks scheduled for today".to_string(),
                mime_type: Some("application/json".to_string()),
            },
        ]
    }

    /// Handle resource read request
    async fn handle_resource_read(
        &self,
        request: ReadResourceRequest,
    ) -> Result<ReadResourceResult> {
        let uri = &request.uri;

        let db = self.db.lock().await;
        let data = match uri.as_str() {
            "things://inbox" => {
                let tasks = db.get_inbox(None)?;
                serde_json::to_string_pretty(&tasks)?
            }
            "things://projects" => {
                let projects = db.get_projects(None)?;
                serde_json::to_string_pretty(&projects)?
            }
            "things://areas" => {
                let areas = db.get_areas()?;
                serde_json::to_string_pretty(&areas)?
            }
            "things://today" => {
                let tasks = db.get_today(None)?;
                drop(db);
                serde_json::to_string_pretty(&tasks)?
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown resource: {uri}"));
            }
        };

        Ok(ReadResourceResult {
            contents: vec![Content::Text { text: data }],
        })
    }
}
