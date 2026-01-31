# Phase 6 Research: End-to-End Demo

## Overview

Phase 6 validates the complete agent-memory system through integration testing and comprehensive documentation. This is a validation phase with no new requirements - it demonstrates all prior phases working together.

## Success Criteria to Validate

From ROADMAP.md:
1. Hook captures a conversation, events flow to daemon, TOC builds automatically
2. Agent can navigate TOC via gRPC to find relevant time periods
3. Query "what did we discuss yesterday?" returns summary-based answer
4. Agent can drill down from summary to grips to raw events for verification
5. System recovers gracefully from daemon restart (crash recovery)

## Components to Test

### 1. Event Ingestion Pipeline
- Client library (memory-client) connects to daemon
- HookEvent maps to domain Event
- IngestEvent RPC persists to RocksDB
- Outbox entry created atomically

### 2. TOC Construction
- Segmenter creates segments from events
- Summarizer generates summaries with grips
- TOC builder creates time hierarchy
- Rollup jobs aggregate children to parents

### 3. Query Navigation
- GetTocRoot returns year-level nodes
- GetNode retrieves specific node details
- BrowseToc paginates child nodes
- GetEvents retrieves events in time range
- ExpandGrip retrieves context around grip

### 4. Crash Recovery
- Checkpoints enable job resumption
- Outbox entries survive restart
- RocksDB durability guarantees

## Testing Approach

### Integration Test Harness

Use Rust's integration test framework with:
- TempDir for isolated storage
- Spawned daemon process
- Client library for gRPC communication
- MockSummarizer for deterministic summaries

### Demo Script

Create a shell script that:
1. Starts the daemon
2. Ingests sample conversation events
3. Triggers TOC building
4. Queries and displays results
5. Demonstrates grip expansion

## Documentation Structure

### README.md Updates
- Quick start guide
- Architecture overview
- Configuration reference

### docs/ Structure
- ARCHITECTURE.md - Component diagrams
- USAGE.md - CLI usage examples
- INTEGRATION.md - Hook integration guide
- API.md - gRPC API reference

## Dependencies

All previous phases must be complete:
- Phase 1: Foundation (storage, types, gRPC, daemon)
- Phase 2: TOC Building (segmentation, summarization)
- Phase 3: Grips & Provenance (excerpt storage, expansion)
- Phase 4: Query Layer (navigation RPCs)
- Phase 5: Integration (client library, CLI tools)
