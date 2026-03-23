//! CLI argument parsing using clap derive API.

use clap::{Parser, Subcommand};

/// Agent memory CLI -- query, ingest, and explore your memory store.
#[derive(Parser, Debug)]
#[command(name = "memory", version, about)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Commands,
}

/// Global arguments shared across all subcommands.
#[derive(Parser, Debug)]
pub struct GlobalArgs {
    /// Output format override (e.g., "json"). When stdout is not a TTY, JSON is used automatically.
    #[arg(long, global = true)]
    pub format: Option<String>,

    /// gRPC endpoint for the memory daemon.
    #[arg(long, global = true, default_value = "http://127.0.0.1:50051")]
    pub endpoint: String,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Orchestrated hybrid search over memory layers.
    Search(SearchArgs),

    /// Build a context window for the current conversation.
    Context(ContextArgs),

    /// Ingest a new memory event.
    Add(AddArgs),

    /// Browse the memory timeline.
    Timeline(TimelineArgs),

    /// Generate a summary of recent memory.
    Summary(SummaryArgs),

    /// Recall: search with LLM reranking (alias for search --rerank=llm).
    Recall(RecallArgs),

    /// Export daily markdown files from memory.
    Daily(DailyArgs),
}

/// Arguments for the `search` subcommand.
#[derive(Parser, Debug)]
pub struct SearchArgs {
    /// Search query.
    pub query: String,

    /// Number of results to return.
    #[arg(long, default_value_t = 10)]
    pub top: usize,

    /// Rerank mode (e.g., "heuristic", "llm").
    #[arg(long)]
    pub rerank: Option<String>,

    /// Output format override.
    #[arg(long)]
    pub format: Option<String>,
}

/// Arguments for the `context` subcommand.
#[derive(Parser, Debug)]
pub struct ContextArgs {
    /// Query to build context for.
    pub query: String,

    /// Output format override.
    #[arg(long)]
    pub format: Option<String>,
}

/// Arguments for the `add` subcommand.
#[derive(Parser, Debug)]
pub struct AddArgs {
    /// Content text to ingest.
    #[arg(long)]
    pub content: String,

    /// Event kind (e.g., "episodic", "semantic").
    #[arg(long, default_value = "episodic")]
    pub kind: String,

    /// Agent identifier.
    #[arg(long)]
    pub agent: Option<String>,
}

/// Arguments for the `timeline` subcommand.
#[derive(Parser, Debug)]
pub struct TimelineArgs {
    /// Filter by entity name.
    #[arg(long)]
    pub entity: Option<String>,

    /// Time range (e.g., "7d", "24h", "30d").
    #[arg(long, default_value = "7d")]
    pub range: String,

    /// Output format override.
    #[arg(long)]
    pub format: Option<String>,
}

/// Arguments for the `summary` subcommand.
#[derive(Parser, Debug)]
pub struct SummaryArgs {
    /// Summary range (e.g., "day", "week", "month").
    #[arg(long, default_value = "week")]
    pub range: String,

    /// Output format override.
    #[arg(long)]
    pub format: Option<String>,
}

/// Arguments for the `recall` subcommand.
#[derive(Parser, Debug)]
pub struct RecallArgs {
    /// Query to recall.
    pub query: String,

    /// Output format override.
    #[arg(long)]
    pub format: Option<String>,
}

/// Arguments for the `daily` subcommand.
#[derive(Parser, Debug)]
pub struct DailyArgs {
    /// Time range for export (e.g., "7d", "30d"). Default: today only.
    #[arg(long)]
    pub range: Option<String>,

