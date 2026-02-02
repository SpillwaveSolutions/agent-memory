# Plan: Revise BM25 PRD to Match Agent-Memory Architecture

## Overview

Revise the "Time-Aware BM25 Lexical Memory System PRD" to align with the actual agent-memory Rust project architecture. The PRD has valuable conceptual ideas but uses different terminology and assumptions that need correction.

## Key Findings

### What the PRD Gets Right
- 4-level agentic search model (L1: TOC, L2: Summaries, L3: Search, L4: Raw)
- Time-based hierarchical organization
- BM25 as a grounding layer, not primary navigation
- Progressive disclosure philosophy

### What Needs Correction
| PRD Concept | Actual Implementation |
|-------------|----------------------|
| Hot/Warm/Cold/Archive layers | TOC levels: Segment → Day → Week → Month → Year |
| Raw conversation indexing | TOC nodes + Grips indexed (NOT raw events) |
| Eviction/TTL policies | **None** - Append-only, no deletion |
| Lexical compaction | LLM-based rollup summarization |
| Separate layer indexes | Single Tantivy index with `doc_type` field |

### Architecture Already Exists
- Phase 11 plans (11-01 through 11-04) already define Tantivy integration
- PROJECT.md establishes "indexes are accelerators, not dependencies"
- Rollup jobs already compact summaries via scheduler

## Implementation Plan

### 1. Create Revised PRD File
**Location:** `docs/prds/bm25-teleport-prd.md`

**Sections:**
1. **Executive Summary** - BM25 as teleport accelerator, not primary navigation
2. **Architecture Alignment** - How BM25 fits existing TOC + storage design
3. **Terminology Mapping** - PRD concepts → actual implementation
4. **What Gets Indexed** - TOC nodes (title + bullets + keywords) + Grips (excerpts)
5. **4-Level Agentic Search Model** - Map to actual RPCs (GetTocRoot, GetNode, TeleportSearch, ExpandGrip)
6. **Bounded Growth via Summarization** - Not eviction, but compression
7. **Requirements** - TEL-01 through TEL-08 (aligning with existing format)
8. **Integration with Phase 11** - Cross-reference existing plans

### 2. Key Content Changes

**Replace Hot/Warm/Cold/Archive with TOC Levels:**
```
Original: Hot (raw) → Daily → Weekly → Monthly
Revised:  Segment (30min/4K tokens) → Day → Week → Month → Year
```

**Remove Eviction Concepts:**
- Raw events: Append-only, never deleted
- TOC nodes: Versioned, immutable
- BM25 index: Rebuildable from storage (disposable accelerator)

**Clarify What Gets Indexed:**
- TOC nodes: `title + bullets.text + keywords`
- Grips: `excerpt` (evidence for summary claims)
- NOT raw events (token explosion, already compressed in summaries)

**Map 4-Level Model to Actual RPCs:**
| Level | Role | Implementation |
|-------|------|----------------|
| L1: TOC Root | Orientation | `GetTocRoot()` |
| L2: Hierarchies | Abstraction | `GetNode()`, `BrowseToc()` |
| L3: BM25 Teleport | Grounding | `TeleportSearch()` (Phase 11) |
| L4: Raw Events | Evidence | `ExpandGrip()`, `GetEvents()` |

### 3. Preserve Valuable PRD Insights

**Keep the "Cognitive Navigation" framing:**
> "Search is performed as a four-stage agentic process using Progressive Disclosure"

**Keep the growth analysis (adjusted numbers):**
| Time | Events | TOC Nodes | Index Size |
|------|--------|-----------|------------|
| 1 month | ~10K | ~500 | ~5 MB |
| 1 year | ~120K | ~6K | ~60 MB |
| 5 years | ~600K | ~30K | ~300 MB |

**Keep the success metrics (adapted):**
| Metric | Target |
|--------|--------|
| Query latency (p99) | < 100ms |
| Rare entity recall | > 95% |
| Index growth | Sub-linear (via summarization) |

### 4. Files to Create/Modify

| File | Action |
|------|--------|
| `docs/prds/bm25-teleport-prd.md` | **Create** - Main revised PRD |
| `.planning/REQUIREMENTS.md` | **Update** - Add TEL-01 through TEL-08 |
| `.planning/ROADMAP.md` | **Update** - Link PRD under Phase 11 |

## Verification

After creating the revised PRD:
1. Cross-reference with `.planning/phases/11-bm25-teleport-tantivy/11-RESEARCH.md` for consistency
2. Ensure terminology matches PROJECT.md
3. Validate requirements format matches existing TEL-* entries

## Summary

The original PRD's core insight - hierarchical, time-aware lexical search with progressive disclosure - is sound and aligns with the project's architecture. The revision mainly involves:
1. Replacing eviction-based bounded growth with summarization-based compression
2. Mapping Hot/Warm/Cold to actual TOC levels
3. Clarifying that TOC nodes + grips are indexed, not raw events
4. Aligning terminology with PROJECT.md ("teleport", "accelerator", "PDA")
