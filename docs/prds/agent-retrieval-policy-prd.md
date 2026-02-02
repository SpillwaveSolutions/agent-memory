# Agent Retrieval Policy - Product Requirements Document

**Version:** 1.0
**Date:** 2026-02-01
**Status:** Normative Specification

---

## 1. Executive Summary

This PRD defines the **retrieval policy** - the "brainstem" governing how agents select and sequence retrieval layers. It formalizes the decision algorithm, fallback chains, execution modes, and skill contracts that were previously scattered across multiple PRDs.

### Purpose

The Agent Retrieval Policy ensures:

1. **Consistent behavior** across all memory-querying skills
2. **Graceful degradation** when layers are unavailable
3. **Intent-aware routing** for different query types
4. **Bounded execution** to prevent runaway queries
5. **Explainable decisions** for user trust

### Normative Status

This document is **normative** - all retrieval-capable skills MUST implement the patterns defined here.

---

## 2. Core Principle: Check-Then-Search

> **All skills MUST check availability before using any layer.**

Skills cannot assume any index is available. Before invoking a retrieval layer:

1. **Call the status RPC** for that layer
2. **Check both `enabled` and `healthy`** flags
3. **Fall back** to the next layer if unavailable
4. **Inform the user** which method was used

### Why This Matters

- Users may disable indexes (resource conservation, privacy, simplicity)
- Indexes may be rebuilding, corrupted, or not yet initialized
- Hard-coding layer assumptions creates brittle skills

---

## 2.5 Skills as Policy Executors

> **"Tools don't decide. Skills decide."**

**Skills are the control plane.** Retrieval capabilities (TOC, BM25, Vector, Topics) are inert until skills decide what to do. The memory substrate provides the building blocks; skills provide the intelligence.

### Normative Requirements for Skills

Every retrieval-capable skill MUST:

| Requirement | Description |
|-------------|-------------|
| **Capability Detection** | Check status RPCs once per request (or use cached tier) |
| **Budget Enforcement** | Respect `max_rpc_calls`, `token_budget`, `timeout` |
| **Fallback Discipline** | Never hard-fail if agentic TOC search can run |
| **Explainability Payload** | Report: chosen tier/mode, candidates considered, why winner won |
| **Evidence Handling** | Include `grip_ids`/citations when returning facts |

### Anti-Patterns

Skills MUST NOT:

- Assume any index is available without checking
- Fail silently when a layer is unavailable
- Ignore stop conditions (depth, tokens, timeout)
- Return facts without provenance (grip links)

---

## 3. Query Intent Classification

Before selecting a retrieval path, skills SHOULD classify query intent:

| Intent | Description | Example Queries |
|--------|-------------|-----------------|
| **Explore** | Discover patterns, related concepts, themes | "What have I been working on?" "Show me recurring topics" |
| **Answer** | Get evidence-backed result fast | "How did we fix the JWT bug?" "What was decided about X?" |
| **Locate** | Find exact snippet, quote, or definition | "Where did I define that config?" "Find the error message" |
| **Time-boxed** | Return best partial in N ms, then stop | Embedded in agentic skills with latency constraints |

### Intent Detection Heuristics

| Signal | Likely Intent |
|--------|---------------|
| Pattern words: "themes", "topics", "working on" | Explore |
| Question words: "how", "why", "what was" | Answer |
| Location words: "where", "find", "locate" | Locate |
| Deadline constraint from skill | Time-boxed |

---

## 4. The Decision Algorithm

### Step 1: Classify Query Intent

```
Parse query → Extract signals → Map to intent
    │
    └─► Default to ANSWER if unclear
```

### Step 2: Route by Intent

```
Parse query → Classify intent
    │
    ├─► EXPLORE intent:
    │   Topics → Hybrid/Vector/BM25 → Agentic → (Scan if allowed)
    │
    ├─► ANSWER intent (default):
    │   Hybrid → BM25/Vector → Agentic → (Scan if allowed)
    │
    ├─► LOCATE intent:
    │   BM25 → Hybrid/Vector → Agentic → (Scan if allowed)
    │
    └─► TIME-BOXED intent:
        Best available accelerator → Agentic → STOP (no scan)
```

### Step 3: Check Layer Availability

For each layer in the intent's order:

```rust
// Pseudocode
for layer in intent_layer_order {
    let status = check_status(layer).await;
    if status.enabled && status.healthy {
        return use_layer(layer, query).await;
    }
    // Log: "Skipping {layer}: {reason}"
}
// Final fallback
return agentic_toc_search(query).await;
```

### Step 4: Execute with Bounds

Every execution respects stop conditions (see Section 5.3).

---

## 5. Capability Tiers

