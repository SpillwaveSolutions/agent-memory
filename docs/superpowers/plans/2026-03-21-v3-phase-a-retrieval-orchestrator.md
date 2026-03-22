# Phase A: Retrieval Orchestrator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `memory-orchestrator` crate that adds query expansion, RRF fusion across all four indexes, and LLM reranking on top of the existing `memory-retrieval` executor — no existing crates modified.

**Architecture:** `memory-orchestrator` wraps `RetrievalExecutor` from `memory-retrieval` for each index layer, then applies Reciprocal Rank Fusion across the four `SearchResult` lists. Query expansion runs before fan-out; reranking runs after fusion. The `ContextBuilder` converts ranked results into a structured JSON-ready `MemoryContext` struct.

**Tech Stack:** Rust 2021, tokio async, `memory-retrieval` (RetrievalExecutor, SearchResult, LayerExecutor), `memory-client` (for LLM rerank gRPC), `serde_json`, `async-trait`, `thiserror`, `anyhow`

**Spec:** `docs/superpowers/specs/2026-03-21-v3-competitive-parity-design.md` — Phase A section

---

## File Map

**New crate:** `crates/memory-orchestrator/`

| File | Responsibility |
|------|----------------|
| `crates/memory-orchestrator/Cargo.toml` | Crate manifest, workspace deps |
| `crates/memory-orchestrator/src/lib.rs` | Public API re-exports, crate docs |
| `crates/memory-orchestrator/src/expand.rs` | Query expansion: heuristic variants |
| `crates/memory-orchestrator/src/fusion.rs` | RRF implementation over N result lists |
| `crates/memory-orchestrator/src/rerank.rs` | `Reranker` trait + `HeuristicReranker` + `LlmReranker` |
| `crates/memory-orchestrator/src/context_builder.rs` | `ContextBuilder`: ranked results → `MemoryContext` |
| `crates/memory-orchestrator/src/orchestrator.rs` | `MemoryOrchestrator`: wires all stages together |
| `crates/memory-orchestrator/src/types.rs` | `OrchestratorConfig`, `MemoryContext`, `RankedResult` |

**Modified files:**
| File | Change |
|------|--------|
| `Cargo.toml` (workspace root) | Add `memory-orchestrator` to `members` and `[workspace.dependencies]` |

---

## Task 1: Scaffold the crate

**Files:**
- Create: `crates/memory-orchestrator/Cargo.toml`
- Create: `crates/memory-orchestrator/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "memory-orchestrator"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
memory-retrieval = { workspace = true }
memory-client = { workspace = true }
memory-types = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
async-trait = { workspace = true }
futures = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio = { workspace = true, features = ["test-util"] }
```

- [ ] **Step 2: Add to workspace `Cargo.toml`**

In `Cargo.toml` (root), add to `members`:
```toml
"crates/memory-orchestrator",
```

And to `[workspace.dependencies]`:
```toml
memory-orchestrator = { path = "crates/memory-orchestrator" }
```

- [ ] **Step 3: Create `src/lib.rs` stub**

```rust
//! # memory-orchestrator
//!
//! Retrieval orchestration layer for agent-memory.
//!
//! Adds query expansion, RRF fusion across all four indexes,
//! and optional LLM reranking on top of `memory-retrieval`.

pub mod context_builder;
pub mod expand;
pub mod fusion;
pub mod orchestrator;
pub mod rerank;
pub mod types;

pub use orchestrator::MemoryOrchestrator;
pub use types::{MemoryContext, OrchestratorConfig, RankedResult, RerankMode};
```

- [ ] **Step 4: Verify crate compiles**

```bash
cargo build -p memory-orchestrator
```

Expected: compiles (empty modules, no errors)

- [ ] **Step 5: Commit**

```bash
git add crates/memory-orchestrator/ Cargo.toml Cargo.lock
git commit -m "feat(orchestrator): scaffold memory-orchestrator crate"
```

---

## Task 2: Define core types

