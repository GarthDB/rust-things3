//! Integration tests for full MCP workflows

use serde_json::json;
use things3_cli::mcp::{
    middleware::MiddlewareConfig,
    test_harness::{McpIntegrationTest, McpTestHarness},
};

/// Test complete MCP server workflows
#[tokio::test]
async fn test_complete_mcp_workflow() {
    let integration_test = McpIntegrationTest::new();

    // 1. Discover available capabilities
    let tools = integration_test.harness().server().list_tools().unwrap();
    let resources = integration_test
        .harness()
        .server()
        .list_resources()
        .unwrap();
    let prompts = integration_test.harness().server().list_prompts().unwrap();

    assert!(!tools.tools.is_empty());
    assert!(!resources.resources.is_empty());
    assert!(!prompts.prompts.is_empty());

    // 2. Test data retrieval workflow
    let inbox_data = integration_test.test_tool_workflow("get_inbox", None).await;
    assert!(!inbox_data.is_error);

    let today_data = integration_test.test_tool_workflow("get_today", None).await;
    assert!(!today_data.is_error);

    let areas_data = integration_test.test_tool_workflow("get_areas", None).await;
    assert!(!areas_data.is_error);

    // 3. Test resource access workflow
    let inbox_resource = integration_test
        .test_resource_workflow("things://inbox")
        .await;
    assert!(!inbox_resource.contents.is_empty());

    let today_resource = integration_test
        .test_resource_workflow("things://today")
        .await;
    assert!(!today_resource.contents.is_empty());

    // 4. Test prompt workflow
    let task_review = integration_test
        .test_prompt_workflow("task_review", Some(json!({"task_title": "Test Task"})))
        .await;
    assert!(!task_review.is_error);

    let project_planning = integration_test
        .test_prompt_workflow(
            "project_planning",
            Some(json!({"project_title": "Test Project"})),
        )
        .await;
    assert!(!project_planning.is_error);
}

/// Test error handling workflows
#[tokio::test]
async fn test_error_handling_workflows() {
    let integration_test = McpIntegrationTest::new();

    // Test error handling workflow
    integration_test.test_error_handling_workflow().await;
}

/// Test performance workflows
#[tokio::test]
async fn test_performance_workflows() {
    let integration_test = McpIntegrationTest::new();

    // Test performance workflow
    integration_test.test_performance_workflow().await;
}

/// Test workflow with custom middleware
#[tokio::test]
async fn test_workflow_with_middleware() {
    let middleware_config = MiddlewareConfig::default();
    let integration_test = McpIntegrationTest::with_middleware_config(middleware_config);

    // Test that workflows work with middleware
    let result = integration_test.test_tool_workflow("get_inbox", None).await;
    assert!(!result.is_error);

    let result = integration_test
        .test_resource_workflow("things://inbox")
        .await;
    assert!(!result.contents.is_empty());

    let result = integration_test
        .test_prompt_workflow("task_review", Some(json!({"task_title": "Test"})))
        .await;
    assert!(!result.is_error);
}

/// Test data consistency across different access methods
#[tokio::test]
async fn test_data_consistency_workflow() {
    let harness = McpTestHarness::new();

    // Get data via tool call
    let tool_result = harness.assert_tool_returns_json("get_inbox", None).await;
    assert!(tool_result.is_array());

    // Get data via resource read
    let resource_result = harness.assert_resource_returns_json("things://inbox").await;
    assert!(resource_result.is_array());

    // Both should return similar data structures
    assert_eq!(tool_result.is_array(), resource_result.is_array());

    // Test today's tasks consistency
    let tool_result = harness.assert_tool_returns_json("get_today", None).await;
    let resource_result = harness.assert_resource_returns_json("things://today").await;

    assert_eq!(tool_result.is_array(), resource_result.is_array());
}

