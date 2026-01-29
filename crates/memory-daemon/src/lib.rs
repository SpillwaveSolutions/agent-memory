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

pub use cli::{Cli, Commands};
pub use commands::{show_status, start_daemon, stop_daemon};
