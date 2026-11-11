# Mac Automation Integration Examples

This guide shows how to integrate `things3` with popular macOS automation tools so you can manage your Things 3 tasks without leaving your workflow.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Apple Shortcuts](#apple-shortcuts)
- [Automator](#automator)
- [Raycast](#raycast)
- [Alfred](#alfred)
- [Hazel](#hazel)
- [Tips and Troubleshooting](#tips-and-troubleshooting)

---

## Prerequisites

**Install `things3`** (if you haven't already):

```bash
# Homebrew (recommended)
brew install garthdb/tap/things3

# Or with Cargo
cargo install --git https://github.com/GarthDB/rust-things3
```

**Find the binary path** — you'll need this for automation tools:

```bash
which things3
# → /opt/homebrew/bin/things3  (Apple Silicon)
# → /usr/local/bin/things3     (Intel)
# → ~/.cargo/bin/things3       (Cargo install)
```

**Verify it works**:

```bash
RUST_LOG=off tasklink inbox | head -5
```

> **Note:** `things3` currently writes log output to stdout alongside JSON results. Set `RUST_LOG=off` in scripts to suppress log output and get clean JSON.

---

## Apple Shortcuts

Shortcuts is Apple's built-in automation tool. The **Run Shell Script** action lets you call `things3` directly or open the [Things URL scheme](https://culturedcode.com/things/support/articles/2803573/) to create tasks.

### Example: Add Task from Clipboard

This shortcut reads clipboard text and creates a Things 3 task from it.

**Setup**:

1. Open the Shortcuts app and create a new shortcut.
2. Add action: **Scripting › Run Shell Script**.
3. Set **Shell** to `/bin/bash`.
4. Set **Input** to **Shortcut Input** (so clipboard text arrives via stdin).
5. Paste the script below into the script field.

**Script** (`examples/integration/mac/shortcuts/add_task_from_clipboard.sh`):

```bash
#!/bin/bash
set -euo pipefail

TITLE=$(cat)

if [ -z "$TITLE" ]; then
    echo "No text on clipboard." >&2
    exit 1
fi

FIRST_LINE=$(echo "$TITLE" | head -1)
REMAINING=$(echo "$TITLE" | tail -n +2)

encode() {
    python3 -c "import urllib.parse, sys; print(urllib.parse.quote(sys.argv[1]))" "$1"
}

URL="things:///add?title=$(encode "$FIRST_LINE")"

if [ -n "$REMAINING" ]; then
    URL="${URL}&notes=$(encode "$REMAINING")"
fi

open "$URL"
echo "Task created: $FIRST_LINE"
```

**Use cases**:
- Copy a URL or article title → add it as a reading task.
- Copy a meeting note → create a follow-up task with notes.
- Trigger from iOS via iCloud Shortcuts.

### Example: Show Inbox Count (Menu Bar Widget)

Add a **Run Shell Script** action followed by **Show Result** to display your inbox count:

```bash
#!/bin/bash
export RUST_LOG=off
THINGS3=$(command -v things3 || echo "${HOME}/.cargo/bin/things3")
COUNT=$("$THINGS3" inbox | python3 -c "import json,sys; print(len(json.load(sys.stdin)))")
echo "Inbox: ${COUNT} task(s)"
```

---

## Automator

Automator's **Run Shell Script** action works the same way. Use it to build folder actions, calendar alarms, or application workflows.

### Example: Morning Review Notification

Fetches your inbox and shows a macOS notification at login or on a schedule.

**Setup**:

1. Open Automator and create a new **Workflow** (or **Calendar Alarm** for scheduling).
2. Search for and add **Utilities › Run Shell Script**.
3. Set **Shell** to `/bin/bash`, **Pass input** to `ignore`.
4. Paste the script below.

**Script** (`examples/integration/mac/automator/morning_review.sh`):

```bash
#!/bin/bash
set -euo pipefail
export RUST_LOG=off

THINGS3=$(command -v things3 || echo "${HOME}/.cargo/bin/things3")
INBOX_JSON=$("$THINGS3" inbox)
COUNT=$(echo "$INBOX_JSON" | python3 -c "import json,sys; print(len(json.load(sys.stdin)))" || echo 0)

osascript -e "display notification \"Inbox: ${COUNT} task(s)\" with title \"Things 3 Morning Review\" sound name \"Glass\""

echo "Inbox: ${COUNT} task(s)"

if [ "$COUNT" -gt 0 ]; then
    echo ""
    echo "$INBOX_JSON" | python3 -c "
import json, sys
for t in json.load(sys.stdin)[:10]:
    print(f'  • {t[\"title\"]}')
"
fi
```

**Run on a schedule**: Save the Automator workflow as a **Calendar Alarm** and set a daily repeating event in Calendar.

**Use cases**:
- Daily inbox review notification at 9 AM.
- Folder action: watch a "To Process" folder and create tasks for new files.
- Login item: show task summary when you start your Mac.

---

## Raycast

Raycast Script Commands are shell scripts with special `@raycast.*` comment headers. Raycast discovers them automatically from a configured scripts directory.

### Example: Search Things 3

**Setup**:

1. In Raycast, open **Extensions › + › Add Script Directory**.
2. Choose (or create) a folder, e.g. `~/Documents/Raycast Scripts/`.
3. Copy `examples/integration/mac/raycast/things3_search.sh` to that folder.
4. Make it executable: `chmod +x things3_search.sh`.
5. Raycast will detect it automatically. Search **"Search Things 3"** to run it.

**Script** (`examples/integration/mac/raycast/things3_search.sh`):

```bash
#!/bin/bash

# Required parameters:
# @raycast.schemaVersion 1
# @raycast.title Search Things 3
# @raycast.mode fullOutput

# Optional parameters:
# @raycast.icon 🎯
# @raycast.packageName Things 3
# @raycast.description Search your Things 3 tasks
# @raycast.argument1 { "type": "text", "placeholder": "Search query" }

set -euo pipefail
export RUST_LOG=off

THINGS3=$(command -v things3 || echo "${HOME}/.cargo/bin/things3")
QUERY="${1:-}"

if [ -z "$QUERY" ]; then
    echo "Please provide a search query."
    exit 0
fi

RESULTS=$("$THINGS3" search "$QUERY")

if [ -z "$RESULTS" ] || [ "$RESULTS" = "[]" ]; then
    echo "No tasks found for: $QUERY"
    exit 0
fi

echo "$RESULTS" | python3 -c "
import json, sys
tasks = json.load(sys.stdin)
for t in tasks:
    icon = '✅' if t['status'] == 'completed' else '⬜'
    print(f\"{icon} {t['title']}\")
    if t.get('notes'):
        print(f\"   {t['notes'][:100]}\")
print()
print(f'Found {len(tasks)} task(s)')
"
```

**Use cases**:
- Quick-search tasks without leaving your current window.
- Find tasks by project name or keyword.
- Assign a keyboard shortcut to the script in Raycast for instant access.

---

## Alfred

Alfred Script Filters output a specific JSON format that Alfred uses to render a searchable list. Selecting a result can open the task in Things 3, copy its UUID, or trigger other actions.

### Example: Script Filter (Search Tasks)

**Setup**:

1. Open **Alfred Preferences › Workflows › + › Blank Workflow**.
2. Right-click the canvas: **Inputs › Script Filter**.
3. Configure:
   - **Language**: `/bin/bash`
   - **Script**: paste the script below
   - **Keyword**: `t3` (or your choice)
   - **Argument**: Required
   - **with input as argv** checked
4. Add an output action. For example, **Actions › Open URL** with URL `things:///show?id={query}` to open the selected task in Things 3. Connect the Script Filter to it.

**Script** (`examples/integration/mac/alfred/script_filter.sh`):

```bash
#!/bin/bash
set -euo pipefail
export RUST_LOG=off

THINGS3=$(command -v things3 || echo "${HOME}/.cargo/bin/things3")
QUERY="${1:-}"

if [ ! -x "$THINGS3" ]; then
    echo '{"items":[{"title":"things3 not found","subtitle":"brew install garthdb/tap/things3","valid":false}]}'
    exit 0
fi

if [ -z "$QUERY" ]; then
    echo '{"items":[{"title":"Search Things 3","subtitle":"Type to search your tasks","valid":false}]}'
    exit 0
fi

RESULTS=$("$THINGS3" search "$QUERY")

if [ -z "$RESULTS" ] || [ "$RESULTS" = "[]" ]; then
    echo "$QUERY" | python3 -c "
import json, sys
q = sys.stdin.read().strip()
print(json.dumps({'items': [{'title': f'No results for: {q}', 'valid': False}]}))
"
    exit 0
fi

echo "$RESULTS" | python3 -c "
import json, sys
tasks = json.load(sys.stdin)
items = []
for t in tasks:
    icon = '✅ ' if t['status'] == 'completed' else ''
    items.append({
        'uid': t['uuid'],
        'title': icon + t['title'],
        'subtitle': (t.get('notes') or t['status'])[:80],
        'arg': t['uuid'],
        'autocomplete': t['title'],
    })
print(json.dumps({'items': items}))
"
```

**Validate the output**:

```bash
echo '{"items":[]}' | python3 -m json.tool  # reference shape
./script_filter.sh "meeting" | python3 -m json.tool
```

**Use cases**:
- `t3 invoice` → see all invoice-related tasks → press Enter to open in Things 3.
- Chain with a **Copy to Clipboard** action to grab a task UUID for scripting.
- Add a **Run Script** action to mark the selected task complete via Things URL scheme.

---

## Hazel

[Hazel](https://www.noodlesoft.com/) watches folders and runs actions when files match rules. The **Run Shell Script** action receives the matched file as `$1`.

### Example: Create Task for Matched File

This example creates a Things 3 task whenever Hazel matches a file (e.g., a receipt PDF arriving in Downloads).

**Setup**:

1. Open Hazel and add a folder to watch (e.g., `~/Downloads`).
2. Add a rule with your conditions:
   - **Kind is PDF** and **Name contains "receipt"** (adjust as needed).
3. Add action: **Run Shell Script**.
4. Set to **Embedded Script**, Shell: `/bin/bash`.
5. Paste the script below.

**Script** (`examples/integration/mac/hazel/file_to_task.sh`):

```bash
#!/bin/bash
set -euo pipefail

FILE_PATH="${1:-}"
FILE_NAME=$(basename "$FILE_PATH")
DATE=$(date +"%Y-%m-%d")

TITLE="Process: ${FILE_NAME}"
NOTES="File: ${FILE_PATH}\nMatched on: ${DATE}"

encode() {
    python3 -c "import urllib.parse, sys; print(urllib.parse.quote(sys.argv[1]))" "$1"
}

URL="things:///add?title=$(encode "$TITLE")&notes=$(encode "$(printf '%b' "$NOTES")")&tags=hazel"
open "$URL"

echo "Created task: $TITLE"
```

**Use cases**:
- `~/Downloads` PDF → create an "expense: process receipt" task.
- New file in `~/Desktop` → create a "clean up" task after 24 hours (combine Hazel's date condition with this script).
- Completed export file → create a "review and archive" task with the file path in notes.

---

## Tips and Troubleshooting

### Binary not found

If `things3` isn't in your shell's PATH, automation tools may not find it. Use an absolute path:

```bash
THINGS3="${HOME}/.cargo/bin/things3"  # Cargo install
# or
THINGS3="/opt/homebrew/bin/things3"   # Homebrew on Apple Silicon
# or
THINGS3="/usr/local/bin/things3"      # Homebrew on Intel
```

Or detect it dynamically:

```bash
THINGS3=$(command -v things3 || echo "${HOME}/.cargo/bin/things3")
```

### Log output mixing with JSON

`things3` currently writes log output to stdout alongside JSON. Set `RUST_LOG=off` to suppress it:

```bash
RUST_LOG=off tasklink search "query"
# or in a script:
export RUST_LOG=off
tasklink search "query"
```

### Things URL scheme reference

All examples use the [Things URL scheme](https://culturedcode.com/things/support/articles/2803573/) to create tasks (`things:///add`). Supported parameters include:

| Parameter  | Description                          | Example                     |
|------------|--------------------------------------|-----------------------------|
| `title`    | Task title (URL-encoded)             | `My%20Task`                 |
| `notes`    | Task notes (URL-encoded)             | `See%20file%3A%20...`       |
| `tags`     | Comma-separated tags (URL-encoded)   | `hazel%2Cauto`              |
| `deadline` | Due date (`YYYY-MM-DD`)              | `2026-01-31`                |
| `list`     | Project or area title (URL-encoded)  | `Work`                      |
| `when`     | Schedule (`today`, `tonight`, etc.)  | `today`                     |

### JSON output structure

Commands like `tasklink search` and `tasklink inbox` return a JSON array of task objects:

```json
[
  {
    "uuid": "H9pNZv1gX6FDqeujkPtupe",
    "title": "Review quarterly report",
    "task_type": "to-do",
    "status": "incomplete",
    "notes": "See shared drive",
    "start_date": null,
    "deadline": "2026-01-31T00:00:00Z",
    "created": "2026-01-10T09:00:00Z",
    "modified": "2026-01-12T14:30:00Z",
    "stop_date": null,
    "project_uuid": "abc123",
    "area_uuid": null,
    "parent_uuid": null,
    "tags": ["urgent"],
    "children": []
  }
]
```

Use `python3 -c "import json,sys; ..."` to parse it in shell scripts without additional dependencies.

---

**Related documentation**:
- [MCP Integration](./mcp-integration.md) — AI/LLM editor integrations (Cursor, VS Code, Zed)
- [Things URL scheme](https://culturedcode.com/things/support/articles/2803573/) — Official Cultured Code docs
