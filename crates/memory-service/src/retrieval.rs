//! Retrieval Policy RPC handlers.
//!
//! Implements the Phase 17 Agent Retrieval Policy RPCs:
//! - GetRetrievalCapabilities: Combined status check for all retrieval layers
//! - ClassifyQueryIntent: Classify query intent and extract time constraints
//! - RouteQuery: Route query through optimal layers with explainability
//!
//! Per PRD: Agent Retrieval Policy - intent routing, tier detection, fallbacks.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tonic::{Request, Response, Status};
use tracing::{debug, info};

use memory_retrieval::{
    classifier::IntentClassifier,
    executor::{FallbackChain, LayerExecutor, RetrievalExecutor, SearchResult},
    types::{
        CapabilityTier as CrateTier, CombinedStatus, ExecutionMode as CrateExecMode,
        LayerStatus as CrateLayerStatus, QueryIntent as CrateIntent, RetrievalLayer as CrateLayer,
        StopConditions as CrateStopConditions,
    },
};
use memory_search::TeleportSearcher;
use memory_storage::Storage;

use crate::pb::{
    CapabilityTier as ProtoTier, ClassifyQueryIntentRequest, ClassifyQueryIntentResponse,
    ExecutionMode as ProtoExecMode, ExplainabilityPayload as ProtoExplainability,
    GetRetrievalCapabilitiesRequest, GetRetrievalCapabilitiesResponse,
    LayerStatus as ProtoLayerStatus, QueryIntent as ProtoIntent, RetrievalLayer as ProtoLayer,
    RetrievalResult as ProtoResult, RouteQueryRequest, RouteQueryResponse,
    StopConditions as ProtoStopConditions,
};
use crate::topics::TopicGraphHandler;
use crate::vector::VectorTeleportHandler;

/// Handler for retrieval policy RPCs.
pub struct RetrievalHandler {
    /// Storage for direct access
    storage: Arc<Storage>,

    /// Intent classifier
    classifier: IntentClassifier,

    /// Optional BM25 searcher
    bm25_searcher: Option<Arc<TeleportSearcher>>,

    /// Optional vector handler
    vector_handler: Option<Arc<VectorTeleportHandler>>,

    /// Optional topic handler
    topic_handler: Option<Arc<TopicGraphHandler>>,
}

