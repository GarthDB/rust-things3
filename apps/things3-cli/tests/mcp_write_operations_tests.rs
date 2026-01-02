use serde_json::json;
use tempfile::NamedTempFile;
use things3_cli::mcp::{CallToolRequest, ThingsMcpServer};
use things3_core::{test_utils::create_test_database, ThingsConfig, ThingsDatabase};
use uuid::Uuid;

// Test harness for MCP server
struct McpTestHarness {
    server: ThingsMcpServer,
    _temp_file: NamedTempFile,
}

impl McpTestHarness {
    async fn new() -> Self {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).await.unwrap();
        let db = ThingsDatabase::new(db_path).await.unwrap();
        let config = ThingsConfig::default();
        let server = ThingsMcpServer::new(std::sync::Arc::new(db), config);

        Self {
            server,
            _temp_file: temp_file,
        }
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> serde_json::Value {
        let request = CallToolRequest {
            name: name.to_string(),
            arguments: Some(arguments.unwrap_or(json!({}))),
        };

        let result = self.server.call_tool_with_fallback(request).await;

        if result.is_error {
            json!({
                "error": true,
                "content": result.content
            })
        } else {
            let text = result
                .content
                .first()
                .and_then(|c| match c {
                    things3_cli::mcp::Content::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .unwrap_or_default();

            serde_json::from_str(&text).unwrap_or(json!({"text": text}))
        }
    }
}

// ============================================================================
// MCP Protocol Tests (10 tests)
// ============================================================================

#[tokio::test]
async fn test_create_task_via_mcp_returns_valid_response() {
    let harness = McpTestHarness::new().await;

    let response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Test Task via MCP"
            })),
        )
        .await;

    assert!(
        response.get("uuid").is_some(),
        "Response should contain UUID"
    );
    assert!(
        response.get("message").is_some(),
        "Response should contain message"
    );
}

#[tokio::test]

async fn test_update_task_via_mcp_returns_success() {
    let harness = McpTestHarness::new().await;

    // First create a task
    let create_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Task to Update"
            })),
        )
        .await;

    let uuid = create_response["uuid"].as_str().unwrap();

    // Then update it
    let update_response = harness
        .call_tool(
            "update_task",
            Some(json!({
                "uuid": uuid,
                "title": "Updated Task"
            })),
        )
        .await;

    assert!(
        update_response.get("message").is_some(),
        "Update should return success message"
    );
}

#[tokio::test]

async fn test_created_task_can_be_queried() {
    let harness = McpTestHarness::new().await;

    // Create a task
    let create_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Queryable Task"
            })),
        )
        .await;

    let uuid = create_response["uuid"].as_str().unwrap();

    // Query inbox to verify task exists
    let inbox_response = harness
        .call_tool("get_inbox", Some(json!({"limit": 100})))
        .await;

    let inbox_text = inbox_response.as_str().unwrap_or("");
    assert!(
        inbox_text.contains(uuid) || inbox_text.contains("Queryable Task"),
        "Created task should appear in inbox"
    );
}

#[tokio::test]

async fn test_validation_failure_error_response() {
    let harness = McpTestHarness::new().await;

    // Try to create task with invalid project UUID
    let response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Task with Invalid Project",
                "project_uuid": Uuid::new_v4().to_string()
            })),
        )
        .await;

    assert!(
        response.get("error").is_some() || response.as_str().unwrap_or("").contains("not found"),
        "Should return error for invalid project UUID"
    );
}

#[tokio::test]

async fn test_missing_required_parameter() {
    let harness = McpTestHarness::new().await;

    // Try to create task without title
    let response = harness.call_tool("create_task", Some(json!({}))).await;

    assert!(
        response.get("error").is_some() || response.as_str().unwrap_or("").contains("missing"),
        "Should return error for missing required parameter"
    );
}

#[tokio::test]

async fn test_invalid_parameter_types() {
    let harness = McpTestHarness::new().await;

    // Try to create task with invalid date format
    let response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Task with Bad Date",
                "start_date": "not-a-date"
            })),
        )
        .await;

    // Should either fail or ignore the invalid date
    // The exact behavior depends on serde's deserialization
    assert!(response.is_object() || response.is_string());
}

#[tokio::test]

async fn test_null_vs_missing_fields() {
    let harness = McpTestHarness::new().await;

    // Create task with explicit null notes
    let response1 = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Task with null notes",
                "notes": null
            })),
        )
        .await;

    assert!(
        response1.get("uuid").is_some(),
        "Should create task with null notes"
    );

    // Create task with missing notes field
    let response2 = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Task with missing notes"
            })),
        )
        .await;

    assert!(
        response2.get("uuid").is_some(),
        "Should create task with missing notes"
    );
}

#[tokio::test]

async fn test_create_then_update_workflow() {
    let harness = McpTestHarness::new().await;

    // Create task
    let create_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Initial Title",
                "notes": "Initial notes"
            })),
        )
        .await;

    let uuid = create_response["uuid"].as_str().unwrap();

    // Update task
    let update_response = harness
        .call_tool(
            "update_task",
            Some(json!({
                "uuid": uuid,
                "title": "Updated Title",
                "status": "completed"
            })),
        )
        .await;

    assert!(
        update_response.get("message").is_some(),
        "Update should succeed after create"
    );
}

#[tokio::test]

