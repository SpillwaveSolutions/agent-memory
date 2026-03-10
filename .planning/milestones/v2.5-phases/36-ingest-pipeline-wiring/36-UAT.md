---
status: complete
phase: 36-ingest-pipeline-wiring
source: [36-01-SUMMARY.md, 36-02-SUMMARY.md, 36-03-SUMMARY.md]
started: 2026-03-06T08:10:00Z
updated: 2026-03-06T08:15:00Z
---

## Current Test

[testing complete]

## Tests

### 1. put_event_only stores without outbox
expected: `Storage::put_event_only` writes event bytes to CF_EVENTS without outbox entry. Idempotent (second call returns created=false).
result: pass

### 2. EventType::is_structural identifies lifecycle events
expected: `EventType::is_structural()` returns true for SessionStart, SessionEnd, SubagentStart, SubagentStop and false for all other event types (Message, ToolResult, etc.).
result: pass

### 3. CandleEmbedderAdapter bridges to EmbedderTrait
expected: `CandleEmbedderAdapter` wraps `CandleEmbedder` and implements `EmbedderTrait`. Uses `tokio::task::spawn_blocking` to avoid blocking the tokio runtime during CPU-bound embedding inference.
result: pass

### 4. Ingest dedup branching — duplicate events skip outbox
expected: In `ingest_event`, when NoveltyChecker marks an event as duplicate, the event is stored via `put_event_only` (no outbox entry), so it never reaches HNSW/BM25 indexing. Novel events go through normal `put_event` with outbox.
result: pass

### 5. Structural events bypass dedup gate
expected: In `ingest_event`, structural events (session_start, session_end, subagent_start, subagent_stop) skip the NoveltyChecker entirely and always get stored with an outbox entry (always indexed).
result: pass

### 6. Novel event embeddings pushed to InFlightBuffer
expected: After a novel event is stored successfully (`created=true && !deduplicated`), its embedding vector is pushed to the InFlightBuffer via `push_to_buffer` for future within-session dedup checks.
result: pass

### 7. Proto: deduplicated field on IngestEventResponse
expected: `proto/memory.proto` has `bool deduplicated = 3` on `IngestEventResponse`. The field is set in `ingest_event` response: true when event was marked duplicate, false otherwise.
result: pass

### 8. Proto: GetDedupStatus RPC
expected: `GetDedupStatus` RPC exists in proto with request/response messages. Handler returns: enabled, threshold, events_checked, events_deduplicated, events_skipped, buffer_size, buffer_capacity.
result: pass

### 9. Daemon startup wires NoveltyChecker with fail-open
expected: `start_daemon` creates NoveltyChecker from `Settings.dedup` config with CandleEmbedder. If CandleEmbedder fails to load, daemon logs a warning and starts without dedup (fail-open). If dedup disabled in config, NoveltyChecker is None.
result: pass

### 10. Cross-session dedup via HNSW (CompositeVectorIndex)
expected: When HNSW vector directory exists on disk, daemon wires `CompositeVectorIndex` (InFlightBuffer + HNSW) into NoveltyChecker. Both within-session (buffer, 256 entries) and cross-session (HNSW, persistent) duplicates are checked. Highest similarity from either backend is used.
result: pass

### 11. HNSW fallback to buffer-only
expected: When HNSW vector directory does NOT exist or fails to open, daemon falls back to `with_in_flight_buffer` (buffer-only dedup). Logs indicate which mode is active (hnsw: true/false).
result: pass

### 12. CompositeVectorIndex fail-open on backend errors
expected: If one backend (InFlightBuffer or HNSW) fails during search, CompositeVectorIndex logs a warning but returns results from the working backend. Search does not fail entirely.
result: pass

### 13. All existing tests pass
expected: `cargo test --workspace --all-features` passes. All 98+ memory-service tests, plus full workspace. No regressions from Phase 36 changes.
result: pass

### 14. Clippy clean
expected: `cargo clippy --workspace --all-targets --all-features -- -D warnings` produces zero warnings across the workspace.
result: pass

## Summary

total: 14
passed: 14
issues: 0
pending: 0
skipped: 0

## Gaps

[none]
