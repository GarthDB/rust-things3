use crate::mcp::{CallToolResult, Content, McpError, McpResult, ThingsMcpServer};
use serde_json::Value;

impl ThingsMcpServer {
    pub(in crate::mcp) async fn handle_get_productivity_metrics(
        &self,
        args: Value,
    ) -> McpResult<CallToolResult> {
        let days = usize::try_from(
            args.get("days")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(7),
        )
        .unwrap_or(7);

        // Get various metrics
        let db = &self.db;
        let inbox_tasks = db
            .get_inbox(None)
            .await
            .map_err(|e| McpError::database_operation_failed("get_inbox for metrics", e))?;
        let today_tasks = db
            .get_today(None)
            .await
            .map_err(|e| McpError::database_operation_failed("get_today for metrics", e))?;
        let projects = db
            .get_projects(None)
            .await
            .map_err(|e| McpError::database_operation_failed("get_projects for metrics", e))?;
        let areas = db
            .get_areas()
            .await
            .map_err(|e| McpError::database_operation_failed("get_areas for metrics", e))?;
        let _ = db;

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
                text: serde_json::to_string_pretty(&metrics).map_err(|e| {
                    McpError::serialization_failed("productivity_metrics serialization", e)
                })?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_get_performance_stats(
        &self,
        _args: Value,
    ) -> McpResult<CallToolResult> {
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
                text: serde_json::to_string_pretty(&response)
                    .map_err(|e| McpError::serialization_failed("performance_stats response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_get_system_metrics(
        &self,
        _args: Value,
    ) -> McpResult<CallToolResult> {
        let metrics = self
            .performance_monitor
            .lock()
            .await
            .get_system_metrics()
            .map_err(|e| {
                McpError::performance_monitoring_failed(
                    "get_system_metrics",
                    things3_core::ThingsError::unknown(e.to_string()),
                )
            })?;

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&metrics)
                    .map_err(|e| McpError::serialization_failed("system_metrics response", e))?,
            }],
            is_error: false,
        })
    }

    pub(in crate::mcp) async fn handle_get_cache_stats(
        &self,
        _args: Value,
    ) -> McpResult<CallToolResult> {
        let stats = self.cache.lock().await.get_stats();

        Ok(CallToolResult {
            content: vec![Content::Text {
                text: serde_json::to_string_pretty(&stats)
                    .map_err(|e| McpError::serialization_failed("cache_stats response", e))?,
            }],
            is_error: false,
        })
    }
}
