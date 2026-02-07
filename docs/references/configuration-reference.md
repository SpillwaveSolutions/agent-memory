# Configuration Reference

This document provides a complete reference for all configuration options in agent-memory, with special attention to Phase 16-17 features and their backward-compatibility defaults.

## Configuration File Location

Configuration is loaded from `~/.config/agent-memory/config.toml` with the following precedence:

1. Built-in defaults (lowest)
2. Config file (`~/.config/agent-memory/config.toml`)
3. CLI-specified config file (`--config path/to/config.toml`)
4. Environment variables (`MEMORY_*`)
5. CLI flags (highest)

---

## Core Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `db_path` | string | `~/.local/share/agent-memory/db` | Path to RocksDB storage directory |
| `grpc_port` | u16 | `50051` | gRPC server port |
| `grpc_host` | string | `0.0.0.0` | gRPC server bind address |
| `log_level` | string | `info` | Log level (trace, debug, info, warn, error) |
| `search_index_path` | string | `~/.local/share/agent-memory/bm25-index` | Path to BM25 Tantivy index |
| `vector_index_path` | string | `~/.local/share/agent-memory/vector-index` | Path to HNSW vector index |

### Multi-Agent Mode

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `multi_agent_mode` | enum | `separate` | Storage mode: `separate` (per-project RocksDB) or `unified` (single store with tags) |
| `agent_id` | string | `null` | Agent ID for unified mode (used as tag prefix) |

---

## Summarizer Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `summarizer.provider` | string | `openai` | LLM provider for summarization |
| `summarizer.model` | string | `gpt-4o-mini` | Model name for summarization |
| `summarizer.api_key` | string | `null` | API key (prefer env var `OPENAI_API_KEY`) |
| `summarizer.api_base_url` | string | `null` | Custom API endpoint URL |

---

## Phase 16: Ranking Enhancements

All Phase 16 features are designed to be backward-compatible with v2.0.0 data. Features are either disabled by default or use safe default values for existing data.

### Novelty Filtering

**Purpose:** Prevent storage of near-duplicate events.

**Backward Compatibility:** DISABLED by default. When disabled, all events are stored without similarity check (preserving v2.0.0 behavior).

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `novelty.enabled` | bool | `false` | **MUST be explicitly enabled.** When false, all events stored without check. |
| `novelty.threshold` | f32 | `0.82` | Similarity threshold (0.0-1.0). Events above this are considered duplicates. |
| `novelty.timeout_ms` | u64 | `50` | Maximum time for novelty check (ms). If exceeded, event stored anyway. |
| `novelty.min_text_length` | usize | `50` | Minimum event text length to check. Shorter events skip check. |

**Fail-Open Behavior:** Novelty check is best-effort. Events are ALWAYS stored if:
- Feature is disabled (default)
- Embedder is unavailable
- Vector index is unavailable or not ready
- Check times out
- Any error occurs

```toml
[novelty]
enabled = false  # Explicit opt-in required
threshold = 0.82
timeout_ms = 50
min_text_length = 50
```

### Salience Scoring (Planned)

**Purpose:** Score memories by importance at write time.

**Backward Compatibility:** Existing data without salience fields uses default value of `0.5` (neutral). No migration required.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `teleport.ranking.salience.enabled` | bool | `true` | Enable salience scoring for new nodes |
| `teleport.ranking.salience.length_density_weight` | f32 | `0.45` | Weight for text length density |
| `teleport.ranking.salience.kind_boost` | f32 | `0.20` | Boost for special memory kinds |
| `teleport.ranking.salience.pinned_boost` | f32 | `0.20` | Boost for pinned memories |

**Schema Changes:**
- `salience_score: f32` - Default `0.5` for existing data
- `memory_kind: MemoryKind` - Default `Observation` for existing data
- `is_pinned: bool` - Default `false` for existing data

### Usage Tracking (Planned)

**Purpose:** Track access patterns for ranking decay.

**Backward Compatibility:** DISABLED by default. New column family (`CF_USAGE_COUNTERS`) created lazily on first write. Reads return default values when CF absent.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `teleport.ranking.usage_decay.enabled` | bool | `false` | Enable usage-based ranking decay |
| `teleport.ranking.usage_decay.decay_factor` | f32 | `0.15` | Decay factor for frequently accessed items |
| `teleport.ranking.usage_decay.flush_interval_secs` | u64 | `60` | How often to flush pending writes to storage |
| `teleport.ranking.usage_decay.prefetch_interval_secs` | u64 | `5` | How often to process prefetch queue |
| `teleport.ranking.usage_decay.cache_size` | usize | `10000` | LRU cache size for hot doc IDs |

```toml
[teleport.ranking.usage_decay]
enabled = false  # Disabled until validated
decay_factor = 0.15
cache_size = 10000
```

