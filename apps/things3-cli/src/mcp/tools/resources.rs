use crate::mcp::{
    Content, McpError, McpResult, ReadResourceRequest, ReadResourceResult, ThingsMcpServer,
};

impl ThingsMcpServer {
    /// Handle resource read request
    pub(in crate::mcp) async fn handle_resource_read(
        &self,
        request: ReadResourceRequest,
    ) -> McpResult<ReadResourceResult> {
        let uri = &request.uri;

        let db = &self.db;
        let data = match uri.as_str() {
            "things://inbox" => {
                let tasks = db.get_inbox(None).await.map_err(|e| {
                    McpError::database_operation_failed("get_inbox for resource", e)
                })?;
                serde_json::to_string_pretty(&tasks).map_err(|e| {
                    McpError::serialization_failed("inbox resource serialization", e)
                })?
            }
            "things://projects" => {
                let projects = db.get_projects(None).await.map_err(|e| {
                    McpError::database_operation_failed("get_projects for resource", e)
                })?;
                serde_json::to_string_pretty(&projects).map_err(|e| {
                    McpError::serialization_failed("projects resource serialization", e)
                })?
            }
            "things://areas" => {
                let areas = db.get_areas().await.map_err(|e| {
                    McpError::database_operation_failed("get_areas for resource", e)
                })?;
                serde_json::to_string_pretty(&areas).map_err(|e| {
                    McpError::serialization_failed("areas resource serialization", e)
                })?
            }
            "things://today" => {
                let tasks = db.get_today(None).await.map_err(|e| {
                    McpError::database_operation_failed("get_today for resource", e)
                })?;
                let _ = db;
                serde_json::to_string_pretty(&tasks).map_err(|e| {
                    McpError::serialization_failed("today resource serialization", e)
                })?
            }
            _ => {
                return Err(McpError::resource_not_found(uri));
            }
        };

        Ok(ReadResourceResult {
            contents: vec![Content::Text { text: data }],
        })
    }
}
