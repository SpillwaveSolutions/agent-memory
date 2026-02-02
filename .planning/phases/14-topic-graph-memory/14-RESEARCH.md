# Phase 14: Topic Graph Memory - Research

**Researched:** 2026-02-01
**Domain:** Semantic topic extraction, embedding clustering, time-decayed importance scoring, gRPC service patterns
**Confidence:** HIGH

## Summary

Phase 14 adds a semantic topic layer that extracts recurring themes from TOC summaries using embedding clustering (HDBSCAN or DBSCAN), tracks topic importance with time-decayed scoring, and enables conceptual navigation through topic relationships. This is Layer 5 (Conceptual Enrichment) of the cognitive architecture.

The research confirms several Rust libraries are production-ready for this work. For clustering, the `hdbscan` crate (v0.12.0) provides pure Rust HDBSCAN with automatic cluster count detection. For embeddings, Phase 12's infrastructure will provide the embedding model, but the `fastembed` crate (v5.8.1) offers local embedding generation without API dependencies using ONNX Runtime. For vector similarity, standard cosine similarity using `f32` vectors is sufficient without external dependencies.

**Primary recommendation:** Create a new `memory-topics` crate that depends on Phase 12's embedding infrastructure. Use `hdbscan` for clustering, implement cosine similarity in pure Rust (trivial function), and store topics in three new RocksDB column families: `CF_TOPICS`, `CF_TOPIC_LINKS`, `CF_TOPIC_RELS`. The feature should be fully optional via configuration (`topics.enabled = false` by default) with a `GetTopicGraphStatus` RPC for agent skill discovery.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| hdbscan | 0.12.0 | HDBSCAN clustering | Pure Rust, MIT/Apache-2.0, no external deps, automatic cluster count |
| (Phase 12) | - | Embedding generation | Reuses existing HNSW/embedding infrastructure |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| (workspace) tokio | 1.43 | Async runtime | Already in workspace, for scheduled jobs |
| (workspace) chrono | 0.4 | Timestamp handling | Time decay calculations |
| (workspace) serde | 1.0 | Serialization | Topic struct storage |
| (workspace) ulid | 1.1 | ID generation | Topic and link IDs |
| tokio-cron-scheduler | 0.15 | Job scheduling | Already in Phase 10, for extraction jobs |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| hdbscan | linfa-clustering (DBSCAN) | DBSCAN requires epsilon param, HDBSCAN auto-detects density |
| hdbscan | petal-clustering | Less active, fewer downloads, similar API |
| hdbscan | Custom agglomerative | Hand-rolling is complex for variable densities |
| Pure Rust cosine | ndarray | Adds dependency for simple vector operation |
| LLM labeling | Pure keyword extraction | LLM labels are more meaningful, keyword is good fallback |

**Installation:**
```toml
# Add to workspace Cargo.toml
[workspace.dependencies]
hdbscan = "0.12"

# In memory-topics crate
[dependencies]
hdbscan = { workspace = true }
memory-types = { workspace = true }
memory-storage = { workspace = true }
tokio = { workspace = true }
chrono = { workspace = true }
serde = { workspace = true }
ulid = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }

# Optional: for local embedding if Phase 12 not ready
fastembed = { version = "5.8", optional = true }
```

## Architecture Patterns

### Recommended Project Structure
```
crates/
  memory-topics/           # NEW crate for topic functionality
    src/
      lib.rs               # Public API exports
      config.rs            # TopicsConfig (enabled, extraction, labeling, etc.)
      error.rs             # TopicsError enum
      storage.rs           # CF_TOPICS, CF_TOPIC_LINKS, CF_TOPIC_RELS operations
      extraction.rs        # HDBSCAN clustering, topic creation
      labeling.rs          # LLM labeling with keyword fallback
      importance.rs        # Time-decayed importance scoring
      relationships.rs     # Similar/parent/child discovery
      lifecycle.rs         # Pruning and resurrection
    Cargo.toml

  memory-service/
    src/
      topic_service.rs     # NEW: gRPC handlers for topic RPCs
      lib.rs               # Wire topic service
```

