//! # memory-retrieval
//!
//! Agent retrieval policy engine for the agent-memory system.
//!
//! This crate implements the retrieval "brainstem" - the decision algorithm
//! for layer selection, intent classification, fallback chains, and skill contracts.
//!
//! ## Core Concepts
//!
//! - **Query Intent**: Classification of what the user wants (Explore/Answer/Locate/TimeBoxed)
//! - **Capability Tier**: Available retrieval capabilities based on layer status
//! - **Fallback Chain**: Ordered list of layers to try when one fails
//! - **Execution Mode**: How to execute retrieval (Sequential/Parallel/Hybrid)
//! - **Skill Contract**: Requirements for retrieval-capable skills
//!
//! ## Usage
//!
//! ```rust,ignore
//! use memory_retrieval::{
//!     IntentClassifier, TierDetector, RetrievalExecutor,
//!     FallbackChain, StopConditions, ExecutionMode,
//!     ExplainabilityPayload, SkillContract,
//! };
//!
//! // 1. Classify intent
//! let classifier = IntentClassifier::new();
//! let intent_result = classifier.classify("How did we fix the JWT bug?");
//!
//! // 2. Detect tier
//! let detector = TierDetector::new(status_provider);
//! let tier_result = detector.detect().await;
//!
//! // 3. Build fallback chain
//! let chain = FallbackChain::for_intent(intent_result.intent, tier_result.tier);
//!
//! // 4. Execute retrieval
//! let executor = RetrievalExecutor::new(layer_executor);
//! let result = executor.execute(
//!     "How did we fix the JWT bug?",
//!     chain,
//!     &StopConditions::default(),
//!     ExecutionMode::Sequential,
//!     tier_result.tier,
//! ).await;
//!
//! // 5. Create explainability payload
//! let payload = ExplainabilityPayload::from_execution(
//!     intent_result.intent,
//!     &result,
//!     &StopConditions::default(),
//! );
//! ```
//!
//! ## Modules
//!
//! - [`types`]: Core types (QueryIntent, CapabilityTier, StopConditions, etc.)
//! - [`classifier`]: Intent classification using keyword heuristics
//! - [`tier`]: Tier detection from layer statuses
//! - [`executor`]: Retrieval execution with fallbacks
//! - [`contracts`]: Skill contracts and explainability
//!
//! ## References
//!
//! - [Agent Retrieval Policy PRD](../../../docs/prds/agent-retrieval-policy-prd.md)

pub mod classifier;
pub mod contracts;
pub mod executor;
pub mod tier;
pub mod types;