impl RetrievalHandler {
    /// Create a new retrieval handler with storage only.
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            storage,
            classifier: IntentClassifier::new(),
            bm25_searcher: None,
            vector_handler: None,
            topic_handler: None,
        }
    }

    /// Create a retrieval handler with all services.
    pub fn with_services(
        storage: Arc<Storage>,
        bm25_searcher: Option<Arc<TeleportSearcher>>,
        vector_handler: Option<Arc<VectorTeleportHandler>>,
        topic_handler: Option<Arc<TopicGraphHandler>>,
    ) -> Self {
        Self {
            storage,
            classifier: IntentClassifier::new(),
            bm25_searcher,
            vector_handler,
            topic_handler,
        }
    }

    /// Handle GetRetrievalCapabilities RPC.
    ///
    /// Per PRD Section 5.2: Combined status check pattern.
    pub async fn get_retrieval_capabilities(
        &self,
        _request: Request<GetRetrievalCapabilitiesRequest>,
    ) -> Result<Response<GetRetrievalCapabilitiesResponse>, Status> {
        let start = Instant::now();
        let mut warnings = Vec::new();

        // Check BM25 status
        let bm25_status = self.check_bm25_status().await;
        if !bm25_status.enabled {
            warnings.push("BM25 index not configured".to_string());
        }

        // Check Vector status
        let vector_status = self.check_vector_status().await;
        if !vector_status.enabled {
            warnings.push("Vector index not configured".to_string());
        }

        // Check Topics status
        let topics_status = self.check_topics_status().await;
        if !topics_status.enabled {
            warnings.push("Topic graph not configured".to_string());
        }

        // Agentic is always available (uses TOC navigation)
        let agentic_status = ProtoLayerStatus {
            layer: ProtoLayer::Agentic as i32,
            enabled: true,
            healthy: true,
            doc_count: 0, // TOC-based, no doc count
            message: Some("Agentic TOC search always available".to_string()),
        };

        // Determine tier based on status
        let combined = CombinedStatus::new(
            layer_status_from_proto(&bm25_status),
            layer_status_from_proto(&vector_status),
            layer_status_from_proto(&topics_status),
        );
        let tier = combined.detect_tier();

        let detection_time_ms = start.elapsed().as_millis() as u64;

        info!(
            tier = ?tier,
            bm25_available = bm25_status.healthy,
            vector_available = vector_status.healthy,
            topics_available = topics_status.healthy,
            detection_time_ms,
            "Retrieval capabilities detected"
        );

        Ok(Response::new(GetRetrievalCapabilitiesResponse {
            tier: tier_to_proto(tier) as i32,
            bm25_status: Some(bm25_status),
            vector_status: Some(vector_status),
            topics_status: Some(topics_status),
            agentic_status: Some(agentic_status),
            detection_time_ms,
            warnings,
        }))
    }

    /// Handle ClassifyQueryIntent RPC.
    ///
    /// Per PRD Section 4: Intent classification with keyword heuristics.
    pub async fn classify_query_intent(
        &self,
        request: Request<ClassifyQueryIntentRequest>,
    ) -> Result<Response<ClassifyQueryIntentResponse>, Status> {
        let req = request.into_inner();

        if req.query.is_empty() {
            return Err(Status::invalid_argument("Query is required"));
        }

        // Build stop conditions for classification
        let _stop_conditions = if let Some(timeout_ms) = req.timeout_ms {
            CrateStopConditions::with_timeout(Duration::from_millis(timeout_ms))
        } else {
            CrateStopConditions::default()
        };

        // Classify the query
        let classification = self.classifier.classify(&req.query);

        debug!(
            query = %req.query,
            intent = ?classification.intent,
            confidence = classification.confidence,
            "Query classified"
        );

        // Extract lookback from time_constraint if present
        let lookback_ms = classification
            .time_constraint
            .as_ref()
            .and_then(|tc| tc.lookback.map(|d| d.as_millis() as u64))
            .unwrap_or(0);

        Ok(Response::new(ClassifyQueryIntentResponse {
            intent: intent_to_proto(classification.intent) as i32,
            confidence: classification.confidence,
            reason: classification.reason,
            matched_keywords: classification.matched_keywords,
            lookback_ms: Some(lookback_ms),
        }))
    }

    /// Handle RouteQuery RPC.
    ///
    /// Per PRD Section 5.4: Route through optimal layers with fallbacks.
    pub async fn route_query(
        &self,
        request: Request<RouteQueryRequest>,
    ) -> Result<Response<RouteQueryResponse>, Status> {
        let req = request.into_inner();

        if req.query.is_empty() {
            return Err(Status::invalid_argument("Query is required"));
        }

        // Get stop conditions
        let stop_conditions = req
            .stop_conditions
            .map(|sc| proto_to_stop_conditions(&sc))
            .unwrap_or_default();

        // Classify intent or use override
        let intent = if let Some(override_intent) = req.intent_override {
            proto_to_intent(ProtoIntent::try_from(override_intent).unwrap_or(ProtoIntent::Answer))
        } else {
            self.classifier.classify(&req.query).intent
        };

        // Get current tier
        let tier = self.detect_current_tier().await;

        // Get execution mode
        let mode = if let Some(override_mode) = req.mode_override {
            proto_to_exec_mode(
                ProtoExecMode::try_from(override_mode).unwrap_or(ProtoExecMode::Sequential),
            )
        } else {
            // Default: Sequential for most, Parallel for complex
            match intent {
                CrateIntent::Explore => CrateExecMode::Parallel,
                CrateIntent::Answer => CrateExecMode::Hybrid,
                CrateIntent::Locate => CrateExecMode::Sequential,
                CrateIntent::TimeBoxed => CrateExecMode::Sequential,
            }
        };

        let limit = if req.limit > 0 {
            req.limit as usize
        } else {
            10
        };

        // Execute the retrieval
        let start = Instant::now();
        let chain = FallbackChain::for_intent(intent, tier);

        // Create a simple executor that delegates to our services
        let executor = Arc::new(SimpleLayerExecutor::new(
            self.storage.clone(),
            self.bm25_searcher.clone(),
            self.vector_handler.clone(),
            self.topic_handler.clone(),
        ));

        let retrieval_executor = RetrievalExecutor::new(executor);
        let result = retrieval_executor
            .execute(&req.query, chain, &stop_conditions, mode, tier)
            .await;

        let total_time_ms = start.elapsed().as_millis() as u64;

        // Convert results to proto
        let results: Vec<ProtoResult> = result
            .results
            .iter()
            .take(limit)
            .map(|r| ProtoResult {
                doc_id: r.doc_id.clone(),
                doc_type: r.doc_type.clone(),
                score: r.score,
                text_preview: r.text_preview.clone(),
                source_layer: layer_to_proto(r.source_layer) as i32,
                metadata: r.metadata.clone(),
                agent: r.metadata.get("agent").cloned(),
            })
            .collect();

        // Build explainability payload
        let explanation = ProtoExplainability {
            intent: intent_to_proto(intent) as i32,
            tier: tier_to_proto(tier) as i32,
            mode: exec_mode_to_proto(mode) as i32,
            candidates_considered: result
                .layers_attempted
                .iter()
                .map(|l| layer_to_proto(*l) as i32)
                .collect(),
            winner: layer_to_proto(result.primary_layer) as i32,
            why_winner: result.explanation.clone(),
            fallback_occurred: result.fallback_occurred,
            fallback_reason: if result.fallback_occurred {
                Some(result.explanation.clone())
            } else {
                None
            },
            total_time_ms,
            grip_ids: result
                .results
                .iter()
                .filter(|r| r.doc_type == "grip")
                .map(|r| r.doc_id.clone())
                .collect(),
        };

        info!(
            query = %req.query,
            intent = ?intent,
            tier = ?tier,
            mode = ?mode,
            result_count = results.len(),
            total_time_ms,
            "Query routed"
        );

        Ok(Response::new(RouteQueryResponse {
            results,
            explanation: Some(explanation),
            has_results: result.has_results(),
            layers_attempted: result
                .layers_attempted
                .iter()
                .map(|l| layer_to_proto(*l) as i32)
                .collect(),
        }))
    }

    /// Check BM25 layer status.
    async fn check_bm25_status(&self) -> ProtoLayerStatus {
        match &self.bm25_searcher {
            Some(searcher) => {
                let doc_count = searcher.num_docs();
                ProtoLayerStatus {
                    layer: ProtoLayer::Bm25 as i32,
                    enabled: true,
                    healthy: doc_count > 0,
                    doc_count,
                    message: if doc_count > 0 {
                        Some(format!("{} documents indexed", doc_count))
                    } else {
                        Some("Index empty".to_string())
                    },
                }
            }
            None => ProtoLayerStatus {
                layer: ProtoLayer::Bm25 as i32,
                enabled: false,
                healthy: false,
                doc_count: 0,
                message: Some("BM25 index not configured".to_string()),
            },
        }
    }

    /// Check vector layer status.
    async fn check_vector_status(&self) -> ProtoLayerStatus {
        match &self.vector_handler {
            Some(handler) => {
                let status = handler.get_status();
                ProtoLayerStatus {
                    layer: ProtoLayer::Vector as i32,
                    enabled: true,
                    healthy: status.available && status.vector_count > 0,
                    doc_count: status.vector_count as u64,
                    message: if status.available {
                        Some(format!("{} vectors indexed", status.vector_count))
                    } else {
                        Some("Index unavailable".to_string())
                    },
                }
            }
            None => ProtoLayerStatus {
                layer: ProtoLayer::Vector as i32,
                enabled: false,
                healthy: false,
                doc_count: 0,
                message: Some("Vector index not configured".to_string()),
            },
        }
    }

    /// Check topics layer status.
    async fn check_topics_status(&self) -> ProtoLayerStatus {
        match &self.topic_handler {
            Some(handler) => {
                let status = handler.get_status().await;
                ProtoLayerStatus {
                    layer: ProtoLayer::Topics as i32,
                    enabled: true,
                    healthy: status.available && status.topic_count > 0,
                    doc_count: status.topic_count,
                    message: if status.available {
                        Some(format!("{} topics available", status.topic_count))
                    } else {
                        Some("Topic graph unavailable".to_string())
                    },
                }
            }
            None => ProtoLayerStatus {
                layer: ProtoLayer::Topics as i32,
                enabled: false,
                healthy: false,
                doc_count: 0,
                message: Some("Topic graph not configured".to_string()),
            },
        }
    }

    /// Detect the current capability tier.
    async fn detect_current_tier(&self) -> CrateTier {
        let bm25_status = self.check_bm25_status().await;
        let vector_status = self.check_vector_status().await;
        let topics_status = self.check_topics_status().await;

        let combined = CombinedStatus::new(
            layer_status_from_proto(&bm25_status),
            layer_status_from_proto(&vector_status),
            layer_status_from_proto(&topics_status),
        );
        combined.detect_tier()
    }
}

