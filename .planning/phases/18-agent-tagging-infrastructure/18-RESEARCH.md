# Phase 18: Agent Tagging Infrastructure - Research

**Researched:** 2026-02-08
**Domain:** Rust protobuf extensions, trait patterns, SQLite schema migrations, clap CLI
**Confidence:** HIGH

## Summary

Phase 18 adds multi-agent support to the agent-memory system by introducing an `agent` field to track which AI agent produced each event. This phase is foundational for cross-agent memory unification - the core value proposition where memories from Claude Code, OpenCode, Gemini CLI, and other agents can be queried together or filtered by source.

The implementation requires coordinated changes across four layers:
1. **Proto/Types Layer**: Add `agent` field to Event message
2. **Storage Layer**: Add agent to RocksDB column family indexes for efficient filtering
3. **CLI Layer**: Add `--agent` filter to query commands
4. **SDK Layer**: Create new `memory-adapters` crate with common adapter trait

**Primary recommendation:** Extend the existing Event model with an `agent: Option<String>` field using serde defaults for backward compatibility, following the Phase 16 pattern used for salience fields.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| prost | 0.13 | Protobuf code generation | Already in workspace, proven patterns |
| serde | 1.0 | Serialization with defaults | Already in workspace, backward compat |
| clap | 4.5 | CLI argument parsing | Already in workspace, derive macro patterns |
| async-trait | 0.1 | Async trait bounds | Already in workspace, used in memory-retrieval |
| thiserror | 2.0 | Error types | Already in workspace, consistent error handling |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| rocksdb | 0.22 | Storage indexing | Already in use, no changes needed |
| chrono | 0.4 | Timestamp handling | For agent activity tracking |
| tracing | 0.1 | Logging | For adapter operations |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `Option<String>` for agent | Enum of known agents | String is more extensible for future agents |
| Separate agent column family | Index within events CF | Separate CF adds complexity, embedded index simpler |

**Installation:**
No new dependencies needed. All required crates already in workspace.

## Architecture Patterns

### Recommended Project Structure

```
crates/
├── memory-types/src/
│   └── event.rs           # Add agent field
├── memory-storage/src/
│   └── db.rs              # Add agent index methods
├── memory-daemon/src/
│   └── cli.rs             # Add --agent filter
└── memory-adapters/       # NEW CRATE
    ├── Cargo.toml
    └── src/
        ├── lib.rs         # Trait re-exports
        ├── adapter.rs     # Adapter trait definition
        ├── normalize.rs   # Event normalization
        └── config.rs      # Configuration loading
```

### Pattern 1: Optional Field with Serde Default

**What:** Add optional `agent` field with serde default for backward compatibility.
**When to use:** When extending existing serialized types that may have old data.
**Example:**

```rust
// Source: crates/memory-types/src/event.rs (Phase 16 pattern)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_id: String,
    pub session_id: String,
    // ... existing fields ...

    // Phase 18: Agent identifier (backward compatible)
    /// Agent that produced this event (e.g., "claude", "opencode", "gemini").
    /// Default: None for existing v2.0.0 data.
    #[serde(default)]
    pub agent: Option<String>,
}
```

### Pattern 2: Proto3 Optional Fields

**What:** Use `optional` keyword in proto3 for fields that may be absent.
**When to use:** For fields added after initial schema that may not be present.
**Example:**

```protobuf
// Source: proto/memory.proto
message Event {
    string event_id = 1;
    string session_id = 2;
    int64 timestamp_ms = 3;
    EventType event_type = 4;
    EventRole role = 5;
    string text = 6;
    map<string, string> metadata = 7;

    // Phase 18: Agent identifier
    optional string agent = 8;
}
```

### Pattern 3: Trait-Based SDK Pattern

**What:** Define traits that adapters implement, with default implementations where useful.
**When to use:** When building an extensible SDK for multiple implementations.
**Example:**

```rust
// Source: Based on memory-retrieval/src/executor.rs LayerExecutor pattern
use async_trait::async_trait;

/// Adapter for a specific AI agent CLI tool.
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    /// Agent identifier (e.g., "claude", "opencode", "gemini").
    fn agent_id(&self) -> &str;

    /// Convert agent-specific event format to unified Event.
    fn normalize_event(&self, raw: RawEvent) -> Result<Event, AdapterError>;

    /// Load adapter-specific configuration.
    fn load_config(&self, path: Option<&Path>) -> Result<AdapterConfig, AdapterError>;

    /// Auto-detect if this adapter should be used based on environment.
    fn detect(&self) -> bool {
        false  // Default: must be explicitly selected
    }
}
```

