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

## Search-Based Navigation

Use search RPCs to efficiently find relevant content without scanning everything.

### Search Workflow

1. **Search at root level** - Find which time periods are relevant:
   ```bash
   memory-daemon query search --query "JWT authentication"
   # Returns: Year/Month nodes with relevance scores
   ```

2. **Drill into best match** - Search children of matching period:
   ```bash
   memory-daemon query search --parent "toc:month:2026-01" --query "JWT authentication"
   # Returns: Week nodes with matches
   ```

3. **Continue until Segment level** - Extract evidence:
   ```bash
   memory-daemon query search --parent "toc:day:2026-01-30" --query "JWT"
   # Returns: Segment nodes with bullet matches and grip IDs
   ```

4. **Expand grip for verification**:
   ```bash
   memory-daemon query expand --grip-id "grip:..." --before 3 --after 3
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

1. Parse query for time hints ("last week", "in January", "yesterday")
2. Start at appropriate level based on hints, or root if no hints
3. Use `search_children` to find relevant nodes at each level
4. Drill into highest-scoring matches
5. At Segment level, extract bullets with grip IDs
6. Offer to expand grips for full context

Example path:
```
Query: "What JWT discussions happened last week?"
-> SearchChildren(parent="toc:week:2026-W04", query="JWT")
  -> Day 2026-01-30 (score: 0.85)
-> SearchChildren(parent="toc:day:2026-01-30", query="JWT")
  -> Segment abc123 (score: 0.92)
-> Return bullets from Segment with grip IDs
-> Offer: "Found 2 relevant points. Expand grip:xyz for context?"
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
