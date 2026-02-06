//! Tier detection for retrieval capability assessment.
//!
//! This module implements the `TierDetector` which queries layer statuses
//! and determines the available capability tier.
//!
//! Per PRD Section 5.2: Tier Detection Algorithm

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::types::{CapabilityTier, CombinedStatus, LayerStatus, QueryIntent, RetrievalLayer};

/// Result of tier detection including full status.
#[derive(Debug, Clone)]
pub struct TierDetectionResult {
    /// Detected capability tier
    pub tier: CapabilityTier,

    /// Combined status of all layers
    pub status: CombinedStatus,

    /// Time taken for detection
    pub detection_time_ms: u64,

    /// Warnings or issues discovered during detection
    pub warnings: Vec<String>,
}

impl TierDetectionResult {
    /// Get the layer priority order for a given intent.
    ///
    /// Per PRD Section 4.2: Different intents use different layer priorities.
    pub fn get_layer_order(&self, intent: QueryIntent) -> Vec<RetrievalLayer> {
        let mut layers = match intent {
            QueryIntent::Explore => vec![
                RetrievalLayer::Topics,
                RetrievalLayer::Hybrid,
                RetrievalLayer::Vector,
                RetrievalLayer::BM25,
                RetrievalLayer::Agentic,
            ],
            QueryIntent::Answer => vec![
                RetrievalLayer::Hybrid,
                RetrievalLayer::BM25,
                RetrievalLayer::Vector,
                RetrievalLayer::Agentic,
            ],
            QueryIntent::Locate => vec![
                RetrievalLayer::BM25,
                RetrievalLayer::Hybrid,
                RetrievalLayer::Vector,
                RetrievalLayer::Agentic,
            ],
            QueryIntent::TimeBoxed => vec![self.tier.best_layer(), RetrievalLayer::Agentic],
        };

        // Filter to only layers supported by current tier
        layers.retain(|layer| self.tier.supports(*layer));

        // Ensure Agentic is always last if not already
        if !layers.is_empty() && layers.last() != Some(&RetrievalLayer::Agentic) {
            layers.retain(|l| *l != RetrievalLayer::Agentic);
            layers.push(RetrievalLayer::Agentic);
        }

        layers
    }

    /// Check if a specific layer is available.
    pub fn is_layer_available(&self, layer: RetrievalLayer) -> bool {
        self.status.get_layer_status(layer).is_ready()
    }

    /// Get a summary description of the detection result.
    pub fn summary(&self) -> String {
        format!(
            "Tier: {} | BM25: {} | Vector: {} | Topics: {} | Detection: {}ms",
            self.tier.description(),
            if self.status.bm25.is_ready() {
                "ready"
            } else {
                "unavailable"
            },
            if self.status.vector.is_ready() {
                "ready"
            } else {
                "unavailable"
            },
            if self.status.topics.is_ready() {
                "ready"
            } else {
                "unavailable"
            },
            self.detection_time_ms
        )
    }
}

/// Trait for layer status providers.
///
/// Implementations query individual layers for their status.
#[async_trait]
pub trait LayerStatusProvider: Send + Sync {
    /// Get BM25 layer status.
    async fn get_bm25_status(&self) -> Result<LayerStatus, String>;

    /// Get Vector layer status.
    async fn get_vector_status(&self) -> Result<LayerStatus, String>;

    /// Get Topics layer status.
    async fn get_topics_status(&self) -> Result<LayerStatus, String>;
}

/// Tier detector that queries layer statuses and determines capability tier.
///
/// Per PRD Section 5.2: Combined Status Check Pattern
pub struct TierDetector<P: LayerStatusProvider> {
    provider: Arc<P>,
    /// Timeout for status checks
    timeout: Duration,
    /// Cache duration for status results
    cache_duration: Duration,
    /// Cached status
    cached_status: std::sync::Mutex<Option<(CombinedStatus, std::time::Instant)>>,
}

impl<P: LayerStatusProvider> TierDetector<P> {
    /// Create a new tier detector with the given status provider.
    pub fn new(provider: Arc<P>) -> Self {
        Self {
            provider,
            timeout: Duration::from_millis(500),
            cache_duration: Duration::from_secs(30),
            cached_status: std::sync::Mutex::new(None),
        }
    }