/// Test parameter validation workflows
#[tokio::test]
async fn test_parameter_validation_workflows() {
    let harness = McpTestHarness::new();

    // Test valid parameters
    let result = harness
        .assert_tool_success("get_inbox", Some(json!({"limit": 5})))
        .await;
    assert!(!result.is_error);

    let result = harness
        .assert_tool_success("search_tasks", Some(json!({"query": "test", "limit": 10})))
        .await;
    assert!(!result.is_error);

    // Test invalid parameters
    harness
        .assert_tool_error("search_tasks", Some(json!({})), |e| {
            matches!(e, things3_cli::mcp::McpError::MissingParameter { .. })
        })
        .await;

    harness
        .assert_tool_error(
            "export_data",
            Some(json!({"format": "invalid", "data_type": "tasks"})),
            |e| matches!(e, things3_cli::mcp::McpError::InvalidFormat { .. }),
        )
        .await;
}

/// Test concurrent access workflows
#[tokio::test]
async fn test_concurrent_access_workflows() {
    let harness = McpTestHarness::new();

    // Test concurrent tool calls
    let futures = vec![
        harness.call_tool("get_inbox", None),
        harness.call_tool("get_today", None),
        harness.call_tool("get_areas", None),
        harness.call_tool("get_projects", None),
    ];

    let results = futures::future::join_all(futures).await;

    for result in results {
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.is_error);
    }

    // Test concurrent resource reads
    let futures = vec![
        harness.read_resource("things://inbox"),
        harness.read_resource("things://today"),
        harness.read_resource("things://areas"),
        harness.read_resource("things://projects"),
    ];

    let results = futures::future::join_all(futures).await;

    for result in results {
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.contents.is_empty());
    }

    // Test concurrent prompt calls
    let futures = vec![
        harness.get_prompt("task_review", Some(json!({"task_title": "Task 1"}))),
        harness.get_prompt(
            "project_planning",
            Some(json!({"project_title": "Project 1"})),
        ),
        harness.get_prompt(
            "productivity_analysis",
            Some(json!({"time_period": "week"})),
        ),
        harness.get_prompt(
            "backup_strategy",
            Some(json!({"data_volume": "small", "frequency": "daily"})),
        ),
    ];

    let results = futures::future::join_all(futures).await;

    for result in results {
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.is_error);
    }
}

/// Test data export workflows
#[tokio::test]
async fn test_data_export_workflows() {
    let harness = McpTestHarness::new();

    // Test JSON export
    let result = harness
        .assert_tool_success(
            "export_data",
            Some(json!({
                "format": "json",
                "data_type": "tasks"
            })),
        )
        .await;
    assert!(!result.is_error);

    // Test all data export
    let result = harness
        .assert_tool_success(
            "export_data",
            Some(json!({
                "format": "json",
                "data_type": "all"
            })),
        )
        .await;
    assert!(!result.is_error);

    // Test projects export
    let result = harness
        .assert_tool_success(
            "export_data",
            Some(json!({
                "format": "json",
                "data_type": "projects"
            })),
        )
        .await;
    assert!(!result.is_error);

    // Test areas export
    let result = harness
        .assert_tool_success(
            "export_data",
            Some(json!({
                "format": "json",
                "data_type": "areas"
            })),
        )
        .await;
    assert!(!result.is_error);
}

/// Test productivity metrics workflows
#[tokio::test]
async fn test_productivity_metrics_workflows() {
    let harness = McpTestHarness::new();

    // Test productivity metrics with default parameters
    let result = harness
        .assert_tool_returns_json("get_productivity_metrics", None)
        .await;
    assert!(result["period_days"].is_number());
    assert!(result["inbox_tasks_count"].is_number());
    assert!(result["today_tasks_count"].is_number());
    assert!(result["projects_count"].is_number());
    assert!(result["areas_count"].is_number());

    // Test productivity metrics with custom parameters
    let result = harness
        .assert_tool_returns_json("get_productivity_metrics", Some(json!({"days": 14})))
        .await;
    assert_eq!(result["period_days"], 14);

    // Test performance stats
    let result = harness
        .assert_tool_returns_json("get_performance_stats", None)
        .await;
    assert!(result["summary"].is_object());
    assert!(result["operation_stats"].is_object());

    // Test system metrics
    let result = harness
        .assert_tool_returns_json("get_system_metrics", None)
        .await;
    assert!(result.is_object());

    // Test cache stats
    let result = harness
        .assert_tool_returns_json("get_cache_stats", None)
        .await;
    assert!(result.is_object());
}

