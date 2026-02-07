//! # memory-types
//!
//! Shared domain types for the Agent Memory system.
//!
//! This crate defines the core data structures used throughout the system:
//! - Events: Immutable records of agent interactions
//! - TOC Nodes: Time-hierarchical table of contents entries
//! - Grips: Provenance anchors linking summaries to source events
//! - Segments: Groups of events for summarization
//! - Settings: Configuration types
//! - Salience: Memory importance scoring (Phase 16)
//! - Usage: Access pattern tracking (Phase 16)
//!
//! ## Usage
//!
//! ```rust
//! use memory_types::{Event, EventRole, EventType, Segment, Settings};
//! use memory_types::{MemoryKind, SalienceScorer, UsageStats};
//! ```

pub mod config;
pub mod error;
pub mod event;
pub mod grip;
pub mod outbox;
pub mod salience;
pub mod segment;
pub mod toc;
pub mod usage;

// Re-export main types at crate root
pub use config::{MultiAgentMode, NoveltyConfig, Settings, SummarizerSettings};
pub use error::MemoryError;
pub use event::{Event, EventRole, EventType};
pub use grip::Grip;
pub use outbox::{OutboxAction, OutboxEntry};
pub use salience::{
    calculate_salience, classify_memory_kind, default_salience, MemoryKind, SalienceConfig,
    SalienceScorer,
};
pub use segment::Segment;
pub use toc::{TocBullet, TocLevel, TocNode};
pub use usage::{usage_penalty, UsageConfig, UsageStats};
