# RFC: Memory Ranking Enhancements

**Status:** Proposal (Tier 1: Ranking/Lifecycle; Tier 2/3 deferred)
**Author:** Claude
**Date:** 2026-02-04
**Phase:** 16 (proposed)

## Summary

Propose incremental enhancements to agent-memory's retrieval and storage policies. The current stack provides excellent navigation and search, but lacks mechanisms for self-improving agent behavior. **Tier 1 (Phase 16) focuses on ranking/lifecycle (salience, usage, novelty, index pruning). Episodic memory and consolidation are explicitly deferred to a future phase (Tier 2/3).**

## Motivation

### Current State (v2.0.0)

| Layer | Component | Status |
|-------|-----------|--------|
| 0 | Raw Events (RocksDB) | Complete |
| 1 | TOC Hierarchy | Complete |
| 2 | Agentic TOC Search | Complete |
| 3 | BM25 Keyword Search | Complete |
| 4 | Vector Semantic Search | Complete |
| 5 | Topic Graph (with time-decay) | Complete |

**What works well:**
- Hierarchical TOC as "always works" backbone
- BM25 + vector + topics layers for accelerated search
- Append-only storage with rollups
- Progressive disclosure retrieval
- Index rebuilds are first-class
- Time-decayed importance scoring (Topics only, 30-day half-life)
- Topic pruning and lifecycle management

### Implementation vs. PRD Gaps

**Vector Index (Phase 12):**
- PRD defines retention days per level (segments/grips 30d, day 365d, week 5y, month forever)
- Code has `VectorIndexPipeline::prune(age_days)` API in `crates/memory-vector/src/pipeline.rs`
- **GAP:** No CLI/admin command or scheduled job wired up for automated pruning

**BM25 Index (Phase 11):**
- PRD explicitly says "Append-only, no eviction" - growth bounded via summarization
- Warm/cold layers are about indexing different granularities, not pruning
- **GAP:** Currently indexes all levels indefinitely; no "stop-indexing-low-level" policy
- To achieve "eventually only month-level indexed," need new lifecycle policy

**Monthly Summaries Only (Aspirational):**
- TOC rollups create day/week/month nodes that get indexed
- Vector lifecycle spec retains coarse levels long-term (month ~forever)
- **GAP:** BM25 has no retention/prune policy; keeps all indexed docs

### Identified Gaps

| Gap | Current State | Impact |
|-----|---------------|--------|
| Episodic memory | Not present | Can't learn from past task outcomes |
| Salience scoring | Topics only (time-decay) | All memories treated equally |
| Novelty gating | Not present | Redundant memories stored |
| Usage-aware retrieval | Not present | Same items retrieved repeatedly |
| Policy layer | Pruning is index/rollup focused | No salience+usage governance |
| Outcome tracking | Not present | No reinforcement of "what worked" |

### Do We Need It?

**If goal is self-improving agent:**
- YES - episodic layer + salience/novelty/usage policy enables learning from past patterns

**If goal is fast recall/navigation:**
- NO - current stack is sufficient

**Recommendation:** Incremental adoption via spike, not full commitment.

## Proposal

### Tier 1: Core Ranking Policy (Low Risk)

Extend existing time-decay pattern from Topics to all memory types.

#### 1.1 Salience Scoring

Add salience calculation to TOC nodes and Grips at write time:

```rust
pub fn calculate_salience(text: &str, kind: MemoryKind, is_pinned: bool) -> f32 {
    let length_density = (text.len() as f32 / 500.0).min(1.0) * 0.45;
    let kind_boost = match kind {
        MemoryKind::Preference | MemoryKind::Procedure |
        MemoryKind::Constraint | MemoryKind::Definition => 0.20,
        MemoryKind::Observation => 0.0,
    };
    let pinned_boost = if is_pinned { 0.20 } else { 0.0 };

    length_density + kind_boost + pinned_boost
}
```

**Schema changes:** Add `salience_score: f32` and `is_pinned: bool` to TocNode and Grip.

**Complexity:** Low (2-3 days)

#### 1.2 Usage-Based Decay

Track access count and apply penalty in retrieval ranking:

```rust
pub fn usage_penalty(access_count: u32) -> f32 {
    1.0 / (1.0 + 0.15 * access_count as f32)
}

// Integrated ranking
fn rank_result(similarity: f32, salience: f32, access_count: u32) -> f32 {
    let salience_factor = 0.55 + 0.45 * salience;
    similarity * salience_factor * usage_penalty(access_count)
}
```

**Schema changes:** Add `access_count: u32` and `last_accessed: Option<DateTime<Utc>>` to TocNode, Grip, Topic.

**Complexity:** Low (2-3 days)

#### 1.3 Novelty Threshold

Check similarity before storing new events:

