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

    /// Teleport (BM25 keyword search) commands
    #[command(subcommand)]
    Teleport(TeleportCommand),

    /// Topic graph management commands
    #[command(subcommand)]
    Topics(TopicsCommand),
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

    /// Search TOC nodes for matching content
    Search {
        /// Search query terms (space-separated)
        #[arg(short, long)]
        query: String,

        /// Search within a specific node (mutually exclusive with --parent)
        #[arg(long, conflicts_with = "parent")]
        node: Option<String>,

        /// Search children of a parent node (empty for root level)
        #[arg(long, conflicts_with = "node")]
        parent: Option<String>,

        /// Fields to search: title, summary, bullets, keywords (comma-separated)
        #[arg(long)]
        fields: Option<String>,

        /// Maximum results to return
        #[arg(long, default_value = "10")]
        limit: u32,
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

    /// Rebuild search indexes from storage
    RebuildIndexes {
        /// Which index to rebuild: bm25, vector, or all
        #[arg(long, default_value = "all")]
        index: String,

        /// Batch size for processing (progress reported after each batch)
        #[arg(long, default_value = "100")]
        batch_size: usize,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,

        /// Path to search index directory (default from config)
        #[arg(long)]
        search_path: Option<String>,

        /// Path to vector index directory (default from config)
        #[arg(long)]
        vector_path: Option<String>,
    },

    /// Show search index statistics
    IndexStats {
        /// Path to search index directory (default from config)
        #[arg(long)]
        search_path: Option<String>,

        /// Path to vector index directory (default from config)
        #[arg(long)]
        vector_path: Option<String>,
    },

    /// Clear and reset a search index
    ClearIndex {
        /// Which index to clear: bm25, vector, or all
        #[arg(long)]
        index: String,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,

        /// Path to search index directory (default from config)
        #[arg(long)]
        search_path: Option<String>,

        /// Path to vector index directory (default from config)
        #[arg(long)]
        vector_path: Option<String>,
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

/// Teleport (BM25 search) commands
#[derive(Subcommand, Debug, Clone)]
pub enum TeleportCommand {
    /// Search for TOC nodes or grips by keyword (BM25)
    Search {
        /// Search query (keywords)
        query: String,

        /// Filter by document type: all, toc, grip
        #[arg(long, short = 't', default_value = "all")]
        doc_type: String,

        /// Maximum results to return
        #[arg(long, short = 'n', default_value = "10")]
        limit: usize,

        /// gRPC server address
        #[arg(long, default_value = "http://[::1]:50051")]
        addr: String,
    },

    /// Semantic similarity search using vector embeddings
    VectorSearch {
        /// Search query text
        #[arg(short, long)]
        query: String,

        /// Number of results to return
        #[arg(long, default_value = "10")]
        top_k: i32,

        /// Minimum similarity score (0.0-1.0)
        #[arg(long, default_value = "0.0")]
        min_score: f32,

        /// Filter by target type: all, toc, grip
        #[arg(long, default_value = "all")]
        target: String,

        /// gRPC server address
        #[arg(long, default_value = "http://[::1]:50051")]
        addr: String,
    },

    /// Combined BM25 + vector search using RRF fusion
    HybridSearch {
        /// Search query text
        #[arg(short, long)]
        query: String,

        /// Number of results to return
        #[arg(long, default_value = "10")]
        top_k: i32,

        /// Search mode: hybrid, vector-only, bm25-only
        #[arg(long, default_value = "hybrid")]
        mode: String,

        /// Weight for BM25 in fusion (0.0-1.0)
        #[arg(long, default_value = "0.5")]
        bm25_weight: f32,

        /// Weight for vector in fusion (0.0-1.0)
        #[arg(long, default_value = "0.5")]
        vector_weight: f32,

        /// Filter by target type: all, toc, grip
        #[arg(long, default_value = "all")]
        target: String,

        /// gRPC server address
        #[arg(long, default_value = "http://[::1]:50051")]
        addr: String,
    },

    /// Show BM25 index statistics
    Stats {
        /// gRPC server address
        #[arg(long, default_value = "http://[::1]:50051")]
        addr: String,
    },

    /// Show vector index statistics
    VectorStats {
        /// gRPC server address
        #[arg(long, default_value = "http://[::1]:50051")]
        addr: String,
    },

    /// Rebuild the search index from storage
    Rebuild {
        /// gRPC server address (for triggering rebuild)
        #[arg(long, default_value = "http://[::1]:50051")]
        addr: String,
    },
}

/// Topics (topic graph) commands
#[derive(Subcommand, Debug, Clone)]
pub enum TopicsCommand {
    /// Show topic graph status and lifecycle stats
    Status {
        /// gRPC server address
        #[arg(long, default_value = "http://[::1]:50051")]
        addr: String,
    },

    /// List topics matching a query
    Explore {
        /// Search query (keywords to match against topic labels/keywords)
        query: String,

        /// Maximum results to return
        #[arg(long, short = 'n', default_value = "10")]
        limit: u32,

        /// gRPC server address
        #[arg(long, default_value = "http://[::1]:50051")]
        addr: String,
    },

    /// Show related topics for a given topic
    Related {
        /// Topic ID to find related topics for
        topic_id: String,

        /// Filter by relationship type (co-occurrence, semantic, hierarchical)
        #[arg(long, short = 't')]
        rel_type: Option<String>,

        /// Maximum results to return
        #[arg(long, short = 'n', default_value = "10")]
        limit: u32,

        /// gRPC server address
        #[arg(long, default_value = "http://[::1]:50051")]
        addr: String,
    },

    /// Show top topics by importance score
    Top {
        /// Maximum results to return
        #[arg(long, short = 'n', default_value = "10")]
        limit: u32,

        /// Look back window in days (default: 30)
        #[arg(long, default_value = "30")]
        days: u32,

        /// gRPC server address
        #[arg(long, default_value = "http://[::1]:50051")]
        addr: String,
    },

    /// Trigger importance score refresh
    RefreshScores {
        /// Database path (default from config)
        #[arg(long)]
        db_path: Option<String>,
    },

    /// Archive stale topics not mentioned in N days
    Prune {
        /// Days of inactivity before pruning (default: 90)
        #[arg(long, default_value = "90")]
        days: u32,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,

        /// Database path (default from config)
        #[arg(long)]
        db_path: Option<String>,
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

    #[test]
    fn test_cli_teleport_search() {
        let cli = Cli::parse_from(["memory-daemon", "teleport", "search", "rust memory"]);
        match cli.command {
            Commands::Teleport(TeleportCommand::Search {
                query,
                doc_type,
                limit,
                ..
            }) => {
                assert_eq!(query, "rust memory");
                assert_eq!(doc_type, "all");
                assert_eq!(limit, 10);
            }
            _ => panic!("Expected Teleport Search command"),
        }
    }

    #[test]
    fn test_cli_teleport_search_with_options() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "teleport",
            "search",
            "rust memory",
            "-t",
            "toc",
            "-n",
            "5",
            "--addr",
            "http://localhost:9999",
        ]);
        match cli.command {
            Commands::Teleport(TeleportCommand::Search {
                query,
                doc_type,
                limit,
                addr,
            }) => {
                assert_eq!(query, "rust memory");
                assert_eq!(doc_type, "toc");
                assert_eq!(limit, 5);
                assert_eq!(addr, "http://localhost:9999");
            }
            _ => panic!("Expected Teleport Search command"),
        }
    }

    #[test]
    fn test_cli_teleport_stats() {
        let cli = Cli::parse_from(["memory-daemon", "teleport", "stats"]);
        match cli.command {
            Commands::Teleport(TeleportCommand::Stats { addr }) => {
                assert_eq!(addr, "http://[::1]:50051");
            }
            _ => panic!("Expected Teleport Stats command"),
        }
    }

    #[test]
    fn test_cli_teleport_rebuild() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "teleport",
            "rebuild",
            "--addr",
            "http://localhost:9999",
        ]);
        match cli.command {
            Commands::Teleport(TeleportCommand::Rebuild { addr }) => {
                assert_eq!(addr, "http://localhost:9999");
            }
            _ => panic!("Expected Teleport Rebuild command"),
        }
    }

    #[test]
    fn test_cli_teleport_vector_search() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "teleport",
            "vector-search",
            "--query",
            "authentication patterns",
        ]);
        match cli.command {
            Commands::Teleport(TeleportCommand::VectorSearch {
                query,
                top_k,
                min_score,
                target,
                ..
            }) => {
                assert_eq!(query, "authentication patterns");
                assert_eq!(top_k, 10);
                assert!((min_score - 0.0).abs() < f32::EPSILON);
                assert_eq!(target, "all");
            }
            _ => panic!("Expected Teleport VectorSearch command"),
        }
    }

    #[test]
    fn test_cli_teleport_vector_search_with_options() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "teleport",
            "vector-search",
            "-q",
            "rust patterns",
            "--top-k",
            "5",
            "--min-score",
            "0.7",
            "--target",
            "toc",
            "--addr",
            "http://localhost:9999",
        ]);
        match cli.command {
            Commands::Teleport(TeleportCommand::VectorSearch {
                query,
                top_k,
                min_score,
                target,
                addr,
            }) => {
                assert_eq!(query, "rust patterns");
                assert_eq!(top_k, 5);
                assert!((min_score - 0.7).abs() < f32::EPSILON);
                assert_eq!(target, "toc");
                assert_eq!(addr, "http://localhost:9999");
            }
            _ => panic!("Expected Teleport VectorSearch command"),
        }
    }

    #[test]
    fn test_cli_teleport_hybrid_search() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "teleport",
            "hybrid-search",
            "--query",
            "memory systems",
        ]);
        match cli.command {
            Commands::Teleport(TeleportCommand::HybridSearch {
                query,
                top_k,
                mode,
                bm25_weight,
                vector_weight,
                target,
                ..
            }) => {
                assert_eq!(query, "memory systems");
                assert_eq!(top_k, 10);
                assert_eq!(mode, "hybrid");
                assert!((bm25_weight - 0.5).abs() < f32::EPSILON);
                assert!((vector_weight - 0.5).abs() < f32::EPSILON);
                assert_eq!(target, "all");
            }
            _ => panic!("Expected Teleport HybridSearch command"),
        }
    }

    #[test]
    fn test_cli_teleport_hybrid_search_with_options() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "teleport",
            "hybrid-search",
            "-q",
            "debugging",
            "--top-k",
            "20",
            "--mode",
            "vector-only",
            "--bm25-weight",
            "0.3",
            "--vector-weight",
            "0.7",
            "--target",
            "grip",
        ]);
        match cli.command {
            Commands::Teleport(TeleportCommand::HybridSearch {
                query,
                top_k,
                mode,
                bm25_weight,
                vector_weight,
                target,
                ..
            }) => {
                assert_eq!(query, "debugging");
                assert_eq!(top_k, 20);
                assert_eq!(mode, "vector-only");
                assert!((bm25_weight - 0.3).abs() < f32::EPSILON);
                assert!((vector_weight - 0.7).abs() < f32::EPSILON);
                assert_eq!(target, "grip");
            }
            _ => panic!("Expected Teleport HybridSearch command"),
        }
    }

    #[test]
    fn test_cli_teleport_vector_stats() {
        let cli = Cli::parse_from(["memory-daemon", "teleport", "vector-stats"]);
        match cli.command {
            Commands::Teleport(TeleportCommand::VectorStats { addr }) => {
                assert_eq!(addr, "http://[::1]:50051");
            }
            _ => panic!("Expected Teleport VectorStats command"),
        }
    }

    #[test]
    fn test_cli_query_search() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "query",
            "search",
            "--query",
            "JWT authentication",
        ]);
        match cli.command {
            Commands::Query { command, .. } => match command {
                QueryCommands::Search {
                    query,
                    node,
                    parent,
                    fields,
                    limit,
                } => {
                    assert_eq!(query, "JWT authentication");
                    assert!(node.is_none());
                    assert!(parent.is_none());
                    assert!(fields.is_none());
                    assert_eq!(limit, 10);
                }
                _ => panic!("Expected Search command"),
            },
            _ => panic!("Expected Query command"),
        }
    }

    #[test]
    fn test_cli_query_search_with_node() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "query",
            "search",
            "--query",
            "debugging",
            "--node",
            "toc:month:2026-01",
        ]);
        match cli.command {
            Commands::Query { command, .. } => match command {
                QueryCommands::Search {
                    query,
                    node,
                    parent,
                    ..
                } => {
                    assert_eq!(query, "debugging");
                    assert_eq!(node, Some("toc:month:2026-01".to_string()));
                    assert!(parent.is_none());
                }
                _ => panic!("Expected Search command"),
            },
            _ => panic!("Expected Query command"),
        }
    }

    #[test]
    fn test_cli_query_search_with_parent() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "query",
            "search",
            "--query",
            "token",
            "--parent",
            "toc:week:2026-W04",
            "--fields",
            "title,bullets",
            "--limit",
            "20",
        ]);
        match cli.command {
            Commands::Query { command, .. } => match command {
                QueryCommands::Search {
                    query,
                    node,
                    parent,
                    fields,
                    limit,
                } => {
                    assert_eq!(query, "token");
                    assert!(node.is_none());
                    assert_eq!(parent, Some("toc:week:2026-W04".to_string()));
                    assert_eq!(fields, Some("title,bullets".to_string()));
                    assert_eq!(limit, 20);
                }
                _ => panic!("Expected Search command"),
            },
            _ => panic!("Expected Query command"),
        }
    }

    #[test]
    fn test_cli_admin_rebuild_indexes_defaults() {
        let cli = Cli::parse_from(["memory-daemon", "admin", "rebuild-indexes"]);
        match cli.command {
            Commands::Admin { command, .. } => match command {
                AdminCommands::RebuildIndexes {
                    index,
                    batch_size,
                    force,
                    search_path,
                    vector_path,
                } => {
                    assert_eq!(index, "all");
                    assert_eq!(batch_size, 100);
                    assert!(!force);
                    assert!(search_path.is_none());
                    assert!(vector_path.is_none());
                }
                _ => panic!("Expected RebuildIndexes command"),
            },
            _ => panic!("Expected Admin command"),
        }
    }

    #[test]
    fn test_cli_admin_rebuild_indexes_with_options() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "admin",
            "rebuild-indexes",
            "--index",
            "bm25",
            "--batch-size",
            "50",
            "--force",
            "--search-path",
            "/custom/search",
        ]);
        match cli.command {
            Commands::Admin { command, .. } => match command {
                AdminCommands::RebuildIndexes {
                    index,
                    batch_size,
                    force,
                    search_path,
                    vector_path,
                } => {
                    assert_eq!(index, "bm25");
                    assert_eq!(batch_size, 50);
                    assert!(force);
                    assert_eq!(search_path, Some("/custom/search".to_string()));
                    assert!(vector_path.is_none());
                }
                _ => panic!("Expected RebuildIndexes command"),
            },
            _ => panic!("Expected Admin command"),
        }
    }

    #[test]
    fn test_cli_admin_index_stats() {
        let cli = Cli::parse_from(["memory-daemon", "admin", "index-stats"]);
        match cli.command {
            Commands::Admin { command, .. } => match command {
                AdminCommands::IndexStats {
                    search_path,
                    vector_path,
                } => {
                    assert!(search_path.is_none());
                    assert!(vector_path.is_none());
                }
                _ => panic!("Expected IndexStats command"),
            },
            _ => panic!("Expected Admin command"),
        }
    }

    #[test]
    fn test_cli_admin_clear_index() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "admin",
            "clear-index",
            "--index",
            "vector",
            "--force",
        ]);
        match cli.command {
            Commands::Admin { command, .. } => match command {
                AdminCommands::ClearIndex {
                    index,
                    force,
                    search_path,
                    vector_path,
                } => {
                    assert_eq!(index, "vector");
                    assert!(force);
                    assert!(search_path.is_none());
                    assert!(vector_path.is_none());
                }
                _ => panic!("Expected ClearIndex command"),
            },
            _ => panic!("Expected Admin command"),
        }
    }

    #[test]
    fn test_cli_topics_status() {
        let cli = Cli::parse_from(["memory-daemon", "topics", "status"]);
        match cli.command {
            Commands::Topics(TopicsCommand::Status { addr }) => {
                assert_eq!(addr, "http://[::1]:50051");
            }
            _ => panic!("Expected Topics Status command"),
        }
    }

    #[test]
    fn test_cli_topics_status_with_addr() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "topics",
            "status",
            "--addr",
            "http://localhost:9999",
        ]);
        match cli.command {
            Commands::Topics(TopicsCommand::Status { addr }) => {
                assert_eq!(addr, "http://localhost:9999");
            }
            _ => panic!("Expected Topics Status command"),
        }
    }

    #[test]
    fn test_cli_topics_explore() {
        let cli = Cli::parse_from(["memory-daemon", "topics", "explore", "rust memory"]);
        match cli.command {
            Commands::Topics(TopicsCommand::Explore {
                query, limit, addr, ..
            }) => {
                assert_eq!(query, "rust memory");
                assert_eq!(limit, 10);
                assert_eq!(addr, "http://[::1]:50051");
            }
            _ => panic!("Expected Topics Explore command"),
        }
    }

    #[test]
    fn test_cli_topics_explore_with_options() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "topics",
            "explore",
            "authentication",
            "-n",
            "5",
            "--addr",
            "http://localhost:9999",
        ]);
        match cli.command {
            Commands::Topics(TopicsCommand::Explore { query, limit, addr }) => {
                assert_eq!(query, "authentication");
                assert_eq!(limit, 5);
                assert_eq!(addr, "http://localhost:9999");
            }
            _ => panic!("Expected Topics Explore command"),
        }
    }

    #[test]
    fn test_cli_topics_related() {
        let cli = Cli::parse_from(["memory-daemon", "topics", "related", "topic-123"]);
        match cli.command {
            Commands::Topics(TopicsCommand::Related {
                topic_id,
                rel_type,
                limit,
                addr,
            }) => {
                assert_eq!(topic_id, "topic-123");
                assert!(rel_type.is_none());
                assert_eq!(limit, 10);
                assert_eq!(addr, "http://[::1]:50051");
            }
            _ => panic!("Expected Topics Related command"),
        }
    }

    #[test]
    fn test_cli_topics_related_with_options() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "topics",
            "related",
            "topic-abc",
            "-t",
            "semantic",
            "-n",
            "20",
        ]);
        match cli.command {
            Commands::Topics(TopicsCommand::Related {
                topic_id,
                rel_type,
                limit,
                ..
            }) => {
                assert_eq!(topic_id, "topic-abc");
                assert_eq!(rel_type, Some("semantic".to_string()));
                assert_eq!(limit, 20);
            }
            _ => panic!("Expected Topics Related command"),
        }
    }

    #[test]
    fn test_cli_topics_top() {
        let cli = Cli::parse_from(["memory-daemon", "topics", "top"]);
        match cli.command {
            Commands::Topics(TopicsCommand::Top { limit, days, addr }) => {
                assert_eq!(limit, 10);
                assert_eq!(days, 30);
                assert_eq!(addr, "http://[::1]:50051");
            }
            _ => panic!("Expected Topics Top command"),
        }
    }

    #[test]
    fn test_cli_topics_top_with_options() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "topics",
            "top",
            "-n",
            "25",
            "--days",
            "7",
        ]);
        match cli.command {
            Commands::Topics(TopicsCommand::Top { limit, days, .. }) => {
                assert_eq!(limit, 25);
                assert_eq!(days, 7);
            }
            _ => panic!("Expected Topics Top command"),
        }
    }

    #[test]
    fn test_cli_topics_refresh_scores() {
        let cli = Cli::parse_from(["memory-daemon", "topics", "refresh-scores"]);
        match cli.command {
            Commands::Topics(TopicsCommand::RefreshScores { db_path }) => {
                assert!(db_path.is_none());
            }
            _ => panic!("Expected Topics RefreshScores command"),
        }
    }

    #[test]
    fn test_cli_topics_refresh_scores_with_path() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "topics",
            "refresh-scores",
            "--db-path",
            "/custom/db",
        ]);
        match cli.command {
            Commands::Topics(TopicsCommand::RefreshScores { db_path }) => {
                assert_eq!(db_path, Some("/custom/db".to_string()));
            }
            _ => panic!("Expected Topics RefreshScores command"),
        }
    }

    #[test]
    fn test_cli_topics_prune() {
        let cli = Cli::parse_from(["memory-daemon", "topics", "prune"]);
        match cli.command {
            Commands::Topics(TopicsCommand::Prune {
                days,
                force,
                db_path,
            }) => {
                assert_eq!(days, 90);
                assert!(!force);
                assert!(db_path.is_none());
            }
            _ => panic!("Expected Topics Prune command"),
        }
    }

    #[test]
    fn test_cli_topics_prune_with_options() {
        let cli = Cli::parse_from([
            "memory-daemon",
            "topics",
            "prune",
            "--days",
            "60",
            "--force",
            "--db-path",
            "/custom/db",
        ]);
        match cli.command {
            Commands::Topics(TopicsCommand::Prune {
                days,
                force,
                db_path,
            }) => {
                assert_eq!(days, 60);
                assert!(force);
                assert_eq!(db_path, Some("/custom/db".to_string()));
            }
            _ => panic!("Expected Topics Prune command"),
        }
    }
}
