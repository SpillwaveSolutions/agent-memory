# Phase 3 Research: Grips & Provenance

## Overview

Phase 3 anchors TOC summaries to source evidence through grips. A grip is an excerpt from the original events paired with pointers to the event range that supports it. This enables agents to verify claims in summaries by drilling down to source material.

## Requirements

| Req ID | Description | Status |
|--------|-------------|--------|
| GRIP-01 | Grip struct with excerpt, event_id_start, event_id_end, timestamp, source | Done (Phase 1) |
| GRIP-02 | TOC node bullets link to supporting grips | Partial (TocBullet.grip_ids exists) |
| GRIP-03 | Grips stored in dedicated column family | Partial (CF_GRIPS exists) |
| GRIP-04 | ExpandGrip returns context events around excerpt | Not started |
| SUMM-03 | Summarizer extracts grips from events | Not started |

## Existing Infrastructure

### Already Implemented

1. **Grip Type** (`crates/memory-types/src/grip.rs`):
   - `grip_id`, `excerpt`, `event_id_start`, `event_id_end`
   - `timestamp`, `source`, `toc_node_id`
   - `to_bytes()` / `from_bytes()` serialization

2. **TocBullet with Grip Support** (`crates/memory-types/src/toc.rs`):
   - `grip_ids: Vec<String>` field for linking bullets to grips

3. **CF_GRIPS Column Family** (`crates/memory-storage/src/column_families.rs`):
   - Already defined and created during database initialization

### Needs Implementation

1. **Grip Storage Methods** (in `Storage`):
   - `put_grip()` - Store a grip
   - `get_grip()` - Retrieve a grip by ID
   - `get_grips_for_node()` - Get all grips linked to a TOC node

2. **Summarizer Grip Extraction**:
   - Modify summarizer to extract key excerpts
   - Create grips during summarization
   - Link grips to generated bullets

3. **Grip Expansion**:
   - Retrieve context events around a grip's event range
   - Configurable context window (events before/after)

## Technical Design

### Grip ID Format

```
grip:{timestamp_ms}:{ulid}
```

Example: `grip:1706540400000:01HN4QXKN6ABC123`

### Storage Keys

**Grips CF:**
- Key: `{grip_id}`
- Value: JSON-serialized Grip

**Index for node lookup:**
- Key: `node:{node_id}:{grip_id}`
- Value: (empty, just for existence check)

### Grip Extraction Strategy

During summarization:
1. Identify key phrases/claims in generated bullets
2. Find supporting events that contain similar content
3. Extract relevant excerpt from those events
4. Create grip linking excerpt to event range
5. Store grip and link to bullet

### Context Window for Expansion

When expanding a grip:
1. Get the start and end event IDs
2. Query events in time range [start_time - context, end_time + context]
3. Return events with the excerpt's position highlighted

## Plan Breakdown

### 03-01: Grip Storage and Data Model
- Add grip storage methods to Storage
- Create grip ID generation utilities
- Add tests for grip CRUD operations

### 03-02: Summarizer Grip Extraction Integration
- Extend Summarizer trait with grip extraction
- Implement excerpt identification
- Link bullets to grips during summarization

### 03-03: Grip Expansion (Context Retrieval)
- Implement ExpandGrip functionality
- Configurable context window
- Return structured response with events and excerpt position

## Dependencies

- Phase 2 (TOC Building): Summarizer, TocBuilder, Storage

## Success Criteria

1. Grips are created during summarization with excerpt and event references
2. TOC node bullets link to supporting grip IDs
3. Grips are stored in dedicated column family
4. Given a grip ID, the system returns context events around the excerpt

## Estimated Plans

| Plan | Description | Wave |
|------|-------------|------|
| 03-01 | Grip storage and data model | 1 |
| 03-02 | Summarizer grip extraction integration | 2 |
| 03-03 | Grip expansion (context retrieval) | 2 |
