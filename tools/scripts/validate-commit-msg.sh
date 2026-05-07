#!/bin/bash
# Wrapper script for conventional_commits_linter to work with rusty-hook
# 
# rusty-hook doesn't pass the commit message file as an argument like native Git hooks,
# so we read it directly from .git/COMMIT_EDITMSG

set -e  # Exit on any error

# Check if we have a commit message file
if [ -n "$1" ] && [ -f "$1" ]; then
    # Standard Git hook behavior (if rusty-hook ever fixes this)
    cat "$1" | conventional_commits_linter --allow-angular-type-only -
elif COMMIT_FILE="$(git rev-parse --git-path COMMIT_EDITMSG 2>/dev/null)" && [ -f "$COMMIT_FILE" ]; then
    # rusty-hook behavior - read from the standard Git commit message file.
    # Resolve via `git rev-parse --git-path` so this works in worktrees, where
    # `.git` is a file pointing at the real gitdir rather than a directory.
    cat "$COMMIT_FILE" | conventional_commits_linter --allow-angular-type-only -
else
    echo "Error: Cannot find commit message file" >&2
    exit 1
fi