**Files:**
- Create: `crates/memory-orchestrator/src/types.rs`

- [ ] **Step 1: Write the types test first**

In `crates/memory-orchestrator/src/types.rs`, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rerank_mode_default() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.rerank_mode, RerankMode::Heuristic);
        assert_eq!(config.top_k, 10);
        assert_eq!(config.expand_query, false);
    }

    #[test]
    fn test_ranked_result_ordering() {
        let mut results = vec![
            RankedResult { score: 0.5, doc_id: "a".to_string(), text: "a".to_string(), source_layer: "bm25".to_string(), confidence: 0.5 },
            RankedResult { score: 0.9, doc_id: "b".to_string(), text: "b".to_string(), source_layer: "vector".to_string(), confidence: 0.9 },
        ];
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        assert_eq!(results[0].doc_id, "b");
    }
}
```

- [ ] **Step 2: Run test — verify fails (types not defined)**

```bash
cargo test -p memory-orchestrator
```

Expected: FAIL (types undefined)

- [ ] **Step 3: Implement types**

```rust
use serde::{Deserialize, Serialize};

/// How to rerank results after RRF fusion.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RerankMode {
    /// Use RRF scores as-is (default, zero extra cost).
    #[default]
    Heuristic,
    /// Use configured LLM to reorder top-k candidates.
    Llm,
}

/// Configuration for the orchestrator pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Maximum results to return after reranking.
    pub top_k: usize,
    /// Reranking mode.
    pub rerank_mode: RerankMode,
    /// Whether to expand the query with heuristic variants.
    pub expand_query: bool,
    /// RRF constant k (default: 60, standard literature value).
    pub rrf_k: f64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            top_k: 10,
            rerank_mode: RerankMode::Heuristic,
            expand_query: false,
            rrf_k: 60.0,
        }
    }
}

/// A single result after RRF fusion and reranking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedResult {
    /// Fused score (RRF or reranker output).
    pub score: f64,
    /// Original document ID.
    pub doc_id: String,
    /// Preview text.
    pub text: String,
    /// Which index contributed this result.
    pub source_layer: String,
    /// Normalized confidence [0.0, 1.0].
    pub confidence: f64,
}

/// Structured context ready for prompt injection.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryContext {
    pub summary: String,
    pub relevant_events: Vec<RankedResult>,
    pub key_entities: Vec<String>,
    pub open_questions: Vec<String>,
    pub retrieval_ms: u64,
    pub tokens_estimated: usize,
    pub confidence: f64,
}
```

- [ ] **Step 4: Run test — verify passes**

```bash
cargo test -p memory-orchestrator types
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/memory-orchestrator/src/types.rs
git commit -m "feat(orchestrator): add core types (OrchestratorConfig, RankedResult, MemoryContext)"
```

---

## Task 3: Implement RRF fusion

**Files:**
- Create: `crates/memory-orchestrator/src/fusion.rs`

Reciprocal Rank Fusion: for each document, sum `1 / (k + rank)` across all lists where it appears. Higher score = better.

- [ ] **Step 1: Write fusion tests first**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(id: &str, score: f64, layer: &str) -> SearchResult {
        SearchResult {
            doc_id: id.to_string(),
            doc_type: "toc_node".to_string(),
            score,
            text_preview: id.to_string(),
            source_layer: memory_retrieval::types::RetrievalLayer::BM25,
            metadata: Default::default(),
        }
    }

    #[test]
    fn test_rrf_single_list_preserves_order() {
        let list = vec![
            make_result("a", 0.9, "bm25"),
            make_result("b", 0.5, "bm25"),
            make_result("c", 0.1, "bm25"),
        ];
        let fused = rrf_fuse(vec![list], 60.0);
        assert_eq!(fused[0].doc_id, "a");
        assert_eq!(fused[1].doc_id, "b");
        assert_eq!(fused[2].doc_id, "c");
    }

    #[test]
    fn test_rrf_consensus_boosts_result() {
        // "b" appears in all three lists at rank 1 → should win despite lower individual scores
        let list1 = vec![make_result("a", 0.95, "bm25"), make_result("b", 0.8, "bm25")];
        let list2 = vec![make_result("b", 0.8, "vector"), make_result("c", 0.7, "vector")];
        let list3 = vec![make_result("b", 0.75, "graph"), make_result("d", 0.9, "graph")];
        let fused = rrf_fuse(vec![list1, list2, list3], 60.0);
        assert_eq!(fused[0].doc_id, "b", "consensus item should rank highest");
    }

    #[test]
    fn test_rrf_empty_lists_handled() {
        let fused = rrf_fuse(vec![vec![], vec![]], 60.0);
        assert!(fused.is_empty());
    }

    #[test]
    fn test_rrf_deduplicates_same_doc() {
        let list1 = vec![make_result("a", 0.9, "bm25"), make_result("a", 0.5, "bm25")];
        let fused = rrf_fuse(vec![list1], 60.0);
        let count = fused.iter().filter(|r| r.doc_id == "a").count();
        assert_eq!(count, 1, "same doc_id should appear once after fusion");
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cargo test -p memory-orchestrator fusion
```

