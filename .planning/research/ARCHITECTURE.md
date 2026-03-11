# Architecture: v2.6 Episodic Memory, Ranking, & Lifecycle Integration

**Project:** Agent Memory (Rust-based cognitive architecture for agents)
**Researched:** 2026-03-11
**Scope:** How episodic memory, salience/usage ranking, lifecycle automation, observability, and hybrid search integrate with existing v2.5 architecture
**Confidence:** HIGH (direct codebase analysis + existing handler/storage patterns)

---

## Executive Summary

Agent Memory v2.5 ships with a complete 6-layer retrieval stack (TOC, agentic search, BM25, vector, topic graph, ranking) backed by RocksDB and managed by a Tokio scheduler. v2.6 adds **four orthogonal capabilities** that integrate cleanly with existing architecture:

1. **Episodic Memory** — New CF_EPISODES + Episode proto + 4 RPCs for recording/retrieving task outcomes
2. **Ranking Quality** — Existing salience (v2.5) + new usage-tracking + StaleFilter decay + ranking payload composition
3. **Lifecycle Automation** — Extend scheduler with vector/BM25 pruning jobs (RPC stubs exist, logic needed)
4. **Observability** — Extend admin RPCs to expose dedup metrics, ranking stats, episode health

**Key insight:** All new features plug into existing patterns—handlers with Arc<Storage>, new column families, scheduler jobs. **No architectural rewrite.** Complexity is *additive, not structural*.

---

## System Architecture (v2.5 → v2.6)

### Current Component Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│                         memory-daemon                                │
├─────────────────────────────────────────────────────────────────────┤
│ gRPC Service Layer (MemoryServiceImpl)                               │
│  ├─ IngestEventHandler (+ DedupGate + StorageHandler)              │
│  ├─ QueryHandler (TOC navigation)                                  │
│  ├─ SearchHandler (SearchNode, SearchChildren)                     │
│  ├─ TeleportHandler (BM25 full-text)                               │
│  ├─ VectorHandler (Vector HNSW similarity)                         │
│  ├─ HybridHandler (BM25 + Vector fusion)                           │
│  ├─ TopicGraphHandler (HDBSCAN clustering)                         │
│  ├─ RetrievalHandler (Intent routing + fallbacks)                  │
│  ├─ AgentDiscoveryHandler (Multi-agent queries)                    │
│  ├─ SchedulerGrpcService (Job status + control)                    │
│  └─ [v2.6] EpisodeHandler [NEW]                                    │
├─────────────────────────────────────────────────────────────────────┤
│ Background Scheduler (tokio-cron-scheduler)                         │
│  ├─ outbox_processor (30s) — Queue → TOC updates                   │
│  ├─ index_sync (5m) — TOC → BM25 + Vector                         │
│  ├─ topic_refresh (1h) — Vector embeddings → HDBSCAN              │
│  ├─ rollup (daily 3am) — Day → Week → Month → Year                │
│  ├─ compaction (weekly Sun 4am) — RocksDB + Tantivy optimize      │
│  ├─ [v2.6] vector_prune (configurable) [NEW JOB]                   │
│  └─ [v2.6] bm25_prune (configurable) [NEW JOB]                     │
├─────────────────────────────────────────────────────────────────────┤
│ Storage Layer (RocksDB + Indexes)                                    │
│  ├─ RocksDB Column Families (9 existing + 2 new)                   │
│  │  ├─ CF_EVENTS (append-only conversation events)                │
│  │  ├─ CF_TOC_NODES (versioned TOC hierarchy)                     │
│  │  ├─ CF_TOC_LATEST (version pointers)                           │
│  │  ├─ CF_GRIPS (excerpt provenance)                              │
│  │  ├─ CF_OUTBOX (async queue)                                    │
│  │  ├─ CF_CHECKPOINTS (job crash recovery)                        │
│  │  ├─ CF_TOPICS (HDBSCAN clusters)                               │
│  │  ├─ CF_TOPIC_LINKS (topic-node associations)                   │
│  │  ├─ CF_TOPIC_RELS (inter-topic relationships)                  │
│  │  ├─ CF_USAGE_COUNTERS (access tracking for ranking)            │
│  │  ├─ [v2.6] CF_EPISODES [NEW]                                    │
│  │  └─ [v2.6] CF_EPISODE_METRICS [NEW]                             │
│  ├─ External Indexes                                               │
│  │  ├─ Tantivy BM25 (full-text search)                            │
│  │  └─ usearch HNSW (vector similarity)                           │
│  └─ [v2.6] Usage Metrics (extended CF_USAGE_COUNTERS)              │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Component Boundaries & Responsibilities

