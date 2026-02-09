//! Novelty checking service with opt-in design and fail-open behavior.
//!
//! Per Phase 16 Plan 03: Key design principles:
//! - DISABLED by default (config.enabled = false)
//! - Explicit fallback on any failure (embedder unavailable, index not ready, timeout)
//! - Async check with configurable timeout
//! - Full metrics for observability
//! - NEVER a hard gate - always stores on any failure

use memory_types::config::NoveltyConfig;
use memory_types::Event;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
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

/// Novelty checker with opt-in design and fail-open behavior.
pub struct NoveltyChecker {
    embedder: Option<Arc<dyn EmbedderTrait>>,
    vector_index: Option<Arc<dyn VectorIndexTrait>>,
    config: NoveltyConfig,
    metrics: Arc<NoveltyMetrics>,
}

impl NoveltyChecker {
    /// Create new novelty checker.
    pub fn new(
        embedder: Option<Arc<dyn EmbedderTrait>>,
        vector_index: Option<Arc<dyn VectorIndexTrait>>,
        config: NoveltyConfig,
    ) -> Self {
        Self {
            embedder,
            vector_index,
            config,
            metrics: Arc::new(NoveltyMetrics::new()),
        }
    }

    /// Get metrics for this checker.
    pub fn metrics(&self) -> Arc<NoveltyMetrics> {
        Arc::clone(&self.metrics)
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
        // GATE 1: Feature must be explicitly enabled
        if !self.config.enabled {
            self.metrics
                .skipped_disabled
                .fetch_add(1, Ordering::Relaxed);
            return true;
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
            return true;
        }

        // GATE 3: Embedder must be available
        let embedder = match &self.embedder {
            Some(e) => e,
            None => {
                self.metrics
                    .skipped_no_embedder
                    .fetch_add(1, Ordering::Relaxed);
                tracing::debug!("Novelty check skipped: embedder unavailable");
                return true;
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
                return true;
            }
        };

        if !index.is_ready() {
            self.metrics
                .skipped_index_not_ready
                .fetch_add(1, Ordering::Relaxed);
            tracing::debug!("Novelty check skipped: vector index not ready");
            return true;
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
            Ok(Ok(is_novel)) => {
                let elapsed = start.elapsed();
                tracing::debug!(
                    elapsed_ms = elapsed.as_millis(),
                    is_novel,
                    "Novelty check completed"
                );

                if is_novel {
                    self.metrics.stored_novel.fetch_add(1, Ordering::Relaxed);
                    true
                } else {
                    self.metrics
                        .rejected_duplicate
                        .fetch_add(1, Ordering::Relaxed);
                    tracing::info!(event_id = %event.event_id, "Novelty check rejected duplicate");
                    false
                }
            }
            Ok(Err(e)) => {
                self.metrics.skipped_error.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(?e, "Novelty check failed, storing anyway");
                true
            }
            Err(_) => {
                self.metrics.skipped_timeout.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(
                    timeout_ms = self.config.timeout_ms,
                    "Novelty check timed out, storing anyway"
                );
                true
            }
        }
    }

    /// Internal similarity check.
    async fn check_similarity(
        &self,
        text: &str,
        embedder: &Arc<dyn EmbedderTrait>,
        index: &Arc<dyn VectorIndexTrait>,
    ) -> Result<bool, String> {
        // Generate embedding
        let embedding = embedder.embed(text).await?;

        // Search for similar
        let results = index.search(&embedding, 1).await?;

        // Check if most similar is above threshold
        if let Some((_, score)) = results.first() {
            Ok(*score <= self.config.threshold)
        } else {
            // No similar documents found - is novel
            Ok(true)
        }
    }

    /// Get configuration.
    pub fn config(&self) -> &NoveltyConfig {
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
        let config = NoveltyConfig::default();
        assert!(!config.enabled);

        let checker = NoveltyChecker::new(None, None, config);
        let event = test_event("This is a test event with enough text to pass length check");

        assert!(checker.should_store(&event).await);

        let metrics = checker.metrics().snapshot();
        assert_eq!(metrics.skipped_disabled, 1);
    }

    #[tokio::test]
    async fn test_skips_short_text() {
        let config = NoveltyConfig {
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
        let config = NoveltyConfig {
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
        let config = NoveltyConfig::default();
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
}