### Pattern 4: CLI Optional Filter Pattern

**What:** Add optional filter argument that, when absent, returns all results.
**When to use:** For filters that should default to "no filter" behavior.
**Example:**

```rust
// Source: crates/memory-daemon/src/cli.rs (existing pattern)
#[derive(Subcommand, Debug, Clone)]
pub enum TeleportCommand {
    Search {
        query: String,

        /// Filter results by agent (e.g., "claude", "opencode")
        #[arg(long, short = 'a')]
        agent: Option<String>,

        #[arg(long, short = 't', default_value = "all")]
        doc_type: String,
        // ...
    },
}
```

### Anti-Patterns to Avoid

- **Hardcoding agent names:** Use strings, not enums, to allow new agents without code changes
- **Breaking backward compatibility:** Always use serde defaults and optional proto fields
- **Agent-specific storage paths:** Store all agents in same RocksDB, use filtering
- **Requiring agent on all events:** Keep it optional for backward compatibility

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Agent detection from environment | Custom env parsing | `std::env::var` with fallback chain | Platform differences |
| Configuration file loading | Manual TOML parsing | `config` crate (already in workspace) | Already handles layered configs |
| Proto code generation | Manual struct matching | `prost-build` with tonic | Maintains type safety |
| Optional field serialization | Custom Option handling | serde(default) attribute | Handles missing fields correctly |

**Key insight:** The project already has established patterns for backward-compatible schema evolution (Phase 16 salience fields) and trait-based SDKs (memory-retrieval contracts). Reuse these patterns.

## Common Pitfalls

### Pitfall 1: Breaking Deserialization of Old Events

**What goes wrong:** Adding a required field breaks reading of pre-phase-18 events.
**Why it happens:** Forgot to add `#[serde(default)]` or used non-optional proto field.
**How to avoid:** Always use `Option<T>` with `#[serde(default)]` in Rust, `optional` in proto3.
**Warning signs:** Deserialization errors when reading existing RocksDB data.

### Pitfall 2: Inefficient Agent Filtering

**What goes wrong:** Filtering by agent requires full table scan of all events.
**Why it happens:** No index on agent field in storage layer.
**How to avoid:** Either create agent prefix index or accept that agent filtering is post-retrieval (simpler for Phase 18).
**Warning signs:** Query performance degrades linearly with event count when filtering by agent.

**Recommendation for Phase 18:** Start with post-retrieval filtering (simple), add index in Phase 20 if needed.

### Pitfall 3: Inconsistent Agent IDs

**What goes wrong:** Same agent has different IDs ("claude", "Claude", "claude-code").
**Why it happens:** No normalization of agent identifiers.
**How to avoid:** Normalize to lowercase in adapter, document canonical names.
**Warning signs:** Queries for "claude" miss events tagged as "Claude".

### Pitfall 4: Circular Dependency in Crate Graph

**What goes wrong:** memory-adapters depends on memory-types which depends back on adapters.
**Why it happens:** Trying to put too much in the adapters crate.
**How to avoid:** Keep adapters as pure consumers of memory-types; never add memory-adapters as dependency of core crates.
**Warning signs:** `cargo build` fails with circular dependency error.

## Code Examples

### Adding Agent Field to Event Proto

```protobuf
// Source: proto/memory.proto
// Add to existing Event message

message Event {
    // ... existing fields 1-7 ...

    // Phase 18: Agent identifier
    // Empty string or absent means unknown/legacy event
    optional string agent = 8;
}
```

### Adding Agent Field to Rust Event

```rust
// Source: crates/memory-types/src/event.rs
// Add to existing Event struct

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    // ... existing fields ...

    /// Agent that produced this event.
    ///
    /// Common values: "claude", "opencode", "gemini", "copilot"
    /// Default: None for pre-phase-18 events.
    #[serde(default)]
    pub agent: Option<String>,
}

impl Event {
    /// Create a new event with agent identifier.
    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }
}
```

### Adapter Trait Definition

