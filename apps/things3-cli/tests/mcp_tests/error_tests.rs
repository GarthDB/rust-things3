//! Error handling tests for MCP server

#![cfg(feature = "mcp-server")]

use super::common::create_test_mcp_server;
use serde_json::json;
use things3_cli::mcp::{CallToolRequest, Content, McpError};

#[tokio::test]
async fn test_mcp_error_creation() {
    // Test McpError creation methods
    let tool_not_found = McpError::tool_not_found("test_tool");
    assert!(
        matches!(tool_not_found, McpError::ToolNotFound { tool_name } if tool_name == "test_tool")
    );

    let resource_not_found = McpError::resource_not_found("test://resource");
    assert!(
        matches!(resource_not_found, McpError::ResourceNotFound { uri } if uri == "test://resource")
    );

    let prompt_not_found = McpError::prompt_not_found("test_prompt");
    assert!(
        matches!(prompt_not_found, McpError::PromptNotFound { prompt_name } if prompt_name == "test_prompt")
    );

    let missing_param = McpError::missing_parameter("test_param");
    assert!(
        matches!(missing_param, McpError::MissingParameter { parameter_name } if parameter_name == "test_param")
    );

    let invalid_param = McpError::invalid_parameter("test_param", "invalid value");
    assert!(
        matches!(invalid_param, McpError::InvalidParameter { parameter_name, message }
        if parameter_name == "test_param" && message == "invalid value")
    );

    let invalid_format = McpError::invalid_format("xml", "json, csv");
    assert!(
        matches!(invalid_format, McpError::InvalidFormat { format, supported }
        if format == "xml" && supported == "json, csv")
    );

    let invalid_data_type = McpError::invalid_data_type("xml", "tasks, projects");
    assert!(
        matches!(invalid_data_type, McpError::InvalidDataType { data_type, supported }
        if data_type == "xml" && supported == "tasks, projects")
    );
}

#[tokio::test]
async fn test_mcp_error_to_call_result() {
    // Test tool not found error
    let tool_error = McpError::tool_not_found("unknown_tool");
    let call_result = tool_error.to_call_result();
    assert!(call_result.is_error);
    assert_eq!(call_result.content.len(), 1);
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Tool 'unknown_tool' not found"));
            assert!(text.contains("Available tools can be listed"));
        }
    }

    // Test missing parameter error
    let param_error = McpError::missing_parameter("query");
    let call_result = param_error.to_call_result();
    assert!(call_result.is_error);
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter 'query'"));
            assert!(text.contains("Please provide this parameter"));
        }
    }

    // Test invalid format error
    let format_error = McpError::invalid_format("xml", "json, csv, markdown");
    let call_result = format_error.to_call_result();
    assert!(call_result.is_error);
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Invalid format 'xml'"));
            assert!(text.contains("Supported formats: json, csv, markdown"));
        }
    }
}

#[tokio::test]
async fn test_mcp_error_to_prompt_result() {
    // Test prompt not found error
    let prompt_error = McpError::prompt_not_found("unknown_prompt");
    let prompt_result = prompt_error.to_prompt_result();
    assert!(prompt_result.is_error);
    match &prompt_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Prompt 'unknown_prompt' not found"));
            assert!(text.contains("Available prompts can be listed"));
        }
    }

    // Test missing parameter error
    let param_error = McpError::missing_parameter("task_title");
    let prompt_result = param_error.to_prompt_result();
    assert!(prompt_result.is_error);
    match &prompt_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter 'task_title'"));
        }
    }
}

#[tokio::test]
async fn test_mcp_error_to_resource_result() {
    // Test resource not found error
    let resource_error = McpError::resource_not_found("things://unknown");
    let resource_result = resource_error.to_resource_result();
    match &resource_result.contents[0] {
        Content::Text { text } => {
            assert!(text.contains("Resource 'things://unknown' not found"));
            assert!(text.contains("Available resources can be listed"));
        }
    }
}