```rust
async fn check_novelty(event: &Event, threshold: f32) -> bool {
    let embedding = embedder.embed(&event.text).await?;
    let similar = vector_index.search(&embedding, 5, threshold).await?;
    similar.first().map(|m| m.score <= threshold).unwrap_or(true)
}
```

**Configuration:**
```toml
[novelty]
enabled = true
threshold = 0.82
```

**Complexity:** Low (1-2 days)

### Tier 2: Episodic Memory (Medium Risk, Deferred)

New crate for task outcome tracking. Enables learning from past successes/failures.

#### 2.1 Episode Schema

```rust
pub struct Episode {
    pub episode_id: String,
    pub task: String,
    pub plan: Vec<String>,
    pub actions: Vec<Action>,
    pub outcome_score: f32,         // 0.0 - 1.0
    pub lessons_learned: Vec<String>,
    pub failure_modes: Vec<String>,
    pub embedding: Vec<f32>,
    pub created_at: DateTime<Utc>,
}

pub struct Action {
    pub action_type: String,
    pub input: String,
    pub result: ActionResult,
    pub timestamp: DateTime<Utc>,
}
```

#### 2.2 Value-Based Retention

Episodes near 0.65 outcome score are most valuable (not too easy, not too hard):

```rust
pub fn calculate_value(outcome_score: f32) -> f32 {
    let target = 0.65;
    let distance = (outcome_score - target).abs();
    (1.0 - distance).max(0.0)
}

pub fn should_retain(episode: &Episode) -> bool {
    episode.value_score >= 0.18
}
```

#### 2.3 New RPCs

```protobuf
rpc StartEpisode(StartEpisodeRequest) returns (StartEpisodeResponse);
rpc RecordAction(RecordActionRequest) returns (RecordActionResponse);
rpc CompleteEpisode(CompleteEpisodeRequest) returns (CompleteEpisodeResponse);
rpc GetSimilarEpisodes(GetSimilarEpisodesRequest) returns (GetSimilarEpisodesResponse);
```

**New column family:** `CF_EPISODES`

**Complexity:** Medium (1-2 weeks)

### Tier 3: Consolidation Hook (Higher Risk)

Extract durable knowledge (preferences, constraints, procedures) from recent events.

#### 3.1 Extraction Patterns

| Pattern | Keywords | Kind |
|---------|----------|------|
| Preferences | "prefer", "like", "avoid", "hate" | Preference |
| Constraints | "must", "should", "need to", "require" | Constraint |
| Procedures | "step 1", "first", "then", "finally" | Procedure |
| Definitions | "is defined as", "means", "refers to" | Definition |

#### 3.2 Scheduler Job

```rust
// Runs daily, extracts knowledge atoms from recent events
pub struct ConsolidationJob {
    extractor: KnowledgeExtractor,
    storage: ConsolidationStorage,
}
```

**New column family:** `CF_CONSOLIDATED`

**Complexity:** High (2-3 weeks, requires NLP or LLM calls)

### Tier 1.5: Index Lifecycle Automation (Fill PRD Gaps)

Wire up existing APIs and add missing lifecycle controls.

#### 1.5.1 Vector Index Pruning Automation

The PRD specifies lifecycle, the API exists, just needs wiring:

```rust
// Already exists in crates/memory-vector/src/pipeline.rs
pub async fn prune(&self, age_days: u32) -> Result<u32>;
```

**Changes needed:**
1. Add scheduler job to call `prune()` daily (3 AM)
2. Add CLI command: `memory-daemon admin prune-vectors --age-days 30`
3. Add gRPC RPC: `PruneVectorIndex(age_days)` for admin use
4. Read retention config from `[teleport.vector.lifecycle]`

**Complexity:** Low (1-2 days)

#### 1.5.2 BM25 Index Lifecycle Policy

The PRD says "no eviction" but aspirationally wants "eventually only month-level indexed."

**Option A: Stop indexing fine-grain after rollup**
- After day rollup completes, mark segments as "coarse only"
- Indexing pipeline skips items flagged "coarse only"
- Requires new field on TocNode: `index_level: IndexLevel`

**Option B: Periodic rebuild with level filter**
- Daily rebuild job re-indexes only items above threshold age
- Segments older than 30 days excluded from rebuild
- Simpler but requires full rebuild

**Recommended:** Option B (simpler, aligns with "rebuildable indexes" philosophy)

**Changes needed:**
1. Add `rebuild-index --min-level day --max-age-days 30` flag
2. Add scheduler job to rebuild BM25 weekly with level filter
3. Document that fine-grain BM25 results age out

**Complexity:** Medium (3-5 days)

#### 1.5.3 Unified Lifecycle Configuration

