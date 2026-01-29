//! # memory-types
//!
//! Shared domain types for the Agent Memory system.
//!
//! This crate defines the core data structures used throughout the system:
//! - Events: Immutable records of agent interactions
//! - TOC Nodes: Time-hierarchical table of contents entries
//! - Grips: Provenance anchors linking summaries to source events
//! - Settings: Configuration types
//!
//! ## Usage
//!
//! ```rust
//! use memory_types::{Event, EventRole, EventType, Settings};
//! ```

pub mod config;
pub mod error;
pub mod event;
pub mod grip;
pub mod outbox;
pub mod toc;

// Re-export main types at crate root
pub use config::{MultiAgentMode, Settings, SummarizerSettings};
pub use error::MemoryError;
pub use event::{Event, EventRole, EventType};
pub use grip::Grip;
pub use outbox::{OutboxAction, OutboxEntry};
pub use toc::{TocBullet, TocLevel, TocNode};
