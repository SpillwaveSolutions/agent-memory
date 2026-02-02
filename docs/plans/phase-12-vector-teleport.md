# Phase 12: Vector Teleport (HNSW) - Technical Plan

## Overview

This phase adds semantic similarity search to the agent-memory system using a local HNSW vector index. It enables "vector teleport" - jumping directly to semantically related TOC nodes or grips even when exact keywords don't match.

**Phase Type:** Planned phase (12)
**Depends On:** Phase 11 (BM25 Teleport)
**Required By:** Phase 13 (Outbox Index Ingestion - index lifecycle)

## Goals

1. Enable semantic similarity search across TOC summaries and grip excerpts
2. Use local embedding model (no external API dependency)
3. Provide hybrid search combining BM25 and vector scores
4. Support optional index lifecycle to bound vector storage growth
5. Maintain fallback to agentic search (Phase 10.5) when index unavailable
6. **Fully optional**: Users can disable vector search entirely via configuration

---

## Optional Feature: User Opt-Out

Vector Teleport is **completely optional**. Users may choose to disable it for:
- **Resource constraints**: Embedding model uses ~200MB RAM
- **Storage concerns**: Even with lifecycle, index adds disk usage
- **Simplicity preference**: BM25 + agentic search may be sufficient
- **Privacy concerns**: Some users prefer no ML inference on their data

### Configuration for Opt-Out

```yaml
# ~/.config/agent-memory/config.yaml

vector_index:
  enabled: false  # Disables all vector functionality
```

When disabled:
- No embedding model is loaded
- No HNSW index is created
- VectorTeleport RPC returns `UNAVAILABLE` status
- HybridSearch falls back to BM25-only
- GetVectorIndexStatus shows `enabled: false`

### Agent Skill Awareness

Agent skills **MUST** check configuration before using vector features:

```markdown
## Before Using Vector Search

1. Check if vector search is available:
   - Call `GetVectorIndexStatus` RPC
   - If `ready: false` or RPC returns UNAVAILABLE, use alternatives

2. When vector search is disabled:
   - Use BM25 Teleport (Phase 11) for keyword search
   - Use Agentic TOC Search (Phase 10.5) as fallback
   - Do NOT repeatedly call VectorTeleport

3. When vector search is enabled:
   - Prefer HybridSearch for best results
   - Use VectorTeleport for semantic-only queries
   - Fall back gracefully if index is temporarily unavailable
```

---

## Data Architecture

### What Gets Indexed

Vector Teleport indexes the same content as BM25 Teleport, but with embeddings:

```
+------------------------------------------------------------------+
|  YEAR (toc:year:2026)                                            |
|  +-- Summary embedding: aggregated themes                        |
|  +-- Indexed: optional (lowest priority, coarsest granularity)   |
+------------------------------------------------------------------+
|  MONTH (toc:month:2026-01)                                       |
|  +-- Summary embedding: monthly themes                           |
|  +-- Indexed: always (coarse semantic anchor)                    |
+------------------------------------------------------------------+
|  WEEK (toc:week:2026-W04)                                        |
|  +-- Summary embedding: weekly themes                            |
|  +-- Indexed: always (medium granularity)                        |
+------------------------------------------------------------------+
|  DAY (toc:day:2026-01-30)                                        |
|  +-- Summary embedding: daily themes                             |
|  +-- Indexed: always (fine granularity)                          |
+------------------------------------------------------------------+
|  SEGMENT (toc:segment:2026-01-30:abc123) [LEAF NODE]             |
|  +-- Bullets with grip_ids                                       |
|  +-- Indexed: recent (prunable after N days)                     |
+------------------------------------------------------------------+
|  GRIPS (grip:1706745600000:xyz)                                  |
|  +-- Excerpt embedding (max 200 chars)                           |
|  +-- Indexed: recent (prunable with parent segment)              |
+------------------------------------------------------------------+
```

### Storage Architecture

