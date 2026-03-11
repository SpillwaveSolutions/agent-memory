# Phase 5 Research: Integration

## Overview

Phase 5 connects the memory system to external hook handlers and provides CLI tools for querying and administration.

## Requirements

- **HOOK-02**: Hook handlers call daemon's IngestEvent RPC
- **HOOK-03**: Event types map 1:1 from hook events (SessionStart, UserPromptSubmit, PostToolUse, Stop, etc.)
- **CLI-02**: Query CLI for manual TOC navigation and testing
- **CLI-03**: Admin commands: rebuild-toc, compact, status

## Technical Analysis

### Hook Handler Integration (05-01)

**gRPC Client Library**

Need to expose a client API that hook handlers can use:

```rust
// memory-client crate
pub struct MemoryClient {
    inner: memory_service::MemoryServiceClient<Channel>,
}

impl MemoryClient {
    pub async fn connect(endpoint: &str) -> Result<Self, Error>;
    pub async fn ingest(&self, event: Event) -> Result<IngestResponse, Error>;
}
```

**Event Type Mapping**

Hook events from code_agent_context_hooks:
- `SessionStart` → `EventType::SessionStart`
- `UserPromptSubmit` → `EventType::UserMessage`
- `PostToolUse` → `EventType::ToolUse`
- `Stop` → `EventType::SessionEnd`

Mapping function:

```rust
pub fn map_hook_event(hook_event: HookEvent) -> memory_types::Event {
    // Convert hook event fields to memory event
}
```

### Query CLI (05-02)

**Commands to Add**

```bash
memory-daemon query root                    # List year nodes
memory-daemon query node <node_id>          # Get specific node
memory-daemon query browse <parent_id> [--limit N]  # Browse children
memory-daemon query events --from TS --to TS [--limit N]  # Get events
memory-daemon query expand <grip_id>        # Expand grip context
```

**Implementation**

Add `Query` subcommand to existing CLI with nested subcommands.

### Admin Commands (05-03)

**Commands to Add**

```bash
memory-daemon admin rebuild-toc [--from-date DATE]  # Rebuild TOC from events
memory-daemon admin compact                         # Trigger RocksDB compaction
memory-daemon admin stats                           # Show database statistics
```

**Implementation**

- `rebuild-toc`: Re-run segmentation and summarization from raw events
- `compact`: Call `db.compact_range()` on RocksDB
- `stats`: Show event count, TOC node count, grip count, disk usage

## Dependencies

- `tonic` for gRPC client
- Existing `memory-service` proto definitions
- `tokio` for async runtime

## File Structure

```
crates/
├── memory-client/          # New crate for client API
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── hook_mapping.rs
├── memory-daemon/
│   └── src/
│       ├── cli.rs          # Update with new commands
│       ├── commands/
│       │   ├── query.rs    # New: query commands
│       │   └── admin.rs    # New: admin commands
└── memory-service/
    └── src/
        └── admin.rs        # Admin RPC implementations
```

## Plan Breakdown

| Plan | Focus | Files |
|------|-------|-------|
| 05-01 | Client library + hook mapping | memory-client crate |
| 05-02 | Query CLI commands | memory-daemon query subcommand |
| 05-03 | Admin CLI commands | memory-daemon admin subcommand |

---
*Research completed: 2026-01-30*