/// Test backup and restore workflows
#[tokio::test]
async fn test_backup_restore_workflows() {
    let harness = McpTestHarness::new();

    // Test list backups
    let temp_dir = tempfile::tempdir().unwrap();
    let backup_dir = temp_dir.path().to_str().unwrap();

    let result = harness
        .assert_tool_success(
            "list_backups",
            Some(json!({
                "backup_dir": backup_dir
            })),
        )
        .await;
    assert!(!result.is_error);

    // Test backup creation (will fail in test environment, but should handle gracefully)
    let result = harness
        .call_tool(
            "backup_database",
            Some(json!({
                "backup_dir": backup_dir,
                "description": "Test backup"
            })),
        )
        .await;
    // This will likely fail in test environment, which is expected
    if let Err(error) = result {
        assert!(matches!(
            error,
            things3_cli::mcp::McpError::BackupOperationFailed { .. }
        ));
    }
}

/// Test search and filtering workflows
#[tokio::test]
async fn test_search_filtering_workflows() {
    let harness = McpTestHarness::new();

    // Test search functionality
    let result = harness
        .assert_tool_success(
            "search_tasks",
            Some(json!({
                "query": "test",
                "limit": 5
            })),
        )
        .await;
    assert!(!result.is_error);

    // Test project filtering
    let result = harness
        .assert_tool_success(
            "get_projects",
            Some(json!({
                "area_uuid": "area-1"
            })),
        )
        .await;
    assert!(!result.is_error);

    // Test recent tasks
    let result = harness
        .assert_tool_success(
            "get_recent_tasks",
            Some(json!({
                "limit": 10,
                "hours": 24
            })),
        )
        .await;
    assert!(!result.is_error);

    // Test recent tasks with default parameters
    let result = harness.assert_tool_success("get_recent_tasks", None).await;
    assert!(!result.is_error);
}

/// Test prompt workflows with various parameters
#[tokio::test]
async fn test_prompt_parameter_workflows() {
    let harness = McpTestHarness::new();

    // Test task review with full parameters
    let result = harness
        .assert_prompt_success(
            "task_review",
            Some(json!({
                "task_title": "Complete project documentation",
                "task_notes": "Need to document the API endpoints",
                "context": "This is for the Q1 release"
            })),
        )
        .await;
    assert!(!result.is_error);

    // Test project planning with full parameters
    let result = harness
        .assert_prompt_success(
            "project_planning",
            Some(json!({
                "project_title": "Website Redesign",
                "project_description": "Complete redesign of company website",
                "deadline": "2024-03-31",
                "complexity": "complex"
            })),
        )
        .await;
    assert!(!result.is_error);

    // Test productivity analysis with full parameters
    let result = harness
        .assert_prompt_success(
            "productivity_analysis",
            Some(json!({
                "time_period": "month",
                "focus_area": "completion_rate",
                "include_recommendations": true
            })),
        )
        .await;
    assert!(!result.is_error);

    // Test backup strategy with full parameters
    let result = harness
        .assert_prompt_success(
            "backup_strategy",
            Some(json!({
                "data_volume": "large",
                "frequency": "daily",
                "retention_period": "1_year",
                "storage_preference": "cloud"
            })),
        )
        .await;
    assert!(!result.is_error);
}

/// Test error recovery workflows
#[tokio::test]
async fn test_error_recovery_workflows() {
    let harness = McpTestHarness::new();

    // Test fallback error handling
    let result = harness.call_tool_with_fallback("unknown_tool", None).await;
    assert!(result.is_error);
    match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => {
            assert!(text.contains("not found"));
        }
    }

    let result = harness
        .read_resource_with_fallback("things://unknown")
        .await;
    match &result.contents[0] {
        things3_cli::mcp::Content::Text { text } => {
            assert!(text.contains("not found"));
        }
    }

    let result = harness
        .get_prompt_with_fallback("unknown_prompt", None)
        .await;
    assert!(result.is_error);
    match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => {
            assert!(text.contains("not found"));
        }
    }
}

