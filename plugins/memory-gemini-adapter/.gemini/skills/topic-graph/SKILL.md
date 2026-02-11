---
name: topic-graph
description: |
  Topic graph exploration for agent-memory. Use when asked to "explore topics", "show related concepts", "what themes have I discussed", "find topic connections", or "discover patterns in conversations". Provides semantic topic extraction with time-decayed importance scoring.
license: MIT
metadata:
  version: 1.0.0
  author: SpillwaveSolutions
---

# Topic Graph Skill

Semantic topic exploration using the agent-memory topic graph (Phase 14).

## When to Use

| Use Case | Best Approach |
|----------|---------------|
| Explore recurring themes | Topic Graph |
| Find concept connections | Topic relationships |
| Discover patterns | Top topics by importance |
| Related discussions | Topics for query |
| Time-based topic trends | Topic with decay |

## When Not to Use

- Specific keyword search (use BM25)
- Exact phrase matching (use BM25)
- Current session context (already in memory)
- Cross-project queries (topic graph is per-project)

## Quick Start

| Command | Purpose | Example |
|---------|---------|---------|
| `topics status` | Topic graph health | `topics status` |
| `topics top` | Most important topics | `topics top --limit 10` |
| `topics query` | Find topics for query | `topics query "authentication"` |
| `topics related` | Related topics | `topics related --topic-id topic:abc` |

## Prerequisites

```bash
memory-daemon status  # Check daemon
memory-daemon start   # Start if needed
```

## Validation Checklist

Before presenting results:
- [ ] Daemon running: `memory-daemon status` returns "running"
- [ ] Topic graph enabled: `topics status` shows `Enabled: true`
- [ ] Topics populated: `topics status` shows `Topics: > 0`
- [ ] Query returns results: Check for non-empty topic list

## Topic Graph Status

```bash
memory-daemon topics status
```

Output:
```
Topic Graph Status
----------------------------------------
Enabled:           true
Healthy:           true
Total Topics:      142
Active Topics:     89
Dormant Topics:    53
Last Extraction:   2026-01-30T15:42:31Z
Half-Life Days:    30
```

## Explore Top Topics

Get the most important topics based on time-decayed scoring:

```bash
# Top 10 topics by importance
memory-daemon topics top --limit 10

# Include dormant topics
memory-daemon topics top --include-dormant

# JSON output for processing
memory-daemon topics top --format json
```

Output:
```
Top Topics (by importance)
----------------------------------------
1. authentication (importance: 0.892)
   Mentions: 47, Last seen: 2026-01-30

2. error-handling (importance: 0.756)
   Mentions: 31, Last seen: 2026-01-29

3. rust-async (importance: 0.698)
   Mentions: 28, Last seen: 2026-01-28
```

## Query Topics

Find topics related to a query:

```bash
# Find topics matching query
memory-daemon topics query "JWT authentication"

# With minimum similarity
memory-daemon topics query "debugging" --min-similarity 0.7
```

Output:
```
Topics for: "JWT authentication"
----------------------------------------
1. jwt-tokens (similarity: 0.923)
   Related to: authentication, security, tokens

2. authentication (similarity: 0.891)
   Related to: jwt-tokens, oauth, users
```

## Topic Relationships

Explore connections between topics:

```bash
# Get related topics
memory-daemon topics related --topic-id "topic:authentication"

# Get parent/child hierarchy
memory-daemon topics hierarchy --topic-id "topic:authentication"

# Get similar topics (by embedding)
memory-daemon topics similar --topic-id "topic:jwt-tokens" --limit 5
```

## Topic-Guided Navigation

Use topics to navigate TOC:

```bash
# Find TOC nodes for a topic
memory-daemon topics nodes --topic-id "topic:authentication"
```

Output:
```
TOC Nodes for topic: authentication
----------------------------------------
1. toc:segment:abc123 (2026-01-30)
   "Implemented JWT authentication..."

2. toc:day:2026-01-28
   "Authentication refactoring complete..."
```

## Configuration

Topic graph is configured in `~/.config/agent-memory/config.toml`:

```toml
[topics]
enabled = true  # Enable/disable topic extraction
min_cluster_size = 3  # Minimum mentions for topic
half_life_days = 30  # Time decay half-life
similarity_threshold = 0.7  # For relationship detection

[topics.extraction]
schedule = "0 */4 * * *"  # Every 4 hours
batch_size = 100

[topics.lifecycle]
prune_dormant_after_days = 365
resurrection_threshold = 3  # Mentions to resurrect
```

## Topic Lifecycle

Topics follow a lifecycle with time-decayed importance:

```
New Topic (mention_count: 1)
    |
    v  (more mentions)
Active Topic (importance > 0.1)
    |
    v  (time decay, no new mentions)
Dormant Topic (importance < 0.1)
    |
    v  (new mention)
Resurrected Topic (active again)
```

### Lifecycle Commands

```bash
# View dormant topics
memory-daemon topics dormant

# Force topic extraction
memory-daemon admin extract-topics

# Prune old dormant topics
memory-daemon admin prune-topics --dry-run
```

## Integration with Search

Topics integrate with the retrieval tier system:

| Intent | Topic Role |
|--------|------------|
| Explore | Primary: Start with topics, drill into TOC |
| Answer | Secondary: Topics for context after search |
| Locate | Tertiary: Topics hint at likely locations |

### Explore Workflow

```bash
# 1. Get top topics in area of interest
memory-daemon topics query "performance optimization"

# 2. Find TOC nodes for relevant topic
memory-daemon topics nodes --topic-id "topic:caching"

# 3. Navigate to specific content
memory-daemon query node --node-id "toc:segment:xyz"
```

## Error Handling

| Error | Resolution |
|-------|------------|
| Connection refused | `memory-daemon start` |
| Topics disabled | Enable in config: `topics.enabled = true` |
| No topics found | Run extraction: `admin extract-topics` |
| Stale topics | Check extraction schedule |

## Advanced: Time Decay

Topic importance uses exponential time decay:

```
importance = mention_count * 0.5^(age_days / half_life)
```

With default 30-day half-life:
- Topic mentioned today: full weight
- Topic mentioned 30 days ago: 50% weight
- Topic mentioned 60 days ago: 25% weight

This surfaces recent topics while preserving historical patterns.

## Relationship Types

| Relationship | Description |
|--------------|-------------|
| similar | Topics with similar embeddings |
| parent | Broader topic containing this one |
| child | Narrower topic under this one |
| co-occurring | Topics that appear together |

See [Command Reference](references/command-reference.md) for full CLI options.
