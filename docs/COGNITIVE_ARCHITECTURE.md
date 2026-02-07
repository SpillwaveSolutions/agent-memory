# Agent Memory Cognitive Architecture

**Version:** 2.1
**Date:** 2026-02-05
**Status:** All cognitive layers (0-5) implemented with ranking policy and retrieval brainstem

---

## The Core Insight

> **"Agentic search beats brute-force scanning."**

Agent Memory is a **cognitive architecture for agents**, not just a memory system. Instead of loading thousands of events into context, an agent navigates a structured hierarchy, reading summaries at each level until it finds the area of interest, then drilling down for details. This mirrors how humans naturally search through information.

---

## The Cognitive Layer Stack

Agent Memory implements a 6-layer cognitive hierarchy, where each layer provides distinct capabilities:

| Layer | Capability | Implemented By | Mode | Purpose |
|-------|------------|----------------|------|---------|
| **0** | Raw Events | RocksDB CF_EVENTS | Always present | Immutable truth |
| **1** | TOC Hierarchy | RocksDB CF_TOC_NODES | Always present | Time-based navigation |
| **2** | Agentic TOC Search | SearchNode/SearchChildren | Always works | Index-free term matching |
| **3** | Lexical Teleport | BM25/Tantivy | Configurable | Keyword grounding |
| **4** | Semantic Teleport | Vector/HNSW | Configurable | Embedding similarity |
| **5** | Conceptual Discovery | Topic Graph | Optional | Pattern and concept enrichment |
| **6** | Ranking Policy | Salience/Usage/Novelty | Optional | Memory importance scoring |
| **Brainstem** | Retrieval Policy | TierDetector/IntentClassifier | Always present | Decision routing |

**Hybrid Mode** (not a layer): Score fusion of layers 3+4 when both are enabled.

**Escalation Procedure** (not a layer): Agent-based Scanning - token-intensive last resort when recall > efficiency.

### Ranking Policy (Layer 6)

The ranking policy layer enhances retrieval quality through:

| Signal | Weight | Description |
|--------|--------|-------------|
| **Salience** | 0.3 | Memory importance (Procedure > Constraint > Definition > Preference > Observation) |
| **Recency** | 0.3 | Time-decayed scoring with configurable half-life |
| **Relevance** | 0.3 | BM25/Vector match score |
| **Usage** | 0.1 | Access frequency with decay (opt-in) |

### Retrieval Brainstem

The retrieval policy acts as the "brainstem" - automatic decision-making for:

- **Tier Detection**: Maps available layers to capability tiers (1-5)
- **Intent Classification**: Routes Explore/Answer/Locate/Time-boxed queries
- **Fallback Chains**: Automatic graceful degradation
- **Explainability**: Every result includes tier used and why

---

## The Foundational Principle

> **"Indexes are accelerators, not dependencies."**

This is the load-bearing wall of the architecture:

- The TOC hierarchy is the **source of truth**
- BM25, Vector, and Topic indexes are **disposable accelerators**
- If any index fails, the system **degrades gracefully** to the next available layer
- Agentic TOC Search (Layer 2) **always works** - no index dependency

---

## The Control Plane: Skills as Executive Function

> **"Tools don't decide. Skills decide."**

Skills are the **executive function** of the cognitive architecture. The memory substrate provides capabilities (TOC, BM25, Vector, Topics), but it does not decide how to use them. Agentic skills encode the "how and when":

- They choose which tools to invoke
- They sequence calls for progressive disclosure
- They enforce budgets (tokens, time, depth)
- They apply fallback chains when layers are unavailable
- They produce explainable outputs with citations

This separation keeps the core system **reliable and deterministic** while allowing behavior to **evolve through skills**.

### Separation of Concerns

| Plane | What It Is | Owned By |
|-------|-----------|----------|
| **Data Plane** | Events, TOC nodes, grips | agent-memory core (RocksDB) |
| **Capability Plane** | BM25, Vector, Topics RPCs | memory-service (gRPC) |
| **Ranking Plane** | Salience, usage, novelty | memory-retrieval (Phase 16) |
| **Control Plane** | Tier detection, intent routing | memory-retrieval (Phase 17) |
| **Skill Plane** | Agent skills + fallback chains | skill ecosystem |

---

## Capability Tiers

The system detects available layers and maps to capability tiers:

