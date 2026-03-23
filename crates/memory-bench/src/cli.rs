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
        /// Compare against competitor baselines.
        #[arg(long)]
        compare: bool,
        /// Path to baselines TOML file.
        #[arg(long, default_value = "benchmarks/baselines.toml")]
        baselines: String,
    },
    /// Run LOCOMO adapter benchmark.
    Locomo {
        /// Path to LOCOMO dataset directory.
        #[arg(long)]
        dataset: String,
        /// Output file for JSON results.
        #[arg(long)]
        output: Option<String>,
        /// Compare against competitor baselines.
        #[arg(long)]
        compare: bool,
        /// Path to baselines TOML file.
        #[arg(long, default_value = "benchmarks/baselines.toml")]
        baselines: String,
    },
}
