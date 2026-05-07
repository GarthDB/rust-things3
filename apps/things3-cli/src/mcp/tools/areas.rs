use crate::mcp::{CallToolResult, Content, McpError, McpResult, ThingsMcpServer};
use serde_json::Value;
use std::str::FromStr;
use things3_core::models::ThingsId;

impl ThingsMcpServer {
    pub(in crate::mcp) async fn handle_get_areas(&self, _args: Value) -> McpResult<CallToolResult> {
        let areas = self
            .db
            .get_areas()
            .await
            .map_err(|e| McpError::database_operation_failed("get_areas", e))?;

        let json = serde_json::to_string_pretty(&areas)
            .map_err(|e| McpError::serialization_failed("get_areas serialization", e))?;

        Ok(CallToolResult {
            content: vec![Content::Text { text: json }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_create_area(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_parameter("title", "Area title is required"))?
            .to_string();

        let request = things3_core::models::CreateAreaRequest { title };

        let uuid = self
            .mutations
            .create_area(request)
            .await
            .map_err(|e| McpError::database_operation_failed("create_area", e))?;

        let response = serde_json::json!({
            "message": "Area created successfully",
            "uuid": uuid.to_string()
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("create_area response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_update_area(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let uuid_str = args
            .get("uuid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_parameter("uuid", "UUID is required"))?;

        let id = ThingsId::from_str(uuid_str)
            .map_err(|e| McpError::invalid_parameter("uuid", format!("Invalid ID: {e}")))?;

        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_parameter("title", "Title is required"))?
            .to_string();

        let request = things3_core::models::UpdateAreaRequest { uuid: id, title };

        self.mutations
            .update_area(request)
            .await
            .map_err(|e| McpError::database_operation_failed("update_area", e))?;

        let response = serde_json::json!({
            "message": "Area updated successfully",
            "uuid": uuid_str
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("update_area response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_delete_area(
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
            .delete_area(&id)
            .await
            .map_err(|e| McpError::database_operation_failed("delete_area", e))?;

        let response = serde_json::json!({
            "message": "Area deleted successfully",
            "uuid": uuid_str
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("delete_area response", e))?,
            }],
            is_error: false,
        })
    }
}