| Tier | Name | Layers Available | Best For |
|------|------|------------------|----------|
| 1 | **Full** | Topics + Hybrid + Agentic | Semantic exploration, topic discovery |
| 2 | **Hybrid** | BM25 + Vector + Agentic | Balanced keyword + semantic |
| 3 | **Semantic** | Vector + Agentic | Conceptual similarity search |
| 4 | **Keyword** | BM25 + Agentic | Exact term matching |
| 5 | **Agentic** | TOC navigation only | Always works (no indices) |

### Tier Detection

```bash
memory-daemon retrieval status
```

Output:
```
Retrieval Capabilities
----------------------------------------
Current Tier:    2 (Hybrid)
Available Layers:
  - bm25:    healthy (2847 docs)
  - vector:  healthy (2103 vectors)
  - agentic: healthy (TOC available)
Unavailable:
  - topics:  disabled (topics.enabled = false)
```

---

## Query Intent Classification

Queries are classified into four intents for optimal routing:

| Intent | Triggers | Optimal Strategy | Stop Conditions |
|--------|----------|------------------|-----------------|
| **Explore** | "browse", "discover", "what topics" | Topics-first, broad fan-out | max_nodes: 100, beam_width: 5 |
| **Answer** | "what did", "how did", "find" | Hybrid, precision-focused | max_nodes: 50, min_confidence: 0.6 |
| **Locate** | Identifiers, exact phrases | BM25-first, exact match | max_nodes: 20, first_match: true |
| **Time-boxed** | "yesterday", "last week", dates | Time-filtered, sequential | max_depth: 2, time_constraint: set |

---

## Progressive Disclosure Architecture (PDA)

The TOC implements **Progressive Disclosure Architecture** - the same pattern used in well-designed agentic skills. Just as a skill reveals complexity progressively, Agent Memory reveals conversation detail progressively.

### The Navigation Pattern

| Step | Level | What the Agent Sees | Decision |
|------|-------|---------------------|----------|
| 1 | **Year** | "2024: 847 conversations about auth, databases, Rust" | Too broad → drill down |
| 2 | **Month** | "January: 156 conversations, heavy focus on authentication" | Promising → drill down |
| 3 | **Week** | "Week 3: JWT implementation, OAuth2 integration" | This is it → drill down |
| 4 | **Day** | "Thursday: Debugged JWT token expiration issue" | Found it → drill down |
| 5 | **Segment/Grip** | Actual conversation excerpt with event links | Verify → expand if needed |

At each level, the agent reads a **summary** (title, bullets, keywords) and decides whether to:
- **Drill down**: This area looks relevant, explore children
- **Move laterally**: Check sibling nodes for better matches
- **Expand grip**: Found the answer, get the raw events for verification

### Human Analogy: Email Search

Think about how you find an important email from last month:

1. **You don't**: Read every email from the beginning of time
2. **You do**: Filter to "last month" (time-based narrowing)
3. **You do**: Scan subject lines for keywords (summary-based search)
4. **You do**: Open the thread that looks right (drill-down)
5. **You do**: Read the specific message (raw content access)

Agent Memory gives AI agents the same efficient search pattern, but structured for programmatic access via gRPC.

---

## The Fallback Chain (Single Source of Truth)

The fallback chain is **configuration-aware** and **intent-aware**.

### By Query Intent

| Intent | Primary | Secondary | Tertiary | Escalation |
|--------|---------|-----------|----------|------------|
| **Explore** | Topics | Hybrid/Vector/BM25 | Agentic | Scan (if allowed) |
| **Answer** | Hybrid | BM25/Vector | Agentic | Scan (if allowed) |
| **Locate** | BM25 | Hybrid/Vector | Agentic | Scan (if allowed) |
| **Time-boxed** | Best available | Agentic | STOP | Never |

### Fallback Flow

```
Query arrives
    │
    ├─► Topics enabled? ──► Yes ──► GetTopicsByQuery
    │         │
    │         └─► No
    │              │
    ├─► Vector enabled? ──► Yes ──► VectorTeleport / HybridSearch
    │         │
    │         └─► No
    │              │
    ├─► BM25 enabled? ──► Yes ──► TeleportSearch
    │         │
    │         └─► No
    │              │
    └─► SearchChildren (always works, no index needed)
```

### Guarantees

1. **Never fails completely** - Agentic TOC Search always works (no index dependency)
2. **Respects configuration** - Disabled layers are skipped
3. **Respects intent** - Topics first only for Explore; BM25 first for Locate
4. **Respects bounds** - Stop conditions enforced per intent

