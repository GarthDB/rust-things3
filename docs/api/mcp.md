# MCP Tools Reference

The Rust Things MCP server provides 17 tools for AI/LLM integration. This document describes each tool in detail.

> **Note**: The MCP server now uses SQLx for async database operations, providing better performance and thread safety compared to the previous rusqlite implementation.

## Core Task Tools

### `get_inbox`
Get tasks from the Things 3 inbox.

**Parameters:**
- `limit` (optional): Maximum number of tasks to return

**Example:**
```json
{
  "name": "get_inbox",
  "arguments": {
    "limit": 10
  }
}
```

**Response:**
```json
{
  "content": [
    {
      "json": [
        {
          "uuid": "123e4567-e89b-12d3-a456-426614174000",
          "title": "Review project proposal",
          "status": "Incomplete",
          "created": "2024-01-15T10:30:00Z"
        }
      ]
    }
  ],
  "is_error": false
}
```

### `get_today`
Get tasks scheduled for today.

**Parameters:**
- `limit` (optional): Maximum number of tasks to return

**Example:**
```json
{
  "name": "get_today",
  "arguments": {
    "limit": 5
  }
}
```

### `get_projects`
Get all projects, optionally filtered by area.

**Parameters:**
- `area_uuid` (optional): UUID of the area to filter by

**Example:**
```json
{
  "name": "get_projects",
  "arguments": {
    "area_uuid": "123e4567-e89b-12d3-a456-426614174000"
  }
}
```

### `get_areas`
Get all areas.

**Parameters:** None

**Example:**
```json
{
  "name": "get_areas",
  "arguments": {}
}
```

### `search_tasks`
Search for tasks by title or notes.

**Parameters:**
- `query` (required): Search query string
- `limit` (optional): Maximum number of results

**Example:**
```json
{
  "name": "search_tasks",
  "arguments": {
    "query": "meeting",
    "limit": 10
  }
}
```

## Task Management Tools

### `create_task`
Create a new task.

**Parameters:**
- `title` (required): Task title
- `notes` (optional): Task notes
- `task_type` (optional): Task type (Todo, Project, Heading, Area)
- `status` (optional): Task status (Incomplete, Completed, Canceled, Trashed)
- `start_date` (optional): Start date (YYYY-MM-DD)
- `deadline` (optional): Deadline (YYYY-MM-DD)
- `project_uuid` (optional): Parent project UUID
- `area_uuid` (optional): Area UUID
- `parent_uuid` (optional): Parent task UUID
- `tags` (optional): Array of tag strings

**Example:**
```json
{
  "name": "create_task",
  "arguments": {
    "title": "Review quarterly report",
    "notes": "Focus on Q4 metrics",
    "deadline": "2024-01-31",
    "area_uuid": "123e4567-e89b-12d3-a456-426614174000",
    "tags": ["work", "urgent"]
  }
}
```

### `update_task`
Update an existing task.

**Parameters:**
- `uuid` (required): Task UUID to update
- All other parameters from `create_task` (optional)

**Example:**
```json
{
  "name": "update_task",
  "arguments": {
    "uuid": "123e4567-e89b-12d3-a456-426614174000",
    "status": "Completed",
    "notes": "Task completed successfully"
  }
}
```

### `bulk_create_tasks`
Create multiple tasks at once.

**Parameters:**
- `tasks` (required): Array of task objects

**Example:**
```json
{
  "name": "bulk_create_tasks",
  "arguments": {
    "tasks": [
      {
        "title": "Task 1",
        "notes": "First task"
      },
      {
        "title": "Task 2",
        "notes": "Second task"
      }
    ]
  }
}
```

### `get_recent_tasks`
Get recently modified tasks.

**Parameters:**
- `limit` (optional): Maximum number of tasks to return

**Example:**
```json
{
  "name": "get_recent_tasks",
  "arguments": {
    "limit": 20
  }
}
```

## Analytics and Metrics Tools

### `get_productivity_metrics`
Get productivity metrics and statistics.

**Parameters:**
- `start_date` (optional): Start date for metrics (YYYY-MM-DD)
- `end_date` (optional): End date for metrics (YYYY-MM-DD)

**Example:**
```json
{
  "name": "get_productivity_metrics",
  "arguments": {
    "start_date": "2024-01-01",
    "end_date": "2024-01-31"
  }
}
```

### `get_performance_stats`
Get performance statistics for operations.

**Parameters:** None

**Example:**
```json
{
  "name": "get_performance_stats",
  "arguments": {}
}
```

### `get_system_metrics`
Get current system resource metrics.

**Parameters:** None

**Example:**
```json
{
  "name": "get_system_metrics",
  "arguments": {}
}
```

### `get_cache_stats`
Get cache performance statistics.

**Parameters:** None

**Example:**
```json
{
  "name": "get_cache_stats",
  "arguments": {}
}
```

## Data Management Tools

### `export_data`
Export data in various formats.

**Parameters:**
- `format` (optional): Export format (json, csv, opml, markdown)
- `path` (optional): File path to save export

**Example:**
```json
{
  "name": "export_data",
  "arguments": {
    "format": "json",
    "path": "/tmp/things_export.json"
  }
}
```

### `backup_database`
Create a database backup.

**Parameters:**
- `backup_dir` (required): Directory to store backup
- `description` (optional): Backup description

**Example:**
```json
{
  "name": "backup_database",
  "arguments": {
    "backup_dir": "/backups",
    "description": "Weekly backup"
  }
}
```

### `restore_database`
Restore from a backup.

**Parameters:**
- `backup_path` (required): Path to backup file

**Example:**
```json
{
  "name": "restore_database",
  "arguments": {
    "backup_path": "/backups/things_backup_20240115_143022.sqlite"
  }
}
```

### `list_backups`
List available backups.

**Parameters:**
- `backup_dir` (required): Directory containing backups

**Example:**
```json
{
  "name": "list_backups",
  "arguments": {
    "backup_dir": "/backups"
  }
}
```

## Error Handling

All tools return a standardized response format:

```json
{
  "content": [
    {
      "text": "Success message" // or
      "json": { /* data */ }
    }
  ],
  "is_error": false
}
```

Error responses:
```json
{
  "content": [
    {
      "text": "Error: Task not found"
    }
  ],
  "is_error": true
}
```

## Best Practices

1. **Use appropriate limits** to avoid overwhelming responses
2. **Handle errors gracefully** by checking `is_error` field
3. **Use caching tools** to monitor performance
4. **Backup regularly** using the backup tools
5. **Monitor system metrics** for performance insights

## Tool Categories

- **Core Tools**: `get_inbox`, `get_today`, `get_projects`, `get_areas`, `search_tasks`
- **Management Tools**: `create_task`, `update_task`, `bulk_create_tasks`, `get_recent_tasks`
- **Analytics Tools**: `get_productivity_metrics`, `get_performance_stats`, `get_system_metrics`, `get_cache_stats`
- **Data Tools**: `export_data`, `backup_database`, `restore_database`, `list_backups`
