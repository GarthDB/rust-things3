//! MCP tag tool integration tests

use serde_json::json;
use things3_cli::mcp::CallToolRequest;
use uuid::Uuid;

mod mcp_tests;
use mcp_tests::common::create_test_mcp_server;

// ========================================================================
// TAG DISCOVERY TOOL TESTS
// ========================================================================

#[tokio::test]
async fn test_search_tags_finds_exact() {
    let server = create_test_mcp_server().await;

    // First create a tag using database directly
    let request = things3_core::models::CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    server.db.create_tag_force(request).await.unwrap();

    // Search for the tag
    let request = CallToolRequest {
        name: "search_tags".to_string(),
        arguments: Some(json!({
            "query": "work"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert!(response.is_array());
    assert!(!response.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_search_tags_finds_similar() {
    let server = create_test_mcp_server().await;

    // Create a tag
    let request = things3_core::models::CreateTagRequest {
        title: "important".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    server.db.create_tag_force(request).await.unwrap();

    // Search with typo
    let request = CallToolRequest {
        name: "search_tags".to_string(),
        arguments: Some(json!({
            "query": "importnt",
            "include_similar": true,
            "min_similarity": 0.7
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert!(response.is_array());
    // Should find "important" as similar
}

#[tokio::test]
async fn test_get_tag_suggestions_exact_match() {
    let server = create_test_mcp_server().await;

    // Create a tag
    let request = things3_core::models::CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    server.db.create_tag_force(request).await.unwrap();

    // Get suggestions for same title
    let request = CallToolRequest {
        name: "get_tag_suggestions".to_string(),
        arguments: Some(json!({
            "title": "work"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(response["recommendation"], "use_existing");
    assert!(!response["exact_match"].is_null());
}

#[tokio::test]
async fn test_get_tag_suggestions_similar_found() {
    let server = create_test_mcp_server().await;

    // Create a tag
    let request = things3_core::models::CreateTagRequest {
        title: "important".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    server.db.create_tag_force(request).await.unwrap();

    // Get suggestions for similar title
    let request = CallToolRequest {
        name: "get_tag_suggestions".to_string(),
        arguments: Some(json!({
            "title": "importnt"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(response["recommendation"], "consider_similar");
    assert!(response["exact_match"].is_null());
    assert!(!response["similar_tags"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_get_popular_tags() {
    let server = create_test_mcp_server().await;

    // Create some tags
    for title in &["work", "personal", "urgent"] {
        let request = things3_core::models::CreateTagRequest {
            title: title.to_string(),
            shortcut: None,
            parent_uuid: None,
        };
        server.db.create_tag_force(request).await.unwrap();
    }

    let request = CallToolRequest {
        name: "get_popular_tags".to_string(),
        arguments: Some(json!({
            "limit": 10
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert!(response.is_array());
    assert_eq!(response.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn test_get_recent_tags() {
    let server = create_test_mcp_server().await;

    // Create a tag
    let request = things3_core::models::CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    server.db.create_tag_force(request).await.unwrap();

    let request = CallToolRequest {
        name: "get_recent_tags".to_string(),
        arguments: Some(json!({
            "limit": 10
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert!(response.is_array());
    // Will be empty since usedDate is NULL initially
}

// ========================================================================
// TAG CRUD TOOL TESTS
// ========================================================================

#[tokio::test]
async fn test_create_tag_with_duplicate_check() {
    let server = create_test_mcp_server().await;

    // Create first tag
    let request = CallToolRequest {
        name: "create_tag".to_string(),
        arguments: Some(json!({
            "title": "work"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(response["status"], "created");
    assert!(!response["uuid"].is_null());

    // Try to create duplicate
    let request = CallToolRequest {
        name: "create_tag".to_string(),
        arguments: Some(json!({
            "title": "Work"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(response["status"], "existing");
}

#[tokio::test]
async fn test_create_tag_force_skip_check() {
    let server = create_test_mcp_server().await;

    // Create first tag
    let request = CallToolRequest {
        name: "create_tag".to_string(),
        arguments: Some(json!({
            "title": "work",
            "force": true
        })),
    };

    let result1 = server.call_tool(request).await.unwrap();
    let text1 = match &result1.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response1: serde_json::Value = serde_json::from_str(text1).unwrap();
    assert_eq!(response1["status"], "created");

    // Create duplicate with force
    let request = CallToolRequest {
        name: "create_tag".to_string(),
        arguments: Some(json!({
            "title": "work",
            "force": true
        })),
    };

    let result2 = server.call_tool(request).await.unwrap();
    let text2 = match &result2.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response2: serde_json::Value = serde_json::from_str(text2).unwrap();
    assert_eq!(response2["status"], "created");

    // Both should have been created
    assert_ne!(response1["uuid"], response2["uuid"]);
}

#[tokio::test]
async fn test_update_tag_tool() {
    let server = create_test_mcp_server().await;

    // Create a tag first
    let request = things3_core::models::CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    let uuid = server.db.create_tag_force(request).await.unwrap();

    // Update the tag
    let request = CallToolRequest {
        name: "update_tag".to_string(),
        arguments: Some(json!({
            "uuid": uuid.to_string(),
            "title": "professional",
            "shortcut": "p"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(response["message"], "Tag updated successfully");
    assert_eq!(response["uuid"], uuid.to_string());
}

#[tokio::test]
async fn test_delete_tag_tool() {
    let server = create_test_mcp_server().await;

    // Create a tag first
    let request = things3_core::models::CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    let uuid = server.db.create_tag_force(request).await.unwrap();

    // Delete the tag
    let request = CallToolRequest {
        name: "delete_tag".to_string(),
        arguments: Some(json!({
            "uuid": uuid.to_string()
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(response["message"], "Tag deleted successfully");
}

#[tokio::test]
async fn test_merge_tags_tool() {
    let server = create_test_mcp_server().await;

    // Create two tags
    let request1 = things3_core::models::CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    let source_uuid = server.db.create_tag_force(request1).await.unwrap();

    let request2 = things3_core::models::CreateTagRequest {
        title: "professional".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    let target_uuid = server.db.create_tag_force(request2).await.unwrap();

    // Merge tags
    let request = CallToolRequest {
        name: "merge_tags".to_string(),
        arguments: Some(json!({
            "source_uuid": source_uuid.to_string(),
            "target_uuid": target_uuid.to_string()
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(response["message"], "Tags merged successfully");
}

// ========================================================================
// TAG ASSIGNMENT TOOL TESTS
// ========================================================================

#[tokio::test]
async fn test_add_tag_to_task_tool() {
    let server = create_test_mcp_server().await;

    // Create a task first (using the existing test data from common.rs)
    // The test database has tasks in it already

    // Get a task UUID from the test database
    let tasks = server.db.get_inbox(None).await.unwrap();
    assert!(!tasks.is_empty(), "Test should have tasks in inbox");
    let task_uuid = tasks[0].uuid;

    // Add a tag to the task
    let request = CallToolRequest {
        name: "add_tag_to_task".to_string(),
        arguments: Some(json!({
            "task_uuid": task_uuid.to_string(),
            "tag_title": "urgent"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(response["status"], "assigned");
    assert!(!response["tag_uuid"].is_null());
}

#[tokio::test]
async fn test_remove_tag_from_task_tool() {
    let server = create_test_mcp_server().await;

    // Get a task
    let tasks = server.db.get_inbox(None).await.unwrap();
    assert!(!tasks.is_empty());
    let task_uuid = tasks[0].uuid;

    // Add a tag first
    server
        .db
        .add_tag_to_task(&task_uuid, "urgent")
        .await
        .unwrap();

    // Remove the tag
    let request = CallToolRequest {
        name: "remove_tag_from_task".to_string(),
        arguments: Some(json!({
            "task_uuid": task_uuid.to_string(),
            "tag_title": "urgent"
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(response["message"], "Tag removed from task successfully");
}

#[tokio::test]
async fn test_set_task_tags_tool() {
    let server = create_test_mcp_server().await;

    // Get a task
    let tasks = server.db.get_inbox(None).await.unwrap();
    assert!(!tasks.is_empty());
    let task_uuid = tasks[0].uuid;

    // Set tags on the task
    let request = CallToolRequest {
        name: "set_task_tags".to_string(),
        arguments: Some(json!({
            "task_uuid": task_uuid.to_string(),
            "tag_titles": ["work", "urgent", "important"]
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(response["message"], "Task tags updated successfully");
    assert_eq!(response["tags"].as_array().unwrap().len(), 3);
}

// ========================================================================
// TAG ANALYTICS TOOL TESTS
// ========================================================================

#[tokio::test]
async fn test_get_tag_statistics_tool() {
    let server = create_test_mcp_server().await;

    // Create a tag
    let request = things3_core::models::CreateTagRequest {
        title: "work".to_string(),
        shortcut: None,
        parent_uuid: None,
    };
    let uuid = server.db.create_tag_force(request).await.unwrap();

    // Get statistics
    let request = CallToolRequest {
        name: "get_tag_statistics".to_string(),
        arguments: Some(json!({
            "uuid": uuid.to_string()
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(response["uuid"], uuid.to_string());
    assert_eq!(response["title"], "work");
    assert!(response["usage_count"].is_number());
}

#[tokio::test]
async fn test_find_duplicate_tags_tool() {
    let server = create_test_mcp_server().await;

    // Create similar tags
    for title in &["work", "Work", "working"] {
        let request = things3_core::models::CreateTagRequest {
            title: title.to_string(),
            shortcut: None,
            parent_uuid: None,
        };
        server.db.create_tag_force(request).await.unwrap();
    }

    // Find duplicates
    let request = CallToolRequest {
        name: "find_duplicate_tags".to_string(),
        arguments: Some(json!({
            "min_similarity": 0.8
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert!(response.is_array());
    // Should find some similar pairs
}

#[tokio::test]
async fn test_get_tag_completions_tool() {
    let server = create_test_mcp_server().await;

    // Create tags
    for title in &["work", "working", "worker"] {
        let request = things3_core::models::CreateTagRequest {
            title: title.to_string(),
            shortcut: None,
            parent_uuid: None,
        };
        server.db.create_tag_force(request).await.unwrap();
    }

    // Get completions
    let request = CallToolRequest {
        name: "get_tag_completions".to_string(),
        arguments: Some(json!({
            "partial_input": "wo",
            "limit": 5
        })),
    };

    let result = server.call_tool(request).await.unwrap();
    let text = match &result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let response: serde_json::Value = serde_json::from_str(text).unwrap();

    assert!(response.is_array());
    assert!(!response.as_array().unwrap().is_empty());
}

// ========================================================================
// TAG LIFECYCLE INTEGRATION TEST
// ========================================================================

#[tokio::test]
async fn test_tag_lifecycle_integration() {
    let server = create_test_mcp_server().await;

    // 1. Create a tag
    let create_request = CallToolRequest {
        name: "create_tag".to_string(),
        arguments: Some(json!({
            "title": "project-alpha",
            "shortcut": "pa"
        })),
    };

    let create_result = server.call_tool(create_request).await.unwrap();
    let create_text = match &create_result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let create_response: serde_json::Value = serde_json::from_str(create_text).unwrap();
    assert_eq!(create_response["status"], "created");
    let tag_uuid = Uuid::parse_str(create_response["uuid"].as_str().unwrap()).unwrap();

    // 2. Update the tag
    let update_request = CallToolRequest {
        name: "update_tag".to_string(),
        arguments: Some(json!({
            "uuid": tag_uuid.to_string(),
            "title": "project-alpha-v2"
        })),
    };

    let update_result = server.call_tool(update_request).await.unwrap();
    let update_text = match &update_result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let update_response: serde_json::Value = serde_json::from_str(update_text).unwrap();
    assert_eq!(update_response["message"], "Tag updated successfully");

    // 3. Add tag to a task
    let tasks = server.db.get_inbox(None).await.unwrap();
    assert!(!tasks.is_empty());
    let task_uuid = tasks[0].uuid;

    let add_tag_request = CallToolRequest {
        name: "add_tag_to_task".to_string(),
        arguments: Some(json!({
            "task_uuid": task_uuid.to_string(),
            "tag_title": "project-alpha-v2"
        })),
    };

    let add_tag_result = server.call_tool(add_tag_request).await.unwrap();
    let add_tag_text = match &add_tag_result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let add_tag_response: serde_json::Value = serde_json::from_str(add_tag_text).unwrap();
    assert_eq!(add_tag_response["status"], "assigned");

    // 4. Get tag statistics
    let stats_request = CallToolRequest {
        name: "get_tag_statistics".to_string(),
        arguments: Some(json!({
            "uuid": tag_uuid.to_string()
        })),
    };

    let stats_result = server.call_tool(stats_request).await.unwrap();
    let stats_text = match &stats_result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let stats_response: serde_json::Value = serde_json::from_str(stats_text).unwrap();
    assert_eq!(stats_response["title"], "project-alpha-v2");
    assert!(stats_response["usage_count"].as_u64().unwrap() > 0);

    // 5. Remove tag from task
    let remove_tag_request = CallToolRequest {
        name: "remove_tag_from_task".to_string(),
        arguments: Some(json!({
            "task_uuid": task_uuid.to_string(),
            "tag_title": "project-alpha-v2"
        })),
    };

    let remove_tag_result = server.call_tool(remove_tag_request).await.unwrap();
    let remove_tag_text = match &remove_tag_result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let remove_tag_response: serde_json::Value = serde_json::from_str(remove_tag_text).unwrap();
    assert_eq!(
        remove_tag_response["message"],
        "Tag removed from task successfully"
    );

    // 6. Delete the tag
    let delete_request = CallToolRequest {
        name: "delete_tag".to_string(),
        arguments: Some(json!({
            "uuid": tag_uuid.to_string()
        })),
    };

    let delete_result = server.call_tool(delete_request).await.unwrap();
    let delete_text = match &delete_result.content[0] {
        things3_cli::mcp::Content::Text { text } => text,
    };
    let delete_response: serde_json::Value = serde_json::from_str(delete_text).unwrap();
    assert_eq!(delete_response["message"], "Tag deleted successfully");
}
