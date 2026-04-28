---
name: things3-daily-review
description: Daily review workflow for Things 3 — pulls today's tasks, inbox, and recent work; produces a structured Markdown summary grouped by area and project with overdue items flagged. Use for morning standups, end-of-day wrap-ups, or weekly reviews.
---

# /things3-daily-review

A read-only workflow skill that drives a daily review over your Things 3 database via the [rust-things3](https://github.com/GarthDB/rust-things3) MCP server. It pulls three data sets, groups them by area and project, and flags any overdue items.

## Claude Code slash command

To register `/things3-daily-review` as a slash command in Claude Code, add a `trigger` key to the frontmatter:

```yaml
trigger: /things3-daily-review
```

This is a Claude Code extension field. The agentskills.io spec validator (`skills-ref validate`) currently flags it as unknown and will fail; add it only to your local working copy, not to the canonical `SKILL.md` in the repository.

## Prerequisites

The `things3` foundational skill must be installed and the MCP server configured before running this workflow — see [`../things3/SKILL.md`](../things3/SKILL.md) for setup instructions.

## When to use this skill

- Morning standup prep: see what's due today and what's sitting in your inbox
- End-of-day wrap-up: review recent work and surface anything overdue
- Weekly review: recent completions grouped by area and project

## Recipe

Run the following three tool calls in order, then render the output as described below.

### Step 1 — Today's tasks

```json
{
  "name": "get_today",
  "arguments": { "limit": 50 }
}
```

Returns tasks scheduled for today. Tasks with a `deadline` earlier than today are overdue.

### Step 2 — Inbox

```json
{
  "name": "get_inbox",
  "arguments": { "limit": 50 }
}
```

Returns unscheduled, uncategorised tasks awaiting triage.

### Step 3 — Recent tasks

```json
{
  "name": "get_recent_tasks",
  "arguments": { "limit": 20, "hours": 24 }
}
```

Returns recently modified tasks from the last `hours` window. To target **completed** tasks specifically, use `logbook_search` instead:

```json
{
  "name": "logbook_search",
  "arguments": {
    "from_date": "<YYYY-MM-DD>",
    "to_date": "<YYYY-MM-DD>",
    "limit": 20
  }
}
```

## Output format

Produce a Markdown summary with these three sections, grouped by area then project. Omit empty sections.

```markdown
## Daily Review — YYYY-MM-DD

### Today (N)

**Area Name**
- _Project Name_
  - **OVERDUE** Task title (deadline: YYYY-MM-DD)
  - Task title
- _No project_
  - Task title

### Inbox (N)

- Task title
- Task title

### Recent (N)

**Area Name**
- _Project Name_
  - Task title
- _No project_
  - Task title
```

## Overdue flagging

A task is overdue when its `deadline` field is set and `deadline < today`. Mark it inline with bold **OVERDUE** and include the deadline date:

```
- **OVERDUE** Review Q3 budget (deadline: 2026-04-20)
```

Flag overdue items in both the **Today** and **Recent** sections wherever they appear.

## Notes

- This skill is read-only — it makes no mutations. For processing inbox items, see the companion `things3-inbox-triage` skill (coming soon).
- Things 3 must be running on macOS for the MCP server to reach its database.
- Tasks without an `area_uuid` or `project_uuid` fall under a _No area_ or _No project_ group respectively.
