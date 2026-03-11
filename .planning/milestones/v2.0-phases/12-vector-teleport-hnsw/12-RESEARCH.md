# Phase 12 Research: Vector Teleport (HNSW)

## Source Documents

This research synthesizes two comprehensive documents:
1. `docs/plans/phase-12-vector-teleport.md` - Technical implementation plan
2. `docs/prds/hierarchical-vector-indexing-prd.md` - Product requirements document

## Executive Summary

Phase 12 implements Layer 4 (Semantic Teleport) of the Cognitive Architecture. Vector search enables semantic similarity queries that find conceptually related content even when exact keywords don't match. This complements BM25 (Phase 11) which excels at exact keyword matching.

**Key Principle:** "Indexes are accelerators, not dependencies" - Vector search accelerates retrieval but the system must work without it.

## Technology Decisions

### Embedding Model

**Choice:** all-MiniLM-L6-v2 via Candle (local inference)

| Criteria | Decision |
|----------|----------|
| Model | all-MiniLM-L6-v2 (384 dimensions) |
| Runtime | Candle (Rust-native, no Python) |
| Inference | Local CPU (no external API calls) |
| Batch size | 32 texts default |

**Rationale:**
- Rust-native avoids Python dependency
- 384 dimensions balances quality vs storage
- Local inference maintains privacy and works offline
- Well-tested model with good semantic understanding

### Vector Index

**Choice:** usearch library with HNSW algorithm

| Criteria | Decision |
|----------|----------|
| Library | usearch (Rust bindings) |
| Algorithm | HNSW (Hierarchical Navigable Small World) |
| Storage | Memory-mapped file + RocksDB metadata |
| Parameters | M=16, ef_construction=200, ef_search=100 |

**Rationale:**
- usearch is production-grade, fast, supports mmap
- HNSW provides O(log n) search with high recall
- Memory-mapped allows larger-than-RAM indexes
- Parameters tuned for quality over speed

## Architecture

### Crate Structure

```
crates/
├── memory-embeddings/    # Embedding model wrapper
│   ├── model.rs          # EmbeddingModel trait
│   ├── candle.rs         # Candle implementation
│   └── cache.rs          # Model file caching
│
└── memory-vector/        # Vector index
    ├── index.rs          # VectorIndex trait
    ├── hnsw.rs           # usearch HNSW implementation
    ├── lifecycle.rs      # Prune/rebuild operations
    └── hybrid.rs         # Score fusion with BM25
```

### Data Flow

```
TOC Node/Grip created
       │
       ▼
   Outbox entry written (CF_OUTBOX)
       │
       ▼
   Outbox consumer reads entry
       │
       ▼
   EmbeddingModel.embed(text)
       │
       ▼
   VectorIndex.add(id, vector)
       │
       ▼
   Checkpoint updated (CF_CHECKPOINTS)
```

### Index Lifecycle

**Important:** Pruning removes vectors from the index but NEVER deletes primary data from RocksDB. The append-only event store remains immutable.

```
Prune Policy:
- age_threshold_days: 365
- access_count_threshold: 0
- Batch size: 1000
- Full rebuild if prune ratio > 30%
```

## gRPC Interface

### VectorTeleport RPC

```protobuf
rpc VectorTeleport(VectorTeleportRequest) returns (VectorTeleportResponse);

message VectorTeleportRequest {
  string query = 1;
  int32 top_k = 2;           // default 10
  float min_score = 3;       // default 0.0
  TimeRange time_filter = 4; // optional
  TargetType target = 5;     // TOC_NODE or GRIP
}

message VectorTeleportResponse {
  repeated VectorMatch matches = 1;
  VectorIndexStatus index_status = 2;
}
```

### HybridSearch RPC

```protobuf
rpc HybridSearch(HybridSearchRequest) returns (HybridSearchResponse);

message HybridSearchRequest {
  string query = 1;
  int32 top_k = 2;
  HybridMode mode = 3;  // VECTOR_ONLY, BM25_ONLY, HYBRID
  float bm25_weight = 4;   // default 0.5
  float vector_weight = 5; // default 0.5
}
```

### GetVectorIndexStatus RPC

```protobuf
rpc GetVectorIndexStatus(Empty) returns (VectorIndexStatus);

message VectorIndexStatus {
  bool available = 1;
  int64 vector_count = 2;
  int64 dimension = 3;
  string last_indexed = 4;
  string index_path = 5;
}
```

## Hybrid Search (RRF)

**Algorithm:** Reciprocal Rank Fusion (RRF)

```
RRF_score(doc) = Σ 1/(k + rank_i(doc))

where:
- k = 60 (constant)
- rank_i = rank in result list i
```

**Score Fusion Modes:**
1. VECTOR_ONLY - Pure semantic search
2. BM25_ONLY - Pure keyword search
3. HYBRID - RRF fusion of both

## Functional Requirements Summary

From PRD Section 3:

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-01 | Local embedding via Candle | Must |
| FR-02 | HNSW index via usearch | Must |
| FR-03 | VectorTeleport gRPC RPC | Must |
| FR-04 | HybridSearch gRPC RPC | Must |
| FR-05 | Time-filtered vector search | Must |
| FR-06 | Index lifecycle (prune, rebuild) | Must |
| FR-07 | GetVectorIndexStatus RPC | Must |
| FR-08 | CLI vector commands | Should |
| FR-09 | Outbox-driven indexing | Must |
| FR-10 | Checkpoint-based recovery | Must |

## Implementation Waves (from Technical Plan)

### Wave 1: Embedding Infrastructure
- memory-embeddings crate
- Candle model loading and inference
- Model file caching
- Batch embedding API

### Wave 2: Vector Index
- memory-vector crate
- usearch HNSW integration
- Index persistence (mmap)
- Metadata storage in RocksDB

### Wave 3: gRPC Integration
- VectorTeleport RPC
- HybridSearch RPC
- GetVectorIndexStatus RPC
- Score normalization

### Wave 4: CLI and Admin
- `memory-cli teleport vector` command
- `memory-cli admin prune-vectors` command
- `memory-cli admin rebuild-vectors` command

## Integration with Cognitive Architecture

**Layer 4 Role:**
- Primary for EXPLORE intent (semantic discovery)
- Secondary for ANSWER intent (after BM25)
- Disabled for LOCATE intent (exact matching preferred)
- Part of Hybrid mode when both indexes available

**Fallback Chain:**
```
Vector unavailable → BM25 → Agentic TOC Search
```

**Check-Then-Search:**
Skills must call GetVectorIndexStatus before using VectorTeleport.

## Testing Strategy

1. **Unit Tests:**
   - Embedding model output shape
   - HNSW add/search operations
   - Score normalization

2. **Integration Tests:**
   - Full indexing pipeline
   - Hybrid search accuracy
   - Graceful degradation when disabled

3. **Benchmarks:**
   - Embedding latency per text
   - Search latency vs index size
   - Memory usage vs vector count

## Dependencies

```toml
[dependencies]
candle-core = "0.8"
candle-nn = "0.8"
candle-transformers = "0.8"
tokenizers = "0.20"
usearch = "2.x"
```

## Success Criteria

1. Embedding generation: < 50ms per text (CPU)
2. Vector search: < 10ms for 100k vectors
3. Hybrid search improves MRR vs BM25-only
4. Graceful degradation when index unavailable
5. No data loss on prune/rebuild operations
