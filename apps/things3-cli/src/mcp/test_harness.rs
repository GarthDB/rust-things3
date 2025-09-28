//! MCP-specific testing framework and utilities

use crate::mcp::{
    CallToolRequest, CallToolResult, Content, GetPromptRequest, GetPromptResult, McpError,
    ReadResourceRequest, ReadResourceResult, ThingsMcpServer,
};
use serde_json::Value;
use std::path::Path;
use tempfile::NamedTempFile;
use things3_core::{config::ThingsConfig, database::ThingsDatabase};

/// Test harness for MCP server testing
pub struct McpTestHarness {
    server: ThingsMcpServer,
    temp_file: NamedTempFile,
}

impl McpTestHarness {
    /// Create a new test harness with a fresh database
    pub fn new() -> Self {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        Self::create_test_database(db_path);

        let db = ThingsDatabase::new(db_path).unwrap();
        let config = ThingsConfig::new(db_path, false);
        let server = ThingsMcpServer::new(db, config);

        Self { server, temp_file }
    }

    /// Create a test harness with custom middleware configuration
    pub fn with_middleware_config(
        middleware_config: crate::mcp::middleware::MiddlewareConfig,
    ) -> Self {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        Self::create_test_database(db_path);

        let db = ThingsDatabase::new(db_path).unwrap();
        let config = ThingsConfig::new(db_path, false);
        let server = ThingsMcpServer::with_middleware_config(db, config, middleware_config);

        Self { server, temp_file }
    }

    /// Get a reference to the MCP server
    pub fn server(&self) -> &ThingsMcpServer {
        &self.server
    }

    /// Get the database path for additional testing
    pub fn db_path(&self) -> &Path {
        self.temp_file.path()
    }

    /// Call a tool and return the result
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<CallToolResult, McpError> {
        let request = CallToolRequest {
            name: name.to_string(),
            arguments,
        };
        self.server.call_tool(request).await
    }