### EpisodeHandler (NEW)

**Location:** `crates/memory-service/src/episode.rs`

**Responsibility:** Manage episode lifecycle (start, record actions, complete, retrieve similar)

**Storage Access:**
- Write: `CF_EPISODES` (immutable append)
- Read: `CF_EPISODES`, vector index (similarity search)
- Query: `GetSimilarEpisodes` uses HNSW to find semantically related past episodes

**Data Structures:**
```rust
pub struct EpisodeHandler {
    storage: Arc<Storage>,
    vector_handler: Option<Arc<VectorTeleportHandler>>,  // For similarity search
    classifier: EpisodeValueClassifier,                   // Compute outcome score
}

pub struct Episode {
    pub episode_id: String,           // ULID
    pub start_time_ms: i64,
    pub end_time_ms: i64,
    pub actions: Vec<EpisodeAction>,
    pub outcome_description: String,
    pub value_score: f32,             // 0.0-1.0 (importance for retention)
    pub retention_policy: RetentionPolicy,
    pub context_grip_ids: Vec<String>, // Links to TOC grips for context
    pub agent_id: String,             // v2.1 multi-agent support
}
```

**RPCs Implemented:**
1. `StartEpisode(description, agent_id)` → Generate episode_id, allocate record
2. `RecordAction(episode_id, action)` → Append action (tool_use, decision, feedback)
3. `CompleteEpisode(episode_id, outcome, value_score)` → Finalize, store immutably
4. `GetSimilarEpisodes(query, limit)` → Find past episodes with similar goals/outcomes

**Pattern:** Handler receives Arc<Storage>, owns internal state (classifier), returns domain objects mapped to proto responses. Same pattern as RetrievalHandler, AgentDiscoveryHandler.

---

### RankingPayloadBuilder (ENHANCEMENT)

**Location:** `crates/memory-service/src/ranking.rs` [NEW FILE]

**Responsibility:** Compose ranking signals (salience, usage decay, stale penalty) into explainable breakdown

**Inputs:**
- TocNode.salience_score (computed at write time, v2.5) ✓
- UsageStats from CF_USAGE_COUNTERS (access_count, last_accessed_ms)
- StaleFilter output (time-decay penalty based on MemoryKind exemptions)

**Output:** RankingPayload
```rust
pub struct RankingPayload {
    pub salience_score: f32,        // 0.0-1.0+ (from node)
    pub usage_adjusted_score: f32,   // e^(-elapsed_days / half_life)
    pub stale_penalty: f32,          // 0.0-0.3 (time-decay, capped)
    pub final_score: f32,            // salience × usage × (1 - stale)
    pub explanation: String,         // "salience=0.8, usage=0.9, stale=0.05 → final=0.67"
}
```

**Formula:**
```
final_score = salience_score × usage_adjusted_score × (1.0 - stale_penalty)

where:
  usage_adjusted_score = e^(-elapsed_days / 30)    [30-day half-life]
  stale_penalty = StaleFilter.compute(...)         [0.0-0.3 cap from v2.5]
```

**Integration Point:** TeleportResult proto extended with optional RankingPayload field. Returned in TeleportSearch, VectorTeleport, HybridSearch RPCs. Used by RouteQuery for explainability (skill contracts).

---

### ObservabilityHandler (ENHANCEMENT)

**Location:** Extend existing handlers in `crates/memory-service/src/retrieval.rs` and new file

**Changes:**
- **GetRankingStatus** → Add breakdown: active_salience_kinds, usage_distribution (histogram), stale_decay_active_count
- **GetDedupStatus** → Add: buffer_memory_bytes, dedup_rate_24h_percent, cross_session_dedup_count
- **[NEW] GetEpisodeMetrics** → total_episodes, completion_rate, average_value_score, retention_distribution

**Data Flow:** Read aggregates from storage + CF_USAGE_COUNTERS + CF_EPISODES + checkpoints. No separate metrics store. Computed on-demand (single source of truth).

---

### Lifecycle Jobs (NEW)

**EpisodeRetentionJob** — `crates/memory-scheduler/src/jobs/episode_retention.rs` [NEW FILE]

