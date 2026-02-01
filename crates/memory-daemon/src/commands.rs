//! Command implementations for the memory daemon.
//!
//! Handles:
//! - start: Load config, open storage, start gRPC server with scheduler
//! - stop: Signal running daemon to stop (via PID file)
//! - status: Check if daemon is running

use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::TimeZone;
use tokio::signal;
use tracing::{info, warn};

use memory_client::MemoryClient;
use memory_scheduler::{
    create_compaction_job, create_rollup_jobs, CompactionJobConfig, RollupJobConfig,
    SchedulerConfig, SchedulerService,
};
use memory_service::pb::{
    GetSchedulerStatusRequest, JobResultStatus, PauseJobRequest, ResumeJobRequest,
    TocLevel as ProtoTocLevel,
};
use memory_service::run_server_with_scheduler;
use memory_storage::Storage;
use memory_toc::summarizer::MockSummarizer;
use memory_types::Settings;

use crate::cli::{AdminCommands, QueryCommands, SchedulerCommands};

/// Get the PID file path
fn pid_file_path() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| {
            // On macOS/Linux, use runtime dir or fall back to cache dir
            #[cfg(unix)]
            {
                dirs.runtime_dir()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| dirs.cache_dir().to_path_buf())
            }
            #[cfg(not(unix))]
            {
                dirs.cache_dir().to_path_buf()
            }
        })
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("agent-memory")
        .join("daemon.pid")
}

/// Write PID to file
fn write_pid_file() -> Result<()> {
    let pid_path = pid_file_path();
    if let Some(parent) = pid_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&pid_path, std::process::id().to_string())?;
    info!("Wrote PID file: {:?}", pid_path);
    Ok(())
}

/// Remove PID file
fn remove_pid_file() {
    let pid_path = pid_file_path();
    if pid_path.exists() {
        if let Err(e) = fs::remove_file(&pid_path) {
            warn!("Failed to remove PID file: {}", e);
        } else {
            info!("Removed PID file");
        }
    }
}

/// Read PID from file
fn read_pid_file() -> Option<u32> {
    let pid_path = pid_file_path();
    fs::read_to_string(&pid_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Check if a process is running
#[cfg(unix)]
fn is_process_running(pid: u32) -> bool {
    // On Unix, sending signal 0 checks if process exists
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(not(unix))]
fn is_process_running(_pid: u32) -> bool {
    // On Windows, we'd need a different approach
    // For now, assume running if PID file exists
    true
}

/// Start the memory daemon.
///
/// 1. Load configuration (CFG-01: defaults -> file -> env -> CLI)
/// 2. Open RocksDB storage
/// 3. Create and start scheduler with rollup and compaction jobs
/// 4. Start gRPC server with scheduler integration
/// 5. Handle graceful shutdown on SIGINT/SIGTERM
pub async fn start_daemon(
    config_path: Option<&str>,
    foreground: bool,
    port_override: Option<u16>,
    db_path_override: Option<&str>,
    log_level_override: Option<&str>,
) -> Result<()> {
    // Load configuration (CFG-01)
    let mut settings = Settings::load(config_path).context("Failed to load configuration")?;

    // Apply CLI overrides (highest precedence per CFG-01)
    if let Some(port) = port_override {
        settings.grpc_port = port;
    }
    if let Some(db_path) = db_path_override {
        settings.db_path = db_path.to_string();
    }
    if let Some(log_level) = log_level_override {
        settings.log_level = log_level.to_string();
    }

    // Initialize logging
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&settings.log_level)),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set tracing subscriber")?;

    info!("Memory daemon starting...");
    info!("Configuration:");
    info!("  Database path: {}", settings.db_path);
    info!("  gRPC address: {}", settings.grpc_addr());
    info!("  Log level: {}", settings.log_level);

    if !foreground {
        // TODO: Implement actual daemonization (double-fork on Unix)
        // For Phase 1, just warn and continue in foreground
        warn!("Background mode not yet implemented, running in foreground");
        warn!("Use a process manager (systemd, launchd) for background operation");
    }

    // Open storage (STOR-04: per-project RocksDB instance)
    let db_path = settings.expanded_db_path();
    info!("Opening storage at {:?}", db_path);

    // Create parent directories if needed
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).context("Failed to create database directory")?;
    }

    let storage = Storage::open(&db_path).context("Failed to open storage")?;
    let storage = Arc::new(storage);

    // Create scheduler
    info!("Initializing scheduler...");
    let scheduler = SchedulerService::new(SchedulerConfig::default())
        .await
        .context("Failed to create scheduler")?;

    // Create summarizer for rollup jobs
    // TODO: Load from config - use ApiSummarizer if OPENAI_API_KEY or ANTHROPIC_API_KEY set
    let summarizer: Arc<dyn memory_toc::summarizer::Summarizer> =
        Arc::new(MockSummarizer::new());

    // Register rollup jobs (day/week/month)
    create_rollup_jobs(
        &scheduler,
        storage.clone(),
        summarizer,
        RollupJobConfig::default(),
    )
    .await
    .context("Failed to register rollup jobs")?;

    // Register compaction job
    create_compaction_job(&scheduler, storage.clone(), CompactionJobConfig::default())
        .await
        .context("Failed to register compaction job")?;

    info!(
        "Scheduler initialized with {} jobs",
        scheduler.registry().job_count()
    );

    // Write PID file
    write_pid_file()?;

    // Parse address
    let addr: SocketAddr = settings
        .grpc_addr()
        .parse()
        .context("Invalid gRPC address")?;

    // Create shutdown signal handler
    let shutdown_signal = async {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("Received Ctrl+C, shutting down...");
            }
            _ = terminate => {
                info!("Received SIGTERM, shutting down...");
            }
        }
    };

    // Start server with scheduler
    let result = run_server_with_scheduler(addr, storage, scheduler, shutdown_signal).await;

    // Cleanup
    remove_pid_file();

    result.map_err(|e| anyhow::anyhow!("Server error: {}", e))
}

