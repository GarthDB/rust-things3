//! Cache invalidation middleware for data consistency

use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};
use uuid::Uuid;

/// Cache invalidation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidationEvent {
    pub event_id: Uuid,
    pub event_type: InvalidationEventType,
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub operation: String,
    pub timestamp: DateTime<Utc>,
    pub affected_caches: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of invalidation events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InvalidationEventType {
    /// Entity was created
    Created,
    /// Entity was updated
    Updated,
    /// Entity was deleted
    Deleted,
    /// Entity was completed
    Completed,
    /// Bulk operation occurred
    BulkOperation,
    /// Cache was manually invalidated
    ManualInvalidation,
    /// Cache expired
    Expired,
    /// Cascade invalidation
    CascadeInvalidation,
}

impl std::fmt::Display for InvalidationEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidationEventType::Created => write!(f, "Created"),
            InvalidationEventType::Updated => write!(f, "Updated"),
            InvalidationEventType::Deleted => write!(f, "Deleted"),
            InvalidationEventType::Completed => write!(f, "Completed"),
            InvalidationEventType::BulkOperation => write!(f, "BulkOperation"),
            InvalidationEventType::ManualInvalidation => write!(f, "ManualInvalidation"),
            InvalidationEventType::Expired => write!(f, "Expired"),
            InvalidationEventType::CascadeInvalidation => write!(f, "CascadeInvalidation"),
        }
    }
}

/// Cache invalidation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidationRule {
    pub rule_id: Uuid,
    pub name: String,
    pub description: String,
    pub entity_type: String,
    pub operations: Vec<String>,
    pub affected_cache_types: Vec<String>,
    pub invalidation_strategy: InvalidationStrategy,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Invalidation strategies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InvalidationStrategy {
    /// Invalidate all caches
    InvalidateAll,
    /// Invalidate specific cache types
    InvalidateSpecific(Vec<String>),
    /// Invalidate by entity ID
    InvalidateByEntity,
    /// Invalidate by pattern
    InvalidateByPattern(String),
    /// Cascade invalidation (invalidate dependent entities)
    CascadeInvalidation,
}

/// Cache invalidation middleware
pub struct CacheInvalidationMiddleware {
    /// Invalidation rules
    rules: Arc<RwLock<HashMap<String, InvalidationRule>>>,
    /// Event history
    events: Arc<RwLock<Vec<InvalidationEvent>>>,
    /// Cache invalidation handlers
    handlers: Arc<RwLock<HashMap<String, Box<dyn CacheInvalidationHandler + Send + Sync>>>>,
    /// Configuration
    config: InvalidationConfig,
    /// Statistics
    stats: Arc<RwLock<InvalidationStats>>,
}

/// Cache invalidation handler trait
pub trait CacheInvalidationHandler {
    /// Handle cache invalidation
    ///
    /// # Errors
    ///
    /// This function will return an error if the invalidation fails
    fn invalidate(&self, event: &InvalidationEvent) -> Result<()>;

    /// Get cache type name
    fn cache_type(&self) -> &str;

    /// Check if this handler can handle the event
    fn can_handle(&self, event: &InvalidationEvent) -> bool;
}

/// Invalidation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidationConfig {
    /// Maximum number of events to keep in history
    pub max_events: usize,
    /// Event retention duration
    pub event_retention: Duration,
    /// Enable cascade invalidation
    pub enable_cascade: bool,
    /// Cascade invalidation depth
    pub cascade_depth: u32,
    /// Enable event batching
    pub enable_batching: bool,
    /// Batch size
    pub batch_size: usize,
    /// Batch timeout
    pub batch_timeout: Duration,
}

impl Default for InvalidationConfig {
    fn default() -> Self {
        Self {
            max_events: 10000,
            event_retention: Duration::from_secs(86400), // 24 hours
            enable_cascade: true,
            cascade_depth: 3,
            enable_batching: true,
            batch_size: 100,
            batch_timeout: Duration::from_secs(5),
        }
    }
}