#[tokio::test]
async fn test_from_traits() {
    // Test From<ThingsError> for McpError
    let things_error = things3_core::ThingsError::validation("Test validation error");
    let mcp_error: McpError = things_error.into();
    assert!(matches!(mcp_error, McpError::ValidationError { message }
        if message == "Test validation error"));

    // Test From<serde_json::Error> for McpError
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let mcp_error: McpError = json_error.into();
    assert!(
        matches!(mcp_error, McpError::SerializationFailed { operation, .. }
        if operation == "json serialization")
    );

    // Test From<std::io::Error> for McpError
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let mcp_error: McpError = io_error.into();
    assert!(
        matches!(mcp_error, McpError::IoOperationFailed { operation, .. }
        if operation == "file operation")
    );
}

#[tokio::test]
async fn test_from_traits_comprehensive() {
    // Test all ThingsError variants
    let db_error = things3_core::ThingsError::Database("TypeNotFound: test_column".to_string());
    let mcp_error: McpError = db_error.into();
    assert!(
        matches!(mcp_error, McpError::DatabaseOperationFailed { operation, .. } if operation == "database operation")
    );

    let serialization_error = things3_core::ThingsError::Serialization(
        serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err(),
    );
    let mcp_error: McpError = serialization_error.into();
    assert!(
        matches!(mcp_error, McpError::SerializationFailed { operation, .. } if operation == "serialization")
    );

    let io_error = things3_core::ThingsError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));
    let mcp_error: McpError = io_error.into();
    assert!(
        matches!(mcp_error, McpError::IoOperationFailed { operation, .. } if operation == "io operation")
    );

    let db_not_found = things3_core::ThingsError::DatabaseNotFound {
        path: "/test/path".to_string(),
    };
    let mcp_error: McpError = db_not_found.into();
    assert!(
        matches!(mcp_error, McpError::ConfigurationError { message } if message.contains("Database not found at: /test/path"))
    );

    let invalid_uuid = things3_core::ThingsError::InvalidUuid {
        uuid: "invalid-uuid".to_string(),
    };
    let mcp_error: McpError = invalid_uuid.into();
    assert!(
        matches!(mcp_error, McpError::ValidationError { message } if message.contains("Invalid UUID format: invalid-uuid"))
    );

    let invalid_date = things3_core::ThingsError::InvalidDate {
        date: "invalid-date".to_string(),
    };
    let mcp_error: McpError = invalid_date.into();
    assert!(
        matches!(mcp_error, McpError::ValidationError { message } if message.contains("Invalid date format: invalid-date"))
    );

    let task_not_found = things3_core::ThingsError::TaskNotFound {
        uuid: "task-uuid".to_string(),
    };
    let mcp_error: McpError = task_not_found.into();
    assert!(
        matches!(mcp_error, McpError::ValidationError { message } if message.contains("Task not found: task-uuid"))
    );

    let project_not_found = things3_core::ThingsError::ProjectNotFound {
        uuid: "project-uuid".to_string(),
    };
    let mcp_error: McpError = project_not_found.into();
    assert!(
        matches!(mcp_error, McpError::ValidationError { message } if message.contains("Project not found: project-uuid"))
    );

    let area_not_found = things3_core::ThingsError::AreaNotFound {
        uuid: "area-uuid".to_string(),
    };
    let mcp_error: McpError = area_not_found.into();
    assert!(
        matches!(mcp_error, McpError::ValidationError { message } if message.contains("Area not found: area-uuid"))
    );

    let validation_error = things3_core::ThingsError::Validation {
        message: "test validation".to_string(),
    };
    let mcp_error: McpError = validation_error.into();
    assert!(
        matches!(mcp_error, McpError::ValidationError { message } if message == "test validation")
    );

    let config_error = things3_core::ThingsError::Configuration {
        message: "test config".to_string(),
    };
    let mcp_error: McpError = config_error.into();
    assert!(
        matches!(mcp_error, McpError::ConfigurationError { message } if message == "test config")
    );

    let unknown_error = things3_core::ThingsError::Unknown {
        message: "test unknown".to_string(),
    };
    let mcp_error: McpError = unknown_error.into();
    assert!(matches!(mcp_error, McpError::InternalError { message } if message == "test unknown"));
}

