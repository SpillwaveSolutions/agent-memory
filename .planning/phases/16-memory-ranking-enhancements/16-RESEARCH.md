# Phase 16 Research: Memory Ranking Enhancements

**Phase**: 16 - Memory Ranking Enhancements
**Status**: Research
**Created**: 2026-02-05

## Overview

This document captures research needed before planning Phase 16 implementation. The goal is to add retrieval policy improvements with salience scoring, usage tracking, novelty filtering, and index lifecycle automation.

## Related Documentation

- RFC: [docs/plans/memory-ranking-enhancements-rfc.md](../../../docs/plans/memory-ranking-enhancements-rfc.md)
- Technical Plan: [docs/plans/phase-16-memory-ranking-plan.md](../../../docs/plans/phase-16-memory-ranking-plan.md)

## Research Areas

### 1. Salience Scoring Algorithms

**Question**: How to compute salience scores efficiently at write time?

**Areas to research**:
- Text density metrics (information per token)
- Entity detection (NER for salient entities)
- Keyword classification (TF-IDF based importance)
- Position weighting (topic sentences, conclusions)
- Reference patterns (links, citations indicate importance)

**Constraints**:
- Must be computable at write time (no deferred processing)
- Cannot require external API calls (local only)
- Should be deterministic for consistent scoring

### 2. Usage Tracking Infrastructure

**Question**: How to efficiently store and retrieve usage counters?

**Areas to research**:
- RocksDB column family design for CF_USAGE_COUNTERS
- LRU cache implementation options (lru crate vs custom)
- Cache invalidation strategies
- Atomic counter updates
- Read-through caching patterns

**Constraints**:
- Cache-first reads for performance
- Must not block on cache misses
- Usage data is advisory (can be lost on crash)

### 3. Novelty Detection

**Question**: How to detect near-duplicate content efficiently?

**Areas to research**:
- Vector similarity threshold tuning
- MinHash/SimHash for fast filtering
- Bloom filter for candidate selection
- False positive handling
- Opt-in configuration patterns

**Constraints**:
- Must be opt-in (disabled by default)
- Fallback on any failure (never block ingestion)
- Configurable similarity threshold

### 4. Vector Index Lifecycle (FR-08)

**Question**: How to implement vector pruning per retention rules?

**Areas to research**:
- usearch vector deletion APIs
- Batch deletion vs individual
- Index compaction after deletions
- Retention policy enforcement
- Scheduler job design

**Constraints**:
- Daily job frequency
- Must respect retention policies
- Should not impact query latency

### 5. BM25 Index Lifecycle (FR-09)

**Question**: How to prune Tantivy index documents?

**Areas to research**:
- Tantivy document deletion
- Index segment merging after deletions
- Garbage collection strategies
- Disabled-by-default configuration
- Recovery from partial failures

**Constraints**:
- Optional (disabled by default)
- Must be idempotent
- Should support dry-run mode

### 6. Feature Flags Design

**Question**: How to implement feature flags with master switch?

**Areas to research**:
- Configuration hierarchy (global disable overrides)
- Runtime vs startup configuration
- Flag validation on startup
- Metrics per feature flag
- Gradual rollout support

**Constraints**:
- Master switch must disable all ranking features
- Individual flags for each feature
- Backward compatible with v2.0.0

## Existing Patterns to Reuse

From Phase 14 (Topic Graph):
- Time-decay scoring implementation
- Configuration flag patterns
- Optional feature with graceful disable

From Phase 13 (Outbox Indexing):
- Checkpoint-based processing
- Scheduler job patterns
- Admin command structure

## Open Questions

1. Should salience scores be stored with the node or in a separate CF?
2. What's the cache hit rate target for usage counters?
3. How to handle novelty detection across agent boundaries?
4. Should index lifecycle jobs be synchronous or async?
5. What metrics should be exposed for monitoring?

## Next Steps

1. Review RFC and technical plan for additional research needs
2. Run /gsd:plan-phase 16 to create executable plans
3. Update REQUIREMENTS.md with RANK-* requirements

---
*Research document created: 2026-02-05*