```rust
pub struct EpisodeRetentionJob {
    storage: Arc<Storage>,
    config: EpisodeRetentionConfig,
}

pub struct EpisodeRetentionConfig {
    pub max_episode_age_days: u32,      // e.g., 180
    pub value_score_threshold: f32,     // e.g., 0.3 (delete if < 0.3)
    pub retention_policies: HashMap<RetentionPolicy, bool>,
}

impl EpisodeRetentionJob {
    pub async fn execute(&self) -> Result<JobResult, SchedulerError> {
        // 1. Scan CF_EPISODES with prefix "ep:"
        // 2. For each episode:
        //    age_days = (now_ms - start_time_ms) / (86400 * 1000)
        //    if age_days > max_episode_age_days AND value_score < threshold:
        //      mark_for_deletion()
        // 3. Write checkpoint: epmet:retention_sweep_{date}
        // 4. Return { deleted_count, retained_count }
    }
}
```

**Extends Scheduler:** Register with cron schedule (e.g., daily 2am), overlap policy (Skip), jitter (60s). Uses checkpoint pattern for crash recovery.

---

**VectorPruneJob** — `crates/memory-scheduler/src/jobs/vector_prune.rs` [EXTEND]

```rust
pub struct VectorPruneJobConfig {
    pub retention_days: u32,  // e.g., 90
    pub min_vectors_keep: u32, // safety limit
}

impl VectorPruneJob {
    pub async fn execute(&self) -> Result<JobResult, SchedulerError> {
        // 1. Read usearch index metadata (directory listing)
        // 2. Extract embedding IDs + timestamps (from metadata file)
        // 3. Mark for deletion if: timestamp < (now - retention_days)
        // 4. Rebuild HNSW without marked vectors (usearch API)
        // 5. Update CF_VECTOR_INDEX metadata pointer
        // 6. Checkpoint: vector_prune_{date}_removed={count}
    }
}
```

**Rationale:** Index rebuild is expensive. Copy-on-write pattern: new HNSW built in temp dir, pointer swapped atomically. Readers see no downtime.

---

## Data Flow: New Capabilities

### Episodic Recording Flow

```
Skill calls: rpc StartEpisode(StartEpisodeRequest)
  request = { description: "Debug JWT token expiration", agent_id: "claude-code" }

  ↓ MemoryServiceImpl routes to EpisodeHandler

EpisodeHandler.start_episode(request)
  ├─ Generate episode_id = ULID()
  ├─ record start_time_ms = now()
  ├─ key = format!("ep:{:013}:{}", start_time_ms, episode_id)
  ├─ episode = Episode { episode_id, start_time_ms, actions: [], ... }
  ├─ storage.put_cf(CF_EPISODES, key, serde_json::to_bytes(&episode))?
  └─ return StartEpisodeResponse { episode_id, start_time_ms }

  ↓ Skill now has episode_id, can record actions

Skill calls: rpc RecordAction(RecordActionRequest)
  request = { episode_id, action: EpisodeAction { action_type: TOOL_USE, ... } }

  ↓ EpisodeHandler.record_action(request)
  ├─ Fetch episode from CF_EPISODES
  ├─ if episode.end_time_ms > 0: return Err(EpisodeAlreadyCompleted)
  ├─ Append action to episodes.actions
  ├─ storage.put_cf(CF_EPISODES, same_key, updated_bytes)?  [UPDATE existing]
  └─ return RecordActionResponse { recorded: true }

  ↓ Repeat RecordAction for each tool_use, decision, etc.

Skill calls: rpc CompleteEpisode(CompleteEpisodeRequest)
  request = { episode_id, outcome_description: "Fixed JWT", value_score: 0.9, retention: KEEP_HIGH_VALUE }

  ↓ EpisodeHandler.complete_episode(request)
  ├─ Fetch episode from CF_EPISODES
  ├─ episode.end_time_ms = now()
  ├─ episode.outcome_description = "Fixed JWT"
  ├─ episode.value_score = 0.9
  ├─ episode.retention_policy = KEEP_HIGH_VALUE
  ├─ storage.put_cf(CF_EPISODES, key, bytes)?  [FINALIZE, immutable]
  ├─ Optional: Generate embedding of outcome_description via Candle
  ├─ Optional: Add to vector index for GetSimilarEpisodes
  └─ return CompleteEpisodeResponse { completed: true }
```

---

### Episodic Retrieval Flow

