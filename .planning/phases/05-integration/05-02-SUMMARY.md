# Phase 05-02 Summary: Query CLI

## Completed Tasks

### Task 1: Add Query RPCs to Proto

Updated `proto/memory.proto` with:
- `TocLevel` enum (Year, Month, Week, Day, Segment)
- `TocBullet` message (text, grip_ids)
- `TocNode` message (full node structure)
- `Grip` message (excerpt with event pointers)
- `GetTocRoot`, `GetNode`, `BrowseToc` RPCs
- `GetEvents`, `ExpandGrip` RPCs

### Task 2: Implement Query Module in Service

Created `crates/memory-service/src/query.rs`:
- `get_toc_root()` - Returns year-level nodes
- `get_node()` - Fetches node by ID
- `browse_toc()` - Paginated child navigation
- `get_events()` - Time range event retrieval
- `expand_grip()` - Context around grip excerpt
- Type conversion functions (domain ↔ proto)
- 8 unit tests

### Task 3: Wire Query RPCs to Service

Updated `MemoryServiceImpl` in `ingest.rs` to implement all query RPCs.

### Task 4: Add Query Methods to MemoryClient

Extended `memory-client/src/client.rs`:
- `get_toc_root()` - Get year nodes
- `get_node()` - Get specific node
- `browse_toc()` - Browse children with pagination
- `get_events()` - Get events in time range
- `expand_grip()` - Expand grip context

### Task 5: Add Query Subcommand to CLI

Updated `memory-daemon/src/cli.rs`:
- `QueryCommands` enum with Root, Node, Browse, Events, Expand subcommands
- Endpoint flag for specifying daemon address

### Task 6: Implement Query Command Handler

Created `handle_query()` in `commands.rs`:
- Formatted output for all query types
- Pagination support with continuation tokens
- Error handling for connection failures

## Key Artifacts

| File | Purpose |
|------|---------|
| `proto/memory.proto` | TOC navigation and query messages |
| `memory-service/src/query.rs` | Query RPC implementations |
| `memory-service/src/ingest.rs` | Service trait implementation |
| `memory-client/src/client.rs` | Client query methods |
| `memory-daemon/src/cli.rs` | Query subcommand definitions |
| `memory-daemon/src/commands.rs` | Query command handler |

## CLI Usage

```bash
# List root nodes
memory-daemon query root

# Get specific node
memory-daemon query node <node_id>

# Browse children with pagination
memory-daemon query browse <parent_id> --limit 20 --token <token>

# Get events in time range
memory-daemon query events --from <ms> --to <ms> --limit 50

# Expand grip context
memory-daemon query expand <grip_id> --before 3 --after 3
```

## Verification

- `cargo build --workspace` compiles
- `cargo test --workspace` passes (117 tests)
- 8 new query tests in memory-service
- 2 new CLI tests in memory-daemon

## Requirements Coverage

- **CLI-02**: Query CLI for manual TOC navigation and testing

---
*Completed: 2026-01-30*