/// Invalidation statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InvalidationStats {
    pub total_events: u64,
    pub successful_invalidations: u64,
    pub failed_invalidations: u64,
    pub cascade_invalidations: u64,
    pub manual_invalidations: u64,
    pub expired_invalidations: u64,
    pub average_processing_time_ms: f64,
    pub last_invalidation: Option<DateTime<Utc>>,
}

impl CacheInvalidationMiddleware {
    /// Create a new cache invalidation middleware
    #[must_use]
    pub fn new(config: InvalidationConfig) -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(RwLock::new(Vec::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(InvalidationStats::default())),
        }
    }

    /// Create a new middleware with default configuration
    #[must_use]
    pub fn new_default() -> Self {
        Self::new(InvalidationConfig::default())
    }

    /// Register a cache invalidation handler
    pub fn register_handler(&self, handler: Box<dyn CacheInvalidationHandler + Send + Sync>) {
        let mut handlers = self.handlers.write();
        handlers.insert(handler.cache_type().to_string(), handler);
    }

    /// Add an invalidation rule
    pub fn add_rule(&self, rule: InvalidationRule) {
        let mut rules = self.rules.write();
        rules.insert(rule.name.clone(), rule);
    }

    /// Process an invalidation event
    ///
    /// # Errors
    ///
    /// This function will return an error if the event processing fails
    pub async fn process_event(&self, event: InvalidationEvent) -> Result<()> {
        let start_time = std::time::Instant::now();

        // Store the event
        self.store_event(&event);

        // Find applicable rules
        let applicable_rules = self.find_applicable_rules(&event);

        // Process invalidation for each rule
        for rule in applicable_rules {
            if let Err(e) = self.process_rule(&event, &rule).await {
                warn!("Failed to process invalidation rule {}: {}", rule.name, e);
                self.record_failed_invalidation();
            } else {
                self.record_successful_invalidation();
            }
        }

        // Handle cascade invalidation if enabled
        if self.config.enable_cascade {
            self.handle_cascade_invalidation(&event).await?;
        }

        // Update statistics
        let processing_time = start_time.elapsed().as_millis().min(u128::from(u64::MAX)) as f64;
        {
            let mut stats = self.stats.write();
            stats.total_events += 1;
        }
        self.update_processing_time(processing_time);

        debug!(
            "Processed invalidation event: {} for entity: {}:{}",
            event.event_type,
            event.entity_type,
            event
                .entity_id
                .map_or_else(|| "none".to_string(), |id| id.to_string())
        );

        Ok(())
    }

    /// Manually invalidate caches
    ///
    /// # Errors
    ///
    /// This function will return an error if the manual invalidation fails
    pub async fn manual_invalidate(
        &self,
        entity_type: &str,
        entity_id: Option<Uuid>,
        cache_types: Option<Vec<String>>,
    ) -> Result<()> {
        let event = InvalidationEvent {
            event_id: Uuid::new_v4(),
            event_type: InvalidationEventType::ManualInvalidation,
            entity_type: entity_type.to_string(),
            entity_id,
            operation: "manual_invalidation".to_string(),
            timestamp: Utc::now(),
            affected_caches: cache_types.unwrap_or_default(),
            metadata: HashMap::new(),
        };

        self.process_event(event).await?;
        self.record_manual_invalidation();
        Ok(())
    }

    /// Get invalidation statistics
    #[must_use]
    pub fn get_stats(&self) -> InvalidationStats {
        self.stats.read().clone()
    }

    /// Get recent invalidation events
    #[must_use]
    pub fn get_recent_events(&self, limit: usize) -> Vec<InvalidationEvent> {
        let events = self.events.read();
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Get events by entity type
    #[must_use]
    pub fn get_events_by_entity_type(&self, entity_type: &str) -> Vec<InvalidationEvent> {
        let events = self.events.read();
        events
            .iter()
            .filter(|event| event.entity_type == entity_type)
            .cloned()
            .collect()
    }

    /// Store an invalidation event
    fn store_event(&self, event: &InvalidationEvent) {
        let mut events = self.events.write();
        events.push(event.clone());

        // Trim events if we exceed max_events
        if events.len() > self.config.max_events {
            let excess = events.len() - self.config.max_events;
            events.drain(0..excess);
        }

        // Remove old events based on retention policy
        let cutoff_time = Utc::now()
            - chrono::Duration::from_std(self.config.event_retention).unwrap_or_default();
        events.retain(|event| event.timestamp > cutoff_time);
    }

    /// Find applicable invalidation rules
    fn find_applicable_rules(&self, event: &InvalidationEvent) -> Vec<InvalidationRule> {
        let rules = self.rules.read();
        rules
            .values()
            .filter(|rule| {
                rule.enabled
                    && rule.entity_type == event.entity_type
                    && (rule.operations.is_empty() || rule.operations.contains(&event.operation))
            })
            .cloned()
            .collect()
    }

    /// Process an invalidation rule
    async fn process_rule(&self, event: &InvalidationEvent, rule: &InvalidationRule) -> Result<()> {
        match &rule.invalidation_strategy {
            InvalidationStrategy::InvalidateAll => {
                // Invalidate all registered caches
                let handlers_guard = self.handlers.read();
                for handler in handlers_guard.values() {
                    if handler.can_handle(event) {
                        handler.invalidate(event)?;
                    }
                }
            }
            InvalidationStrategy::InvalidateSpecific(cache_types) => {
                // Invalidate specific cache types
                let handlers_guard = self.handlers.read();
                for cache_type in cache_types {
                    if let Some(handler) = handlers_guard.get(cache_type) {
                        if handler.can_handle(event) {
                            handler.invalidate(event)?;
                        }
                    }
                }
            }
            InvalidationStrategy::InvalidateByEntity => {
                // Invalidate by entity ID
                if let Some(_entity_id) = event.entity_id {
                    let handlers_guard = self.handlers.read();
                    for handler in handlers_guard.values() {
                        if handler.can_handle(event) {
                            handler.invalidate(event)?;
                        }
                    }
                }
            }
            InvalidationStrategy::InvalidateByPattern(pattern) => {
                // Invalidate by pattern matching
                let handlers_guard = self.handlers.read();
                for handler in handlers_guard.values() {
                    if handler.can_handle(event) && Self::matches_pattern(event, pattern) {
                        handler.invalidate(event)?;
                    }
                }
            }
            InvalidationStrategy::CascadeInvalidation => {
                // Handle cascade invalidation
                self.handle_cascade_invalidation(event).await?;
            }
        }

        Ok(())
    }

    /// Handle cascade invalidation
    async fn handle_cascade_invalidation(&self, event: &InvalidationEvent) -> Result<()> {
        // Find dependent entities and invalidate them
        let dependent_entities = Self::find_dependent_entities(event);

        for dependent_entity in dependent_entities {
            let dependent_event = InvalidationEvent {
                event_id: Uuid::new_v4(),
                event_type: InvalidationEventType::CascadeInvalidation,
                entity_type: dependent_entity.entity_type,
                entity_id: dependent_entity.entity_id,
                operation: "cascade_invalidation".to_string(),
                timestamp: Utc::now(),
                affected_caches: dependent_entity.affected_caches,
                metadata: HashMap::new(),
            };

            Box::pin(self.process_event(dependent_event)).await?;
            self.record_cascade_invalidation();
        }

        Ok(())
    }

    /// Find dependent entities for cascade invalidation
    fn find_dependent_entities(event: &InvalidationEvent) -> Vec<DependentEntity> {
        // This is a simplified implementation
        // In a real system, you would query a dependency graph or database
        let mut dependent_entities = Vec::new();

        match event.entity_type.as_str() {
            "task" => {
                // If a task is updated, invalidate related projects and areas
                if let Some(_task_id) = event.entity_id {
                    dependent_entities.push(DependentEntity {
                        entity_type: "project".to_string(),
                        entity_id: None, // Would need to look up project ID
                        affected_caches: vec!["l1".to_string(), "l2".to_string()],
                    });
                    dependent_entities.push(DependentEntity {
                        entity_type: "area".to_string(),
                        entity_id: None, // Would need to look up area ID
                        affected_caches: vec!["l1".to_string(), "l2".to_string()],
                    });
                }
            }
            "project" => {
                // If a project is updated, invalidate related tasks
                if let Some(_project_id) = event.entity_id {
                    dependent_entities.push(DependentEntity {
                        entity_type: "task".to_string(),
                        entity_id: None, // Would need to look up task IDs
                        affected_caches: vec!["l1".to_string(), "l2".to_string()],
                    });
                }
            }
            "area" => {
                // If an area is updated, invalidate related projects and tasks
                if let Some(_area_id) = event.entity_id {
                    dependent_entities.push(DependentEntity {
                        entity_type: "project".to_string(),
                        entity_id: None,
                        affected_caches: vec!["l1".to_string(), "l2".to_string()],
                    });
                    dependent_entities.push(DependentEntity {
                        entity_type: "task".to_string(),
                        entity_id: None,
                        affected_caches: vec!["l1".to_string(), "l2".to_string()],
                    });
                }
            }
            _ => {
                // No dependencies for unknown entity types
            }
        }

        dependent_entities
    }

    /// Check if event matches a pattern
    fn matches_pattern(event: &InvalidationEvent, pattern: &str) -> bool {
        // Simple pattern matching - in production, use regex or more sophisticated matching
        event.entity_type.contains(pattern) || event.operation.contains(pattern)
    }

    /// Record successful invalidation
    fn record_successful_invalidation(&self) {
        let mut stats = self.stats.write();
        stats.successful_invalidations += 1;
        stats.last_invalidation = Some(Utc::now());
    }

    /// Record failed invalidation
    fn record_failed_invalidation(&self) {
        let mut stats = self.stats.write();
        stats.failed_invalidations += 1;
    }

    /// Record cascade invalidation
    fn record_cascade_invalidation(&self) {
        let mut stats = self.stats.write();
        stats.cascade_invalidations += 1;
    }

    /// Record manual invalidation
    fn record_manual_invalidation(&self) {
        let mut stats = self.stats.write();
        stats.manual_invalidations += 1;
    }

    /// Update average processing time
    fn update_processing_time(&self, processing_time: f64) {
        let mut stats = self.stats.write();

        // Update running average
        let total_events = stats.total_events as f64;
        stats.average_processing_time_ms =
            (stats.average_processing_time_ms * (total_events - 1.0) + processing_time)
                / total_events;
    }
}