```
RocksDB (Source of Truth)
+-- CF_TOC_NODES     # TOC nodes (immutable summaries)
+-- CF_GRIPS         # Grips (immutable excerpts)
+-- CF_EVENTS        # Events (immutable)
+-- CF_EMBEDDINGS    # NEW: Persisted embeddings [Phase 12]
    +-- emb:{node_id} -> f32[] + metadata
    +-- emb:grip:{grip_id} -> f32[] + metadata

HNSW Index (Ephemeral, Rebuildable)
+-- ~/.local/share/agent-memory/vector-index/
    +-- hnsw.bin      # HNSW graph structure
    +-- metadata.json # Model, dimension, version

Key Insight: HNSW index is a rebuildable accelerator.
If corrupted or deleted, rebuild from CF_EMBEDDINGS.
```

### Index Lifecycle (NOT Data Deletion)

The append-only architecture means events, TOC nodes, and grips are never deleted. Index lifecycle only applies to the vector index:

| Age | Segment Embeddings | Grip Embeddings | Higher-Level Embeddings |
|-----|-------------------|-----------------|------------------------|
| < 30 days | Keep in HNSW | Keep in HNSW | Keep |
| 30-90 days | Aggregate to Day | Remove from HNSW | Keep |
| 90-365 days | Day only in HNSW | Removed | Keep |
| > 365 days | Week only in HNSW | Removed | Keep |

**Why This Works:**
- Agentic search (Phase 10.5) still works - traverses TOC hierarchy
- BM25 (Phase 11) still works - indexes TOC text, not vectors
- Vector search finds month/week/day nodes, agent drills down from there
- Grips remain in RocksDB - only vectors removed from HNSW index

---

## gRPC API Design

### New RPCs

Add to `MemoryService` in `proto/memory.proto`:

```protobuf
// Vector-based semantic similarity search
rpc VectorTeleport(VectorTeleportRequest) returns (VectorTeleportResponse);

// Combined BM25 + vector hybrid search
rpc HybridSearch(HybridSearchRequest) returns (HybridSearchResponse);

// Admin: Rebuild vector index from persisted embeddings
rpc RebuildVectorIndex(RebuildVectorIndexRequest) returns (RebuildVectorIndexResponse);

// Admin: Get vector index health and metrics
rpc GetVectorIndexStatus(GetVectorIndexStatusRequest) returns (GetVectorIndexStatusResponse);
```

### VectorTeleport Messages

```protobuf
// Request for vector similarity search
message VectorTeleportRequest {
  // Natural language query (will be embedded)
  string query = 1;
  // Maximum results to return
  int32 limit = 2;
  // Filter by TOC levels (empty = all levels)
  repeated TocLevel levels = 3;
  // Include grip embeddings in search
  bool include_grips = 4;
  // Minimum similarity threshold (0.0-1.0)
  float min_similarity = 5;
}

// A teleport result with similarity score
message TeleportResult {
  // Node ID or grip ID
  string id = 1;
  // Cosine similarity score (0.0-1.0)
  float similarity = 2;
  // TOC level (UNSPECIFIED for grips)
  TocLevel level = 3;
  // Title or excerpt for display
  string title = 4;
  // True if this is a grip result
  bool is_grip = 5;
}

// Response with vector search results
message VectorTeleportResponse {
  // Matching nodes/grips sorted by similarity
  repeated TeleportResult results = 1;
  // Query embedding latency (ms)
  int64 embed_latency_ms = 2;
  // Search latency (ms)
  int64 search_latency_ms = 3;
}
```

### HybridSearch Messages

```protobuf
// Request for combined BM25 + vector search
message HybridSearchRequest {
  // Search query
  string query = 1;
  // Maximum results to return
  int32 limit = 2;
  // BM25 weight (0.0-1.0, default 0.5)
  float bm25_weight = 3;
  // Vector weight (0.0-1.0, default 0.5)
  float vector_weight = 4;
  // Filter by TOC levels
  repeated TocLevel levels = 5;
  // Include grips
  bool include_grips = 6;
}

// A hybrid search result with combined score
message HybridResult {
  // Node ID or grip ID
  string id = 1;
  // Combined score (weighted BM25 + vector)
  float score = 2;
  // BM25 score component
  float bm25_score = 3;
  // Vector similarity component
  float vector_score = 4;
  // TOC level
  TocLevel level = 5;
  // Title or excerpt
  string title = 6;
  // True if grip result
  bool is_grip = 7;
}

// Response with hybrid results
message HybridSearchResponse {
  // Results sorted by combined score
  repeated HybridResult results = 1;
}
```