/// Simple layer executor that delegates to available services.
struct SimpleLayerExecutor {
    _storage: Arc<Storage>,
    bm25_searcher: Option<Arc<TeleportSearcher>>,
    vector_handler: Option<Arc<VectorTeleportHandler>>,
    topic_handler: Option<Arc<TopicGraphHandler>>,
}

impl SimpleLayerExecutor {
    fn new(
        storage: Arc<Storage>,
        bm25_searcher: Option<Arc<TeleportSearcher>>,
        vector_handler: Option<Arc<VectorTeleportHandler>>,
        topic_handler: Option<Arc<TopicGraphHandler>>,
    ) -> Self {
        Self {
            _storage: storage,
            bm25_searcher,
            vector_handler,
            topic_handler,
        }
    }
}

#[async_trait]
impl LayerExecutor for SimpleLayerExecutor {
    async fn execute(
        &self,
        query: &str,
        layer: CrateLayer,
        limit: usize,
    ) -> Result<Vec<SearchResult>, String> {
        match layer {
            CrateLayer::BM25 => {
                if let Some(searcher) = &self.bm25_searcher {
                    let opts = memory_search::SearchOptions::new().with_limit(limit);
                    let results = searcher.search(query, opts).map_err(|e| e.to_string())?;
                    Ok(results
                        .into_iter()
                        .map(|r| SearchResult {
                            doc_id: r.doc_id,
                            doc_type: format!("{:?}", r.doc_type).to_lowercase(),
                            score: r.score,
                            text_preview: r.keywords.unwrap_or_default(),
                            source_layer: CrateLayer::BM25,
                            metadata: HashMap::new(),
                        })
                        .collect())
                } else {
                    Err("BM25 not available".to_string())
                }
            }
            CrateLayer::Vector => {
                if let Some(handler) = &self.vector_handler {
                    let results = handler.search(query, limit, 0.0).await?;
                    Ok(results
                        .into_iter()
                        .map(|r| SearchResult {
                            doc_id: r.doc_id,
                            doc_type: r.doc_type,
                            score: r.score,
                            text_preview: r.text_preview,
                            source_layer: CrateLayer::Vector,
                            metadata: HashMap::new(),
                        })
                        .collect())
                } else {
                    Err("Vector not available".to_string())
                }
            }
            CrateLayer::Topics => {
                if let Some(handler) = &self.topic_handler {
                    let topics = handler.search_topics(query, limit as u32).await?;
                    Ok(topics
                        .into_iter()
                        .map(|t| SearchResult {
                            doc_id: t.id,
                            doc_type: "topic".to_string(),
                            score: t.importance_score,
                            text_preview: t.label,
                            source_layer: CrateLayer::Topics,
                            metadata: HashMap::new(),
                        })
                        .collect())
                } else {
                    Err("Topics not available".to_string())
                }
            }
            CrateLayer::Hybrid => {
                // Hybrid combines BM25 and Vector - for now, delegate to BM25 if available
                if let Some(searcher) = &self.bm25_searcher {
                    let opts = memory_search::SearchOptions::new().with_limit(limit);
                    let results = searcher.search(query, opts).map_err(|e| e.to_string())?;
                    Ok(results
                        .into_iter()
                        .map(|r| SearchResult {
                            doc_id: r.doc_id,
                            doc_type: format!("{:?}", r.doc_type).to_lowercase(),
                            score: r.score,
                            text_preview: r.keywords.unwrap_or_default(),
                            source_layer: CrateLayer::Hybrid,
                            metadata: HashMap::new(),
                        })
                        .collect())
                } else if let Some(handler) = &self.vector_handler {
                    let results = handler.search(query, limit, 0.0).await?;
                    Ok(results
                        .into_iter()
                        .map(|r| SearchResult {
                            doc_id: r.doc_id,
                            doc_type: r.doc_type,
                            score: r.score,
                            text_preview: r.text_preview,
                            source_layer: CrateLayer::Hybrid,
                            metadata: HashMap::new(),
                        })
                        .collect())
                } else {
                    Err("Hybrid requires BM25 or Vector".to_string())
                }
            }
            CrateLayer::Agentic => {
                // Agentic uses TOC navigation - perform basic TOC search
                // This is a fallback that always works
                // TODO: Implement full TOC navigation when Storage API is extended
                debug!("Agentic layer search for: {}", query);
                Ok(Vec::new())
            }
        }
    }

