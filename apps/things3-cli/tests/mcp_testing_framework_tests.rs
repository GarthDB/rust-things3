//! Comprehensive tests for the MCP testing framework

use serde_json::json;
use things3_cli::mcp::{
    test_harness::{
        McpIntegrationTest, McpPerformanceTest, McpTestHarness, McpTestUtils, MockDatabase,
    },
    McpError,
};

#[tokio::test]
async fn test_mcp_test_harness_basic_functionality() {
    let harness = McpTestHarness::new();

    // Test that we can list tools
    let tools_result = harness.server().list_tools().unwrap();
    assert!(!tools_result.tools.is_empty());

    // Test that we can list resources
    let resources_result = harness.server().list_resources().unwrap();
    assert!(!resources_result.resources.is_empty());

    // Test that we can list prompts
    let prompts_result = harness.server().list_prompts().unwrap();
    assert!(!prompts_result.prompts.is_empty());
}

#[tokio::test]
async fn test_mcp_test_harness_tool_calls() {
    let harness = McpTestHarness::new();

    // Test successful tool calls
    let result = harness.assert_tool_success("get_inbox", None).await;
    assert!(!result.is_error);
    assert!(!result.content.is_empty());

    // Test tool call with arguments
    let result = harness
        .assert_tool_success("get_inbox", Some(json!({"limit": 3})))
        .await;
    assert!(!result.is_error);

    // Test tool call with JSON assertion
    let json_result = harness.assert_tool_returns_json("get_inbox", None).await;
    assert!(json_result.is_array());
}

#[tokio::test]
async fn test_mcp_test_harness_resource_calls() {
    let harness = McpTestHarness::new();

    // Test successful resource reads
    let result = harness.assert_resource_success("things://inbox").await;
    assert!(!result.contents.is_empty());

    // Test resource read with JSON assertion
    let json_result = harness.assert_resource_returns_json("things://inbox").await;
    assert!(json_result.is_array());
}

#[tokio::test]
async fn test_mcp_test_harness_prompt_calls() {
    let harness = McpTestHarness::new();

    // Test successful prompt calls
    let result = harness
        .assert_prompt_success("task_review", Some(json!({"task_title": "Test Task"})))
        .await;
    assert!(!result.is_error);
    assert!(!result.content.is_empty());

    // Test prompt call with text assertion
    let text_result = harness
        .assert_prompt_returns_text("task_review", Some(json!({"task_title": "Test Task"})))
        .await;
    assert!(text_result.contains("Test Task"));
    assert!(text_result.contains("Task Review"));
}

#[tokio::test]
async fn test_mcp_test_harness_error_handling() {
    let harness = McpTestHarness::new();

    // Test tool not found error
    harness
        .assert_tool_error("unknown_tool", None, |e| {
            matches!(e, McpError::ToolNotFound { .. })
        })
        .await;

    // Test resource not found error
    harness
        .assert_resource_error("things://unknown", |e| {
            matches!(e, McpError::ResourceNotFound { .. })
        })
        .await;

    // Test prompt not found error
    harness
        .assert_prompt_error("unknown_prompt", None, |e| {
            matches!(e, McpError::PromptNotFound { .. })
        })
        .await;

    // Test missing parameter error
    harness
        .assert_tool_error("search_tasks", Some(json!({})), |e| {
            matches!(e, McpError::MissingParameter { .. })
        })
        .await;
}

#[tokio::test]
async fn test_mcp_test_harness_fallback_methods() {
    let harness = McpTestHarness::new();

    // Test tool call with fallback
    let result = harness.call_tool_with_fallback("get_inbox", None).await;
    assert!(!result.is_error);

    // Test tool call with fallback for unknown tool
    let result = harness.call_tool_with_fallback("unknown_tool", None).await;
    assert!(result.is_error);
    match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => {
            assert!(text.contains("not found"));
        }
    }

    // Test resource read with fallback
    let result = harness.read_resource_with_fallback("things://inbox").await;
    assert!(!result.contents.is_empty());

    // Test resource read with fallback for unknown resource
    let result = harness
        .read_resource_with_fallback("things://unknown")
        .await;
    assert!(!result.contents.is_empty());
    match &result.contents[0] {
        things3_cli::mcp::Content::Text { text } => {
            assert!(text.contains("not found"));
        }
    }

    // Test prompt with fallback
    let result = harness
        .get_prompt_with_fallback("task_review", Some(json!({"task_title": "Test"})))
        .await;
    assert!(!result.is_error);

    // Test prompt with fallback for unknown prompt
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

