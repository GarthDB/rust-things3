//! MCP logbook search integration tests

use chrono::Utc;
use serde_json::{json, Value};
use things3_cli::mcp::test_harness::McpTestHarness;

// Helper to create harness
fn create_harness() -> McpTestHarness {
    McpTestHarness::new()
}

// Helper to parse CallToolResult into JSON Value
fn parse_tool_result(result: &things3_cli::mcp::CallToolResult) -> Value {
    if result.is_error {
        return json!({"error": "Tool call failed"});
    }

    match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => {
            serde_json::from_str(text).unwrap_or(json!({"text": text}))
        }
    }
}

// Helper to create and complete a task via MCP
async fn create_and_complete_task(
    harness: &McpTestHarness,
    title: &str,
    tags: Option<Vec<String>>,
    project_uuid: Option<String>,
    area_uuid: Option<String>,
) -> String {
    // Create task
    let mut task_args = json!({
        "title": title,
    });

    if let Some(t) = tags {
        task_args["tags"] = json!(t);
    }
    if let Some(p) = project_uuid {
        task_args["project_uuid"] = json!(p);
    }
    if let Some(a) = area_uuid {
        task_args["area_uuid"] = json!(a);
    }

    let result = harness.call_tool("create_task", Some(task_args)).await;
    let response = parse_tool_result(&result);
    let uuid = response["uuid"].as_str().unwrap().to_string();

    // Complete the task
    harness
        .call_tool("complete_task", Some(json!({"uuid": uuid})))
        .await;

    uuid
}

// ============================================================================
// MCP Logbook Search Tests (10+ tests)
// ============================================================================

#[tokio::test]
async fn test_logbook_search_basic() {
    let harness = create_harness();

    // Create and complete some tasks
    create_and_complete_task(&harness, "Completed task 1", None, None, None).await;
    create_and_complete_task(&harness, "Completed task 2", None, None, None).await;

    // Search logbook with no filters
    let result = harness.call_tool("logbook_search", None).await;
    let response = parse_tool_result(&result);

    assert!(response.is_array(), "Result should be an array");
    let tasks = response.as_array().unwrap();
    assert!(tasks.len() >= 2, "Should find at least 2 completed tasks");

    // Verify all tasks are completed
    for task in tasks {
        assert_eq!(task["status"], "completed");
    }
}