```
Skill calls: rpc GetSimilarEpisodes(GetSimilarEpisodesRequest)
  request = { query: "How do we handle JWT expiration?", limit: 10, agent_id: "claude-code" }

  ↓ EpisodeHandler.get_similar_episodes(request)
  ├─ Embed query using Candle (all-MiniLM-L6-v2)
  ├─ Search usearch HNSW for similar embeddings (up to limit results)
  ├─ Collect matching episode_ids from search results
  ├─ Scan CF_EPISODES for matching episodes
  ├─ Score by: embedding_similarity (0.0-1.0) + recency_boost + value_score
  ├─ Sort by final_score descending
  ├─ Build EpisodeSummary objects:
  │  {
  │    episode_id,
  │    start_time_ms,
  │    outcome_description: "Fixed JWT",
  │    value_score: 0.9,
  │    action_count: 7,
  │    context_grip_ids: [grip_1, grip_2]  ← Links to TOC for full context
  │  }
  └─ return GetSimilarEpisodesResponse { episodes: [summary_1, ...] }

  ↓ Skill inspects results, decides to expand context

Skill calls: rpc ExpandGrip(ExpandGripRequest)
  request = { grip_id: "grip_1" }  [from context_grip_ids]

  ↓ Existing ExpandGrip RPC (v2.5)
  ├─ Fetch Grip from CF_GRIPS
  ├─ Get event_ids from grip.event_id_start..event_id_end
  ├─ GetEvents returns raw events + context
  └─ Skill now has full transcript of that episode step-by-step
```

---

### Ranking Composition Flow

```
Skill calls: rpc RouteQuery(RouteQueryRequest)
  request = { query: "What did we learn about dedup?", mode: SEQUENTIAL }

  ↓ RetrievalHandler.route_query(request)
  ├─ ClassifyIntent(query) → Intent::Explore
  ├─ TierDetector() → CapabilityTier::Five (all layers available)
  ├─ FallbackChain::for_intent(...) → [AgenticTOC, BM25, Vector, Topics]
  │
  ├─ Execute each layer (example: BM25)
  │  └─ TeleportSearch(query) → [TocNode_1, TocNode_2, TocNode_3]
  │
  └─ For EACH result TocNode:
      ├─ RankingPayloadBuilder.build_for_node(node)
      │
      │  ├─ Read salience_score from node (pre-computed at write time, v2.5)
      │  │  salience_score = 0.8
      │  │
      │  ├─ Query CF_USAGE_COUNTERS for node.node_id
      │  │  access_count = 5
      │  │  last_accessed_ms = 1710078000000  (3 days ago)
      │  │
      │  ├─ Compute usage_adjusted_score:
      │  │  elapsed_days = (now - last_accessed_ms) / (86400 * 1000) = 3
      │  │  usage_adjusted = e^(-3 / 30) = e^(-0.1) = 0.905
      │  │
      │  ├─ Call StaleFilter.compute_penalty(node.timestamp_ms, node.memory_kind)
      │  │  timestamp_ms = 1709900000000 (11 days ago)
      │  │  memory_kind = Constraint (exempt from decay, so penalty = 0.0)
      │  │  stale_penalty = 0.0
      │  │
      │  ├─ Compute final_score:
      │  │  final_score = 0.8 × 0.905 × (1.0 - 0.0) = 0.724
      │  │
      │  ├─ Build explanation:
      │  │  "salience=0.8, usage_adjusted=0.905, stale_penalty=0.0 → final=0.724"
      │  │
      │  └─ Return RankingPayload {
      │       salience_score: 0.8,
      │       usage_adjusted_score: 0.905,
      │       stale_penalty: 0.0,
      │       final_score: 0.724,
      │       explanation: "..."
      │     }
      │
      └─ TeleportResult.ranking_payload = ABOVE

  ↓ Results sorted by final_score, returned with ranking_payload

Skill receives: [
  { node: TocNode_1, rank: 0.724, ranking_payload: { explanation: "..." } },
  { node: TocNode_2, rank: 0.618, ranking_payload: { explanation: "..." } },
  { node: TocNode_3, rank: 0.501, ranking_payload: { explanation: "..." } },
]

Skill inspects ranking_payload.explanation:
  → "Node 1 high because dedup Constraint (exempt from decay) + high salience + recent access"
```

---

### Lifecycle Sweep Flow

