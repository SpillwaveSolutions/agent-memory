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
//! use memory_types::Event;
//! ```

/// Placeholder module for event types.
/// Will be implemented in Phase 1, Plan 02.
pub mod event {
    /// Placeholder for Event type.
    /// Events are immutable records of agent interactions.
    pub struct Event;
}

/// Placeholder module for TOC (Table of Contents) node types.
/// Will be implemented in Phase 1, Plan 02.
pub mod toc {
    /// Placeholder for TocNode type.
    /// TOC nodes form a time-based hierarchy for navigation.
    pub struct TocNode;
}

/// Placeholder module for grip types.
/// Will be implemented in Phase 3.
pub mod grip {
    /// Placeholder for Grip type.
    /// Grips anchor summary bullets to source events.
    pub struct Grip;
}

/// Placeholder module for settings types.
/// Will be implemented in Phase 1, Plan 02.
pub mod settings {
    /// Placeholder for Settings type.
    /// Configuration for the memory daemon.
    pub struct Settings;
}

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        // Placeholder test to verify crate compiles
        assert!(true);
    }
}
