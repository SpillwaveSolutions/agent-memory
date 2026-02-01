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

pub use cli::{AdminCommands, Cli, Commands, QueryCommands, SchedulerCommands};
pub use commands::{
    handle_admin, handle_query, handle_scheduler, show_status, start_daemon, stop_daemon,
};