/// Dependent entity for cascade invalidation
#[derive(Debug, Clone)]
struct DependentEntity {
    entity_type: String,
    entity_id: Option<Uuid>,
    affected_caches: Vec<String>,
}

/// Cascade invalidation event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CascadeInvalidationEvent {
    /// Invalidate all dependent entities
    InvalidateAll,
    /// Invalidate specific dependent entities
    InvalidateSpecific(Vec<String>),
    /// Invalidate by dependency level
    InvalidateByLevel(u32),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Mock cache invalidation handler for testing
    struct MockCacheHandler {
        cache_type: String,
        invalidated_events: Arc<RwLock<Vec<InvalidationEvent>>>,
    }

    impl MockCacheHandler {
        fn new(cache_type: &str) -> Self {
            Self {
                cache_type: cache_type.to_string(),
                invalidated_events: Arc::new(RwLock::new(Vec::new())),
            }
        }

        fn _get_invalidated_events(&self) -> Vec<InvalidationEvent> {
            self.invalidated_events.read().clone()
        }
    }

    impl CacheInvalidationHandler for MockCacheHandler {
        fn invalidate(&self, event: &InvalidationEvent) -> Result<()> {
            let mut events = self.invalidated_events.write();
            events.push(event.clone());
            Ok(())
        }

        fn cache_type(&self) -> &str {
            &self.cache_type
        }

        fn can_handle(&self, event: &InvalidationEvent) -> bool {
            event.affected_caches.is_empty() || event.affected_caches.contains(&self.cache_type)
        }
    }

    #[tokio::test]
    async fn test_invalidation_middleware_basic() {
        let middleware = CacheInvalidationMiddleware::new_default();

        // Register mock handlers
        let _l1_handler = Arc::new(MockCacheHandler::new("l1"));
        let _l2_handler = Arc::new(MockCacheHandler::new("l2"));

        middleware.register_handler(Box::new(MockCacheHandler::new("l1")));
        middleware.register_handler(Box::new(MockCacheHandler::new("l2")));

        // Add rules for task, project, and area entities
        let task_rule = InvalidationRule {
            rule_id: Uuid::new_v4(),
            name: "task_rule".to_string(),
            description: "Rule for task invalidation".to_string(),
            entity_type: "task".to_string(),
            operations: vec!["updated".to_string()],
            affected_cache_types: vec!["l1".to_string(), "l2".to_string()],
            invalidation_strategy: InvalidationStrategy::InvalidateAll,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        middleware.add_rule(task_rule);

        let project_rule = InvalidationRule {
            rule_id: Uuid::new_v4(),
            name: "project_rule".to_string(),
            description: "Rule for project invalidation".to_string(),
            entity_type: "project".to_string(),
            operations: vec!["cascade_invalidation".to_string()],
            affected_cache_types: vec!["l1".to_string(), "l2".to_string()],
            invalidation_strategy: InvalidationStrategy::InvalidateAll,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        middleware.add_rule(project_rule);

        let area_rule = InvalidationRule {
            rule_id: Uuid::new_v4(),
            name: "area_rule".to_string(),
            description: "Rule for area invalidation".to_string(),
            entity_type: "area".to_string(),
            operations: vec!["cascade_invalidation".to_string()],
            affected_cache_types: vec!["l1".to_string(), "l2".to_string()],
            invalidation_strategy: InvalidationStrategy::InvalidateAll,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        middleware.add_rule(area_rule);

        // Create an invalidation event
        let event = InvalidationEvent {
            event_id: Uuid::new_v4(),
            event_type: InvalidationEventType::Updated,
            entity_type: "task".to_string(),
            entity_id: Some(Uuid::new_v4()),
            operation: "updated".to_string(),
            timestamp: Utc::now(),
            affected_caches: vec!["l1".to_string(), "l2".to_string()],
            metadata: HashMap::new(),
        };

        // Process the event
        middleware.process_event(event).await.unwrap();

        // Check statistics
        let stats = middleware.get_stats();
        assert_eq!(stats.total_events, 3); // 1 original + 2 cascade events
        assert_eq!(stats.successful_invalidations, 3);
    }

    #[tokio::test]
    async fn test_manual_invalidation() {
        let middleware = CacheInvalidationMiddleware::new_default();

        middleware.register_handler(Box::new(MockCacheHandler::new("l1")));

        // Manual invalidation
        middleware
            .manual_invalidate("task", Some(Uuid::new_v4()), None)
            .await
            .unwrap();

        let stats = middleware.get_stats();
        assert_eq!(stats.manual_invalidations, 1);
    }

    #[tokio::test]
    async fn test_event_storage() {
        let middleware = CacheInvalidationMiddleware::new_default();

        let event = InvalidationEvent {
            event_id: Uuid::new_v4(),
            event_type: InvalidationEventType::Created,
            entity_type: "task".to_string(),
            entity_id: Some(Uuid::new_v4()),
            operation: "created".to_string(),
            timestamp: Utc::now(),
            affected_caches: vec![],
            metadata: HashMap::new(),
        };

        middleware.store_event(&event);

        let recent_events = middleware.get_recent_events(1);
        assert_eq!(recent_events.len(), 1);
        assert_eq!(recent_events[0].entity_type, "task");
    }

    #[tokio::test]
    async fn test_invalidation_middleware_creation() {
        let middleware = CacheInvalidationMiddleware::new_default();
        let stats = middleware.get_stats();

        assert_eq!(stats.total_events, 0);
        assert_eq!(stats.successful_invalidations, 0);
        assert_eq!(stats.failed_invalidations, 0);
        assert_eq!(stats.manual_invalidations, 0);
    }

    #[tokio::test]
    async fn test_invalidation_middleware_with_config() {
        let config = CacheInvalidationConfig {
            enable_cascade_invalidation: true,
            max_events_stored: 1000,
            event_retention_duration: Duration::from_secs(3600),
            batch_processing_size: 10,
            processing_timeout: Duration::from_secs(30),
        };

        let middleware = CacheInvalidationMiddleware::new(config);
        let stats = middleware.get_stats();

        assert_eq!(stats.total_events, 0);
    }

    #[tokio::test]
    async fn test_add_rule() {
        let middleware = CacheInvalidationMiddleware::new_default();

        let rule = InvalidationRule {
            rule_id: Uuid::new_v4(),
            entity_type: "task".to_string(),
            operations: vec!["created".to_string(), "updated".to_string()],
            affected_caches: vec!["task_cache".to_string()],
            cascade_invalidation: true,
            priority: 1,
            enabled: true,
            created_at: Utc::now(),
        };

        middleware.add_rule(rule);

        // Rules are stored internally, we can't directly test them
        // but we can test that the method doesn't panic
    }

    #[tokio::test]
    async fn test_remove_rule() {
        let middleware = CacheInvalidationMiddleware::new_default();
        let rule_id = Uuid::new_v4();

        // Remove non-existent rule should not panic
        middleware.remove_rule(&rule_id);
    }

    #[tokio::test]
    async fn test_register_handler() {
        let middleware = CacheInvalidationMiddleware::new_default();
        let handler = Arc::new(MockCacheHandler::new("test_cache"));

        middleware.register_handler("test_cache", handler);

        // Handler is stored internally, we can't directly test it
        // but we can test that the method doesn't panic
    }

    #[tokio::test]
    async fn test_unregister_handler() {
        let middleware = CacheInvalidationMiddleware::new_default();

        // Unregister non-existent handler should not panic
        middleware.unregister_handler("non_existent_cache");
    }

    #[tokio::test]
    async fn test_process_event_with_handler() {
        let middleware = CacheInvalidationMiddleware::new_default();
        let handler = Arc::new(MockCacheHandler::new("test_cache"));

        middleware.register_handler("test_cache", handler);

        let rule = InvalidationRule {
            rule_id: Uuid::new_v4(),
            entity_type: "task".to_string(),
            operations: vec!["created".to_string()],
            affected_caches: vec!["test_cache".to_string()],
            cascade_invalidation: false,
            priority: 1,
            enabled: true,
            created_at: Utc::now(),
        };
        middleware.add_rule(rule);

        let event = InvalidationEvent {
            event_id: Uuid::new_v4(),
            event_type: InvalidationEventType::Created,
            entity_type: "task".to_string(),
            entity_id: Some(Uuid::new_v4()),
            operation: "created".to_string(),
            timestamp: Utc::now(),
            affected_caches: vec!["test_cache".to_string()],
            metadata: HashMap::new(),
        };

        middleware.process_event(&event).await;

        let stats = middleware.get_stats();
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.successful_invalidations, 1);
    }

    #[tokio::test]
    async fn test_process_event_without_handler() {
        let middleware = CacheInvalidationMiddleware::new_default();

        let event = InvalidationEvent {
            event_id: Uuid::new_v4(),
            event_type: InvalidationEventType::Created,
            entity_type: "task".to_string(),
            entity_id: Some(Uuid::new_v4()),
            operation: "created".to_string(),
            timestamp: Utc::now(),
            affected_caches: vec!["non_existent_cache".to_string()],
            metadata: HashMap::new(),
        };

        middleware.process_event(&event).await;

        let stats = middleware.get_stats();
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.failed_invalidations, 1);
    }

    #[tokio::test]
    async fn test_cascade_invalidation() {
        let middleware = CacheInvalidationMiddleware::new_default();

        // Add rules for cascade invalidation
        let task_rule = InvalidationRule {
            rule_id: Uuid::new_v4(),
            entity_type: "task".to_string(),
            operations: vec!["updated".to_string()],
            affected_caches: vec!["task_cache".to_string()],
            cascade_invalidation: true,
            priority: 1,
            enabled: true,
            created_at: Utc::now(),
        };
        middleware.add_rule(task_rule);

        let project_rule = InvalidationRule {
            rule_id: Uuid::new_v4(),
            entity_type: "project".to_string(),
            operations: vec!["cascade_invalidation".to_string()],
            affected_caches: vec!["project_cache".to_string()],
            cascade_invalidation: false,
            priority: 2,
            enabled: true,
            created_at: Utc::now(),
        };
        middleware.add_rule(project_rule);

        let area_rule = InvalidationRule {
            rule_id: Uuid::new_v4(),
            entity_type: "area".to_string(),
            operations: vec!["cascade_invalidation".to_string()],
            affected_caches: vec!["area_cache".to_string()],
            cascade_invalidation: false,
            priority: 3,
            enabled: true,
            created_at: Utc::now(),
        };
        middleware.add_rule(area_rule);

        let event = InvalidationEvent {
            event_id: Uuid::new_v4(),
            event_type: InvalidationEventType::Updated,
            entity_type: "task".to_string(),
            entity_id: Some(Uuid::new_v4()),
            operation: "updated".to_string(),
            timestamp: Utc::now(),
            affected_caches: vec!["task_cache".to_string()],
            metadata: HashMap::new(),
        };

        middleware.process_event(&event).await;

        let stats = middleware.get_stats();
        assert_eq!(stats.total_events, 3); // Original + 2 cascade events
        assert_eq!(stats.successful_invalidations, 3);
    }

    #[tokio::test]
    async fn test_get_recent_events() {
        let middleware = CacheInvalidationMiddleware::new_default();

        // Add multiple events
        for i in 0..5 {
            let event = InvalidationEvent {
                event_id: Uuid::new_v4(),
                event_type: InvalidationEventType::Created,
                entity_type: format!("task_{}", i),
                entity_id: Some(Uuid::new_v4()),
                operation: "created".to_string(),
                timestamp: Utc::now(),
                affected_caches: vec![],
                metadata: HashMap::new(),
            };
            middleware.store_event(&event);
        }

        // Get recent events
        let recent_events = middleware.get_recent_events(3);
        assert_eq!(recent_events.len(), 3);

        // Get all events
        let all_events = middleware.get_recent_events(10);
        assert_eq!(all_events.len(), 5);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let middleware = CacheInvalidationMiddleware::new_default();

        let initial_stats = middleware.get_stats();
        assert_eq!(initial_stats.total_events, 0);
        assert_eq!(initial_stats.successful_invalidations, 0);
        assert_eq!(initial_stats.failed_invalidations, 0);
        assert_eq!(initial_stats.manual_invalidations, 0);
        assert_eq!(initial_stats.average_processing_time_ms, 0.0);
        assert_eq!(initial_stats.success_rate, 0.0);
    }

    #[tokio::test]
    async fn test_invalidate_all() {
        let middleware = CacheInvalidationMiddleware::new_default();
        let handler = Arc::new(MockCacheHandler::new("test_cache"));

        middleware.register_handler("test_cache", handler);

        middleware.invalidate_all().await;

        let stats = middleware.get_stats();
        assert_eq!(stats.manual_invalidations, 1);
    }

    #[tokio::test]
    async fn test_invalidate_by_entity_type() {
        let middleware = CacheInvalidationMiddleware::new_default();
        let handler = Arc::new(MockCacheHandler::new("test_cache"));

        middleware.register_handler("test_cache", handler);

        middleware.invalidate_by_entity_type("task").await;

        let stats = middleware.get_stats();
        assert_eq!(stats.manual_invalidations, 1);
    }

    #[tokio::test]
    async fn test_invalidate_by_entity_id() {
        let middleware = CacheInvalidationMiddleware::new_default();
        let handler = Arc::new(MockCacheHandler::new("test_cache"));

        middleware.register_handler("test_cache", handler);

        let entity_id = Uuid::new_v4();
        middleware.invalidate_by_entity_id("task", &entity_id).await;

        let stats = middleware.get_stats();
        assert_eq!(stats.manual_invalidations, 1);
    }

    #[tokio::test]
    async fn test_find_dependent_entities() {
        let event = InvalidationEvent {
            event_id: Uuid::new_v4(),
            event_type: InvalidationEventType::Updated,
            entity_type: "task".to_string(),
            entity_id: Some(Uuid::new_v4()),
            operation: "updated".to_string(),
            timestamp: Utc::now(),
            affected_caches: vec![],
            metadata: HashMap::new(),
        };

        let dependent_entities = CacheInvalidationMiddleware::find_dependent_entities(&event);

        assert_eq!(dependent_entities.len(), 2); // project and area
        assert!(dependent_entities
            .iter()
            .any(|dep| dep.entity_type == "project"));
        assert!(dependent_entities
            .iter()
            .any(|dep| dep.entity_type == "area"));
    }

    #[tokio::test]
    async fn test_find_dependent_entities_project() {
        let event = InvalidationEvent {
            event_id: Uuid::new_v4(),
            event_type: InvalidationEventType::Updated,
            entity_type: "project".to_string(),
            entity_id: Some(Uuid::new_v4()),
            operation: "updated".to_string(),
            timestamp: Utc::now(),
            affected_caches: vec![],
            metadata: HashMap::new(),
        };

        let dependent_entities = CacheInvalidationMiddleware::find_dependent_entities(&event);

        assert_eq!(dependent_entities.len(), 1); // area only
        assert!(dependent_entities
            .iter()
            .any(|dep| dep.entity_type == "area"));
    }

    #[tokio::test]
    async fn test_find_dependent_entities_area() {
        let event = InvalidationEvent {
            event_id: Uuid::new_v4(),
            event_type: InvalidationEventType::Updated,
            entity_type: "area".to_string(),
            entity_id: Some(Uuid::new_v4()),
            operation: "updated".to_string(),
            timestamp: Utc::now(),
            affected_caches: vec![],
            metadata: HashMap::new(),
        };

        let dependent_entities = CacheInvalidationMiddleware::find_dependent_entities(&event);

        assert_eq!(dependent_entities.len(), 0); // no dependencies
    }

    #[tokio::test]
    async fn test_find_dependent_entities_unknown() {
        let event = InvalidationEvent {
            event_id: Uuid::new_v4(),
            event_type: InvalidationEventType::Updated,
            entity_type: "unknown".to_string(),
            entity_id: Some(Uuid::new_v4()),
            operation: "updated".to_string(),
            timestamp: Utc::now(),
            affected_caches: vec![],
            metadata: HashMap::new(),
        };

        let dependent_entities = CacheInvalidationMiddleware::find_dependent_entities(&event);

        assert_eq!(dependent_entities.len(), 0); // no dependencies for unknown entity
    }

    #[tokio::test]
    async fn test_concurrent_event_processing() {
        let middleware = Arc::new(CacheInvalidationMiddleware::new_default());
        let mut handles = vec![];

        // Spawn multiple tasks to process events concurrently
        for i in 0..10 {
            let middleware_clone = Arc::clone(&middleware);
            let handle = tokio::spawn(async move {
                let event = InvalidationEvent {
                    event_id: Uuid::new_v4(),
                    event_type: InvalidationEventType::Created,
                    entity_type: format!("task_{}", i),
                    entity_id: Some(Uuid::new_v4()),
                    operation: "created".to_string(),
                    timestamp: Utc::now(),
                    affected_caches: vec![],
                    metadata: HashMap::new(),
                };
                middleware_clone.process_event(&event).await;
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        let stats = middleware.get_stats();
        assert_eq!(stats.total_events, 10);
    }

    #[tokio::test]
    async fn test_event_retention() {
        let config = CacheInvalidationConfig {
            enable_cascade_invalidation: false,
            max_events_stored: 3, // Very small limit
            event_retention_duration: Duration::from_secs(1),
            batch_processing_size: 10,
            processing_timeout: Duration::from_secs(30),
        };

        let middleware = CacheInvalidationMiddleware::new(config);

        // Add more events than the limit
        for i in 0..5 {
            let event = InvalidationEvent {
                event_id: Uuid::new_v4(),
                event_type: InvalidationEventType::Created,
                entity_type: format!("task_{}", i),
                entity_id: Some(Uuid::new_v4()),
                operation: "created".to_string(),
                timestamp: Utc::now(),
                affected_caches: vec![],
                metadata: HashMap::new(),
            };
            middleware.store_event(&event);
        }

        // Should only store the most recent events
        let recent_events = middleware.get_recent_events(10);
        assert!(recent_events.len() <= 3);
    }
}