### Admin Messages

```protobuf
// Request to rebuild vector index
message RebuildVectorIndexRequest {
  // If true, rebuild from RocksDB embeddings
  // If false, re-embed all content (slower, updates embeddings)
  bool from_persisted = 1;
}

message RebuildVectorIndexResponse {
  bool success = 1;
  optional string error = 2;
  int64 vectors_indexed = 3;
  int64 duration_ms = 4;
}

// Request for index status
message GetVectorIndexStatusRequest {}

message GetVectorIndexStatusResponse {
  // Whether index is loaded and ready
  bool ready = 1;
  // Total vectors in index
  int64 vector_count = 2;
  // Index size on disk (bytes)
  int64 size_bytes = 3;
  // Embedding model name
  string model_name = 4;
  // Embedding dimension
  int32 dimension = 5;
  // Last rebuild timestamp (ms)
  int64 last_rebuild_ms = 6;
}
```

---

## Implementation Components

### Component Layout

| Component | Crate | File | Purpose |
|-----------|-------|------|---------|
| Embedding model | `memory-embeddings` (new) | `src/lib.rs` | Local embedding generation |
| HNSW index | `memory-vector` (new) | `src/lib.rs` | Vector storage and search |
| Vector service | `memory-service` | `src/vector_service.rs` | RPC handlers |
| Proto definitions | `proto` | `memory.proto` | Message definitions |
| CLI commands | `memory-daemon` | `src/cli.rs` | `vector-search`, `rebuild-vector` |
| Config | `memory-core` | `src/config.rs` | Vector index settings |

### Embedding Model Integration

```rust
// memory-embeddings/src/lib.rs

use candle_core::{Device, Tensor};
use candle_transformers::models::bert::BertModel;

/// Configuration for the embedding model
pub struct EmbeddingConfig {
    /// Model name (e.g., "all-MiniLM-L6-v2")
    pub model_name: String,
    /// Embedding dimension (e.g., 384)
    pub dimension: usize,
    /// Device (CPU or CUDA)
    pub device: Device,
}

/// Embedding model for generating text embeddings
pub struct EmbeddingModel {
    model: BertModel,
    tokenizer: tokenizers::Tokenizer,
    config: EmbeddingConfig,
}

impl EmbeddingModel {
    /// Load model from local weights
    pub fn load(config: EmbeddingConfig) -> Result<Self, Error> {
        // Load from ~/.local/share/agent-memory/models/
        // Falls back to downloading if not present
    }

    /// Generate embedding for text
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, Error> {
        let tokens = self.tokenizer.encode(text, true)?;
        let input_ids = Tensor::new(&tokens.get_ids(), &self.config.device)?;
        let embeddings = self.model.forward(&input_ids)?;
        // Mean pooling over tokens
        let mean = embeddings.mean(1)?;
        Ok(mean.to_vec1()?)
    }

    /// Batch embed multiple texts
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, Error> {
        texts.iter().map(|t| self.embed(t)).collect()
    }
}
```

### HNSW Index Implementation