#[tokio::test]
async fn test_specific_error_types_in_tool_handlers() {
    let server = create_test_mcp_server().await;

    // Test missing parameter error
    let request = CallToolRequest {
        name: "search_tasks".to_string(),
        arguments: Some(json!({ "limit": 5 })), // Missing required 'query' parameter
    };

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::MissingParameter { parameter_name } => {
            assert_eq!(parameter_name, "query");
        }
        _ => panic!("Expected MissingParameter error"),
    }
}

#[tokio::test]
async fn test_invalid_format_error() {
    let server = create_test_mcp_server().await;

    // Test invalid format error
    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "xml", // Invalid format
            "data_type": "tasks"
        })),
    };

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::InvalidFormat { format, supported } => {
            assert_eq!(format, "xml");
            assert_eq!(supported, "json, csv, markdown");
        }
        _ => panic!("Expected InvalidFormat error"),
    }
}

#[tokio::test]
async fn test_invalid_data_type_error() {
    let server = create_test_mcp_server().await;

    // Test invalid data type error
    let request = CallToolRequest {
        name: "export_data".to_string(),
        arguments: Some(json!({
            "format": "json",
            "data_type": "invalid_type" // Invalid data type
        })),
    };

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::InvalidDataType {
            data_type,
            supported,
        } => {
            assert_eq!(data_type, "invalid_type");
            assert_eq!(supported, "tasks, projects, areas, all");
        }
        _ => panic!("Expected InvalidDataType error"),
    }
}

#[tokio::test]
async fn test_tool_not_found_error() {
    let server = create_test_mcp_server().await;

    // Test tool not found error
    let request = CallToolRequest {
        name: "nonexistent_tool".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ToolNotFound { tool_name } => {
            assert_eq!(tool_name, "nonexistent_tool");
        }
        _ => panic!("Expected ToolNotFound error"),
    }
}

#[tokio::test]
async fn test_prompt_not_found_error() {
    let server = create_test_mcp_server().await;

    // Test prompt not found error
    let request = things3_cli::mcp::GetPromptRequest {
        name: "nonexistent_prompt".to_string(),
        arguments: None,
    };

    let result = server.get_prompt(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::PromptNotFound { prompt_name } => {
            assert_eq!(prompt_name, "nonexistent_prompt");
        }
        _ => panic!("Expected PromptNotFound error"),
    }
}

#[tokio::test]
async fn test_resource_not_found_error() {
    let server = create_test_mcp_server().await;

    // Test resource not found error
    let request = things3_cli::mcp::ReadResourceRequest {
        uri: "things://nonexistent".to_string(),
    };

    let result = server.read_resource(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        McpError::ResourceNotFound { uri } => {
            assert_eq!(uri, "things://nonexistent");
        }
        _ => panic!("Expected ResourceNotFound error"),
    }
}

#[tokio::test]
async fn test_error_message_quality() {
    // Test that error messages are helpful and actionable
    let errors = vec![
        McpError::tool_not_found("test_tool"),
        McpError::missing_parameter("test_param"),
        McpError::invalid_format("xml", "json, csv"),
        McpError::invalid_data_type("xml", "tasks, projects"),
    ];

    for error in errors {
        let call_result = error.to_call_result();
        assert!(call_result.is_error);

        match &call_result.content[0] {
            Content::Text { text } => {
                // Error messages should be informative
                assert!(text.len() > 20);
                // Should contain helpful suggestions
                assert!(
                    text.contains("Please")
                        || text.contains("Available")
                        || text.contains("Supported")
                );
                // Should not be just generic error messages
                assert!(!text.contains("Error: Error"));
            }
        }
    }
}