/// Stop the running daemon by sending SIGTERM.
pub fn stop_daemon() -> Result<()> {
    let pid = read_pid_file().context("No PID file found - daemon may not be running")?;

    if !is_process_running(pid) {
        remove_pid_file();
        anyhow::bail!("Daemon not running (stale PID file removed)");
    }

    info!("Stopping daemon (PID {})", pid);

    #[cfg(unix)]
    {
        unsafe {
            if libc::kill(pid as i32, libc::SIGTERM) != 0 {
                anyhow::bail!("Failed to send SIGTERM to daemon");
            }
        }
        println!("Sent SIGTERM to daemon (PID {})", pid);
    }

    #[cfg(not(unix))]
    {
        anyhow::bail!("Stop command not yet implemented on this platform");
    }

    Ok(())
}

/// Show daemon status.
pub fn show_status() -> Result<()> {
    let pid_path = pid_file_path();

    match read_pid_file() {
        Some(pid) if is_process_running(pid) => {
            println!("Memory daemon is running (PID {})", pid);
            println!("PID file: {:?}", pid_path);
            Ok(())
        }
        Some(pid) => {
            println!(
                "Memory daemon is NOT running (stale PID {} in {:?})",
                pid, pid_path
            );
            Ok(())
        }
        None => {
            println!("Memory daemon is NOT running (no PID file)");
            Ok(())
        }
    }
}

