---
description: Show recent conversation summaries
---

# Memory Recent

Display recent conversation summaries from the past N days.

## Usage

```
/memory-recent
/memory-recent --days 3
/memory-recent --days 14 --limit 20
```

## Arguments

Parse from `$ARGUMENTS`:
- **--days <N>**: Number of days to look back (default: 7)
- **--limit <N>**: Maximum number of segments to show (default: 10)

Example: `/memory-recent --days 3 --limit 20`
-> $ARGUMENTS = "--days 3 --limit 20"

## Process

1. **Check daemon status**
   ```bash
   memory-daemon status
   ```

2. **Get TOC root** to find current year
   ```bash
   memory-daemon query --endpoint http://[::1]:50051 root
   ```

3. **Navigate to current period**
   ```bash
   memory-daemon query --endpoint http://[::1]:50051 node --node-id "toc:month:2026-01"
   memory-daemon query --endpoint http://[::1]:50051 node --node-id "toc:week:2026-W05"
   ```

4. **Collect recent day nodes** within the specified range

5. **Present summaries** with timestamps and grip IDs

## Output Format

```markdown
## Recent Conversations (Last [N] Days)

### [Date]
**Topics:** [keywords from node]

**Segments:**
1. **[Time]** - [segment title/summary]
   - [bullet 1] `grip:ID`
   - [bullet 2] `grip:ID`

2. **[Time]** - [segment title/summary]
   - [bullet] `grip:ID`

---
Total: [N] segments across [M] days
Expand any excerpt: `/memory-context grip:ID`
```

## Examples

**Show last week's conversations:**
```
/memory-recent
```

**Show last 3 days:**
```
/memory-recent --days 3
```

**Extended history:**
```
/memory-recent --days 30 --limit 50
```

## Skill Reference

This command uses the **memory-query** skill for tier-aware retrieval with automatic fallback chains.