---

## Phase 16-17: Index Lifecycle

### Vector Index Lifecycle (FR-08)

**Purpose:** Automatic pruning of old vectors from HNSW index.

**Backward Compatibility:** Enabled by default but respects retention rules that protect existing data. Month and Year vectors are NEVER pruned.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `teleport.vector.lifecycle.enabled` | bool | `true` | Enable automatic vector pruning |
| `teleport.vector.lifecycle.segment_retention_days` | u32 | `30` | Segment vector retention |
| `teleport.vector.lifecycle.grip_retention_days` | u32 | `30` | Grip vector retention |
| `teleport.vector.lifecycle.day_retention_days` | u32 | `365` | Day vector retention |
| `teleport.vector.lifecycle.week_retention_days` | u32 | `1825` | Week vector retention (5 years) |

**Protected Levels:** Month and Year vectors are NEVER pruned (not configurable).

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `teleport.vector.maintenance.prune_schedule` | string | `0 3 * * *` | Cron schedule for prune job (daily 3 AM) |
| `teleport.vector.maintenance.prune_batch_size` | u32 | `1000` | Batch size for prune operations |
| `teleport.vector.maintenance.optimize_after_prune` | bool | `true` | Run index optimization after pruning |

```toml
[teleport.vector.lifecycle]
enabled = true
segment_retention_days = 30
grip_retention_days = 30
day_retention_days = 365
week_retention_days = 1825
# month/year: NEVER pruned (protected)

[teleport.vector.maintenance]
prune_schedule = "0 3 * * *"
optimize_after_prune = true
```

### BM25 Index Lifecycle (FR-09)

**Purpose:** Automatic pruning of old documents from Tantivy BM25 index.

**Backward Compatibility:** DISABLED by default per PRD "append-only, no eviction" philosophy. Must be explicitly enabled to prune BM25 docs.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `teleport.bm25.lifecycle.enabled` | bool | `false` | **MUST be explicitly enabled.** Append-only by default. |
| `teleport.bm25.lifecycle.segment_retention_days` | u32 | `30` | Segment doc retention |
| `teleport.bm25.lifecycle.grip_retention_days` | u32 | `30` | Grip doc retention |
| `teleport.bm25.lifecycle.day_retention_days` | u32 | `180` | Day doc retention |
| `teleport.bm25.lifecycle.week_retention_days` | u32 | `1825` | Week doc retention (5 years) |

**Protected Levels:** Month and Year docs are NEVER pruned (not configurable).

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `teleport.bm25.maintenance.prune_schedule` | string | `0 3 * * *` | Cron schedule for prune job (daily 3 AM) |
| `teleport.bm25.maintenance.optimize_after_prune` | bool | `true` | Run index optimization after pruning |

```toml
[teleport.bm25.lifecycle]
enabled = false  # Explicit opt-in required
segment_retention_days = 30
grip_retention_days = 30
day_retention_days = 180
week_retention_days = 1825
# month/year: NEVER pruned (protected)

[teleport.bm25.maintenance]
prune_schedule = "0 3 * * *"
optimize_after_prune = true
```

---

## Topics Configuration

**Backward Compatibility:** Topics are DISABLED by default per TOPIC-07.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `topics.enabled` | bool | `false` | Master switch for topic functionality |

### Topic Extraction

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `topics.extraction.min_cluster_size` | usize | `3` | Minimum cluster size for HDBSCAN |
| `topics.extraction.similarity_threshold` | f32 | `0.75` | Minimum similarity for cluster membership |
| `topics.extraction.schedule` | string | `0 4 * * *` | Cron schedule for extraction job (4 AM daily) |
| `topics.extraction.batch_size` | usize | `500` | Maximum nodes to process per batch |

### Topic Labeling

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `topics.labeling.use_llm` | bool | `true` | Use LLM for topic labeling |
| `topics.labeling.fallback_to_keywords` | bool | `true` | Fall back to keyword extraction if LLM fails |
| `topics.labeling.max_label_length` | usize | `50` | Maximum label length |
| `topics.labeling.top_keywords` | usize | `5` | Number of top keywords to extract |

### Topic Importance

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `topics.importance.half_life_days` | u32 | `30` | Half-life in days for time decay |
| `topics.importance.recency_boost` | f64 | `2.0` | Boost multiplier for mentions within 7 days |

### Topic Relationships

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `topics.relationships.similar_threshold` | f32 | `0.8` | Minimum similarity for "similar" relationship |
| `topics.relationships.max_hierarchy_depth` | usize | `3` | Maximum hierarchy depth |
| `topics.relationships.enable_hierarchy` | bool | `true` | Enable parent/child detection |

