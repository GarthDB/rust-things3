---
name: things3-inbox-triage
description: GTD-style inbox triage for Things 3 — walks every item in the inbox, classifies it (do-now, schedule, delegate, archive, delete), and applies the user's choice via MCP write tools. Requires explicit confirmation before any destructive op. Use when the user wants to "triage", "process", or "clear" their Things 3 inbox.
---

# /things3-inbox-triage

A GTD-style inbox triage workflow over the [rust-things3](https://github.com/GarthDB/rust-things3) MCP server. Walks each inbox item with the user, applies their disposition, and confirms before deletion. Companion to `things3-daily-review` — the daily-review skill is read-only; this one writes.

## Claude Code slash command

To use `/things3-inbox-triage` as a Claude Code slash command, copy the skill to your local skills directory and patch in a `trigger` key. The canonical file omits it because `skills-ref validate` rejects unknown frontmatter fields.

```bash
cp -r skills/things3-inbox-triage ~/.claude/skills/things3-inbox-triage
python3 -c "
import pathlib, re
p = pathlib.Path('~/.claude/skills/things3-inbox-triage/SKILL.md').expanduser()
p.write_text(re.sub(r'(?m)^---\$(?=\n\n)', 'trigger: /things3-inbox-triage\n---', p.read_text(), count=1))
"
```

## Prerequisites

The `things3` foundational skill must be installed and the MCP server configured before running this workflow — see [`../things3/SKILL.md`](../things3/SKILL.md) for setup instructions.

**Backend note.** Writes route through AppleScript by default on macOS (see [`../things3/SKILL.md` → Mutation backend](../things3/SKILL.md#mutation-backend)). The first write of the session may trigger a one-time macOS Automation permission prompt; grant it and continue.

## When to use this skill

- Clearing a piled-up Things 3 inbox during a weekly review
- Processing items captured throughout the day before end-of-day shutdown
- Any time the user says "triage my inbox", "process my inbox", or "get to inbox zero"

## Recipe

### Step 1 — Pull the inbox

```json
{
  "name": "get_inbox",
  "arguments": { "limit": 50 }
}
```

If the response is empty, report "Inbox is clear" and stop. Otherwise, present a one-line count: "N items to triage."

### Step 2 — Walk each item, classify, apply

For **each** inbox item, in the order returned:

1. **Show the item.** Display `title`, `notes` (if present), and any existing `tags` so the user can decide without switching to Things 3.
2. **Ask for a disposition** — exactly one of:
   - **do-now** — do or schedule for today
   - **schedule** — defer to a future date
   - **delegate** — hand off (typically tagged `@waiting`)
   - **archive** — already done, mark complete
   - **delete** — capture mistake; remove from inbox
3. **Apply the choice** by calling the matching tool below. Confirm to the user with a one-line acknowledgement (e.g. "✓ scheduled for 2026-05-10") and move to the next item.

| Disposition | MCP call | Notes |
|---|---|---|
| **do-now** | `update_task` with `start_date` = today (or move to a project) | Optional: also `add_tag_to_task` with `today` if the user uses that convention |
| **schedule** | `update_task` with `start_date` = future date and/or `deadline` | Ask for the date if the user did not supply one |
| **delegate** | `add_tag_to_task` with `tag_title: "@waiting"` (adjust to user's convention); optionally `update_task` to append a `notes` line "delegated to NAME" | The task stays in the inbox (or is moved to a "Waiting" project if the user has one) |
| **archive** | `complete_task` | Use when the item was already finished |
| **delete** | **Confirm first** with the user (show title + ask "delete this?"), then `delete_task` | `delete_task` is a soft-delete (moves to Trash); not reversible from this skill |

**Confirmation rule for `delete_task`.** Always re-show the item title and ask for explicit "yes" before calling `delete_task`. Do not batch-confirm a list. This is the only step in this skill that requires a hard stop.

### Step 3 — Bulk shortcuts (optional)

If the user wants to fast-path a group of similar items, prefer the bulk tools over a loop:

- `bulk_complete` — archive several items at once
- `bulk_move` — relocate several items into a project or area
- `bulk_update_dates` — schedule several items for the same date
- `bulk_delete` — **only after** the user confirms the full list

Bulk calls run a single AppleScript and report per-item success/failure in the response. Re-run `get_inbox` after a bulk op to confirm what landed.

### Step 4 — Confirm completion

After the last item:

```json
{
  "name": "get_inbox",
  "arguments": { "limit": 50 }
}
```

Report the final state:
- "Inbox cleared." if empty.
- "N item(s) remaining" with the titles, if the user stopped early.

## Output format

Each item-pass produces a compact log line, suitable for streaming back to the user:

```
[1/12] "Email Q3 vendor list" — schedule → 2026-05-10
[2/12] "Pick up dry cleaning" — do-now (today)
[3/12] "Old TODO from December" — delete (confirmed)
```

Tally at the end:

```
Triage summary
  do-now:    2
  schedule:  5
  delegate:  1
  archive:   3
  delete:    1
  remaining: 0
```

## Notes

- Delete is a soft-delete — items move to the Things 3 Trash. The MCP server has no tool to permanently purge or restore from Trash; users do that in the Things 3 UI.
- For the read-only complement (today's plan, recent work, overdue), see the companion `things3-daily-review` skill.
- This skill is a recipe, not a guarantee — if any tool call fails (e.g. macOS Automation permission revoked), surface the error to the user and stop the loop rather than skipping items silently.
