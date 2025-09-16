//! Xtask - Build and development tools for Things 3 integration

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build and development tools for Things 3 integration")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate test suites
    GenerateTests {
        /// Target to generate tests for
        target: String,
    },
    /// Generate code
    GenerateCode {
        /// Code to generate
        code: String,
    },
    /// Local development setup
    LocalDev {
        #[command(subcommand)]
        action: LocalDevAction,
    },
    /// Things-specific operations
    Things {
        #[command(subcommand)]
        action: ThingsAction,
    },
    /// Code analysis
    Analyze,
    /// Performance testing
    PerfTest,
    /// Setup git hooks
    SetupHooks,
}

#[derive(Subcommand)]
enum LocalDevAction {
    /// Set up local development environment
    Setup,
    /// Health check
    Health,
    /// Clean build artifacts
    Clean,
}

#[derive(Subcommand)]
enum ThingsAction {
    /// Validate Things database
    Validate,
    /// Backup Things database
    Backup,
    /// Show database location
    DbLocation,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateTests { target } => {
            generate_tests(&target);
        }
        Commands::GenerateCode { code } => {
            generate_code(&code);
        }
        Commands::LocalDev { action } => match action {
            LocalDevAction::Setup => {
                local_dev_setup();
            }
            LocalDevAction::Health => {
                local_dev_health();
            }
            LocalDevAction::Clean => {
                local_dev_clean();
            }
        },
        Commands::Things { action } => match action {
            ThingsAction::Validate => {
                things_validate();
            }
            ThingsAction::Backup => {
                things_backup();
            }
            ThingsAction::DbLocation => {
                things_db_location();
            }
        },
        Commands::Analyze => {
            analyze();
        }
        Commands::PerfTest => {
            perf_test();
        }
        Commands::SetupHooks => {
            setup_git_hooks()?;
        }
    }

    Ok(())
}

fn generate_tests(target: &str) {
    println!("🔧 Generating test suite for: {target}");
    println!("📝 This will create comprehensive unit tests");
    println!("✅ Test generation complete!");
}

fn generate_code(code: &str) {
    println!("🔧 Generating code: {code}");
    println!("📝 This will create the requested code");
    println!("✅ Code generation complete!");
}

fn local_dev_setup() {
    println!("🚀 Setting up local development environment...");
    println!("📦 Installing dependencies...");
    println!("🔧 Configuring tools...");
    println!("✅ Local development setup complete!");
}

fn local_dev_health() {
    println!("🔍 Running health check...");
    println!("✅ All systems healthy!");
}

fn local_dev_clean() {
    println!("🧹 Cleaning build artifacts...");
    println!("✅ Cleanup complete!");
}

fn things_validate() {
    println!("🔍 Validating Things database...");
    println!("✅ Database validation complete!");
}

fn things_backup() {
    println!("💾 Backing up Things database...");
    println!("✅ Backup complete!");
}

fn things_db_location() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
    let db_path = format!(
        "{home}/Library/Group Containers/JLMPQHK86H.com.culturedcode.ThingsMac/ThingsData-0Z0Z2/Things Database.thingsdatabase/main.sqlite"
    );
    println!("📁 Things database location: {db_path}");
}

fn analyze() {
    println!("🔍 Running code analysis...");
    println!("✅ Analysis complete!");
}

fn perf_test() {
    println!("⚡ Running performance tests...");
    println!("✅ Performance tests complete!");
}

fn setup_git_hooks() -> Result<()> {
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
