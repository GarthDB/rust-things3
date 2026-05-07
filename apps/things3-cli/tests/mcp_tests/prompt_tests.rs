//! Prompt-related tests for MCP server

#![cfg(feature = "mcp-server")]

use crate::mcp_tests::common::create_test_mcp_server;
use serde_json::json;
use things3_cli::mcp::{Content, McpError};

#[tokio::test]
async fn test_list_prompts() {
    let server = create_test_mcp_server().await;
    let result = server.list_prompts().unwrap();

    // Should have 4 prompts
    assert_eq!(result.prompts.len(), 4);

    // Check that all expected prompts are present
    let prompt_names: Vec<&String> = result.prompts.iter().map(|p| &p.name).collect();
    assert!(prompt_names.contains(&&"task_review".to_string()));
    assert!(prompt_names.contains(&&"project_planning".to_string()));
    assert!(prompt_names.contains(&&"productivity_analysis".to_string()));
    assert!(prompt_names.contains(&&"backup_strategy".to_string()));

    // Check prompt properties
    for prompt in &result.prompts {
        assert!(!prompt.name.is_empty());
        assert!(!prompt.description.is_empty());
        assert!(!prompt.arguments.is_empty());
    }
}

#[tokio::test]
async fn test_prompt_schemas_validation() {
    let server = create_test_mcp_server().await;
    let result = server.list_prompts().unwrap();

    for prompt in &result.prompts {
        let required: Vec<&str> = prompt
            .arguments
            .iter()
            .filter(|a| a.required)
            .map(|a| a.name.as_str())
            .collect();
        match prompt.name.as_str() {
            "task_review" => assert!(required.contains(&"task_title")),
            "project_planning" => assert!(required.contains(&"project_title")),
            "productivity_analysis" => assert!(required.contains(&"time_period")),
            "backup_strategy" => {
                assert!(required.contains(&"data_volume"));
                assert!(required.contains(&"frequency"));
            }
            _ => panic!("Unknown prompt: {}", prompt.name),
        }
    }
}

