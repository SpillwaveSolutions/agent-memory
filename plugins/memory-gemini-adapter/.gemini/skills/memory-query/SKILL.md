---
name: memory-query
description: |
  Query past conversations from the agent-memory system. Use when asked to "recall what we discussed", "search conversation history", "find previous session", "what did we talk about last week", or "get context from earlier". Provides tier-aware retrieval with automatic fallback chains, intent-based routing, and full explainability. Includes embedded Navigator agent logic for autonomous complex query handling.
license: MIT
metadata:
  version: 2.0.0
  author: SpillwaveSolutions
---

# Memory Query Skill

Query past conversations using intelligent tier-based retrieval with automatic fallback chains and query intent classification.

This skill includes embedded **Navigator Mode** for autonomous complex query handling. When Gemini activates this skill, it gets both the query capability AND the navigator intelligence -- no separate agent definition is needed.

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
Tier 1: Topics -> Hybrid -> Vector -> BM25 -> Agentic
Tier 2: Hybrid -> Vector -> BM25 -> Agentic
Tier 3: Vector -> BM25 -> Agentic
Tier 4: BM25 -> Agentic
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
Search used: Hybrid tier (BM25 + Vector)
0 fallbacks needed
Time filter: 2026-01-28
```

---

## Navigator Mode

**This section embeds the Navigator agent logic directly into this skill.** When Gemini encounters queries about past conversations, it should activate Navigator Mode to provide intelligent, tier-aware retrieval with full explainability.

### Trigger Patterns

Activate Navigator Mode when a user query matches any of these patterns:

- "what (did|were) we (discuss|talk|work)" -- past conversation recall
- "(remember|recall|find).*(conversation|discussion|session)" -- explicit memory requests
- "(last|previous|earlier) (session|conversation|time)" -- temporal references
- "context from (last|previous|yesterday|last week)" -- context retrieval
- "(explore|discover|browse).*(topics|themes|patterns)" -- topic exploration
- "search conversation history" -- direct search requests
- "find previous session" -- session lookup
- "get context from earlier" -- context retrieval

**Tip:** Any query about past conversations, previous sessions, or recalling what was discussed should trigger Navigator Mode.

### Navigator Process

When Navigator Mode is activated, execute these steps. Where possible, invoke steps in parallel to minimize latency (e.g., classify intent while checking retrieval status).

#### Step 1: Check Retrieval Capabilities (parallel with Step 2)

```bash
memory-daemon retrieval status
```

Note the tier (1-5) and available layers. This determines the search strategy.

#### Step 2: Classify Query Intent (parallel with Step 1)

```bash
memory-daemon retrieval classify "<user query>"
```

Determine: Intent (Explore/Answer/Locate/Time-boxed), time constraints, keywords.

#### Step 3: Select Execution Mode

Based on the classified intent, select the execution strategy:

| Intent | Execution Mode | Stop Conditions |
|--------|---------------|-----------------|
| **Explore** | Parallel (broad fan-out) | max_nodes: 100, beam_width: 5 |
| **Answer** | Hybrid (precision) | max_nodes: 50, min_confidence: 0.6 |
| **Locate** | Sequential (exact match) | max_nodes: 20, first_match: true |
| **Time-boxed** | Sequential + time filter | max_depth: 2, time_constraint: set |

#### Step 4: Execute Through Layer Chain

Route through layers based on the detected tier:

**Tier 1-2 (Hybrid available):**
```bash
# Try hybrid search first
memory-daemon teleport hybrid-search -q "<query>" --top-k 10