```
Scheduler fires: EpisodeRetentionJob (daily 2am)

  ↓ EpisodeRetentionJob.execute()
  ├─ Load config: max_age=180 days, threshold=0.3
  ├─ Load checkpoint from CF_EPISODE_METRICS (resume position)
  │
  ├─ Scan CF_EPISODES with prefix "ep:" starting from checkpoint
  │  For EACH episode:
  │    ├─ Parse key: ep:{ts:13}:{ulid}
  │    ├─ Deserialize Episode
  │    ├─ Compute age_days = (now_ms - start_time_ms) / (86400 * 1000)
  │    │
  │    └─ If age_days > 180 AND value_score < 0.3:
  │        └─ Delete (storage.delete_cf(CF_EPISODES, key)?)
  │           [NOTE: RocksDB doesn't delete in place; tombstone + compaction]
  │
  ├─ Write checkpoint: CF_EPISODE_METRICS[ "epmet:retention_sweep_2026_03_11" ]
  │  checkpoint = { last_episode_checked: 1234, episodes_deleted: 42, timestamp_ms: now }
  │
  └─ Return JobResult {
       status: Success,
       message: "Deleted 42 low-value episodes older than 180 days",
       metadata: { deleted_count: 42, retained_count: 1058 }
     }

  ↓ Scheduler records result in JobRegistry (for GetSchedulerStatus RPC)

Scheduler fires: VectorPruneJob (weekly Sunday 1am)

  ↓ VectorPruneJob.execute()
  ├─ Load config: retention_days=90
  ├─ Read usearch index metadata:
  │  ├─ Open index directory: {db_path}/usearch/
  │  ├─ Read metadata file containing embedding_id → timestamp mappings
  │  └─ Collect embeddings with timestamp < (now - 90 days)
  │
  ├─ Rebuild HNSW WITHOUT marked embeddings:
  │  ├─ Create temp directory: {db_path}/usearch.tmp/
  │  ├─ usearch::new_index(dimension=384) in temp dir
  │  ├─ For EACH embedding in original index:
  │  │    if NOT marked_for_deletion:
  │  │      new_index.add(embedding_id, vector)
  │  ├─ Write new index to temp dir
  │  └─ Atomic rename: {db_path}/usearch.tmp/ → {db_path}/usearch/
  │       [Safe: readers hold RwLock on directory pointer]
  │
  ├─ Update CF_VECTOR_INDEX metadata:
  │  metadata = { index_path: ..., last_prune_ts: now, vectors_count: new_count }
  │  storage.put_cf(CF_VECTOR_INDEX, "vec:meta", metadata)?
  │
  ├─ Write checkpoint: CF_EPISODE_METRICS[ "epmet:vector_prune_2026_03_11" ]
  │  checkpoint = { vectors_removed: 123, new_size_mb: 456, timestamp_ms: now }
  │
  └─ Return JobResult {
       status: Success,
       message: "Removed 123 vectors older than 90 days, new index size 456 MB",
       metadata: { vectors_removed: 123 }
     }
```

---

## Integration Points: Proto, Storage, Scheduler

### 1. Proto Additions (memory.proto)

**New enums:**
```protobuf
enum EpisodeStatus {
    STATUS_UNSPECIFIED = 0;
    STATUS_ACTIVE = 1;
    STATUS_COMPLETED = 2;
    STATUS_FAILED = 3;
}

enum ActionType {
    ACTION_UNSPECIFIED = 0;
    ACTION_TOOL_USE = 1;
    ACTION_DECISION = 2;
    ACTION_OUTCOME = 3;
    ACTION_FEEDBACK = 4;
}

enum RetentionPolicy {
    POLICY_UNSPECIFIED = 0;
    POLICY_KEEP_ALL = 1;
    POLICY_KEEP_HIGH_VALUE = 2;
    POLICY_TIME_DECAY = 3;
}
```

**New messages:**
```protobuf
message EpisodeAction {
    int64 timestamp_ms = 1;
    ActionType action_type = 2;
    string description = 3;
    map<string, string> metadata = 4;  // tool_name, input, output, etc.
}

message Episode {
    string episode_id = 1;
    int64 start_time_ms = 2;
    int64 end_time_ms = 3;
    repeated EpisodeAction actions = 4;
    string outcome_description = 5;
    float value_score = 6;
    RetentionPolicy retention_policy = 7;
    repeated string context_grip_ids = 8;  // Links to TOC grips
    string agent_id = 9;  // v2.1 multi-agent support
}

message StartEpisodeRequest {
    string description = 1;
    string agent_id = 2;
}

message StartEpisodeResponse {
    string episode_id = 1;
    int64 start_time_ms = 2;
}

message RecordActionRequest {
    string episode_id = 1;
    EpisodeAction action = 2;
}

message RecordActionResponse {
    bool recorded = 1;
    string error = 2;
}

message CompleteEpisodeRequest {
    string episode_id = 1;
    string outcome_description = 2;
    float value_score = 3;
    RetentionPolicy retention_policy = 4;
}

message CompleteEpisodeResponse {
    bool completed = 1;
    string error = 2;
}

message GetSimilarEpisodesRequest {
    string query = 1;
    int32 limit = 2;
    optional string agent_id = 3;
}

message EpisodeSummary {
    string episode_id = 1;
    int64 start_time_ms = 2;
    string outcome_description = 3;
    float value_score = 4;
    int32 action_count = 5;
}

message GetSimilarEpisodesResponse {
    repeated EpisodeSummary episodes = 1;
}
```

