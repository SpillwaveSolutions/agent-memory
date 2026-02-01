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

    /// Query the memory system
    Query {
        /// gRPC endpoint (default: http://[::1]:50051)
        #[arg(short, long, default_value = "http://[::1]:50051")]
        endpoint: String,

        #[command(subcommand)]
        command: QueryCommands,
    },

    /// Administrative commands
    Admin {
        /// Database path (default from config)
        #[arg(long)]
        db_path: Option<String>,

        #[command(subcommand)]
        command: AdminCommands,
    },

    /// Scheduler management commands
    Scheduler {
        /// gRPC endpoint (default: http://[::1]:50051)
        #[arg(short, long, default_value = "http://[::1]:50051")]
        endpoint: String,

        #[command(subcommand)]
        command: SchedulerCommands,
    },
}

/// Query subcommands
#[derive(Subcommand, Debug, Clone)]
pub enum QueryCommands {
    /// List root TOC nodes (year level)
    Root,

    /// Get a specific TOC node
    Node {
        /// Node ID to retrieve
        node_id: String,
    },

    /// Browse children of a node
    Browse {
        /// Parent node ID
        parent_id: String,

        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Continuation token for pagination
        #[arg(short, long)]
        token: Option<String>,
    },

    /// Get events in time range
    Events {
        /// Start time (Unix ms)
        #[arg(long)]
        from: i64,

        /// End time (Unix ms)
        #[arg(long)]
        to: i64,

        /// Maximum results
        #[arg(short, long, default_value = "50")]
        limit: u32,
    },

    /// Expand a grip to show context
    Expand {
        /// Grip ID to expand
        grip_id: String,

        /// Number of events before excerpt
        #[arg(long, default_value = "3")]
        before: u32,

        /// Number of events after excerpt
        #[arg(long, default_value = "3")]
        after: u32,
    },
}

/// Admin subcommands
#[derive(Subcommand, Debug, Clone)]
pub enum AdminCommands {
    /// Show database statistics
    Stats,

    /// Trigger RocksDB compaction
    Compact {
        /// Compact only specific column family
        #[arg(long)]
        cf: Option<String>,
    },

    /// Rebuild TOC from raw events
    RebuildToc {
        /// Start from this date (YYYY-MM-DD)
        #[arg(long)]
        from_date: Option<String>,

        /// Dry run - show what would be done
        #[arg(long)]
        dry_run: bool,
    },
}

/// Scheduler subcommands
#[derive(Subcommand, Debug, Clone)]
pub enum SchedulerCommands {
    /// Show scheduler and job status
    Status,

    /// Pause a scheduled job
    Pause {
        /// Job name to pause
        job_name: String,
    },

    /// Resume a paused job
    Resume {
        /// Job name to resume
        job_name: String,
    },
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

    #[test]
    fn test_cli_scheduler_status() {
        let cli = Cli::parse_from(["memory-daemon", "scheduler", "status"]);
        match cli.command {
            Commands::Scheduler { command, .. } => {
                assert!(matches!(command, SchedulerCommands::Status));
            }
            _ => panic!("Expected Scheduler command"),
        }
    }

    #[test]
    fn test_cli_scheduler_pause() {
        let cli = Cli::parse_from(["memory-daemon", "scheduler", "pause", "hourly-rollup"]);
        match cli.command {
            Commands::Scheduler { command, .. } => match command {
                SchedulerCommands::Pause { job_name } => {
                    assert_eq!(job_name, "hourly-rollup");
                }
                _ => panic!("Expected Pause command"),
            },
            _ => panic!("Expected Scheduler command"),
        }
    }

    #[test]
    fn test_cli_scheduler_resume() {
        let cli = Cli::parse_from(["memory-daemon", "scheduler", "resume", "daily-cleanup"]);
        match cli.command {
            Commands::Scheduler { command, .. } => match command {
                SchedulerCommands::Resume { job_name } => {
                    assert_eq!(job_name, "daily-cleanup");
                }
                _ => panic!("Expected Resume command"),
            },
            _ => panic!("Expected Scheduler command"),
        }
    }

    #[test]
    fn test_cli_scheduler_with_endpoint() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "scheduler",
            "-e",
            "http://localhost:9999",
            "status",
        ]);
        match cli.command {
            Commands::Scheduler { endpoint, .. } => {
                assert_eq!(endpoint, "http://localhost:9999");
            }
            _ => panic!("Expected Scheduler command"),
        }
    }
}
