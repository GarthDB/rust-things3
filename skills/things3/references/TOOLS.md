# things3 MCP Tool Reference

Complete parameter schemas for all 45 tools exposed by `things3 mcp`.

Source of truth: `apps/things3-cli/src/mcp.rs` — `get_available_tools()` and the dispatch table at line 1907.

> **Note:** `docs/MCP_INTEGRATION.md` documents a subset of these tools in narrative form. This file is the authoritative catalog.

---

## Data retrieval

### `get_inbox`
Get tasks from the inbox.
```json
{ "limit": 50 }
```

### `get_today`
Get tasks scheduled for today.
```json
{ "limit": 50 }
```

### `get_projects`
Get all projects, optionally filtered by area.
```json
{ "area_uuid": "<uuid>", "limit": 50 }
```

### `get_areas`
Get all areas. No parameters.

### `search_tasks`
Search tasks by title or notes.
```json
{ "query": "meeting", "limit": 20 }
```
`query` is required.

### `get_recent_tasks`
Get recently created or modified tasks.
```json
{ "limit": 20, "hours": 24 }
```

### `logbook_search`
Search completed tasks in the logbook.
```json
{
  "search_text": "report",
  "from_date": "2026-01-01",
  "to_date": "2026-04-28",
  "project_uuid": "<uuid>",
  "area_uuid": "<uuid>",
  "tags": ["work"],
  "limit": 50
}
```
All fields optional. `limit` max 500, default 50.

---

## Task mutations

### `create_task`
Create a new task.
```json
{
  "title": "Buy groceries",
  "task_type": "to-do",
  "notes": "Milk, eggs, bread",
  "start_date": "2026-04-28",
  "deadline": "2026-04-30",
  "project_uuid": "<uuid>",
  "area_uuid": "<uuid>",
  "parent_uuid": "<uuid>",
  "tags": ["errands"],
  "status": "incomplete"
}
```
`title` required. `task_type`: `to-do` | `project` | `heading`.

### `update_task`
Update an existing task (patch — only provided fields change).
```json
{
  "uuid": "<uuid>",
  "title": "Buy groceries and wine",
  "notes": "Updated list",
  "start_date": "2026-04-29",
  "deadline": "2026-05-01",
  "status": "incomplete",
  "project_uuid": "<uuid>",
  "area_uuid": "<uuid>",
  "tags": ["errands", "weekend"]
}
```
`uuid` required.

### `complete_task`
Mark a task as completed.
```json
{ "uuid": "<uuid>" }
```

### `uncomplete_task`
Reopen a completed task.
```json
{ "uuid": "<uuid>" }
```

### `delete_task`
Soft-delete a task (moves to Trash).
```json
{
  "uuid": "<uuid>",
  "child_handling": "error"
}
```
`child_handling`: `error` (fail if children exist) | `cascade` (delete children too) | `orphan` (delete parent only). Default: `error`.

### `bulk_create_tasks`
Create multiple tasks in one call.
```json
{
  "tasks": [
    { "title": "Task A", "project_uuid": "<uuid>" },
    { "title": "Task B", "notes": "Details", "area_uuid": "<uuid>" }
  ]
}
```
Each item requires `title`.

---

## Project mutations

### `create_project`
```json
{
  "title": "Website redesign",
  "notes": "Q2 initiative",
  "area_uuid": "<uuid>",
  "start_date": "2026-05-01",
  "deadline": "2026-06-30",
  "tags": ["work"]
}
```
`title` required.

### `update_project`
Patch update — only provided fields change.
```json
{
  "uuid": "<uuid>",
  "title": "Website redesign v2",
  "notes": "Updated scope",
  "area_uuid": "<uuid>",
  "start_date": "2026-05-15",
  "deadline": "2026-07-31"
}
```
`uuid` required.

### `complete_project`
```json
{ "uuid": "<uuid>" }
```

### `delete_project`
Soft-delete a project.
```json
{ "uuid": "<uuid>" }
```

---

## Area mutations

### `create_area`
```json
{ "title": "Personal" }
```

### `update_area`
```json
{ "uuid": "<uuid>", "title": "Personal Projects" }
```

### `delete_area`
```json
{ "uuid": "<uuid>" }
```

---

## Bulk operations (all transactional — all-or-nothing)

### `bulk_move`
Move multiple tasks to a project or area.
```json
{
  "task_uuids": ["<uuid1>", "<uuid2>"],
  "project_uuid": "<uuid>"
}
```
`task_uuids` required. Provide `project_uuid` or `area_uuid` (not both).

### `bulk_update_dates`
Reschedule multiple tasks.
```json
{
  "task_uuids": ["<uuid1>", "<uuid2>"],
  "start_date": "2026-05-01",
  "deadline": "2026-05-07",
  "clear_start_date": false,
  "clear_deadline": false
}
```

### `bulk_complete`
```json
{ "task_uuids": ["<uuid1>", "<uuid2>"] }
```

### `bulk_delete`
Soft-delete multiple tasks.
```json
{ "task_uuids": ["<uuid1>", "<uuid2>"] }
```

---

## Tag discovery

### `search_tags`
Find existing tags (exact + fuzzy).
```json
{ "query": "work", "include_similar": true, "min_similarity": 0.7 }
```

### `get_tag_suggestions`
Dedup-safe suggestions before creating a tag.
```json
{ "title": "Work Projects" }
```

### `get_popular_tags`
```json
{ "limit": 20 }
```

### `get_recent_tags`
```json
{ "limit": 20 }
```

### `get_tag_completions`
Autocomplete for partial input.
```json
{ "partial_input": "wo" }
```

---

## Tag CRUD

### `create_tag`
```json
{
  "title": "work",
  "shortcut": "w",
  "parent_uuid": "<uuid>",
  "force": false
}
```
`title` required. `force: true` skips duplicate check.

### `update_tag`
```json
{
  "uuid": "<uuid>",
  "title": "Work",
  "shortcut": "W",
  "parent_uuid": "<uuid>"
}
```

### `delete_tag`
```json
{ "uuid": "<uuid>", "remove_from_tasks": false }
```

### `merge_tags`
Combine source into target; source is deleted.
```json
{ "source_uuid": "<uuid>", "target_uuid": "<uuid>" }
```

---

## Tag assignment

### `add_tag_to_task`
```json
{ "task_uuid": "<uuid>", "tag_title": "work" }
```

### `remove_tag_from_task`
```json
{ "task_uuid": "<uuid>", "tag_title": "work" }
```

### `set_task_tags`
Replace all tags on a task.
```json
{ "task_uuid": "<uuid>", "tag_titles": ["work", "urgent"] }
```

---

## Tag analytics

### `get_tag_statistics`
Usage stats for a specific tag.
```json
{ "uuid": "<uuid>" }
```

### `find_duplicate_tags`
Surface near-duplicate tags.
```json
{ "min_similarity": 0.85 }
```

---

## Analytics & export

### `get_productivity_metrics`
Task completion rates and trends. No parameters.

### `export_data`
Full data export. No parameters.

---

## Backup

### `backup_database`
Create a database backup. No parameters.

### `restore_database`
Restore from backup. No parameters.

### `list_backups`
```json
{ "backup_dir": "/path/to/backups" }
```
`backup_dir` required.

---

## System / observability

### `get_performance_stats`
Query performance metrics. No parameters.

### `get_system_metrics`
Current resource usage. No parameters.

### `get_cache_stats`
Cache hit rates. No parameters.

---

## Prompts (not tools)

These are MCP prompts, not tools — invoke via `prompts/get`, not `tools/call`:

- `task_review`
- `project_planning`
- `productivity_analysis`
- `backup_strategy`