    /// Create a tier detector with custom timeout.
    pub fn with_timeout(provider: Arc<P>, timeout: Duration) -> Self {
        Self {
            provider,
            timeout,
            cache_duration: Duration::from_secs(30),
            cached_status: std::sync::Mutex::new(None),
        }
    }

    /// Set the cache duration.
    pub fn with_cache_duration(mut self, duration: Duration) -> Self {
        self.cache_duration = duration;
        self
    }

    /// Detect the current capability tier.
    ///
    /// This performs the combined status check pattern from PRD Section 5.2.
    pub async fn detect(&self) -> TierDetectionResult {
        let start = std::time::Instant::now();

        // Check cache first
        if let Some(cached) = self.get_cached_status() {
            debug!("Using cached tier detection result");
            return TierDetectionResult {
                tier: cached.detect_tier(),
                status: cached,
                detection_time_ms: start.elapsed().as_millis() as u64,
                warnings: vec!["Using cached status".to_string()],
            };
        }

        // Query all layers in parallel
        let (bm25_result, vector_result, topics_result) = tokio::join!(
            self.get_status_with_timeout(StatusType::BM25),
            self.get_status_with_timeout(StatusType::Vector),
            self.get_status_with_timeout(StatusType::Topics),
        );

        let mut warnings = Vec::new();

        // Convert results to LayerStatus, handling errors
        let bm25_status = match bm25_result {
            Ok(status) => status,
            Err(e) => {
                warn!("BM25 status check failed: {}", e);
                warnings.push(format!("BM25 status check failed: {}", e));
                LayerStatus::unhealthy(RetrievalLayer::BM25, &e)
            }
        };

        let vector_status = match vector_result {
            Ok(status) => status,
            Err(e) => {
                warn!("Vector status check failed: {}", e);
                warnings.push(format!("Vector status check failed: {}", e));
                LayerStatus::unhealthy(RetrievalLayer::Vector, &e)
            }
        };

        let topics_status = match topics_result {
            Ok(status) => status,
            Err(e) => {
                warn!("Topics status check failed: {}", e);
                warnings.push(format!("Topics status check failed: {}", e));
                LayerStatus::unhealthy(RetrievalLayer::Topics, &e)
            }
        };

        let combined = CombinedStatus::new(bm25_status, vector_status, topics_status);
        let tier = combined.detect_tier();
        let detection_time = start.elapsed().as_millis() as u64;

        // Update cache
        self.set_cached_status(combined.clone());

        info!(
            tier = ?tier,
            bm25_ready = combined.bm25.is_ready(),
            vector_ready = combined.vector.is_ready(),
            topics_ready = combined.topics.is_ready(),
            detection_time_ms = detection_time,
            "Tier detection complete"
        );

        TierDetectionResult {
            tier,
            status: combined,
            detection_time_ms: detection_time,
            warnings,
        }
    }

    /// Force a fresh detection, bypassing the cache.
    pub async fn detect_fresh(&self) -> TierDetectionResult {
        self.invalidate_cache();
        self.detect().await
    }

    /// Invalidate the cached status.
    pub fn invalidate_cache(&self) {
        if let Ok(mut cache) = self.cached_status.lock() {
            *cache = None;
        }
    }

    async fn get_status_with_timeout(
        &self,
        status_type: StatusType,
    ) -> Result<LayerStatus, String> {
        let timeout_result = tokio::time::timeout(self.timeout, self.get_status(status_type)).await;

        match timeout_result {
            Ok(result) => result,
            Err(_) => Err(format!("{:?} status check timed out", status_type)),
        }
    }

    async fn get_status(&self, status_type: StatusType) -> Result<LayerStatus, String> {
        match status_type {
            StatusType::BM25 => self.provider.get_bm25_status().await,
            StatusType::Vector => self.provider.get_vector_status().await,
            StatusType::Topics => self.provider.get_topics_status().await,
        }
    }

    fn get_cached_status(&self) -> Option<CombinedStatus> {
        if let Ok(cache) = self.cached_status.lock() {
            if let Some((status, timestamp)) = cache.as_ref() {
                if timestamp.elapsed() < self.cache_duration {
                    return Some(status.clone());
                }
            }
        }
        None
    }