```rust
// memory-vector/src/lib.rs

use usearch::{Index, IndexOptions, MetricKind};

/// Configuration for the HNSW index
pub struct HnswConfig {
    /// Embedding dimension
    pub dimension: usize,
    /// Max connections per node (M parameter)
    pub m: usize,
    /// Construction-time search depth
    pub ef_construction: usize,
    /// Query-time search depth
    pub ef_search: usize,
    /// Index file path
    pub path: PathBuf,
}

/// HNSW vector index
pub struct VectorIndex {
    index: Index,
    config: HnswConfig,
    /// Maps internal index IDs to node_ids/grip_ids
    id_map: HashMap<u64, String>,
}

impl VectorIndex {
    /// Create or load index from disk
    pub fn open(config: HnswConfig) -> Result<Self, Error> {
        let options = IndexOptions {
            dimensions: config.dimension,
            metric: MetricKind::Cos, // Cosine similarity
            connectivity: config.m,
            expansion_add: config.ef_construction,
            expansion_search: config.ef_search,
        };

        if config.path.exists() {
            let index = Index::restore(&config.path)?;
            let id_map = load_id_map(&config.path)?;
            Ok(Self { index, config, id_map })
        } else {
            let index = Index::new(&options)?;
            Ok(Self { index, config, id_map: HashMap::new() })
        }
    }

    /// Add or update a vector
    pub fn upsert(&mut self, id: &str, embedding: &[f32]) -> Result<(), Error> {
        let internal_id = self.get_or_create_internal_id(id);
        self.index.add(internal_id, embedding)?;
        self.id_map.insert(internal_id, id.to_string());
        Ok(())
    }

    /// Search for similar vectors
    pub fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>, Error> {
        let results = self.index.search(query, limit)?;
        Ok(results
            .iter()
            .map(|(internal_id, similarity)| SearchResult {
                id: self.id_map.get(internal_id).cloned().unwrap_or_default(),
                similarity: *similarity,
            })
            .collect())
    }

    /// Remove a vector by ID
    pub fn remove(&mut self, id: &str) -> Result<bool, Error> {
        if let Some(internal_id) = self.find_internal_id(id) {
            self.index.remove(internal_id)?;
            self.id_map.remove(&internal_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Persist index to disk
    pub fn save(&self) -> Result<(), Error> {
        self.index.save(&self.config.path)?;
        save_id_map(&self.config.path, &self.id_map)?;
        Ok(())
    }

    /// Get index statistics
    pub fn stats(&self) -> IndexStats {
        IndexStats {
            vector_count: self.index.size(),
            dimension: self.config.dimension,
            size_bytes: self.config.path.metadata().map(|m| m.len()).unwrap_or(0),
        }
    }
}

/// Search result with ID and similarity
pub struct SearchResult {
    pub id: String,
    pub similarity: f32,
}

/// Index statistics
pub struct IndexStats {
    pub vector_count: usize,
    pub dimension: usize,
    pub size_bytes: u64,
}
```

### gRPC Service Implementation