    /// Call a tool with fallback error handling
    pub async fn call_tool_with_fallback(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> CallToolResult {
        let request = CallToolRequest {
            name: name.to_string(),
            arguments,
        };
        self.server.call_tool_with_fallback(request).await
    }

    /// Read a resource
    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, McpError> {
        let request = ReadResourceRequest {
            uri: uri.to_string(),
        };
        self.server.read_resource(request).await
    }

    /// Read a resource with fallback error handling
    pub async fn read_resource_with_fallback(&self, uri: &str) -> ReadResourceResult {
        let request = ReadResourceRequest {
            uri: uri.to_string(),
        };
        self.server.read_resource_with_fallback(request).await
    }

    /// Get a prompt
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<GetPromptResult, McpError> {
        let request = GetPromptRequest {
            name: name.to_string(),
            arguments,
        };
        self.server.get_prompt(request).await
    }

    /// Get a prompt with fallback error handling
    pub async fn get_prompt_with_fallback(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> GetPromptResult {
        let request = GetPromptRequest {
            name: name.to_string(),
            arguments,
        };
        self.server.get_prompt_with_fallback(request).await
    }

    /// Assert that a tool call succeeds
    pub async fn assert_tool_success(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> CallToolResult {
        let result = self.call_tool(name, arguments).await;
        assert!(result.is_ok(), "Tool call '{}' should succeed", name);
        let result = result.unwrap();
        assert!(
            !result.is_error,
            "Tool call '{}' should not return an error",
            name
        );
        result
    }

    /// Assert that a tool call fails with a specific error
    pub async fn assert_tool_error(
        &self,
        name: &str,
        arguments: Option<Value>,
        expected_error: fn(&McpError) -> bool,
    ) {
        let result = self.call_tool(name, arguments).await;
        assert!(result.is_err(), "Tool call '{}' should fail", name);
        let error = result.unwrap_err();
        assert!(
            expected_error(&error),
            "Tool call '{}' should fail with expected error: {:?}",
            name,
            error
        );
    }

    /// Assert that a resource read succeeds
    pub async fn assert_resource_success(&self, uri: &str) -> ReadResourceResult {
        let result = self.read_resource(uri).await;
        assert!(result.is_ok(), "Resource read '{}' should succeed", uri);
        result.unwrap()
    }

    /// Assert that a resource read fails with a specific error
    pub async fn assert_resource_error(&self, uri: &str, expected_error: fn(&McpError) -> bool) {
        let result = self.read_resource(uri).await;
        assert!(result.is_err(), "Resource read '{}' should fail", uri);
        let error = result.unwrap_err();
        assert!(
            expected_error(&error),
            "Resource read '{}' should fail with expected error: {:?}",
            uri,
            error
        );
    }

    /// Assert that a prompt succeeds
    pub async fn assert_prompt_success(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> GetPromptResult {
        let result = self.get_prompt(name, arguments).await;
        assert!(result.is_ok(), "Prompt '{}' should succeed", name);
        let result = result.unwrap();
        assert!(
            !result.is_error,
            "Prompt '{}' should not return an error",
            name
        );
        result
    }

    /// Assert that a prompt fails with a specific error
    pub async fn assert_prompt_error(
        &self,
        name: &str,
        arguments: Option<Value>,
        expected_error: fn(&McpError) -> bool,
    ) {
        let result = self.get_prompt(name, arguments).await;
        assert!(result.is_err(), "Prompt '{}' should fail", name);
        let error = result.unwrap_err();
        assert!(
            expected_error(&error),
            "Prompt '{}' should fail with expected error: {:?}",
            name,
            error
        );
    }

    /// Assert that a tool call returns valid JSON
    pub async fn assert_tool_returns_json(&self, name: &str, arguments: Option<Value>) -> Value {
        let result = self.assert_tool_success(name, arguments).await;
        assert_eq!(
            result.content.len(),
            1,
            "Tool call should return exactly one content item"
        );

        match &result.content[0] {
            Content::Text { text } => {
                serde_json::from_str(text).expect("Tool call should return valid JSON")
            }
        }
    }

    /// Assert that a resource read returns valid JSON
    pub async fn assert_resource_returns_json(&self, uri: &str) -> Value {
        let result = self.assert_resource_success(uri).await;
        assert_eq!(
            result.contents.len(),
            1,
            "Resource read should return exactly one content item"
        );

        match &result.contents[0] {
            Content::Text { text } => {
                serde_json::from_str(text).expect("Resource read should return valid JSON")
            }
        }
    }

    /// Assert that a prompt returns valid text
    pub async fn assert_prompt_returns_text(&self, name: &str, arguments: Option<Value>) -> String {
        let result = self.assert_prompt_success(name, arguments).await;
        assert_eq!(
            result.content.len(),
            1,
            "Prompt should return exactly one content item"
        );

        match &result.content[0] {
            Content::Text { text } => text.clone(),
        }
    }

    /// Create a comprehensive test database with mock data
    fn create_test_database<P: AsRef<Path>>(db_path: P) -> rusqlite::Connection {
        let conn = rusqlite::Connection::open(db_path).unwrap();

        // Create the Things 3 schema
        conn.execute_batch(
            r#"
            -- TMTask table (main tasks table)
            CREATE TABLE IF NOT EXISTS TMTask (
                uuid TEXT PRIMARY KEY,
                title TEXT,
                type INTEGER,
                status INTEGER,
                notes TEXT,
                startDate INTEGER,
                deadline INTEGER,
                creationDate REAL,
                userModificationDate REAL,
                project TEXT,
                area TEXT,
                heading TEXT
            );

            -- TMArea table (areas)
            CREATE TABLE IF NOT EXISTS TMArea (
                uuid TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                visible INTEGER,
                "index" INTEGER NOT NULL DEFAULT 0
            );

            -- TMTag table (tags)
            CREATE TABLE IF NOT EXISTS TMTag (
                uuid TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created TEXT NOT NULL,
                modified TEXT NOT NULL,
                "index" INTEGER NOT NULL DEFAULT 0
            );

            -- TMTaskTag table (task-tag relationships)
            CREATE TABLE IF NOT EXISTS TMTaskTag (
                task_uuid TEXT NOT NULL,
                tag_uuid TEXT NOT NULL,
                PRIMARY KEY (task_uuid, tag_uuid),
                FOREIGN KEY (task_uuid) REFERENCES TMTask(uuid),
                FOREIGN KEY (tag_uuid) REFERENCES TMTag(uuid)
            );
            "#,
        )
        .unwrap();

        let now = chrono::Utc::now();

        // Insert areas
        let areas = vec![
            ("area-1", "Work", 1, 0),
            ("area-2", "Personal", 1, 1),
            ("area-3", "Health", 1, 2),
        ];

        for (uuid, title, visible, index) in areas {
            conn.execute(
                "INSERT INTO TMArea (uuid, title, visible, \"index\") VALUES (?, ?, ?, ?)",
                (uuid, title, visible, index),
            )
            .unwrap();
        }

        // Insert tasks
        let tasks = vec![
            // Inbox tasks
            (
                "task-1",
                "Review quarterly reports",
                0,
                0,
                "Need to review Q3 reports",
                None,
                Some(1),
                None::<&str>,
                None::<&str>,
                None::<&str>,
            ),
            (
                "task-2",
                "Call dentist",
                0,
                0,
                "Schedule annual checkup",
                None,
                None,
                None::<&str>,
                None::<&str>,
                None::<&str>,
            ),
            (
                "task-3",
                "Buy groceries",
                0,
                0,
                "Milk, bread, eggs",
                None,
                None,
                None::<&str>,
                None::<&str>,
                None::<&str>,
            ),
            // Today's tasks
            (
                "task-4",
                "Team meeting",
                0,
                0,
                "Weekly standup",
                Some(0),
                None,
                None::<&str>,
                None::<&str>,
                None::<&str>,
            ),
            (
                "task-5",
                "Code review",
                0,
                0,
                "Review PR #123",
                Some(0),
                None,
                None::<&str>,
                None::<&str>,
                None::<&str>,
            ),
            // Project tasks
            (
                "task-6",
                "Design new feature",
                0,
                0,
                "Create wireframes",
                None,
                None,
                Some("project-1"),
                Some("area-1"),
                None::<&str>,
            ),
            (
                "task-7",
                "Write documentation",
                0,
                0,
                "API documentation",
                None,
                None,
                Some("project-1"),
                Some("area-1"),
                None::<&str>,
            ),
        ];

        for (
            uuid,
            title,
            task_type,
            status,
            notes,
            start_days,
            deadline_days,
            project,
            area,
            heading,
        ) in tasks
        {
            let start_date = start_days.map(|d: i64| {
                let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
                #[allow(clippy::cast_sign_loss)]
                { base_date.checked_add_days(chrono::Days::new(d as u64)) }.map(|d| {
                    d.signed_duration_since(chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap())
                        .num_days()
                })
            });

            let deadline = deadline_days.map(|d: i64| {
                let base_date = chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
                #[allow(clippy::cast_sign_loss)]
                { base_date.checked_add_days(chrono::Days::new(d as u64)) }.map(|d| {
                    d.signed_duration_since(chrono::NaiveDate::from_ymd_opt(2001, 1, 1).unwrap())
                        .num_days()
                })
            });

            conn.execute(
                "INSERT INTO TMTask (uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, project, area, heading) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (uuid, title, task_type, status, notes, start_date, deadline,
                    #[allow(clippy::cast_precision_loss)]
                    {
                        now.timestamp() as f64
                    },
                    #[allow(clippy::cast_precision_loss)]
                    {
                        now.timestamp() as f64
                    },
                    project.map(std::string::ToString::to_string),
                    area.map(std::string::ToString::to_string),
                    heading),
            ).unwrap();
        }

        conn
    }
}

impl Default for McpTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock database for testing without real database dependencies
pub struct MockDatabase {
    pub tasks: Vec<MockTask>,
    pub projects: Vec<MockProject>,
    pub areas: Vec<MockArea>,
}

#[derive(Debug, Clone)]
pub struct MockTask {
    pub uuid: String,
    pub title: String,
    pub notes: Option<String>,
    pub status: String,
    pub project_uuid: Option<String>,
    pub area_uuid: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MockProject {
    pub uuid: String,
    pub title: String,
    pub area_uuid: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct MockArea {
    pub uuid: String,
    pub title: String,
    pub visible: bool,
}

impl MockDatabase {
    pub fn new() -> Self {
        Self {
            tasks: vec![
                MockTask {
                    uuid: "task-1".to_string(),
                    title: "Test Task 1".to_string(),
                    notes: Some("Test notes".to_string()),
                    status: "incomplete".to_string(),
                    project_uuid: None,
                    area_uuid: None,
                },
                MockTask {
                    uuid: "task-2".to_string(),
                    title: "Test Task 2".to_string(),
                    notes: None,
                    status: "completed".to_string(),
                    project_uuid: Some("project-1".to_string()),
                    area_uuid: Some("area-1".to_string()),
                },
            ],
            projects: vec![MockProject {
                uuid: "project-1".to_string(),
                title: "Test Project".to_string(),
                area_uuid: Some("area-1".to_string()),
                status: "incomplete".to_string(),
            }],
            areas: vec![
                MockArea {
                    uuid: "area-1".to_string(),
                    title: "Work".to_string(),
                    visible: true,
                },
                MockArea {
                    uuid: "area-2".to_string(),
                    title: "Personal".to_string(),
                    visible: true,
                },
            ],
        }
    }

    pub fn add_task(&mut self, task: MockTask) {
        self.tasks.push(task);
    }

    pub fn add_project(&mut self, project: MockProject) {
        self.projects.push(project);
    }

    pub fn add_area(&mut self, area: MockArea) {
        self.areas.push(area);
    }

    pub fn get_task(&self, uuid: &str) -> Option<&MockTask> {
        self.tasks.iter().find(|t| t.uuid == uuid)
    }

    pub fn get_project(&self, uuid: &str) -> Option<&MockProject> {
        self.projects.iter().find(|p| p.uuid == uuid)
    }

    pub fn get_area(&self, uuid: &str) -> Option<&MockArea> {
        self.areas.iter().find(|a| a.uuid == uuid)
    }

    pub fn get_tasks_by_status(&self, status: &str) -> Vec<&MockTask> {
        self.tasks.iter().filter(|t| t.status == status).collect()
    }

    pub fn get_tasks_by_project(&self, project_uuid: &str) -> Vec<&MockTask> {
        self.tasks
            .iter()
            .filter(|t| t.project_uuid.as_ref() == Some(&project_uuid.to_string()))
            .collect()
    }

    pub fn get_tasks_by_area(&self, area_uuid: &str) -> Vec<&MockTask> {
        self.tasks
            .iter()
            .filter(|t| t.area_uuid.as_ref() == Some(&area_uuid.to_string()))
            .collect()
    }
}

impl Default for MockDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// Test utilities for common MCP operations
pub struct McpTestUtils;

impl McpTestUtils {
    /// Create a test tool request
    pub fn create_tool_request(name: &str, arguments: Option<Value>) -> CallToolRequest {
        CallToolRequest {
            name: name.to_string(),
            arguments,
        }
    }

    /// Create a test resource request
    pub fn create_resource_request(uri: &str) -> ReadResourceRequest {
        ReadResourceRequest {
            uri: uri.to_string(),
        }
    }

    /// Create a test prompt request
    pub fn create_prompt_request(name: &str, arguments: Option<Value>) -> GetPromptRequest {
        GetPromptRequest {
            name: name.to_string(),
            arguments,
        }
    }

    /// Assert that a tool result contains expected content
    pub fn assert_tool_result_contains(result: &CallToolResult, expected_text: &str) {
        assert!(!result.is_error, "Tool result should not be an error");
        assert!(
            !result.content.is_empty(),
            "Tool result should have content"
        );

        match &result.content[0] {
            Content::Text { text } => {
                assert!(
                    text.contains(expected_text),
                    "Tool result should contain '{}', but got: {}",
                    expected_text,
                    text
                );
            }
        }
    }

    /// Assert that a resource result contains expected content
    pub fn assert_resource_result_contains(result: &ReadResourceResult, expected_text: &str) {
        assert!(
            !result.contents.is_empty(),
            "Resource result should have content"
        );

        match &result.contents[0] {
            Content::Text { text } => {
                assert!(
                    text.contains(expected_text),
                    "Resource result should contain '{}', but got: {}",
                    expected_text,
                    text
                );
            }
        }
    }

    /// Assert that a prompt result contains expected content
    pub fn assert_prompt_result_contains(result: &GetPromptResult, expected_text: &str) {
        assert!(!result.is_error, "Prompt result should not be an error");
        assert!(
            !result.content.is_empty(),
            "Prompt result should have content"
        );

        match &result.content[0] {
            Content::Text { text } => {
                assert!(
                    text.contains(expected_text),
                    "Prompt result should contain '{}', but got: {}",
                    expected_text,
                    text
                );
            }
        }
    }

    /// Assert that a tool result is valid JSON
    pub fn assert_tool_result_is_json(result: &CallToolResult) -> Value {
        assert!(!result.is_error, "Tool result should not be an error");
        assert!(
            !result.content.is_empty(),
            "Tool result should have content"
        );

        match &result.content[0] {
            Content::Text { text } => {
                serde_json::from_str(text).expect("Tool result should be valid JSON")
            }
        }
    }

    /// Assert that a resource result is valid JSON
    pub fn assert_resource_result_is_json(result: &ReadResourceResult) -> Value {
        assert!(
            !result.contents.is_empty(),
            "Resource result should have content"
        );

        match &result.contents[0] {
            Content::Text { text } => {
                serde_json::from_str(text).expect("Resource result should be valid JSON")
            }
        }
    }

    /// Create test data for various scenarios
    pub fn create_test_data() -> MockDatabase {
        MockDatabase::new()
    }

    /// Create test data with specific scenarios
    pub fn create_test_data_with_scenarios() -> MockDatabase {
        let mut db = MockDatabase::new();

        // Add more test data for different scenarios
        db.add_task(MockTask {
            uuid: "task-urgent".to_string(),
            title: "Urgent Task".to_string(),
            notes: Some("This is urgent".to_string()),
            status: "incomplete".to_string(),
            project_uuid: None,
            area_uuid: None,
        });

        db.add_task(MockTask {
            uuid: "task-completed".to_string(),
            title: "Completed Task".to_string(),
            notes: None,
            status: "completed".to_string(),
            project_uuid: Some("project-1".to_string()),
            area_uuid: Some("area-1".to_string()),
        });

        db.add_project(MockProject {
            uuid: "project-2".to_string(),
            title: "Another Project".to_string(),
            area_uuid: Some("area-2".to_string()),
            status: "incomplete".to_string(),
        });

        db.add_area(MockArea {
            uuid: "area-3".to_string(),
            title: "Health".to_string(),
            visible: true,
        });

        db
    }
}

/// Performance testing utilities for MCP operations
pub struct McpPerformanceTest {
    start_time: std::time::Instant,
}

impl McpPerformanceTest {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    pub fn assert_under_threshold(&self, threshold: std::time::Duration) {
        let elapsed = self.elapsed();
        assert!(
            elapsed < threshold,
            "Operation took {:?}, which exceeds threshold of {:?}",
            elapsed,
            threshold
        );
    }

    pub fn assert_under_ms(&self, threshold_ms: u64) {
        self.assert_under_threshold(std::time::Duration::from_millis(threshold_ms));
    }
}

impl Default for McpPerformanceTest {
    fn default() -> Self {
        Self::new()
    }
}

/// Integration test utilities for full MCP workflows
pub struct McpIntegrationTest {
    harness: McpTestHarness,
}

impl McpIntegrationTest {
    pub fn new() -> Self {
        Self {
            harness: McpTestHarness::new(),
        }
    }

    pub fn with_middleware_config(
        middleware_config: crate::mcp::middleware::MiddlewareConfig,
    ) -> Self {
        Self {
            harness: McpTestHarness::with_middleware_config(middleware_config),
        }
    }

    pub fn harness(&self) -> &McpTestHarness {
        &self.harness
    }

    /// Test a complete workflow: list tools -> call tool -> verify result
    pub async fn test_tool_workflow(
        &self,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> CallToolResult {
        // First, verify the tool exists
        let tools_result = self.harness.server().list_tools().unwrap();
        let tool_exists = tools_result.tools.iter().any(|t| t.name == tool_name);
        assert!(
            tool_exists,
            "Tool '{}' should exist in the tools list",
            tool_name
        );

        // Call the tool
        self.harness.call_tool(tool_name, arguments).await.unwrap()
    }

    /// Test a complete resource workflow: list resources -> read resource -> verify result
    pub async fn test_resource_workflow(&self, uri: &str) -> ReadResourceResult {
        // First, verify the resource exists
        let resources_result = self.harness.server().list_resources().unwrap();
        let resource_exists = resources_result.resources.iter().any(|r| r.uri == uri);
        assert!(
            resource_exists,
            "Resource '{}' should exist in the resources list",
            uri
        );

        // Read the resource
        self.harness.read_resource(uri).await.unwrap()
    }

    /// Test a complete prompt workflow: list prompts -> get prompt -> verify result
    pub async fn test_prompt_workflow(
        &self,
        prompt_name: &str,
        arguments: Option<Value>,
    ) -> GetPromptResult {
        // First, verify the prompt exists
        let prompts_result = self.harness.server().list_prompts().unwrap();
        let prompt_exists = prompts_result.prompts.iter().any(|p| p.name == prompt_name);
        assert!(
            prompt_exists,
            "Prompt '{}' should exist in the prompts list",
            prompt_name
        );

        // Get the prompt
        self.harness
            .get_prompt(prompt_name, arguments)
            .await
            .unwrap()
    }

    /// Test error handling workflow
    pub async fn test_error_handling_workflow(&self) {
        // Test unknown tool
        let result = self.harness.call_tool("unknown_tool", None).await;
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(matches!(error, McpError::ToolNotFound { .. }));
        }

        // Test unknown resource
        let result = self.harness.read_resource("things://unknown").await;
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(matches!(error, McpError::ResourceNotFound { .. }));
        }

        // Test unknown prompt
        let result = self.harness.get_prompt("unknown_prompt", None).await;
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(matches!(error, McpError::PromptNotFound { .. }));
        }
    }

    /// Test performance workflow
    pub async fn test_performance_workflow(&self) {
        let perf_test = McpPerformanceTest::new();

        // Test tool call performance
        let _result = self.harness.call_tool("get_inbox", None).await.unwrap();
        perf_test.assert_under_ms(1000); // Should complete within 1 second

        // Test resource read performance
        let perf_test = McpPerformanceTest::new();
        let _result = self.harness.read_resource("things://inbox").await.unwrap();
        perf_test.assert_under_ms(1000);

        // Test prompt performance
        let perf_test = McpPerformanceTest::new();
        let _result = self
            .harness
            .get_prompt(
                "task_review",
                Some(serde_json::json!({"task_title": "Test"})),
            )
            .await
            .unwrap();
        perf_test.assert_under_ms(1000);
    }
}

impl Default for McpIntegrationTest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_mcp_test_harness_creation() {
        let harness = McpTestHarness::new();
        assert!(!harness.server().list_tools().unwrap().tools.is_empty());
    }

    #[tokio::test]
    async fn test_mcp_test_harness_tool_calls() {
        let harness = McpTestHarness::new();

        // Test successful tool call
        let result = harness.assert_tool_success("get_inbox", None).await;
        assert!(!result.is_error);

        // Test tool call with arguments
        let result = harness
            .assert_tool_success("get_inbox", Some(json!({"limit": 5})))
            .await;
        assert!(!result.is_error);
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
    }

    #[tokio::test]
    async fn test_mcp_test_harness_json_assertions() {
        let harness = McpTestHarness::new();

        // Test JSON assertion
        let json_result = harness.assert_tool_returns_json("get_inbox", None).await;
        assert!(json_result.is_array());
    }

    #[tokio::test]
    async fn test_mock_database() {
        let mut db = MockDatabase::new();
        assert_eq!(db.tasks.len(), 2);
        assert_eq!(db.projects.len(), 1);
        assert_eq!(db.areas.len(), 2);

        // Test adding data
        db.add_task(MockTask {
            uuid: "new-task".to_string(),
            title: "New Task".to_string(),
            notes: None,
            status: "incomplete".to_string(),
            project_uuid: None,
            area_uuid: None,
        });
        assert_eq!(db.tasks.len(), 3);

        // Test querying data
        let task = db.get_task("task-1").unwrap();
        assert_eq!(task.title, "Test Task 1");

        let completed_tasks = db.get_tasks_by_status("completed");
        assert_eq!(completed_tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_mcp_test_utils() {
        let request =
            McpTestUtils::create_tool_request("test_tool", Some(json!({"param": "value"})));
        assert_eq!(request.name, "test_tool");
        assert!(request.arguments.is_some());

        let request = McpTestUtils::create_resource_request("things://test");
        assert_eq!(request.uri, "things://test");

        let request =
            McpTestUtils::create_prompt_request("test_prompt", Some(json!({"param": "value"})));
        assert_eq!(request.name, "test_prompt");
        assert!(request.arguments.is_some());
    }

    #[tokio::test]
    async fn test_mcp_performance_test() {
        let perf_test = McpPerformanceTest::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = perf_test.elapsed();
        assert!(elapsed.as_millis() >= 10);

        let perf_test = McpPerformanceTest::new();
        perf_test.assert_under_ms(1000); // Should pass
    }

    #[tokio::test]
    async fn test_mcp_integration_test() {
        let integration_test = McpIntegrationTest::new();

        // Test tool workflow
        let result = integration_test.test_tool_workflow("get_inbox", None).await;
        assert!(!result.is_error);

        // Test resource workflow
        let result = integration_test
            .test_resource_workflow("things://inbox")
            .await;
        assert!(!result.contents.is_empty());

        // Test prompt workflow
        let result = integration_test
            .test_prompt_workflow("task_review", Some(json!({"task_title": "Test"})))
            .await;
        assert!(!result.is_error);

        // Test error handling workflow
        integration_test.test_error_handling_workflow().await;

        // Test performance workflow
        integration_test.test_performance_workflow().await;
    }
}
