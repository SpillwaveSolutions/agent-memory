---
name: topic-graph
description: |
  Topic graph exploration for agent-memory. Use when asked to "explore topics", "show related concepts", "what themes have I discussed", "find topic connections", or "discover patterns in conversations". Provides semantic topic extraction with time-decayed importance scoring.
---

# Topic Graph Skill

Semantic topic exploration using the agent-memory topic graph.

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

## Query Topics

Find topics related to a query:

```bash
# Find topics matching query
memory-daemon topics query "JWT authentication"

# With minimum similarity
memory-daemon topics query "debugging" --min-similarity 0.7
```

## Topic Relationships

Explore connections between topics:

```bash
# Get related topics
memory-daemon topics related --topic-id "topic:authentication"

# Get similar topics (by embedding)
memory-daemon topics similar --topic-id "topic:jwt-tokens" --limit 5
```

## Topic-Guided Navigation

Use topics to navigate TOC:

```bash
# Find TOC nodes for a topic
memory-daemon topics nodes --topic-id "topic:authentication"
```

## Error Handling

| Error | Resolution |
|-------|------------|
| Connection refused | `memory-daemon start` |
| Topics disabled | Enable in config: `topics.enabled = true` |
| No topics found | Run extraction: `admin extract-topics` |
| Stale topics | Check extraction schedule |

See [Command Reference](references/command-reference.md) for full CLI options.