// Re-export main types at crate root
pub use classifier::{ClassificationResult, ClassifierConfig, IntentClassifier, TimeConstraint};
pub use contracts::{
    generate_skill_md_section, BoundAction, BoundHit, BoundType, ExplainabilityPayload,
    IssueSeverity, SkillContract, SkillContractIssue, SkillContractValidation,
};
pub use executor::{
    ExecutionResult, FallbackChain, LayerExecutor, LayerResults, MockLayerExecutor,
    RetrievalExecutor, SearchResult,
};
pub use tier::{LayerStatusProvider, MockLayerStatusProvider, TierDetectionResult, TierDetector};
pub use types::{
    CapabilityTier, CombinedStatus, ExecutionMode, LayerStatus, QueryIntent, RetrievalLayer,
    StopConditions,
};

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::classifier::IntentClassifier;
    pub use crate::contracts::{ExplainabilityPayload, SkillContract};
    pub use crate::executor::{FallbackChain, RetrievalExecutor};
    pub use crate::tier::TierDetector;
    pub use crate::types::{
        CapabilityTier, ExecutionMode, QueryIntent, RetrievalLayer, StopConditions,
    };
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::sync::Arc;

    /// End-to-end test of the retrieval policy.
    #[tokio::test]
    async fn test_full_retrieval_flow() {
        // 1. Classify intent
        let classifier = IntentClassifier::new();
        let intent_result = classifier.classify("How did we fix the JWT bug?");
        assert_eq!(intent_result.intent, QueryIntent::Answer);

        // 2. Detect tier (using mock)
        let provider = Arc::new(MockLayerStatusProvider::hybrid_available());
        let detector = TierDetector::new(provider);
        let tier_result = detector.detect().await;
        assert_eq!(tier_result.tier, CapabilityTier::Hybrid);

        // 3. Build fallback chain
        let chain = FallbackChain::for_intent(intent_result.intent, tier_result.tier);
        assert!(!chain.layers.is_empty());

        // 4. Execute retrieval (using mock)
        // For Answer intent with Hybrid tier, it tries Hybrid first, then BM25
        let mock_executor = MockLayerExecutor::default().with_results(
            RetrievalLayer::Hybrid,
            vec![SearchResult {
                doc_id: "node-123".to_string(),
                doc_type: "toc_node".to_string(),
                score: 0.85,
                text_preview: "Fixed JWT token validation".to_string(),
                source_layer: RetrievalLayer::Hybrid,
                metadata: std::collections::HashMap::new(),
            }],
        );

        let executor = RetrievalExecutor::new(Arc::new(mock_executor));
        let conditions = StopConditions::default();

        let result = executor
            .execute(
                "How did we fix the JWT bug?",
                chain,
                &conditions,
                ExecutionMode::Sequential,
                tier_result.tier,
            )
            .await;

        assert!(result.has_results());
        // Hybrid found results directly, no fallback needed
        assert!(!result.fallback_occurred);

        // 5. Create explainability payload
        let payload =
            ExplainabilityPayload::from_execution(intent_result.intent, &result, &conditions);

        assert_eq!(payload.intent, QueryIntent::Answer);
        assert_eq!(payload.tier, CapabilityTier::Hybrid);
        assert!(payload.result_count > 0);

        // 6. Verify skill contract would be valid
        let contract = SkillContract::new("test-skill")
            .with_capability_detection()
            .with_budget_enforcement()
            .with_fallback_discipline()
            .with_explainability()
            .with_evidence_handling();

        let validation = contract.validate();
        assert!(validation.is_valid);
    }

    /// Test intent classification variations.
    #[test]
    fn test_intent_classification_variations() {
        let classifier = IntentClassifier::new();

        // Explore queries
        let explore_queries = [
            "What topics have we discussed?",
            "Show me the recurring themes",
            "What have I been working on?",
        ];

        for query in explore_queries {
            let result = classifier.classify(query);
            assert_eq!(
                result.intent,
                QueryIntent::Explore,
                "Query '{}' should be Explore",
                query
            );
        }

        // Locate queries
        let locate_queries = [
            "Where is the config defined?",
            "Find the error message",
            "Locate the database schema",
        ];

        for query in locate_queries {
            let result = classifier.classify(query);
            assert_eq!(
                result.intent,
                QueryIntent::Locate,
                "Query '{}' should be Locate",
                query
            );
        }

        // Answer queries
        let answer_queries = [
            "How did we solve the bug?",
            "Why was that approach chosen?",
            "What was the solution?",
        ];

        for query in answer_queries {
            let result = classifier.classify(query);
            assert_eq!(
                result.intent,
                QueryIntent::Answer,
                "Query '{}' should be Answer",
                query
            );
        }
    }

    /// Test tier detection with various layer configurations.
    #[tokio::test]
    async fn test_tier_detection_configurations() {
        let test_cases = [
            (
                MockLayerStatusProvider::all_available(),
                CapabilityTier::Full,
            ),
            (
                MockLayerStatusProvider::hybrid_available(),
                CapabilityTier::Hybrid,
            ),
            (
                MockLayerStatusProvider::vector_only(),
                CapabilityTier::Semantic,
            ),
            (
                MockLayerStatusProvider::bm25_only(),
                CapabilityTier::Keyword,
            ),
            (
                MockLayerStatusProvider::agentic_only(),
                CapabilityTier::Agentic,
            ),
        ];

        for (provider, expected_tier) in test_cases {
            let detector = TierDetector::new(Arc::new(provider));
            let result = detector.detect().await;
            assert_eq!(
                result.tier, expected_tier,
                "Expected tier {:?} but got {:?}",
                expected_tier, result.tier
            );
        }
    }

    /// Test fallback behavior.
    #[tokio::test]
    async fn test_fallback_behavior() {
        // Setup: BM25 fails, Vector succeeds
        let mock_executor = MockLayerExecutor::default()
            .with_failure(RetrievalLayer::BM25)
            .with_results(
                RetrievalLayer::Vector,
                vec![SearchResult {
                    doc_id: "node-456".to_string(),
                    doc_type: "toc_node".to_string(),
                    score: 0.7,
                    text_preview: "Found via vector search".to_string(),
                    source_layer: RetrievalLayer::Vector,
                    metadata: std::collections::HashMap::new(),
                }],
            );

        let executor = RetrievalExecutor::new(Arc::new(mock_executor));
        let chain = FallbackChain::for_intent(QueryIntent::Locate, CapabilityTier::Hybrid);
        let conditions = StopConditions::default();

        let result = executor
            .execute(
                "find something",
                chain,
                &conditions,
                ExecutionMode::Sequential,
                CapabilityTier::Hybrid,
            )
            .await;

        // Should have results from Vector after BM25 fallback
        assert!(result.has_results());
        assert!(result.fallback_occurred);
        assert_eq!(result.primary_layer, RetrievalLayer::Vector);
    }

    /// Test stop conditions are respected.
    #[tokio::test]
    async fn test_stop_conditions() {
        use std::time::Duration;

        // Setup: BM25 takes longer than per-layer timeout, but overall timeout allows fallback
        let mock_executor = MockLayerExecutor::default()
            .with_delay(RetrievalLayer::BM25, Duration::from_millis(200))
            .with_results(
                RetrievalLayer::Agentic,
                vec![SearchResult {
                    doc_id: "agentic-result".to_string(),
                    doc_type: "toc_node".to_string(),
                    score: 0.5,
                    text_preview: "Agentic fallback".to_string(),
                    source_layer: RetrievalLayer::Agentic,
                    metadata: std::collections::HashMap::new(),
                }],
            );

        let executor = RetrievalExecutor::new(Arc::new(mock_executor));
        let chain = FallbackChain::for_intent(QueryIntent::Locate, CapabilityTier::Keyword);
        // Overall timeout is 500ms: BM25 times out after ~100ms, then Agentic runs
        let conditions = StopConditions::with_timeout(Duration::from_millis(500));

        let result = executor
            .execute(
                "test",
                chain,
                &conditions,
                ExecutionMode::Sequential,
                CapabilityTier::Keyword,
            )
            .await;

        // Should timeout on BM25 and fallback to Agentic
        assert!(result.has_results());
        assert_eq!(result.primary_layer, RetrievalLayer::Agentic);
    }
}