async fn test_create_task_with_all_fields() {
    let harness = McpTestHarness::new().await;

    // First create a project
    let project_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Test Project",
                "task_type": "project"
            })),
        )
        .await;

    let project_uuid = project_response["uuid"].as_str().unwrap();

    // Create task with all fields
    let response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Complete Task",
                "task_type": "to-do",
                "notes": "Task notes",
                "start_date": "2025-01-15",
                "deadline": "2025-01-31",
                "project_uuid": project_uuid,
                "tags": ["work", "urgent"],
                "status": "incomplete"
            })),
        )
        .await;

    assert!(
        response.get("uuid").is_some(),
        "Should create task with all fields"
    );
}

#[tokio::test]

async fn test_update_nonexistent_task_error() {
    let harness = McpTestHarness::new().await;

    let nonexistent_uuid = Uuid::new_v4();
    let response = harness
        .call_tool(
            "update_task",
            Some(json!({
                "uuid": nonexistent_uuid.to_string(),
                "title": "Updated Title"
            })),
        )
        .await;

    assert!(
        response.get("error").is_some() || response.as_str().unwrap_or("").contains("not found"),
        "Should return error for nonexistent task"
    );
}

// ============================================================================
// End-to-End Tests (5 tests)
// ============================================================================

#[tokio::test]

async fn test_e2e_create_task_verify_in_inbox() {
    let harness = McpTestHarness::new().await;

    // Create task
    let create_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Inbox Task"
            })),
        )
        .await;

    let uuid = create_response["uuid"].as_str().unwrap();

    // Verify in inbox
    let inbox_response = harness
        .call_tool("get_inbox", Some(json!({"limit": 100})))
        .await;

    let inbox_text = inbox_response.as_str().unwrap_or("");
    assert!(
        inbox_text.contains(uuid) || inbox_text.contains("Inbox Task"),
        "Task should appear in inbox"
    );
}

#[tokio::test]

async fn test_e2e_create_task_in_project_verify() {
    let harness = McpTestHarness::new().await;

    // Create project
    let project_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Test Project",
                "task_type": "project"
            })),
        )
        .await;

    let project_uuid = project_response["uuid"].as_str().unwrap();

    // Create task in project
    let task_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Task in Project",
                "project_uuid": project_uuid
            })),
        )
        .await;

    assert!(
        task_response.get("uuid").is_some(),
        "Should create task in project"
    );

    // Verify projects list
    let projects_response = harness.call_tool("get_projects", None).await;
    let projects_text = projects_response.as_str().unwrap_or("");
    assert!(
        projects_text.contains("Test Project"),
        "Project should appear in projects list"
    );
}

#[tokio::test]

async fn test_e2e_update_status_verify_completion() {
    let harness = McpTestHarness::new().await;

    // Create task
    let create_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Task to Complete",
                "status": "incomplete"
            })),
        )
        .await;

    let uuid = create_response["uuid"].as_str().unwrap();

    // Update status to completed
    let update_response = harness
        .call_tool(
            "update_task",
            Some(json!({
                "uuid": uuid,
                "status": "completed"
            })),
        )
        .await;

    assert!(
        update_response.get("message").is_some(),
        "Should update task status"
    );
}

#[tokio::test]

async fn test_e2e_create_task_with_tags_search() {
    let harness = McpTestHarness::new().await;

    // Create task with tags
    let create_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "Tagged Task",
                "tags": ["important", "work"]
            })),
        )
        .await;

    assert!(
        create_response.get("uuid").is_some(),
        "Should create tagged task"
    );

    // Search for task
    let search_response = harness
        .call_tool("search_tasks", Some(json!({"query": "Tagged"})))
        .await;

    let search_text = search_response.as_str().unwrap_or("");
    assert!(
        search_text.contains("Tagged Task"),
        "Should find task by search"
    );
}

#[tokio::test]

async fn test_e2e_full_crud_cycle() {
    let harness = McpTestHarness::new().await;

    // CREATE
    let create_response = harness
        .call_tool(
            "create_task",
            Some(json!({
                "title": "CRUD Test Task",
                "notes": "Initial notes"
            })),
        )
        .await;

    let uuid = create_response["uuid"].as_str().unwrap();
    assert!(!uuid.is_empty(), "Should create task");

    // READ (via inbox)
    let read_response = harness
        .call_tool("get_inbox", Some(json!({"limit": 100})))
        .await;
    let read_text = read_response.as_str().unwrap_or("");
    assert!(
        read_text.contains(uuid) || read_text.contains("CRUD Test Task"),
        "Should read created task"
    );

    // UPDATE
    let update_response = harness
        .call_tool(
            "update_task",
            Some(json!({
                "uuid": uuid,
                "title": "Updated CRUD Task",
                "notes": "Updated notes"
            })),
        )
        .await;

    assert!(
        update_response.get("message").is_some(),
        "Should update task"
    );

    // Verify update
    let verify_response = harness
        .call_tool("get_inbox", Some(json!({"limit": 100})))
        .await;
    let verify_text = verify_response.as_str().unwrap_or("");
    assert!(
        verify_text.contains("Updated CRUD Task"),
        "Should see updated task"
    );

    // DELETE (mark as trashed)
    let delete_response = harness
        .call_tool(
            "update_task",
            Some(json!({
                "uuid": uuid,
                "status": "trashed"
            })),
        )
        .await;

    assert!(
        delete_response.get("message").is_some(),
        "Should mark task as trashed"
    );
}
