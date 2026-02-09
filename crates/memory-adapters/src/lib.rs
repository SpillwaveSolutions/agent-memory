//! # memory-adapters
//!
//! Agent adapter SDK for multi-agent memory integration.
//!
//! This crate provides the foundation for building adapters that connect
//! various AI agent CLIs (OpenCode, Gemini CLI, Copilot CLI) to Agent Memory.
//!
//! ## Core Components
//!
//! - [`AgentAdapter`]: Trait that all adapters must implement
//! - [`AdapterConfig`]: Configuration for adapter-specific settings
//! - [`AdapterError`]: Error types for adapter operations
//! - [`RawEvent`]: Raw event data before normalization
//!
//! ## Usage
//!
//! Implement the `AgentAdapter` trait for your agent:
//!
//! ```rust,ignore
//! use memory_adapters::{AgentAdapter, AdapterConfig, AdapterError, RawEvent};
//! use memory_types::Event;
//!
//! struct MyAgentAdapter;
//!
//! #[async_trait::async_trait]
//! impl AgentAdapter for MyAgentAdapter {
//!     fn agent_id(&self) -> &str { "myagent" }
//!     fn display_name(&self) -> &str { "My Agent CLI" }
//!     fn normalize(&self, raw: RawEvent) -> Result<Event, AdapterError> {
//!         // Convert raw event to unified format
//!         todo!()
//!     }
//!     fn load_config(&self, path: Option<&std::path::Path>) -> Result<AdapterConfig, AdapterError> {
//!         Ok(AdapterConfig::default())
//!     }
//! }
//! ```

pub mod adapter;
pub mod config;
pub mod error;

// Re-export main types at crate root
pub use adapter::{AgentAdapter, RawEvent};
pub use config::AdapterConfig;
pub use error::AdapterError;