### Pattern 1: Optional Feature with Status RPC
**What:** Make topics fully optional, with a status RPC for agent discovery
**When to use:** Always - topics are opt-in enhancement
**Example:**
```rust
// Source: Existing scheduler pattern in memory-service
impl MemoryServiceImpl {
    pub async fn get_topic_graph_status(
        &self,
        _request: Request<GetTopicGraphStatusRequest>,
    ) -> Result<Response<GetTopicGraphStatusResponse>, Status> {
        let (enabled, healthy, stats) = match &self.topic_storage {
            Some(storage) => {
                let stats = storage.get_stats().await.unwrap_or_default();
                (true, stats.topic_count > 0, stats)
            }
            None => (false, false, TopicStats::default()),
        };

        Ok(Response::new(GetTopicGraphStatusResponse {
            enabled,
            healthy,
            topic_count: stats.topic_count as i64,
            link_count: stats.link_count as i64,
            last_extraction_ms: stats.last_extraction_ms,
            message: if enabled {
                format!("{} topics indexed", stats.topic_count)
            } else {
                "Topic graph is disabled".to_string()
            },
            half_life_days: stats.half_life_days as i32,
            similarity_threshold: stats.similarity_threshold,
        }))
    }
}
```

### Pattern 2: HDBSCAN Clustering with Pure Rust
**What:** Use hdbscan crate for density-based clustering
**When to use:** For extracting topic clusters from embeddings
**Example:**
```rust
// Source: https://docs.rs/hdbscan/latest/hdbscan/
use hdbscan::{Hdbscan, HdbscanHyperParams};

pub struct TopicExtractor {
    min_cluster_size: usize,
}

impl TopicExtractor {
    pub fn cluster_embeddings(&self, embeddings: &[Vec<f32>]) -> Result<Vec<i32>, TopicsError> {
        // Convert to 2D array format expected by hdbscan
        let data: Vec<Vec<f64>> = embeddings
            .iter()
            .map(|e| e.iter().map(|&x| x as f64).collect())
            .collect();

        // Create clusterer with custom params
        let params = HdbscanHyperParams::builder()
            .min_cluster_size(self.min_cluster_size)
            .build();

        let clusterer = Hdbscan::new(&data, params);
        let labels = clusterer.cluster()?;

        // labels: -1 = noise, 0..N = cluster assignment
        Ok(labels)
    }
}
```

### Pattern 3: Pure Rust Cosine Similarity
**What:** Implement cosine similarity without external dependencies
**When to use:** For topic similarity and query matching
**Example:**
```rust
// Source: Standard vector math
/// Calculate cosine similarity between two vectors.
/// Returns value in [-1.0, 1.0] where 1.0 = identical direction.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same dimension");

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

/// Calculate centroid of multiple embeddings (normalized).
pub fn calculate_centroid(embeddings: &[&[f32]]) -> Vec<f32> {
    if embeddings.is_empty() {
        return Vec::new();
    }

    let dim = embeddings[0].len();
    let n = embeddings.len() as f32;
    let mut centroid = vec![0.0f32; dim];

    for embedding in embeddings {
        for (i, &val) in embedding.iter().enumerate() {
            centroid[i] += val;
        }
    }

    for val in centroid.iter_mut() {
        *val /= n;
    }

    // Normalize
    let norm: f32 = centroid.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in centroid.iter_mut() {
            *val /= norm;
        }
    }

    centroid
}
```

