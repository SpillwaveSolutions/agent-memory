//! Agent Memory Daemon
//!
//! A local, append-only conversational memory system for AI agents.
//!
//! # Usage
//!
//! ```bash
//! memory-daemon start [--foreground] [--port PORT] [--db-path PATH]
//! memory-daemon stop
//! memory-daemon status
//! ```
//!
//! # Configuration
//!
//! Configuration is loaded in order (later sources override earlier):
//! 1. Built-in defaults
//! 2. Config file (~/.config/agent-memory/config.toml)
//! 3. Environment variables (MEMORY_*)
//! 4. CLI flags

use anyhow::Result;
use clap::Parser;

use memory_daemon::{
    handle_admin, handle_query, handle_scheduler, handle_teleport_command, handle_topics_command,
    show_status, start_daemon, stop_daemon, Cli, Commands,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            foreground,
            port,
            db_path,
        } => {
            start_daemon(
                cli.config.as_deref(),
                foreground,
                port,
                db_path.as_deref(),
                cli.log_level.as_deref(),
            )
            .await?;
        }
        Commands::Stop => {
            stop_daemon()?;
        }
        Commands::Status => {
            show_status()?;
        }
        Commands::Query { endpoint, command } => {
            handle_query(&endpoint, command).await?;
        }
        Commands::Admin { db_path, command } => {
            handle_admin(db_path, command)?;
        }
        Commands::Scheduler { endpoint, command } => {
            handle_scheduler(&endpoint, command).await?;
        }
        Commands::Teleport(cmd) => {
            handle_teleport_command(cmd).await?;
        }
        Commands::Topics(cmd) => {
            handle_topics_command(cmd).await?;
        }
    }

    Ok(())
}