#[tokio::test]
async fn test_error_consistency() {
    // Test that similar errors produce consistent messages
    let param_errors = vec![
        McpError::missing_parameter("param1"),
        McpError::missing_parameter("param2"),
    ];

    for error in param_errors {
        let call_result = error.to_call_result();
        match &call_result.content[0] {
            Content::Text { text } => {
                assert!(text.contains("Missing required parameter"));
                assert!(text.contains("Please provide this parameter"));
            }
        }
    }
}

#[tokio::test]
async fn test_error_serialization() {
    // Test that McpError can be serialized/deserialized for logging
    let error = McpError::tool_not_found("test_tool");
    let error_string = format!("{error:?}");
    assert!(error_string.contains("ToolNotFound"));
    assert!(error_string.contains("test_tool"));
}

#[tokio::test]
async fn test_mcp_error_helper_methods() {
    // Test all the helper methods for creating specific error types
    let tool_not_found = McpError::tool_not_found("test_tool");
    assert!(
        matches!(tool_not_found, McpError::ToolNotFound { tool_name } if tool_name == "test_tool")
    );

    let prompt_not_found = McpError::prompt_not_found("test_prompt");
    assert!(
        matches!(prompt_not_found, McpError::PromptNotFound { prompt_name } if prompt_name == "test_prompt")
    );

    let resource_not_found = McpError::resource_not_found("test_resource");
    assert!(
        matches!(resource_not_found, McpError::ResourceNotFound { uri } if uri == "test_resource")
    );

    let invalid_param = McpError::invalid_parameter("test_param", "invalid value");
    assert!(
        matches!(invalid_param, McpError::InvalidParameter { parameter_name, message }
        if parameter_name == "test_param" && message == "invalid value")
    );

    let missing_param = McpError::missing_parameter("test_param");
    assert!(
        matches!(missing_param, McpError::MissingParameter { parameter_name } if parameter_name == "test_param")
    );

    let invalid_format = McpError::invalid_format("xml", "json, csv");
    assert!(
        matches!(invalid_format, McpError::InvalidFormat { format, supported }
        if format == "xml" && supported == "json, csv")
    );

    let invalid_data_type = McpError::invalid_data_type("xml", "tasks, projects");
    assert!(
        matches!(invalid_data_type, McpError::InvalidDataType { data_type, supported }
        if data_type == "xml" && supported == "tasks, projects")
    );

    let db_error = McpError::database_operation_failed(
        "test_op",
        things3_core::ThingsError::validation("test error"),
    );
    assert!(
        matches!(db_error, McpError::DatabaseOperationFailed { operation, .. } if operation == "test_op")
    );

    let backup_error = McpError::backup_operation_failed(
        "test_backup",
        things3_core::ThingsError::validation("backup error"),
    );
    assert!(
        matches!(backup_error, McpError::BackupOperationFailed { operation, .. } if operation == "test_backup")
    );

    let export_error = McpError::export_operation_failed(
        "test_export",
        things3_core::ThingsError::validation("export error"),
    );
    assert!(
        matches!(export_error, McpError::ExportOperationFailed { operation, .. } if operation == "test_export")
    );

    let perf_error = McpError::performance_monitoring_failed(
        "test_perf",
        things3_core::ThingsError::validation("perf error"),
    );
    assert!(
        matches!(perf_error, McpError::PerformanceMonitoringFailed { operation, .. } if operation == "test_perf")
    );

    let cache_error = McpError::cache_operation_failed(
        "test_cache",
        things3_core::ThingsError::validation("cache error"),
    );
    assert!(
        matches!(cache_error, McpError::CacheOperationFailed { operation, .. } if operation == "test_cache")
    );

    let serialization_error = McpError::serialization_failed(
        "test_serialization",
        serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err(),
    );
    assert!(
        matches!(serialization_error, McpError::SerializationFailed { operation, .. } if operation == "test_serialization")
    );

    let io_error = McpError::io_operation_failed(
        "test_io",
        std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    );
    assert!(
        matches!(io_error, McpError::IoOperationFailed { operation, .. } if operation == "test_io")
    );

    let config_error = McpError::configuration_error("test config error");
    assert!(
        matches!(config_error, McpError::ConfigurationError { message } if message == "test config error")
    );

    let validation_error = McpError::validation_error("test validation error");
    assert!(
        matches!(validation_error, McpError::ValidationError { message } if message == "test validation error")
    );

    let internal_error = McpError::internal_error("test internal error");
    assert!(
        matches!(internal_error, McpError::InternalError { message } if message == "test internal error")
    );
}