    fn supports(&self, layer: CrateLayer) -> bool {
        match layer {
            CrateLayer::BM25 => self.bm25_searcher.is_some(),
            CrateLayer::Vector => self.vector_handler.is_some(),
            CrateLayer::Topics => self.topic_handler.is_some(),
            CrateLayer::Hybrid => self.bm25_searcher.is_some() || self.vector_handler.is_some(),
            CrateLayer::Agentic => true, // Always available
        }
    }
}

// ===== Conversion helpers =====

/// Convert proto LayerStatus to crate LayerStatus.
fn layer_status_from_proto(proto: &ProtoLayerStatus) -> CrateLayerStatus {
    let layer = match ProtoLayer::try_from(proto.layer) {
        Ok(ProtoLayer::Bm25) => CrateLayer::BM25,
        Ok(ProtoLayer::Vector) => CrateLayer::Vector,
        Ok(ProtoLayer::Topics) => CrateLayer::Topics,
        Ok(ProtoLayer::Hybrid) => CrateLayer::Hybrid,
        Ok(ProtoLayer::Agentic) | Ok(ProtoLayer::Unspecified) | Err(_) => CrateLayer::Agentic,
    };

    if !proto.enabled {
        CrateLayerStatus::disabled(layer)
    } else if !proto.healthy {
        CrateLayerStatus::unhealthy(layer, proto.message.as_deref().unwrap_or("Unhealthy"))
    } else {
        CrateLayerStatus::available(layer, proto.doc_count)
    }
}