Expected: FAIL

- [ ] **Step 3: Implement `rrf_fuse`**

```rust
use memory_retrieval::executor::SearchResult;
use std::collections::HashMap;

/// Reciprocal Rank Fusion over multiple ranked result lists.
///
/// For each document across all lists, computes: Σ 1/(k + rank_i)
/// where rank_i is the 1-based position in list i.
/// Documents not in a list contribute 0 for that list.
pub fn rrf_fuse(lists: Vec<Vec<SearchResult>>, k: f64) -> Vec<FusedResult> {
    // Map doc_id → accumulated RRF score + best preview
    let mut scores: HashMap<String, (f64, SearchResult)> = HashMap::new();

    for list in &lists {
        for (rank, result) in list.iter().enumerate() {
            let rrf_score = 1.0 / (k + (rank + 1) as f64);
            scores
                .entry(result.doc_id.clone())
                .and_modify(|(s, _)| *s += rrf_score)
                .or_insert((rrf_score, result.clone()));
        }
    }

    let mut fused: Vec<FusedResult> = scores
        .into_values()
        .map(|(score, result)| FusedResult { rrf_score: score, inner: result })
        .collect();

    fused.sort_by(|a, b| b.rrf_score.partial_cmp(&a.rrf_score).unwrap_or(std::cmp::Ordering::Equal));
    fused
}

/// A search result annotated with its RRF score.
#[derive(Debug, Clone)]
pub struct FusedResult {
    pub rrf_score: f64,
    pub inner: SearchResult,
}
```

- [ ] **Step 4: Run tests — verify all pass**

```bash
cargo test -p memory-orchestrator fusion
```

Expected: 4 tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/memory-orchestrator/src/fusion.rs
git commit -m "feat(orchestrator): implement RRF fusion with deduplication"
```

---

## Task 4: Implement query expansion

**Files:**
- Create: `crates/memory-orchestrator/src/expand.rs`

Heuristic expansion: lowercase, strip punctuation, generate simple variants (plural/singular, tense). Returns original query plus 1-2 variants.

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expansion_always_includes_original() {
        let expanded = expand_query("JWT authentication bug");
        assert!(expanded.contains(&"JWT authentication bug".to_string()));
    }

    #[test]
    fn test_expansion_returns_multiple_variants() {
        let expanded = expand_query("what did we decide");
        assert!(expanded.len() >= 2);
    }

    #[test]
    fn test_expansion_empty_query() {
        let expanded = expand_query("");
        assert_eq!(expanded, vec!["".to_string()]);
    }
}
```

- [ ] **Step 2: Run tests — verify fail**

```bash
cargo test -p memory-orchestrator expand
```

- [ ] **Step 3: Implement**