### 5.1 Tier Definitions

| Tier | Available Layers | Best For |
|------|-----------------|----------|
| **Tier 1** (Full) | Topics + Hybrid + Agentic | Explore + contextual answers |
| **Tier 2** (Hybrid) | Hybrid (BM25 + Vector) + Agentic | Default for most Answer queries |
| **Tier 3** (Semantic) | Vector + Agentic | Semantic-heavy, concept queries |
| **Tier 4** (Keyword) | BM25 + Agentic | Exact term matching, technical queries |
| **Tier 5** (Agentic) | Agentic TOC Search only | Always works (guaranteed fallback) |

### 5.2 Status RPCs

Skills detect the current tier by checking these RPCs:

| RPC | Layer | Returns |
|-----|-------|---------|
| `GetTeleportStatus` | BM25 | `bm25_enabled`, `bm25_healthy`, `bm25_doc_count` |
| `GetVectorIndexStatus` | Vector | `enabled`, `ready`, `vector_count` |
| `GetTopicGraphStatus` | Topics | `enabled`, `healthy`, `topic_count` |

### Combined Status Check Pattern

```rust
async fn detect_tier() -> Tier {
    let bm25 = client.get_teleport_status().await.ok();
    let vector = client.get_vector_index_status().await.ok();
    let topics = client.get_topic_graph_status().await.ok();

    let bm25_ready = bm25.map(|s| s.bm25_enabled && s.bm25_healthy).unwrap_or(false);
    let vector_ready = vector.map(|s| s.enabled && s.ready).unwrap_or(false);
    let topics_ready = topics.map(|s| s.enabled && s.healthy).unwrap_or(false);

    match (topics_ready, vector_ready, bm25_ready) {
        (true, true, true) => Tier::Full,
        (_, true, true) => Tier::Hybrid,
        (_, true, false) => Tier::Semantic,
        (_, false, true) => Tier::Keyword,
        _ => Tier::Agentic,
    }
}
```

### 5.3 Escalation Procedure: Agent-Based Scanning

**Scanning is NOT a tier** - it's an emergency procedure when recall > efficiency:

| Aspect | Description |
|--------|-------------|
| **When triggered** | Results insufficient AND intent allows AND within bounds |
| **Method** | Spawn scanning agents for specific time ranges |
| **Scope** | Time constraints from query bound the scan scope |
| **Resources** | Uses TOC + summaries + grips (no new indexes) |
| **Cost** | Token-intensive but guaranteed if content exists |
| **Limit** | Only for EXPLORE, ANSWER, LOCATE; never for TIME-BOXED |

---

## 5.4 Retrieval Execution Modes

| Mode | Description | Cost | Use When |
|------|-------------|------|----------|
| **Sequential** (default) | One layer at a time, beam width 1 | Lowest | Most queries, best explainability |
| **Parallel** | Multiple accelerators or siblings at once | Higher | Low latency tolerance, recall critical |
| **Hybrid** | Start parallel, cancel losers when one dominates | Medium | Ambiguous queries, weak top-level results |

### Parallel-Safe Operations

| Operation | Parallel? | Notes |
|-----------|-----------|-------|
| BM25 + Vector + Topics simultaneously | Yes | Read-only, independent indexes |
| Time-slice scanning (week1, week2, week3) | Yes | TOC naturally partitionable |
| Beam drill-down (top-K siblings) | Yes | SearchChildren on siblings |
| Unbounded full fan-out | **No** | Causes token blow-up |
| Without capability check | **No** | Wastes calls on disabled layers |

### Parallel Safety Requirements

- **Bounded fan-out**: Only top-K nodes, not whole tree (beam width 2-5)
- **Early stopping**: Cancel other paths when one produces strong evidence
- **Rank merge**: Normalize scores, dedupe by `node_id`/`grip_id`
- **Explainable arbitration**: "BM25 found exact matches, vector found similar, chose intersection"

### Default Rule

```
Default = Sequential + beam_width=1
Parallel = beam_width=K (2-5) + timeouts + early_stop
```

---

## 5.5 Stop Conditions (Safety Bounds)

Every retrieval operation MUST respect these bounds:

| Condition | Default | Configurable | Purpose |
|-----------|---------|--------------|---------|
| `max_depth` | 5 levels | Yes | Prevent infinite drill-down |
| `max_nodes_visited` | 100 | Yes | Bound exploration breadth |
| `max_rpc_calls` | 20 | Yes | Prevent API explosion |
| `max_token_budget` | 4000 | Yes | Context window protection |
| `timeout_ms` | 5000 | Yes | Latency ceiling |
| `beam_width` | 1 | Yes (2-5) | Control parallelism |

