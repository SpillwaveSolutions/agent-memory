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
//! use memory_types::{Event, EventRole, EventType};
//! ```

pub mod error;
pub mod event;
pub mod outbox;

// Re-export main types at crate root
pub use error::MemoryError;
pub use event::{Event, EventRole, EventType};
pub use outbox::{OutboxAction, OutboxEntry};
