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
elif [ -f ".git/COMMIT_EDITMSG" ]; then
    # rusty-hook behavior - read from the standard Git commit message file
    cat ".git/COMMIT_EDITMSG" | conventional_commits_linter --allow-angular-type-only -
else
    echo "Error: Cannot find commit message file" >&2
    exit 1
fi