#[tokio::test]
async fn test_error_conversion_methods_comprehensive() {
    // Test to_call_result with all error types
    let tool_error = McpError::tool_not_found("test_tool");
    let call_result = tool_error.to_call_result();
    assert!(call_result.is_error);
    assert_eq!(call_result.content.len(), 1);
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Tool 'test_tool' not found"));
        }
    }

    let resource_error = McpError::resource_not_found("test_resource");
    let call_result = resource_error.to_call_result();
    assert!(call_result.is_error);
    assert_eq!(call_result.content.len(), 1);
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Resource 'test_resource' not found"));
        }
    }

    let prompt_error = McpError::prompt_not_found("test_prompt");
    let call_result = prompt_error.to_call_result();
    assert!(call_result.is_error);
    assert_eq!(call_result.content.len(), 1);
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Prompt 'test_prompt' not found"));
        }
    }

    // Test to_prompt_result
    let prompt_error = McpError::prompt_not_found("test_prompt");
    let prompt_result = prompt_error.to_prompt_result();
    assert!(prompt_result.is_error);
    assert_eq!(prompt_result.content.len(), 1);
    match &prompt_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Prompt 'test_prompt' not found"));
        }
    }

    // Test to_resource_result
    let resource_error = McpError::resource_not_found("test_resource");
    let resource_result = resource_error.to_resource_result();
    assert_eq!(resource_result.contents.len(), 1);
    match &resource_result.contents[0] {
        Content::Text { text } => {
            assert!(text.contains("Resource 'test_resource' not found"));
        }
    }
}

#[tokio::test]
async fn test_error_message_formatting() {
    // Test that error messages are properly formatted with context
    let invalid_param = McpError::invalid_parameter("test_param", "invalid value");
    let call_result = invalid_param.to_call_result();
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Invalid parameter 'test_param'"));
            assert!(text.contains("invalid value"));
            assert!(text.contains("Please check the parameter format"));
        }
    }

    let missing_param = McpError::missing_parameter("test_param");
    let call_result = missing_param.to_call_result();
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter 'test_param'"));
            assert!(text.contains("Please provide this parameter"));
        }
    }

    let invalid_format = McpError::invalid_format("xml", "json, csv");
    let call_result = invalid_format.to_call_result();
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Invalid format 'xml'"));
            assert!(text.contains("Supported formats: json, csv"));
            assert!(text.contains("Please use one of the supported formats"));
        }
    }

    let db_error = McpError::database_operation_failed(
        "test_op",
        things3_core::ThingsError::validation("test error"),
    );
    let call_result = db_error.to_call_result();
    match &call_result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Database operation 'test_op' failed"));
            assert!(text.contains("Please check your database connection"));
        }
    }
}

#[tokio::test]
async fn test_error_display() {
    // Test that McpError implements Display trait properly
    let error = McpError::missing_parameter("test_param");
    let error_string = error.to_string();
    assert!(error_string.contains("Missing required parameter"));
    assert!(error_string.contains("test_param"));
}

