# Phase 18 Plan 02: Create memory-adapters crate with AgentAdapter trait

## Status: COMPLETED

## Summary

Created the `memory-adapters` crate providing the foundation SDK for multi-agent memory integration. This crate defines the common interface that all agent adapters (Claude, OpenCode, Gemini, Copilot) will implement.

## Artifacts Created

### Files Created

| File | Purpose |
|------|---------|
| `crates/memory-adapters/Cargo.toml` | Crate manifest with dependencies |
| `crates/memory-adapters/src/lib.rs` | Crate root with module structure and re-exports |
| `crates/memory-adapters/src/adapter.rs` | AgentAdapter trait and RawEvent struct |
| `crates/memory-adapters/src/config.rs` | AdapterConfig struct with builder pattern |
| `crates/memory-adapters/src/error.rs` | AdapterError enum with helper constructors |

### Files Modified

| File | Change |
|------|--------|
| `Cargo.toml` | Added memory-adapters to workspace members (alphabetical order) |

## Key Components

### AgentAdapter Trait

The core trait that all agent adapters must implement:

```rust
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    fn agent_id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn normalize(&self, raw: RawEvent) -> Result<Event, AdapterError>;
    fn load_config(&self, path: Option<&Path>) -> Result<AdapterConfig, AdapterError>;
    fn detect(&self) -> bool { false }  // default
    fn is_available(&self) -> bool { true }  // default
}
```

### RawEvent Struct

Raw event data before normalization with builder pattern:

```rust
RawEvent::new("evt-1", timestamp_ms, "content")
    .with_event_type("user_message")
    .with_role("user")
    .with_session_id("session-123")
    .with_metadata("key", "value")
```

### AdapterConfig Struct

Configuration with builder pattern and serde support:

```rust
AdapterConfig::with_event_source("/var/log/agent.log")
    .with_ingest_target("http://localhost:50051")
    .with_setting("poll_interval_ms", "1000")
```

### AdapterError Enum

Error types with helper constructors:

- `Config { path, message }` - Configuration errors
- `Normalize(String)` - Event normalization failures
- `Io(std::io::Error)` - IO errors
- `Parse(String)` - Parsing errors
- `Detection(String)` - Agent detection failures

## Prerequisites

The `Event` struct in `memory-types` was previously updated to include:
- `agent: Option<String>` field with `#[serde(default)]`
- `with_agent()` builder method

This was required for the adapter tests to compile and pass.

## Verification Results

| Check | Result |
|-------|--------|
| `cargo build -p memory-adapters` | PASS |
| `cargo test -p memory-adapters` | PASS (19 tests) |
| `cargo clippy -p memory-adapters -- -D warnings` | PASS (no warnings) |
| `cargo doc -p memory-adapters --no-deps` | PASS |
| Workspace membership | Verified via `cargo metadata` |

## Dependencies

```toml
[dependencies]
async-trait = "0.1"
memory-types = { path = "../memory-types" }
serde = { version = "1.0", features = ["derive"] }
thiserror = "2.0"
tracing = "0.1"
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde_json = "1.0"
```

## Next Steps

This crate provides the foundation for:
- **Plan 18-03**: Implement ClaudeAdapter for Claude Code
- **Plan 18-04**: Implement OpenCodeAdapter for OpenCode CLI
- Future adapters for Gemini CLI and GitHub Copilot CLI

## Implementation Notes

1. The trait uses `async_trait` for async compatibility though the base methods are sync
2. `normalize_agent_id()` is a static helper for consistent ID normalization
3. `detect()` and `is_available()` have sensible defaults (false and true respectively)
4. Manual `Default` implementation ensures `enabled: true` for both Rust defaults and serde deserialization
