//! CLI Extension Example
//!
//! This example shows how to extend the Things 3 CLI with custom commands
//! and functionality. This is useful for:
//! - Adding organization-specific commands
//! - Creating custom workflows
//! - Building specialized tools on top of Things 3
//!
//! Run this example with:
//! ```bash
//! cargo run --example cli_extension
//! ```

use clap::{Parser, Subcommand};
use std::sync::Arc;
use things3_core::{ThingsDatabase, ThingsConfig};

/// Extended CLI with custom commands
#[derive(Parser)]
#[command(name = "things3-extended")]
#[command(about = "Extended Things 3 CLI with custom commands")]
struct ExtendedCli {
    /// Path to Things 3 database
    #[arg(long)]
    database: Option<String>,

    #[command(subcommand)]
    command: ExtendedCommands,
}

#[derive(Subcommand)]
enum ExtendedCommands {
    /// Standard inbox command
    Inbox {
        #[arg(long, short)]
        limit: Option<usize>,
    },
    
    /// Custom: Get overdue tasks
    Overdue {
        #[arg(long, short)]
        limit: Option<usize>,
    },
    
    /// Custom: Get high priority tasks across all projects
    HighPriority {
        #[arg(long, short)]
        limit: Option<usize>,
    },
    
    /// Custom: Generate weekly report
    WeeklyReport {
        /// Output format (text, json, markdown)
        #[arg(long, short, default_value = "text")]
        format: String,
    },
    