fn tier_to_proto(tier: CrateTier) -> ProtoTier {
    match tier {
        CrateTier::Full => ProtoTier::Full,
        CrateTier::Hybrid => ProtoTier::Hybrid,
        CrateTier::Semantic => ProtoTier::Semantic,
        CrateTier::Keyword => ProtoTier::Keyword,
        CrateTier::Agentic => ProtoTier::Agentic,
    }
}

fn intent_to_proto(intent: CrateIntent) -> ProtoIntent {
    match intent {
        CrateIntent::Explore => ProtoIntent::Explore,
        CrateIntent::Answer => ProtoIntent::Answer,
        CrateIntent::Locate => ProtoIntent::Locate,
        CrateIntent::TimeBoxed => ProtoIntent::TimeBoxed,
    }
}

fn proto_to_intent(proto: ProtoIntent) -> CrateIntent {
    match proto {
        ProtoIntent::Explore => CrateIntent::Explore,
        ProtoIntent::Answer => CrateIntent::Answer,
        ProtoIntent::Locate => CrateIntent::Locate,
        ProtoIntent::TimeBoxed => CrateIntent::TimeBoxed,
        ProtoIntent::Unspecified => CrateIntent::Answer, // Default
    }
}

fn layer_to_proto(layer: CrateLayer) -> ProtoLayer {
    match layer {
        CrateLayer::Topics => ProtoLayer::Topics,
        CrateLayer::Hybrid => ProtoLayer::Hybrid,
        CrateLayer::Vector => ProtoLayer::Vector,
        CrateLayer::BM25 => ProtoLayer::Bm25,
        CrateLayer::Agentic => ProtoLayer::Agentic,
    }
}

