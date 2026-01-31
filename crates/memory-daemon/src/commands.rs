//! Command implementations for the memory daemon.
//!
//! Handles:
//! - start: Load config, open storage, start gRPC server
//! - stop: Signal running daemon to stop (via PID file)
//! - status: Check if daemon is running

use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::signal;
use tracing::{info, warn};

use memory_client::MemoryClient;
use memory_service::run_server_with_shutdown;
use memory_service::pb::TocLevel as ProtoTocLevel;
use memory_storage::Storage;
use memory_types::Settings;

use crate::cli::QueryCommands;

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
/// 3. Start gRPC server
/// 4. Handle graceful shutdown on SIGINT/SIGTERM
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

    // Start server
    let result = run_server_with_shutdown(addr, storage, shutdown_signal).await;

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