```rust
/// Expand a query into 1-3 heuristic variants.
///
/// Always includes the original. Adds simple rewrites:
/// - lowercase variant if original has uppercase
/// - "we" → "I" substitution for self-referential queries
/// - drops leading question words for keyword bias
pub fn expand_query(query: &str) -> Vec<String> {
    if query.is_empty() {
        return vec![query.to_string()];
    }

    let mut variants = vec![query.to_string()];

    // Lowercase variant (helps BM25 match case-insensitive terms)
    let lower = query.to_lowercase();
    if lower != query {
        variants.push(lower.clone());
    }

    // Strip leading question words to produce a keyword-biased variant
    let stripped = lower
        .trim_start_matches("what ")
        .trim_start_matches("how ")
        .trim_start_matches("why ")
        .trim_start_matches("when ")
        .trim_start_matches("did we ")
        .trim_start_matches("do we ")
        .to_string();

    if stripped != lower && !stripped.is_empty() {
        variants.push(stripped);
    }

    variants.dedup();
    variants
}
```

- [ ] **Step 4: Run tests — verify pass**

```bash
cargo test -p memory-orchestrator expand
```

- [ ] **Step 5: Commit**

```bash
git add crates/memory-orchestrator/src/expand.rs
git commit -m "feat(orchestrator): add heuristic query expansion"
```

---

## Task 5: Implement the reranker trait + impls

**Files:**
- Create: `crates/memory-orchestrator/src/rerank.rs`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fusion::FusedResult;
    use memory_retrieval::executor::SearchResult;
    use memory_retrieval::types::RetrievalLayer;

    fn make_fused(id: &str, score: f64) -> FusedResult {
        FusedResult {
            rrf_score: score,
            inner: SearchResult {
                doc_id: id.to_string(),
                doc_type: "node".to_string(),
                score,
                text_preview: format!("text for {id}"),
                source_layer: RetrievalLayer::BM25,
                metadata: Default::default(),
            },
        }
    }

    #[tokio::test]
    async fn test_heuristic_reranker_preserves_rrf_order() {
        let reranker = HeuristicReranker;
        let input = vec![make_fused("a", 0.9), make_fused("b", 0.5)];
        let result = reranker.rerank("query", input).await.unwrap();
        assert_eq!(result[0].doc_id, "a");
    }

    #[tokio::test]
    async fn test_heuristic_reranker_trims_to_top_k() {
        let reranker = HeuristicReranker;
        let input = (0..20).map(|i| make_fused(&i.to_string(), i as f64)).collect();
        let result = reranker.rerank("query", input).await.unwrap();
        assert!(result.len() <= 10);
    }
}
```

- [ ] **Step 2: Run tests — verify fail**

```bash
cargo test -p memory-orchestrator rerank
```

- [ ] **Step 3: Implement trait + HeuristicReranker**

```rust
use crate::fusion::FusedResult;
use crate::types::RankedResult;
use anyhow::Result;
use async_trait::async_trait;

/// Output of reranking — doc_id + final score.
#[derive(Debug, Clone)]
pub struct RerankedResult {
    pub doc_id: String,
    pub score: f64,
    pub text: String,
    pub source_layer: String,
}

/// Trait for all reranking strategies.
#[async_trait]
pub trait Reranker: Send + Sync {
    /// Rerank fused results for the given query. Returns top results.
    async fn rerank(&self, query: &str, results: Vec<FusedResult>) -> Result<Vec<RerankedResult>>;
}

/// Default reranker: uses RRF score as-is, trims to top 10.
pub struct HeuristicReranker;

