//! Background Service Example
//!
//! This example demonstrates how to build a long-running background service
//! that monitors Things 3 data with graceful shutdown handling. This is useful for:
//! - Scheduled task processing
//! - Automated notifications
//! - Data synchronization
//! - Monitoring and alerting
//!
//! Run this example with:
//! ```bash
//! cargo run --example background_service
//! ```
//!
//! Stop with Ctrl+C for graceful shutdown.

use std::sync::Arc;
use std::time::Duration;
use things3_core::{ThingsDatabase, ThingsConfig};
use tokio::signal;
use tokio::sync::broadcast;
use tokio::time::{interval, sleep};
use tracing::{info, warn, error};

/// Service configuration
struct ServiceConfig {
    /// How often to check for tasks (in seconds)
    check_interval: u64,
    /// How often to generate reports (in seconds)
    report_interval: u64,
    /// Service name
    name: String,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            check_interval: 30,
            report_interval: 300,
            name: "things3-background-service".to_string(),
        }
    }
}

/// Background service for Things 3
struct BackgroundService {
    db: Arc<ThingsDatabase>,
    config: ServiceConfig,
    shutdown_tx: broadcast::Sender<()>,
}

impl BackgroundService {
    fn new(db: Arc<ThingsDatabase>, config: ServiceConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            db,
            config,
            shutdown_tx,
        }
    }

    /// Start the background service
    async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Starting {} service", self.config.name);

        // Create shutdown signal
        let shutdown_signal = self.setup_shutdown_signal();

        // Spawn worker tasks
        let mut handles = vec![];

        // Worker 1: Task checker (checks for overdue tasks)
        handles.push(tokio::spawn({
            let db = Arc::clone(&self.db);
            let interval_secs = self.config.check_interval;
            let mut shutdown_rx = self.shutdown_tx.subscribe();
            
            async move {
                Self::task_checker_worker(db, interval_secs, shutdown_rx).await
            }
        }));

        // Worker 2: Report generator (generates periodic reports)
        handles.push(tokio::spawn({
            let db = Arc::clone(&self.db);
            let interval_secs = self.config.report_interval;
            let mut shutdown_rx = self.shutdown_tx.subscribe();
            
            async move {
                Self::report_generator_worker(db, interval_secs, shutdown_rx).await
            }
        }));

        // Worker 3: Health monitor (monitors service health)
        handles.push(tokio::spawn({
            let db = Arc::clone(&self.db);
            let mut shutdown_rx = self.shutdown_tx.subscribe();
            
            async move {
                Self::health_monitor_worker(db, shutdown_rx).await
            }
        }));

        // Wait for shutdown signal
        shutdown_signal.await;

        info!("📡 Shutdown signal received, stopping workers...");

        // Send shutdown signal to all workers
        let _ = self.shutdown_tx.send(());

        // Wait for all workers to finish
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Worker error during shutdown: {}", e);
            }
        }

        info!("✅ Service stopped gracefully");

        Ok(())
    }

    /// Setup graceful shutdown signal handling
    async fn setup_shutdown_signal(&self) -> tokio::sync::oneshot::Receiver<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            // Wait for Ctrl+C or termination signal
            let ctrl_c = async {
                signal::ctrl_c()
                    .await
                    .expect("Failed to install Ctrl+C handler");
            };

            #[cfg(unix)]
            let terminate = async {
                signal::unix::signal(signal::unix::SignalKind::terminate())
                    .expect("Failed to install signal handler")
                    .recv()
                    .await;
            };

            #[cfg(not(unix))]
            let terminate = std::future::pending::<()>();

            tokio::select! {
                _ = ctrl_c => {
                    info!("Received Ctrl+C signal");
                }
                _ = terminate => {
                    info!("Received terminate signal");
                }
            }

            let _ = tx.send(());
        });

        rx
    }

    /// Worker: Check for overdue tasks
    async fn task_checker_worker(
        db: Arc<ThingsDatabase>,
        interval_secs: u64,
        mut shutdown: broadcast::Receiver<()>,
    ) {
        let mut interval = interval(Duration::from_secs(interval_secs));
        info!("👷 Task checker worker started (checking every {}s)", interval_secs);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = Self::check_overdue_tasks(&db).await {
                        error!("Task checker error: {}", e);
                    }
                }
                _ = shutdown.recv() => {
                    info!("Task checker worker shutting down");
                    break;
                }
            }
        }
    }

    /// Check for overdue tasks
    async fn check_overdue_tasks(db: &ThingsDatabase) -> Result<(), Box<dyn std::error::Error>> {
        let tasks = db.get_all_tasks().await?;
        let now = chrono::Utc::now().naive_utc().date();

        let overdue_count = tasks
            .iter()
            .filter(|task| {
                if let Some(deadline) = task.deadline {
                    deadline < now && task.status == things3_core::TaskStatus::Incomplete
                } else {
                    false
                }
            })
            .count();

        if overdue_count > 0 {
            warn!("⚠️  Found {} overdue tasks", overdue_count);
            // In production: Send notifications, update dashboard, etc.
        } else {
            info!("✅ No overdue tasks found");
        }

        Ok(())
    }

    /// Worker: Generate periodic reports
    async fn report_generator_worker(
        db: Arc<ThingsDatabase>,
        interval_secs: u64,
        mut shutdown: broadcast::Receiver<()>,
    ) {
        let mut interval = interval(Duration::from_secs(interval_secs));
        info!("📊 Report generator worker started (generating every {}s)", interval_secs);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = Self::generate_report(&db).await {
                        error!("Report generator error: {}", e);
                    }
                }
                _ = shutdown.recv() => {
                    info!("Report generator worker shutting down");
                    break;
                }
            }
        }
    }

    /// Generate periodic report
    async fn generate_report(db: &ThingsDatabase) -> Result<(), Box<dyn std::error::Error>> {
        let tasks = db.get_all_tasks().await?;
        let projects = db.get_all_projects().await?;

        let completed_today = tasks
            .iter()
            .filter(|t| {
                t.status == things3_core::TaskStatus::Completed &&
                t.modified.date_naive() == chrono::Utc::now().date_naive()
            })
            .count();

        let active_tasks = tasks.iter().filter(|t| t.status == things3_core::TaskStatus::Incomplete).count();
        let active_projects = projects.iter().filter(|p| p.status == things3_core::TaskStatus::Incomplete).count();

        info!("📈 Report: {} tasks completed today, {} active tasks, {} active projects",
            completed_today, active_tasks, active_projects);

        Ok(())
    }

    /// Worker: Monitor service health
    async fn health_monitor_worker(
        db: Arc<ThingsDatabase>,
        mut shutdown: broadcast::Receiver<()>,
    ) {
        let mut interval = interval(Duration::from_secs(60));
        info!("🏥 Health monitor worker started");

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = Self::check_health(&db).await {
                        error!("Health check failed: {}", e);
                    }
                }
                _ = shutdown.recv() => {
                    info!("Health monitor worker shutting down");
                    break;
                }
            }
        }
    }

    /// Perform health check
    async fn check_health(db: &ThingsDatabase) -> Result<(), Box<dyn std::error::Error>> {
        // Simple health check: try to query database
        let _ = db.get_stats().await?;
        info!("💚 Health check passed");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .with_ansi(true)
        .init();

    info!("🔧 Initializing Things 3 Background Service");

    // Load configuration
    let db_config = ThingsConfig::from_env();
    let service_config = ServiceConfig::default();

    // Connect to database
    info!("📦 Connecting to database: {}", db_config.database_path.display());
    let db = ThingsDatabase::new(&db_config.database_path).await?;
    let db = Arc::new(db);

    // Create and start service
    let service = BackgroundService::new(Arc::clone(&db), service_config);
    
    info!("✨ Service initialized successfully");
    info!("Press Ctrl+C to stop the service\n");

    // Run service (blocks until shutdown)
    service.start().await?;

    // Give a brief moment for cleanup
    sleep(Duration::from_millis(100)).await;

    Ok(())
}