**Extended messages:**
```protobuf
message RankingPayload {
    float salience_score = 1;
    float usage_adjusted_score = 2;
    float stale_penalty = 3;
    float final_score = 4;
    string explanation = 5;
}

message TeleportResult {
    // ... existing fields ...
    optional RankingPayload ranking_payload = 20;  // Field number > 200 per v2.6 reservation
}

// Extend status RPCs
message GetRankingStatusResponse {
    // ... v2.5 fields ...
    int32 usage_tracked_count = 11;  // NEW
    int32 high_salience_kind_count = 12;  // NEW
    map<string, int32> memory_kind_distribution = 13;  // NEW
}

message GetDedupStatusResponse {
    // ... v2.5 fields ...
    int64 buffer_memory_bytes = 6;  // NEW
    int32 dedup_rate_24h_percent = 7;  // NEW
    int32 cross_session_dedup_count = 8;  // NEW
}

message GetEpisodeMetricsResponse {  // NEW RPC
    int32 total_episodes = 1;
    int32 completed_episodes = 2;
    int32 failed_episodes = 3;
    float average_value_score = 4;
    map<string, int32> retention_distribution = 5;
    int64 last_retention_sweep_ms = 6;
}
```

---

### 2. Storage: New Column Families

**In memory-storage/src/column_families.rs:**

```rust
pub const CF_EPISODES: &str = "episodes";
pub const CF_EPISODE_METRICS: &str = "episode_metrics";

pub const ALL_CF_NAMES: &[&str] = &[
    // ... existing 9 CFs ...
    CF_EPISODES,
    CF_EPISODE_METRICS,
];

fn episodes_options() -> Options {
    let mut opts = Options::default();
    opts.set_compression_type(rocksdb::DBCompressionType::Zstd);
    opts  // Standard options for immutable append
}

pub fn build_cf_descriptors() -> Vec<ColumnFamilyDescriptor> {
    vec![
        // ... existing descriptors ...
        ColumnFamilyDescriptor::new(CF_EPISODES, episodes_options()),
        ColumnFamilyDescriptor::new(CF_EPISODE_METRICS, Options::default()),
    ]
}
```

**Key formats:**
```rust
// Episode: ep:{start_ts:013}:{ulid}
// Example: ep:1710120000000:01ARZ3NDEKTSV4RRFFQ69G5FAV
pub fn episode_key(start_ts_ms: i64, episode_id: &str) -> String {
    format!("ep:{:013}:{}", start_ts_ms, episode_id)
}

// Episode metrics checkpoint: epmet:{checkpoint_type}
// Example: epmet:retention_sweep_2026_03_11
pub fn episode_metrics_key(checkpoint_type: &str) -> String {
    format!("epmet:{}", checkpoint_type)
}
```

**Usage Tracking Enhancement (CF_USAGE_COUNTERS):**

```rust
// Existing in memory-storage/src/usage.rs, extend:
pub struct UsageStats {
    pub access_count: u32,
    pub last_accessed_ms: i64,  // NEW
}

impl UsageTracker {
    pub fn record_access(&self, node_id: &str) -> Result<(), StorageError> {
        // Increment access_count in CF_USAGE_COUNTERS
        // Update last_accessed_ms to now
    }

    pub fn compute_access_decay(
        &self,
        access_count: u32,
        last_accessed_ms: i64,
        now_ms: i64,
    ) -> f32 {
        // exponential decay: e^(-lambda * time_elapsed)
        // lambda = ln(2) / 30 days half-life
        let elapsed_days = (now_ms - last_accessed_ms) as f32 / (86400.0 * 1000.0);
        (-0.0231 * elapsed_days).exp()  // 0.0231 ≈ ln(2)/30
    }
}
```

---

### 3. Scheduler Jobs

**Register in memory-daemon/src/main.rs:**

```rust
async fn register_jobs(scheduler: Arc<SchedulerService>, storage: Arc<Storage>) {
    // ... existing jobs ...

    // NEW: Episode retention (daily 2am)
    let episode_job = EpisodeRetentionJob::new(
        storage.clone(),
        EpisodeRetentionConfig {
            max_episode_age_days: 180,
            value_score_threshold: 0.3,
            retention_policies: Default::default(),
        },
    );
    scheduler.register_job(
        "episode_retention",
        "0 2 * * * *",
        None,
        OverlapPolicy::Skip,
        JitterConfig::new(60),
        || Box::pin(episode_job.execute()),
    ).await?;

    // NEW: Vector pruning (weekly Sunday 1am)
    let vector_prune_job = VectorPruneJob::new(
        storage.clone(),
        vector_handler.clone(),
        VectorPruneJobConfig {
            retention_days: 90,
            min_vectors_keep: 1000,
        },
    );
    scheduler.register_job(
        "vector_prune",
        "0 1 * * 0 *",
        None,
        OverlapPolicy::Skip,
        JitterConfig::new(120),
        || Box::pin(vector_prune_job.execute()),
    ).await?;

    // NOTE: BM25 pruning deferred to Phase 42 (requires SearchIndexer write access)
}
```

