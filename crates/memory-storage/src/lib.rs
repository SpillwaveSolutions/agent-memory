//! Storage layer for agent-memory system.
//!
//! Provides RocksDB-backed storage with:
//! - Column family isolation for different data types (STOR-02)
//! - Time-prefixed keys for efficient range scans (STOR-01)
//! - Atomic writes via WriteBatch (ING-05)
//! - Idempotent event writes (ING-03)
//! - Checkpoint-based crash recovery (STOR-03)

pub mod column_families;
pub mod db;
pub mod error;
pub mod keys;

pub use db::{Storage, StorageStats};
pub use error::StorageError;
pub use keys::{EventKey, OutboxKey, CheckpointKey};
