---
name: memory-search
description: Search past conversations by topic or keyword
parameters:
  - name: topic
    description: Topic or keyword to search for
    required: true
  - name: period
    description: Time period to search (e.g., "last week", "january", "2026")
    required: false
skills:
  - memory-query
---

# Memory Search

Search past conversations by topic or keyword using hierarchical TOC navigation.

## Usage

```
/memory-search <topic>
/memory-search <topic> --period "last week"
/memory-search authentication
/memory-search "database migration" --period january
```

## Process

1. **Check daemon status**
   ```bash
   memory-daemon status
   ```

2. **Get TOC root** to find available time periods
   ```bash
   memory-daemon query --endpoint http://[::1]:50051 root
   ```

3. **Navigate to relevant period** based on `--period` or search all
   ```bash
   memory-daemon query --endpoint http://[::1]:50051 node --node-id "toc:year:2026"
   memory-daemon query --endpoint http://[::1]:50051 browse --parent-id "toc:month:2026-01" --limit 20
   ```

4. **Search node summaries** for matching keywords in bullets/keywords fields

5. **Present results** with grip IDs for drill-down

## Output Format

```markdown
## Memory Search: [topic]

### [Time Period]
**Summary:** [matching bullet points]

**Excerpts:**
- "[excerpt text]" `grip:ID`
  _Source: [timestamp]_

---
Expand any excerpt: `/memory-context grip:ID`
```

## Examples

**Search for authentication discussions:**
```
/memory-search authentication
```

**Search within specific period:**
```
/memory-search "JWT tokens" --period "last week"
```
