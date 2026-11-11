# Spike #228: .app bundle vs bare binary permissions

## Setup

```bash
cargo build --release -p things3-cli            # binary name is tasklink after #224
SIGN_ID="Developer ID Application: Blacksmith Industries, LLC (YR2436ED8U)" \
  scripts/spike-app-bundle.sh target/release/tasklink
```

Uses a fresh bundle ID, so it starts with no grants. Reset any time with the `tccutil` line the script prints.
Also run the bare `target/release/tasklink` as a control.

## Procedure (for each of: bundle, bare binary)

1. Reset grants. Remove any Tasklink/tasklink entry from Full Disk Access and Automation.
2. Configure Claude Desktop to launch the bundle: command `<path>/Tasklink.app/Contents/MacOS/tasklink`, args `["mcp"]`. (Optionally also test via `.mcpb`.)
3. Restart Claude Desktop and call `get_inbox`. Record the result.
4. Call a write tool (create a to-do). Record the prompt.
5. Open System Settings > Privacy & Security and record where the entry appears.

## Record

| Question | Bundle | Bare binary |
|---|---|---|
| Prompt on first DB read? Wording / name shown | | |
| Read fails with code 14? | | |
| Full Disk Access needed? | | |
| Entry in FDA list: name, icon, easy to add? | | |
| Automation prompt names (Tasklink / tasklink / Claude)? | | |
| Shows our usage string? | | |
| Process credited when launched by Claude Desktop | | |
| Grants survive Claude Desktop restart? | | |

## Decision

Ship the bundle? Exact permission instructions for users:
