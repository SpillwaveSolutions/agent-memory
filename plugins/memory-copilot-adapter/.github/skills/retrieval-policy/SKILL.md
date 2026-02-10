---
name: retrieval-policy
description: |
  Agent retrieval policy for intelligent memory search. Use when implementing memory queries to detect capabilities, classify intent, route through optimal layers, and handle fallbacks. Provides tier detection, intent classification, fallback chains, and full explainability for all retrieval operations.
license: MIT
metadata:
  version: 2.1.0
  author: SpillwaveSolutions
---

# Retrieval Policy Skill

Intelligent retrieval decision-making for agent memory queries. The "brainstem" that decides how to search.

## When to Use

| Use Case | Best Approach |
|----------|---------------|
| Detect available search capabilities | `retrieval status` |
| Classify query intent | `retrieval classify <query>` |
| Route query through optimal layers | `retrieval route <query>` |
| Understand why a method was chosen | Check explainability payload |
| Handle layer failures gracefully | Automatic fallback chains |

## When Not to Use

- Direct search operations (use memory-query skill)
- Topic exploration (use topic-graph skill)
- BM25 keyword search (use bm25-search skill)
- Vector semantic search (use vector-search skill)

## Quick Start

```bash
# Check retrieval tier
memory-daemon retrieval status

# Classify query intent
memory-daemon retrieval classify "What JWT issues did we have?"

# Route query through layers
memory-daemon retrieval route "authentication errors last week"
```

## Capability Tiers

The system detects available layers and maps to tiers:

| Tier | Name | Layers Available | Description |
|------|------|------------------|-------------|
| 1 | Full | Topics + Hybrid + Agentic | Complete cognitive stack |
| 2 | Hybrid | BM25 + Vector + Agentic | Keyword + semantic |
| 3 | Semantic | Vector + Agentic | Embeddings only |
| 4 | Keyword | BM25 + Agentic | Text matching only |
| 5 | Agentic | Agentic only | TOC navigation (always works) |

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

## Query Intent Classification

Queries are classified into four intents:

| Intent | Triggers | Optimal Strategy |
|--------|----------|------------------|
| **Explore** | "browse", "discover", "what topics" | Topics-first, broad fan-out |
| **Answer** | "what did", "how did", "find" | Hybrid, precision-focused |
| **Locate** | Identifiers, exact phrases, quotes | BM25-first, exact match |
| **Time-boxed** | "yesterday", "last week", dates | Time-filtered, sequential |

### Classification Command

```bash
memory-daemon retrieval classify "What JWT issues did we debug last Tuesday?"
```

Output:
```
Query Intent Classification
----------------------------------------
Intent:          Answer
Confidence:      0.87
Time Constraint: 2026-01-28 (last Tuesday)
Keywords:        [JWT, issues, debug]
Suggested Mode:  Hybrid (BM25 + Vector)
```

## Fallback Chains

Each tier has a predefined fallback chain:

```
Tier 1: Topics -> Hybrid -> Vector -> BM25 -> Agentic
Tier 2: Hybrid -> Vector -> BM25 -> Agentic
Tier 3: Vector -> BM25 -> Agentic
Tier 4: BM25 -> Agentic
Tier 5: Agentic (no fallback needed)
```

### Fallback Triggers

| Condition | Action |
|-----------|--------|
| Layer returns 0 results | Try next layer |
| Layer timeout exceeded | Skip to next layer |
| Layer health check failed | Skip layer entirely |
| Min confidence not met | Continue to next layer |

## Stop Conditions

Control query execution with stop conditions:

| Condition | Default | Description |
|-----------|---------|-------------|
| `max_depth` | 3 | Maximum drill-down levels |
| `max_nodes` | 50 | Maximum nodes to visit |
| `timeout_ms` | 5000 | Query timeout in milliseconds |
| `beam_width` | 3 | Parallel branches to explore |
| `min_confidence` | 0.5 | Minimum result confidence |

### Intent-Specific Defaults

| Intent | max_nodes | timeout_ms | beam_width |
|--------|-----------|------------|------------|
| Explore | 100 | 10000 | 5 |
| Answer | 50 | 5000 | 3 |
| Locate | 20 | 3000 | 1 |
| Time-boxed | 30 | 4000 | 2 |

## Execution Modes

| Mode | Description | Best For |
|------|-------------|----------|
| **Sequential** | One layer at a time, stop on success | Locate intent, exact matches |
| **Parallel** | All layers simultaneously, merge results | Explore intent, broad discovery |
| **Hybrid** | Primary layer + backup, merge with weights | Answer intent, balanced results |

## Explainability Payload

Every retrieval returns an explanation:

```json
{
  "tier_used": 2,
  "tier_name": "Hybrid",
  "intent": "Answer",
  "method": "bm25_then_vector",
  "layers_tried": ["bm25", "vector"],
  "layers_succeeded": ["bm25", "vector"],
  "fallbacks_used": [],
  "time_constraint": "2026-01-28",
  "stop_reason": "max_results_reached",
  "results_per_layer": {
    "bm25": 5,
    "vector": 3
  },
  "execution_time_ms": 234,
  "confidence": 0.87
}
```

### Displaying to Users

```
## Retrieval Report

Method: Hybrid tier (BM25 + Vector reranking)
Layers: bm25 (5 results), vector (3 results)
Fallbacks: 0
Time filter: 2026-01-28
Execution: 234ms
Confidence: 0.87
```

## Skill Contract

When implementing memory queries, follow this contract:

### Required Steps

1. **Always check tier first**:
   ```bash
   memory-daemon retrieval status
   ```

2. **Classify intent before routing**:
   ```bash
   memory-daemon retrieval classify "<query>"
   ```

3. **Use tier-appropriate commands**:
   - Tier 1-2: `teleport hybrid`
   - Tier 3: `teleport vector`
   - Tier 4: `teleport search`
   - Tier 5: `query search`

4. **Include explainability in response**:
   - Report tier used
   - Report layers tried
   - Report fallbacks triggered

### Validation Checklist

Before returning results:
- [ ] Tier detection completed
- [ ] Intent classified
- [ ] Appropriate layers used for tier
- [ ] Fallbacks handled gracefully
- [ ] Explainability payload included
- [ ] Stop conditions respected

## Configuration

Retrieval policy is configured in `~/.config/agent-memory/config.toml`:

```toml
[retrieval]
default_timeout_ms = 5000
default_max_nodes = 50
default_max_depth = 3
parallel_fan_out = 3

[retrieval.intent_defaults]
explore_beam_width = 5
answer_beam_width = 3
locate_early_stop = true
timeboxed_max_depth = 2

[retrieval.fallback]
enabled = true
max_fallback_attempts = 3
fallback_timeout_factor = 0.5
```

## Error Handling

| Error | Resolution |
|-------|------------|
| All layers failed | Return Tier 5 (Agentic) results |
| Timeout exceeded | Return partial results with explanation |
| No results found | Broaden query or suggest alternatives |
| Intent unclear | Default to Answer intent |

## Integration with Ranking

Results are ranked using Phase 16 signals:

| Signal | Weight | Description |
|--------|--------|-------------|
| Salience score | 0.3 | Memory importance (Procedure > Observation) |
| Recency | 0.3 | Time-decayed scoring |
| Relevance | 0.3 | BM25/Vector match score |
| Usage | 0.1 | Access frequency (if enabled) |

See [Command Reference](references/command-reference.md) for full CLI options.
