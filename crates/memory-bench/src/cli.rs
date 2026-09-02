use clap::{Parser, Subcommand};

/// Benchmark suite for Agent Memory.
#[derive(Parser)]
#[command(name = "memory-bench", about = "Benchmark suite for Agent Memory")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to memory binary (default: searches PATH).
    #[arg(long, global = true, default_value = "memory")]
    pub memory_bin: String,

    /// Retrieval backend: `mock` (isolated in-process) or `cli` (running daemon).
    #[arg(long, global = true, default_value = "mock")]
    pub backend: String,

    /// gRPC endpoint for `--backend cli` (ignored when isolation spawns a daemon).
    #[arg(long, global = true, default_value = "http://127.0.0.1:50051")]
    pub endpoint: String,

    /// Path to memory-daemon binary (used by `--isolation daemon-per-conversation`).
    #[arg(long, global = true, default_value = "memory-daemon")]
    pub daemon_bin: String,
}

/// Available benchmark subcommands.
#[derive(Subcommand)]
pub enum Commands {
    /// Run temporal recall benchmarks.
    Temporal {
        /// Path to fixtures directory.
        #[arg(long, default_value = "benchmarks/fixtures")]
        fixtures: String,
        /// Output file for JSON results.
        #[arg(long)]
        output: Option<String>,
    },
    /// Run multi-session reasoning benchmarks.
    Multisession {
        /// Path to fixtures directory.
        #[arg(long, default_value = "benchmarks/fixtures")]
        fixtures: String,
        /// Output file for JSON results.
        #[arg(long)]
        output: Option<String>,
    },
    /// Run compression efficiency benchmarks.
    Compression {
        /// Path to fixtures directory.
        #[arg(long, default_value = "benchmarks/fixtures")]
        fixtures: String,
        /// Output file for JSON results.
        #[arg(long)]
        output: Option<String>,
    },
    /// Run full custom benchmark suite (all categories).
    All {
        /// Path to fixtures directory.
        #[arg(long, default_value = "benchmarks/fixtures")]
        fixtures: String,
        /// Output file for JSON results.
        #[arg(long)]
        output: Option<String>,
        /// Compare against competitor baselines (labeled, incommensurable).
        #[arg(long)]
        compare: bool,
        /// Path to baselines TOML file.
        #[arg(long, default_value = "benchmarks/baselines.toml")]
        baselines: String,
    },
    /// Run LOCOMO adapter. Substring mode is `context_hit_rate`, not a LOCOMO score.
    Locomo {
        /// Path to LOCOMO dataset file or directory (`locomo10.json`).
        #[arg(long)]
        dataset: String,
        /// Output file for JSON results.
        #[arg(long)]
        output: Option<String>,
        /// `mock` (context_hit_rate) or `llm-judge` (locomo_llm_judge).
        #[arg(long, default_value = "mock")]
        scorer: String,
        /// Top-k retrieved snippets passed to the generator/judge.
        #[arg(long, default_value_t = 5)]
        top: usize,
        /// Compare against competitor baselines. Refused for mock scorer.
        #[arg(long)]
        compare: bool,
        /// Path to baselines TOML file.
        #[arg(long, default_value = "benchmarks/baselines.toml")]
        baselines: String,
        /// Isolation: `daemon-per-conversation` (default for `--backend cli`) or `shared`.
        #[arg(long, value_parser = ["daemon-per-conversation", "shared"])]
        isolation: Option<String>,
        /// Cap total questions across conversations (60-02 dry-run).
        #[arg(long)]
        limit_questions: Option<usize>,
    },
    /// CI smoke: 1-conversation fixture + mock backend + mock judge.
    Smoke {
        /// Path to the 1-conversation fixture (real locomo10.json shape).
        #[arg(long, default_value = "benchmarks/fixtures/locomo-smoke.json")]
        dataset: String,
        /// Output file for JSON results.
        #[arg(long)]
        output: Option<String>,
    },
}