/*
 * Production Enhancements:
 * 
 * 1. Configuration: Load from file or environment
 * 2. Logging: Add file-based logging with rotation
 * 3. Metrics: Export Prometheus metrics
 * 4. Error Recovery: Implement retry logic with exponential backoff
 * 5. State Management: Persist service state across restarts
 * 6. Notifications: Add email/Slack/webhook notifications
 * 7. API Server: Add HTTP API for service control
 * 8. Hot Reload: Support configuration hot reload
 * 9. Multi-tenancy: Support multiple databases
 * 10. Distributed: Use message queue for distributed processing
 */

/*
 * Deployment Examples:
 * 
 * // Systemd service (Linux)
 * [Unit]
 * Description=Things 3 Background Service
 * After=network.target
 * 
 * [Service]
 * Type=simple
 * User=things3
 * WorkingDirectory=/opt/things3
 * ExecStart=/opt/things3/background_service
 * Restart=always
 * RestartSec=10
 * 
 * [Install]
 * WantedBy=multi-user.target
 * 
 * // Docker
 * FROM rust:1.70 as builder
 * WORKDIR /app
 * COPY . .
 * RUN cargo build --release --example background_service
 * 
 * FROM debian:bookworm-slim
 * COPY --from=builder /app/target/release/examples/background_service /usr/local/bin/
 * CMD ["background_service"]
 * 
 * // Docker Compose
 * services:
 *   things3-service:
 *     image: things3-background-service
 *     restart: unless-stopped
 *     environment:
 *       - THINGS_DB_PATH=/data/things.db
 *     volumes:
 *       - ./data:/data
 */

