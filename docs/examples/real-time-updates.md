# Real-Time Updates and Progress Tracking

This document demonstrates the new real-time updates and progress tracking features implemented for issue #15.

## Features Implemented

### 1. Progress Tracking
- **ProgressTracker**: Tracks long-running operations with real-time updates
- **ProgressManager**: Manages multiple progress trackers
- **Visual Progress Bars**: Optional progress bars using `indicatif`
- **Cancellation Support**: Operations can be cancelled mid-execution

### 2. WebSocket Server
- **Real-time Communication**: WebSocket server for live updates
- **Client Management**: Handles multiple concurrent connections
- **Message Types**: Subscribe, unsubscribe, progress updates, ping/pong
- **Filtered Subscriptions**: Clients can subscribe to specific operations

### 3. Event Broadcasting
- **Event System**: Comprehensive event broadcasting for all entity changes
- **Event Types**: Task, project, area, and progress events
- **Filtering**: Advanced filtering by event type, entity ID, source, and timestamp
- **Real-time Notifications**: Instant notifications for all changes

### 4. Bulk Operations
- **Progress-aware Operations**: All bulk operations show progress
- **Export Operations**: Export all tasks with progress tracking
- **Bulk Updates**: Update multiple tasks with real-time progress
- **Search and Process**: Search and process tasks with progress updates

## Usage Examples

### Starting the WebSocket Server

```bash
# Start the WebSocket server on default port 8080
things3 server

# Start on a custom port
things3 server --port 9000
```

### Watching Real-time Updates

```bash
# Connect to the WebSocket server
things3 watch

# Connect to a custom server
things3 watch --url ws://localhost:9000
```

### Bulk Operations with Progress

```bash
# Export all tasks with progress tracking
things3 bulk export --format json

# Update multiple tasks status
things3 bulk update-status "task-id-1,task-id-2,task-id-3" completed

# Search and process tasks
things3 bulk search-and-process "meeting"
```

### Using Progress Tracking in Code

```rust
use things3_cli::progress::{ProgressManager, ProgressTracker};
use things3_cli::bulk_operations::BulkOperationsManager;

// Create a progress manager
let progress_manager = Arc::new(ProgressManager::new());

// Create a progress tracker for an operation
let tracker = progress_manager.create_tracker(
    "My Operation".to_string(),
    Some(100), // Total items
    true,      // Show progress bar
);

// Update progress
tracker.inc(10);  // Increment by 10
tracker.set_current(50);  // Set to specific value
tracker.set_message("Processing items...".to_string());

// Complete the operation
tracker.complete();
```

### Using Event Broadcasting

```rust
use things3_cli::events::{EventBroadcaster, EventType};

// Create an event broadcaster
let broadcaster = Arc::new(EventBroadcaster::new());

// Subscribe to all events
let mut receiver = broadcaster.subscribe_all();

// Subscribe to specific event types
let filter = EventFilter {
    event_types: Some(vec![EventType::TaskCreated { task_id: Uuid::new_v4() }]),
    entity_ids: None,
    sources: None,
    since: None,
};
let mut filtered_receiver = broadcaster.subscribe(filter).await;

// Broadcast an event
broadcaster.broadcast_task_event(
    EventType::TaskCreated { task_id: task_uuid },
    task_uuid,
    Some(serde_json::to_value(task)?),
    "my_source",
).await?;
```

## WebSocket API

### Message Types

#### Subscribe to Progress Updates
```json
{
  "type": "Subscribe",
  "operation_id": "optional-operation-uuid"
}
```

#### Unsubscribe from Updates
```json
{
  "type": "Unsubscribe",
  "operation_id": "optional-operation-uuid"
}
```

#### Progress Update (from server)
```json
{
  "type": "ProgressUpdate",
  "operation_id": "operation-uuid",
  "operation_name": "Export All Tasks",
  "current": 50,
  "total": 100,
  "message": "Processing task: Meeting notes",
  "timestamp": "2024-01-15T10:30:00Z",
  "status": "InProgress"
}
```

#### Ping/Pong for Keepalive
```json
{
  "type": "Ping"
}
```

```json
{
  "type": "Pong"
}
```

## Event Types

### Task Events
- `TaskCreated { task_id }`
- `TaskUpdated { task_id }`
- `TaskDeleted { task_id }`
- `TaskCompleted { task_id }`
- `TaskCancelled { task_id }`

### Project Events
- `ProjectCreated { project_id }`
- `ProjectUpdated { project_id }`
- `ProjectDeleted { project_id }`
- `ProjectCompleted { project_id }`

### Area Events
- `AreaCreated { area_id }`
- `AreaUpdated { area_id }`
- `AreaDeleted { area_id }`

### Progress Events
- `ProgressStarted { operation_id }`
- `ProgressUpdated { operation_id }`
- `ProgressCompleted { operation_id }`
- `ProgressFailed { operation_id }`

## Configuration

### Environment Variables
- `THINGS3_WS_PORT`: Default WebSocket port (default: 8080)
- `THINGS3_WS_HOST`: WebSocket host (default: 127.0.0.1)

### CLI Options
- `--port`: WebSocket server port
- `--url`: WebSocket client URL
- `--format`: Export format for bulk operations

## Performance Considerations

### Progress Tracking
- Progress updates are sent asynchronously
- Configurable update frequency to avoid overwhelming clients
- Memory-efficient tracking with atomic operations

### WebSocket Server
- Handles multiple concurrent connections
- Automatic cleanup of disconnected clients
- Configurable message buffer sizes

### Event Broadcasting
- Efficient filtering to reduce unnecessary broadcasts
- Memory-bounded event history
- Configurable subscription limits

## Testing

The implementation includes comprehensive tests for:
- Progress tracking functionality
- WebSocket message handling
- Event broadcasting and filtering
- Bulk operations with progress
- Error handling and edge cases

Run tests with:
```bash
cargo test --package things3-cli
```

## Future Enhancements

Potential future improvements:
1. **Persistence**: Store event history in database
2. **Authentication**: Add authentication for WebSocket connections
3. **Rate Limiting**: Implement rate limiting for progress updates
4. **Metrics**: Add performance metrics and monitoring
5. **Webhooks**: Support for webhook notifications
6. **Mobile Support**: Mobile-optimized WebSocket client