#[tokio::test]
async fn test_error_chain() {
    // Test error chaining and source information
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let mcp_error: McpError = io_error.into();

    match mcp_error {
        McpError::IoOperationFailed { operation, source } => {
            assert_eq!(operation, "file operation");
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        _ => panic!("Expected IoOperationFailed error"),
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_all_error_variants_to_call_result() {
    // Test all error variants in to_call_result method
    let tool_error = McpError::tool_not_found("test_tool");
    let result = tool_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Tool 'test_tool' not found"));
            assert!(text.contains("list_tools method"));
        }
    }

    let resource_error = McpError::resource_not_found("test_resource");
    let result = resource_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Resource 'test_resource' not found"));
            assert!(text.contains("list_resources method"));
        }
    }

    let prompt_error = McpError::prompt_not_found("test_prompt");
    let result = prompt_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Prompt 'test_prompt' not found"));
            assert!(text.contains("list_prompts method"));
        }
    }

    let invalid_data_type = McpError::invalid_data_type("xml", "json, csv");
    let result = invalid_data_type.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Invalid data type 'xml'"));
            assert!(text.contains("Supported types: json, csv"));
            assert!(text.contains("Please use one of the supported types"));
        }
    }

    let backup_error = McpError::backup_operation_failed(
        "test_backup",
        things3_core::ThingsError::validation("backup error"),
    );
    let result = backup_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Backup operation 'test_backup' failed"));
            assert!(text.contains("Please check backup permissions"));
        }
    }

    let export_error = McpError::export_operation_failed(
        "test_export",
        things3_core::ThingsError::validation("export error"),
    );
    let result = export_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Export operation 'test_export' failed"));
            assert!(text.contains("Please check export parameters"));
        }
    }

    let perf_error = McpError::performance_monitoring_failed(
        "test_perf",
        things3_core::ThingsError::validation("perf error"),
    );
    let result = perf_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Performance monitoring 'test_perf' failed"));
            assert!(text.contains("Please try again later"));
        }
    }

    let cache_error = McpError::cache_operation_failed(
        "test_cache",
        things3_core::ThingsError::validation("cache error"),
    );
    let result = cache_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Cache operation 'test_cache' failed"));
            assert!(text.contains("Please try again later"));
        }
    }

    let serialization_error = McpError::serialization_failed(
        "test_serialization",
        serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err(),
    );
    let result = serialization_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Serialization 'test_serialization' failed"));
            assert!(text.contains("Please check data format"));
        }
    }

    let io_error = McpError::io_operation_failed(
        "test_io",
        std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    );
    let result = io_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("IO operation 'test_io' failed"));
            assert!(text.contains("Please check file permissions"));
        }
    }

    let config_error = McpError::configuration_error("test config error");
    let result = config_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Configuration error: test config error"));
            assert!(text.contains("Please check your configuration"));
        }
    }

    let validation_error = McpError::validation_error("test validation error");
    let result = validation_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Validation error: test validation error"));
            assert!(text.contains("Please check your input"));
        }
    }

    let internal_error = McpError::internal_error("test internal error");
    let result = internal_error.to_call_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Internal error: test internal error"));
            assert!(text.contains("Please try again later or contact support"));
        }
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_all_error_variants_to_prompt_result() {
    // Test all error variants in to_prompt_result method
    let prompt_error = McpError::prompt_not_found("test_prompt");
    let result = prompt_error.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Prompt 'test_prompt' not found"));
            assert!(text.contains("list_prompts method"));
        }
    }

    let invalid_param = McpError::invalid_parameter("test_param", "invalid value");
    let result = invalid_param.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Invalid parameter 'test_param'"));
            assert!(text.contains("invalid value"));
            assert!(text.contains("Please check the parameter format"));
        }
    }

    let missing_param = McpError::missing_parameter("test_param");
    let result = missing_param.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Missing required parameter 'test_param'"));
            assert!(text.contains("Please provide this parameter"));
        }
    }

    let invalid_format = McpError::invalid_format("xml", "json, csv");
    let result = invalid_format.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            // to_prompt_result uses catch-all pattern for InvalidFormat
            assert!(text.contains("Error: Invalid format: xml - supported formats: json, csv"));
            assert!(text.contains("Please try again later"));
        }
    }

    let invalid_data_type = McpError::invalid_data_type("xml", "json, csv");
    let result = invalid_data_type.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            // to_prompt_result uses catch-all pattern for InvalidDataType
            assert!(text.contains("Error: Invalid data type: xml - supported types: json, csv"));
            assert!(text.contains("Please try again later"));
        }
    }

    let db_error = McpError::database_operation_failed(
        "test_op",
        things3_core::ThingsError::validation("test error"),
    );
    let result = db_error.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Database operation 'test_op' failed"));
            assert!(text.contains("Please check your database connection"));
        }
    }

    let serialization_error = McpError::serialization_failed(
        "test_serialization",
        serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err(),
    );
    let result = serialization_error.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Serialization 'test_serialization' failed"));
            assert!(text.contains("Please check data format"));
        }
    }

    let io_error = McpError::io_operation_failed(
        "test_io",
        std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    );
    let result = io_error.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            // to_prompt_result uses catch-all pattern for IoOperationFailed
            assert!(text.contains("Error: IO operation failed: test_io"));
            assert!(text.contains("Please try again later"));
        }
    }

    let config_error = McpError::configuration_error("test config error");
    let result = config_error.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            // to_prompt_result uses catch-all pattern for ConfigurationError
            assert!(text.contains("Error: Configuration error: test config error"));
            assert!(text.contains("Please try again later"));
        }
    }

    let validation_error = McpError::validation_error("test validation error");
    let result = validation_error.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Validation error: test validation error"));
            assert!(text.contains("Please check your input"));
        }
    }

    let internal_error = McpError::internal_error("test internal error");
    let result = internal_error.to_prompt_result();
    assert!(result.is_error);
    match &result.content[0] {
        Content::Text { text } => {
            assert!(text.contains("Internal error: test internal error"));
            assert!(text.contains("Please try again later or contact support"));
        }
    }
}

