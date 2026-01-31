---
name: memory-query
description: |
  Query past conversations from the agent-memory system. Use when asked to "recall what we discussed", "search conversation history", "find previous session", "what did we talk about last week", or "get context from earlier". Provides hierarchical TOC navigation for topic search, recent summaries, and excerpt expansion.
license: MIT
metadata:
  version: 1.0.0
  author: SpillwaveSolutions
---

# Memory Query Skill

Query past conversations from the agent-memory system using Progressive Disclosure via time-based TOC navigation.

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

## Prerequisites

```bash
memory-daemon status  # Check daemon
memory-daemon start   # Start if needed
```

## Validation Checklist

Before presenting results:
- [ ] Daemon running: `memory-daemon status` returns "running"
- [ ] TOC populated: `root` command returns year nodes
- [ ] Query returns results: Check for non-empty `bullets` arrays
- [ ] Grip IDs valid: Format matches `grip:{13-digit-ms}:{26-char-ulid}`

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
