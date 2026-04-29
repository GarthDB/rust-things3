# Agent Skills

Reusable skill files for driving Things 3 from AI agents via the rust-things3 MCP server. Skills conform to the [agentskills.io specification](https://agentskills.io/specification).

## Available skills

| Skill | Description |
|-------|-------------|
| [`things3`](things3/SKILL.md) | Foundational skill — MCP setup and full tool catalog. Use this first; other skills depend on it for setup. |
| [`things3-daily-review`](things3-daily-review/SKILL.md) | Read-only daily review workflow. Pulls today's tasks, inbox, and recent work; produces a structured Markdown summary grouped by area and project with overdue items flagged. |

## Install instructions

Skills are loaded from a local directory that your AI host watches. The steps below put the skill files in the right place for each supported host.

### Claude Code

```bash
# Copy both skills to your Claude Code skills directory
cp -r skills/things3 ~/.claude/skills/things3
cp -r skills/things3-daily-review ~/.claude/skills/things3-daily-review
```

Then add `trigger` keys so the skills are available as slash commands:

```bash
for skill in things3 things3-daily-review; do
  python3 -c "
import pathlib, re
p = pathlib.Path('~/.claude/skills/$skill/SKILL.md').expanduser()
p.write_text(re.sub(r'(?m)^---$(?=\n\n)', 'trigger: /$skill\n---', p.read_text(), count=1))
"
done
```

Use with `/things3` and `/things3-daily-review` in Claude Code.

### Claude Desktop, Cursor, Zed

Check your host's documentation for the skills directory path, then copy the skill directories there:

```bash
cp -r skills/things3 /path/to/host/skills/things3
cp -r skills/things3-daily-review /path/to/host/skills/things3-daily-review
```

## Spec compliance

Skills are validated with [`skills-ref`](https://pypi.org/project/skills-ref/):

```bash
pip install skills-ref
skills-ref validate skills/things3
skills-ref validate skills/things3-daily-review
```

CI runs this automatically on every PR that touches `skills/`.