### Enforcement by Intent

| Intent | Enforcement | Escalation Allowed |
|--------|-------------|-------------------|
| **Time-boxed** | Strict (hard stop) | No |
| **Locate** | Soft (can exceed slightly) | Yes, if exact match not found |
| **Answer** | Soft | Yes, if insufficient evidence |
| **Explore** | Soft | Yes, if pattern incomplete |

---

## 6. Functional Requirements

### Status & Detection

| ID | Requirement | Description |
|----|-------------|-------------|
| FR-01 | Combined status check | Single call pattern to detect all layer availability |
| FR-02 | Tier detection algorithm | Map layer availability to capability tier |
| FR-03 | Capability advertisement | Skills can query current tier before user interaction |

### Intent Classification

| ID | Requirement | Description |
|----|-------------|-------------|
| FR-04 | Query intent classification | Classify as Explore/Answer/Locate/Time-boxed |
| FR-05 | Intent-aware routing | Different intents use different layer priorities |
| FR-06 | Time constraint extraction | Extract deadline hints from queries |

### Fallback & Degradation

| ID | Requirement | Description |
|----|-------------|-------------|
| FR-07 | Configuration-aware search | Skip disabled layers in fallback chain |
| FR-08 | Graceful degradation | Automatic fallback through chain |
| FR-09 | Partial result return | Return best results on timeout, don't fail |

### Safety & Bounds

| ID | Requirement | Description |
|----|-------------|-------------|
| FR-10 | Stop condition enforcement | Respect depth, nodes, calls, tokens limits |
| FR-11 | Timeout handling per intent | Time-boxed = hard stop; others = soft |
| FR-12 | Scanning trigger conditions | Only when intent allows and results insufficient |

### Execution Modes

| ID | Requirement | Description |
|----|-------------|-------------|
| FR-15 | Mode selection | Choose Sequential/Parallel/Hybrid based on query |
| FR-16 | Bounded fan-out | Configurable beam width (2-5 for parallel) |
| FR-17 | Early stopping | Cancel other paths when strong evidence found |
| FR-18 | Rank merge | Normalize and dedupe parallel results |
| FR-19 | Explainable arbitration | Log why each path was chosen/rejected |

### User Communication

| ID | Requirement | Description |
|----|-------------|-------------|
| FR-13 | Tier/method reporting | Inform user which tier/method was used |
| FR-14 | Fallback explanation | Explain when fallback occurred and why |

---

## 7. Skill Integration Patterns

### Pattern 1: Check-Then-Search

```rust
async fn search_with_fallback(query: &str) -> Result<SearchResults> {
    // Check availability
    let status = client.get_teleport_status().await;

    match status {
        Ok(s) if s.bm25_enabled && s.bm25_healthy => {
            // BM25 available - use teleport
            client.teleport_search(query).await
        }
        _ => {
            // BM25 unavailable - fall back to agentic
            client.search_children(query).await
        }
    }
}
```

### Pattern 2: Progressive Enhancement

```markdown
## Search Capability Tiers (for SKILL.md)

This skill supports multiple search tiers with automatic fallback:

### Tier 1: Topic-Guided (Best)
- **When:** Topics enabled and healthy
- **Method:** GetTopicsByQuery → GetTocNodesForTopic
- **Best for:** Exploring themes and patterns

### Tier 2: Hybrid Search (Great)
- **When:** BM25 + Vector both available
- **Method:** HybridSearch RPC
- **Best for:** Most Answer queries

### Tier 3: Keyword Search (Good)
- **When:** BM25 available, Vector unavailable
- **Method:** TeleportSearch RPC
- **Best for:** Exact term matching

### Tier 4: Agentic Navigation (Always Works)
- **When:** No indexes available
- **Method:** SearchChildren RPC
- **Best for:** Guaranteed fallback

The skill automatically selects the best available tier.
```

### Pattern 3: User Communication

```markdown
## When Indexes are Disabled

If a user requests search but indexes are disabled:

1. **Inform clearly:** "BM25 search is not enabled. Using TOC navigation."
2. **Suggest enabling:** "Enable with: `teleport.bm25.enabled: true`"
3. **Show results:** Provide agentic search results
4. **Don't fail silently:** Always tell user which method was used
```

---

## 8. Skill Contract (Normative)

**What every retrieval-capable skill MUST provide:**

| Requirement | Description | Validation |
|-------------|-------------|------------|
| **Capability Detection** | Check status RPCs once per request (or use cached tier) | Logs show status check before search |
| **Budget Enforcement** | Respect `max_rpc_calls`, `token_budget`, `timeout` | Operations abort within limits |
| **Fallback Discipline** | Never hard-fail if agentic TOC search can run | No errors when all indexes disabled |
| **Explainability Payload** | Report: chosen tier/mode, candidates considered, why winner won | Response includes method metadata |
| **Evidence Handling** | Include `grip_ids`/citations when returning facts | Claims link to grips |