```rust
// memory-service/src/vector_service.rs

impl MemoryService {
    pub async fn vector_teleport(
        &self,
        request: Request<VectorTeleportRequest>,
    ) -> Result<Response<VectorTeleportResponse>, Status> {
        let req = request.into_inner();

        // Check if vector index is available
        let vector_index = self.vector_index.as_ref()
            .ok_or_else(|| Status::unavailable("Vector index not loaded"))?;

        // Embed the query
        let embed_start = Instant::now();
        let query_embedding = self.embedding_model.embed(&req.query)
            .map_err(|e| Status::internal(format!("Embedding failed: {}", e)))?;
        let embed_latency = embed_start.elapsed().as_millis() as i64;

        // Search the index
        let search_start = Instant::now();
        let limit = if req.limit > 0 { req.limit as usize } else { 10 };
        let raw_results = vector_index.search(&query_embedding, limit * 2)?;
        let search_latency = search_start.elapsed().as_millis() as i64;

        // Filter and enrich results
        let mut results = Vec::with_capacity(limit);
        for result in raw_results {
            // Apply similarity threshold
            if result.similarity < req.min_similarity {
                continue;
            }

            // Parse ID to determine type
            let (is_grip, level, title) = if result.id.starts_with("grip:") {
                let grip = self.storage.get_grip(&result.id)?;
                (true, TocLevel::Unspecified, grip.map(|g| g.excerpt).unwrap_or_default())
            } else {
                let node = self.storage.get_toc_node(&result.id)?;
                let (level, title) = node.map(|n| (n.level, n.title)).unwrap_or_default();
                (false, level, title)
            };

            // Apply level filter
            if !req.levels.is_empty() && !is_grip {
                if !req.levels.contains(&(level as i32)) {
                    continue;
                }
            }

            // Skip grips if not requested
            if is_grip && !req.include_grips {
                continue;
            }

            results.push(TeleportResult {
                id: result.id,
                similarity: result.similarity,
                level: level as i32,
                title,
                is_grip,
            });

            if results.len() >= limit {
                break;
            }
        }

        Ok(Response::new(VectorTeleportResponse {
            results,
            embed_latency_ms: embed_latency,
            search_latency_ms: search_latency,
        }))
    }

    pub async fn hybrid_search(
        &self,
        request: Request<HybridSearchRequest>,
    ) -> Result<Response<HybridSearchResponse>, Status> {
        let req = request.into_inner();

        // Get BM25 results (from Phase 11)
        let bm25_results = self.bm25_search(&req.query, req.limit as usize * 2).await?;

        // Get vector results
        let vector_results = self.vector_teleport(Request::new(VectorTeleportRequest {
            query: req.query.clone(),
            limit: req.limit * 2,
            levels: req.levels.clone(),
            include_grips: req.include_grips,
            min_similarity: 0.0,
        })).await?.into_inner();

        // Combine and rank using reciprocal rank fusion (RRF)
        let bm25_weight = if req.bm25_weight > 0.0 { req.bm25_weight } else { 0.5 };
        let vector_weight = if req.vector_weight > 0.0 { req.vector_weight } else { 0.5 };

        let mut score_map: HashMap<String, HybridResult> = HashMap::new();

        // Add BM25 scores
        for (rank, result) in bm25_results.iter().enumerate() {
            let rrf_score = 1.0 / (60.0 + rank as f32); // k=60 for RRF
            let entry = score_map.entry(result.id.clone()).or_insert_with(|| HybridResult {
                id: result.id.clone(),
                score: 0.0,
                bm25_score: result.score,
                vector_score: 0.0,
                level: result.level,
                title: result.title.clone(),
                is_grip: result.is_grip,
            });
            entry.score += bm25_weight * rrf_score;
        }

        // Add vector scores
        for (rank, result) in vector_results.results.iter().enumerate() {
            let rrf_score = 1.0 / (60.0 + rank as f32);
            let entry = score_map.entry(result.id.clone()).or_insert_with(|| HybridResult {
                id: result.id.clone(),
                score: 0.0,
                bm25_score: 0.0,
                vector_score: result.similarity,
                level: result.level,
                title: result.title.clone(),
                is_grip: result.is_grip,
            });
            entry.vector_score = result.similarity;
            entry.score += vector_weight * rrf_score;
        }

        // Sort by combined score and limit
        let mut results: Vec<_> = score_map.into_values().collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        results.truncate(req.limit as usize);

        Ok(Response::new(HybridSearchResponse { results }))
    }
}
```

---

## Configuration

### Config Schema

```yaml
# ~/.config/agent-memory/config.yaml

vector_index:
  # OPTIONAL: Set to false to completely disable vector search
  # When disabled: no model loaded, no index created, RPCs return UNAVAILABLE
  # Default: true
  enabled: true

  # Embedding model settings
  embedding:
    # Model identifier (downloads from HuggingFace if not cached)
    model_name: "sentence-transformers/all-MiniLM-L6-v2"
    # Embedding dimension (must match model)
    dimension: 384
    # Model cache directory
    cache_dir: "~/.local/share/agent-memory/models"

  # HNSW index settings
  hnsw:
    # Max connections per node
    m: 16
    # Construction-time search depth
    ef_construction: 200
    # Query-time search depth
    ef_search: 50
    # Index file path
    path: "~/.local/share/agent-memory/vector-index"

  # Index lifecycle (optional)
  lifecycle:
    enabled: true
    # Keep segment embeddings in HNSW for N days
    segment_retention_days: 30
    # Keep grip embeddings in HNSW for N days
    grip_retention_days: 30
    # Keep day embeddings for N days
    day_retention_days: 365
    # Keep week embeddings for N days (5 years)
    week_retention_days: 1825

  # Scheduled maintenance
  maintenance:
    # Cron expression for lifecycle pruning
    prune_schedule: "0 3 * * *"  # 3 AM daily
    # Batch size for pruning operations
    prune_batch_size: 1000
```

