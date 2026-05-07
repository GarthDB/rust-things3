use crate::mcp::{CallToolResult, Content, McpError, McpResult, ThingsMcpServer};
use serde_json::Value;
use std::str::FromStr;
use things3_core::models::ThingsId;

impl ThingsMcpServer {
    pub(in crate::mcp) async fn handle_get_projects(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let _area_uuid = args
            .get("area_uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| ThingsId::from_str(s).ok());

        let projects = self
            .db
            .get_projects(None)
            .await
            .map_err(|e| McpError::database_operation_failed("get_projects", e))?;

        let json = serde_json::to_string_pretty(&projects)
            .map_err(|e| McpError::serialization_failed("get_projects serialization", e))?;

        Ok(CallToolResult {
            content: vec![Content::Text { text: json }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_create_project(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_parameter("title", "Project title is required"))?
            .to_string();

        let notes = args.get("notes").and_then(|v| v.as_str()).map(String::from);

        let area_uuid: Option<ThingsId> = args
            .get("area_uuid")
            .and_then(|v| v.as_str())
            .map(|s| {
                ThingsId::from_str(s).map_err(|e| {
                    McpError::invalid_parameter("area_uuid", format!("Invalid ID: {e}"))
                })
            })
            .transpose()?;

        let start_date = args
            .get("start_date")
            .and_then(|v| v.as_str())
            .map(|s| {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| {
                    McpError::invalid_parameter("start_date", format!("Invalid date: {e}"))
                })
            })
            .transpose()?;

        let deadline = args
            .get("deadline")
            .and_then(|v| v.as_str())
            .map(|s| {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| {
                    McpError::invalid_parameter("deadline", format!("Invalid date: {e}"))
                })
            })
            .transpose()?;

        let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        });

        let request = things3_core::models::CreateProjectRequest {
            title,
            notes,
            area_uuid,
            start_date,
            deadline,
            tags,
        };

        let id = self
            .mutations
            .create_project(request)
            .await
            .map_err(|e| McpError::database_operation_failed("create_project", e))?;

        let response = serde_json::json!({
            "message": "Project created successfully",
            "uuid": id.to_string()
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("create_project response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_update_project(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let uuid_str = args
            .get("uuid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_parameter("uuid", "UUID is required"))?;

        let id = ThingsId::from_str(uuid_str)
            .map_err(|e| McpError::invalid_parameter("uuid", format!("Invalid ID: {e}")))?;

        let title = args.get("title").and_then(|v| v.as_str()).map(String::from);
        let notes = args.get("notes").and_then(|v| v.as_str()).map(String::from);

        let area_uuid: Option<ThingsId> = args
            .get("area_uuid")
            .and_then(|v| v.as_str())
            .map(|s| {
                ThingsId::from_str(s).map_err(|e| {
                    McpError::invalid_parameter("area_uuid", format!("Invalid ID: {e}"))
                })
            })
            .transpose()?;

        let start_date = args
            .get("start_date")
            .and_then(|v| v.as_str())
            .map(|s| {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| {
                    McpError::invalid_parameter("start_date", format!("Invalid date: {e}"))
                })
            })
            .transpose()?;

        let deadline = args
            .get("deadline")
            .and_then(|v| v.as_str())
            .map(|s| {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| {
                    McpError::invalid_parameter("deadline", format!("Invalid date: {e}"))
                })
            })
            .transpose()?;

        let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        });

        let request = things3_core::models::UpdateProjectRequest {
            uuid: id,
            title,
            notes,
            area_uuid,
            start_date,
            deadline,
            tags,
        };

        self.mutations
            .update_project(request)
            .await
            .map_err(|e| McpError::database_operation_failed("update_project", e))?;

        let response = serde_json::json!({
            "message": "Project updated successfully",
            "uuid": uuid_str
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("update_project response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_complete_project(
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
            "cascade" => things3_core::models::ProjectChildHandling::Cascade,
            "orphan" => things3_core::models::ProjectChildHandling::Orphan,
            _ => things3_core::models::ProjectChildHandling::Error,
        };

        self.mutations
            .complete_project(&id, child_handling)
            .await
            .map_err(|e| McpError::database_operation_failed("complete_project", e))?;

        let response = serde_json::json!({
            "message": "Project completed successfully",
            "uuid": uuid_str
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("complete_project response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_delete_project(
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
            "cascade" => things3_core::models::ProjectChildHandling::Cascade,
            "orphan" => things3_core::models::ProjectChildHandling::Orphan,
            _ => things3_core::models::ProjectChildHandling::Error,
        };

        self.mutations
            .delete_project(&id, child_handling)
            .await
            .map_err(|e| McpError::database_operation_failed("delete_project", e))?;

        let response = serde_json::json!({
            "message": "Project deleted successfully",
            "uuid": uuid_str
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("delete_project response", e))?,
            }],
            is_error: false,
        })
    }
}