### Pattern 4: Time-Decayed Importance Scoring
**What:** Calculate importance based on recency and frequency
**When to use:** For ranking topics by relevance
**Example:**
```rust
// Source: PRD specification
use chrono::{DateTime, Utc};

pub struct ImportanceConfig {
    /// Half-life in days (topic importance halves every N days)
    pub half_life_days: u32,
    /// Boost multiplier for mentions within 7 days
    pub recency_boost: f64,
}

impl ImportanceConfig {
    pub fn default() -> Self {
        Self {
            half_life_days: 30,
            recency_boost: 2.0,
        }
    }
}

/// Calculate time-decayed importance score.
/// Formula: Σ(weight × 0.5^(days_since / half_life))
pub fn calculate_importance(
    mention_timestamps: &[DateTime<Utc>],
    now: DateTime<Utc>,
    config: &ImportanceConfig,
) -> f64 {
    let half_life_secs = config.half_life_days as f64 * 24.0 * 3600.0;

    mention_timestamps.iter().map(|ts| {
        let age_secs = (now - *ts).num_seconds() as f64;
        let days_ago = age_secs / (24.0 * 3600.0);

        // Base weight with recency boost
        let weight = if days_ago <= 7.0 {
            config.recency_boost
        } else {
            1.0
        };

        // Apply exponential decay
        let decay = 0.5_f64.powf(age_secs / half_life_secs);
        weight * decay
    }).sum()
}
```

### Pattern 5: RocksDB Column Family Key Design
**What:** Design efficient key formats for topic storage
**When to use:** For all topic-related storage operations
**Example:**
```rust
// Source: Existing column_families.rs pattern

/// Column family for topic records
pub const CF_TOPICS: &str = "topics";

/// Column family for topic-node links
pub const CF_TOPIC_LINKS: &str = "topic_links";

/// Column family for topic relationships
pub const CF_TOPIC_RELS: &str = "topic_rels";

/// Key format for topics: topic:{topic_id}
/// Example: "topic:01HRQ7D5KQJMX1234567890ABC"
pub fn topic_key(topic_id: &str) -> String {
    format!("topic:{}", topic_id)
}

/// Key format for topic links: link:{topic_id}:{node_id}
/// Allows prefix scan of all links for a topic
pub fn topic_link_key(topic_id: &str, node_id: &str) -> String {
    format!("link:{}:{}", topic_id, node_id)
}

/// Secondary index key: node:{node_id}:{topic_id}
/// Allows reverse lookup: "what topics does this node belong to?"
pub fn node_topic_key(node_id: &str, topic_id: &str) -> String {
    format!("node:{}:{}", node_id, topic_id)
}

/// Key format for relationships: rel:{from_topic_id}:{rel_type}:{to_topic_id}
/// rel_type: "sim" (similar), "par" (parent), "chi" (child)
pub fn relationship_key(from_id: &str, rel_type: &str, to_id: &str) -> String {
    format!("rel:{}:{}:{}", from_id, rel_type, to_id)
}
```

### Pattern 6: Scheduled Extraction Job
**What:** Integrate topic extraction with Phase 10 scheduler
**When to use:** For periodic batch topic extraction
**Example:**
```rust
// Source: Existing scheduler job pattern (Phase 10)
use memory_scheduler::{SchedulerService, OverlapPolicy, JitterConfig};

pub async fn create_topic_extraction_job(
    scheduler: &SchedulerService,
    topic_storage: Arc<TopicStorage>,
    toc_storage: Arc<Storage>,
    config: &TopicsConfig,
) -> Result<(), SchedulerError> {
    if !config.enabled {
        tracing::info!("Topic extraction disabled, skipping job registration");
        return Ok(());
    }

    scheduler.register_job(
        "topic-extraction",
        &config.extraction.schedule,  // e.g., "0 4 * * *" (4 AM daily)
        None,  // Use default timezone
        OverlapPolicy::Skip,  // Don't run concurrent extractions
        JitterConfig::new(300),  // Up to 5 minutes jitter
        move || {
            let storage = topic_storage.clone();
            let toc = toc_storage.clone();
            async move {
                extract_topics(&storage, &toc).await
            }
        },
    ).await
}
```