---

## CLI Commands

### New Commands

```bash
# Vector similarity search
memory-daemon vector-search --query "authentication patterns"

# Vector search with level filter
memory-daemon vector-search --query "JWT tokens" --level day --level segment

# Hybrid search (BM25 + vector)
memory-daemon hybrid-search --query "authentication" --bm25-weight 0.3 --vector-weight 0.7

# Rebuild vector index from persisted embeddings
memory-daemon rebuild-vector --from-persisted

# Rebuild vector index with re-embedding
memory-daemon rebuild-vector --full

# Show vector index status
memory-daemon vector-status
```

### CLI Implementation

```rust
// memory-daemon/src/cli.rs

#[derive(Parser)]
enum Commands {
    // ... existing commands ...

    /// Search using vector similarity
    VectorSearch {
        /// Search query
        #[arg(long)]
        query: String,

        /// Filter by TOC level (year, month, week, day, segment)
        #[arg(long)]
        level: Vec<String>,

        /// Include grip results
        #[arg(long)]
        include_grips: bool,

        /// Maximum results
        #[arg(long, default_value = "10")]
        limit: u32,

        /// Minimum similarity threshold
        #[arg(long, default_value = "0.5")]
        min_similarity: f32,
    },

    /// Combined BM25 + vector search
    HybridSearch {
        /// Search query
        #[arg(long)]
        query: String,

        /// BM25 weight (0.0-1.0)
        #[arg(long, default_value = "0.5")]
        bm25_weight: f32,

        /// Vector weight (0.0-1.0)
        #[arg(long, default_value = "0.5")]
        vector_weight: f32,

        /// Maximum results
        #[arg(long, default_value = "10")]
        limit: u32,
    },

    /// Rebuild vector index
    RebuildVector {
        /// Rebuild from persisted embeddings (faster)
        #[arg(long)]
        from_persisted: bool,

        /// Full rebuild with re-embedding (slower, updates embeddings)
        #[arg(long)]
        full: bool,
    },

    /// Show vector index status
    VectorStatus,
}
```

---

## Testing Strategy

### Unit Tests

```rust
// memory-embeddings/src/lib.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_dimension() {
        let model = EmbeddingModel::load(EmbeddingConfig {
            model_name: "all-MiniLM-L6-v2".to_string(),
            dimension: 384,
            device: Device::Cpu,
        }).unwrap();

        let embedding = model.embed("test text").unwrap();
        assert_eq!(embedding.len(), 384);
    }

    #[test]
    fn test_similar_texts_have_high_similarity() {
        let model = EmbeddingModel::load_test().unwrap();
        let e1 = model.embed("JWT authentication").unwrap();
        let e2 = model.embed("JSON web token auth").unwrap();
        let e3 = model.embed("database migrations").unwrap();

        let sim_12 = cosine_similarity(&e1, &e2);
        let sim_13 = cosine_similarity(&e1, &e3);

        // Related concepts should be more similar
        assert!(sim_12 > sim_13, "JWT/token should be more similar than JWT/database");
        assert!(sim_12 > 0.7, "Related concepts should have high similarity");
    }
}

// memory-vector/src/lib.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_index_upsert_and_search() {
        let tmp = TempDir::new().unwrap();
        let mut index = VectorIndex::open(HnswConfig {
            dimension: 4,
            m: 8,
            ef_construction: 50,
            ef_search: 20,
            path: tmp.path().join("test.idx"),
        }).unwrap();

        // Insert some vectors
        index.upsert("node:1", &[1.0, 0.0, 0.0, 0.0]).unwrap();
        index.upsert("node:2", &[0.9, 0.1, 0.0, 0.0]).unwrap();
        index.upsert("node:3", &[0.0, 0.0, 1.0, 0.0]).unwrap();

        // Search for similar
        let results = index.search(&[1.0, 0.0, 0.0, 0.0], 2).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "node:1");
        assert_eq!(results[1].id, "node:2");
    }

    #[test]
    fn test_index_persistence() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.idx");

        {
            let mut index = VectorIndex::open(HnswConfig {
                dimension: 4,
                path: path.clone(),
                ..Default::default()
            }).unwrap();
            index.upsert("node:1", &[1.0, 0.0, 0.0, 0.0]).unwrap();
            index.save().unwrap();
        }

        // Reload from disk
        let index = VectorIndex::open(HnswConfig {
            dimension: 4,
            path,
            ..Default::default()
        }).unwrap();

        assert_eq!(index.stats().vector_count, 1);
    }
}
```

