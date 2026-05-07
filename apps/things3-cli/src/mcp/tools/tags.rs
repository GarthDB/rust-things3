use crate::mcp::{CallToolResult, Content, McpError, McpResult, ThingsMcpServer};
use serde_json::Value;
use std::str::FromStr;
use things3_core::models::ThingsId;
use tracing::warn;

impl ThingsMcpServer {
    pub(in crate::mcp) async fn handle_search_tags_tool(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let query: String = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_parameter("query", "Missing 'query' parameter"))?
            .to_string();

        let include_similar = args
            .get("include_similar")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let min_similarity = args
            .get("min_similarity")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7) as f32;

        let tags = if include_similar {
            self.db
                .find_similar_tags(&query, min_similarity)
                .await
                .map_err(|e| McpError::database_operation_failed("search_tags", e))?
                .into_iter()
                .map(|tm| tm.tag)
                .collect()
        } else {
            self.db
                .search_tags(&query)
                .await
                .map_err(|e| McpError::database_operation_failed("search_tags", e))?
        };

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&tags)
                    .map_err(|e| McpError::serialization_failed("tags", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_get_tag_suggestions(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let title: String = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_parameter("title", "Missing 'title' parameter"))?
            .to_string();

        use things3_core::database::tag_utils::normalize_tag_title;
        let normalized = normalize_tag_title(&title);

        // Check for exact match
        let exact_match = self
            .db
            .find_tag_by_normalized_title(&normalized)
            .await
            .map_err(|e| McpError::database_operation_failed("get_tag_suggestions", e))?;

        // Find similar tags
        let similar_tags = self
            .db
            .find_similar_tags(&normalized, 0.7)
            .await
            .map_err(|e| McpError::database_operation_failed("get_tag_suggestions", e))?;

        let recommendation = if exact_match.is_some() {
            "use_existing"
        } else if !similar_tags.is_empty() {
            "consider_similar"
        } else {
            "create_new"
        };

        let response = serde_json::json!({
            "exact_match": exact_match,
            "similar_tags": similar_tags,
            "recommendation": recommendation
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("tag_suggestions", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_get_popular_tags(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

        let tags = self
            .db
            .get_popular_tags(limit)
            .await
            .map_err(|e| McpError::database_operation_failed("get_popular_tags", e))?;

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&tags)
                    .map_err(|e| McpError::serialization_failed("popular_tags", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_get_recent_tags(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

        let tags = self
            .db
            .get_recent_tags(limit)
            .await
            .map_err(|e| McpError::database_operation_failed("get_recent_tags", e))?;

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&tags)
                    .map_err(|e| McpError::serialization_failed("recent_tags", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_create_tag(&self, args: Value) -> McpResult<CallToolResult> {
        let title: String = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_parameter("title", "Missing 'title' parameter"))?
            .to_string();

        let shortcut: Option<String> = args
            .get("shortcut")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let parent_uuid: Option<ThingsId> = args
            .get("parent_uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| ThingsId::from_str(s).ok());

        let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

        let request = things3_core::models::CreateTagRequest {
            title,
            shortcut,
            parent_uuid,
        };

        let result = match self
            .mutations
            .create_tag(request, force)
            .await
            .map_err(|e| McpError::database_operation_failed("create_tag", e))?
        {
            things3_core::models::TagCreationResult::Created { uuid, .. } => {
                let message = if force {
                    "Tag created successfully (duplicate check skipped)"
                } else {
                    "Tag created successfully"
                };
                serde_json::json!({
                    "status": "created",
                    "uuid": uuid,
                    "message": message,
                })
            }
            things3_core::models::TagCreationResult::Existing { tag, .. } => {
                serde_json::json!({
                    "status": "existing",
                    "uuid": tag.uuid,
                    "tag": tag,
                    "message": "Tag already exists"
                })
            }
            things3_core::models::TagCreationResult::SimilarFound {
                similar_tags,
                requested_title,
            } => {
                serde_json::json!({
                    "status": "similar_found",
                    "similar_tags": similar_tags,
                    "requested_title": requested_title,
                    "message": "Similar tags found. Use force=true to create anyway."
                })
            }
        };

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::serialization_failed("create_tag_response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_update_tag(&self, args: Value) -> McpResult<CallToolResult> {
        let uuid: ThingsId = args
            .get("uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| ThingsId::from_str(s).ok())
            .ok_or_else(|| {
                McpError::invalid_parameter("uuid", "Missing or invalid 'uuid' parameter")
            })?;

        let title: Option<String> = args
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let shortcut: Option<String> = args
            .get("shortcut")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let parent_uuid: Option<ThingsId> = args
            .get("parent_uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| ThingsId::from_str(s).ok());

        let uuid_str = uuid.to_string();
        let request = things3_core::models::UpdateTagRequest {
            uuid,
            title,
            shortcut,
            parent_uuid,
        };

        self.mutations
            .update_tag(request)
            .await
            .map_err(|e| McpError::database_operation_failed("update_tag", e))?;

        let response = serde_json::json!({
            "message": "Tag updated successfully",
            "uuid": uuid_str
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("update_tag_response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_delete_tag(&self, args: Value) -> McpResult<CallToolResult> {
        let uuid: ThingsId = args
            .get("uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| ThingsId::from_str(s).ok())
            .ok_or_else(|| {
                McpError::invalid_parameter("uuid", "Missing or invalid 'uuid' parameter")
            })?;

        let remove_from_tasks = args
            .get("remove_from_tasks")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        self.mutations
            .delete_tag(&uuid, remove_from_tasks)
            .await
            .map_err(|e| McpError::database_operation_failed("delete_tag", e))?;

        let response = serde_json::json!({
            "message": "Tag deleted successfully",
            "uuid": uuid
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("delete_tag_response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_merge_tags(&self, args: Value) -> McpResult<CallToolResult> {
        let source_uuid: ThingsId = args
            .get("source_uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| ThingsId::from_str(s).ok())
            .ok_or_else(|| {
                McpError::invalid_parameter(
                    "source_uuid",
                    "Missing or invalid 'source_uuid' parameter",
                )
            })?;

        let target_uuid: ThingsId = args
            .get("target_uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| ThingsId::from_str(s).ok())
            .ok_or_else(|| {
                McpError::invalid_parameter(
                    "target_uuid",
                    "Missing or invalid 'target_uuid' parameter",
                )
            })?;

        self.mutations
            .merge_tags(&source_uuid, &target_uuid)
            .await
            .map_err(|e| McpError::database_operation_failed("merge_tags", e))?;

        let response = serde_json::json!({
            "message": "Tags merged successfully",
            "source_uuid": source_uuid,
            "target_uuid": target_uuid
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("merge_tags_response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_add_tag_to_task(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let task_uuid: ThingsId = args
            .get("task_uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| ThingsId::from_str(s).ok())
            .ok_or_else(|| {
                McpError::invalid_parameter("task_uuid", "Missing or invalid 'task_uuid' parameter")
            })?;

        let tag_title: String = args
            .get("tag_title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpError::invalid_parameter("tag_title", "Missing 'tag_title' parameter")
            })?
            .to_string();

        let result = self
            .mutations
            .add_tag_to_task(&task_uuid, &tag_title)
            .await
            .map_err(|e| McpError::database_operation_failed("add_tag_to_task", e))?;

        let response = match result {
            things3_core::models::TagAssignmentResult::Assigned { tag_uuid } => {
                serde_json::json!({
                    "status": "assigned",
                    "tag_uuid": tag_uuid,
                    "message": "Tag added to task successfully"
                })
            }
            things3_core::models::TagAssignmentResult::Suggestions { similar_tags } => {
                serde_json::json!({
                    "status": "suggestions",
                    "similar_tags": similar_tags,
                    "message": "Similar tags found. Please confirm or use a different tag."
                })
            }
        };

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("add_tag_to_task_response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_remove_tag_from_task(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let task_uuid: ThingsId = args
            .get("task_uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| ThingsId::from_str(s).ok())
            .ok_or_else(|| {
                McpError::invalid_parameter("task_uuid", "Missing or invalid 'task_uuid' parameter")
            })?;

        let tag_title: String = args
            .get("tag_title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpError::invalid_parameter("tag_title", "Missing 'tag_title' parameter")
            })?
            .to_string();

        self.mutations
            .remove_tag_from_task(&task_uuid, &tag_title)
            .await
            .map_err(|e| McpError::database_operation_failed("remove_tag_from_task", e))?;

        let response = serde_json::json!({
            "message": "Tag removed from task successfully",
            "task_uuid": task_uuid,
            "tag_title": tag_title
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response).map_err(|e| {
                    McpError::serialization_failed("remove_tag_from_task_response", e)
                })?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_set_task_tags(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let task_uuid: ThingsId = args
            .get("task_uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| ThingsId::from_str(s).ok())
            .ok_or_else(|| {
                McpError::invalid_parameter("task_uuid", "Missing or invalid 'task_uuid' parameter")
            })?;

        let tag_titles: Vec<String> = args
            .get("tag_titles")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                McpError::invalid_parameter("tag_titles", "Missing 'tag_titles' parameter")
            })?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        let suggestions = self
            .mutations
            .set_task_tags(&task_uuid, tag_titles.clone())
            .await
            .map_err(|e| McpError::database_operation_failed("set_task_tags", e))?;

        let response = serde_json::json!({
            "message": "Task tags updated successfully",
            "task_uuid": task_uuid,
            "tags": tag_titles,
            "suggestions": suggestions
        });

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("set_task_tags_response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_get_tag_statistics(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let uuid: ThingsId = args
            .get("uuid")
            .and_then(|v| v.as_str())
            .and_then(|s| ThingsId::from_str(s).ok())
            .ok_or_else(|| {
                McpError::invalid_parameter("uuid", "Missing or invalid 'uuid' parameter")
            })?;

        let stats = self
            .db
            .get_tag_statistics(&uuid)
            .await
            .map_err(|e| McpError::database_operation_failed("get_tag_statistics", e))?;

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&stats)
                    .map_err(|e| McpError::serialization_failed("tag_statistics", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_find_duplicate_tags(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let min_similarity = args
            .get("min_similarity")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.85) as f32;

        let duplicates = self
            .db
            .find_duplicate_tags(min_similarity)
            .await
            .map_err(|e| McpError::database_operation_failed("find_duplicate_tags", e))?;

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&duplicates)
                    .map_err(|e| McpError::serialization_failed("duplicate_tags", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_get_tag_completions(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let prefix: String = if let Some(v) = args.get("prefix").and_then(|v| v.as_str()) {
            v.to_string()
        } else if let Some(v) = args.get("partial_input").and_then(|v| v.as_str()) {
            warn!("get_tag_completions: 'partial_input' is deprecated, use 'prefix' instead");
            v.to_string()
        } else {
            return Err(McpError::invalid_parameter(
                "prefix",
                "Missing 'prefix' parameter",
            ));
        };

        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        let completions = self
            .db
            .get_tag_completions(&prefix, limit)
            .await
            .map_err(|e| McpError::database_operation_failed("get_tag_completions", e))?;

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&completions)
                    .map_err(|e| McpError::serialization_failed("tag_completions", e))?,
            }],
            is_error: false,
        })
    }
}