### Anti-Patterns to Avoid
- **Running extraction on every ingestion:** Expensive; use scheduled batch instead
- **Storing full embeddings in CF_TOPICS if already in Phase 12:** Double storage; store only topic centroid
- **Using HDBSCAN with min_cluster_size=1:** Creates too many singleton "topics"
- **Committing after each topic write:** Batch writes for efficiency
- **Blocking gRPC handlers with clustering:** Use tokio::spawn_blocking

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Density-based clustering | Custom clustering | hdbscan crate | HDBSCAN is complex with MST extraction and persistence |
| Cron scheduling | Custom scheduler | tokio-cron-scheduler (Phase 10) | Already integrated, handles DST, jitter, overlap |
| Embedding generation | Custom model loading | Phase 12 infrastructure or fastembed | ONNX runtime, tokenizer, model handling |
| gRPC service scaffolding | Manual proto impl | tonic with existing patterns | Macro-generated, type-safe |
| Column family management | Direct RocksDB calls | memory-storage crate patterns | Consistent error handling, serialization |

**Key insight:** The heavy lifting (clustering, embeddings, scheduling) is solved. Focus on integration, storage schema, and API design.

## Common Pitfalls

### Pitfall 1: Blocking Async with HDBSCAN
**What goes wrong:** gRPC requests timeout during clustering
**Why it happens:** HDBSCAN `cluster()` is synchronous and CPU-intensive
**How to avoid:** Use `tokio::task::spawn_blocking` for clustering operations
**Warning signs:** High latency during extraction, scheduler hangs
```rust
// WRONG
let labels = clusterer.cluster()?;

// RIGHT
let labels = tokio::task::spawn_blocking(move || {
    clusterer.cluster()
}).await??;
```

### Pitfall 2: Too Many Small Topics
**What goes wrong:** Hundreds of near-duplicate topics
**Why it happens:** min_cluster_size too small, similarity threshold too low
**How to avoid:** Start with min_cluster_size=3, similarity_threshold=0.75, tune based on results
**Warning signs:** Topics with overlapping keywords, topics with only 1-2 nodes

### Pitfall 3: Forgetting Optional Feature Check
**What goes wrong:** Agent skills crash when topics disabled
**Why it happens:** RPCs called without checking GetTopicGraphStatus first
**How to avoid:** RPCs return Status::unavailable("Topic graph not enabled") when disabled
**Warning signs:** UNAVAILABLE errors in agent logs

### Pitfall 4: Stale Importance Scores
**What goes wrong:** Old topics remain "important" despite no recent mentions
**Why it happens:** Importance scores not recalculated after extraction
**How to avoid:** Recalculate all importance scores during extraction job
**Warning signs:** Topics from months ago ranking higher than recent ones

### Pitfall 5: Circular Parent/Child Relationships
**What goes wrong:** Graph traversal infinite loops
**Why it happens:** A is parent of B, B is parent of C, C is parent of A
**How to avoid:** Validate hierarchy on creation, limit depth to 3 levels
**Warning signs:** Stack overflow in GetRelatedTopics, timeout on hierarchy queries

### Pitfall 6: LLM Labeling Rate Limits
**What goes wrong:** Extraction job fails partway through
**Why it happens:** Too many LLM calls without rate limiting
**How to avoid:** Batch LLM calls, implement retry with backoff, always have keyword fallback
**Warning signs:** 429 errors from LLM API, partial extraction results

## Code Examples

Verified patterns from official sources and project conventions:

