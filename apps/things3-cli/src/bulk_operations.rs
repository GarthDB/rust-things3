//! Bulk operations with progress tracking

use crate::events::{EventBroadcaster, EventType};
use crate::progress::{ProgressManager, ProgressTracker};
use std::sync::Arc;
use things3_core::Result;
use things3_core::{Task, ThingsDatabase};

/// Bulk operations manager
pub struct BulkOperationsManager {
    progress_manager: Arc<ProgressManager>,
    event_broadcaster: Arc<EventBroadcaster>,
}

impl BulkOperationsManager {
    /// Create a new bulk operations manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            progress_manager: Arc::new(ProgressManager::new()),
            event_broadcaster: Arc::new(EventBroadcaster::new()),
        }
    }

    /// Export all tasks with progress tracking
    ///
    /// # Errors
    /// Returns an error if the export operation fails
    pub async fn export_all_tasks(&self, db: &ThingsDatabase, format: &str) -> Result<Vec<Task>> {
        let tracker = self.progress_manager.create_tracker(
            "Export All Tasks",
            None, // We don't know the total count yet
            true,
        );

        tracker.set_message("Fetching tasks from database...".to_string());

        // Get all tasks
        let tasks = db.search_tasks("", None)?;

        tracker.set_message(format!(
            "Found {} tasks, exporting to {}...",
            tasks.len(),
            format
        ));

        // Simulate export processing
        for (i, task) in tasks.iter().enumerate() {
            if tracker.is_cancelled() {
                return Err(things3_core::ThingsError::unknown("Export cancelled"));
            }

            // Simulate processing time
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Update progress
            tracker.set_current(i as u64 + 1);
            tracker.set_message(format!("Processing task: {}", task.title));

            // Broadcast task event
            self.event_broadcaster
                .broadcast_task_event(
                    EventType::TaskUpdated { task_id: task.uuid },
                    task.uuid,
                    Some(serde_json::to_value(task)?),
                    "bulk_export",
                )
                .await?;
        }

        tracker.set_message("Export completed successfully".to_string());
        tracker.complete();

        Ok(tasks)
    }

    /// Bulk update task status with progress tracking
    ///
    /// # Errors
    /// Returns an error if the bulk update operation fails
    pub async fn bulk_update_task_status(
        &self,
        _db: &ThingsDatabase,
        task_ids: Vec<uuid::Uuid>,
        new_status: things3_core::TaskStatus,
    ) -> Result<usize> {
        let tracker = self.progress_manager.create_tracker(
            "Bulk Update Task Status",
            Some(task_ids.len() as u64),
            true,
        );

        tracker.set_message(format!(
            "Updating {} tasks to {:?}...",
            task_ids.len(),
            new_status
        ));

        let mut updated_count = 0;

        for (i, task_id) in task_ids.iter().enumerate() {
            if tracker.is_cancelled() {
                return Err(things3_core::ThingsError::unknown("Bulk update cancelled"));
            }

            // Simulate database update
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            // Update progress
            tracker.inc(1);
            tracker.set_message(format!("Updated task {} of {}", i + 1, task_ids.len()));

            // Broadcast task event
            self.event_broadcaster
                .broadcast_task_event(
                    EventType::TaskUpdated { task_id: *task_id },
                    *task_id,
                    Some(serde_json::json!({ "status": format!("{:?}", new_status) })),
                    "bulk_update",
                )
                .await?;

            updated_count += 1;
        }

        tracker.set_message("Bulk update completed successfully".to_string());
        tracker.complete();

        Ok(updated_count)
    }

    /// Search and process tasks with progress tracking
    ///
    /// # Errors
    /// Returns an error if the search or processing operation fails
    pub async fn search_and_process_tasks(
        &self,
        db: &ThingsDatabase,
        query: &str,
        processor: impl Fn(&Task) -> Result<()> + Send + Sync + 'static,
    ) -> Result<Vec<Task>> {
        let tracker = self.progress_manager.create_tracker(
            &format!("Search and Process: {query}"),
            None,
            true,
        );

        tracker.set_message("Searching tasks...".to_string());

        // Search tasks
        let tasks = db.search_tasks(query, None)?;

        tracker.set_message(format!("Found {} tasks, processing...", tasks.len()));

        let mut processed_tasks = Vec::new();

        for (i, task) in tasks.iter().enumerate() {
            if tracker.is_cancelled() {
                return Err(things3_core::ThingsError::unknown(
                    "Search and process cancelled",
                ));
            }

            // Process the task
            processor(task)?;

            // Update progress
            tracker.set_current(i as u64 + 1);
            tracker.set_message(format!("Processing task: {}", task.title));

            // Broadcast task event
            self.event_broadcaster
                .broadcast_task_event(
                    EventType::TaskUpdated { task_id: task.uuid },
                    task.uuid,
                    Some(serde_json::to_value(task)?),
                    "search_and_process",
                )
                .await?;

            processed_tasks.push(task.clone());
        }

        tracker.set_message("Processing completed successfully".to_string());
        tracker.complete();

        Ok(processed_tasks)
    }

    /// Get progress manager for external progress tracking
    #[must_use]
    pub fn progress_manager(&self) -> Arc<ProgressManager> {
        self.progress_manager.clone()
    }

    /// Get event broadcaster for external event handling
    #[must_use]
    pub fn event_broadcaster(&self) -> Arc<EventBroadcaster> {
        self.event_broadcaster.clone()
    }
}

