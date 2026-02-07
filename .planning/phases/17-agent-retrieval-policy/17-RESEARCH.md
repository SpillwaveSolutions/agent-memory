# Phase 17 Research: Agent Retrieval Policy

**Phase**: 17 - Agent Retrieval Policy
**Status**: Research
**Created**: 2026-02-05

## Overview

This document captures research needed before planning Phase 17 implementation. The goal is to implement the retrieval "brainstem" - the decision algorithm for layer selection, intent classification, fallback chains, and skill contracts.

## Related Documentation

- PRD: [docs/prds/agent-retrieval-policy-prd.md](../../../docs/prds/agent-retrieval-policy-prd.md)

## Research Areas

### 1. Query Intent Classification

**Question**: How to classify query intent without external LLM calls?

**Areas to research**:
- Keyword-based heuristics (time words, question patterns)
- Query structure analysis
- Entity type detection
- Historical pattern matching
- Confidence scoring for classification

**Intent types from PRD**:
- **Explore**: Open-ended browsing ("what have we discussed?")
- **Answer**: Specific fact retrieval ("what was the decision on X?")
- **Locate**: Find exact content ("where did we talk about Y?")
- **Time-boxed**: Temporal constraint ("what happened yesterday?")

**Constraints**:
- No external API calls
- Deterministic classification
- Fast (<10ms latency)

### 2. Capability Tier Detection

**Question**: How to detect available capabilities and map to tiers?

**Tiers from PRD**:
- Tier 1: TOC only (minimum viable)
- Tier 2: TOC + BM25
- Tier 3: TOC + BM25 + Vector
- Tier 4: TOC + BM25 + Vector + Topics
- Tier 5: Full stack + Ranking

**Areas to research**:
- Health check patterns for each layer
- Combined status check (single call)
- Graceful degradation logic
- Tier advertisement to skills

### 3. Fallback Chain Design

**Question**: How to implement automatic fallback on layer failure?

**Areas to research**:
- Chain-of-responsibility pattern
- Circuit breaker patterns
- Timeout handling per layer
- Partial result aggregation
- Error classification (transient vs permanent)

**Constraints**:
- Must skip disabled layers
- Should not cascade failures
- Must provide explanation of path taken

### 4. Execution Modes

**Question**: How to implement Sequential/Parallel/Hybrid execution?

**Modes from PRD**:
- **Sequential**: One layer at a time, stop on success
- **Parallel**: All layers simultaneously, merge results
- **Hybrid**: Priority layers first, expand if needed

**Areas to research**:
- Tokio task spawning patterns
- Bounded fan-out (max concurrent)
- Early stopping conditions
- Result merging strategies
- Resource limits

### 5. Rank Fusion

**Question**: How to merge results from multiple layers?

**Areas to research**:
- Reciprocal Rank Fusion (RRF)
- Weighted combination
- Score normalization
- Deduplication
- Configurable weights

### 6. Stop Conditions

**Question**: How to enforce retrieval limits per intent?

**Constraints from PRD**:
- max_depth: How deep to traverse
- max_nodes: Maximum results
- timeout: Per-intent time limit

**Areas to research**:
- Timeout propagation with tokio
- Node counting across layers
- Depth tracking in navigation
- Early termination signals

### 7. Skill Contracts

**Question**: What information should skills receive?

**Explainability payload**:
- Tier used
- Method(s) employed
- Why this path was chosen
- Fallback history
- Confidence scores

**Areas to research**:
- Contract versioning
- Optional vs required fields
- Backward compatibility
- Validation patterns

## Existing Patterns to Reuse

From Phase 10.5 (Agentic TOC Search):
- Navigation path tracking
- Explainability reporting
- Search result formatting

From Phase 14 (Topics):
- Multi-layer coordination
- Optional feature patterns
- Status check design

From Phase 16 (Ranking):
- Score combination
- Feature flags

## Open Questions

1. Should intent classification be pluggable?
2. How to handle conflicting signals from multiple layers?
3. What's the default timeout per intent type?
4. Should tier detection be cached or computed per request?
5. How to expose retrieval policy to CLI for debugging?

## Next Steps

1. Review PRD FR-01 through FR-19 for detailed requirements
2. Run /gsd:plan-phase 17 to create executable plans
3. Update REQUIREMENTS.md with RETR-* requirements

---
*Research document created: 2026-02-05*