fn exec_mode_to_proto(mode: CrateExecMode) -> ProtoExecMode {
    match mode {
        CrateExecMode::Sequential => ProtoExecMode::Sequential,
        CrateExecMode::Parallel => ProtoExecMode::Parallel,
        CrateExecMode::Hybrid => ProtoExecMode::Hybrid,
    }
}

fn proto_to_exec_mode(proto: ProtoExecMode) -> CrateExecMode {
    match proto {
        ProtoExecMode::Sequential => CrateExecMode::Sequential,
        ProtoExecMode::Parallel => CrateExecMode::Parallel,
        ProtoExecMode::Hybrid => CrateExecMode::Hybrid,
        ProtoExecMode::Unspecified => CrateExecMode::Sequential, // Default
    }
}

fn proto_to_stop_conditions(proto: &ProtoStopConditions) -> CrateStopConditions {
    let mut conditions = CrateStopConditions::default();

    if proto.max_depth > 0 {
        conditions.max_depth = proto.max_depth;
    }
    if proto.max_nodes > 0 {
        conditions.max_nodes = proto.max_nodes;
    }
    if proto.max_rpc_calls > 0 {
        conditions.max_rpc_calls = proto.max_rpc_calls;
    }
    if proto.max_tokens > 0 {
        conditions.max_tokens = proto.max_tokens;
    }
    if proto.timeout_ms > 0 {
        conditions.timeout_ms = proto.timeout_ms;
    }
    if proto.beam_width > 0 {
        conditions.beam_width = proto.beam_width as u8;
    }
    if proto.min_confidence > 0.0 {
        conditions.min_confidence = proto.min_confidence;
    }

    conditions
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_handler() -> (RetrievalHandler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::open(temp_dir.path()).unwrap();
        let handler = RetrievalHandler::new(Arc::new(storage));
        (handler, temp_dir)
    }

    #[tokio::test]
    async fn test_get_retrieval_capabilities_agentic_only() {
        let (handler, _temp) = create_test_handler();

        let response = handler
            .get_retrieval_capabilities(Request::new(GetRetrievalCapabilitiesRequest {}))
            .await
            .unwrap();

        let resp = response.into_inner();

        // Should detect Agentic tier when no indexes configured
        assert_eq!(resp.tier, ProtoTier::Agentic as i32);

        // Agentic should always be available
        assert!(resp.agentic_status.unwrap().healthy);

        // Other layers should not be configured
        assert!(!resp.bm25_status.unwrap().enabled);
        assert!(!resp.vector_status.unwrap().enabled);
        assert!(!resp.topics_status.unwrap().enabled);
    }

    #[tokio::test]
    async fn test_classify_query_intent_explore() {
        let (handler, _temp) = create_test_handler();

        let response = handler
            .classify_query_intent(Request::new(ClassifyQueryIntentRequest {
                query: "what topics did we discuss about rust?".to_string(),
                timeout_ms: None,
            }))
            .await
            .unwrap();

        let resp = response.into_inner();
        // Should classify as Explore (topics keyword)
        assert_eq!(resp.intent, ProtoIntent::Explore as i32);
    }

    #[tokio::test]
    async fn test_classify_query_intent_locate() {
        let (handler, _temp) = create_test_handler();

        let response = handler
            .classify_query_intent(Request::new(ClassifyQueryIntentRequest {
                query: "find the exact error message about auth".to_string(),
                timeout_ms: None,
            }))
            .await
            .unwrap();

        let resp = response.into_inner();
        // Should classify as Locate (find, exact keywords)
        assert_eq!(resp.intent, ProtoIntent::Locate as i32);
    }

    #[tokio::test]
    async fn test_classify_query_empty_query() {
        let (handler, _temp) = create_test_handler();

        let result = handler
            .classify_query_intent(Request::new(ClassifyQueryIntentRequest {
                query: "".to_string(),
                timeout_ms: None,
            }))
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_route_query_basic() {
        let (handler, _temp) = create_test_handler();

        let response = handler
            .route_query(Request::new(RouteQueryRequest {
                query: "what is rust?".to_string(),
                intent_override: None,
                stop_conditions: None,
                mode_override: None,
                limit: 10,
                agent_filter: None,
            }))
            .await
            .unwrap();

        let resp = response.into_inner();

        // Should have explanation
        assert!(resp.explanation.is_some());

        // Should have attempted at least agentic layer
        assert!(!resp.layers_attempted.is_empty());
    }

    #[tokio::test]
    async fn test_route_query_empty_query() {
        let (handler, _temp) = create_test_handler();

        let result = handler
            .route_query(Request::new(RouteQueryRequest {
                query: "".to_string(),
                intent_override: None,
                stop_conditions: None,
                mode_override: None,
                limit: 10,
                agent_filter: None,
            }))
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn test_tier_conversion() {
        assert_eq!(tier_to_proto(CrateTier::Full), ProtoTier::Full);
        assert_eq!(tier_to_proto(CrateTier::Hybrid), ProtoTier::Hybrid);
        assert_eq!(tier_to_proto(CrateTier::Semantic), ProtoTier::Semantic);
        assert_eq!(tier_to_proto(CrateTier::Keyword), ProtoTier::Keyword);
        assert_eq!(tier_to_proto(CrateTier::Agentic), ProtoTier::Agentic);
    }

    #[test]
    fn test_intent_conversion() {
        assert_eq!(intent_to_proto(CrateIntent::Explore), ProtoIntent::Explore);
        assert_eq!(intent_to_proto(CrateIntent::Answer), ProtoIntent::Answer);
        assert_eq!(intent_to_proto(CrateIntent::Locate), ProtoIntent::Locate);
        assert_eq!(
            intent_to_proto(CrateIntent::TimeBoxed),
            ProtoIntent::TimeBoxed
        );

        assert_eq!(proto_to_intent(ProtoIntent::Explore), CrateIntent::Explore);
        assert_eq!(proto_to_intent(ProtoIntent::Answer), CrateIntent::Answer);
        assert_eq!(
            proto_to_intent(ProtoIntent::Unspecified),
            CrateIntent::Answer
        );
    }

    #[test]
    fn test_retrieval_result_agent_from_metadata() {
        use memory_retrieval::executor::SearchResult;
        use memory_retrieval::types::RetrievalLayer;

        // Result with agent in metadata
        let mut metadata = HashMap::new();
        metadata.insert("agent".to_string(), "opencode".to_string());
        let result = SearchResult {
            doc_id: "doc-1".to_string(),
            doc_type: "toc".to_string(),
            score: 0.95,
            text_preview: "test".to_string(),
            source_layer: RetrievalLayer::BM25,
            metadata,
        };
        let proto_result = ProtoResult {
            doc_id: result.doc_id.clone(),
            doc_type: result.doc_type.clone(),
            score: result.score,
            text_preview: result.text_preview.clone(),
            source_layer: layer_to_proto(result.source_layer) as i32,
            metadata: result.metadata.clone(),
            agent: result.metadata.get("agent").cloned(),
        };
        assert_eq!(proto_result.agent, Some("opencode".to_string()));

        // Result without agent
        let result_no_agent = SearchResult {
            doc_id: "doc-2".to_string(),
            doc_type: "grip".to_string(),
            score: 0.5,
            text_preview: "test".to_string(),
            source_layer: RetrievalLayer::Vector,
            metadata: HashMap::new(),
        };
        let proto_no_agent = ProtoResult {
            doc_id: result_no_agent.doc_id.clone(),
            doc_type: result_no_agent.doc_type.clone(),
            score: result_no_agent.score,
            text_preview: result_no_agent.text_preview.clone(),
            source_layer: layer_to_proto(result_no_agent.source_layer) as i32,
            metadata: result_no_agent.metadata.clone(),
            agent: result_no_agent.metadata.get("agent").cloned(),
        };
        assert_eq!(proto_no_agent.agent, None);
    }
}
