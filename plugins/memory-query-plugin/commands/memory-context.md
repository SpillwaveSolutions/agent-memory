---
name: memory-context
description: Expand a grip to see full conversation context around an excerpt
parameters:
  - name: grip-id
    description: The grip ID to expand (format grip:timestamp:ulid)
    required: true
  - name: before
    description: Number of events to include before excerpt (default 3)
    required: false
    default: 3
  - name: after
    description: Number of events to include after excerpt (default 3)
    required: false
    default: 3
skills:
  - memory-query
---

# Memory Context

Expand a grip to retrieve full conversation context around a specific excerpt.

## Usage

```
/memory-context <grip-id>
/memory-context grip:1706540400000:01HN4QXKN6YWXVKZ3JMHP4BCDE
/memory-context grip:1706540400000:01HN4QXKN6 --before 5 --after 5
```

## Process

1. **Validate grip ID format**
   - Must match: `grip:{13-digit-timestamp}:{26-char-ulid}`

2. **Expand the grip**
   ```bash
   memory-daemon query --endpoint http://[::1]:50051 expand \
     --grip-id "grip:1706540400000:01HN4QXKN6YWXVKZ3JMHP4BCDE" \
     --before 3 \
     --after 3
   ```

3. **Format and present** the conversation thread

## Output Format

```markdown
## Conversation Context

**Grip:** `grip:ID`
**Timestamp:** [human-readable date/time]

### Before (N events)
| Role | Message |
|------|---------|
| user | [message text] |
| assistant | [response text] |

### Excerpt (Referenced)
> [The excerpt text that was summarized]

### After (N events)
| Role | Message |
|------|---------|
| assistant | [continuation] |
| user | [follow-up] |

---
**Source:** [segment ID]
**Session:** [session ID]
```

## Examples

**Expand with default context:**
```
/memory-context grip:1706540400000:01HN4QXKN6YWXVKZ3JMHP4BCDE
```

**Expand with more context:**
```
/memory-context grip:1706540400000:01HN4QXKN6 --before 10 --after 10
```

## Error Handling

| Error | Resolution |
|-------|------------|
| Invalid grip format | Verify format: `grip:{timestamp}:{ulid}` |
| Grip not found | The excerpt may have been from a compacted segment |
| Connection refused | Run `memory-daemon start` |