/// Handle query commands.
pub async fn handle_query(endpoint: &str, command: QueryCommands) -> Result<()> {
    let mut client = MemoryClient::connect(endpoint)
        .await
        .context("Failed to connect to daemon")?;

    match command {
        QueryCommands::Root => {
            let nodes = client.get_toc_root().await.context("Failed to get TOC root")?;
            if nodes.is_empty() {
                println!("No TOC nodes found.");
            } else {
                println!("Root TOC Nodes ({} found):\n", nodes.len());
                for node in nodes {
                    let level = level_to_string(node.level);
                    println!("  {} [{}]", node.title, level);
                    println!("    ID: {}", node.node_id);
                    println!("    Children: {}", node.child_node_ids.len());
                    println!();
                }
            }
        }

        QueryCommands::Node { node_id } => {
            match client.get_node(&node_id).await.context("Failed to get node")? {
                Some(node) => {
                    print_node_details(&node);
                }
                None => {
                    println!("Node not found: {}", node_id);
                }
            }
        }

        QueryCommands::Browse { parent_id, limit, token } => {
            let result = client.browse_toc(&parent_id, limit, token)
                .await
                .context("Failed to browse TOC")?;

            if result.children.is_empty() {
                println!("No children found for: {}", parent_id);
            } else {
                println!("Children of {} ({} found):\n", parent_id, result.children.len());
                for child in result.children {
                    let level = level_to_string(child.level);
                    println!("  {} [{}]", child.title, level);
                    println!("    ID: {}", child.node_id);
                }
            }

            if result.has_more {
                if let Some(token) = result.continuation_token {
                    println!("\nMore results available. Use --token {}", token);
                }
            }
        }

        QueryCommands::Events { from, to, limit } => {
            let result = client.get_events(from, to, limit)
                .await
                .context("Failed to get events")?;

            if result.events.is_empty() {
                println!("No events found in time range.");
            } else {
                println!("Events ({} found):\n", result.events.len());
                for event in result.events {
                    let role = match event.role {
                        1 => "user",
                        2 => "assistant",
                        3 => "system",
                        4 => "tool",
                        _ => "unknown",
                    };
                    let text_preview = if event.text.len() > 80 {
                        format!("{}...", &event.text[..80])
                    } else {
                        event.text.clone()
                    };
                    println!("  [{}] {}: {}", event.timestamp_ms, role, text_preview);
                }
            }

            if result.has_more {
                println!("\nMore events available. Increase --limit to see more.");
            }
        }

        QueryCommands::Expand { grip_id, before, after } => {
            let result = client.expand_grip(&grip_id, Some(before), Some(after))
                .await
                .context("Failed to expand grip")?;

            match result.grip {
                Some(grip) => {
                    println!("Grip: {}\n", grip.grip_id);
                    println!("Excerpt: {}\n", grip.excerpt);

                    if !result.events_before.is_empty() {
                        println!("=== Events Before ({}) ===", result.events_before.len());
                        for event in result.events_before {
                            println!("  {}", truncate_text(&event.text, 100));
                        }
                        println!();
                    }

                    if !result.excerpt_events.is_empty() {
                        println!("=== Excerpt Events ({}) ===", result.excerpt_events.len());
                        for event in result.excerpt_events {
                            println!("  {}", truncate_text(&event.text, 100));
                        }
                        println!();
                    }

                    if !result.events_after.is_empty() {
                        println!("=== Events After ({}) ===", result.events_after.len());
                        for event in result.events_after {
                            println!("  {}", truncate_text(&event.text, 100));
                        }
                    }
                }
                None => {
                    println!("Grip not found: {}", grip_id);
                }
            }
        }
    }

    Ok(())
}

fn level_to_string(level: i32) -> &'static str {
    match level {
        l if l == ProtoTocLevel::Year as i32 => "Year",
        l if l == ProtoTocLevel::Month as i32 => "Month",
        l if l == ProtoTocLevel::Week as i32 => "Week",
        l if l == ProtoTocLevel::Day as i32 => "Day",
        l if l == ProtoTocLevel::Segment as i32 => "Segment",
        _ => "Unknown",
    }
}