#[tokio::test]
async fn test_logbook_search_with_text() {
    let harness = create_harness();

    // Create tasks with different titles
    create_and_complete_task(&harness, "Buy groceries", None, None, None).await;
    create_and_complete_task(&harness, "Write project report", None, None, None).await;
    create_and_complete_task(&harness, "Finish coding project", None, None, None).await;

    // Search for "project"
    let result = harness
        .call_tool(
            "logbook_search",
            Some(json!({
                "search_text": "project"
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    assert!(response.is_array());
    let tasks = response.as_array().unwrap();
    assert_eq!(tasks.len(), 2, "Should find 2 tasks with 'project'");

    for task in tasks {
        let title = task["title"].as_str().unwrap().to_lowercase();
        assert!(title.contains("project"), "Title should contain 'project'");
    }
}

#[tokio::test]
async fn test_logbook_search_with_date_range() {
    let harness = create_harness();

    // Create and complete tasks today
    create_and_complete_task(&harness, "Task 1", None, None, None).await;
    create_and_complete_task(&harness, "Task 2", None, None, None).await;

    let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
    let yesterday = (Utc::now() - chrono::Duration::days(1))
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let tomorrow = (Utc::now() + chrono::Duration::days(1))
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();

    // Search from yesterday to tomorrow
    let result = harness
        .call_tool(
            "logbook_search",
            Some(json!({
                "from_date": yesterday,
                "to_date": tomorrow
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    assert!(response.is_array());
    let tasks = response.as_array().unwrap();
    assert!(
        tasks.len() >= 2,
        "Should find at least 2 tasks in date range"
    );
}

#[tokio::test]
async fn test_logbook_search_from_date_only() {
    let harness = create_harness();

    // Create and complete a task
    create_and_complete_task(&harness, "Task today", None, None, None).await;

    let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();

    // Search from today
    let result = harness
        .call_tool(
            "logbook_search",
            Some(json!({
                "from_date": today
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    assert!(response.is_array());
    let tasks = response.as_array().unwrap();
    assert!(tasks.len() >= 1, "Should find at least 1 task from today");
}

#[tokio::test]
async fn test_logbook_search_to_date_only() {
    let harness = create_harness();

    // Create and complete a task
    create_and_complete_task(&harness, "Task today", None, None, None).await;

    let tomorrow = (Utc::now() + chrono::Duration::days(1))
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();

    // Search up to tomorrow
    let result = harness
        .call_tool(
            "logbook_search",
            Some(json!({
                "to_date": tomorrow
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    assert!(response.is_array());
    let tasks = response.as_array().unwrap();
    assert!(
        tasks.len() >= 1,
        "Should find at least 1 task up to tomorrow"
    );
}

#[tokio::test]
async fn test_logbook_search_with_project() {
    let harness = create_harness();

    // Create a project first
    let project_result = harness
        .call_tool(
            "create_project",
            Some(json!({
                "title": "Test Project"
            })),
        )
        .await;
    let project_response = parse_tool_result(&project_result);
    let project_uuid = project_response["uuid"].as_str().unwrap();

    // Create tasks in the project
    create_and_complete_task(
        &harness,
        "Project task 1",
        None,
        Some(project_uuid.to_string()),
        None,
    )
    .await;
    create_and_complete_task(
        &harness,
        "Project task 2",
        None,
        Some(project_uuid.to_string()),
        None,
    )
    .await;
    create_and_complete_task(&harness, "No project task", None, None, None).await;

    // Search by project
    let result = harness
        .call_tool(
            "logbook_search",
            Some(json!({
                "project_uuid": project_uuid
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    assert!(response.is_array());
    let tasks = response.as_array().unwrap();
    assert_eq!(tasks.len(), 2, "Should find 2 tasks in the project");

    for task in tasks {
        let task_project = task["project_uuid"].as_str();
        assert_eq!(
            task_project,
            Some(project_uuid),
            "Task should belong to the project"
        );
    }
}

#[tokio::test]
async fn test_logbook_search_with_area() {
    let harness = create_harness();

    // Create an area first
    let area_result = harness
        .call_tool(
            "create_area",
            Some(json!({
                "title": "Test Area"
            })),
        )
        .await;
    let area_response = parse_tool_result(&area_result);
    let area_uuid = area_response["uuid"].as_str().unwrap();

    // Create tasks in the area
    create_and_complete_task(
        &harness,
        "Area task 1",
        None,
        None,
        Some(area_uuid.to_string()),
    )
    .await;
    create_and_complete_task(
        &harness,
        "Area task 2",
        None,
        None,
        Some(area_uuid.to_string()),
    )
    .await;
    create_and_complete_task(&harness, "No area task", None, None, None).await;

    // Search by area
    let result = harness
        .call_tool(
            "logbook_search",
            Some(json!({
                "area_uuid": area_uuid
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    assert!(response.is_array());
    let tasks = response.as_array().unwrap();
    assert_eq!(tasks.len(), 2, "Should find 2 tasks in the area");

    for task in tasks {
        let task_area = task["area_uuid"].as_str();
        assert_eq!(task_area, Some(area_uuid), "Task should belong to the area");
    }
}

#[tokio::test]
async fn test_logbook_search_with_tags() {
    let harness = create_harness();

    // Create tasks with tags
    create_and_complete_task(
        &harness,
        "Work task",
        Some(vec!["work".to_string()]),
        None,
        None,
    )
    .await;
    create_and_complete_task(
        &harness,
        "Personal task",
        Some(vec!["personal".to_string()]),
        None,
        None,
    )
    .await;
    create_and_complete_task(
        &harness,
        "Both tags task",
        Some(vec!["work".to_string(), "personal".to_string()]),
        None,
        None,
    )
    .await;

    // Search by "work" tag
    let result = harness
        .call_tool(
            "logbook_search",
            Some(json!({
                "tags": ["work"]
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    assert!(response.is_array());
    let tasks = response.as_array().unwrap();
    assert_eq!(tasks.len(), 2, "Should find 2 tasks with 'work' tag");

    for task in tasks {
        let tags = task["tags"].as_array().unwrap();
        assert!(
            tags.iter().any(|t| t.as_str() == Some("work")),
            "Task should have 'work' tag"
        );
    }
}

#[tokio::test]
async fn test_logbook_search_with_multiple_tags() {
    let harness = create_harness();

    // Create tasks with various tag combinations
    create_and_complete_task(
        &harness,
        "Work and urgent",
        Some(vec!["work".to_string(), "urgent".to_string()]),
        None,
        None,
    )
    .await;
    create_and_complete_task(
        &harness,
        "Work only",
        Some(vec!["work".to_string()]),
        None,
        None,
    )
    .await;
    create_and_complete_task(
        &harness,
        "Urgent only",
        Some(vec!["urgent".to_string()]),
        None,
        None,
    )
    .await;

    // Search for tasks with BOTH "work" AND "urgent" tags
    let result = harness
        .call_tool(
            "logbook_search",
            Some(json!({
                "tags": ["work", "urgent"]
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    assert!(response.is_array());
    let tasks = response.as_array().unwrap();
    assert_eq!(tasks.len(), 1, "Should find only 1 task with both tags");

    let task = &tasks[0];
    let tags = task["tags"].as_array().unwrap();
    assert!(tags.iter().any(|t| t.as_str() == Some("work")));
    assert!(tags.iter().any(|t| t.as_str() == Some("urgent")));
}

#[tokio::test]
async fn test_logbook_search_with_limit() {
    let harness = create_harness();

    // Create many completed tasks
    for i in 1..=20 {
        create_and_complete_task(&harness, &format!("Task {i}"), None, None, None).await;
    }

    // Search with limit of 5
    let result = harness
        .call_tool(
            "logbook_search",
            Some(json!({
                "limit": 5
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    assert!(response.is_array());
    let tasks = response.as_array().unwrap();
    assert_eq!(tasks.len(), 5, "Should return exactly 5 results");
}

#[tokio::test]
async fn test_logbook_search_combined_filters() {
    let harness = create_harness();

    // Create a project
    let project_result = harness
        .call_tool(
            "create_project",
            Some(json!({
                "title": "Important Project"
            })),
        )
        .await;
    let project_response = parse_tool_result(&project_result);
    let project_uuid = project_response["uuid"].as_str().unwrap();

    let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();

    // Create tasks with various properties
    create_and_complete_task(
        &harness,
        "Matching important task",
        None,
        Some(project_uuid.to_string()),
        None,
    )
    .await;
    create_and_complete_task(&harness, "Non-matching task", None, None, None).await;
    create_and_complete_task(
        &harness,
        "Another matching important task",
        None,
        Some(project_uuid.to_string()),
        None,
    )
    .await;

    // Search with text + project + date filter
    let result = harness
        .call_tool(
            "logbook_search",
            Some(json!({
                "search_text": "matching",
                "project_uuid": project_uuid,
                "from_date": today
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    assert!(response.is_array());
    let tasks = response.as_array().unwrap();
    assert_eq!(tasks.len(), 2, "Should find 2 tasks matching all filters");

    for task in tasks {
        let title = task["title"].as_str().unwrap().to_lowercase();
        assert!(
            title.contains("matching"),
            "Title should contain 'matching'"
        );
        assert_eq!(
            task["project_uuid"].as_str(),
            Some(project_uuid),
            "Task should belong to the project"
        );
    }
}

#[tokio::test]
async fn test_logbook_search_empty_results() {
    let harness = create_harness();

    // Create a completed task
    create_and_complete_task(&harness, "Test task", None, None, None).await;

    // Search for non-existent text
    let result = harness
        .call_tool(
            "logbook_search",
            Some(json!({
                "search_text": "nonexistent"
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    assert!(response.is_array());
    let tasks = response.as_array().unwrap();
    assert_eq!(tasks.len(), 0, "Should return empty array for no matches");
}

#[tokio::test]
async fn test_logbook_search_invalid_date_format() {
    let harness = create_harness();

    // Try to search with invalid date format
    let result = harness
        .call_tool(
            "logbook_search",
            Some(json!({
                "from_date": "invalid-date"
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    // Should still succeed but ignore the invalid date
    assert!(response.is_array());
}

#[tokio::test]
async fn test_logbook_search_invalid_uuid() {
    let harness = create_harness();

    // Try to search with invalid UUID
    let result = harness
        .call_tool(
            "logbook_search",
            Some(json!({
                "project_uuid": "invalid-uuid"
            })),
        )
        .await;
    let response = parse_tool_result(&result);

    // Should still succeed but ignore the invalid UUID
    assert!(response.is_array());
}
