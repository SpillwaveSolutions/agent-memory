# Phase 18 Plan 03 Summary: TocNode contributing_agents and CLI --agent filter

## Completed: 2026-02-08

### Overview

Added `contributing_agents` field to TocNode for tracking which agents contributed events to each time period, and added `--agent` / `-a` CLI filter to search and retrieval commands.

### Tasks Completed

#### Task 1: Add contributing_agents to TocNode

**File:** `crates/memory-types/src/toc.rs`

**Changes:**
1. Added `contributing_agents: Vec<String>` field with `#[serde(default)]` for backward compatibility
2. Updated `TocNode::new()` to initialize `contributing_agents` to empty Vec
3. Added `with_contributing_agent()` builder method with lowercase normalization and deduplication
4. Added `with_contributing_agents()` bulk builder method with sort/dedup

**Tests Added:**
- `test_toc_node_backward_compat_no_agents` - Verifies pre-phase-18 JSON deserializes with empty contributing_agents
- `test_toc_node_with_contributing_agents` - Tests individual agent addition with deduplication
- `test_toc_node_with_contributing_agents_bulk` - Tests bulk agent setting with normalization

**Verification:**
- `cargo test -p memory-types` - All 61 tests pass
- `cargo clippy -p memory-types` - No warnings

#### Task 2: Add --agent filter to CLI commands

**File:** `crates/memory-daemon/src/cli.rs`

**Commands Updated:**
1. `TeleportCommand::Search` - Added `--agent`/`-a` option
2. `TeleportCommand::VectorSearch` - Added `--agent`/`-a` option
3. `TeleportCommand::HybridSearch` - Added `--agent`/`-a` option
4. `RetrievalCommand::Route` - Added `--agent`/`-a` option

**Tests Added:**
- `test_cli_teleport_search_with_agent` - Tests `--agent` long form
- `test_cli_teleport_search_agent_short` - Tests `-a` short form
- `test_cli_teleport_vector_search_with_agent` - Tests vector search with agent
- `test_cli_teleport_hybrid_search_with_agent` - Tests hybrid search with agent
- `test_cli_retrieval_route_with_agent` - Tests route with `--agent`
- `test_cli_retrieval_route_agent_short` - Tests route with `-a`

**Verification:**
- `rustfmt --check crates/memory-daemon/src/cli.rs` - Passes
- Note: Full memory-daemon tests blocked by C++ environment issue (librocksdb-sys compilation), but CLI syntax verified

### Success Criteria Met

| Criterion | Status |
|-----------|--------|
| TocNode has `contributing_agents: Vec<String>` with serde(default) | DONE |
| Builder methods for adding contributing agents exist | DONE |
| Old TocNodes deserialize with empty contributing_agents | DONE |
| TeleportCommand::Search has --agent/-a filter | DONE |
| TeleportCommand::VectorSearch has --agent/-a filter | DONE |
| TeleportCommand::HybridSearch has --agent/-a filter | DONE |
| RetrievalCommand::Route has --agent/-a filter | DONE |
| memory-types tests pass | DONE |

### Artifacts

| Path | Purpose |
|------|---------|
| `crates/memory-types/src/toc.rs` | TocNode with contributing_agents field |
| `crates/memory-daemon/src/cli.rs` | CLI commands with --agent filter |

### Notes

- Agent IDs are normalized to lowercase to ensure case-insensitive matching
- The `with_contributing_agent()` method prevents duplicate entries
- The `with_contributing_agents()` method sorts and deduplicates the list
- CLI tests use `Cli::parse_from()` for isolated unit testing without the daemon dependencies
- Full integration testing will be verified in CI where the C++ toolchain is properly configured

### Next Steps

This plan enables Phase 18-04 (gRPC proto extensions) and subsequent plans to wire the agent filtering through the retrieval layer.
