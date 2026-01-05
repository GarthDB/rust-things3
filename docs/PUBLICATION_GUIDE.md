# crates.io Publication Guide

This guide outlines the process for publishing the `things3-core`, `things3-common`, and `things3-cli` crates to crates.io.

## Prerequisites

1. **crates.io Account**: Create an account at https://crates.io if you don't have one
2. **API Token**: Get your API token from https://crates.io/me
3. **Configure Cargo**: Run `cargo login <your-api-token>`

## Pre-Publication Checklist

- [x] LICENSE file created (MIT)
- [x] Cargo.toml metadata updated (keywords, categories, readme, homepage, documentation)
- [x] Documentation builds successfully (`cargo doc --workspace`)
- [x] All tests pass (`cargo test --workspace`)
- [x] Code compiles without warnings (`cargo check --workspace`)
- [x] API stability documented (`docs/API_STABILITY.md`)
- [ ] All changes committed to git
- [ ] Version numbers are correct in Cargo.toml

## Publication Order

Publish crates in dependency order:

1. **things3-common** (no workspace dependencies)
2. **things3-core** (depends on things3-common)
3. **things3-cli** (depends on both)

## Publication Steps

### Step 1: Commit All Changes

```bash
git add .
git commit -m "chore: prepare for crates.io publication

- Add LICENSE file
- Update Cargo.toml metadata (keywords, categories, readme, etc.)
- Enhance module-level documentation
- Create API stability policy document"
```

### Step 2: Publish things3-common

```bash
cd libs/things3-common
cargo publish --dry-run  # Verify package
cargo publish             # Actual publication
```

### Step 3: Publish things3-core

```bash
cd libs/things3-core
cargo publish --dry-run  # Verify package
cargo publish             # Actual publication
```

### Step 4: Publish things3-cli

```bash
cd apps/things3-cli
cargo publish --dry-run  # Verify package
cargo publish             # Actual publication
```

## Verification

After publication, verify each crate:

1. Visit https://crates.io/crates/things3-common
2. Visit https://crates.io/crates/things3-core
3. Visit https://crates.io/crates/things3-cli

Test installation:

```bash
cargo install things3-cli
things3 --version
```

## Updating Published Crates

To publish updates:

1. Update version in `Cargo.toml` (following SemVer)
2. Update `CHANGELOG.md`
3. Commit changes
4. Tag the release: `git tag v0.2.1`
5. Publish in dependency order (same as above)

## Troubleshooting

### Error: "crate already exists"

If you see this warning, it means the crate version already exists on crates.io. You need to:
- Bump the version number
- Update CHANGELOG.md
- Commit and publish the new version

### Error: "uncommitted changes"

Either:
- Commit your changes: `git add . && git commit -m "message"`
- Or use `--allow-dirty` flag (not recommended for actual publication)

### Error: "dependency version requirement"

For workspace dependencies, cargo automatically converts path dependencies to version dependencies during publication. Ensure:
- Workspace version is set correctly in root `Cargo.toml`
- All crates use `version.workspace = true`

## Post-Publication

1. Update README.md badges to reflect actual crates.io links
2. Update documentation links
3. Announce the release (optional)

## Notes

- **Dry-run first**: Always run `cargo publish --dry-run` before actual publication
- **Version numbers**: Follow SemVer strictly - once published, versions cannot be changed
- **Yanking**: If you need to remove a broken version, use `cargo yank --version 0.2.0`
- **Documentation**: Documentation is automatically published to docs.rs after crate publication

