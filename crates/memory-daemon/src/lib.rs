//! Memory daemon library exports.
//!
//! This crate provides the CLI daemon binary for the Agent Memory system.
//!
//! # Modules
//!
//! - `cli`: Command-line argument parsing with clap
//! - `commands`: Command implementations (start, stop, status)

pub mod cli;
pub mod commands;

pub use cli::{
    AdminCommands, AgentsCommand, Cli, Commands, QueryCommands, RetrievalCommand,
    SchedulerCommands, TeleportCommand, TopicsCommand,
};
pub use commands::{
    handle_admin, handle_agents_command, handle_query, handle_retrieval_command, handle_scheduler,
    handle_teleport_command, handle_topics_command, show_status, start_daemon, stop_daemon,
};
