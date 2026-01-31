//! Client library for Agent Memory daemon.
//!
//! This crate provides:
//! - `MemoryClient` for connecting to the daemon and ingesting events
//! - Hook event mapping for converting code_agent_context_hooks events
//!
//! # Example
//!
//! ```rust,no_run
//! use memory_client::{MemoryClient, HookEvent, HookEventType, map_hook_event};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to daemon
//!     let mut client = MemoryClient::connect("http://[::1]:50051").await?;
//!
//!     // Create a hook event
//!     let hook = HookEvent::new("session-1", HookEventType::UserPromptSubmit, "Hello!");
//!
//!     // Map to memory event and ingest
//!     let event = map_hook_event(hook);
//!     let (event_id, created) = client.ingest(event).await?;
//!
//!     println!("Ingested: {} (created: {})", event_id, created);
//!     Ok(())
//! }
//! ```
//!
//! # Requirements
//!
//! - HOOK-02: Hook handlers call daemon's IngestEvent RPC
//! - HOOK-03: Event types map 1:1 from hook events

pub mod client;
pub mod error;
pub mod hook_mapping;

pub use client::{
    BrowseTocResult, ExpandGripResult, GetEventsResult,
    MemoryClient, DEFAULT_ENDPOINT,
};
pub use error::ClientError;
pub use hook_mapping::{map_hook_event, HookEvent, HookEventType};

// Re-export Event type for convenience
pub use memory_types::Event;