### Topic Lifecycle

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `topics.lifecycle.prune_after_days` | u32 | `90` | Days of inactivity before pruning |
| `topics.lifecycle.prune_schedule` | string | `0 5 * * 0` | Cron schedule (5 AM Sunday) |
| `topics.lifecycle.auto_resurrect` | bool | `true` | Enable automatic resurrection on re-mention |

---

## TOC Segmentation

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `toc.segmentation.time_threshold_ms` | i64 | `1800000` | Max time gap before new segment (30 min) |
| `toc.segmentation.token_threshold` | usize | `4000` | Max tokens before new segment |
| `toc.segmentation.overlap_time_ms` | i64 | `300000` | Overlap time from previous segment (5 min) |
| `toc.segmentation.overlap_tokens` | usize | `500` | Overlap tokens from previous segment |
| `toc.segmentation.max_tool_result_chars` | usize | `1000` | Max text length to count for tool results |
| `toc.min_events_per_segment` | usize | `2` | Minimum events to create a segment |

---

## Scheduler Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `scheduler.default_timezone` | string | `UTC` | Default timezone for jobs (IANA format) |
| `scheduler.shutdown_timeout_secs` | u64 | `30` | Graceful shutdown timeout for jobs |

---

## Environment Variable Overrides

All configuration options can be overridden via environment variables with the `MEMORY_` prefix:

```bash
# Core settings
MEMORY_DB_PATH=/custom/path/db
MEMORY_GRPC_PORT=50052
MEMORY_LOG_LEVEL=debug

# Summarizer
MEMORY_SUMMARIZER_PROVIDER=anthropic
MEMORY_SUMMARIZER_MODEL=claude-3-haiku

# Phase 16 features (emergency disable)
MEMORY_NOVELTY_ENABLED=false

# Future Phase 16 features (when implemented)
MEMORY_TELEPORT_RANKING_ENABLED=false
MEMORY_TELEPORT_RANKING_SALIENCE_ENABLED=false
MEMORY_TELEPORT_RANKING_USAGE_DECAY_ENABLED=false
MEMORY_TELEPORT_RANKING_NOVELTY_ENABLED=false
```

---

## Backward Compatibility Summary

### v2.0.0 to v2.1.0 (Phase 16)

All Phase 16 features are designed for zero-friction upgrades:

| Feature | Default State | Existing Data Handling |
|---------|---------------|------------------------|
| Novelty Filtering | DISABLED | N/A (disabled) |
| Salience Scoring | Enabled but safe | Existing nodes use default `0.5` |
| Usage Tracking | DISABLED | N/A (disabled) |
| Vector Lifecycle | Enabled | Respects retention rules; protects month/year |
| BM25 Lifecycle | DISABLED | N/A (disabled) |

**No Data Migration Required:** All new features use serde defaults for backward compatibility with existing serialized data.

**Schema Evolution:**
- New fields added with `#[serde(default)]`
- Proto3 fields use implicit defaults (0, false, empty)
- New column families created lazily on first write

---

## Complete Example Configuration

```toml
# ~/.config/agent-memory/config.toml

# Core settings
db_path = "~/.local/share/agent-memory/db"
grpc_port = 50051
grpc_host = "0.0.0.0"
log_level = "info"
search_index_path = "~/.local/share/agent-memory/bm25-index"
vector_index_path = "~/.local/share/agent-memory/vector-index"

# Multi-agent mode
multi_agent_mode = "separate"

# Summarizer
[summarizer]
provider = "openai"
model = "gpt-4o-mini"

# Novelty filtering (Phase 16)
[novelty]
enabled = false  # Explicit opt-in required
threshold = 0.82
timeout_ms = 50
min_text_length = 50

# Topics (disabled by default)
[topics]
enabled = false

# TOC segmentation
[toc]
min_events_per_segment = 2

[toc.segmentation]
time_threshold_ms = 1800000
token_threshold = 4000
overlap_time_ms = 300000
overlap_tokens = 500

# Scheduler
[scheduler]
default_timezone = "UTC"
shutdown_timeout_secs = 30

# Vector lifecycle (FR-08)
[teleport.vector.lifecycle]
enabled = true
segment_retention_days = 30
grip_retention_days = 30
day_retention_days = 365
week_retention_days = 1825

[teleport.vector.maintenance]
prune_schedule = "0 3 * * *"
prune_batch_size = 1000
optimize_after_prune = true

# BM25 lifecycle (FR-09) - disabled by default
[teleport.bm25.lifecycle]
enabled = false
segment_retention_days = 30
grip_retention_days = 30
day_retention_days = 180
week_retention_days = 1825

[teleport.bm25.maintenance]
prune_schedule = "0 3 * * *"
optimize_after_prune = true
```

---

*Last Updated: 2026-02-06*
*Covers: v2.0.0 through Phase 16-17 (planned)*