```toml
[lifecycle]
enabled = true

[lifecycle.vector]
# Existing PRD config - just needs automation
segment_retention_days = 30
grip_retention_days = 30
day_retention_days = 365
prune_schedule = "0 3 * * *"

[lifecycle.bm25]
# NEW: Controls what gets indexed/kept
segment_retention_days = 30
grip_retention_days = 30
rebuild_schedule = "0 4 * * 0"  # Weekly Sunday 4 AM
min_level_after_rollup = "day"  # Only keep day+ in BM25 after rollup
```

## Implementation Options

### Option A: Tier 1 Only

Add salience + usage decay + novelty to existing retrieval. Minimal risk, immediate value.

**Effort:** ~1 week
**Risk:** Low
**Value:** Medium (better ranking without structural changes)

### Option A.5: Tier 1 + Lifecycle Automation (Recommended)

Add core ranking improvements PLUS fill the PRD implementation gaps.

**Effort:** ~2 weeks
**Risk:** Low-Medium
**Value:** High (ranking improvements + realizes PRD intent for index lifecycle)

Includes:
- Salience scoring (Tier 1)
- Usage-based decay (Tier 1)
- Novelty threshold (Tier 1)
- Vector pruning scheduler job (Tier 1.5)
- Vector prune CLI command (Tier 1.5)
- BM25 rebuild with level filter (Tier 1.5)

### Option B: Tier 1 + Tier 2

Add episodic memory for task outcome tracking.

**Effort:** ~3 weeks
**Risk:** Medium
**Value:** High (enables learning from past tasks)

### Option C: Full Implementation

All three tiers including consolidation.

**Effort:** ~6 weeks
**Risk:** High (consolidation requires NLP/LLM integration)
**Value:** High (full self-improving agent capability)

## Recommendation

**Start with Option A.5 (Tier 1 + Lifecycle Automation)** as Phase 16.

Rationale:
1. Builds on existing time-decay pattern in Topics
2. Low risk, can be feature-flagged
3. Provides immediate retrieval quality improvement
4. Fills PRD implementation gaps (vector pruning, BM25 lifecycle)
5. Realizes the "eventually only month-level indexed" vision from PRDs
6. Doesn't require new crates - uses existing `VectorIndexPipeline::prune()` API
7. Can evaluate value before committing to Tier 2/3

If Tier 1 + Lifecycle proves valuable, propose Tier 2 (Episodic) as Phase 17. Until then, episodic features are out-of-scope for Phase 16.

## Success Criteria

### Tier 1

1. Salience scoring applied to new TOC nodes and Grips
2. Usage tracking increments on retrieval
3. Hybrid search ranking incorporates salience and usage factors
4. Novelty filtering rejects >82% similar events (configurable)
5. All features behind config flags
6. Backward compatible with v2.0.0 data

### Tier 1.5 (Lifecycle Automation)

1. Vector pruning scheduler job runs daily (configurable)
2. `memory-daemon admin prune-vectors` CLI command works
3. Old segment/grip vectors removed from HNSW per retention config
4. BM25 rebuild with `--min-level` flag excludes fine-grained docs
5. PRDs updated to reflect actual implementation behavior

### Tier 2 (if pursued)

1. Episodes can be created, updated, and completed via gRPC
2. Similar episode search returns relevant past task patterns
3. Failure mode queries help avoid repeated mistakes
4. Value-based retention keeps useful episodes, prunes trivial ones

## Configuration

```toml
# Tier 1 - Core Ranking Policy
[salience]
enabled = true
length_density_weight = 0.45
kind_boost = 0.20
pinned_boost = 0.20

[usage_decay]
enabled = true
decay_factor = 0.15

[novelty]
enabled = true
threshold = 0.82

# Tier 2 - Episodic Memory (if pursued)
[episodic]
enabled = false  # Off by default
value_threshold = 0.18
midpoint_target = 0.65
max_episodes = 1000
```

## Open Questions

1. **Should salience scoring use entity density?** Original proposal included numeric/entity density (+0.20), but this requires NER. May add complexity without proportional value.

2. **How to detect MemoryKind?** Keyword matching is simple but imprecise. LLM classification is accurate but adds latency/cost.

3. **Should novelty check be async?** Blocking event ingestion for similarity check adds latency. Could batch check periodically instead.

4. **Who records episodes?** Agent framework integration required. May need hooks or explicit API calls.

## References

- [FEATURES.md](.planning/research/FEATURES.md) - "Memory decay/importance scoring" identified as future work
- [Phase 14 - Topic Graph](docs/plans/topic-graph-memory.md) - Existing time-decay implementation
- [importance.rs](crates/memory-topics/src/importance.rs) - 30-day half-life scorer
- [lifecycle.rs](crates/memory-topics/src/lifecycle.rs) - Topic pruning implementation