    fn set_cached_status(&self, status: CombinedStatus) {
        if let Ok(mut cache) = self.cached_status.lock() {
            *cache = Some((status, std::time::Instant::now()));
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum StatusType {
    BM25,
    Vector,
    Topics,
}

/// Mock layer status provider for testing.
#[derive(Default)]
pub struct MockLayerStatusProvider {
    pub bm25_enabled: bool,
    pub bm25_healthy: bool,
    pub bm25_doc_count: u64,
    pub vector_enabled: bool,
    pub vector_healthy: bool,
    pub vector_count: u64,
    pub topics_enabled: bool,
    pub topics_healthy: bool,
    pub topic_count: u64,
}

impl MockLayerStatusProvider {
    /// Create a provider with all layers enabled and healthy.
    pub fn all_available() -> Self {
        Self {
            bm25_enabled: true,
            bm25_healthy: true,
            bm25_doc_count: 100,
            vector_enabled: true,
            vector_healthy: true,
            vector_count: 100,
            topics_enabled: true,
            topics_healthy: true,
            topic_count: 50,
        }
    }

    /// Create a provider with no layers available (agentic only).
    pub fn agentic_only() -> Self {
        Self::default()
    }

    /// Create a provider with BM25 only.
    pub fn bm25_only() -> Self {
        Self {
            bm25_enabled: true,
            bm25_healthy: true,
            bm25_doc_count: 100,
            ..Default::default()
        }
    }

    /// Create a provider with Vector only.
    pub fn vector_only() -> Self {
        Self {
            vector_enabled: true,
            vector_healthy: true,
            vector_count: 100,
            ..Default::default()
        }
    }

    /// Create a provider with hybrid (BM25 + Vector) available.
    pub fn hybrid_available() -> Self {
        Self {
            bm25_enabled: true,
            bm25_healthy: true,
            bm25_doc_count: 100,
            vector_enabled: true,
            vector_healthy: true,
            vector_count: 100,
            ..Default::default()
        }
    }
}

#[async_trait]
impl LayerStatusProvider for MockLayerStatusProvider {
    async fn get_bm25_status(&self) -> Result<LayerStatus, String> {
        if !self.bm25_enabled {
            return Ok(LayerStatus::disabled(RetrievalLayer::BM25));
        }
        if !self.bm25_healthy {
            return Ok(LayerStatus::unhealthy(RetrievalLayer::BM25, "Unhealthy"));
        }
        Ok(LayerStatus::available(
            RetrievalLayer::BM25,
            self.bm25_doc_count,
        ))
    }

    async fn get_vector_status(&self) -> Result<LayerStatus, String> {
        if !self.vector_enabled {
            return Ok(LayerStatus::disabled(RetrievalLayer::Vector));
        }
        if !self.vector_healthy {
            return Ok(LayerStatus::unhealthy(RetrievalLayer::Vector, "Unhealthy"));
        }
        Ok(LayerStatus::available(
            RetrievalLayer::Vector,
            self.vector_count,
        ))
    }

    async fn get_topics_status(&self) -> Result<LayerStatus, String> {
        if !self.topics_enabled {
            return Ok(LayerStatus::disabled(RetrievalLayer::Topics));
        }
        if !self.topics_healthy {
            return Ok(LayerStatus::unhealthy(RetrievalLayer::Topics, "Unhealthy"));
        }
        Ok(LayerStatus::available(
            RetrievalLayer::Topics,
            self.topic_count,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_full_tier() {
        let provider = Arc::new(MockLayerStatusProvider::all_available());
        let detector = TierDetector::new(provider);

        let result = detector.detect().await;

        assert_eq!(result.tier, CapabilityTier::Full);
        assert!(result.status.bm25.is_ready());
        assert!(result.status.vector.is_ready());
        assert!(result.status.topics.is_ready());
    }

    #[tokio::test]
    async fn test_detect_hybrid_tier() {
        let provider = Arc::new(MockLayerStatusProvider::hybrid_available());
        let detector = TierDetector::new(provider);

        let result = detector.detect().await;

        assert_eq!(result.tier, CapabilityTier::Hybrid);
        assert!(result.status.bm25.is_ready());
        assert!(result.status.vector.is_ready());
        assert!(!result.status.topics.is_ready());
    }

    #[tokio::test]
    async fn test_detect_semantic_tier() {
        let provider = Arc::new(MockLayerStatusProvider::vector_only());
        let detector = TierDetector::new(provider);

        let result = detector.detect().await;

        assert_eq!(result.tier, CapabilityTier::Semantic);
        assert!(!result.status.bm25.is_ready());
        assert!(result.status.vector.is_ready());
    }

    #[tokio::test]
    async fn test_detect_keyword_tier() {
        let provider = Arc::new(MockLayerStatusProvider::bm25_only());
        let detector = TierDetector::new(provider);

        let result = detector.detect().await;

        assert_eq!(result.tier, CapabilityTier::Keyword);
        assert!(result.status.bm25.is_ready());
        assert!(!result.status.vector.is_ready());
    }

    #[tokio::test]
    async fn test_detect_agentic_tier() {
        let provider = Arc::new(MockLayerStatusProvider::agentic_only());
        let detector = TierDetector::new(provider);

        let result = detector.detect().await;

        assert_eq!(result.tier, CapabilityTier::Agentic);
        assert!(!result.status.bm25.is_ready());
        assert!(!result.status.vector.is_ready());
        assert!(!result.status.topics.is_ready());
    }

    #[tokio::test]
    async fn test_layer_order_for_explore() {
        let provider = Arc::new(MockLayerStatusProvider::all_available());
        let detector = TierDetector::new(provider);

        let result = detector.detect().await;
        let order = result.get_layer_order(QueryIntent::Explore);

        // Explore should prioritize Topics
        assert_eq!(order[0], RetrievalLayer::Topics);
        // Agentic should always be last
        assert_eq!(*order.last().unwrap(), RetrievalLayer::Agentic);
    }

    #[tokio::test]
    async fn test_layer_order_for_locate() {
        let provider = Arc::new(MockLayerStatusProvider::all_available());
        let detector = TierDetector::new(provider);

        let result = detector.detect().await;
        let order = result.get_layer_order(QueryIntent::Locate);

        // Locate should prioritize BM25
        assert_eq!(order[0], RetrievalLayer::BM25);
    }

    #[tokio::test]
    async fn test_layer_order_filters_unavailable() {
        let provider = Arc::new(MockLayerStatusProvider::bm25_only());
        let detector = TierDetector::new(provider);

        let result = detector.detect().await;
        let order = result.get_layer_order(QueryIntent::Explore);

        // Should not include Vector or Topics since unavailable
        assert!(!order.contains(&RetrievalLayer::Vector));
        assert!(!order.contains(&RetrievalLayer::Topics));
        // Should have BM25 and Agentic
        assert!(order.contains(&RetrievalLayer::BM25));
        assert!(order.contains(&RetrievalLayer::Agentic));
    }

    #[tokio::test]
    async fn test_cache_works() {
        let provider = Arc::new(MockLayerStatusProvider::all_available());
        let detector = TierDetector::new(provider).with_cache_duration(Duration::from_secs(60));

        // First detection
        let result1 = detector.detect().await;
        assert_eq!(result1.tier, CapabilityTier::Full);

        // Second detection should use cache
        let result2 = detector.detect().await;
        assert!(!result2.warnings.is_empty());
        assert!(result2.warnings.iter().any(|w| w.contains("cached")));
    }

    #[tokio::test]
    async fn test_fresh_detection_bypasses_cache() {
        let provider = Arc::new(MockLayerStatusProvider::all_available());
        let detector = TierDetector::new(provider).with_cache_duration(Duration::from_secs(60));

        // First detection
        let _ = detector.detect().await;

        // Fresh detection should not use cache
        let result = detector.detect_fresh().await;
        assert!(!result.warnings.iter().any(|w| w.contains("cached")));
    }

    #[test]
    fn test_result_summary() {
        let status = CombinedStatus::new(
            LayerStatus::available(RetrievalLayer::BM25, 100),
            LayerStatus::available(RetrievalLayer::Vector, 100),
            LayerStatus::disabled(RetrievalLayer::Topics),
        );

        let result = TierDetectionResult {
            tier: CapabilityTier::Hybrid,
            status,
            detection_time_ms: 50,
            warnings: vec![],
        };

        let summary = result.summary();
        assert!(summary.contains("Hybrid"));
        assert!(summary.contains("BM25: ready"));
        assert!(summary.contains("Vector: ready"));
        assert!(summary.contains("Topics: unavailable"));
    }
}
