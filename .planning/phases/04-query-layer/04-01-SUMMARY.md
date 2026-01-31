# Phase 04-01 Summary: TOC Navigation RPCs

## Completed Tasks

### Task 1: Update Proto with TOC Navigation Messages
- Added `TocLevel` enum (Year, Month, Week, Day, Segment)
- Added `TocBullet` message (text, grip_ids)
- Added `TocNode` message (full node with summary)
- Added `Grip` message for provenance
- Added `GetTocRoot`, `GetNode`, `BrowseToc` RPCs
- Added `GetEvents`, `ExpandGrip` RPCs (Plan 04-02)

### Task 2: Implement Query Module
- Created `crates/memory-service/src/query.rs` with:
  - `get_toc_root()` - Returns year-level nodes
  - `get_node()` - Fetches node by ID
  - `browse_toc()` - Paginated child navigation
  - `get_events()` - Time range event retrieval
  - `expand_grip()` - Context around grip excerpt
- Added 7 tests for query functions

### Task 3: Add get_all_year_nodes to Storage
- Added `get_all_year_nodes()` method to Storage
- Scans toc_latest for "latest:toc:year:" prefix
- Fixed versioned key format consistency issue

### Task 4: Wire RPCs into Service
- Updated `MemoryServiceImpl` to implement all query RPCs
- Added imports for query module and new proto types

## Key Artifacts

| File | Purpose | Exports |
|------|---------|---------|
| `proto/memory.proto` | Navigation proto messages | `TocNode`, `TocBullet`, `TocLevel`, `Grip`, RPCs |
| `memory-service/src/query.rs` | Query RPC implementations | `get_toc_root`, `get_node`, `browse_toc`, `get_events`, `expand_grip` |
| `memory-storage/src/db.rs` | Added `get_all_year_nodes` | Storage method for year nodes |

## Proto Messages Added

- `TocLevel` - Enum for hierarchy levels
- `TocBullet` - Bullet with grip IDs
- `TocNode` - Full node with summary
- `Grip` - Excerpt with event pointers
- `GetTocRootRequest/Response` - Year node query
- `GetNodeRequest/Response` - Single node query
- `BrowseTocRequest/Response` - Paginated children
- `GetEventsRequest/Response` - Time range events
- `ExpandGripRequest/Response` - Grip context

## Bug Fixed

Fixed inconsistent versioned key format in `get_toc_nodes_by_level` and `get_all_year_nodes` - was missing "toc:" prefix that `put_toc_node` and `get_toc_node` use.

## Verification

- `cargo build --workspace` compiles
- `cargo test --workspace` passes (103 tests)

## Requirements Coverage

- QRY-01: GetTocRoot returns year-level nodes
- QRY-02: GetNode returns node with children and summary
- QRY-03: BrowseToc supports paginated child navigation
- QRY-04: GetEvents retrieves raw events by time range
- QRY-05: ExpandGrip retrieves context around grip excerpt