### Integration Tests

```rust
// memory-service/tests/vector_integration.rs

#[tokio::test]
async fn test_vector_teleport_finds_semantic_matches() {
    let server = setup_test_server_with_vector().await;

    // Ingest test data with different phrasings
    ingest_event(&server, "User discussed JWT token validation").await;
    ingest_event(&server, "Implemented OAuth authentication flow").await;
    ingest_event(&server, "Fixed database migration script").await;

    // Trigger TOC build and embedding
    trigger_toc_build(&server).await;
    trigger_embedding(&server).await;

    // Search for "auth" - should find JWT and OAuth, not database
    let response = server.vector_teleport(VectorTeleportRequest {
        query: "authentication system".to_string(),
        limit: 10,
        include_grips: true,
        ..Default::default()
    }).await.unwrap();

    // Verify semantic matching works
    let titles: Vec<_> = response.results.iter().map(|r| &r.title).collect();
    assert!(titles.iter().any(|t| t.contains("JWT") || t.contains("OAuth")));
    assert!(!titles.iter().any(|t| t.contains("database")));
}

#[tokio::test]
async fn test_hybrid_search_combines_signals() {
    let server = setup_test_server_with_vector().await;

    // Ingest test data
    ingest_event(&server, "JWT token expiry bug fixed").await;
    ingest_event(&server, "Authentication token refresh logic").await;

    trigger_toc_build(&server).await;
    trigger_embedding(&server).await;

    // Hybrid search - should rank by combined BM25 + vector
    let response = server.hybrid_search(HybridSearchRequest {
        query: "JWT token".to_string(),
        limit: 5,
        bm25_weight: 0.5,
        vector_weight: 0.5,
        ..Default::default()
    }).await.unwrap();

    assert!(!response.results.is_empty());
    // First result should have both BM25 and vector scores
    let first = &response.results[0];
    assert!(first.bm25_score > 0.0);
    assert!(first.vector_score > 0.0);
}
```

---

## Success Criteria

| Criterion | Verification |
|-----------|--------------|
| Vector search finds semantically related content | Integration test: "JWT" finds "token authentication" |
| Embedding model runs locally without API | Unit test: embed() works offline |
| HNSW index persists across restarts | Unit test: save/load roundtrip |
| Hybrid search combines BM25 and vector | Integration test: combined scores |
| Index rebuild from CF_EMBEDDINGS works | Integration test: delete HNSW, rebuild |
| Search latency < 200ms for 10K vectors | Performance test |
| Embedding latency < 50ms per query | Performance test |
| CLI commands work | Manual testing |

---

## Integration Points

### Existing Component Integration

| Component | Integration Point |
|-----------|------------------|
| `Storage::get_toc_node` | Load node for title/metadata |
| `Storage::get_grip` | Load grip for excerpt/metadata |
| `TocNode.title`, `TocNode.summary` | Text to embed |
| `Grip.excerpt` | Text to embed |
| Phase 11 BM25 index | Combined in HybridSearch |
| Phase 10 Scheduler | Lifecycle pruning job |

### Phase 13 Integration

Phase 13 (Outbox Index Ingestion) will drive:
1. Incremental embedding generation for new TOC nodes/grips
2. Lifecycle pruning based on outbox timestamps
3. Checkpoint-based crash recovery for embedding pipeline

---

## Implementation Plan

### Wave 1: Embedding Model (Plan 12-01)

**Files Modified:**
- `Cargo.toml` - Add candle dependencies
- `crates/memory-embeddings/src/lib.rs` (new) - Embedding model
- `crates/memory-embeddings/Cargo.toml` (new) - Crate manifest