### SKILL.md Requirements

Every skill that queries memory MUST document:

```markdown
## Memory Integration

### Retrieval Layers Used
- [ ] Topics (optional)
- [ ] Vector (optional)
- [ ] BM25 (optional)
- [ ] Agentic TOC Search (always available)

### Fallback Behavior
[Describe what happens when each layer is unavailable]

### Stop Conditions
[List any custom bounds beyond defaults]

### Configuration
[How users can enable/disable features this skill depends on]
```

---

## 9. Error Handling

### Error Codes and Actions

| RPC | Status Code | Message | Skill Action |
|-----|-------------|---------|--------------|
| TeleportSearch | UNAVAILABLE | "BM25 index not enabled" | Use SearchChildren (agentic) |
| VectorTeleport | UNAVAILABLE | "Vector index not enabled" | Use TeleportSearch (BM25) |
| GetTopicsByQuery | UNAVAILABLE | "Topic graph not enabled" | Use VectorTeleport or BM25 |
| HybridSearch | OK | (works, but one score=0) | Results are partial (one method) |
| Any search | DEADLINE_EXCEEDED | "Timeout exceeded" | Return partial results |

### Response Handling Example

```rust
match client.teleport_search(request).await {
    Ok(response) => {
        display_results(response.results);
    }
    Err(status) if status.code() == Code::Unavailable => {
        // BM25 disabled - inform user and fall back
        log::info!("BM25 search disabled. Using TOC navigation.");
        let fallback = client.search_children(fallback_request).await?;
        display_results(fallback.results);
    }
    Err(status) if status.code() == Code::DeadlineExceeded => {
        // Timeout - return partial
        log::info!("Search timed out. Returning partial results.");
        display_partial_results();
    }
    Err(status) => {
        // Other error - propagate
        return Err(status.into());
    }
}
```

---

## 10. Appendix: Reference Implementation

### Full Search Algorithm

```rust
pub async fn intelligent_search(
    query: &str,
    intent: QueryIntent,
    bounds: StopConditions,
) -> Result<SearchResults> {
    // Step 1: Detect available tier
    let tier = detect_tier().await;

    // Step 2: Get layer order for intent
    let layer_order = match intent {
        QueryIntent::Explore => vec![Layer::Topics, Layer::Hybrid, Layer::BM25, Layer::Agentic],
        QueryIntent::Answer => vec![Layer::Hybrid, Layer::BM25, Layer::Vector, Layer::Agentic],
        QueryIntent::Locate => vec![Layer::BM25, Layer::Hybrid, Layer::Vector, Layer::Agentic],
        QueryIntent::TimeBoxed => vec![Layer::best_available(tier), Layer::Agentic],
    };

    // Step 3: Try each layer in order
    for layer in layer_order {
        if !tier.supports(layer) {
            log::debug!("Skipping {layer}: not available in tier {tier}");
            continue;
        }

        let result = execute_layer(layer, query, &bounds).await;

        match result {
            Ok(r) if r.is_sufficient() => {
                return Ok(SearchResults {
                    results: r.results,
                    method_used: layer,
                    tier_used: tier,
                    fallback_occurred: layer != layer_order[0],
                });
            }
            Ok(r) => {
                log::debug!("{layer} returned insufficient results, trying next");
            }
            Err(e) => {
                log::warn!("{layer} failed: {e}, trying next");
            }
        }
    }

    // Step 4: Final fallback - agentic always works
    let agentic_result = agentic_toc_search(query, &bounds).await?;

    Ok(SearchResults {
        results: agentic_result,
        method_used: Layer::Agentic,
        tier_used: Tier::Agentic,
        fallback_occurred: true,
    })
}
```

---

## See Also

- [Cognitive Architecture Manifesto](../COGNITIVE_ARCHITECTURE.md) - Philosophy and layer stack
- [BM25 Teleport PRD](bm25-teleport-prd.md) - Layer 3 specification
- [Hierarchical Vector Indexing PRD](hierarchical-vector-indexing-prd.md) - Layer 4 specification
- [Topic Graph Memory PRD](topic-graph-memory-prd.md) - Layer 5 specification
- [Agentic TOC Search PRD](agentic-toc-search-prd.md) - Layer 2 specification

---

*PRD Created: 2026-02-01*
*Last Updated: 2026-02-01*
*Author: Agent Memory Team*
*Status: Normative Specification*