---

## Cognitive Contracts

Each layer declares its **contract** - what it can and cannot do:

| Layer | Strength | Weakness | Failure Mode | Safe Fallback |
|-------|----------|----------|--------------|---------------|
| **TOC** | Always works, time-grounded | Requires traversal | None | N/A (foundation) |
| **Agentic** | No dependencies | Slow for large scans | Timeout | Return partial |
| **BM25** | Fast exact keyword | Misses synonyms | Index unavailable | Agentic |
| **Vector** | Semantic similarity | Can hallucinate relevance | Index unavailable | BM25 or Agentic |
| **Topics** | Pattern discovery | Stale labels, overly broad | Index unavailable | Vector or BM25 |

---

## Stop Conditions (Safety Bounds)

Every retrieval operation must respect these safety bounds:

| Condition | Default | Configurable | Purpose |
|-----------|---------|--------------|---------|
| `max_depth` | 5 levels | Yes | Prevent infinite drill-down |
| `max_nodes_visited` | 100 | Yes | Bound exploration breadth |
| `max_rpc_calls` | 20 | Yes | Prevent API explosion |
| `max_token_budget` | 4000 | Yes | Context window protection |
| `timeout_ms` | 5000 | Yes | Latency ceiling |
| `beam_width` | 1 (sequential) | Yes (2-5 for parallel) | Control parallelism |

**Time-boxed intent** enforces these strictly. Other intents use them as soft limits with escalation.

---

## Design Philosophy

The following principles guide all architectural decisions:

| Principle | Description |
|-----------|-------------|
| **Time is truth** | Events are immutable, time-ordered, append-only |
| **Summaries before detail** | Progressive disclosure minimizes context usage |
| **Grips provide provenance** | Every claim links to source evidence |
| **Agents navigate, not scan** | Hierarchical exploration beats brute-force |
| **Indexes accelerate, never required** | Any index can fail; TOC always works |
| **Intent determines routing** | Different query types use different paths |
| **Tools don't decide - skills decide** | The control plane is the skill ecosystem |

---

## For Skill Implementers

Skills that interact with Agent Memory must follow the **Agent Retrieval Policy**:

1. **Check availability** before using any layer (GetTeleportStatus, GetVectorIndexStatus, GetTopicGraphStatus)
2. **Implement fallback chains** - never hard-fail if agentic search can run
3. **Respect budgets** - enforce max_rpc_calls, token_budget, timeout
4. **Explain decisions** - report which tier/mode was used and why
5. **Include evidence** - provide grip_ids/citations when returning facts

**See:** [Agent Retrieval Policy PRD](prds/agent-retrieval-policy-prd.md)

---

## References

### Product Requirements Documents

| PRD | Layer | Purpose |
|-----|-------|---------|
| [Agent Retrieval Policy](prds/agent-retrieval-policy-prd.md) | Brainstem | Tier detection, intent routing, fallbacks |
| [Agentic TOC Search](prds/agentic-toc-search-prd.md) | Layer 2 | Index-free search |
| [BM25 Teleport](prds/bm25-teleport-prd.md) | Layer 3 | Keyword acceleration |
| [Hierarchical Vector Indexing](prds/hierarchical-vector-indexing-prd.md) | Layer 4 | Semantic acceleration |
| [Topic Graph Memory](prds/topic-graph-memory-prd.md) | Layer 5 | Conceptual enrichment |

### Technical Plans

| Plan | Phase | Purpose |
|------|-------|---------|
| [Memory Ranking Enhancements RFC](plans/memory-ranking-enhancements-rfc.md) | 16 | Salience, usage, novelty, lifecycle |
| [Phase 16 Memory Ranking Plan](plans/phase-16-memory-ranking-plan.md) | 16 | Implementation details |
| [Configuration Wizard Skills](plans/configuration-wizard-skills-plan.md) | 15 | Interactive configuration |
| [Topic Graph Memory](plans/topic-graph-memory.md) | 14 | Topic extraction and relationships |

### Planning Documents

- [PROJECT.md](../.planning/PROJECT.md) - Requirements and key decisions
- [ROADMAP.md](../.planning/ROADMAP.md) - Phase execution order
- [STATE.md](../.planning/STATE.md) - Current development state

---

*Manifesto Created: 2026-02-01*
*Updated: 2026-02-05 (Phase 16-17 ranking and retrieval policy)*
*Author: Agent Memory Team*