**Tasks:**
1. Create memory-embeddings crate
2. Implement EmbeddingModel with candle
3. Add model download/caching
4. Unit tests for embedding generation

### Wave 2: HNSW Index (Plan 12-02)

**Files Modified:**
- `crates/memory-vector/src/lib.rs` (new) - HNSW index
- `crates/memory-vector/Cargo.toml` (new) - Crate manifest
- `crates/memory-core/src/config.rs` - Vector config

**Tasks:**
1. Create memory-vector crate
2. Implement VectorIndex with usearch
3. Add persistence and ID mapping
4. Unit tests for index operations

### Wave 3: gRPC Integration (Plan 12-03)

**Files Modified:**
- `proto/memory.proto` - VectorTeleport, HybridSearch RPCs
- `crates/memory-service/src/vector_service.rs` (new)
- `crates/memory-service/src/lib.rs` - Wire up handlers

**Tasks:**
1. Add proto message definitions
2. Implement VectorTeleport RPC
3. Implement HybridSearch RPC
4. Integration tests

### Wave 4: CLI & Admin (Plan 12-04)

**Files Modified:**
- `crates/memory-daemon/src/cli.rs` - New commands
- `crates/memory-service/src/vector_service.rs` - Admin RPCs

**Tasks:**
1. Add vector-search CLI command
2. Add hybrid-search CLI command
3. Add rebuild-vector command
4. Add vector-status command
5. Implement admin RPCs

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Embedding model too large (>1GB) | Slow startup, high memory | Use quantized MiniLM (30MB) |
| HNSW index corrupted | Search unavailable | Rebuild from CF_EMBEDDINGS |
| Embedding model drift (new version) | Old embeddings incompatible | Store model version, re-embed on upgrade |
| Cold start latency | First query slow | Background warmup on daemon start |
| Dependency conflicts (candle, usearch) | Build issues | Pin versions, use feature flags |

---

## Agent Skill Integration Guide

### Checking Vector Search Availability

Agent skills MUST handle the case where vector search is disabled or unavailable:

```rust
// Example: Check before using vector search
async fn search_with_fallback(query: &str) -> Result<SearchResults> {
    // First, check if vector search is available
    let status = client.get_vector_index_status().await;

    match status {
        Ok(status) if status.ready => {
            // Vector search available - use hybrid for best results
            client.hybrid_search(HybridSearchRequest {
                query: query.to_string(),
                ..Default::default()
            }).await
        }
        _ => {
            // Vector search disabled or unavailable - fall back to BM25
            client.teleport_search(TeleportSearchRequest {
                query: query.to_string(),
                ..Default::default()
            }).await
        }
    }
}
```

### Skill SKILL.md Documentation Pattern

Skills should document the optional nature:

```markdown
## Search Methods

### Preferred: Hybrid Search (if available)
Combines keyword and semantic search for best results.
**Requires:** Vector index enabled (`vector_index.enabled: true`)

### Fallback: BM25 Search
Keyword-based search, always available when Phase 11 is enabled.

### Ultimate Fallback: Agentic Search
Index-free term matching, always works.

## Checking Configuration

Before using vector search, call `GetVectorIndexStatus`:
- If `ready: true` → Use HybridSearch or VectorTeleport
- If `ready: false` → Use TeleportSearch (BM25) or SearchChildren (agentic)
- If RPC fails → System may be starting up, retry or use agentic
```

### Error Handling for Disabled Features

When vector search is disabled, the daemon returns specific error codes:

| RPC | Response When Disabled |
|-----|----------------------|
| VectorTeleport | `UNAVAILABLE` with message "Vector index not enabled" |
| HybridSearch | Falls back to BM25-only (no error, but `vector_score` is 0) |
| RebuildVectorIndex | `FAILED_PRECONDITION` with message "Vector index not enabled" |
| GetVectorIndexStatus | Returns `ready: false, enabled: false` |

---

*Plan created: 2026-02-01*
*Target: Phase 12 - Vector Teleport (HNSW)*
