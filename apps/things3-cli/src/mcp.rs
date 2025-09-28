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

/// MCP Prompt for reusable templates
#[derive(Debug, Serialize, Deserialize)]
pub struct Prompt {
    pub name: String,
    pub description: String,
    pub arguments: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListPromptsResult {
    pub prompts: Vec<Prompt>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPromptRequest {
    pub name: String,
    pub arguments: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPromptResult {
    pub content: Vec<Content>,
    pub is_error: bool,
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

    /// List available MCP prompts
    ///
    /// # Errors
    /// Returns an error if prompt generation fails
    pub fn list_prompts(&self) -> Result<ListPromptsResult> {
        Ok(ListPromptsResult {
            prompts: Self::get_available_prompts(),
        })
    }

    /// Get a specific MCP prompt with arguments
    ///
    /// # Errors
    /// Returns an error if prompt retrieval fails or prompt is not found
    pub async fn get_prompt(&self, request: GetPromptRequest) -> Result<GetPromptResult> {
        self.handle_prompt_request(request).await
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

    /// Get available MCP prompts
    fn get_available_prompts() -> Vec<Prompt> {
        vec![
            Self::create_task_review_prompt(),
            Self::create_project_planning_prompt(),
            Self::create_productivity_analysis_prompt(),
            Self::create_backup_strategy_prompt(),
        ]
    }

    /// Create task review prompt
    fn create_task_review_prompt() -> Prompt {
        Prompt {
            name: "task_review".to_string(),
            description: "Review task for completeness and clarity".to_string(),
            arguments: serde_json::json!({
                "type": "object",
                "properties": {
                    "task_title": {
                        "type": "string",
                        "description": "The title of the task to review"
                    },
                    "task_notes": {
                        "type": "string",
                        "description": "Optional notes or description of the task"
                    },
                    "context": {
                        "type": "string",
                        "description": "Optional context about the task or project"
                    }
                },
                "required": ["task_title"]
            }),
        }
    }

    /// Create project planning prompt
    fn create_project_planning_prompt() -> Prompt {
        Prompt {
            name: "project_planning".to_string(),
            description: "Help plan projects with tasks and deadlines".to_string(),
            arguments: serde_json::json!({
                "type": "object",
                "properties": {
                    "project_title": {
                        "type": "string",
                        "description": "The title of the project to plan"
                    },
                    "project_description": {
                        "type": "string",
                        "description": "Description of what the project aims to achieve"
                    },
                    "deadline": {
                        "type": "string",
                        "description": "Optional deadline for the project"
                    },
                    "complexity": {
                        "type": "string",
                        "description": "Project complexity level",
                        "enum": ["simple", "medium", "complex"]
                    }
                },
                "required": ["project_title"]
            }),
        }
    }

    /// Create productivity analysis prompt
    fn create_productivity_analysis_prompt() -> Prompt {
        Prompt {
            name: "productivity_analysis".to_string(),
            description: "Analyze productivity patterns".to_string(),
            arguments: serde_json::json!({
                "type": "object",
                "properties": {
                    "time_period": {
                        "type": "string",
                        "description": "Time period to analyze",
                        "enum": ["week", "month", "quarter", "year"]
                    },
                    "focus_area": {
                        "type": "string",
                        "description": "Specific area to focus analysis on",
                        "enum": ["completion_rate", "time_management", "task_distribution", "all"]
                    },
                    "include_recommendations": {
                        "type": "boolean",
                        "description": "Whether to include improvement recommendations"
                    }
                },
                "required": ["time_period"]
            }),
        }
    }

    /// Create backup strategy prompt
    fn create_backup_strategy_prompt() -> Prompt {
        Prompt {
            name: "backup_strategy".to_string(),
            description: "Suggest backup strategies".to_string(),
            arguments: serde_json::json!({
                "type": "object",
                "properties": {
                    "data_volume": {
                        "type": "string",
                        "description": "Estimated data volume",
                        "enum": ["small", "medium", "large"]
                    },
                    "frequency": {
                        "type": "string",
                        "description": "Desired backup frequency",
                        "enum": ["daily", "weekly", "monthly"]
                    },
                    "retention_period": {
                        "type": "string",
                        "description": "How long to keep backups",
                        "enum": ["1_month", "3_months", "6_months", "1_year", "indefinite"]
                    },
                    "storage_preference": {
                        "type": "string",
                        "description": "Preferred storage type",
                        "enum": ["local", "cloud", "hybrid"]
                    }
                },
                "required": ["data_volume", "frequency"]
            }),
        }
    }

    /// Handle prompt request
    async fn handle_prompt_request(&self, request: GetPromptRequest) -> Result<GetPromptResult> {
        let prompt_name = &request.name;
        let arguments = request.arguments.unwrap_or_default();

        let result = match prompt_name.as_str() {
            "task_review" => self.handle_task_review_prompt(arguments).await,
            "project_planning" => self.handle_project_planning_prompt(arguments).await,
            "productivity_analysis" => self.handle_productivity_analysis_prompt(arguments).await,
            "backup_strategy" => self.handle_backup_strategy_prompt(arguments).await,
            _ => {
                return Ok(GetPromptResult {
                    content: vec![Content::Text {
                        text: format!("Unknown prompt: {prompt_name}"),
                    }],
                    is_error: true,
                });
            }
        };

        match result {
            Ok(prompt_result) => Ok(prompt_result),
            Err(e) => Ok(GetPromptResult {
                content: vec![Content::Text {
                    text: format!("Error: {e}"),
                }],
                is_error: true,
            }),
        }
    }

    /// Handle task review prompt
    async fn handle_task_review_prompt(&self, args: Value) -> Result<GetPromptResult> {
        let task_title = args
            .get("task_title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: task_title"))?;
        let task_notes = args.get("task_notes").and_then(|v| v.as_str());
        let context = args.get("context").and_then(|v| v.as_str());

        // Get current data for context
        let db = self.db.lock().await;
        let inbox_tasks = db.get_inbox(Some(5))?;
        let today_tasks = db.get_today(Some(5))?;
        drop(db);

        let prompt_text = format!(
            "# Task Review: {}\n\n\
            ## Current Task Details\n\
            - **Title**: {}\n\
            - **Notes**: {}\n\
            - **Context**: {}\n\n\
            ## Review Checklist\n\
            Please review this task for:\n\
            1. **Clarity**: Is the task title clear and actionable?\n\
            2. **Completeness**: Does it have all necessary details?\n\
            3. **Priority**: How urgent/important is this task?\n\
            4. **Dependencies**: Are there any prerequisites?\n\
            5. **Time Estimate**: How long should this take?\n\n\
            ## Current Context\n\
            - **Inbox Tasks**: {} tasks\n\
            - **Today's Tasks**: {} tasks\n\n\
            ## Recommendations\n\
            Based on the current workload and task details, provide specific recommendations for:\n\
            - Improving task clarity\n\
            - Breaking down complex tasks\n\
            - Setting appropriate deadlines\n\
            - Managing dependencies\n\n\
            ## Next Steps\n\
            Suggest concrete next steps to move this task forward effectively.",
            task_title,
            task_title,
            task_notes.unwrap_or("No notes provided"),
            context.unwrap_or("No additional context"),
            inbox_tasks.len(),
            today_tasks.len()
        );

        Ok(GetPromptResult {
            content: vec![Content::Text { text: prompt_text }],
            is_error: false,
        })
    }

    /// Handle project planning prompt
    async fn handle_project_planning_prompt(&self, args: Value) -> Result<GetPromptResult> {
        let project_title = args
            .get("project_title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: project_title"))?;
        let project_description = args.get("project_description").and_then(|v| v.as_str());
        let deadline = args.get("deadline").and_then(|v| v.as_str());
        let complexity = args
            .get("complexity")
            .and_then(|v| v.as_str())
            .unwrap_or("medium");

        // Get current data for context
        let db = self.db.lock().await;
        let projects = db.get_projects(None)?;
        let areas = db.get_areas()?;
        drop(db);

        let prompt_text = format!(
            "# Project Planning: {}\n\n\
            ## Project Overview\n\
            - **Title**: {}\n\
            - **Description**: {}\n\
            - **Deadline**: {}\n\
            - **Complexity**: {}\n\n\
            ## Planning Framework\n\
            Please help plan this project by:\n\
            1. **Breaking down** the project into manageable tasks\n\
            2. **Estimating** time requirements for each task\n\
            3. **Identifying** dependencies between tasks\n\
            4. **Suggesting** milestones and checkpoints\n\
            5. **Recommending** project organization (areas, tags, etc.)\n\n\
            ## Current Context\n\
            - **Existing Projects**: {} projects\n\
            - **Available Areas**: {} areas\n\n\
            ## Task Breakdown\n\
            Create a detailed task list with:\n\
            - Clear, actionable task titles\n\
            - Estimated time for each task\n\
            - Priority levels\n\
            - Dependencies\n\
            - Suggested deadlines\n\n\
            ## Project Organization\n\
            Suggest:\n\
            - Appropriate area for this project\n\
            - Useful tags for organization\n\
            - Project structure and hierarchy\n\n\
            ## Risk Assessment\n\
            Identify potential challenges and mitigation strategies.\n\n\
            ## Success Metrics\n\
            Define how to measure project success and completion.",
            project_title,
            project_title,
            project_description.unwrap_or("No description provided"),
            deadline.unwrap_or("No deadline specified"),
            complexity,
            projects.len(),
            areas.len()
        );

        Ok(GetPromptResult {
            content: vec![Content::Text { text: prompt_text }],
            is_error: false,
        })
    }

    /// Handle productivity analysis prompt
    async fn handle_productivity_analysis_prompt(&self, args: Value) -> Result<GetPromptResult> {
        let time_period = args
            .get("time_period")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: time_period"))?;
        let focus_area = args
            .get("focus_area")
            .and_then(|v| v.as_str())
            .unwrap_or("all");
        let include_recommendations = args
            .get("include_recommendations")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        // Get current data for analysis
        let db = self.db.lock().await;
        let inbox_tasks = db.get_inbox(None)?;
        let today_tasks = db.get_today(None)?;
        let projects = db.get_projects(None)?;
        let areas = db.get_areas()?;
        drop(db);

        let completed_tasks = projects
            .iter()
            .filter(|p| p.status == things3_core::TaskStatus::Completed)
            .count();
        let incomplete_tasks = projects
            .iter()
            .filter(|p| p.status == things3_core::TaskStatus::Incomplete)
            .count();

        let prompt_text = format!(
            "# Productivity Analysis - {}\n\n\
            ## Analysis Period: {}\n\
            ## Focus Area: {}\n\n\
            ## Current Data Overview\n\
            - **Inbox Tasks**: {} tasks\n\
            - **Today's Tasks**: {} tasks\n\
            - **Total Projects**: {} projects\n\
            - **Areas**: {} areas\n\
            - **Completed Tasks**: {} tasks\n\
            - **Incomplete Tasks**: {} tasks\n\n\
            ## Analysis Framework\n\
            Please analyze productivity patterns focusing on:\n\n\
            ### 1. Task Completion Patterns\n\
            - Completion rates over the period\n\
            - Task types that are completed vs. delayed\n\
            - Time patterns in task completion\n\n\
            ### 2. Workload Distribution\n\
            - Balance between different areas/projects\n\
            - Task complexity distribution\n\
            - Deadline adherence patterns\n\n\
            ### 3. Time Management\n\
            - Task scheduling effectiveness\n\
            - Inbox vs. scheduled task completion\n\
            - Overdue task patterns\n\n\
            ### 4. Project Progress\n\
            - Project completion rates\n\
            - Project complexity vs. completion time\n\
            - Area-based productivity differences\n\n\
            ## Key Insights\n\
            Identify:\n\
            - Peak productivity times\n\
            - Most/least productive areas\n\
            - Common bottlenecks\n\
            - Success patterns\n\n\
            ## Recommendations\n\
            {}",
            time_period,
            time_period,
            focus_area,
            inbox_tasks.len(),
            today_tasks.len(),
            projects.len(),
            areas.len(),
            completed_tasks,
            incomplete_tasks,
            if include_recommendations {
                "Provide specific, actionable recommendations for:\n\
                - Improving task completion rates\n\
                - Better time management\n\
                - Workload balancing\n\
                - Process optimization\n\
                - Goal setting and tracking"
            } else {
                "Focus on analysis without recommendations"
            }
        );

        Ok(GetPromptResult {
            content: vec![Content::Text { text: prompt_text }],
            is_error: false,
        })
    }

    /// Handle backup strategy prompt
    async fn handle_backup_strategy_prompt(&self, args: Value) -> Result<GetPromptResult> {
        let data_volume = args
            .get("data_volume")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: data_volume"))?;
        let frequency = args
            .get("frequency")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: frequency"))?;
        let retention_period = args
            .get("retention_period")
            .and_then(|v| v.as_str())
            .unwrap_or("3_months");
        let storage_preference = args
            .get("storage_preference")
            .and_then(|v| v.as_str())
            .unwrap_or("hybrid");

        // Get current data for context
        let db = self.db.lock().await;
        let projects = db.get_projects(None)?;
        let areas = db.get_areas()?;
        drop(db);

        let prompt_text = format!(
            "# Backup Strategy Recommendation\n\n\
            ## Requirements\n\
            - **Data Volume**: {}\n\
            - **Backup Frequency**: {}\n\
            - **Retention Period**: {}\n\
            - **Storage Preference**: {}\n\n\
            ## Current Data Context\n\
            - **Projects**: {} projects\n\
            - **Areas**: {} areas\n\
            - **Database Type**: SQLite (Things 3)\n\n\
            ## Backup Strategy Analysis\n\n\
            ### 1. Data Assessment\n\
            Analyze the current data volume and growth patterns:\n\
            - Database size estimation\n\
            - Growth rate projections\n\
            - Critical data identification\n\n\
            ### 2. Backup Frequency Optimization\n\
            For {} frequency backups:\n\
            - Optimal timing considerations\n\
            - Incremental vs. full backup strategy\n\
            - Performance impact analysis\n\n\
            ### 3. Storage Strategy\n\
            For {} storage preference:\n\
            - Local storage recommendations\n\
            - Cloud storage options\n\
            - Hybrid approach benefits\n\
            - Cost considerations\n\n\
            ### 4. Retention Policy\n\
            For {} retention period:\n\
            - Data lifecycle management\n\
            - Compliance considerations\n\
            - Storage optimization\n\n\
            ## Recommended Implementation\n\
            Provide specific recommendations for:\n\
            - Backup tools and software\n\
            - Storage locations and providers\n\
            - Automation setup\n\
            - Monitoring and alerting\n\
            - Recovery procedures\n\n\
            ## Risk Mitigation\n\
            Address:\n\
            - Data loss prevention\n\
            - Backup verification\n\
            - Disaster recovery planning\n\
            - Security considerations\n\n\
            ## Cost Analysis\n\
            Estimate costs for:\n\
            - Storage requirements\n\
            - Backup software/tools\n\
            - Cloud services\n\
            - Maintenance overhead",
            data_volume,
            frequency,
            retention_period,
            storage_preference,
            projects.len(),
            areas.len(),
            frequency,
            storage_preference,
            retention_period
        );

        Ok(GetPromptResult {
            content: vec![Content::Text { text: prompt_text }],
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