#[tokio::test]
async fn test_all_error_variants_to_resource_result() {
    // Test all error variants in to_resource_result method
    let resource_error = McpError::resource_not_found("test_resource");
    let result = resource_error.to_resource_result();
    match &result.contents[0] {
        Content::Text { text } => {
            assert!(text.contains("Resource 'test_resource' not found"));
            assert!(text.contains("list_resources method"));
        }
    }

    let db_error = McpError::database_operation_failed(
        "test_op",
        things3_core::ThingsError::validation("test error"),
    );
    let result = db_error.to_resource_result();
    match &result.contents[0] {
        Content::Text { text } => {
            assert!(text.contains("Database operation 'test_op' failed"));
            assert!(text.contains("Please check your database connection"));
        }
    }

    let serialization_error = McpError::serialization_failed(
        "test_serialization",
        serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err(),
    );
    let result = serialization_error.to_resource_result();
    match &result.contents[0] {
        Content::Text { text } => {
            assert!(text.contains("Serialization 'test_serialization' failed"));
            assert!(text.contains("Please check data format"));
        }
    }

    let io_error = McpError::io_operation_failed(
        "test_io",
        std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    );
    let result = io_error.to_resource_result();
    match &result.contents[0] {
        Content::Text { text } => {
            // to_resource_result uses catch-all pattern for IoOperationFailed
            assert!(text.contains("Error: IO operation failed: test_io"));
            assert!(text.contains("Please try again later"));
        }
    }

    let config_error = McpError::configuration_error("test config error");
    let result = config_error.to_resource_result();
    match &result.contents[0] {
        Content::Text { text } => {
            // to_resource_result uses catch-all pattern for ConfigurationError
            assert!(text.contains("Error: Configuration error: test config error"));
            assert!(text.contains("Please try again later"));
        }
    }

    let validation_error = McpError::validation_error("test validation error");
    let result = validation_error.to_resource_result();
    match &result.contents[0] {
        Content::Text { text } => {
            // to_resource_result uses catch-all pattern for ValidationError
            assert!(text.contains("Error: Validation error: test validation error"));
            assert!(text.contains("Please try again later"));
        }
    }

    let internal_error = McpError::internal_error("test internal error");
    let result = internal_error.to_resource_result();
    match &result.contents[0] {
        Content::Text { text } => {
            assert!(text.contains("Internal error: test internal error"));
            assert!(text.contains("Please try again later or contact support"));
        }
    }
}