    /// Custom: Bulk tag operations
    BulkTag {
        /// Tag to add
        #[arg(long)]
        tag: String,
        
        /// Search query to find tasks
        #[arg(long)]
        query: String,
        
        /// Dry run (don't actually modify)
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Custom: Project health check
    ProjectHealth {
        /// Check only active projects
        #[arg(long)]
        active_only: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = ExtendedCli::parse();

    // Initialize database
    let config = if let Some(db_path) = cli.database {
        ThingsConfig::new(db_path, false)
    } else {
        ThingsConfig::from_env()
    };

    let db = ThingsDatabase::new(&config.database_path).await?;
    let db = Arc::new(db);

    // Handle commands
    match cli.command {
        ExtendedCommands::Inbox { limit } => {
            handle_inbox(Arc::clone(&db), limit).await?;
        }
        ExtendedCommands::Overdue { limit } => {
            handle_overdue(Arc::clone(&db), limit).await?;
        }
        ExtendedCommands::HighPriority { limit } => {
            handle_high_priority(Arc::clone(&db), limit).await?;
        }
        ExtendedCommands::WeeklyReport { format } => {
            handle_weekly_report(Arc::clone(&db), &format).await?;
        }
        ExtendedCommands::BulkTag { tag, query, dry_run } => {
            handle_bulk_tag(Arc::clone(&db), &tag, &query, dry_run).await?;
        }
        ExtendedCommands::ProjectHealth { active_only } => {
            handle_project_health(Arc::clone(&db), active_only).await?;
        }
    }

    Ok(())
}

/// Standard inbox command
async fn handle_inbox(
    db: Arc<ThingsDatabase>,
    limit: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📥 Inbox Tasks");
    println!("═══════════════\n");

    let tasks = db.get_inbox(limit).await?;

    for task in &tasks {
        println!("• {}", task.title);
        if let Some(notes) = &task.notes {
            println!("  Notes: {}", notes);
        }
        println!();
    }

    println!("Total: {} tasks", tasks.len());
    Ok(())
}

/// Custom: Get overdue tasks
async fn handle_overdue(
    db: Arc<ThingsDatabase>,
    limit: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("⚠️  Overdue Tasks");
    println!("═══════════════\n");

    let all_tasks = db.get_all_tasks().await?;
    let now = chrono::Utc::now().naive_utc().date();

    let mut overdue_tasks: Vec<_> = all_tasks
        .into_iter()
        .filter(|task| {
            if let Some(deadline) = task.deadline {
                deadline < now && task.status == things3_core::TaskStatus::Open
            } else {
                false
            }
        })
        .collect();

    // Sort by deadline (oldest first)
    overdue_tasks.sort_by_key(|task| task.deadline);

    let tasks_to_show = if let Some(limit) = limit {
        overdue_tasks.iter().take(limit).collect::<Vec<_>>()
    } else {
        overdue_tasks.iter().collect()
    };

    for task in &tasks_to_show {
        let days_overdue = if let Some(deadline) = task.deadline {
            (now - deadline).num_days()
        } else {
            0
        };

        println!("• {} ({}days overdue)", task.title, days_overdue);
        if let Some(deadline) = task.deadline {
            println!("  Deadline: {}", deadline);
        }
        println!();
    }

    println!("Total overdue: {} tasks", overdue_tasks.len());
    Ok(())
}

/// Custom: Get high priority tasks
async fn handle_high_priority(
    _db: Arc<ThingsDatabase>,
    _limit: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔥 High Priority Tasks");
    println!("═══════════════════\n");

    // Note: Things 3 doesn't have explicit priority levels in the database
    // This is a placeholder showing how you might implement custom logic
    println!("This is a custom command that could filter tasks by:");
    println!("  - Tags containing 'urgent' or 'important'");
    println!("  - Deadlines within next 3 days");
    println!("  - Tasks in specific 'critical' projects");
    println!("\nImplement your organization's priority logic here!");

    Ok(())
}

/// Custom: Generate weekly report
async fn handle_weekly_report(
    db: Arc<ThingsDatabase>,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Weekly Report");
    println!("═══════════════\n");

    let projects = db.get_all_projects().await?;
    let tasks = db.get_all_tasks().await?;

    let completed_tasks = tasks.iter().filter(|t| {
        t.status == things3_core::TaskStatus::Completed
    }).count();

    let active_tasks = tasks.iter().filter(|t| {
        t.status == things3_core::TaskStatus::Open
    }).count();

    let active_projects = projects.iter().filter(|p| {
        p.status == things3_core::TaskStatus::Open
    }).count();

    match format {
        "json" => {
            let report = serde_json::json!({
                "week": chrono::Utc::now().iso_week().week(),
                "year": chrono::Utc::now().year(),
                "stats": {
                    "completed_tasks": completed_tasks,
                    "active_tasks": active_tasks,
                    "active_projects": active_projects,
                    "total_tasks": tasks.len(),
                    "total_projects": projects.len(),
                }
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "markdown" => {
            println!("# Weekly Report\n");
            println!("**Week**: {} of {}\n", chrono::Utc::now().iso_week().week(), chrono::Utc::now().year());
            println!("## Statistics\n");
            println!("- ✅ Completed Tasks: {}", completed_tasks);
            println!("- 📝 Active Tasks: {}", active_tasks);
            println!("- 📂 Active Projects: {}", active_projects);
            println!("- 📊 Total Tasks: {}", tasks.len());
            println!("- 📁 Total Projects: {}", projects.len());
        }
        _ => {
            println!("Week: {} of {}", chrono::Utc::now().iso_week().week(), chrono::Utc::now().year());
            println!("\nStatistics:");
            println!("  Completed Tasks: {}", completed_tasks);
            println!("  Active Tasks: {}", active_tasks);
            println!("  Active Projects: {}", active_projects);
            println!("  Total Tasks: {}", tasks.len());
            println!("  Total Projects: {}", projects.len());
        }
    }

    Ok(())
}

/// Custom: Bulk tag operations
async fn handle_bulk_tag(
    db: Arc<ThingsDatabase>,
    tag: &str,
    query: &str,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🏷️  Bulk Tag Operation");
    println!("══════════════════\n");

    let tasks = db.search_tasks(query).await?;

    println!("Found {} tasks matching '{}'", tasks.len(), query);
    println!("Tag to add: {}", tag);
    println!("Dry run: {}\n", dry_run);

    for task in &tasks {
        println!("  • {}", task.title);
    }

    if dry_run {
        println!("\n⚠️  DRY RUN: No changes made");
        println!("Remove --dry-run flag to apply changes");
    } else {
        println!("\n✅ Tags would be added here");
        println!("Note: Tag modification requires write operations");
    }

    Ok(())
}

/// Custom: Project health check
async fn handle_project_health(
    db: Arc<ThingsDatabase>,
    active_only: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🏥 Project Health Check");
    println!("══════════════════════\n");

    let projects = if active_only {
        db.get_all_projects().await?.into_iter()
            .filter(|p| p.status == things3_core::TaskStatus::Open)
            .collect()
    } else {
        db.get_all_projects().await?
    };

    println!("Analyzing {} projects...\n", projects.len());

    for project in &projects {
        println!("📂 {}", project.title);
        
        // Check for potential issues
        let mut warnings = Vec::new();

        if project.notes.is_none() {
            warnings.push("No notes/description");
        }

        if project.deadline.is_none() {
            warnings.push("No deadline set");
        }

        if warnings.is_empty() {
            println!("   ✅ Healthy");
        } else {
            println!("   ⚠️  Warnings:");
            for warning in warnings {
                println!("      - {}", warning);
            }
        }
        println!();
    }

    Ok(())
}

/*
 * Extension Ideas:
 * 
 * 1. Time Tracking: Add custom time tracking fields
 * 2. Integrations: Sync with external tools (Notion, Todoist, etc.)
 * 3. Automation: Scheduled task creation, reminders
 * 4. Analytics: Advanced reporting and insights
 * 5. Team Features: Share tasks, assign to team members
 * 6. Custom Filters: Save and reuse complex queries
 * 7. Templates: Create tasks from templates
 * 8. Batch Operations: Mass update, archive, delete
 */

