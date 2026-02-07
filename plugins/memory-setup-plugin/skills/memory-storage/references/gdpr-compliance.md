# GDPR Compliance Mode

GDPR mode enables privacy-focused data handling compliant with the European Union's General Data Protection Regulation.

## What GDPR Mode Enables

When GDPR mode is enabled:

1. **Complete Data Removal** - No tombstones or soft deletes; data is fully removed
2. **Audit Logging** - All deletions are logged with timestamps
3. **Export-Before-Delete** - Option to export data before deletion
4. **Right to Erasure Support** - API endpoint for data subject deletion requests

## When to Enable

Enable GDPR mode if:

- Your users are in the European Union
- You process data of EU data subjects
- Your organization has GDPR compliance requirements
- You want privacy-first data handling
- You need audit trails for data deletion

## Trade-offs

| Aspect | Standard Mode | GDPR Mode |
|--------|---------------|-----------|
| Deletion | Soft delete (tombstones) | Complete removal |
| Recovery | Possible from tombstones | Not possible |
| Audit | Optional | Required, automatic |
| Export | Manual | Pre-deletion export available |
| Performance | Faster deletes | Slightly slower (logging) |
| Storage | Keeps tombstones | No tombstone overhead |
| Compliance | Basic | EU GDPR compliant |

## Configuration

```toml
[retention]
policy = "days:90"
gdpr_mode = true

# Optional: export before delete
gdpr_export_before_delete = true
gdpr_export_path = "~/.memory-exports"

# Audit log location
gdpr_audit_log = "~/.memory-logs/gdpr-audit.log"
```

## Data Subject Rights

GDPR mode supports these data subject rights:

### Right to Access (Article 15)

Export all data for a specific agent or session:

```bash
# Export all data
memory-daemon admin export --format json --output ~/my-data.json

# Export specific date range
memory-daemon admin export --from 2024-01-01 --to 2024-12-31
```

### Right to Erasure (Article 17)

Delete all data for a specific agent:

```bash
# Delete all data (with audit log)
memory-daemon admin delete --all --gdpr

# Delete specific sessions
memory-daemon admin delete --session-id abc123 --gdpr
```

### Right to Portability (Article 20)

Export in machine-readable format:

```bash
# Export as JSON
memory-daemon admin export --format json

# Export as CSV
memory-daemon admin export --format csv
```

## Audit Log Format

GDPR mode creates audit logs for all data operations:

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "action": "delete",
  "data_type": "conversation_events",
  "count": 150,
  "reason": "retention_policy",
  "retention_days": 90,
  "exported_before_delete": true,
  "export_path": "~/.memory-exports/2024-01-15.json"
}
```

## Enabling GDPR Mode

Via wizard:
```
/memory-storage --advanced
```

Or manually in config.toml:
```toml
[retention]
gdpr_mode = true
```

## Verification

Check GDPR mode status:

```bash
# View current GDPR settings
grep gdpr ~/.config/memory-daemon/config.toml

# View audit log
tail -20 ~/.memory-logs/gdpr-audit.log
```

## Best Practices

1. **Enable audit logging** - Required for compliance documentation
2. **Set appropriate retention** - Match your data retention policy
3. **Regular exports** - Schedule periodic exports for backup
4. **Document procedures** - Have clear data subject request procedures
5. **Test deletion** - Verify complete data removal works

## Limitations

- GDPR mode deletions are **irreversible**
- Export-before-delete increases deletion time
- Audit logs themselves must be retained per your policy
- Does not handle data in external systems (LLM providers)
