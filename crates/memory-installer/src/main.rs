use std::path::PathBuf;

use clap::{Parser, Subcommand};
use memory_installer::types::Runtime;

#[derive(Parser, Debug)]
#[command(name = "memory-installer")]
#[command(about = "Install memory-agent plugins for various AI runtimes")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Install memory plugins for an AI runtime
    Install {
        /// Target runtime
        #[arg(long, value_enum)]
        agent: Runtime,

        /// Install to project directory (e.g., ./.claude/)
        #[arg(long, conflicts_with = "global")]
        project: bool,

        /// Install to global user directory (e.g., ~/.claude/)
        #[arg(long, conflicts_with = "project")]
        global: bool,

        /// Custom target directory (required with --agent skills)
        #[arg(long)]
        dir: Option<PathBuf>,

        /// Preview what would be installed without writing files
        #[arg(long)]
        dry_run: bool,

        /// Path to canonical source root (defaults to auto-discovery)
        #[arg(long)]
        source: Option<PathBuf>,
    },
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Install {
            agent,
            project,
            global,
            dir,
            dry_run: _,
            source: _,
        } => {
            if !project && !global && dir.is_none() {
                eprintln!("error: one of --project, --global, or --dir is required");
                std::process::exit(1);
            }

            if agent == Runtime::Skills && dir.is_none() {
                eprintln!("error: --agent skills requires --dir <path>");
                std::process::exit(1);
            }

            println!(
                "Install for {:?} not yet wired (pending parser + writer)",
                agent
            );
        }
    }
}
