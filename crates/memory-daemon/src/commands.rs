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

use memory_service::run_server_with_shutdown;
use memory_storage::Storage;
use memory_types::Settings;

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
}