#[tokio::test]
async fn test_mock_database_functionality() {
    let mut db = MockDatabase::new();

    // Test initial state
    assert_eq!(db.tasks.len(), 2);
    assert_eq!(db.projects.len(), 1);
    assert_eq!(db.areas.len(), 2);

    // Test adding new data
    db.add_task(things3_cli::mcp::test_harness::MockTask {
        uuid: "new-task".to_string(),
        title: "New Task".to_string(),
        notes: Some("New notes".to_string()),
        status: "incomplete".to_string(),
        project_uuid: Some("project-1".to_string()),
        area_uuid: Some("area-1".to_string()),
    });
    assert_eq!(db.tasks.len(), 3);

    // Test querying data
    let task = db.get_task("task-1").unwrap();
    assert_eq!(task.title, "Test Task 1");
    assert_eq!(task.status, "incomplete");

    let project = db.get_project("project-1").unwrap();
    assert_eq!(project.title, "Test Project");

    let area = db.get_area("area-1").unwrap();
    assert_eq!(area.title, "Work");

    // Test filtering by status
    let completed_tasks = db.get_tasks_by_status("completed");
    assert_eq!(completed_tasks.len(), 1);
    assert_eq!(completed_tasks[0].title, "Test Task 2");

    // Test filtering by project
    let project_tasks = db.get_tasks_by_project("project-1");
    assert_eq!(project_tasks.len(), 2); // task-2 and new-task

    // Test filtering by area
    let area_tasks = db.get_tasks_by_area("area-1");
    assert_eq!(area_tasks.len(), 2); // task-2 and new-task
}

#[tokio::test]
async fn test_mock_database_with_scenarios() {
    let db = McpTestUtils::create_test_data_with_scenarios();

    // Test that we have more data
    assert!(db.tasks.len() > 2);
    assert!(db.projects.len() > 1);
    assert!(db.areas.len() > 2);

    // Test specific scenarios
    let urgent_task = db.get_task("task-urgent").unwrap();
    assert_eq!(urgent_task.title, "Urgent Task");
    assert_eq!(urgent_task.status, "incomplete");

    let completed_task = db.get_task("task-completed").unwrap();
    assert_eq!(completed_task.title, "Completed Task");
    assert_eq!(completed_task.status, "completed");

    let another_project = db.get_project("project-2").unwrap();
    assert_eq!(another_project.title, "Another Project");

    let health_area = db.get_area("area-3").unwrap();
    assert_eq!(health_area.title, "Health");
}

#[tokio::test]
async fn test_mcp_test_utils() {
    // Test creating requests
    let tool_request =
        McpTestUtils::create_tool_request("test_tool", Some(json!({"param": "value"})));
    assert_eq!(tool_request.name, "test_tool");
    assert!(tool_request.arguments.is_some());

    let resource_request = McpTestUtils::create_resource_request("things://test");
    assert_eq!(resource_request.uri, "things://test");

    let prompt_request =
        McpTestUtils::create_prompt_request("test_prompt", Some(json!({"param": "value"})));
    assert_eq!(prompt_request.name, "test_prompt");
    assert!(prompt_request.arguments.is_some());

    // Test creating test data
    let test_data = McpTestUtils::create_test_data();
    assert!(!test_data.tasks.is_empty());
    assert!(!test_data.projects.is_empty());
    assert!(!test_data.areas.is_empty());
}

#[tokio::test]
async fn test_mcp_performance_test() {
    let perf_test = McpPerformanceTest::new();

    // Simulate some work
    std::thread::sleep(std::time::Duration::from_millis(10));

    let elapsed = perf_test.elapsed();
    assert!(elapsed.as_millis() >= 10);

    // Test threshold assertions
    let perf_test = McpPerformanceTest::new();
    perf_test.assert_under_ms(1000); // Should pass

    let perf_test = McpPerformanceTest::new();
    std::thread::sleep(std::time::Duration::from_millis(5));
    perf_test.assert_under_ms(100); // Should pass

    // Test that it fails when threshold is exceeded
    let perf_test = McpPerformanceTest::new();
    std::thread::sleep(std::time::Duration::from_millis(10));
    // This should not panic because 10ms < 1000ms
    perf_test.assert_under_ms(1000);
}