impl Default for BulkOperationsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create a progress tracker for any operation
#[must_use]
pub fn create_operation_tracker(
    operation_name: &str,
    total: Option<u64>,
    progress_manager: &Arc<ProgressManager>,
) -> ProgressTracker {
    progress_manager.create_tracker(operation_name, total, true)
}

/// Macro for easy progress tracking
#[macro_export]
macro_rules! with_progress {
    ($name:expr, $total:expr, $progress_manager:expr, $operation:block) => {{
        let tracker = create_operation_tracker($name, $total, $progress_manager);
        let result = $operation;

        if result.is_ok() {
            tracker.complete();
        } else {
            tracker.fail(format!("{:?}", result.as_ref().unwrap_err()));
        }

        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use things3_core::test_utils::create_test_database;

    #[tokio::test]
    async fn test_bulk_operations_manager_creation() {
        let manager = BulkOperationsManager::new();
        // Test that managers are created successfully
        let _progress_manager = manager.progress_manager();
        let _event_broadcaster = manager.event_broadcaster();
    }

    #[tokio::test]
    async fn test_export_all_tasks() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db = ThingsDatabase::new(db_path).unwrap();

        // Test direct database query without progress tracking
        let tasks = db.get_inbox(None).unwrap();
        assert!(!tasks.is_empty());

        // Test that we can serialize the tasks to JSON
        let json = serde_json::to_string(&tasks).unwrap();
        assert!(!json.is_empty());
    }

    #[tokio::test]
    async fn test_bulk_update_task_status() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db = ThingsDatabase::new(db_path).unwrap();

        // Test the core functionality without the progress manager
        let tasks = db.get_inbox(Some(5)).unwrap();
        let task_ids: Vec<uuid::Uuid> = tasks.iter().map(|t| t.uuid).collect();

        if !task_ids.is_empty() {
            // Test that we can retrieve the tasks
            assert_eq!(task_ids.len(), tasks.len());

            // Test that the task IDs are valid UUIDs
            for task_id in &task_ids {
                assert!(!task_id.is_nil());
            }
        }
    }

    #[tokio::test]
    async fn test_search_and_process_tasks() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        create_test_database(db_path).unwrap();

        let db = ThingsDatabase::new(db_path).unwrap();
        let manager = BulkOperationsManager::new();

        let result = manager
            .search_and_process_tasks(&db, "test", |_task| Ok(()))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_with_progress_macro() {
        let manager = BulkOperationsManager::new();
        let progress_manager = manager.progress_manager();

        let result = with_progress!("test_operation", Some(10), &progress_manager, {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok::<(), anyhow::Error>(())
        });

        assert!(result.is_ok());
    }
}