    /// Output directory for markdown files.
    #[arg(long, default_value = "./memory")]
    pub dir: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_parse_search() {
        let cli = Cli::try_parse_from(["memory", "search", "test query"]).unwrap();
        match cli.command {
            Commands::Search(args) => {
                assert_eq!(args.query, "test query");
                assert_eq!(args.top, 10);
                assert!(args.rerank.is_none());
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_parse_search_with_options() {
        let cli =
            Cli::try_parse_from(["memory", "search", "hello", "--top", "5", "--rerank", "llm"])
                .unwrap();
        match cli.command {
            Commands::Search(args) => {
                assert_eq!(args.query, "hello");
                assert_eq!(args.top, 5);
                assert_eq!(args.rerank.as_deref(), Some("llm"));
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_parse_context() {
        let cli = Cli::try_parse_from(["memory", "context", "what happened"]).unwrap();
        match cli.command {
            Commands::Context(args) => {
                assert_eq!(args.query, "what happened");
            }
            _ => panic!("Expected Context command"),
        }
    }

    #[test]
    fn test_parse_add() {
        let cli = Cli::try_parse_from(["memory", "add", "--content", "hello world"]).unwrap();
        match cli.command {
            Commands::Add(args) => {
                assert_eq!(args.content, "hello world");
                assert_eq!(args.kind, "episodic");
                assert!(args.agent.is_none());
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_parse_add_with_agent() {
        let cli = Cli::try_parse_from([
            "memory",
            "add",
            "--content",
            "event",
            "--kind",
            "semantic",
            "--agent",
            "claude",
        ])
        .unwrap();
        match cli.command {
            Commands::Add(args) => {
                assert_eq!(args.content, "event");
                assert_eq!(args.kind, "semantic");
                assert_eq!(args.agent.as_deref(), Some("claude"));
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_parse_timeline() {
        let cli = Cli::try_parse_from(["memory", "timeline"]).unwrap();
        match cli.command {
            Commands::Timeline(args) => {
                assert_eq!(args.range, "7d");
                assert!(args.entity.is_none());
            }
            _ => panic!("Expected Timeline command"),
        }
    }

    #[test]
    fn test_parse_summary() {
        let cli = Cli::try_parse_from(["memory", "summary"]).unwrap();
        match cli.command {
            Commands::Summary(args) => {
                assert_eq!(args.range, "week");
            }
            _ => panic!("Expected Summary command"),
        }
    }

    #[test]
    fn test_parse_recall() {
        let cli = Cli::try_parse_from(["memory", "recall", "what did I say"]).unwrap();
        match cli.command {
            Commands::Recall(args) => {
                assert_eq!(args.query, "what did I say");
            }
            _ => panic!("Expected Recall command"),
        }
    }

    #[test]
    fn test_global_args_default_endpoint() {
        let cli = Cli::try_parse_from(["memory", "search", "test"]).unwrap();
        assert_eq!(cli.global.endpoint, "http://127.0.0.1:50051");
        assert!(cli.global.format.is_none());
    }

    #[test]
    fn test_global_args_custom_endpoint() {
        let cli = Cli::try_parse_from([
            "memory",
            "--endpoint",
            "http://localhost:9090",
            "search",
            "test",
        ])
        .unwrap();
        assert_eq!(cli.global.endpoint, "http://localhost:9090");
    }

    #[test]
    fn test_parse_daily() {
        let cli = Cli::try_parse_from(["memory", "daily"]).unwrap();
        match cli.command {
            Commands::Daily(args) => {
                assert!(args.range.is_none());
                assert_eq!(args.dir, "./memory");
            }
            _ => panic!("Expected Daily command"),
        }
    }

    #[test]
    fn test_parse_daily_with_options() {
        let cli =
            Cli::try_parse_from(["memory", "daily", "--range", "7d", "--dir", "/tmp/out"]).unwrap();
        match cli.command {
            Commands::Daily(args) => {
                assert_eq!(args.range.as_deref(), Some("7d"));
                assert_eq!(args.dir, "/tmp/out");
            }
            _ => panic!("Expected Daily command"),
        }
    }

    #[test]
    fn test_all_subcommands_parse() {
        // Verify all 7 subcommands can be parsed
        let cases = vec![
            vec!["memory", "search", "q"],
            vec!["memory", "context", "q"],
            vec!["memory", "add", "--content", "c"],
            vec!["memory", "timeline"],
            vec!["memory", "summary"],
            vec!["memory", "recall", "q"],
            vec!["memory", "daily"],
        ];
        for args in cases {
            Cli::try_parse_from(&args)
                .unwrap_or_else(|e| panic!("Failed to parse {:?}: {}", args, e));
        }
    }
}