#[tokio::test]
async fn test_mcp_integration_test_basic() {
    let integration_test = McpIntegrationTest::new();

    // Test tool workflow
    let result = integration_test.test_tool_workflow("get_inbox", None).await;
    assert!(!result.is_error);
    assert!(!result.content.is_empty());

    // Test resource workflow
    let result = integration_test
        .test_resource_workflow("things://inbox")
        .await;
    assert!(!result.contents.is_empty());

    // Test prompt workflow
    let result = integration_test
        .test_prompt_workflow("task_review", Some(json!({"task_title": "Test Task"})))
        .await;
    assert!(!result.is_error);
    assert!(!result.content.is_empty());
}

#[tokio::test]
async fn test_mcp_integration_test_error_handling() {
    let integration_test = McpIntegrationTest::new();

    // Test error handling workflow
    integration_test.test_error_handling_workflow().await;
}

#[tokio::test]
async fn test_mcp_integration_test_performance() {
    let integration_test = McpIntegrationTest::new();

    // Test performance workflow
    integration_test.test_performance_workflow().await;
}

#[tokio::test]
async fn test_mcp_integration_test_with_middleware() {
    use things3_cli::mcp::middleware::MiddlewareConfig;

    let middleware_config = MiddlewareConfig::default();
    let integration_test = McpIntegrationTest::with_middleware_config(middleware_config);

    // Test that it works with middleware
    let result = integration_test.test_tool_workflow("get_inbox", None).await;
    assert!(!result.is_error);
}

#[tokio::test]
async fn test_mcp_test_utils_assertions() {
    let harness = McpTestHarness::new();

    // Test tool result assertions
    let result = harness.call_tool("get_inbox", None).await.unwrap();
    McpTestUtils::assert_tool_result_contains(&result, "uuid");

    let json_result = McpTestUtils::assert_tool_result_is_json(&result);
    assert!(json_result.is_array());

    // Test resource result assertions
    let result = harness.read_resource("things://inbox").await.unwrap();
    McpTestUtils::assert_resource_result_contains(&result, "uuid");

    let json_result = McpTestUtils::assert_resource_result_is_json(&result);
    assert!(json_result.is_array());

    // Test prompt result assertions
    let result = harness
        .get_prompt("task_review", Some(json!({"task_title": "Test Task"})))
        .await
        .unwrap();
    McpTestUtils::assert_prompt_result_contains(&result, "Test Task");
    McpTestUtils::assert_prompt_result_contains(&result, "Task Review");
}

#[tokio::test]
async fn test_mcp_test_harness_with_middleware() {
    use things3_cli::mcp::middleware::MiddlewareConfig;

    let middleware_config = MiddlewareConfig::default();
    let harness = McpTestHarness::with_middleware_config(middleware_config);

    // Test that it works with middleware
    let result = harness.assert_tool_success("get_inbox", None).await;
    assert!(!result.is_error);
}

#[tokio::test]
async fn test_mcp_test_harness_database_path() {
    let harness = McpTestHarness::new();

    // Test that we can get the database path
    let db_path = harness.db_path();
    assert!(db_path.exists());
    assert!(db_path.is_file());
}

#[tokio::test]
async fn test_mcp_test_harness_comprehensive_workflow() {
    let harness = McpTestHarness::new();

    // Test a comprehensive workflow
    // 1. List all available tools, resources, and prompts
    let tools = harness.server().list_tools().unwrap();
    let resources = harness.server().list_resources().unwrap();
    let prompts = harness.server().list_prompts().unwrap();

    assert!(!tools.tools.is_empty());
    assert!(!resources.resources.is_empty());
    assert!(!prompts.prompts.is_empty());

    // 2. Test various tool calls
    let inbox_result = harness.assert_tool_returns_json("get_inbox", None).await;
    assert!(inbox_result.is_array());

    let today_result = harness.assert_tool_returns_json("get_today", None).await;
    assert!(today_result.is_array());

    let areas_result = harness.assert_tool_returns_json("get_areas", None).await;
    assert!(areas_result.is_array());

    // 3. Test resource reads
    let inbox_resource = harness.assert_resource_returns_json("things://inbox").await;
    assert!(inbox_resource.is_array());

    let today_resource = harness.assert_resource_returns_json("things://today").await;
    assert!(today_resource.is_array());

    // 4. Test prompt calls
    let task_review = harness
        .assert_prompt_returns_text("task_review", Some(json!({"task_title": "Test Task"})))
        .await;
    assert!(task_review.contains("Test Task"));
    assert!(task_review.contains("Task Review"));

    let project_planning = harness
        .assert_prompt_returns_text(
            "project_planning",
            Some(json!({"project_title": "Test Project"})),
        )
        .await;
    assert!(project_planning.contains("Test Project"));
    assert!(project_planning.contains("Project Planning"));

    // 5. Test error handling
    harness
        .assert_tool_error("unknown_tool", None, |e| {
            matches!(e, McpError::ToolNotFound { .. })
        })
        .await;
    harness
        .assert_resource_error("things://unknown", |e| {
            matches!(e, McpError::ResourceNotFound { .. })
        })
        .await;
    harness
        .assert_prompt_error("unknown_prompt", None, |e| {
            matches!(e, McpError::PromptNotFound { .. })
        })
        .await;
}