---

## Build Order & Phases

**v2.6 is 4 phases. Each phase has dependency constraints:**

### Phase 39: Episodic Memory Storage (Foundation)

**Deliverables:**
- Add CF_EPISODES, CF_EPISODE_METRICS to column families
- Define Episode proto + messages in memory.proto
- Add Episode struct to memory-types
- Storage::put_episode(), get_episode(), scan_episodes() helpers

**Dependencies:** v2.5 storage ✓
**Tests:** Unit tests for episode storage operations (CRUD)
**Blockers:** None

---

### Phase 40: Episodic Memory Handler (RPC Implementation)

**Deliverables:**
- EpisodeHandler struct (memory-service/src/episode.rs)
- Implement 4 RPCs: StartEpisode, RecordAction, CompleteEpisode, GetSimilarEpisodes
- Wire handler into MemoryServiceImpl
- Integrate vector search for GetSimilarEpisodes (similarity scoring)

**Dependencies:** Phase 39 ✓, vector index (v2.5) ✓
**Tests:** E2E tests: start → record → complete → retrieve similar
**Blockers:** None

---

### Phase 41: Ranking Payload & Observability (Signal Composition)

**Deliverables:**
- RankingPayloadBuilder (new file memory-service/src/ranking.rs)
- Merge salience + usage_decay + stale_penalty → final_score + explanation
- Extend GetRankingStatus response with new fields
- Extend GetDedupStatus response with new fields
- NEW: GetEpisodeMetrics RPC
- Add ranking_payload field to TeleportResult proto
- Wire ranking_payload into TeleportSearch, VectorTeleport, HybridSearch RPCs

**Dependencies:** Phase 39 (storage) ✓, Phase 40 (handler) ✓, v2.5 ranking ✓
**Tests:** Unit tests for ranking formula, E2E test for RouteQuery explainability
**Blockers:** None

---

### Phase 42: Lifecycle Automation Jobs (Scheduler)

**Deliverables:**
- EpisodeRetentionJob (memory-scheduler/src/jobs/episode_retention.rs)
- Extend VectorPruneJob (memory-scheduler/src/jobs/vector_prune.rs)
- Register both jobs in daemon startup
- Checkpoint-based crash recovery for both jobs

**Dependencies:** Phase 39 (storage) ✓, Phase 41 (observability) ✓, scheduler (v2.5) ✓
**Tests:** Unit tests for retention logic, E2E test for vector rebuild, integration test for checkpoint recovery
**Blockers:** None

---

## Patterns & Constraints

### Append-Only Immutability

Episodes are **immutable after CompleteEpisode**:

```rust
impl EpisodeHandler {
    pub async fn record_action(&self, ep_id: &str, action: Action) -> Result<()> {
        let episode = self.storage.get_episode(ep_id)?;
        if episode.end_time_ms > 0 {
            return Err(MemoryError::EpisodeAlreadyCompleted(ep_id.to_string()));
        }
        // Append-only: CF_EPISODES never updates, only adds new versions
        Ok(())
    }
}
```

**Rationale:** Maintains append-only invariant (STOR-01), enables crash recovery, simplifies concurrency.

---

### Handler Injection Pattern

All handlers use dependency injection via Arc<Storage>:

```rust
pub struct EpisodeHandler {
    storage: Arc<Storage>,  // Injected
    vector_handler: Option<Arc<VectorTeleportHandler>>,  // Optional
    classifier: EpisodeValueClassifier,  // Internal
}

impl EpisodeHandler {
    pub fn with_services(
        storage: Arc<Storage>,
        vector_handler: Option<Arc<VectorTeleportHandler>>,
    ) -> Self { ... }
}
```

**Rationale:** Separates concerns, testable with mock storage, follows existing RetrievalHandler pattern.

---

### Metrics On-Demand (Single Source of Truth)

Observability computes metrics by reading primary data, never maintains separate metrics store:

```rust
impl GetRankingStatus {
    pub async fn handle(&self, _req: Request<...>) -> Result<Response<...>> {
        let usage_count = self.storage.cf_usage_counters.len()?;  // Read current state
        let salience_kinds = self.storage.count_memory_kinds()?;  // Aggregate from nodes
        let stale_decay_active = self.storage.count_stale_nodes()?;

        Ok(Response::new(GetRankingStatusResponse {
            usage_tracked_count: usage_count,
            high_salience_kind_count: salience_kinds.len(),
            memory_kind_distribution: salience_kinds,
        }))
    }
}
```

**Rationale:** No sync issues, single source of truth, easy to test.

---

### Job Checkpoint Recovery

Jobs use checkpoints for crash recovery:

```rust
pub async fn execute(&self) -> Result<JobResult, SchedulerError> {
    let checkpoint = self.load_checkpoint()?;  // Resume from last position

    let mut idx = checkpoint.last_processed_idx;
    while idx < total_episodes {
        let episode = self.get_episode(idx)?;
        match self.should_delete(episode) {
            Ok(true) => self.mark_delete(episode),
            Ok(false) => { /* keep */ },
            Err(e) => {
                self.save_checkpoint(idx)?;  // Save progress and retry next run
                return Err(e);
            }
        }
        idx += 1;
    }

    self.save_checkpoint(total_episodes)?;  // Mark complete
    Ok(JobResult { ... })
}
```

**Rationale:** Scheduler retries on next cron tick; checkpoint resumes from last good position.

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Episode retention job deletes wrong records | Data loss | (1) Dry-run mode in config, (2) Conservative defaults (max_age=180d), (3) Checkpoint recovery |
| Vector index rebuild locks queries | Query latency spike | (1) RwLock on index pointer, (2) Copy-on-write (tmp → live), (3) Fallback to TOC |
| Ranking payload computation slows retrieval | Latency increase | (1) Lazy-compute (only for top-K), (2) Cache optional, (3) Metrics show impact |
| GetSimilarEpisodes on large datasets | O(n) scan | (1) usearch HNSW is O(log n), (2) Limit top-10 by default, (3) Time filter (90d) |
| Episode disabled → RPCs return Unimplemented | Skill failure | (1) Skill checks capabilities, (2) Graceful fallback to TOC, (3) Clear docs |

---

## Configuration

**New config entries (config.toml):**

```toml
[episode]
enabled = true
max_episode_age_days = 180
value_score_retention_threshold = 0.3
vector_search_limit = 10

[lifecycle]
vector_prune_enabled = true
vector_prune_retention_days = 90
bm25_prune_enabled = false  # Deferred to Phase 42b

[ranking]
# Note: Salience, usage, stale already configured in v2.5
salience_weight = 0.5
usage_weight = 0.3
stale_weight = 0.2
```

---

## Success Criteria

**v2.6 is complete when:**

1. **Episodic Memory:**
   - Episode start → record actions → complete → retrieve similar ✓
   - GetSimilarEpisodes returns top-10 semantically matched past episodes ✓
   - Episode context retrievable via ExpandGrip on linked grips ✓

2. **Ranking Quality:**
   - RankingPayload = salience × usage_decay × (1 - stale_penalty) ✓
   - Explanation human-readable ✓
   - TeleportResult includes ranking_payload ✓

3. **Lifecycle Automation:**
   - VectorPruneJob removes vectors > 90 days old ✓
   - EpisodeRetentionJob deletes episodes (age > 180d AND value < 0.3) ✓
   - Jobs report metrics to observability ✓

4. **Observability:**
   - GetRankingStatus includes usage_tracked_count, high_salience_kind_count ✓
   - GetDedupStatus includes buffer_memory_bytes, dedup_rate_24h_percent ✓
   - GetEpisodeMetrics returns completion_rate, value_distribution ✓

5. **No Regressions:**
   - All v2.5 E2E tests pass ✓
   - Dedup gate unaffected ✓
   - Features optional (feature-gated if needed) ✓

---

## Summary

v2.6 integrates **four orthogonal capabilities** into v2.5 via:

1. **New handlers** (EpisodeHandler) using existing patterns (Arc<Storage> injection)
2. **New column families** (CF_EPISODES, CF_EPISODE_METRICS) following storage conventions
3. **Extended RPCs** (4 episode RPCs, enhanced status RPCs) with new protos
4. **New scheduler jobs** (episode retention, vector pruning) using checkpoint recovery
5. **Signal composition** (ranking payload) merging v2.5 rankings into explainable payload

**No architectural rewrite.** All additions are *additive, not structural.* Build order respects dependencies. Patterns align with existing codebase (handler injection, checkpoint recovery, immutable storage, single-source-of-truth metrics).
