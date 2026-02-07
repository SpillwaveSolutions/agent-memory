# Upgrading Guide

This document provides upgrade instructions between agent-memory versions, with special attention to backward compatibility and migration requirements.

---

## v2.0.0 to v2.1.0 (Phase 16-17)

**Release Focus:** Memory Ranking Enhancements and Index Lifecycle Automation

### Summary

This release adds new features while maintaining full backward compatibility with v2.0.0 data:

- **Salience Scoring** - Write-time importance scoring for TOC nodes and Grips
- **Usage Tracking** - Access pattern tracking for ranking decay
- **Novelty Filtering** - Prevent near-duplicate event storage
- **Vector Lifecycle Pruning** - Automated vector index cleanup (FR-08)
- **BM25 Lifecycle Pruning** - Automated BM25 index cleanup (FR-09)

### Upgrade Requirements

| Requirement | Status |
|-------------|--------|
| Data Migration | **NOT REQUIRED** |
| Config Migration | **NOT REQUIRED** |
| Schema Changes | Additive only (backward compatible) |
| Breaking Changes | **NONE** |

### What Happens on Upgrade

1. **Existing data reads normally** - All new fields have serde defaults
2. **New features are off or safe by default** - No behavior change without explicit configuration
3. **New column families created lazily** - Only when features are enabled and used
4. **Proto compatibility maintained** - Old clients work without modification

### Feature Defaults (Backward Compatible)

| Feature | Default | Behavior for Existing Data |
|---------|---------|---------------------------|
| Salience Scoring | Enabled | Existing nodes use `salience_score: 0.5` (neutral) |
| Usage Tracking | **DISABLED** | No effect until explicitly enabled |
| Novelty Filtering | **DISABLED** | All events stored (v2.0.0 behavior) |
| Vector Lifecycle | Enabled | Respects retention; protects month/year vectors |
| BM25 Lifecycle | **DISABLED** | Append-only (v2.0.0 behavior) |

### Detailed Changes

#### Schema Additions (TocNode and Grip)

New fields added with defaults:

```rust
// v2.1.0 - Additive fields with serde defaults
pub struct TocNode {
    // ... existing fields unchanged ...

    #[serde(default = "default_salience")]  // Returns 0.5
    pub salience_score: f32,

    #[serde(default)]  // Returns Observation
    pub memory_kind: MemoryKind,

    #[serde(default)]  // Returns false
    pub is_pinned: bool,
}
```

**Impact:** Zero. Existing serialized nodes deserialize correctly with default values.

#### New Column Family: CF_USAGE_COUNTERS

- **Created:** Only when usage tracking is enabled AND first access is recorded
- **If absent:** All usage reads return default values (count=0)
- **Not created on startup:** Lazy initialization

#### Proto Field Additions

New fields use high field numbers to avoid conflicts:

```protobuf
message TocNode {
    // ... existing fields (1-50) unchanged ...

    float salience_score = 101;   // Default: 0.0 (treated as 0.5)
    MemoryKind memory_kind = 102; // Default: OBSERVATION
    bool is_pinned = 103;         // Default: false
}
```

**Proto3 Compatibility:** Unset fields use implicit defaults. Service layer translates `0.0` salience to `0.5` for neutral scoring.

### Enabling New Features

After upgrade, enable features incrementally:

#### 1. Enable Novelty Filtering (Optional)

```toml
# Only if you want to prevent duplicate events
[novelty]
enabled = true
threshold = 0.82
timeout_ms = 50
```

**Note:** Requires vector index to be available. Fails open (stores event) if unavailable.

#### 2. Enable Usage Tracking (Optional)

```toml
# Only if you want usage-based ranking decay
[teleport.ranking.usage_decay]
enabled = true
decay_factor = 0.15
cache_size = 10000
```

**Note:** Creates `CF_USAGE_COUNTERS` column family on first use.

#### 3. Enable BM25 Lifecycle Pruning (Optional)

```toml
# Only if you want to prune old BM25 docs
[teleport.bm25.lifecycle]
enabled = true
segment_retention_days = 30
day_retention_days = 180
```

**Note:** BM25 prune is off by default per "append-only" philosophy.

### Verification

After upgrade, verify system health:

```bash
# Check daemon status
memory-daemon status

# Verify config loaded correctly
memory-daemon config get novelty.enabled
# Expected: false (default)

# Verify existing data readable
memory-daemon query node --node-id "toc:day:2026-01-01"
# Should return node with salience_score: 0.5

# Check teleport status
memory-daemon teleport status
# Should show BM25 and vector health
```

### Rollback Procedure

If issues occur after upgrade:

1. **Disable new features** (no code change needed):
   ```toml
   [novelty]
   enabled = false

   [teleport.ranking.usage_decay]
   enabled = false

   [teleport.bm25.lifecycle]
   enabled = false
   ```

2. **Restart daemon:**
   ```bash
   memory-daemon restart
   ```

3. **Behavior reverts to v2.0.0:**
   - Salience fields retained but unused (factor = 1.0)
   - Usage data retained but ignored
   - No pruning occurs

### Configuration Reference

See [Configuration Reference](references/configuration-reference.md) for complete option documentation.

---

## General Upgrade Guidelines

### Pre-Upgrade Checklist

1. **Backup data:**
   ```bash
   cp -r ~/.local/share/agent-memory/db ~/agent-memory-backup
   ```

2. **Check release notes** for breaking changes

3. **Test in non-production** environment first

4. **Verify disk space** for potential index rebuilds

### Post-Upgrade Checklist

1. **Verify daemon starts:**
   ```bash
   memory-daemon start
   memory-daemon status
   ```

2. **Check logs for errors:**
   ```bash
   memory-daemon logs --tail 100
   ```

3. **Verify data accessible:**
   ```bash
   memory-daemon query toc-root
   ```

4. **Run health check:**
   ```bash
   memory-daemon admin health
   ```

### Index Rebuild (If Needed)

Some upgrades may benefit from index rebuilds:

```bash
# Rebuild BM25 index
memory-daemon admin rebuild-index --type bm25

# Rebuild vector index
memory-daemon admin rebuild-index --type vector

# Rebuild both
memory-daemon admin rebuild-index --type all
```

**Note:** Rebuilds are optional. Indexes are accelerators, not dependencies. The system falls back to TOC navigation if indexes are unavailable.

---

## Version History

| Version | Release Date | Key Changes |
|---------|--------------|-------------|
| v2.1.0 | TBD | Phase 16-17: Ranking enhancements, index lifecycle |
| v2.0.0 | 2026-02-01 | Topic graph, vector search, hybrid search |
| v1.0.0 | 2026-01-15 | Initial release: TOC, BM25, grips |

---

*Last Updated: 2026-02-06*