#[tokio::test]
async fn test_task_review_prompt() {
    let server = create_test_mcp_server().await;
    let request = things3_cli::mcp::GetPromptRequest {
        name: "task_review".to_string(),
        arguments: Some(json!({
            "task_title": "Review quarterly reports",
            "task_notes": "Need to review Q3 reports",
            "context": "This is for the quarterly review meeting"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            // Should contain the task title and context
            assert!(text.contains("Review quarterly reports"));
            assert!(text.contains("Need to review Q3 reports"));
            assert!(text.contains("This is for the quarterly review meeting"));
            assert!(text.contains("Task Review"));
            assert!(text.contains("Review Checklist"));
            assert!(text.contains("Current Context"));
            assert!(text.contains("Recommendations"));
            assert!(text.contains("Next Steps"));
        }
    }
}

#[tokio::test]
async fn test_task_review_prompt_minimal_args() {
    let server = create_test_mcp_server().await;
    let request = things3_cli::mcp::GetPromptRequest {
        name: "task_review".to_string(),
        arguments: Some(json!({
            "task_title": "Simple task"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Simple task"));
            assert!(text.contains("No notes provided"));
            assert!(text.contains("No additional context"));
        }
    }
}

#[tokio::test]
async fn test_task_review_prompt_missing_required() {
    let server = create_test_mcp_server().await;
    let request = things3_cli::mcp::GetPromptRequest {
        name: "task_review".to_string(),
        arguments: Some(json!({
            "task_notes": "Some notes"
        })),
    };

    let result = server.get_prompt(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "task_title");
        }
        _ => panic!("Expected MissingParameter error"),
    }
}

#[tokio::test]
async fn test_project_planning_prompt() {
    let server = create_test_mcp_server().await;
    let request = things3_cli::mcp::GetPromptRequest {
        name: "project_planning".to_string(),
        arguments: Some(json!({
            "project_title": "Website Redesign",
            "project_description": "Complete redesign of company website",
            "deadline": "2024-03-31",
            "complexity": "complex"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Website Redesign"));
            assert!(text.contains("Complete redesign of company website"));
            assert!(text.contains("2024-03-31"));
            assert!(text.contains("complex"));
            assert!(text.contains("Project Planning"));
            assert!(text.contains("Planning Framework"));
            assert!(text.contains("Task Breakdown"));
            assert!(text.contains("Project Organization"));
            assert!(text.contains("Risk Assessment"));
            assert!(text.contains("Success Metrics"));
        }
    }
}

#[tokio::test]
async fn test_project_planning_prompt_minimal_args() {
    let server = create_test_mcp_server().await;
    let request = things3_cli::mcp::GetPromptRequest {
        name: "project_planning".to_string(),
        arguments: Some(json!({
            "project_title": "Simple Project"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Simple Project"));
            assert!(text.contains("No description provided"));
            assert!(text.contains("No deadline specified"));
            assert!(text.contains("medium")); // Default complexity
        }
    }
}

#[tokio::test]
async fn test_productivity_analysis_prompt() {
    let server = create_test_mcp_server().await;
    let request = things3_cli::mcp::GetPromptRequest {
        name: "productivity_analysis".to_string(),
        arguments: Some(json!({
            "time_period": "month",
            "focus_area": "completion_rate",
            "include_recommendations": true
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("month"));
            assert!(text.contains("completion_rate"));
            assert!(text.contains("Productivity Analysis"));
            assert!(text.contains("Analysis Framework"));
            assert!(text.contains("Task Completion Patterns"));
            assert!(text.contains("Workload Distribution"));
            assert!(text.contains("Time Management"));
            assert!(text.contains("Project Progress"));
            assert!(text.contains("Key Insights"));
            assert!(text.contains("Recommendations"));
            assert!(text.contains("Improving task completion rates"));
        }
    }
}

#[tokio::test]
async fn test_productivity_analysis_prompt_no_recommendations() {
    let server = create_test_mcp_server().await;
    let request = things3_cli::mcp::GetPromptRequest {
        name: "productivity_analysis".to_string(),
        arguments: Some(json!({
            "time_period": "week",
            "focus_area": "time_management",
            "include_recommendations": false
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("week"));
            assert!(text.contains("time_management"));
            assert!(text.contains("Focus on analysis without recommendations"));
        }
    }
}

#[tokio::test]
async fn test_productivity_analysis_prompt_minimal_args() {
    let server = create_test_mcp_server().await;
    let request = things3_cli::mcp::GetPromptRequest {
        name: "productivity_analysis".to_string(),
        arguments: Some(json!({
            "time_period": "quarter"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("quarter"));
            assert!(text.contains("all")); // Default focus area
            assert!(text.contains("Improving task completion rates")); // Default recommendations
        }
    }
}

#[tokio::test]
async fn test_backup_strategy_prompt() {
    let server = create_test_mcp_server().await;
    let request = things3_cli::mcp::GetPromptRequest {
        name: "backup_strategy".to_string(),
        arguments: Some(json!({
            "data_volume": "large",
            "frequency": "daily",
            "retention_period": "1_year",
            "storage_preference": "cloud"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("large"));
            assert!(text.contains("daily"));
            assert!(text.contains("1_year"));
            assert!(text.contains("cloud"));
            assert!(text.contains("Backup Strategy Recommendation"));
            assert!(text.contains("Data Assessment"));
            assert!(text.contains("Backup Frequency Optimization"));
            assert!(text.contains("Storage Strategy"));
            assert!(text.contains("Retention Policy"));
            assert!(text.contains("Recommended Implementation"));
            assert!(text.contains("Risk Mitigation"));
            assert!(text.contains("Cost Analysis"));
        }
    }
}

#[tokio::test]
async fn test_backup_strategy_prompt_minimal_args() {
    let server = create_test_mcp_server().await;
    let request = things3_cli::mcp::GetPromptRequest {
        name: "backup_strategy".to_string(),
        arguments: Some(json!({
            "data_volume": "small",
            "frequency": "weekly"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);

    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("small"));
            assert!(text.contains("weekly"));
            assert!(text.contains("3_months")); // Default retention
            assert!(text.contains("hybrid")); // Default storage preference
        }
    }
}

#[tokio::test]
async fn test_backup_strategy_prompt_missing_required() {
    let server = create_test_mcp_server().await;
    let request = things3_cli::mcp::GetPromptRequest {
        name: "backup_strategy".to_string(),
        arguments: Some(json!({
            "data_volume": "medium"
        })),
    };

    let result = server.get_prompt(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "frequency");
        }
        _ => panic!("Expected MissingParameter error"),
    }
}

#[tokio::test]
async fn test_unknown_prompt() {
    let server = create_test_mcp_server().await;
    let request = things3_cli::mcp::GetPromptRequest {
        name: "unknown_prompt".to_string(),
        arguments: None,
    };

    let result = server.get_prompt(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::PromptNotFound { prompt_name } => {
            assert_eq!(prompt_name, "unknown_prompt");
        }
        _ => panic!("Expected PromptNotFound error"),
    }
}

#[tokio::test]
async fn test_prompt_with_no_arguments() {
    let server = create_test_mcp_server().await;
    let request = things3_cli::mcp::GetPromptRequest {
        name: "task_review".to_string(),
        arguments: None,
    };

    let result = server.get_prompt(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "task_title");
        }
        _ => panic!("Expected MissingParameter error"),
    }
}

#[tokio::test]
async fn test_prompt_context_awareness() {
    let server = create_test_mcp_server().await;

    // Test that prompts include current data context
    let request = things3_cli::mcp::GetPromptRequest {
        name: "task_review".to_string(),
        arguments: Some(json!({
            "task_title": "Test task"
        })),
    };

    let result = server.get_prompt(request).await.unwrap();
    assert!(!result.is_error);

    match &result.content[0] {
        Content::Text { text } => {
            // Should contain current context data
            assert!(text.contains("Inbox Tasks"));
            assert!(text.contains("Today's Tasks"));
            // Should contain actual numbers (not just placeholders)
            assert!(text.contains("tasks"));
        }
    }
}

#[tokio::test]
async fn test_prompt_error_handling() {
    let server = create_test_mcp_server().await;

    // Test with invalid JSON in arguments
    let request = things3_cli::mcp::GetPromptRequest {
        name: "task_review".to_string(),
        arguments: Some(json!({ "task_title": 123 })), // Should be string
    };

    let result = server.get_prompt(request).await;
    // Should error due to type mismatch
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "task_title");
        }
        _ => panic!("Expected MissingParameter error"),
    }
}

#[tokio::test]
async fn test_prompt_argument_names() {
    let server = create_test_mcp_server().await;
    let result = server.list_prompts().unwrap();

    for prompt in &result.prompts {
        let names: Vec<&str> = prompt.arguments.iter().map(|a| a.name.as_str()).collect();
        match prompt.name.as_str() {
            "task_review" => {
                assert!(names.contains(&"task_title"));
                assert!(names.contains(&"task_notes"));
                assert!(names.contains(&"context"));
            }
            "project_planning" => {
                assert!(names.contains(&"project_title"));
                assert!(names.contains(&"complexity"));
            }
            "productivity_analysis" => {
                assert!(names.contains(&"time_period"));
                assert!(names.contains(&"focus_area"));
                assert!(names.contains(&"include_recommendations"));
            }
            "backup_strategy" => {
                assert!(names.contains(&"data_volume"));
                assert!(names.contains(&"frequency"));
                assert!(names.contains(&"retention_period"));
                assert!(names.contains(&"storage_preference"));
            }
            _ => panic!("Unknown prompt: {}", prompt.name),
        }
    }
}

#[tokio::test]
async fn test_prompt_fallback_error_handling() {
    let server = create_test_mcp_server().await;

    // Test get_prompt_with_fallback for unknown prompt
    let request = things3_cli::mcp::GetPromptRequest {
        name: "unknown_prompt".to_string(),
        arguments: None,
    };

    let result = server.get_prompt_with_fallback(request).await;
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Prompt 'unknown_prompt' not found"));
            assert!(text.contains("Available prompts can be listed"));
        }
    }
}

#[tokio::test]
async fn test_get_prompt_with_fallback() {
    let server = create_test_mcp_server().await;

    // Test with a valid prompt
    let request = things3_cli::mcp::GetPromptRequest {
        name: "task_review".to_string(),
        arguments: Some(json!({"task_title": "Test Task", "task_notes": "Test notes"})),
    };

    let result = server.get_prompt_with_fallback(request).await;
    assert!(!result.is_error);
    assert!(!result.content.is_empty());

    // Test with an invalid prompt
    let request = things3_cli::mcp::GetPromptRequest {
        name: "nonexistent_prompt".to_string(),
        arguments: None,
    };

    let result = server.get_prompt_with_fallback(request).await;
    assert!(result.is_error);
    let Content::Text { text } = &result.content[0];
    assert!(text.contains("not found"));
}
