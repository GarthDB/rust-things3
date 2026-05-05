---
name: things3
description: Interact with Things 3 (read tasks, create and update tasks and projects, manage tags, triage inbox) via the rust-things3 MCP server. Use when the user wants to work with their Things 3 task manager from an AI agent.
---

# /things3

Drive Things 3 from your AI agent via the [rust-things3](https://github.com/GarthDB/rust-things3) MCP server. Read tasks and projects, create and update items, manage tags, search the logbook, and bulk-process your inbox — all over stdio JSON-RPC.

## Claude Code slash command

To use `/things3` as a Claude Code slash command, copy the skill to your local skills directory and patch in a `trigger` key. The canonical file omits it because `skills-ref validate` rejects unknown frontmatter fields.

```bash
cp -r skills/things3 ~/.claude/skills/things3
python3 -c "
import pathlib, re
p = pathlib.Path('~/.claude/skills/things3/SKILL.md').expanduser()
p.write_text(re.sub(r'(?m)^---\$(?=\n\n)', 'trigger: /things3\n---', p.read_text(), count=1))
"
```

See [`references/HOSTS.md`](references/HOSTS.md) for MCP server installation.

## Prerequisites

The `things3` CLI must be installed and the MCP server must be configured in your host. See [`references/HOSTS.md`](references/HOSTS.md) for copy-paste config snippets for Claude Desktop, Cursor, VS Code, and Zed.

**Things 3 must be running** on macOS for the MCP server to reach its database.

## Mutation backend

On macOS, all 24 write tools route through **AppleScript** by default — CulturedCode's documented scripting API. Reads continue to use direct SQLite. This is the safe-by-default configuration: the data-corruption risk that motivated CulturedCode's [warning against direct database writes](https://culturedcode.com/things/support/articles/5510170/) does not apply to AppleScript-mediated mutations.

### One-time Automation permission

The first write triggers a macOS prompt asking your terminal/host to control Things 3. Grant it once. If denied or later revoked, you will see:

> macOS Automation permission denied. Grant access in System Settings → Privacy & Security → Automation, then retry.

Re-enable in **System Settings → Privacy & Security → Automation**, then retry the call.

### `--unsafe-direct-db` opt-in (discouraged)

The deprecated direct-SQLite backend (`SqlxBackend`) can be re-enabled with `--unsafe-direct-db` or `THINGS_UNSAFE_DIRECT_DB=1`. CulturedCode warns that direct writes "can corrupt your data and break syncs." Use only when:

- Running on non-macOS (Linux/CI), where AppleScript is unavailable.
- You explicitly need `restore_database` (see below).

This flag will be removed in a future release.

### `restore_database` is gated

`restore_database` overwrites the live SQLite file and is the highest-corruption operation the server exposes. It refuses to run unless **both** conditions hold:

1. The server was launched with `--unsafe-direct-db` (or `THINGS_UNSAFE_DIRECT_DB=1`).
2. Things 3 is **not** running. Quit Things 3 (Cmd-Q) before invoking.

Without (1) you get a structured validation error pointing to the CulturedCode article. Without (2) you get a "refuses to run while Things 3 is open" error.

## When to use this skill

- Reading inbox, today, or project task lists
- Creating, updating, or completing tasks and projects
- GTD workflows (see the companion `things3-inbox-triage` skill)
- Daily reviews (see the companion `things3-daily-review` skill)
- Tag management — searching, deduplicating, or reorganising tags
- Querying the logbook for completed work
- Exporting data or pulling performance / cache diagnostics

## Tool catalog (46 tools)

Tools are grouped by domain. One-line signatures below; full parameter schemas in [`references/TOOLS.md`](references/TOOLS.md).

### Read — task lists

| Tool | Key params | When to use |
|---|---|---|
| `get_inbox` | `limit?` | Unscheduled, uncategorised tasks |
| `get_today` | `limit?` | Tasks scheduled for today |
| `get_recent_tasks` | `limit?, hours?` | Recently created or modified tasks |
| `search_tasks` | `query*, limit?` | Full-text search across all tasks |
| `logbook_search` | `search_text?, from_date?, to_date?, project_uuid?, area_uuid?, tags?, limit?` | Completed tasks in the logbook |

### Read — structure

| Tool | Key params | When to use |
|---|---|---|
| `get_projects` | `area_uuid?, limit?` | All projects (optionally filtered by area) |
| `get_areas` | — | All areas |

### Task mutations

| Tool | Key params | When to use |
|---|---|---|
| `create_task` | `title*, notes?, start_date?, deadline?, project_uuid?, area_uuid?, tags?, status?` | Add a new task |
| `update_task` | `uuid*, title?, notes?, start_date?, deadline?, status?, project_uuid?, area_uuid?, tags?` | Edit any field on an existing task |
| `complete_task` | `uuid*` | Mark a task done |
| `uncomplete_task` | `uuid*` | Reopen a completed task |
| `delete_task` | `uuid*, child_handling?` | Soft-delete (trashed); `child_handling`: error/cascade/orphan |
| `bulk_create_tasks` | `tasks*[]` | Create multiple tasks in one call |

### Project mutations

| Tool | Key params | When to use |
|---|---|---|
| `create_project` | `title*, notes?, area_uuid?, start_date?, deadline?, tags?` | New project |
| `update_project` | `uuid*, title?, notes?, area_uuid?, start_date?, deadline?` | Edit a project |
| `complete_project` | `uuid*` | Mark a project done |
| `delete_project` | `uuid*` | Soft-delete a project |

### Area mutations

| Tool | Key params | When to use |
|---|---|---|
| `create_area` | `title*` | New area |
| `update_area` | `uuid*, title?` | Rename an area |
| `delete_area` | `uuid*` | Delete an area |

### Bulk operations (transactional)

| Tool | Key params | When to use |
|---|---|---|
| `bulk_move` | `task_uuids*, project_uuid?, area_uuid?` | Move many tasks at once (`project_uuid` and `area_uuid` are mutually exclusive — provide exactly one) |
| `bulk_update_dates` | `task_uuids*, start_date?, deadline?, clear_start_date?, clear_deadline?` | Reschedule many tasks |
| `bulk_complete` | `task_uuids*` | Complete many tasks |
| `bulk_delete` | `task_uuids*` | Delete many tasks |

### Tag discovery

| Tool | Key params | When to use |
|---|---|---|
| `search_tags` | `query*, include_similar?, min_similarity?` | Find existing tags before creating |
| `get_tag_suggestions` | `title*` | Prevent duplicates when creating a tag |
| `get_popular_tags` | `limit?` | Most-used tags |
| `get_recent_tags` | `limit?` | Recently-used tags |
| `get_tag_completions` | `partial_input*` | Autocomplete for partial tag input |

### Tag CRUD

| Tool | Key params | When to use |
|---|---|---|
| `create_tag` | `title*, shortcut?, parent_uuid?, force?` | New tag (dedup-checked by default) |
| `update_tag` | `uuid*, title?, shortcut?, parent_uuid?` | Rename or re-nest a tag |
| `delete_tag` | `uuid*, remove_from_tasks?` | Delete a tag |
| `merge_tags` | `source_uuid*, target_uuid*` | Consolidate duplicate tags |

### Tag assignment

| Tool | Key params | When to use |
|---|---|---|
| `add_tag_to_task` | `task_uuid*, tag_title*` | Add one tag |
| `remove_tag_from_task` | `task_uuid*, tag_title*` | Remove one tag |
| `set_task_tags` | `task_uuid*, tag_titles*[]` | Replace all tags on a task |

### Tag analytics

| Tool | Key params | When to use |
|---|---|---|
| `get_tag_statistics` | `uuid*` | Usage stats for a specific tag |
| `find_duplicate_tags` | `min_similarity?` | Surface near-duplicate tags |

### Analytics & export

| Tool | Key params | When to use |
|---|---|---|
| `get_productivity_metrics` | — | Task completion rates and trends |
| `export_data` | — | Full data export |

### Backup

| Tool | Key params | When to use |
|---|---|---|
| `backup_database` | — | Create a database backup |
| `restore_database` | — | Restore from backup |
| `list_backups` | `backup_dir*` | List available backups |

### System / observability

| Tool | Key params | When to use |
|---|---|---|
| `get_performance_stats` | — | Query performance metrics |
| `get_system_metrics` | — | Current resource usage |
| `get_cache_stats` | — | Cache hit rates |

`*` = required. Full schemas: [`references/TOOLS.md`](references/TOOLS.md). (46 tools total)

## Prompts (4)

The server also exposes four prompt templates: `task_review`, `project_planning`, `productivity_analysis`, `backup_strategy`. Call these via `prompts/get` in the MCP protocol, not `tools/call`.

## Notes

- **Things 3 must be open** on macOS. The server reads directly from the SQLite database at `~/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-*/Containers/com.culturedcode.ThingsMac/Data/Library/Application Support/Cultured Code/Things 3/Things Database.thingsdatabase/main.sqlite`. Writes go through AppleScript — see [Mutation backend](#mutation-backend).
- Override the read database path with `THINGS_DB_PATH` env var.
- `delete_task` / `delete_project` are soft-deletes (move to Trash). There is no MCP tool to permanently purge or restore from Trash.
- For full parameter schemas and advanced configuration, see [`docs/MCP_INTEGRATION.md`](../../docs/MCP_INTEGRATION.md).