/// Test workflow with different data scenarios
#[tokio::test]
async fn test_data_scenario_workflows() {
    let harness = McpTestHarness::new();

    // Test with empty results
    let result = harness
        .assert_tool_success("get_inbox", Some(json!({"limit": 0})))
        .await;
    assert!(!result.is_error);

    // Test with large limits
    let result = harness
        .assert_tool_success("get_inbox", Some(json!({"limit": 1000})))
        .await;
    assert!(!result.is_error);

    // Test with various search queries
    let search_queries = vec!["test", "work", "project", "urgent", "completed"];

    for query in search_queries {
        let result = harness
            .assert_tool_success(
                "search_tasks",
                Some(json!({
                    "query": query,
                    "limit": 5
                })),
            )
            .await;
        assert!(!result.is_error);
    }
}

/// Test workflow stress testing
#[tokio::test]
async fn test_stress_workflows() {
    let harness = McpTestHarness::new();

    // Test rapid successive calls
    for i in 0..10 {
        let result = harness
            .call_tool("get_inbox", Some(json!({"limit": 1})))
            .await;
        assert!(result.is_ok(), "Call {} failed", i);
    }

    // Test mixed operation types
    let tool_futures = vec![
        harness.call_tool("get_inbox", None),
        harness.call_tool("get_today", None),
    ];

    let resource_futures = vec![
        harness.read_resource("things://inbox"),
        harness.read_resource("things://today"),
    ];

    let prompt_futures = vec![
        harness.get_prompt("task_review", Some(json!({"task_title": "Test"}))),
        harness.get_prompt("project_planning", Some(json!({"project_title": "Test"}))),
    ];

    let tool_results = futures::future::join_all(tool_futures).await;
    let resource_results = futures::future::join_all(resource_futures).await;
    let prompt_results = futures::future::join_all(prompt_futures).await;

    // Check tool results
    for (i, result) in tool_results.iter().enumerate() {
        match result {
            Ok(tool_result) => assert!(!tool_result.is_error, "Tool call {} failed", i),
            Err(e) => panic!("Tool operation {} failed: {:?}", i, e),
        }
    }

    // Check resource results
    for (i, result) in resource_results.iter().enumerate() {
        match result {
            Ok(resource_result) => assert!(
                !resource_result.contents.is_empty(),
                "Resource call {} failed",
                i
            ),
            Err(e) => panic!("Resource operation {} failed: {:?}", i, e),
        }
    }

    // Check prompt results
    for (i, result) in prompt_results.iter().enumerate() {
        match result {
            Ok(prompt_result) => assert!(!prompt_result.is_error, "Prompt call {} failed", i),
            Err(e) => panic!("Prompt operation {} failed: {:?}", i, e),
        }
    }
}

/// Test workflow with middleware chain
#[tokio::test]
async fn test_middleware_workflow() {
    let middleware_config = MiddlewareConfig {
        logging: things3_cli::mcp::middleware::LoggingConfig {
            enabled: true,
            level: "debug".to_string(),
        },
        validation: things3_cli::mcp::middleware::ValidationConfig {
            enabled: true,
            strict_mode: true,
        },
        performance: things3_cli::mcp::middleware::PerformanceConfig {
            enabled: true,
            slow_request_threshold_ms: 1000,
        },
        security: things3_cli::mcp::middleware::SecurityConfig::default(),
    };

    let harness = McpTestHarness::with_middleware_config(middleware_config);

    // Test that operations work with middleware
    let result = harness.assert_tool_success("get_inbox", None).await;
    assert!(!result.is_error);

    let result = harness.assert_resource_success("things://inbox").await;
    assert!(!result.contents.is_empty());

    let result = harness
        .assert_prompt_success("task_review", Some(json!({"task_title": "Test"})))
        .await;
    assert!(!result.is_error);
}