fn print_node_details(node: &memory_service::pb::TocNode) {
    let level = level_to_string(node.level);
    println!("TOC Node: {}", node.title);
    println!("  ID: {}", node.node_id);
    println!("  Level: {}", level);
    println!("  Version: {}", node.version);
    println!("  Time Range: {} - {}", node.start_time_ms, node.end_time_ms);

    if let Some(summary) = &node.summary {
        println!("\nSummary: {}", summary);
    }

    if !node.bullets.is_empty() {
        println!("\nBullets:");
        for bullet in &node.bullets {
            println!("  • {}", bullet.text);
            if !bullet.grip_ids.is_empty() {
                println!("    Grips: {}", bullet.grip_ids.join(", "));
            }
        }
    }

    if !node.keywords.is_empty() {
        println!("\nKeywords: {}", node.keywords.join(", "));
    }

    if !node.child_node_ids.is_empty() {
        println!("\nChildren ({}):", node.child_node_ids.len());
        for (i, child_id) in node.child_node_ids.iter().enumerate() {
            if i >= 10 {
                println!("  ... and {} more", node.child_node_ids.len() - 10);
                break;
            }
            println!("  {}", child_id);
        }
    }
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() > max_len {
        format!("{}...", &text[..max_len])
    } else {
        text.to_string()
    }
}