#[tokio::test]
async fn test_mcp_test_harness_performance_benchmarks() {
    let harness = McpTestHarness::new();

    // Test performance of various operations
    let perf_test = McpPerformanceTest::new();
    let _result = harness.call_tool("get_inbox", None).await.unwrap();
    perf_test.assert_under_ms(1000);

    let perf_test = McpPerformanceTest::new();
    let _result = harness.read_resource("things://inbox").await.unwrap();
    perf_test.assert_under_ms(1000);

    let perf_test = McpPerformanceTest::new();
    let _result = harness
        .get_prompt("task_review", Some(json!({"task_title": "Test"})))
        .await
        .unwrap();
    perf_test.assert_under_ms(1000);
}

#[tokio::test]
async fn test_mcp_test_harness_error_scenarios() {
    let harness = McpTestHarness::new();

    // Test various error scenarios
    harness
        .assert_tool_error("unknown_tool", None, |e| {
            matches!(e, McpError::ToolNotFound { .. })
        })
        .await;
    harness
        .assert_tool_error("search_tasks", Some(json!({})), |e| {
            matches!(e, McpError::MissingParameter { .. })
        })
        .await;
    harness
        .assert_tool_error(
            "export_data",
            Some(json!({"format": "invalid", "data_type": "tasks"})),
            |e| matches!(e, McpError::InvalidFormat { .. }),
        )
        .await;
}

#[tokio::test]
async fn test_mcp_test_harness_data_validation() {
    let harness = McpTestHarness::new();

    // Test that returned data has expected structure
    let inbox_json = harness.assert_tool_returns_json("get_inbox", None).await;
    assert!(inbox_json.is_array());

    let areas_json = harness.assert_tool_returns_json("get_areas", None).await;
    assert!(areas_json.is_array());

    let projects_json = harness.assert_tool_returns_json("get_projects", None).await;
    assert!(projects_json.is_array());

    // Test that resources return expected data
    let inbox_resource = harness.assert_resource_returns_json("things://inbox").await;
    assert!(inbox_resource.is_array());

    let today_resource = harness.assert_resource_returns_json("things://today").await;
    assert!(today_resource.is_array());
}

#[tokio::test]
async fn test_mcp_test_harness_concurrent_operations() {
    let harness = McpTestHarness::new();

    // Test concurrent operations
    let tool_futures = vec![
        harness.call_tool("get_inbox", None),
        harness.call_tool("get_today", None),
        harness.call_tool("get_areas", None),
    ];

    let resource_futures = vec![
        harness.read_resource("things://inbox"),
        harness.read_resource("things://today"),
    ];

    let prompt_futures =
        vec![harness.get_prompt("task_review", Some(json!({"task_title": "Test"})))];

    let tool_results = futures::future::join_all(tool_futures).await;
    let resource_results = futures::future::join_all(resource_futures).await;
    let prompt_results = futures::future::join_all(prompt_futures).await;

    // All tool operations should succeed
    for result in tool_results {
        match result {
            Ok(_) => {
                // Tool results are already handled by the harness
            }
            Err(e) => panic!("Concurrent tool operation failed: {e:?}"),
        }
    }

    // All resource operations should succeed
    for result in resource_results {
        match result {
            Ok(_) => {
                // Resource results are already handled by the harness
            }
            Err(e) => panic!("Concurrent resource operation failed: {e:?}"),
        }
    }

    // All prompt operations should succeed
    for result in prompt_results {
        match result {
            Ok(_) => {
                // Prompt results are already handled by the harness
            }
            Err(e) => panic!("Concurrent prompt operation failed: {e:?}"),
        }
    }
}
