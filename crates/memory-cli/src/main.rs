//! `memory` CLI binary -- simple interface for querying and ingesting agent memory.

mod cli;
mod client;
mod commands;
mod output;

use clap::Parser;
use cli::{Cli, Commands};
use output::JsonEnvelope;
use std::process;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Search(args) => commands::search::run(args, &cli.global).await,
        Commands::Context(args) => commands::context::run(args, &cli.global).await,
        Commands::Add(args) => commands::add::run(args, &cli.global).await,
        Commands::Timeline(args) => commands::timeline::run(args, &cli.global).await,
        Commands::Summary(args) => commands::summary::run(args, &cli.global).await,
        Commands::Recall(args) => commands::recall::run(args, &cli.global).await,
    };

    if let Err(err) = result {
        let envelope = JsonEnvelope::error(&format!("{err:#}"));
        eprintln!("{}", envelope.to_json_string());
        process::exit(1);
    }
}
