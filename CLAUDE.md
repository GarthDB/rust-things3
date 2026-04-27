# rust-things3 — Claude Code Notes

## Code Intelligence (ferrograph)

`ferrograph index .` builds `.ferrograph` (gitignored). Use before refactors:

```bash
ferrograph status .                          # node/edge overview
ferrograph callers <node_id> -d .ferrograph  # who calls this?
ferrograph blast <node_id> -d .ferrograph    # what breaks if this changes?
ferrograph dead -d .ferrograph               # unreachable code
```

Run `ferrograph search "name" -d .ferrograph` to find node IDs by symbol name.
Particularly important before touching shared types like `TaskFilters`.

## SemVer constraint — do NOT modify `TaskFilters`

`TaskFilters` (in `libs/things3-core/src/models.rs`) is a stable public struct since
1.0.0 with no `#[non_exhaustive]`. Adding public fields is a breaking change. All
new filter capabilities must live as **private fields on `TaskQueryBuilder`** only.

## Builder-only pattern for new filters

All new query predicates follow this shape:

1. Private field on `TaskQueryBuilder`, gated: `#[cfg(feature = "advanced-queries")]`
2. Public builder method, also gated
3. Applied in `execute()` after `query_tasks()` returns, never in `build()` / `TaskFilters`
4. When any post-filter is active, `execute()` strips `limit`/`offset` before the DB call
   and re-applies pagination in Rust after filtering

## Feature flag: `advanced-queries`

Gates all query execution APIs: `query_tasks()`, `TaskQueryBuilder::execute()`, and
every builder-only predicate. Unit tests that touch gated fields must be gated too.

```bash
cargo test -p things3-core --lib                              # without feature
cargo test -p things3-core --features advanced-queries --lib  # with feature
cargo clippy -p things3-core --lib --features advanced-queries -- -D warnings
```