/// Handle admin commands.
///
/// Per CLI-03: Admin commands include rebuild-toc, compact, status.
pub fn handle_admin(db_path: Option<String>, command: AdminCommands) -> Result<()> {
    // Load settings to get default db_path if not provided
    let settings = Settings::load(None).context("Failed to load configuration")?;
    let db_path = db_path.unwrap_or_else(|| settings.db_path.clone());
    let expanded_path = shellexpand::tilde(&db_path).to_string();

    // Open storage directly (not via gRPC)
    let storage = Storage::open(std::path::Path::new(&expanded_path))
        .context(format!("Failed to open storage at {}", expanded_path))?;

    match command {
        AdminCommands::Stats => {
            let stats = storage.get_stats().context("Failed to get stats")?;

            println!("Database Statistics");
            println!("===================");
            println!("Path: {}", expanded_path);
            println!();
            println!("Events:       {:>10}", stats.event_count);
            println!("TOC Nodes:    {:>10}", stats.toc_node_count);
            println!("Grips:        {:>10}", stats.grip_count);
            println!("Outbox:       {:>10}", stats.outbox_count);
            println!();
            println!("Disk Usage:   {:>10}", format_bytes(stats.disk_usage_bytes));
        }

        AdminCommands::Compact { cf } => {
            match cf {
                Some(cf_name) => {
                    println!("Compacting column family: {}", cf_name);
                    storage.compact_cf(&cf_name)
                        .context(format!("Failed to compact {}", cf_name))?;
                    println!("Compaction complete.");
                }
                None => {
                    println!("Compacting all column families...");
                    storage.compact().context("Failed to compact")?;
                    println!("Compaction complete.");
                }
            }
        }

        AdminCommands::RebuildToc { from_date, dry_run } => {
            if dry_run {
                println!("DRY RUN - No changes will be made");
                println!();
            }

            let from_timestamp = if let Some(date_str) = from_date {
                // Parse YYYY-MM-DD date
                let date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                    .context(format!("Invalid date format: {}. Use YYYY-MM-DD", date_str))?;
                let datetime = date.and_hms_opt(0, 0, 0).unwrap();
                Some(chrono::Utc.from_utc_datetime(&datetime).timestamp_millis())
            } else {
                None
            };

            // Get events to process
            let start_ms = from_timestamp.unwrap_or(0);
            let end_ms = chrono::Utc::now().timestamp_millis();

            let events = storage.get_events_in_range(start_ms, end_ms)
                .context("Failed to query events")?;

            println!("Found {} events to process", events.len());

            if events.is_empty() {
                println!("No events to rebuild TOC from.");
                return Ok(());
            }

            if dry_run {
                println!();
                println!("Would process events from {} to {}", start_ms, end_ms);
                println!("First event timestamp: {}", events.first().map(|(k, _)| k.timestamp_ms).unwrap_or(0));
                println!("Last event timestamp: {}", events.last().map(|(k, _)| k.timestamp_ms).unwrap_or(0));
                println!();
                println!("To actually rebuild, run without --dry-run");
            } else {
                // TODO: Full TOC rebuild would require integrating with memory-toc
                // For now, just report what would be done
                println!();
                println!("TOC rebuild not yet fully implemented.");
                println!("This would require re-running segmentation and summarization.");
                println!("Events are intact and can be manually processed.");
            }
        }
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Handle scheduler commands.
///
/// Per SCHED-05: Job status observable via CLI.
pub async fn handle_scheduler(endpoint: &str, command: SchedulerCommands) -> Result<()> {
    use memory_service::pb::memory_service_client::MemoryServiceClient;

    let mut client = MemoryServiceClient::connect(endpoint.to_string())
        .await
        .context("Failed to connect to daemon")?;

    match command {
        SchedulerCommands::Status => {
            let response = client
                .get_scheduler_status(GetSchedulerStatusRequest {})
                .await
                .context("Failed to get scheduler status")?
                .into_inner();

            let status_str = if response.scheduler_running {
                "RUNNING"
            } else {
                "STOPPED"
            };
            println!("Scheduler: {}", status_str);
            println!();

            if response.jobs.is_empty() {
                println!("No jobs registered.");
            } else {
                println!(
                    "{:<20} {:<12} {:<20} {:<20} {:<10} {:<10}",
                    "JOB", "STATUS", "LAST RUN", "NEXT RUN", "RUNS", "ERRORS"
                );
                println!("{}", "-".repeat(92));

                for job in response.jobs {
                    let status = if job.is_paused {
                        "PAUSED"
                    } else if job.is_running {
                        "RUNNING"
                    } else {
                        "IDLE"
                    };

                    let last_run = if job.last_run_ms > 0 {
                        format_timestamp(job.last_run_ms)
                    } else {
                        "Never".to_string()
                    };

                    let next_run = if job.next_run_ms > 0 && !job.is_paused {
                        format_timestamp(job.next_run_ms)
                    } else {
                        "-".to_string()
                    };

                    println!(
                        "{:<20} {:<12} {:<20} {:<20} {:<10} {:<10}",
                        job.job_name, status, last_run, next_run, job.run_count, job.error_count
                    );

                    // Show last result if there was an error
                    if job.last_result == JobResultStatus::Failed as i32 {
                        if let Some(error) = &job.last_error {
                            println!("  Last error: {}", error);
                        }
                    }
                }
            }
        }

        SchedulerCommands::Pause { job_name } => {
            let response = client
                .pause_job(PauseJobRequest {
                    job_name: job_name.clone(),
                })
                .await
                .context("Failed to pause job")?
                .into_inner();

            if response.success {
                println!("Job '{}' paused.", job_name);
            } else {
                println!(
                    "Failed to pause '{}': {}",
                    job_name,
                    response.error.unwrap_or_default()
                );
            }
        }

        SchedulerCommands::Resume { job_name } => {
            let response = client
                .resume_job(ResumeJobRequest {
                    job_name: job_name.clone(),
                })
                .await
                .context("Failed to resume job")?
                .into_inner();

            if response.success {
                println!("Job '{}' resumed.", job_name);
            } else {
                println!(
                    "Failed to resume '{}': {}",
                    job_name,
                    response.error.unwrap_or_default()
                );
            }
        }
    }

    Ok(())
}

/// Format a timestamp in milliseconds as a local time string.
fn format_timestamp(ms: i64) -> String {
    use chrono::{DateTime, Local, Utc};

    DateTime::<Utc>::from_timestamp_millis(ms)
        .map(|t| t.with_timezone(&Local))
        .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "Invalid".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_file_path() {
        let path = pid_file_path();
        assert!(path.ends_with("daemon.pid"));
        assert!(path
            .parent()
            .unwrap()
            .to_string_lossy()
            .contains("agent-memory"));
    }

    #[test]
    fn test_status_no_daemon() {
        // Just verify it doesn't panic
        let result = show_status();
        assert!(result.is_ok());
    }

    #[test]
    fn test_level_to_string() {
        assert_eq!(level_to_string(ProtoTocLevel::Year as i32), "Year");
        assert_eq!(level_to_string(ProtoTocLevel::Month as i32), "Month");
        assert_eq!(level_to_string(ProtoTocLevel::Segment as i32), "Segment");
    }

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("hello", 10), "hello");
        assert_eq!(truncate_text("hello world!", 5), "hello...");
    }
}
