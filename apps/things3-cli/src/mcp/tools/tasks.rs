use crate::mcp::{CallToolResult, Content, McpError, McpResult, ThingsMcpServer};
use serde_json::Value;
use std::str::FromStr;
use things3_core::models::{DeleteChildHandling, ThingsId};

impl ThingsMcpServer {
    pub(in crate::mcp) async fn handle_get_inbox(&self, args: Value) -> McpResult<CallToolResult> {
        let limit = args
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .map(|v| usize::try_from(v).unwrap_or(usize::MAX));

        let tasks = self
            .db
            .get_inbox(limit)
            .await
            .map_err(|e| McpError::database_operation_failed("get_inbox", e))?;

        let json = serde_json::to_string_pretty(&tasks)
            .map_err(|e| McpError::serialization_failed("get_inbox serialization", e))?;

        Ok(CallToolResult {
            content: vec![Content::Text { text: json }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_get_today(&self, args: Value) -> McpResult<CallToolResult> {
        let limit = args
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .map(|v| usize::try_from(v).unwrap_or(usize::MAX));

        let tasks = self.db.get_today(limit).await.map_err(|e| {
            // Include the actual error message for debugging
            McpError::database_operation_failed(
                "get_today",
                things3_core::ThingsError::unknown(format!("Failed to get today's tasks: {}", e)),
            )
        })?;

        let json = serde_json::to_string_pretty(&tasks)
            .map_err(|e| McpError::serialization_failed("get_today serialization", e))?;

        Ok(CallToolResult {
            content: vec![Content::Text { text: json }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_search_tasks(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_parameter("query"))?;

        let _limit = args
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .map(|v| usize::try_from(v).unwrap_or(usize::MAX));

        let tasks = self
            .db
            .search_tasks(query)
            .await
            .map_err(|e| McpError::database_operation_failed("search_tasks", e))?;

        let json = serde_json::to_string_pretty(&tasks)
            .map_err(|e| McpError::serialization_failed("search_tasks serialization", e))?;

        Ok(CallToolResult {
            content: vec![Content::Text { text: json }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_logbook_search(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        // Parse all optional parameters
        let search_text = args
            .get("search_text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let from_date = args
            .get("from_date")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        let to_date = args
            .get("to_date")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        let project_uuid: Option<ThingsId> = args
            .get("project_uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| ThingsId::from_str(s).ok());

        let area_uuid: Option<ThingsId> = args
            .get("area_uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| ThingsId::from_str(s).ok());

        let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        });

        let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as u32);
        let offset = args
            .get("offset")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        // Call database method
        let tasks = self
            .db
            .search_logbook(
                search_text,
                from_date,
                to_date,
                project_uuid,
                area_uuid,
                tags,
                limit,
                offset,
            )
            .await
            .map_err(|e| McpError::database_operation_failed("logbook_search", e))?;

        // Serialize results
        let json = serde_json::to_string_pretty(&tasks)
            .map_err(|e| McpError::serialization_failed("logbook_search serialization", e))?;

        Ok(CallToolResult {
            content: vec![Content::Text { text: json }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_create_task(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        // Parse request from JSON
        let request: things3_core::CreateTaskRequest =
            serde_json::from_value(args).map_err(|e| {
                McpError::invalid_parameter(
                    "request",
                    format!("Failed to parse create task request: {e}"),
                )
            })?;

        // Create task
        let uuid = self
            .mutations
            .create_task(request)
            .await
            .map_err(|e| McpError::database_operation_failed("create_task", e))?;

        // Return created task UUID
        let response = serde_json::json!({
            "uuid": uuid,
            "message": "Task created successfully"
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("create_task response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_update_task(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        // Parse request from JSON
        let request: things3_core::UpdateTaskRequest =
            serde_json::from_value(args).map_err(|e| {
                McpError::invalid_parameter(
                    "request",
                    format!("Failed to parse update task request: {e}"),
                )
            })?;

        // Update task
        self.mutations
            .update_task(request)
            .await
            .map_err(|e| McpError::database_operation_failed("update_task", e))?;

        // Return success
        let response = serde_json::json!({
            "message": "Task updated successfully"
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("update_task response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_complete_task(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let uuid_str = args
            .get("uuid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_parameter("uuid", "UUID is required"))?;

        let id = ThingsId::from_str(uuid_str)
            .map_err(|e| McpError::invalid_parameter("uuid", format!("Invalid ID: {e}")))?;

        self.mutations
            .complete_task(&id)
            .await
            .map_err(|e| McpError::database_operation_failed("complete_task", e))?;

        let response = serde_json::json!({
            "message": "Task completed successfully",
            "uuid": uuid_str
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("complete_task response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_uncomplete_task(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let uuid_str = args
            .get("uuid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_parameter("uuid", "UUID is required"))?;

        let id = ThingsId::from_str(uuid_str)
            .map_err(|e| McpError::invalid_parameter("uuid", format!("Invalid ID: {e}")))?;

        self.mutations
            .uncomplete_task(&id)
            .await
            .map_err(|e| McpError::database_operation_failed("uncomplete_task", e))?;

        let response = serde_json::json!({
            "message": "Task marked as incomplete successfully",
            "uuid": uuid_str
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("uncomplete_task response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_delete_task(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let uuid_str = args
            .get("uuid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_parameter("uuid", "UUID is required"))?;

        let id = ThingsId::from_str(uuid_str)
            .map_err(|e| McpError::invalid_parameter("uuid", format!("Invalid ID: {e}")))?;

        let child_handling_str = args
            .get("child_handling")
            .and_then(|v| v.as_str())
            .unwrap_or("error");

        let child_handling = match child_handling_str {
            "cascade" => DeleteChildHandling::Cascade,
            "orphan" => DeleteChildHandling::Orphan,
            _ => DeleteChildHandling::Error,
        };

        self.mutations
            .delete_task(&id, child_handling)
            .await
            .map_err(|e| McpError::database_operation_failed("delete_task", e))?;

        let response = serde_json::json!({
            "message": "Task deleted successfully",
            "uuid": uuid_str
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("delete_task response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_bulk_move(&self, args: Value) -> McpResult<CallToolResult> {
        // Parse task UUIDs
        let task_uuid_strs: Vec<String> = args
            .get("task_uuids")
            .and_then(|v| v.as_array())
            .ok_or_else(|| McpError::invalid_parameter("task_uuids", "Array of UUIDs is required"))?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        let task_uuids: Vec<ThingsId> = task_uuid_strs
            .iter()
            .map(|s| {
                ThingsId::from_str(s).map_err(|e| {
                    McpError::invalid_parameter("task_uuids", format!("Invalid ID: {e}"))
                })
            })
            .collect::<McpResult<Vec<_>>>()?;

        // Parse optional project_uuid
        let project_uuid: Option<ThingsId> = args
            .get("project_uuid")
            .and_then(|v| v.as_str())
            .map(|s| {
                ThingsId::from_str(s).map_err(|e| {
                    McpError::invalid_parameter("project_uuid", format!("Invalid ID: {e}"))
                })
            })
            .transpose()?;

        // Parse optional area_uuid
        let area_uuid: Option<ThingsId> = args
            .get("area_uuid")
            .and_then(|v| v.as_str())
            .map(|s| {
                ThingsId::from_str(s).map_err(|e| {
                    McpError::invalid_parameter("area_uuid", format!("Invalid ID: {e}"))
                })
            })
            .transpose()?;

        let request = things3_core::models::BulkMoveRequest {
            task_uuids,
            project_uuid,
            area_uuid,
        };

        let result = self
            .mutations
            .bulk_move(request)
            .await
            .map_err(|e| McpError::database_operation_failed("bulk_move", e))?;

        let response = serde_json::json!({
            "success": result.success,
            "processed_count": result.processed_count,
            "message": result.message
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("bulk_move response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_bulk_update_dates(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        use chrono::NaiveDate;

        // Parse task UUIDs
        let task_uuid_strs: Vec<String> = args
            .get("task_uuids")
            .and_then(|v| v.as_array())
            .ok_or_else(|| McpError::invalid_parameter("task_uuids", "Array of UUIDs is required"))?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        let task_uuids: Vec<ThingsId> = task_uuid_strs
            .iter()
            .map(|s| {
                ThingsId::from_str(s).map_err(|e| {
                    McpError::invalid_parameter("task_uuids", format!("Invalid ID: {e}"))
                })
            })
            .collect::<McpResult<Vec<_>>>()?;

        // Parse optional dates
        let start_date = args
            .get("start_date")
            .and_then(|v| v.as_str())
            .map(|s| {
                NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| {
                    McpError::invalid_parameter("start_date", format!("Invalid date format: {e}"))
                })
            })
            .transpose()?;

        let deadline = args
            .get("deadline")
            .and_then(|v| v.as_str())
            .map(|s| {
                NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| {
                    McpError::invalid_parameter("deadline", format!("Invalid date format: {e}"))
                })
            })
            .transpose()?;

        let clear_start_date = args
            .get("clear_start_date")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let clear_deadline = args
            .get("clear_deadline")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let request = things3_core::models::BulkUpdateDatesRequest {
            task_uuids,
            start_date,
            deadline,
            clear_start_date,
            clear_deadline,
        };

        let result = self
            .mutations
            .bulk_update_dates(request)
            .await
            .map_err(|e| McpError::database_operation_failed("bulk_update_dates", e))?;

        let response = serde_json::json!({
            "success": result.success,
            "processed_count": result.processed_count,
            "message": result.message
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("bulk_update_dates response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_bulk_complete(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        // Parse task UUIDs
        let task_uuid_strs: Vec<String> = args
            .get("task_uuids")
            .and_then(|v| v.as_array())
            .ok_or_else(|| McpError::invalid_parameter("task_uuids", "Array of UUIDs is required"))?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        let task_uuids: Vec<ThingsId> = task_uuid_strs
            .iter()
            .map(|s| {
                ThingsId::from_str(s).map_err(|e| {
                    McpError::invalid_parameter("task_uuids", format!("Invalid ID: {e}"))
                })
            })
            .collect::<McpResult<Vec<_>>>()?;

        let request = things3_core::models::BulkCompleteRequest { task_uuids };

        let result = self
            .mutations
            .bulk_complete(request)
            .await
            .map_err(|e| McpError::database_operation_failed("bulk_complete", e))?;

        let response = serde_json::json!({
            "success": result.success,
            "processed_count": result.processed_count,
            "message": result.message
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("bulk_complete response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_bulk_delete(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        // Parse task UUIDs
        let task_uuid_strs: Vec<String> = args
            .get("task_uuids")
            .and_then(|v| v.as_array())
            .ok_or_else(|| McpError::invalid_parameter("task_uuids", "Array of UUIDs is required"))?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        let task_uuids: Vec<ThingsId> = task_uuid_strs
            .iter()
            .map(|s| {
                ThingsId::from_str(s).map_err(|e| {
                    McpError::invalid_parameter("task_uuids", format!("Invalid ID: {e}"))
                })
            })
            .collect::<McpResult<Vec<_>>>()?;

        let request = things3_core::models::BulkDeleteRequest { task_uuids };

        let result = self
            .mutations
            .bulk_delete(request)
            .await
            .map_err(|e| McpError::database_operation_failed("bulk_delete", e))?;

        let response = serde_json::json!({
            "success": result.success,
            "processed_count": result.processed_count,
            "message": result.message
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("bulk_delete response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_bulk_create_tasks(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        // Validate top-level shape before delegating so we keep the historical
        // "missing tasks" error contract — `serde_json::from_value` would also
        // reject this, but with a less actionable error.
        args.get("tasks")
            .and_then(|v| v.as_array())
            .ok_or_else(|| McpError::missing_parameter("tasks"))?;

        let request: things3_core::models::BulkCreateTasksRequest = serde_json::from_value(args)
            .map_err(|e| {
                McpError::invalid_parameter(
                    "tasks",
                    format!("Failed to parse bulk_create_tasks request: {e}"),
                )
            })?;

        let result = self
            .mutations
            .bulk_create_tasks(request)
            .await
            .map_err(|e| McpError::database_operation_failed("bulk_create_tasks", e))?;

        let response = serde_json::json!({
            "success": result.success,
            "processed_count": result.processed_count,
            "message": result.message,
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("bulk_create_tasks response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_get_recent_tasks(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
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
        let tasks = self
            .db
            .get_inbox(limit)
            .await
            .map_err(|e| McpError::database_operation_failed("get_recent_tasks", e))?;

        let response = serde_json::json!({
            "message": "Recent tasks (using inbox as proxy)",
            "hours_lookback": hours,
            "tasks": tasks
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("get_recent_tasks response", e))?,
            }],
            is_error: false,
        })
    }
}