```rust
// Source: crates/memory-adapters/src/adapter.rs
use async_trait::async_trait;
use memory_types::Event;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Normalization error: {0}")]
    Normalize(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Raw event data before normalization.
#[derive(Debug, Clone)]
pub struct RawEvent {
    pub id: String,
    pub timestamp_ms: i64,
    pub content: String,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Adapter-specific configuration.
#[derive(Debug, Clone, Default)]
pub struct AdapterConfig {
    /// Path to agent's log/history file
    pub event_source_path: Option<std::path::PathBuf>,

    /// Additional agent-specific settings
    pub settings: std::collections::HashMap<String, String>,
}

/// Trait for agent-specific adapters.
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    /// Canonical agent identifier (lowercase, e.g., "claude", "opencode").
    fn agent_id(&self) -> &str;

    /// Human-readable agent name (e.g., "Claude Code", "OpenCode CLI").
    fn display_name(&self) -> &str;

    /// Convert raw event to unified Event format.
    fn normalize(&self, raw: RawEvent) -> Result<Event, AdapterError>;

    /// Load adapter configuration from path or default location.
    fn load_config(&self, path: Option<&Path>) -> Result<AdapterConfig, AdapterError>;

    /// Attempt to auto-detect this adapter from environment.
    fn detect(&self) -> bool {
        false
    }
}
```

### CLI Agent Filter

```rust
// Source: crates/memory-daemon/src/cli.rs
// Add to TeleportCommand::Search

#[derive(Subcommand, Debug, Clone)]
pub enum TeleportCommand {
    Search {
        query: String,

        /// Filter results to a specific agent
        #[arg(long, short = 'a')]
        agent: Option<String>,

        // ... existing fields ...
    },
    // ... other commands ...
}
```

### TOC Node Agent Tracking

```rust
// Source: crates/memory-types/src/toc.rs
// Add to TocNode struct

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocNode {
    // ... existing fields ...

    /// Agents that contributed events to this time period.
    ///
    /// Populated during TOC building when events from multiple
    /// agents fall within the same time window.
    #[serde(default)]
    pub contributing_agents: Vec<String>,
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single-agent memory | Multi-agent unified memory | This phase | Enables cross-agent queries |
| Implicit agent detection | Explicit agent field | This phase | Reliable agent identification |
| No adapter SDK | Trait-based adapters | This phase | Extensible agent integration |

**Deprecated/outdated:**
- None - this is new functionality

## Open Questions

1. **Agent Detection Strategy**
   - What we know: Can use environment variables (CLAUDE_CODE_ENV, etc.)
   - What's unclear: Which env vars are reliable across versions
   - Recommendation: Start with explicit `--agent` flag, add auto-detect later

2. **Agent ID Normalization**
   - What we know: Should be lowercase, no spaces
   - What's unclear: Canonical list of agent IDs
   - Recommendation: Document canonical names in adapter trait docs, normalize in normalize() method

3. **Performance of Agent Filtering**
   - What we know: Post-retrieval filtering is O(n) on result set
   - What's unclear: Whether this is acceptable at scale
   - Recommendation: Start simple (post-retrieval), add index in Phase 20 if metrics show need

## Sources

### Primary (HIGH confidence)

- crates/memory-types/src/event.rs - Existing Event model (Phase 16 salience pattern)
- crates/memory-types/src/toc.rs - TocNode structure for contributing_agents
- crates/memory-retrieval/src/ - Trait patterns for executor and contracts
- proto/memory.proto - Existing protobuf schema

### Secondary (MEDIUM confidence)

- crates/memory-daemon/src/cli.rs - CLI argument patterns with clap
- crates/memory-storage/src/db.rs - Storage patterns for optional fields

### Tertiary (LOW confidence)

- None - all findings verified against codebase

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All dependencies already in workspace
- Architecture: HIGH - Based on existing Phase 16 and memory-retrieval patterns
- Pitfalls: HIGH - Common Rust/protobuf schema evolution issues well-known
- Adapter SDK: MEDIUM - First SDK crate, but follows memory-retrieval trait patterns

**Research date:** 2026-02-08
**Valid until:** 90 days (stable Rust patterns, no fast-moving dependencies)

---

## Implementation Checklist

The planner should verify these items are covered:

- [ ] Add `agent` field to proto/memory.proto Event message
- [ ] Add `agent` field to memory-types Event struct with serde(default)
- [ ] Add `contributing_agents` field to memory-types TocNode struct
- [ ] Update ingest handler to extract agent from proto Event
- [ ] Create memory-adapters crate with Cargo.toml
- [ ] Define AgentAdapter trait in memory-adapters
- [ ] Define AdapterConfig and AdapterError types
- [ ] Add `--agent` filter to TeleportSearch CLI command
- [ ] Add `--agent` filter to other query commands
- [ ] Update RouteQuery to support agent filtering
- [ ] Add agent field to search results metadata
- [ ] Unit tests for backward compatibility (read old events)
- [ ] Unit tests for new events with agent field
- [ ] Integration test for agent filtering
