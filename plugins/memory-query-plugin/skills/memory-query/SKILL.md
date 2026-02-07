---
name: memory-query
description: |
  Query past conversations from the agent-memory system. Use when asked to "recall what we discussed", "search conversation history", "find previous session", "what did we talk about last week", or "get context from earlier". Provides tier-aware retrieval with automatic fallback chains, intent-based routing, and full explainability.
license: MIT
metadata:
  version: 2.0.0
  author: SpillwaveSolutions
---

# Memory Query Skill

Query past conversations using intelligent tier-based retrieval with automatic fallback chains and query intent classification.

## When Not to Use

- Current session context (already in memory)
- Real-time conversation (skill queries historical data only)
- Cross-project search (memory stores are per-project)

## Quick Start

| Command | Purpose | Example |
|---------|---------|---------|
| `/memory-search <topic>` | Search by topic | `/memory-search authentication` |
| `/memory-recent` | Recent summaries | `/memory-recent --days 7` |
| `/memory-context <grip>` | Expand excerpt | `/memory-context grip:...` |
| `retrieval status` | Check tier capabilities | `memory-daemon retrieval status` |

## Prerequisites

```bash
memory-daemon status  # Check daemon
memory-daemon start   # Start if needed
```

## Validation Checklist

Before presenting results:
- [ ] Daemon running: `memory-daemon status` returns "running"
- [ ] Retrieval tier detected: `retrieval status` shows tier and layers
- [ ] TOC populated: `root` command returns year nodes
- [ ] Query returns results: Check for non-empty `bullets` arrays
- [ ] Grip IDs valid: Format matches `grip:{13-digit-ms}:{26-char-ulid}`

## Retrieval Tiers

The system automatically detects available capability tiers:

| Tier | Name | Available Layers | Best For |
|------|------|------------------|----------|
| 1 | Full | Topics + Hybrid + Agentic | Semantic exploration, topic discovery |
| 2 | Hybrid | BM25 + Vector + Agentic | Balanced keyword + semantic |
| 3 | Semantic | Vector + Agentic | Conceptual similarity search |
| 4 | Keyword | BM25 + Agentic | Exact term matching |
| 5 | Agentic | TOC navigation only | Always works (no indices) |

Check current tier:
```bash
memory-daemon retrieval status
```

## Query Intent Classification

Queries are automatically classified by intent for optimal routing:

| Intent | Characteristics | Strategy |
|--------|----------------|----------|
| **Explore** | "browse", "what topics", "discover" | Topics-first, broad search |
| **Answer** | "what did", "how did", "find" | Precision-focused, hybrid |
| **Locate** | Specific identifiers, exact phrases | BM25-first, keyword match |
| **Time-boxed** | "yesterday", "last week", date refs | TOC navigation + filters |

The classifier extracts time constraints automatically:
```
Query: "What did we discuss about JWT last Tuesday?"
-> Intent: Answer
-> Time constraint: 2026-01-28 (Tuesday)
-> Keywords: ["JWT"]
```

## Fallback Chains

The system automatically falls back when layers are unavailable:

```
Tier 1: Topics → Hybrid → Vector → BM25 → Agentic
Tier 2: Hybrid → Vector → BM25 → Agentic
Tier 3: Vector → BM25 → Agentic
Tier 4: BM25 → Agentic
Tier 5: Agentic (always works)
```

**Fallback triggers:**
- Layer returns no results
- Layer timeout exceeded
- Layer health check failed

## Explainability

Every query result includes an explanation:

```json
{
  "tier_used": 2,
  "tier_name": "Hybrid",
  "method": "bm25_then_vector",
  "layers_tried": ["bm25", "vector"],
  "fallbacks_used": [],
  "time_constraint": "2026-01-28",
  "stop_reason": "max_results_reached",
  "confidence": 0.87
}
```

Display to user:
```
📊 Search used: Hybrid tier (BM25 + Vector)
📍 0 fallbacks needed
⏱️ Time filter: 2026-01-28
```

## TOC Navigation

Hierarchical time-based structure:

```
Year → Month → Week → Day → Segment
```

**Node ID formats:**
- `toc:year:2026`
- `toc:month:2026-01`
- `toc:week:2026-W04`
- `toc:day:2026-01-30`

## Intelligent Search

The retrieval system routes queries through optimal layers based on intent and tier.

### Intent-Driven Workflow

1. **Classify intent** - System determines query type:
   ```bash
   memory-daemon retrieval classify "What JWT discussions happened last week?"
   # Intent: Answer, Time: last week, Keywords: [JWT]
   ```