### Topic Data Model
```rust
// Source: PRD specification + memory-types patterns
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    /// Unique identifier (ULID)
    pub topic_id: String,
    /// Human-readable label (max 50 chars)
    pub label: String,
    /// Centroid embedding for similarity
    pub embedding: Vec<f32>,
    /// Time-decayed importance score
    pub importance_score: f64,
    /// Number of linked TOC nodes
    pub node_count: u32,
    /// First occurrence timestamp
    pub created_at: DateTime<Utc>,
    /// Most recent mention timestamp
    pub last_mentioned_at: DateTime<Utc>,
    /// Active or pruned
    pub status: TopicStatus,
    /// Keywords extracted from cluster
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TopicStatus {
    Active,
    Pruned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicLink {
    pub topic_id: String,
    pub node_id: String,
    pub relevance: f32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelationshipType {
    Similar,
    Parent,
    Child,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicRelationship {
    pub from_topic_id: String,
    pub to_topic_id: String,
    pub relationship_type: RelationshipType,
    pub score: f32,
}
```

### Topics Configuration
```rust
// Source: Existing config patterns in memory-types
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicsConfig {
    /// Master switch for topic functionality (default: false)
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub extraction: ExtractionConfig,

    #[serde(default)]
    pub labeling: LabelingConfig,

    #[serde(default)]
    pub importance: ImportanceConfig,

    #[serde(default)]
    pub relationships: RelationshipsConfig,

    #[serde(default)]
    pub lifecycle: LifecycleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    #[serde(default = "default_min_cluster_size")]
    pub min_cluster_size: usize,
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,
    #[serde(default = "default_extraction_schedule")]
    pub schedule: String,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

fn default_min_cluster_size() -> usize { 3 }
fn default_similarity_threshold() -> f32 { 0.75 }
fn default_extraction_schedule() -> String { "0 4 * * *".to_string() }
fn default_batch_size() -> usize { 500 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelingConfig {
    #[serde(default = "default_true")]
    pub use_llm: bool,
    #[serde(default = "default_true")]
    pub fallback_to_keywords: bool,
    #[serde(default = "default_max_label_length")]
    pub max_label_length: usize,
}

fn default_true() -> bool { true }
fn default_max_label_length() -> usize { 50 }

// ... similar patterns for ImportanceConfig, RelationshipsConfig, LifecycleConfig
```

### Proto Definitions for Topic RPCs
```protobuf
// Source: PRD specification, following existing memory.proto patterns

// Topic entity
message Topic {
    string topic_id = 1;
    string label = 2;
    double importance_score = 3;
    int32 node_count = 4;
    int64 created_at_ms = 5;
    int64 last_mentioned_at_ms = 6;
    repeated string keywords = 7;
}

enum RelationshipType {
    RELATIONSHIP_TYPE_UNSPECIFIED = 0;
    RELATIONSHIP_TYPE_SIMILAR = 1;
    RELATIONSHIP_TYPE_PARENT = 2;
    RELATIONSHIP_TYPE_CHILD = 3;
}

message RelatedTopic {
    Topic topic = 1;
    RelationshipType relationship_type = 2;
    float score = 3;
}

// GetTopicsByQuery - find topics matching natural language
message GetTopicsByQueryRequest {
    string query = 1;
    int32 limit = 2;
    float min_score = 3;
}

message GetTopicsByQueryResponse {
    repeated Topic topics = 1;
    repeated float scores = 2;
}

// GetTocNodesForTopic - get nodes linked to a topic
message GetTocNodesForTopicRequest {
    string topic_id = 1;
    int32 limit = 2;
    float min_relevance = 3;
}

message TopicNodeLink {
    string node_id = 1;
    string title = 2;
    TocLevel level = 3;
    float relevance = 4;
    int64 timestamp_ms = 5;
}

message GetTocNodesForTopicResponse {
    repeated TopicNodeLink nodes = 1;
    bool has_more = 2;
}

// GetTopTopics - most important topics
message GetTopTopicsRequest {
    int32 limit = 1;
    optional int64 start_time_ms = 2;
    optional int64 end_time_ms = 3;
}

message GetTopTopicsResponse {
    repeated Topic topics = 1;
}

// GetRelatedTopics - similar/parent/child topics
message GetRelatedTopicsRequest {
    string topic_id = 1;
    repeated RelationshipType relationship_types = 2;
    int32 limit = 3;
}

message GetRelatedTopicsResponse {
    repeated RelatedTopic related = 1;
}

// GetTopicGraphStatus - health and config for agent discovery
message GetTopicGraphStatusRequest {}

message GetTopicGraphStatusResponse {
    bool enabled = 1;
    bool healthy = 2;
    int64 topic_count = 3;
    int64 link_count = 4;
    int64 last_extraction_ms = 5;
    string message = 6;
    int32 half_life_days = 7;
    float similarity_threshold = 8;
}
```

