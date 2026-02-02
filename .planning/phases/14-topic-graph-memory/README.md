# Phase 14: Topic Graph Memory

## Overview

Phase 14 adds **Conceptual Enrichment** to the cognitive architecture through semantic topic extraction, time-decayed importance scoring, and topic relationships. Topics enrich discovery without replacing time-based navigation.

**Philosophy**: "Topics enrich discovery; time remains truth."

## Documentation

| Document | Location | Description |
|----------|----------|-------------|
| PRD | `docs/prds/topic-graph-memory-prd.md` | Product requirements, API surface, data model |
| Technical Plan | `docs/plans/topic-graph-memory.md` | Implementation details, Rust code examples |

## Key Features

1. **Topic Extraction** - Semantic topics from TOC summaries via embedding clustering
2. **LLM Labeling** - Human-readable topic names with keyword fallback
3. **Time-Decayed Importance** - Surfaces recent/frequent topics naturally
4. **Topic Relationships** - Similar topics, parent/child hierarchy
5. **Navigation RPCs** - Explore conceptual connections across time
6. **Lifecycle Management** - Pruning dormant topics, resurrection on reactivation

## Architecture Fit

```
┌─────────────────────────────────────────────────────────────┐
│                     Agent Memory                             │
├─────────────────────────────────────────────────────────────┤
│  EXISTING:                                                   │
│  ├── Events (CF_EVENTS) ........... Raw conversations       │
│  ├── TocNodes (CF_TOC_NODES) ...... Time hierarchy          │
│  ├── Grips (CF_GRIPS) ............. Provenance anchors      │
│  ├── Tantivy (Phase 11) ........... BM25 teleport           │
│  └── HNSW (Phase 12) .............. Vector teleport         │
│                                                              │
│  NEW (Phase 14):                                             │
│  ├── Topics (CF_TOPICS) ........... Semantic concepts       │
│  ├── TopicLinks ................... Topic ↔ Node links      │
│  └── TopicRelationships ........... Similar/parent-child    │
└─────────────────────────────────────────────────────────────┘
```

## Dependencies

- **Phase 12**: Uses embedding infrastructure for topic clustering
- **Phase 13**: Uses outbox pattern for incremental topic updates

## Configuration

Topics are fully optional:

```toml
[topics]
enabled = true                    # Master toggle (default: false)
extraction_threshold = 0.7        # Cluster confidence threshold
importance_half_life_days = 30    # Time decay half-life
min_importance_threshold = 0.1    # Pruning threshold
max_topics = 1000                 # Storage bounds
```

## Plans

| Plan | Wave | Description |
|------|------|-------------|
| 14-01 | 1 | Topic extraction (CF_TOPICS, embedding clustering) |
| 14-02 | 2 | Topic labeling (LLM integration with keyword fallback) |
| 14-03 | 3 | Importance scoring (time decay with configurable half-life) |
| 14-04 | 4 | Topic relationships (similarity, hierarchy discovery) |
| 14-05 | 5 | Navigation RPCs (GetTopicsByQuery, GetTocNodesForTopic) |
| 14-06 | 6 | Lifecycle management (pruning, resurrection, CLI) |

## Requirements Traceability

| Requirement | Description |
|-------------|-------------|
| TOPIC-01 | Topic extraction from TOC summaries via embedding clustering |
| TOPIC-02 | Topic labeling via LLM with fallback to keywords |
| TOPIC-03 | Topic storage in CF_TOPICS column family |
| TOPIC-04 | Time-decayed importance scoring with configurable half-life |
| TOPIC-05 | Topic relationship discovery (similarity, hierarchy) |
| TOPIC-06 | Topic navigation RPCs (GetTopicsByQuery, GetTocNodesForTopic) |
| TOPIC-07 | Topic pruning and resurrection lifecycle |
| TOPIC-08 | GetTopicGraphStatus RPC for feature discovery |
