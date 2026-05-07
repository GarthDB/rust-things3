use anyhow::Result;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn setup_git_hooks() -> Result<()> {
    println!("🔧 Setting up git hooks...");

    // Create .git/hooks directory if it doesn't exist
    let hooks_dir = Path::new(".git/hooks");
    if !hooks_dir.exists() {
        fs::create_dir_all(hooks_dir)?;
    }

    // Create pre-commit hook
    let pre_commit_hook = r#"#!/bin/bash
# Pre-commit hook for Rust Things project

echo "🔍 Running pre-commit checks..."

# Format code
echo "📝 Formatting code..."
cargo fmt --all
if [ $? -ne 0 ]; then
    echo "❌ Code formatting failed"
    exit 1
fi

# Run clippy with pedantic lints
echo "🔍 Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic -A clippy::missing_docs_in_private_items -A clippy::module_name_repetitions
if [ $? -ne 0 ]; then
    echo "❌ Clippy checks failed"
    exit 1
fi

# Run tests
echo "🧪 Running tests..."
cargo test --all-features
if [ $? -ne 0 ]; then
    echo "❌ Tests failed"
    exit 1
fi

echo "✅ All pre-commit checks passed!"
"#;

    let pre_commit_path = hooks_dir.join("pre-commit");
    fs::write(&pre_commit_path, pre_commit_hook)?;

    // Make the hook executable
    let mut perms = fs::metadata(&pre_commit_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&pre_commit_path, perms)?;

    // Create pre-push hook
    let pre_push_hook = r#"#!/bin/bash
# Pre-push hook for Rust Things project

echo "🔍 Running pre-push checks..."

# Run clippy with pedantic lints
echo "🔍 Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic -A clippy::missing_docs_in_private_items -A clippy::module_name_repetitions
if [ $? -ne 0 ]; then
    echo "❌ Clippy checks failed"
    exit 1
fi

# Run tests
echo "🧪 Running tests..."
cargo test --all-features
if [ $? -ne 0 ]; then
    echo "❌ Tests failed"
    exit 1
fi

echo "✅ All pre-push checks passed!"
"#;

    let pre_push_path = hooks_dir.join("pre-push");
    fs::write(&pre_push_path, pre_push_hook)?;

    // Make the hook executable
    let mut perms = fs::metadata(&pre_push_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&pre_push_path, perms)?;

    println!("✅ Git hooks installed successfully!");
    println!("📝 Pre-commit hook: .git/hooks/pre-commit");
    println!("📝 Pre-push hook: .git/hooks/pre-push");
    println!();
    println!("The hooks will run:");
    println!("  • cargo fmt --all");
    println!("  • cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic");
    println!("  • cargo test --all-features");

    Ok(())
}