#[async_trait]
impl Reranker for HeuristicReranker {
    async fn rerank(&self, _query: &str, mut results: Vec<FusedResult>) -> Result<Vec<RerankedResult>> {
        results.sort_by(|a, b| b.rrf_score.partial_cmp(&a.rrf_score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(results
            .into_iter()
            .take(10)
            .map(|r| RerankedResult {
                doc_id: r.inner.doc_id,
                score: r.rrf_score,
                text: r.inner.text_preview,
                source_layer: format!("{:?}", r.inner.source_layer),
            })
            .collect())
    }
}

/// Extension point for cross-encoder reranker (not implemented in Phase A).
///
/// A cross-encoder requires a different inference path than the bi-encoder
/// in `memory-embeddings` (concatenated query+passage, single logit output).
/// This stub exists so Phase B+ can slot in the implementation without API changes.
pub struct CrossEncoderReranker;

#[async_trait]
impl Reranker for CrossEncoderReranker {
    async fn rerank(&self, query: &str, results: Vec<FusedResult>) -> Result<Vec<RerankedResult>> {
        // Deferred: falls back to heuristic until cross-encoder model is integrated
        tracing::warn!("CrossEncoderReranker not implemented — falling back to heuristic");
        HeuristicReranker.rerank(query, results).await
    }
}
```

- [ ] **Step 4: Run tests — verify pass**

```bash
cargo test -p memory-orchestrator rerank
```

- [ ] **Step 5: Commit**

```bash
git add crates/memory-orchestrator/src/rerank.rs
git commit -m "feat(orchestrator): add Reranker trait, HeuristicReranker, CrossEncoderReranker stub"
```

---

## Task 6: Implement ContextBuilder

**Files:**
- Create: `crates/memory-orchestrator/src/context_builder.rs`

Converts ranked results into a structured `MemoryContext` with summary, events, entities, and token estimate.

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rerank::RerankedResult;

    fn make_reranked(id: &str, text: &str, score: f64) -> RerankedResult {
        RerankedResult {
            doc_id: id.to_string(),
            score,
            text: text.to_string(),
            source_layer: "bm25".to_string(),
        }
    }

    #[test]
    fn test_context_builder_empty_results() {
        let ctx = ContextBuilder::build("query", vec![]);
        assert_eq!(ctx.relevant_events.len(), 0);
        assert_eq!(ctx.confidence, 0.0);
    }

    #[test]
    fn test_context_builder_confidence_from_top_score() {
        let results = vec![make_reranked("a", "text", 0.75)];
        let ctx = ContextBuilder::build("query", results);
        assert!((ctx.confidence - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_context_builder_tokens_estimated_nonzero() {
        let results = vec![make_reranked("a", "hello world", 0.8)];
        let ctx = ContextBuilder::build("query", results);
        assert!(ctx.tokens_estimated > 0);
    }
}
```

- [ ] **Step 2: Run tests — verify fail**

```bash
cargo test -p memory-orchestrator context_builder
```

- [ ] **Step 3: Implement**

```rust
use crate::rerank::RerankedResult;
use crate::types::{MemoryContext, RankedResult};

pub struct ContextBuilder;

impl ContextBuilder {
    /// Convert reranked results into a structured `MemoryContext`.
    pub fn build(query: &str, results: Vec<RerankedResult>) -> MemoryContext {
        let confidence = results.first().map(|r| r.score).unwrap_or(0.0);

        let relevant_events: Vec<RankedResult> = results
            .iter()
            .map(|r| RankedResult {
                score: r.score,
                doc_id: r.doc_id.clone(),
                text: r.text.clone(),
                source_layer: r.source_layer.clone(),
                confidence: r.score,
            })
            .collect();

        // Rough token estimate: ~0.75 tokens per character for English text
        let total_chars: usize = relevant_events.iter().map(|r| r.text.len()).sum();
        let tokens_estimated = (total_chars as f64 * 0.75) as usize + 50; // +50 for envelope overhead

        let summary = if relevant_events.is_empty() {
            "No relevant memory found.".to_string()
        } else {
            format!(
                "Found {} relevant memory entries for: \"{}\"",
                relevant_events.len(),
                query
            )
        };

        MemoryContext {
            summary,
            relevant_events,
            key_entities: vec![],   // Phase C will populate via NER
            open_questions: vec![], // Phase C will populate
            retrieval_ms: 0,        // Set by orchestrator after timing
            tokens_estimated,
            confidence,
        }
    }
}
```

- [ ] **Step 4: Run tests — verify pass**

```bash
cargo test -p memory-orchestrator context_builder
```

- [ ] **Step 5: Commit**

```bash
git add crates/memory-orchestrator/src/context_builder.rs
git commit -m "feat(orchestrator): add ContextBuilder for structured memory context"
```

---

## Task 7: Wire the MemoryOrchestrator

**Files:**
- Create: `crates/memory-orchestrator/src/orchestrator.rs`

- [ ] **Step 1: Write integration tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use memory_retrieval::executor::{MockLayerExecutor, SearchResult};
    use memory_retrieval::types::RetrievalLayer;
    use std::sync::Arc;

    fn mock_result(id: &str, score: f64, layer: RetrievalLayer) -> SearchResult {
        SearchResult {
            doc_id: id.to_string(),
            doc_type: "node".to_string(),
            score,
            text_preview: format!("preview for {id}"),
            source_layer: layer,
            metadata: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_orchestrator_returns_fused_results() {
        // All four layers return results — RRF should merge them
        let mock = Arc::new(
            MockLayerExecutor::default()
                .with_results(RetrievalLayer::BM25, vec![mock_result("doc-1", 0.9, RetrievalLayer::BM25)])
                .with_results(RetrievalLayer::Vector, vec![mock_result("doc-1", 0.8, RetrievalLayer::Vector)])
                .with_results(RetrievalLayer::Graph, vec![mock_result("doc-2", 0.7, RetrievalLayer::Graph)])
                .with_results(RetrievalLayer::Agentic, vec![mock_result("doc-3", 0.6, RetrievalLayer::Agentic)]),
        );

        let orchestrator = MemoryOrchestrator::new(mock, OrchestratorConfig::default());
        let ctx = orchestrator.query("JWT bug fix").await.unwrap();

        assert!(!ctx.relevant_events.is_empty());
        // doc-1 appears in two lists → should have highest RRF score
        assert_eq!(ctx.relevant_events[0].doc_id, "doc-1");
    }

    #[tokio::test]
    async fn test_orchestrator_fail_open_when_one_layer_fails() {
        let mock = Arc::new(
            MockLayerExecutor::default()
                .with_failure(RetrievalLayer::BM25)
                .with_results(RetrievalLayer::Vector, vec![mock_result("doc-a", 0.8, RetrievalLayer::Vector)]),
        );

        let orchestrator = MemoryOrchestrator::new(mock, OrchestratorConfig::default());
        let ctx = orchestrator.query("test").await.unwrap();

        assert!(!ctx.relevant_events.is_empty(), "should return results despite BM25 failure");
    }

    #[tokio::test]
    async fn test_orchestrator_llm_rerank_mode_accepted() {
        let mock = Arc::new(
            MockLayerExecutor::default()
                .with_results(RetrievalLayer::BM25, vec![mock_result("x", 0.9, RetrievalLayer::BM25)]),
        );
        let config = OrchestratorConfig { rerank_mode: RerankMode::Llm, ..Default::default() };
        // LLM reranker requires a client — without one it should fall back gracefully
        let orchestrator = MemoryOrchestrator::new(mock, config);
        let result = orchestrator.query("test").await;
        assert!(result.is_ok(), "should not panic when LLM reranker has no client configured");
    }
}
```

- [ ] **Step 2: Run tests — verify fail**

```bash
cargo test -p memory-orchestrator orchestrator
```

- [ ] **Step 3: Implement `MemoryOrchestrator`**

```rust
use crate::context_builder::ContextBuilder;
use crate::expand::expand_query;
use crate::fusion::rrf_fuse;
use crate::rerank::{HeuristicReranker, Reranker};
use crate::types::{MemoryContext, OrchestratorConfig, RerankMode};
use anyhow::Result;
use memory_retrieval::executor::{FallbackChain, LayerExecutor, RetrievalExecutor, SearchResult};
use memory_retrieval::types::{CapabilityTier, ExecutionMode, QueryIntent, RetrievalLayer, StopConditions};
use std::sync::Arc;
use std::time::Instant;

pub struct MemoryOrchestrator<E: LayerExecutor> {
    executor: Arc<E>,
    config: OrchestratorConfig,
}

impl<E: LayerExecutor + Send + Sync + 'static> MemoryOrchestrator<E> {
    pub fn new(executor: Arc<E>, config: OrchestratorConfig) -> Self {
        Self { executor, config }
    }

    /// Run the full orchestration pipeline for a query.
    pub async fn query(&self, query: &str) -> Result<MemoryContext> {
        let start = Instant::now();

        // 1. Query expansion
        let queries = if self.config.expand_query {
            expand_query(query)
        } else {
            vec![query.to_string()]
        };

        // 2. Fan-out: run each query against each index layer, collect all result lists
        let layers = [
            RetrievalLayer::BM25,
            RetrievalLayer::Vector,
            RetrievalLayer::Graph,
            RetrievalLayer::Agentic, // TOC/Agentic
        ];

        let re = RetrievalExecutor::new(self.executor.clone());
        let mut all_lists: Vec<Vec<SearchResult>> = Vec::new();

        for q in &queries {
            for &layer in &layers {
                let chain = FallbackChain { layers: vec![layer] };
                let conds = StopConditions::default();
                match re.execute(q, chain, &conds, ExecutionMode::Sequential, CapabilityTier::Full).await {
                    r if r.has_results() => all_lists.push(r.results),
                    _ => {} // fail-open: skip empty/failed layers
                }
            }
        }

        // 3. RRF fusion
        let fused = rrf_fuse(all_lists, self.config.rrf_k);

        // 4. Reranking
        let reranker: Box<dyn Reranker> = match self.config.rerank_mode {
            RerankMode::Heuristic => Box::new(HeuristicReranker),
            RerankMode::Llm => {
                // LLM reranker requires external client — fall back to heuristic if unconfigured
                tracing::debug!("LLM rerank requested; using heuristic fallback (client not wired in Phase A)");
                Box::new(HeuristicReranker)
            }
        };

        let reranked = reranker.rerank(query, fused).await?;

        // 5. Build context
        let mut ctx = ContextBuilder::build(query, reranked);
        ctx.retrieval_ms = start.elapsed().as_millis() as u64;

        Ok(ctx)
    }
}
```

- [ ] **Step 4: Run all orchestrator tests**

```bash
cargo test -p memory-orchestrator
```

Expected: all tests PASS

- [ ] **Step 5: Run full workspace check**

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: zero warnings

- [ ] **Step 6: Commit**

```bash
git add crates/memory-orchestrator/src/orchestrator.rs crates/memory-orchestrator/src/lib.rs
git commit -m "feat(orchestrator): wire MemoryOrchestrator with RRF fusion + fail-open fan-out"
```

---

## Task 8: Final QA and Phase A wrap-up

- [ ] **Step 1: Run full test suite**

```bash
cargo test --workspace --all-features
```

Expected: all tests pass

- [ ] **Step 2: Run pr-precheck**

```bash
task pr-precheck
```

Or manually:
```bash
cargo fmt --all -- --check && \
cargo clippy --workspace --all-targets --all-features -- -D warnings && \
cargo test --workspace --all-features && \
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --all-features
```

Expected: all pass

- [ ] **Step 3: Verify Phase A success criteria from spec**

- [ ] RRF unit tests confirm different ranking than single-index input ✓ (Task 3)
- [ ] Fail-open: orchestrator returns results when one index returns empty ✓ (Task 7)
- [ ] LLM rerank mode accepted without panic ✓ (Task 7)
- [ ] `cargo test -p memory-orchestrator` passes, zero clippy warnings ✓

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "feat(phase-a): complete Retrieval Orchestrator — RRF fusion, query expansion, rerank trait"
```
