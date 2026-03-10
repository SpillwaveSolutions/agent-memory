//! Novelty checking service with opt-in design and fail-open behavior.
//!
//! Per Phase 16 Plan 03: Key design principles:
//! - DISABLED by default (config.enabled = false)
//! - Explicit fallback on any failure (embedder unavailable, index not ready, timeout)
//! - Async check with configurable timeout
//! - Full metrics for observability
//! - NEVER a hard gate - always stores on any failure

use memory_embeddings::{CandleEmbedder, Embedding, EmbeddingModel};
use memory_types::config::DedupConfig;
use memory_types::dedup::InFlightBuffer;
use memory_types::Event;
use memory_vector::{HnswIndex, VectorIndex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing;

/// Metrics for novelty checking.
#[derive(Debug, Default)]
pub struct NoveltyMetrics {
    pub skipped_disabled: AtomicU64,
    pub skipped_no_embedder: AtomicU64,
    pub skipped_no_index: AtomicU64,
    pub skipped_index_not_ready: AtomicU64,
    pub skipped_error: AtomicU64,
    pub skipped_timeout: AtomicU64,
    pub skipped_short_text: AtomicU64,
    pub stored_novel: AtomicU64,
    pub rejected_duplicate: AtomicU64,
}

impl NoveltyMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all counts as a snapshot.
    pub fn snapshot(&self) -> NoveltyMetricsSnapshot {
        NoveltyMetricsSnapshot {
            skipped_disabled: self.skipped_disabled.load(Ordering::Relaxed),
            skipped_no_embedder: self.skipped_no_embedder.load(Ordering::Relaxed),
            skipped_no_index: self.skipped_no_index.load(Ordering::Relaxed),
            skipped_index_not_ready: self.skipped_index_not_ready.load(Ordering::Relaxed),
            skipped_error: self.skipped_error.load(Ordering::Relaxed),
            skipped_timeout: self.skipped_timeout.load(Ordering::Relaxed),
            skipped_short_text: self.skipped_short_text.load(Ordering::Relaxed),
            stored_novel: self.stored_novel.load(Ordering::Relaxed),
            rejected_duplicate: self.rejected_duplicate.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of novelty metrics.
#[derive(Debug, Clone)]
pub struct NoveltyMetricsSnapshot {
    pub skipped_disabled: u64,
    pub skipped_no_embedder: u64,
    pub skipped_no_index: u64,
    pub skipped_index_not_ready: u64,
    pub skipped_error: u64,
    pub skipped_timeout: u64,
    pub skipped_short_text: u64,
    pub stored_novel: u64,
    pub rejected_duplicate: u64,
}

impl NoveltyMetricsSnapshot {
    /// Total events that were stored (novel + all skipped).
    pub fn total_stored(&self) -> u64 {
        self.stored_novel
            + self.skipped_disabled
            + self.skipped_no_embedder
            + self.skipped_no_index
            + self.skipped_index_not_ready
            + self.skipped_error
            + self.skipped_timeout
            + self.skipped_short_text
    }

    /// Total events checked (novel + rejected).
    pub fn total_checked(&self) -> u64 {
        self.stored_novel + self.rejected_duplicate
    }

    /// Total events rejected.
    pub fn total_rejected(&self) -> u64 {
        self.rejected_duplicate
    }
}

/// Trait for embedder (to allow mocking).
#[async_trait::async_trait]
pub trait EmbedderTrait: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, String>;
}

/// Trait for vector index (to allow mocking).
#[async_trait::async_trait]
pub trait VectorIndexTrait: Send + Sync {
    fn is_ready(&self) -> bool;
    async fn search(&self, embedding: &[f32], top_k: usize) -> Result<Vec<(String, f32)>, String>;
}

/// Adapter that wraps [`CandleEmbedder`] to implement [`EmbedderTrait`].
///
/// Since `CandleEmbedder::embed()` is synchronous and CPU-bound, this adapter
/// uses `tokio::task::spawn_blocking` to avoid blocking the tokio runtime.
pub struct CandleEmbedderAdapter {
    embedder: Arc<CandleEmbedder>,
}

impl CandleEmbedderAdapter {
    /// Create a new adapter wrapping the given embedder.
    pub fn new(embedder: CandleEmbedder) -> Self {
        Self {
            embedder: Arc::new(embedder),
        }
    }
}

#[async_trait::async_trait]
impl EmbedderTrait for CandleEmbedderAdapter {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let embedder = Arc::clone(&self.embedder);
        let text = text.to_string();
        tokio::task::spawn_blocking(move || {
            embedder
                .embed(&text)
                .map(|e| e.values)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| format!("spawn_blocking failed: {e}"))?
    }
}

/// Result of a dedup check, including the embedding for buffer push.
pub struct DedupResult {
    /// True if the event should be stored (is novel or check was skipped).
    pub should_store: bool,
    /// The embedding vector, if one was successfully generated.
    /// Used by the caller to push to the InFlightBuffer after confirmed storage.
    pub embedding: Option<Vec<f32>>,
}

/// Adapter that wraps an [`InFlightBuffer`] to implement [`VectorIndexTrait`].
///
/// This allows the in-flight dedup buffer to be used as the vector index for
/// novelty checking, enabling duplicate detection on recent events before they
/// reach the HNSW index.
pub struct InFlightBufferIndex {
    buffer: Arc<RwLock<InFlightBuffer>>,
}

impl InFlightBufferIndex {
    /// Create a new adapter wrapping the given buffer.
    pub fn new(buffer: Arc<RwLock<InFlightBuffer>>) -> Self {
        Self { buffer }
    }

    /// Access the underlying buffer reference.
    pub fn buffer(&self) -> &Arc<RwLock<InFlightBuffer>> {
        &self.buffer
    }
}

#[async_trait::async_trait]
impl VectorIndexTrait for InFlightBufferIndex {
    fn is_ready(&self) -> bool {
        true // in-memory buffer is always ready
    }

    async fn search(&self, embedding: &[f32], top_k: usize) -> Result<Vec<(String, f32)>, String> {
        if top_k == 0 {
            return Ok(vec![]);
        }

        let guard = self
            .buffer
            .read()
            .map_err(|e| format!("InFlightBuffer lock poisoned: {e}"))?;

        // Use threshold 0.0 to return any match; caller does threshold comparison
        match guard.find_similar(embedding, 0.0) {
            Some((id, score)) => Ok(vec![(id, score)]),
            None => Ok(vec![]),
        }
    }
}

/// Adapter that wraps an [`HnswIndex`] behind `Arc<RwLock<_>>` to implement [`VectorIndexTrait`].
///
/// This enables cross-session duplicate detection by querying the persistent HNSW vector
/// index for events similar to the incoming event (DEDUP-02).
pub struct HnswIndexAdapter {
    index: Arc<std::sync::RwLock<HnswIndex>>,
}

impl HnswIndexAdapter {
    /// Create a new adapter wrapping the given HNSW index.
    pub fn new(index: Arc<std::sync::RwLock<HnswIndex>>) -> Self {
        Self { index }
    }
}

#[async_trait::async_trait]
impl VectorIndexTrait for HnswIndexAdapter {
    fn is_ready(&self) -> bool {
        let guard = match self.index.read() {
            Ok(g) => g,
            Err(_) => return false,
        };
        guard.len() > 0
    }

    async fn search(&self, embedding: &[f32], top_k: usize) -> Result<Vec<(String, f32)>, String> {
        let index = self
            .index
            .read()
            .map_err(|e| format!("HNSW lock poisoned: {e}"))?;
        let query = Embedding::new(embedding.to_vec());
        let results = index
            .search(&query, top_k)
            .map_err(|e| format!("HNSW search failed: {e}"))?;
        // Convert SearchResult { vector_id: u64, score: f32 } to (String, f32)
        // HNSW scores are cosine similarity (1.0 - distance), matching InFlightBuffer polarity.
        Ok(results
            .into_iter()
            .map(|r| (r.vector_id.to_string(), r.score))
            .collect())
    }
}

/// Composite vector index that searches multiple [`VectorIndexTrait`] backends
/// and returns the best (highest similarity) results from any of them.
///
/// Used to combine InFlightBuffer (within-session, fast) with HNSW (cross-session, persistent)
/// for comprehensive duplicate detection.
pub struct CompositeVectorIndex {
    indexes: Vec<Arc<dyn VectorIndexTrait>>,
}

impl CompositeVectorIndex {
    /// Create a new composite index searching the given backends.
    pub fn new(indexes: Vec<Arc<dyn VectorIndexTrait>>) -> Self {
        Self { indexes }
    }
}

#[async_trait::async_trait]
impl VectorIndexTrait for CompositeVectorIndex {
    fn is_ready(&self) -> bool {
        // Ready if ANY index is ready (fail-open: partial availability is fine)
        self.indexes.iter().any(|idx| idx.is_ready())
    }

    async fn search(&self, embedding: &[f32], top_k: usize) -> Result<Vec<(String, f32)>, String> {
        let mut all_results: Vec<(String, f32)> = Vec::new();
        for index in &self.indexes {
            if !index.is_ready() {
                continue; // Skip unavailable indexes gracefully
            }
            match index.search(embedding, top_k).await {
                Ok(results) => all_results.extend(results),
                Err(e) => {
                    // Log but don't fail -- fail-open on individual index errors
                    tracing::warn!(error = %e, "Composite index: one backend failed, continuing");
                }
            }
        }
        // Sort by score descending (highest similarity first) and take top_k
        all_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        all_results.truncate(top_k);
        Ok(all_results)
    }
}

/// Novelty checker with opt-in design and fail-open behavior.
pub struct NoveltyChecker {
    embedder: Option<Arc<dyn EmbedderTrait>>,
    vector_index: Option<Arc<dyn VectorIndexTrait>>,
    config: DedupConfig,
    metrics: Arc<NoveltyMetrics>,
    in_flight_buffer: Option<Arc<RwLock<InFlightBuffer>>>,
}

impl NoveltyChecker {
    /// Create new novelty checker.
    pub fn new(
        embedder: Option<Arc<dyn EmbedderTrait>>,
        vector_index: Option<Arc<dyn VectorIndexTrait>>,
        config: DedupConfig,
    ) -> Self {
        Self {
            embedder,
            vector_index,
            config,
            metrics: Arc::new(NoveltyMetrics::new()),
            in_flight_buffer: None,
        }
    }

    /// Create a novelty checker wired to an in-flight buffer for dedup.
    ///
    /// The buffer is used both as the vector index (via [`InFlightBufferIndex`])
    /// and stored for [`push_to_buffer`](Self::push_to_buffer) after novel events.
    /// This is the constructor Phase 36 will use to integrate into the ingest pipeline.
    pub fn with_in_flight_buffer(
        embedder: Option<Arc<dyn EmbedderTrait>>,
        buffer: Arc<RwLock<InFlightBuffer>>,
        config: DedupConfig,
    ) -> Self {
        let index = Arc::new(InFlightBufferIndex::new(Arc::clone(&buffer)));
        Self {
            embedder,
            vector_index: Some(index as Arc<dyn VectorIndexTrait>),
            config,
            metrics: Arc::new(NoveltyMetrics::new()),
            in_flight_buffer: Some(buffer),
        }
    }

    /// Create a novelty checker with a composite index (InFlightBuffer + HNSW) for cross-session dedup.
    ///
    /// The buffer is used for push_to_buffer after novel events.
    /// The composite index searches both InFlightBuffer and HNSW, returning the best match.
    pub fn with_composite_index(
        embedder: Option<Arc<dyn EmbedderTrait>>,
        buffer: Arc<RwLock<InFlightBuffer>>,
        hnsw_index: Arc<std::sync::RwLock<HnswIndex>>,
        config: DedupConfig,
    ) -> Self {
        let buffer_index =
            Arc::new(InFlightBufferIndex::new(Arc::clone(&buffer))) as Arc<dyn VectorIndexTrait>;
        let hnsw_adapter = Arc::new(HnswIndexAdapter::new(hnsw_index)) as Arc<dyn VectorIndexTrait>;
        let composite = Arc::new(CompositeVectorIndex::new(vec![buffer_index, hnsw_adapter]))
            as Arc<dyn VectorIndexTrait>;
        Self {
            embedder,
            vector_index: Some(composite),
            config,
            metrics: Arc::new(NoveltyMetrics::new()),
            in_flight_buffer: Some(buffer),
        }
    }

    /// Get metrics for this checker.
    pub fn metrics(&self) -> Arc<NoveltyMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Push an embedding into the in-flight buffer for future dedup checks.
    ///
    /// Called by the ingest pipeline AFTER an event is confirmed stored.
    /// Does nothing if no in-flight buffer is configured.
    pub fn push_to_buffer(&self, event_id: &str, embedding: &[f32]) {
        if let Some(ref buffer) = self.in_flight_buffer {
            if let Ok(mut guard) = buffer.write() {
                guard.push(event_id.to_string(), embedding.to_vec());
            } else {
                tracing::warn!("InFlightBuffer lock poisoned, skipping push");
            }
        }
    }

    /// Check if event should be stored (novel or check skipped).
    ///
    /// Returns true if event should be stored:
    /// - Feature disabled -> true (store)
    /// - Embedder unavailable -> true (store)
    /// - Index unavailable or not ready -> true (store)
    /// - Timeout -> true (store)
    /// - Error -> true (store)
    /// - Below similarity threshold -> true (store, is novel)
    /// - Above similarity threshold -> false (reject, is duplicate)
    pub async fn should_store(&self, event: &Event) -> bool {
        self.should_store_with_embedding(event).await.should_store
    }

    /// Check if event should be stored, returning the embedding alongside the decision.
    ///
    /// Same logic as [`should_store`](Self::should_store) but returns a [`DedupResult`]
    /// that includes the embedding vector (if one was generated). The caller can use
    /// the embedding to push to the InFlightBuffer after confirmed storage.
    pub async fn should_store_with_embedding(&self, event: &Event) -> DedupResult {
        // GATE 1: Feature must be explicitly enabled
        if !self.config.enabled {
            self.metrics
                .skipped_disabled
                .fetch_add(1, Ordering::Relaxed);
            return DedupResult {
                should_store: true,
                embedding: None,
            };
        }

        // GATE 2: Skip very short text
        if event.text.len() < self.config.min_text_length {
            self.metrics
                .skipped_short_text
                .fetch_add(1, Ordering::Relaxed);
            tracing::debug!(
                text_len = event.text.len(),
                min_len = self.config.min_text_length,
                "Novelty check skipped: text too short"
            );
            return DedupResult {
                should_store: true,
                embedding: None,
            };
        }

        // GATE 3: Embedder must be available
        let embedder = match &self.embedder {
            Some(e) => e,
            None => {
                self.metrics
                    .skipped_no_embedder
                    .fetch_add(1, Ordering::Relaxed);
                tracing::debug!("Novelty check skipped: embedder unavailable");
                return DedupResult {
                    should_store: true,
                    embedding: None,
                };
            }
        };

        // GATE 4: Vector index must be available and ready
        let index = match &self.vector_index {
            Some(i) => i,
            None => {
                self.metrics
                    .skipped_no_index
                    .fetch_add(1, Ordering::Relaxed);
                tracing::debug!("Novelty check skipped: vector index unavailable");
                return DedupResult {
                    should_store: true,
                    embedding: None,
                };
            }
        };

        if !index.is_ready() {
            self.metrics
                .skipped_index_not_ready
                .fetch_add(1, Ordering::Relaxed);
            tracing::debug!("Novelty check skipped: vector index not ready");
            return DedupResult {
                should_store: true,
                embedding: None,
            };
        }

        // GATE 5: Check must complete within timeout
        let start = Instant::now();
        let timeout_duration = Duration::from_millis(self.config.timeout_ms);

        match timeout(
            timeout_duration,
            self.check_similarity(&event.text, embedder, index),
        )
        .await
        {
            Ok(Ok((is_novel, embedding))) => {
                let elapsed = start.elapsed();
                tracing::debug!(
                    elapsed_ms = elapsed.as_millis(),
                    is_novel,
                    "Novelty check completed"
                );

                if is_novel {
                    self.metrics.stored_novel.fetch_add(1, Ordering::Relaxed);
                    DedupResult {
                        should_store: true,
                        embedding: Some(embedding),
                    }
                } else {
                    self.metrics
                        .rejected_duplicate
                        .fetch_add(1, Ordering::Relaxed);
                    tracing::info!(event_id = %event.event_id, "Novelty check rejected duplicate");
                    DedupResult {
                        should_store: false,
                        embedding: Some(embedding),
                    }
                }
            }
            Ok(Err(e)) => {
                self.metrics.skipped_error.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(?e, "Novelty check failed, storing anyway");
                DedupResult {
                    should_store: true,
                    embedding: None,
                }
            }
            Err(_) => {
                self.metrics.skipped_timeout.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(
                    timeout_ms = self.config.timeout_ms,
                    "Novelty check timed out, storing anyway"
                );
                DedupResult {
                    should_store: true,
                    embedding: None,
                }
            }
        }
    }

    /// Internal similarity check — returns (is_novel, embedding).
    async fn check_similarity(
        &self,
        text: &str,
        embedder: &Arc<dyn EmbedderTrait>,
        index: &Arc<dyn VectorIndexTrait>,
    ) -> Result<(bool, Vec<f32>), String> {
        // Generate embedding
        let embedding = embedder.embed(text).await?;

        // Search for similar
        let results = index.search(&embedding, 1).await?;

        // Check if most similar is above threshold
        let is_novel = if let Some((_, score)) = results.first() {
            *score <= self.config.threshold
        } else {
            // No similar documents found - is novel
            true
        };

        Ok((is_novel, embedding))
    }

    /// Get configuration.
    pub fn config(&self) -> &DedupConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_types::{EventRole, EventType};

    fn test_event(text: &str) -> Event {
        Event {
            event_id: "test-event-1".to_string(),
            session_id: "test-session".to_string(),
            timestamp: chrono::Utc::now(),
            event_type: EventType::UserMessage,
            role: EventRole::User,
            text: text.to_string(),
            metadata: Default::default(),
            agent: None,
        }
    }

    #[tokio::test]
    async fn test_disabled_by_default_returns_true() {
        let config = DedupConfig::default();
        assert!(!config.enabled);

        let checker = NoveltyChecker::new(None, None, config);
        let event = test_event("This is a test event with enough text to pass length check");

        assert!(checker.should_store(&event).await);

        let metrics = checker.metrics().snapshot();
        assert_eq!(metrics.skipped_disabled, 1);
    }

    #[tokio::test]
    async fn test_skips_short_text() {
        let config = DedupConfig {
            enabled: true,
            min_text_length: 100,
            ..Default::default()
        };

        let checker = NoveltyChecker::new(None, None, config);
        let event = test_event("Short text");

        assert!(checker.should_store(&event).await);

        let metrics = checker.metrics().snapshot();
        assert_eq!(metrics.skipped_short_text, 1);
    }

    #[tokio::test]
    async fn test_skips_when_no_embedder() {
        let config = DedupConfig {
            enabled: true,
            min_text_length: 10,
            ..Default::default()
        };

        let checker = NoveltyChecker::new(None, None, config);
        let event = test_event("This is a test event with enough text");

        assert!(checker.should_store(&event).await);

        let metrics = checker.metrics().snapshot();
        assert_eq!(metrics.skipped_no_embedder, 1);
    }

    #[tokio::test]
    async fn test_metrics_snapshot_totals() {
        let config = DedupConfig::default();
        let checker = NoveltyChecker::new(None, None, config);

        // Call twice to get 2 skipped_disabled
        let event = test_event("Test event text");
        checker.should_store(&event).await;
        checker.should_store(&event).await;

        let metrics = checker.metrics().snapshot();
        assert_eq!(metrics.skipped_disabled, 2);
        assert_eq!(metrics.total_stored(), 2);
        assert_eq!(metrics.total_checked(), 0);
        assert_eq!(metrics.total_rejected(), 0);
    }

    // --- Mock types for dedup tests ---

    struct MockEmbedder {
        embedding: Vec<f32>,
    }

    #[async_trait::async_trait]
    impl EmbedderTrait for MockEmbedder {
        async fn embed(&self, _text: &str) -> Result<Vec<f32>, String> {
            Ok(self.embedding.clone())
        }
    }

    struct FailingEmbedder;

    #[async_trait::async_trait]
    impl EmbedderTrait for FailingEmbedder {
        async fn embed(&self, _text: &str) -> Result<Vec<f32>, String> {
            Err("embed failed".to_string())
        }
    }

    struct MockVectorIndex {
        results: Vec<(String, f32)>,
        ready: bool,
    }

    #[async_trait::async_trait]
    impl VectorIndexTrait for MockVectorIndex {
        fn is_ready(&self) -> bool {
            self.ready
        }

        async fn search(
            &self,
            _embedding: &[f32],
            _top_k: usize,
        ) -> Result<Vec<(String, f32)>, String> {
            Ok(self.results.clone())
        }
    }

    /// Helper: create a normalized uniform vector of given dimension.
    fn uniform_normalized(dim: usize) -> Vec<f32> {
        let val = 1.0 / (dim as f32).sqrt();
        vec![val; dim]
    }

    /// Helper: create an enabled DedupConfig for testing.
    fn enabled_config() -> DedupConfig {
        DedupConfig {
            enabled: true,
            threshold: 0.85,
            min_text_length: 10,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_duplicate_detected_via_in_flight_buffer() {
        let dim = 384;
        let vec_a = uniform_normalized(dim);

        // Pre-populate buffer with one entry
        let buffer = Arc::new(RwLock::new(InFlightBuffer::new(256, dim)));
        {
            let mut guard = buffer.write().unwrap();
            guard.push("original".to_string(), vec_a.clone());
        }

        let embedder: Arc<dyn EmbedderTrait> = Arc::new(MockEmbedder { embedding: vec_a });
        let checker = NoveltyChecker::with_in_flight_buffer(
            Some(embedder),
            Arc::clone(&buffer),
            enabled_config(),
        );

        let event = test_event("This is a test event that should be detected as duplicate");
        // Same embedding -> similarity ~1.0 > 0.85 threshold -> duplicate
        assert!(!checker.should_store(&event).await);

        let metrics = checker.metrics().snapshot();
        assert_eq!(metrics.rejected_duplicate, 1);
    }

    #[tokio::test]
    async fn test_novel_event_passes_through() {
        let dim = 384;
        // Vector A: first half positive, second half zero
        let mut vec_a = vec![0.0f32; dim];
        for v in vec_a.iter_mut().take(dim / 2) {
            *v = 1.0 / ((dim / 2) as f32).sqrt();
        }

        // Vector B: first half zero, second half positive (orthogonal to A)
        let mut vec_b = vec![0.0f32; dim];
        for v in vec_b.iter_mut().skip(dim / 2) {
            *v = 1.0 / ((dim / 2) as f32).sqrt();
        }

        let buffer = Arc::new(RwLock::new(InFlightBuffer::new(256, dim)));
        {
            let mut guard = buffer.write().unwrap();
            guard.push("existing".to_string(), vec_a);
        }

        let embedder: Arc<dyn EmbedderTrait> = Arc::new(MockEmbedder { embedding: vec_b });
        let checker = NoveltyChecker::with_in_flight_buffer(
            Some(embedder),
            Arc::clone(&buffer),
            enabled_config(),
        );

        let event = test_event("This is a novel event that should pass through");
        // Orthogonal vectors -> similarity ~0.0 < 0.85 -> novel
        assert!(checker.should_store(&event).await);

        let metrics = checker.metrics().snapshot();
        assert_eq!(metrics.stored_novel, 1);
    }

    #[tokio::test]
    async fn test_fail_open_on_embedder_error() {
        let buffer = Arc::new(RwLock::new(InFlightBuffer::new(256, 384)));
        let embedder: Arc<dyn EmbedderTrait> = Arc::new(FailingEmbedder);
        let checker = NoveltyChecker::with_in_flight_buffer(
            Some(embedder),
            Arc::clone(&buffer),
            enabled_config(),
        );

        let event = test_event("This event has a failing embedder so should store anyway");
        assert!(checker.should_store(&event).await);

        let metrics = checker.metrics().snapshot();
        assert_eq!(metrics.skipped_error, 1);
    }

    #[tokio::test]
    async fn test_fail_open_when_no_index() {
        let embedder: Arc<dyn EmbedderTrait> = Arc::new(MockEmbedder {
            embedding: uniform_normalized(384),
        });
        // No vector_index, no in_flight_buffer
        let checker = NoveltyChecker::new(Some(embedder), None, enabled_config());

        let event = test_event("This event should store because no index is available");
        assert!(checker.should_store(&event).await);

        let metrics = checker.metrics().snapshot();
        assert_eq!(metrics.skipped_no_index, 1);
    }

    #[tokio::test]
    async fn test_fail_open_when_index_not_ready() {
        let embedder: Arc<dyn EmbedderTrait> = Arc::new(MockEmbedder {
            embedding: uniform_normalized(384),
        });
        let index: Arc<dyn VectorIndexTrait> = Arc::new(MockVectorIndex {
            results: vec![],
            ready: false,
        });
        let checker = NoveltyChecker::new(Some(embedder), Some(index), enabled_config());

        let event = test_event("This event should store because index is not ready");
        assert!(checker.should_store(&event).await);

        let metrics = checker.metrics().snapshot();
        assert_eq!(metrics.skipped_index_not_ready, 1);
    }

    #[tokio::test]
    async fn test_push_to_buffer_populates_for_next_check() {
        let dim = 384;
        let vec_v = uniform_normalized(dim);

        let buffer = Arc::new(RwLock::new(InFlightBuffer::new(256, dim)));
        let embedder: Arc<dyn EmbedderTrait> = Arc::new(MockEmbedder {
            embedding: vec_v.clone(),
        });
        let checker = NoveltyChecker::with_in_flight_buffer(
            Some(embedder),
            Arc::clone(&buffer),
            enabled_config(),
        );

        let event = test_event("First event should be novel since buffer is empty");

        // First call: buffer empty -> novel
        assert!(checker.should_store(&event).await);

        // Push the embedding after storing
        checker.push_to_buffer("evt-1", &vec_v);

        // Second call: buffer now has the same vector -> duplicate
        assert!(!checker.should_store(&event).await);

        let metrics = checker.metrics().snapshot();
        assert_eq!(metrics.stored_novel, 1);
        assert_eq!(metrics.rejected_duplicate, 1);
    }

    #[tokio::test]
    async fn test_empty_buffer_always_novel() {
        let dim = 384;
        let buffer = Arc::new(RwLock::new(InFlightBuffer::new(256, dim)));
        let embedder: Arc<dyn EmbedderTrait> = Arc::new(MockEmbedder {
            embedding: uniform_normalized(dim),
        });
        let checker = NoveltyChecker::with_in_flight_buffer(
            Some(embedder),
            Arc::clone(&buffer),
            enabled_config(),
        );

        let event = test_event("Any event against an empty buffer should be novel");
        assert!(checker.should_store(&event).await);

        let metrics = checker.metrics().snapshot();
        assert_eq!(metrics.stored_novel, 1);
    }

    // --- CompositeVectorIndex and HnswIndexAdapter tests ---

    struct FailingVectorIndex;

    #[async_trait::async_trait]
    impl VectorIndexTrait for FailingVectorIndex {
        fn is_ready(&self) -> bool {
            true
        }

        async fn search(
            &self,
            _embedding: &[f32],
            _top_k: usize,
        ) -> Result<Vec<(String, f32)>, String> {
            Err("index failed".to_string())
        }
    }

    #[tokio::test]
    async fn test_composite_returns_highest_scoring_result() {
        let idx_a: Arc<dyn VectorIndexTrait> = Arc::new(MockVectorIndex {
            results: vec![("a".to_string(), 0.7)],
            ready: true,
        });
        let idx_b: Arc<dyn VectorIndexTrait> = Arc::new(MockVectorIndex {
            results: vec![("b".to_string(), 0.95)],
            ready: true,
        });
        let composite = CompositeVectorIndex::new(vec![idx_a, idx_b]);

        let results = composite.search(&[0.1, 0.2], 1).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "b");
        assert!((results[0].1 - 0.95).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_composite_returns_all_results_when_top_k_large() {
        let idx_a: Arc<dyn VectorIndexTrait> = Arc::new(MockVectorIndex {
            results: vec![("a".to_string(), 0.7)],
            ready: true,
        });
        let idx_b: Arc<dyn VectorIndexTrait> = Arc::new(MockVectorIndex {
            results: vec![("b".to_string(), 0.95)],
            ready: true,
        });
        let composite = CompositeVectorIndex::new(vec![idx_a, idx_b]);

        let results = composite.search(&[0.1, 0.2], 10).await.unwrap();
        assert_eq!(results.len(), 2);
        // Sorted by score descending
        assert_eq!(results[0].0, "b");
        assert_eq!(results[1].0, "a");
    }

    #[tokio::test]
    async fn test_composite_gracefully_handles_one_failing_backend() {
        let good: Arc<dyn VectorIndexTrait> = Arc::new(MockVectorIndex {
            results: vec![("good".to_string(), 0.8)],
            ready: true,
        });
        let bad: Arc<dyn VectorIndexTrait> = Arc::new(FailingVectorIndex);
        let composite = CompositeVectorIndex::new(vec![bad, good]);

        let results = composite.search(&[0.1, 0.2], 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "good");
    }

    #[tokio::test]
    async fn test_composite_is_ready_when_any_index_ready() {
        let ready: Arc<dyn VectorIndexTrait> = Arc::new(MockVectorIndex {
            results: vec![],
            ready: true,
        });
        let not_ready: Arc<dyn VectorIndexTrait> = Arc::new(MockVectorIndex {
            results: vec![],
            ready: false,
        });
        let composite = CompositeVectorIndex::new(vec![not_ready, ready]);
        assert!(composite.is_ready());
    }

    #[tokio::test]
    async fn test_composite_not_ready_when_no_index_ready() {
        let not_ready_a: Arc<dyn VectorIndexTrait> = Arc::new(MockVectorIndex {
            results: vec![],
            ready: false,
        });
        let not_ready_b: Arc<dyn VectorIndexTrait> = Arc::new(MockVectorIndex {
            results: vec![],
            ready: false,
        });
        let composite = CompositeVectorIndex::new(vec![not_ready_a, not_ready_b]);
        assert!(!composite.is_ready());
    }

    #[tokio::test]
    async fn test_composite_skips_not_ready_indexes() {
        let ready: Arc<dyn VectorIndexTrait> = Arc::new(MockVectorIndex {
            results: vec![("ready".to_string(), 0.9)],
            ready: true,
        });
        let not_ready: Arc<dyn VectorIndexTrait> = Arc::new(MockVectorIndex {
            results: vec![("should_not_appear".to_string(), 1.0)],
            ready: false,
        });
        let composite = CompositeVectorIndex::new(vec![not_ready, ready]);

        let results = composite.search(&[0.1], 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "ready");
    }

    #[tokio::test]
    async fn test_hnsw_adapter_not_ready_when_empty() {
        let dir = tempfile::tempdir().unwrap();
        let config = memory_vector::HnswConfig::new(384, dir.path());
        let hnsw = HnswIndex::open_or_create(config).unwrap();
        let hnsw = Arc::new(std::sync::RwLock::new(hnsw));
        let adapter = HnswIndexAdapter::new(hnsw);
        assert!(!adapter.is_ready());
    }

    #[tokio::test]
    async fn test_hnsw_adapter_ready_when_has_vectors() {
        let dir = tempfile::tempdir().unwrap();
        let config = memory_vector::HnswConfig::new(4, dir.path());
        let mut hnsw = HnswIndex::open_or_create(config).unwrap();
        let embedding = Embedding::new(vec![0.5, 0.5, 0.5, 0.5]);
        hnsw.add(1, &embedding).unwrap();
        let hnsw = Arc::new(std::sync::RwLock::new(hnsw));
        let adapter = HnswIndexAdapter::new(hnsw);
        assert!(adapter.is_ready());
    }

    #[tokio::test]
    async fn test_hnsw_adapter_search_returns_results() {
        let dir = tempfile::tempdir().unwrap();
        let config = memory_vector::HnswConfig::new(4, dir.path());
        let mut hnsw = HnswIndex::open_or_create(config).unwrap();
        let embedding = Embedding::new(vec![0.5, 0.5, 0.5, 0.5]);
        hnsw.add(42, &embedding).unwrap();
        let hnsw = Arc::new(std::sync::RwLock::new(hnsw));
        let adapter = HnswIndexAdapter::new(hnsw);

        let results = adapter.search(&[0.5, 0.5, 0.5, 0.5], 1).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "42");
        // Same vector should have high similarity (close to 1.0)
        assert!(results[0].1 > 0.9);
    }
}
