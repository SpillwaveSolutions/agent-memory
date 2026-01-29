//! CLI argument parsing for the memory daemon.
//!
//! Per CLI-01: Memory daemon binary with start/stop/status commands.
//! Per CFG-01: CLI flags override all other config sources.

use clap::{Parser, Subcommand};

/// Agent Memory Daemon
///
/// A local, append-only conversational memory system for AI agents.
#[derive(Parser, Debug)]
#[command(name = "memory-daemon")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to config file (overrides default ~/.config/agent-memory/config.toml)
    #[arg(short, long, global = true)]
    pub config: Option<String>,

    /// Set log level (trace, debug, info, warn, error)
    #[arg(short, long, global = true)]
    pub log_level: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

/// Daemon commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the memory daemon
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,

        /// Override gRPC port
        #[arg(short, long)]
        port: Option<u16>,

        /// Override database path
        #[arg(long)]
        db_path: Option<String>,
    },

    /// Stop the running daemon
    Stop,

    /// Show daemon status
    Status,
}

impl Cli {
    /// Parse CLI arguments
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_start_foreground() {
        let cli = Cli::parse_from(["memory-daemon", "start", "--foreground"]);
        match cli.command {
            Commands::Start { foreground, .. } => assert!(foreground),
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_cli_start_with_port() {
        let cli = Cli::parse_from(["memory-daemon", "start", "-p", "9999"]);
        match cli.command {
            Commands::Start { port, .. } => assert_eq!(port, Some(9999)),
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_cli_with_config() {
        let cli = Cli::parse_from(["memory-daemon", "--config", "/path/to/config.toml", "start"]);
        assert_eq!(cli.config, Some("/path/to/config.toml".to_string()));
    }

    #[test]
    fn test_cli_status() {
        let cli = Cli::parse_from(["memory-daemon", "status"]);
        assert!(matches!(cli.command, Commands::Status));
    }

    #[test]
    fn test_cli_stop() {
        let cli = Cli::parse_from(["memory-daemon", "stop"]);
        assert!(matches!(cli.command, Commands::Stop));
    }

    #[test]
    fn test_cli_start_with_db_path() {
        let cli = Cli::parse_from(["memory-daemon", "start", "--db-path", "/custom/db"]);
        match cli.command {
            Commands::Start { db_path, .. } => assert_eq!(db_path, Some("/custom/db".to_string())),
            _ => panic!("Expected Start command"),
        }
    }

    #[test]
    fn test_cli_with_log_level() {
        let cli = Cli::parse_from(["memory-daemon", "--log-level", "debug", "start"]);
        assert_eq!(cli.log_level, Some("debug".to_string()));
    }
}