### Keyword Extraction Fallback
```rust
// Source: Common TF-IDF-like pattern
use std::collections::{HashMap, HashSet};

/// Extract top keywords from a collection of text summaries.
pub fn extract_keywords(summaries: &[String], top_n: usize) -> Vec<String> {
    let stopwords = get_stopwords();
    let mut word_counts: HashMap<String, usize> = HashMap::new();

    for summary in summaries {
        for word in summary.split_whitespace() {
            let normalized = word.to_lowercase()
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_string();

            if normalized.len() >= 3 && !stopwords.contains(&normalized.as_str()) {
                *word_counts.entry(normalized).or_insert(0) += 1;
            }
        }
    }

    let mut keywords: Vec<_> = word_counts.into_iter().collect();
    keywords.sort_by(|a, b| b.1.cmp(&a.1));

    keywords.into_iter()
        .take(top_n)
        .map(|(word, _)| word)
        .collect()
}

/// Generate label from top keywords.
pub fn label_from_keywords(keywords: &[String]) -> String {
    keywords.iter()
        .take(3)
        .map(|k| {
            let mut chars = k.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn get_stopwords() -> HashSet<&'static str> {
    ["the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
     "of", "with", "by", "from", "as", "is", "was", "are", "were", "been",
     "be", "have", "has", "had", "do", "does", "did", "will", "would",
     "could", "should", "may", "might", "must", "this", "that", "these",
     "those", "it", "its", "we", "our", "you", "your", "about", "into"]
        .into_iter().collect()
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| K-means with fixed k | HDBSCAN auto-clustering | Well-established | No need to guess cluster count |
| External embedding APIs | Local ONNX models (fastembed) | 2024-2025 | No API costs, faster, offline capable |
| Manual topic curation | Automatic extraction | Standard for ML | Scales without human intervention |
| Static importance | Time-decayed scoring | Industry standard | Surfaces recent relevant topics |

**Deprecated/outdated:**
- Requiring external vector databases for similarity (RocksDB + pure Rust sufficient for topic scale)
- Using K-means for variable-density clusters (HDBSCAN is superior)
- HTTP APIs for topic queries (gRPC only per requirements)

## Open Questions

Things that couldn't be fully resolved:

1. **Optimal min_cluster_size for typical workloads**
   - What we know: 3 is recommended minimum to avoid noise
   - What's unclear: Ideal value for agent-memory scale (hundreds of TOC nodes)
   - Recommendation: Start with 3, make configurable, tune based on real usage

2. **LLM model selection for labeling**
   - What we know: PRD says "use configured summarizer model"
   - What's unclear: Whether summarizer model is optimized for labeling task
   - Recommendation: Reuse summarizer model with labeling-specific prompt, keyword fallback

3. **Parent/child hierarchy inference algorithm**
   - What we know: Use co-occurrence patterns and label analysis
   - What's unclear: Exact algorithm for determining parent/child vs. sibling
   - Recommendation: Start with simple "broader term" detection (e.g., "Security" contains "Authentication"), defer complex hierarchy to v2

4. **Phase 12 embedding model compatibility**
   - What we know: Phase 12 provides embedding infrastructure
   - What's unclear: Exact API surface of Phase 12's embedding service
   - Recommendation: Design TopicExtractor to accept trait/interface for embeddings, adapt to Phase 12 when implemented

## Sources

### Primary (HIGH confidence)
- [hdbscan crate docs.rs](https://docs.rs/hdbscan/latest/hdbscan/index.html) - API, usage patterns, version 0.12.0
- [linfa clustering docs](https://github.com/rust-ml/linfa/blob/master/algorithms/linfa-clustering/README.md) - DBSCAN alternative
- [tonic docs](https://docs.rs/tonic/latest/tonic/) - gRPC service patterns
- Existing codebase: memory-storage, memory-scheduler, memory-types patterns

### Secondary (MEDIUM confidence)
- [fastembed-rs GitHub](https://github.com/Anush008/fastembed-rs) - Local embedding generation
- [petal-clustering](https://github.com/petabi/petal-clustering) - Alternative HDBSCAN implementation
- PRD: docs/prds/topic-graph-memory-prd.md - Feature requirements
- Technical Plan: docs/plans/topic-graph-memory.md - Implementation outline

### Tertiary (LOW confidence)
- Web search results for HDBSCAN best practices - require validation with real data

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - hdbscan and tonic are well-documented, stable
- Architecture: HIGH - Patterns derived from existing codebase and standard Rust practices
- Pitfalls: MEDIUM - Based on documentation and experience, some require validation
- Integration approach: HIGH - Clear crate boundaries, follows existing patterns

**Research date:** 2026-02-01
**Valid until:** 2026-04-01 (60 days - dependencies are stable)

---

## Recommended Plan Breakdown

Based on this research, Phase 14 should be split into 6 plans:

### Plan 14-01: Topic Extraction (Wave 1)
**Focus:** Create memory-topics crate, CF_TOPICS, embedding clustering
**Tasks:**
- Add hdbscan to workspace dependencies
- Create memory-topics crate with proper structure
- Define Topic, TopicLink, TopicRelationship structs
- Implement CF_TOPICS, CF_TOPIC_LINKS, CF_TOPIC_RELS storage
- Implement HDBSCAN clustering with spawn_blocking
- Implement cosine_similarity and calculate_centroid
- Add TopicsConfig to settings
- Unit tests for clustering and storage

### Plan 14-02: Topic Labeling (Wave 2)
**Focus:** LLM-based labeling with keyword fallback
**Tasks:**
- Implement keyword extraction from cluster summaries
- Implement LLM labeling via existing summarizer
- Add fallback logic when LLM fails
- Add label truncation and normalization
- Unit tests for labeling

### Plan 14-03: Importance Scoring (Wave 3)
**Focus:** Time-decayed importance calculation
**Tasks:**
- Implement calculate_importance function
- Add importance recalculation in extraction job
- Add ImportanceConfig validation
- Integration with scheduler for periodic recalc
- Unit tests for decay math

### Plan 14-04: Topic Relationships (Wave 4)
**Focus:** Similar topics and hierarchy discovery
**Tasks:**
- Implement similarity relationship detection
- Implement parent/child hierarchy inference
- Store relationships in CF_TOPIC_RELS
- Add relationship validation (no cycles)
- Unit tests for relationship logic

### Plan 14-05: Navigation RPCs (Wave 5)
**Focus:** gRPC service implementation
**Tasks:**
- Add proto definitions to memory.proto
- Implement GetTopicsByQuery RPC
- Implement GetTocNodesForTopic RPC
- Implement GetTopTopics RPC
- Implement GetRelatedTopics RPC
- Implement GetTopicGraphStatus RPC
- Wire topic service into memory-service
- Integration tests

### Plan 14-06: Lifecycle Management (Wave 6)
**Focus:** Pruning, resurrection, CLI, scheduler integration
**Tasks:**
- Implement pruning logic (inactive topics)
- Implement resurrection on re-mention
- Add CLI commands for topic operations
- Register extraction job with scheduler
- Add pruning job with scheduler
- End-to-end integration tests
- Documentation
