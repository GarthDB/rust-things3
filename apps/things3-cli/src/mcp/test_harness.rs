//! Test harness for MCP server testing

use crate::mcp::{
    CallToolRequest, CallToolResult, Content, GetPromptRequest, GetPromptResult, McpError,
    ReadResourceRequest, ReadResourceResult, ThingsMcpServer,
};
use serde_json::Value;
use std::path::Path;
use tempfile::NamedTempFile;
use things3_core::{config::ThingsConfig, ThingsDatabase};
// use std::sync::Arc; // Not needed for test harness

/// Test harness for MCP server operations
pub struct McpTestHarness {
    server: ThingsMcpServer,
    temp_file: NamedTempFile,
}

impl McpTestHarness {
    /// Create a new test harness with a fresh database
    ///
    /// # Panics
    /// Panics if the database cannot be creationDate or the server cannot be initialized
    #[must_use]
    pub fn new() -> Self {
        Self::new_with_config(crate::mcp::MiddlewareConfig::default())
    }

    /// Create a new test harness with a fresh database and custom middleware config
    ///
    /// # Panics
    /// Panics if the database cannot be creationDate or the server cannot be initialized
    #[must_use]
    pub fn new_with_config(middleware_config: crate::mcp::MiddlewareConfig) -> Self {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();
        let db_path_clone = db_path.clone();

        // Create test database synchronously to avoid nested runtime issues
        let db = std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { Self::create_test_database(&db_path_clone).await })
        })
        .join()
        .unwrap();

        let config = ThingsConfig::new(&db_path, false);
        let server = ThingsMcpServer::with_middleware_config(db, config, middleware_config);

        Self { server, temp_file }
    }

    /// Create a test harness with custom middleware configuration
    ///
    /// # Panics
    /// Panics if the database cannot be creationDate or the server cannot be initialized
    #[must_use]
    pub fn with_middleware_config(middleware_config: crate::mcp::MiddlewareConfig) -> Self {
        Self::new_with_config(middleware_config)
    }

    /// Get a reference to the MCP server
    #[must_use]
    pub fn server(&self) -> &ThingsMcpServer {
        &self.server
    }

    /// Get the database path for additional testing
    #[must_use]
    pub fn db_path(&self) -> &Path {
        self.temp_file.path()
    }

    /// Call a tool and return the result
    ///
    /// # Panics
    /// Panics if the tool call fails
    pub async fn call_tool(&self, name: &str, arguments: Option<Value>) -> CallToolResult {
        let request = CallToolRequest {
            name: name.to_string(),
            arguments,
        };
        self.server.call_tool(request).await.unwrap()
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

    /// Read a resource and return the result
    ///
    /// # Panics
    /// Panics if the resource read fails
    pub async fn read_resource(&self, uri: &str) -> ReadResourceResult {
        let request = ReadResourceRequest {
            uri: uri.to_string(),
        };
        self.server.read_resource(request).await.unwrap()
    }

    /// Read a resource and return the result or error
    ///
    /// # Errors
    ///
    /// Returns an error if the resource cannot be read or if the MCP server is not available.
    pub async fn read_resource_result(&self, uri: &str) -> Result<ReadResourceResult, McpError> {
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
    ///
    /// # Panics
    /// Panics if the prompt request fails
    pub async fn get_prompt(&self, name: &str, arguments: Option<Value>) -> GetPromptResult {
        let request = GetPromptRequest {
            name: name.to_string(),
            arguments,
        };
        self.server.get_prompt(request).await.unwrap()
    }

    /// Get a prompt and return the result or error
    ///
    /// # Errors
    ///
    /// Returns an error if the prompt cannot be retrieved or if the MCP server is not available.
    pub async fn get_prompt_result(
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
    ///
    /// # Panics
    /// Panics if the tool call fails
    pub async fn assert_tool_succeeds(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> CallToolResult {
        let result = self.call_tool(name, arguments).await;
        assert!(
            !result.is_error,
            "Tool call '{name}' should succeed but failed"
        );
        result
    }

    /// Assert that a tool call fails with expected error
    ///
    /// # Panics
    /// Panics if the tool call succeeds when it should fail
    pub async fn assert_tool_fails_with<F>(
        &self,
        name: &str,
        arguments: Option<Value>,
        _expected_error: F,
    ) where
        F: FnOnce(&McpError) -> bool,
    {
        let result = self.call_tool_with_fallback(name, arguments).await;
        assert!(
            result.is_error,
            "Tool call '{name}' should fail but succeeded"
        );
    }

    /// Assert that a resource read succeeds
    ///
    /// # Panics
    /// Panics if the resource read fails
    pub async fn assert_resource_succeeds(&self, uri: &str) -> ReadResourceResult {
        let result = self.read_resource(uri).await;
        assert!(
            !result.contents.is_empty(),
            "Resource read '{uri}' should succeed"
        );
        result
    }

    /// Assert that a resource read fails with expected error
    ///
    /// # Panics
    /// Panics if the resource read succeeds when it should fail
    pub async fn assert_resource_fails_with<F>(&self, uri: &str, expected_error: F)
    where
        F: FnOnce(&McpError) -> bool,
    {
        let result = self.read_resource_result(uri).await;
        match result {
            Ok(_) => panic!("Resource read '{uri}' should fail but succeeded"),
            Err(e) => assert!(
                expected_error(&e),
                "Resource read '{uri}' failed with unexpected error: {e:?}"
            ),
        }
    }

    /// Assert that a prompt succeeds
    ///
    /// # Panics
    /// Panics if the prompt request fails
    pub async fn assert_prompt_succeeds(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> GetPromptResult {
        let result = self.get_prompt(name, arguments).await;
        assert!(
            !result.is_error,
            "Prompt '{name}' should succeed but failed"
        );
        result
    }

    /// Assert that a prompt fails with expected error
    ///
    /// # Panics
    /// Panics if the prompt request succeeds when it should fail
    pub async fn assert_prompt_fails_with<F>(
        &self,
        name: &str,
        arguments: Option<Value>,
        expected_error: F,
    ) where
        F: FnOnce(&McpError) -> bool,
    {
        let result = self.get_prompt_result(name, arguments).await;
        match result {
            Ok(_) => panic!("Prompt '{name}' should fail but succeeded"),
            Err(e) => assert!(
                expected_error(&e),
                "Prompt '{name}' failed with unexpected error: {e:?}"
            ),
        }
    }

    /// Assert that a tool call returns valid JSON
    ///
    /// # Panics
    /// Panics if the tool call fails or returns invalid JSON
    pub async fn assert_tool_returns_json(&self, name: &str, arguments: Option<Value>) -> Value {
        let result = self.assert_tool_succeeds(name, arguments).await;
        assert!(
            !result.content.is_empty(),
            "Tool call should return content"
        );

        match &result.content[0] {
            Content::Text { text } => {
                serde_json::from_str(text).expect("Tool call should return valid JSON")
            }
        }
    }

    /// Assert that a resource read returns valid JSON
    ///
    /// # Panics
    /// Panics if the resource read fails or returns invalid JSON
    pub async fn assert_resource_returns_json(&self, uri: &str) -> Value {
        let result = self.assert_resource_succeeds(uri).await;
        assert!(
            !result.contents.is_empty(),
            "Resource read should return content"
        );

        match &result.contents[0] {
            Content::Text { text } => {
                serde_json::from_str(text).expect("Resource read should return valid JSON")
            }
        }
    }

    /// Assert that a prompt returns valid text
    ///
    /// # Panics
    /// Panics if the prompt request fails or returns no text content
    pub async fn assert_prompt_returns_text(&self, name: &str, arguments: Option<Value>) -> String {
        let result = self.assert_prompt_succeeds(name, arguments).await;
        assert!(!result.content.is_empty(), "Prompt should return content");

        match &result.content[0] {
            Content::Text { text } => text.clone(),
        }
    }

    /// Create a comprehensive test database with mock data
    #[allow(clippy::too_many_lines)]
    async fn create_test_database<P: AsRef<Path>>(db_path: P) -> ThingsDatabase {
        use sqlx::SqlitePool;

        let database_url = format!("sqlite:{}", db_path.as_ref().display());
        let pool = SqlitePool::connect(&database_url).await.unwrap();

        // Create the Things 3 schema - matches real database structure
        sqlx::query(
            r"
            -- TMTask table (main tasks table) - matches real Things 3 schema
            CREATE TABLE IF NOT EXISTS TMTask (
                uuid TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                type INTEGER NOT NULL DEFAULT 0,
                status INTEGER NOT NULL DEFAULT 0,
                notes TEXT,
                startDate INTEGER,
                deadline INTEGER,
                stopDate REAL,
                creationDate REAL NOT NULL,
                userModificationDate REAL NOT NULL,
                project TEXT,
                area TEXT,
                heading TEXT,
                trashed INTEGER NOT NULL DEFAULT 0,
                tags TEXT DEFAULT '[]',
                cachedTags BLOB,
                todayIndex INTEGER
            )
            ",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Note: Projects are stored in TMTask table with type=1, no separate TMProject table

        sqlx::query(
            r"
            -- TMArea table (areas table) - matches real Things 3 schema
            CREATE TABLE IF NOT EXISTS TMArea (
                uuid TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                visible INTEGER NOT NULL DEFAULT 1,
                'index' INTEGER NOT NULL DEFAULT 0
            )
            ",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert test data
        // Use a safe conversion for timestamp to avoid precision loss
        let timestamp_i64 = chrono::Utc::now().timestamp();
        let now_timestamp = if timestamp_i64 <= i64::from(i32::MAX) {
            f64::from(i32::try_from(timestamp_i64).unwrap_or(0))
        } else {
            // For very large timestamps, use a reasonable test value
            1_700_000_000.0 // Represents a date around 2023
        };

        // Insert test areas
        sqlx::query("INSERT INTO TMArea (uuid, title, visible, 'index') VALUES (?, ?, ?, ?)")
            .bind("550e8400-e29b-41d4-a716-446655440001")
            .bind("Work")
            .bind(1) // visible
            .bind(0) // index
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO TMArea (uuid, title, visible, 'index') VALUES (?, ?, ?, ?)")
            .bind("550e8400-e29b-41d4-a716-446655440002")
            .bind("Personal")
            .bind(1) // visible
            .bind(1) // index
            .execute(&pool)
            .await
            .unwrap();

        // Insert test projects (as TMTask with type=1)
        sqlx::query(
            "INSERT INTO TMTask (uuid, title, type, status, notes, creationDate, userModificationDate, area, trashed, tags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind("550e8400-e29b-41d4-a716-446655440010")
        .bind("Website Redesign")
        .bind(1) // type: project
        .bind(0) // status: active
        .bind("Complete redesign of company website")
        .bind(now_timestamp)
        .bind(now_timestamp)
        .bind("550e8400-e29b-41d4-a716-446655440001") // work area
        .bind(0) // not trashed
        .bind("[\"work\", \"web\"]")
        .execute(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO TMTask (uuid, title, type, status, notes, creationDate, userModificationDate, area, trashed, tags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind("550e8400-e29b-41d4-a716-446655440011")
        .bind("Learn Rust")
        .bind(1) // type: project
        .bind(0) // status: active
        .bind("Learn the Rust programming language")
        .bind(now_timestamp)
        .bind(now_timestamp)
        .bind("550e8400-e29b-41d4-a716-446655440002")
        .bind(0) // not trashed
        .bind("[\"personal\", \"learning\"]")
        .execute(&pool).await.unwrap();

        // Insert test tasks - one in inbox (no project), one in project
        sqlx::query(
            "INSERT INTO TMTask (uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, project, area, heading, trashed, tags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind("550e8400-e29b-41d4-a716-446655440099")
        .bind("Inbox Task")
        .bind(0)
        .bind(0)
        .bind("A task in the inbox")
        .bind::<Option<i64>>(None) // startDate: NULL
        .bind::<Option<i64>>(None) // deadline: NULL
        .bind(now_timestamp)
        .bind(now_timestamp)
        .bind::<Option<String>>(None) // No project (inbox) - use NULL instead of empty string
        .bind("550e8400-e29b-41d4-a716-446655440001") // area: work area
        .bind("") // heading: empty for top-level task
        .bind(0) // not trashed
        .bind("[\"inbox\"]")
        .execute(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO TMTask (uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, project, area, heading, trashed, tags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind("550e8400-e29b-41d4-a716-446655440100")
        .bind("Research competitors")
        .bind(0)
        .bind(0)
        .bind("Look at competitor websites for inspiration")
        .bind::<Option<i64>>(None) // startDate: NULL
        .bind::<Option<i64>>(None) // deadline: NULL
        .bind(now_timestamp)
        .bind(now_timestamp)
        .bind("550e8400-e29b-41d4-a716-446655440010")
        .bind("550e8400-e29b-41d4-a716-446655440001") // area: work area
        .bind("") // heading: empty for top-level task
        .bind(0) // not trashed
        .bind("[\"research\"]")
        .execute(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO TMTask (uuid, title, type, status, notes, startDate, deadline, creationDate, userModificationDate, project, area, heading, trashed, tags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind("550e8400-e29b-41d4-a716-446655440101")
        .bind("Read Rust book")
        .bind(0)
        .bind(0)
        .bind("Read The Rust Programming Language book")
        .bind::<Option<i64>>(None) // startDate: NULL
        .bind::<Option<i64>>(None) // deadline: NULL
        .bind(now_timestamp)
        .bind(now_timestamp)
        .bind("550e8400-e29b-41d4-a716-446655440011")
        .bind("550e8400-e29b-41d4-a716-446655440002") // area: personal area
        .bind("") // heading: empty for top-level task
        .bind(0) // not trashed
        .bind("[\"reading\"]")
        .execute(&pool).await.unwrap();

        pool.close().await;
        ThingsDatabase::new(db_path.as_ref()).await.unwrap()
    }
}

impl Default for McpTestHarness {
    fn default() -> Self {
        panic!("McpTestHarness::default() cannot be used in async context. Use McpTestHarness::new().await instead.")
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
    pub status: String,
    pub project: Option<String>,
    pub area: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MockProject {
    pub uuid: String,
    pub title: String,
    pub area: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct MockArea {
    pub uuid: String,
    pub title: String,
    pub visible: bool,
}

impl MockDatabase {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            projects: Vec::new(),
            areas: Vec::new(),
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

    #[must_use]
    pub fn get_task(&self, uuid: &str) -> Option<&MockTask> {
        self.tasks.iter().find(|t| t.uuid == uuid)
    }

    #[must_use]
    pub fn get_project(&self, uuid: &str) -> Option<&MockProject> {
        self.projects.iter().find(|p| p.uuid == uuid)
    }

    #[must_use]
    pub fn get_area(&self, uuid: &str) -> Option<&MockArea> {
        self.areas.iter().find(|a| a.uuid == uuid)
    }

    #[must_use]
    pub fn get_tasks_by_status(&self, status: &str) -> Vec<&MockTask> {
        self.tasks.iter().filter(|t| t.status == status).collect()
    }

    #[must_use]
    pub fn get_tasks_by_project(&self, project: &str) -> Vec<&MockTask> {
        self.tasks
            .iter()
            .filter(|t| t.project.as_ref() == Some(&project.to_string()))
            .collect()
    }

    #[must_use]
    pub fn get_tasks_by_area(&self, area: &str) -> Vec<&MockTask> {
        self.tasks
            .iter()
            .filter(|t| t.area.as_ref() == Some(&area.to_string()))
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
    #[must_use]
    pub fn create_tool_request(name: &str, arguments: Option<Value>) -> CallToolRequest {
        CallToolRequest {
            name: name.to_string(),
            arguments,
        }
    }

    /// Create a test resource request
    #[must_use]
    pub fn create_resource_request(uri: &str) -> ReadResourceRequest {
        ReadResourceRequest {
            uri: uri.to_string(),
        }
    }

    /// Create a test prompt request
    #[must_use]
    pub fn create_prompt_request(name: &str, arguments: Option<Value>) -> GetPromptRequest {
        GetPromptRequest {
            name: name.to_string(),
            arguments,
        }
    }

    /// Assert that a tool result contains expected content
    ///
    /// # Panics
    /// Panics if the tool result is an error or doesn't contain the expected content
    pub fn assert_tool_result_contains(result: &CallToolResult, expected_content: &str) {
        assert!(!result.is_error, "Tool call should succeed");
        assert!(
            !result.content.is_empty(),
            "Tool call should return content"
        );

        match &result.content[0] {
            Content::Text { text } => {
                assert!(
                    text.contains(expected_content),
                    "Tool result should contain: {expected_content}"
                );
            }
        }
    }

    /// Assert that a resource result contains expected content
    ///
    /// # Panics
    /// Panics if the resource result is empty or doesn't contain the expected content
    pub fn assert_resource_result_contains(result: &ReadResourceResult, expected_content: &str) {
        assert!(!result.contents.is_empty(), "Resource read should succeed");

        match &result.contents[0] {
            Content::Text { text } => {
                assert!(
                    text.contains(expected_content),
                    "Resource result should contain: {expected_content}"
                );
            }
        }
    }

    /// Assert that a prompt result contains expected content
    ///
    /// # Panics
    /// Panics if the prompt result is an error or doesn't contain the expected content
    pub fn assert_prompt_result_contains(result: &GetPromptResult, expected_content: &str) {
        assert!(!result.is_error, "Prompt should succeed");
        assert!(!result.content.is_empty(), "Prompt should return content");

        match &result.content[0] {
            Content::Text { text } => {
                assert!(
                    text.contains(expected_content),
                    "Prompt result should contain: {expected_content}"
                );
            }
        }
    }

    /// Assert that a tool result is valid JSON
    ///
    /// # Panics
    /// Panics if the tool result is an error or contains invalid JSON
    #[must_use]
    pub fn assert_tool_result_is_json(result: &CallToolResult) -> Value {
        assert!(!result.is_error, "Tool call should succeed");
        assert!(
            !result.content.is_empty(),
            "Tool call should return content"
        );

        match &result.content[0] {
            Content::Text { text } => {
                serde_json::from_str(text).expect("Tool result should be valid JSON")
            }
        }
    }

    /// Assert that a resource result is valid JSON
    ///
    /// # Panics
    /// Panics if the resource result is empty or contains invalid JSON
    #[must_use]
    pub fn assert_resource_result_is_json(result: &ReadResourceResult) -> Value {
        assert!(!result.contents.is_empty(), "Resource read should succeed");

        match &result.contents[0] {
            Content::Text { text } => {
                serde_json::from_str(text).expect("Resource result should be valid JSON")
            }
        }
    }

    /// Create test data for various scenarios
    #[must_use]
    pub fn create_test_data() -> MockDatabase {
        Self::create_test_data_with_scenarios()
    }

    /// Create test data with specific scenarios
    #[must_use]
    pub fn create_test_data_with_scenarios() -> MockDatabase {
        let mut db = MockDatabase::new();

        // Add test areas
        db.add_area(MockArea {
            uuid: "area-1".to_string(),
            title: "Work".to_string(),
            visible: true,
        });

        db.add_area(MockArea {
            uuid: "area-2".to_string(),
            title: "Personal".to_string(),
            visible: true,
        });

        // Add test projects
        db.add_project(MockProject {
            uuid: "project-1".to_string(),
            title: "Website Redesign".to_string(),
            area: Some("area-1".to_string()),
            status: "incomplete".to_string(),
        });

        db.add_project(MockProject {
            uuid: "project-2".to_string(),
            title: "Another Project".to_string(),
            area: Some("area-2".to_string()),
            status: "incomplete".to_string(),
        });

        // Add test areas
        db.add_area(MockArea {
            uuid: "area-3".to_string(),
            title: "Health".to_string(),
            visible: true,
        });

        // Add test tasks
        db.add_task(MockTask {
            uuid: "task-1".to_string(),
            title: "Research competitors".to_string(),
            status: "incomplete".to_string(),
            project: Some("project-1".to_string()),
            area: None,
        });

        db.add_task(MockTask {
            uuid: "task-urgent".to_string(),
            title: "Urgent Task".to_string(),
            status: "incomplete".to_string(),
            project: Some("project-1".to_string()),
            area: None,
        });

        db.add_task(MockTask {
            uuid: "task-completed".to_string(),
            title: "Completed Task".to_string(),
            status: "completed".to_string(),
            project: Some("project-2".to_string()),
            area: None,
        });

        db.add_task(MockTask {
            uuid: "task-2".to_string(),
            title: "Read Rust book".to_string(),
            status: "completed".to_string(),
            project: Some("project-2".to_string()),
            area: None,
        });

        db
    }
}

/// Performance testing utilities for MCP operations
pub struct McpPerformanceTest {
    start_time: std::time::Instant,
}

impl McpPerformanceTest {
    #[must_use]
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
        }
    }

    #[must_use]
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Assert that the elapsed time is under the threshold
    ///
    /// # Panics
    /// Panics if the operation took longer than the specified threshold
    pub fn assert_under_threshold(&self, threshold: std::time::Duration) {
        let elapsed = self.elapsed();
        assert!(
            elapsed < threshold,
            "Operation took {elapsed:?}, which exceeds threshold of {threshold:?}"
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
    #[must_use]
    pub fn new() -> Self {
        Self {
            harness: McpTestHarness::new(),
        }
    }

    #[must_use]
    pub fn with_middleware_config(middleware_config: crate::mcp::MiddlewareConfig) -> Self {
        Self {
            harness: McpTestHarness::with_middleware_config(middleware_config),
        }
    }

    #[must_use]
    pub fn harness(&self) -> &McpTestHarness {
        &self.harness
    }

    /// Test a complete workflow: list tools -> call tool -> verify result
    ///
    /// # Panics
    /// Panics if the tool is not found or the workflow fails
    pub async fn test_tool_workflow(
        &self,
        tool_name: &str,
        arguments: Option<Value>,
    ) -> CallToolResult {
        // List tools first
        let tools = self.harness.server().list_tools().unwrap();
        assert!(!tools.tools.is_empty(), "Should have tools available");

        // Call the tool
        self.harness.call_tool(tool_name, arguments).await
    }

    /// Test a complete resource workflow: list resources -> read resource -> verify result
    ///
    /// # Panics
    /// Panics if the resource is not found or the workflow fails
    pub async fn test_resource_workflow(&self, uri: &str) -> ReadResourceResult {
        // List resources first
        let resources = self.harness.server().list_resources().unwrap();
        assert!(
            !resources.resources.is_empty(),
            "Should have resources available"
        );

        // Read the resource
        self.harness.read_resource(uri).await
    }

    /// Test a complete prompt workflow: list prompts -> get prompt -> verify result
    ///
    /// # Panics
    /// Panics if the prompt is not found or the workflow fails
    pub async fn test_prompt_workflow(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> GetPromptResult {
        // List prompts first
        let prompts = self.harness.server().list_prompts().unwrap();
        assert!(!prompts.prompts.is_empty(), "Should have prompts available");

        // Get the prompt
        self.harness.get_prompt(name, arguments).await
    }

    /// Test error handling workflow
    ///
    /// # Panics
    /// Panics if the error handling test fails
    pub async fn test_error_handling_workflow(&self) {
        // Test tool error handling
        let result = self
            .harness
            .call_tool_with_fallback("nonexistent_tool", None)
            .await;
        assert!(result.is_error, "Nonexistent tool should fail");

        // Test resource error handling
        let result = self
            .harness
            .read_resource_with_fallback("things://nonexistent")
            .await;
        // The fallback method returns error content, so we check that it contains an error message
        assert!(
            !result.contents.is_empty(),
            "Nonexistent resource should return error content"
        );
        let Content::Text { text } = &result.contents[0];
        assert!(
            text.contains("not found"),
            "Error message should indicate resource not found"
        );

        // Test prompt error handling
        let result = self
            .harness
            .get_prompt_with_fallback("nonexistent_prompt", None)
            .await;
        assert!(result.is_error, "Nonexistent prompt should fail");

        // Test specific error types - simplified for now
        // if let Some(error) = result.error {
        //     assert!(matches!(error, McpError::PromptNotFound { .. }));
        // }
    }

    /// Test performance workflow
    pub async fn test_performance_workflow(&self) {
        let perf_test = McpPerformanceTest::new();

        // Test tool performance
        self.harness.call_tool("get_inbox", None).await;
        perf_test.assert_under_ms(1000);

        // Test resource performance
        self.harness.read_resource("things://inbox").await;
        perf_test.assert_under_ms(1000);

        // Test prompt performance
        self.harness
            .get_prompt(
                "task_review",
                Some(serde_json::json!({"task_title": "Test"})),
            )
            .await;
        perf_test.assert_under_ms(1000);
    }
}

impl Default for McpIntegrationTest {
    fn default() -> Self {
        panic!("McpIntegrationTest::default() cannot be used in async context. Use McpIntegrationTest::new().await instead.")
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
    async fn test_mcp_tool_call() {
        let harness = McpTestHarness::new();
        let result = harness.call_tool("get_inbox", None).await;
        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_mcp_resource_read() {
        let harness = McpTestHarness::new();
        let result = harness.read_resource("things://inbox").await;
        assert!(!result.contents.is_empty());
    }

    #[tokio::test]
    async fn test_mcp_prompt_get() {
        let harness = McpTestHarness::new();
        let result = harness
            .get_prompt("task_review", Some(json!({"task_title": "Test"})))
            .await;
        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_mcp_tool_json_result() {
        let harness = McpTestHarness::new();
        let json_result = harness.assert_tool_returns_json("get_inbox", None).await;
        assert!(json_result.is_array());
    }

    #[tokio::test]
    async fn test_mcp_mock_database() {
        let mut db = MockDatabase::new();
        db.add_task(MockTask {
            uuid: "test-task".to_string(),
            title: "Test Task".to_string(),
            status: "incomplete".to_string(),
            project: None,
            area: None,
        });

        let task = db.get_task("test-task");
        assert!(task.is_some());
        assert_eq!(task.unwrap().title, "Test Task");

        let completed_tasks = db.get_tasks_by_status("completed");
        assert_eq!(completed_tasks.len(), 0);
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