2. **Route through optimal layers** - Automatic tier detection:
   ```bash
   memory-daemon retrieval route "JWT authentication"
   # Tier: 2 (Hybrid), Method: bm25_then_vector
   ```

3. **Execute with fallbacks** - Automatic failover:
   ```bash
   memory-daemon teleport search "JWT authentication" --top-k 10
   # Falls back to agentic if indices unavailable
   ```

4. **Expand grip for verification**:
   ```bash
   memory-daemon query expand --grip-id "grip:..." --before 3 --after 3
   ```

### Teleport Search (BM25 + Vector)

For Tier 1-4, use teleport commands for fast index-based search:

```bash
# BM25 keyword search
memory-daemon teleport search "authentication error"

# Vector semantic search
memory-daemon teleport vector "conceptual understanding of auth"

# Hybrid search (best of both)
memory-daemon teleport hybrid "JWT token validation"
```

### Topic-Based Discovery (Tier 1 only)

When topics are available, explore conceptually:

```bash
# Find related topics
memory-daemon topics query "authentication"

# Get top topics by importance
memory-daemon topics top --limit 10

# Navigate from topic to TOC nodes
memory-daemon topics nodes --topic-id "topic:authentication"
```

### Search Command Reference

```bash
# Search within a specific node
memory-daemon query search --node "toc:month:2026-01" --query "debugging"

# Search children of a parent
memory-daemon query search --parent "toc:week:2026-W04" --query "JWT token"

# Search root level (years)
memory-daemon query search --query "authentication"

# Filter by fields (title, summary, bullets, keywords)
memory-daemon query search --query "JWT" --fields "title,bullets" --limit 20
```

### Agent Navigation Loop

When answering "find discussions about X":

1. **Check retrieval capabilities**:
   ```bash
   memory-daemon retrieval status
   # Returns: Tier 2 (Hybrid) - BM25 + Vector available
   ```

2. **Classify query intent**:
   ```bash
   memory-daemon retrieval classify "What JWT discussions happened last week?"
   # Intent: Answer, Time: 2026-W04, Keywords: [JWT]
   ```

3. **Route through optimal layers**:
   - **Tier 1-4**: Use teleport for fast results
   - **Tier 5**: Fall back to agentic TOC navigation

4. **Execute with stop conditions**:
   - `max_depth`: How deep to drill (default: 3)
   - `max_nodes`: Max nodes to visit (default: 50)
   - `timeout_ms`: Query timeout (default: 5000)

5. **Return results with explainability**:
   ```
   📊 Method: Hybrid (BM25 + Vector reranking)
   ⏱️ Time filter: 2026-W04
   📍 Layers: bm25 → vector
   ```

Example with tier-aware routing:
```
Query: "What JWT discussions happened last week?"
-> retrieval status -> Tier 2 (Hybrid)
-> retrieval classify -> Intent: Answer, Time: 2026-W04
-> teleport hybrid "JWT" --time-filter 2026-W04
  -> Match: toc:segment:abc123 (score: 0.92)
-> Return bullets with grip IDs
-> Offer: "Found 2 relevant points. Expand grip:xyz for context?"
-> Include: "Used Hybrid tier, BM25+Vector, 0 fallbacks"
```

### Agentic Fallback (Tier 5)

When indices are unavailable:

```
Query: "What JWT discussions happened last week?"
-> retrieval status -> Tier 5 (Agentic only)
-> query search --parent "toc:week:2026-W04" --query "JWT"
  -> Day 2026-01-30 (score: 0.85)
-> query search --parent "toc:day:2026-01-30" --query "JWT"
  -> Segment abc123 (score: 0.78)
-> Return bullets from Segment with grip IDs
-> Include: "Used Agentic tier (indices unavailable)"
```

## CLI Reference

```bash
# Get root periods
memory-daemon query --endpoint http://[::1]:50051 root

# Navigate node
memory-daemon query --endpoint http://[::1]:50051 node --node-id "toc:year:2026"

# Browse children
memory-daemon query --endpoint http://[::1]:50051 browse --parent-id "toc:month:2026-01"

# Expand grip
memory-daemon query --endpoint http://[::1]:50051 expand --grip-id "grip:..." --before 3 --after 3
```

## Response Format

```markdown
## Memory Results: [query]

### [Time Period]
**Summary:** [bullet points]

**Excerpts:**
- "[excerpt]" `grip:ID`

---
Expand: `/memory-context grip:ID`
```

## Error Handling

| Error | Resolution |
|-------|------------|
| Connection refused | `memory-daemon start` |
| No results | Broaden search or check different period |
| Invalid grip | Verify format: `grip:{timestamp}:{ulid}` |

## Advanced

See [Command Reference](references/command-reference.md) for full CLI options.