# If no results, fall back to individual layers
memory-daemon teleport search "<keywords>" --top-k 20
memory-daemon teleport vector-search -q "<query>" --top-k 10
```

**Tier 3 (Vector only):**
```bash
memory-daemon teleport vector-search -q "<query>" --top-k 10
```

**Tier 4 (BM25 only):**
```bash
memory-daemon teleport search "<keywords>" --top-k 10
```

**Tier 5 (Agentic fallback -- always works):**
```bash
# Navigate TOC hierarchy
memory-daemon query --endpoint http://[::1]:50051 root
memory-daemon query search --query "<keywords>" --limit 20
memory-daemon query search --parent "toc:week:2026-W06" --query "<keywords>"
```

#### Step 5: Apply Stop Conditions

- `max_depth`: Stop drilling at N levels (default: 3)
- `max_nodes`: Stop after visiting N nodes (default: 50)
- `timeout_ms`: Stop after N milliseconds (default: 5000)
- `min_confidence`: Skip results below threshold (default: 0.5)

#### Step 6: Collect and Rank Results

Rank results using retrieval signals:
- **Salience score** (0.3 weight): Memory importance (Procedure > Observation)
- **Recency** (0.3 weight): Time-decayed scoring
- **Relevance** (0.3 weight): BM25/Vector match score
- **Usage** (0.1 weight): Access frequency (if enabled)

#### Step 7: Expand Relevant Grips

For the top results, expand grips to provide conversation context:

```bash
memory-daemon query --endpoint http://[::1]:50051 expand \
  --grip-id "grip:..." --before 5 --after 5
```

#### Step 8: Return with Explainability

Always include in the response:
- Tier used and why
- Layers tried and which succeeded
- Fallbacks triggered (if any)
- Confidence scores
- Time constraints applied

### Navigator Output Format

```markdown
## Memory Navigation Results

**Query:** [user's question]
**Intent:** [Explore | Answer | Locate | Time-boxed]
**Tier:** [1-5] ([Full | Hybrid | Semantic | Keyword | Agentic])
**Matches:** [N results from M layers]

### Summary

[Synthesized answer to the user's question]

### Source Conversations

#### [Date 1] (score: 0.92, salience: 0.85)
> [Relevant excerpt]
`grip:ID1`

#### [Date 2] (score: 0.87, salience: 0.78)
> [Relevant excerpt]
`grip:ID2`

### Related Topics (if Tier 1)

- [Topic 1] (importance: 0.89) - mentioned in [N] conversations
- [Topic 2] (importance: 0.76) - mentioned in [M] conversations

### Retrieval Explanation

**Method:** Hybrid (BM25 -> Vector reranking)
**Layers tried:** bm25, vector
**Time filter:** 2026-01-28
**Fallbacks:** 0
**Confidence:** 0.87

---
Expand any excerpt: /memory-context grip:ID
Search related: /memory-search [topic]
```

### Topic-Guided Discovery (Tier 1)

When topics are available, use them for conceptual exploration:

```bash
# Find related topics
memory-daemon topics query "authentication"

# Get TOC nodes for a topic
memory-daemon topics nodes --topic-id "topic:jwt"

# Explore topic relationships
memory-daemon topics related --topic-id "topic:authentication" --type similar
```

### Parallel Invocation

For optimal performance, Gemini should invoke retrieval steps in parallel where possible:

1. **Parallel pair:** `retrieval status` + `retrieval classify` (no dependency)
2. **Sequential:** Use tier from status + intent from classify to select execution mode
3. **Parallel pair:** Multiple layer searches if mode is Parallel (Explore intent)
4. **Sequential:** Rank results, then expand top grips

This minimizes round-trips and reduces total query latency.

---

## TOC Navigation

Hierarchical time-based structure:

```
Year -> Month -> Week -> Day -> Segment
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
   Method: Hybrid (BM25 + Vector reranking)
   Time filter: 2026-W04
   Layers: bm25 -> vector
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

## Limitations

- Cannot access conversations not yet ingested into memory-daemon
- Topic layer (Tier 1) requires topics.enabled = true in config
- Novelty filtering is opt-in and may exclude repeated mentions
- Cross-project search not supported (memory stores are per-project)

## Advanced

See [Command Reference](references/command-reference.md) for full CLI options.
