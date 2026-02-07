//! Core retrieval types for the Agent Retrieval Policy.
//!
//! This module defines the fundamental types used throughout the retrieval
//! policy engine:
//! - `QueryIntent`: Classification of what the user wants to accomplish
//! - `CapabilityTier`: Available retrieval capabilities based on layer status
//! - `StopConditions`: Safety bounds for retrieval operations
//! - `ExecutionMode`: How to execute retrieval (sequential/parallel/hybrid)
//! - `RetrievalLayer`: Individual search layer identifiers
//! - `LayerStatus`: Health and availability of a single layer
//! - `CombinedStatus`: Status of all layers combined

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Query intent classification per PRD Section 3.
///
/// Determines the retrieval strategy and layer priority order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QueryIntent {
    /// Discover patterns, related concepts, themes.
    /// Examples: "What have I been working on?", "Show me recurring topics"
    /// Priority: Topics -> Hybrid/Vector/BM25 -> Agentic
    Explore,

    /// Get evidence-backed result fast.
    /// Examples: "How did we fix the JWT bug?", "What was decided about X?"
    /// Priority: Hybrid -> BM25/Vector -> Agentic
    /// Per PRD: "Default to ANSWER if unclear"
    #[default]
    Answer,

    /// Find exact snippet, quote, or definition.
    /// Examples: "Where did I define that config?", "Find the error message"
    /// Priority: BM25 -> Hybrid/Vector -> Agentic
    Locate,

    /// Return best partial in N ms, then stop.
    /// Used by agentic skills with latency constraints.
    /// Priority: Best available accelerator -> Agentic -> STOP
    TimeBoxed,
}

impl QueryIntent {
    /// Returns true if this intent allows escalation to scanning.
    ///
    /// Per PRD Section 5.3: "Limit: Only for EXPLORE, ANSWER, LOCATE; never for TIME-BOXED"
    pub fn allows_escalation(&self) -> bool {
        match self {
            QueryIntent::Explore | QueryIntent::Answer | QueryIntent::Locate => true,
            QueryIntent::TimeBoxed => false,
        }
    }

    /// Returns whether stop conditions should be enforced strictly.
    ///
    /// Per PRD Section 5.5:
    /// - Time-boxed: Strict (hard stop)
    /// - Others: Soft (can exceed slightly)
    pub fn is_strict_enforcement(&self) -> bool {
        matches!(self, QueryIntent::TimeBoxed)
    }

    /// Returns the display name for this intent.
    pub fn as_str(&self) -> &'static str {
        match self {
            QueryIntent::Explore => "explore",
            QueryIntent::Answer => "answer",
            QueryIntent::Locate => "locate",
            QueryIntent::TimeBoxed => "time-boxed",
        }
    }
}

/// Capability tier based on available retrieval layers.
///
/// Per PRD Section 5.1, tiers indicate what retrieval methods are available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityTier {
    /// All layers available: Topics + Hybrid + Agentic
    /// Best for: Explore + contextual answers
    Full = 1,

    /// Hybrid (BM25 + Vector) + Agentic available
    /// Best for: Default for most Answer queries
    Hybrid = 2,

    /// Vector + Agentic available (BM25 unavailable)
    /// Best for: Semantic-heavy, concept queries
    Semantic = 3,

    /// BM25 + Agentic available (Vector unavailable)
    /// Best for: Exact term matching, technical queries
    Keyword = 4,

    /// Only Agentic TOC Search available
    /// Always works (guaranteed fallback)
    Agentic = 5,
}

impl CapabilityTier {
    /// Check if this tier supports a given layer.
    pub fn supports(&self, layer: RetrievalLayer) -> bool {
        match (self, layer) {
            // Full tier supports everything
            (CapabilityTier::Full, _) => true,

            // Hybrid tier: BM25 + Vector + Agentic
            (CapabilityTier::Hybrid, RetrievalLayer::BM25) => true,
            (CapabilityTier::Hybrid, RetrievalLayer::Vector) => true,
            (CapabilityTier::Hybrid, RetrievalLayer::Hybrid) => true,
            (CapabilityTier::Hybrid, RetrievalLayer::Agentic) => true,
            (CapabilityTier::Hybrid, RetrievalLayer::Topics) => false,

            // Semantic tier: Vector + Agentic
            (CapabilityTier::Semantic, RetrievalLayer::Vector) => true,
            (CapabilityTier::Semantic, RetrievalLayer::Agentic) => true,
            (CapabilityTier::Semantic, _) => false,

            // Keyword tier: BM25 + Agentic
            (CapabilityTier::Keyword, RetrievalLayer::BM25) => true,
            (CapabilityTier::Keyword, RetrievalLayer::Agentic) => true,
            (CapabilityTier::Keyword, _) => false,

            // Agentic tier: only Agentic
            (CapabilityTier::Agentic, RetrievalLayer::Agentic) => true,
            (CapabilityTier::Agentic, _) => false,
        }
    }

    /// Get human-readable description of this tier.
    pub fn description(&self) -> &'static str {
        match self {
            CapabilityTier::Full => "Full capability (Topics + Hybrid + Agentic)",
            CapabilityTier::Hybrid => "Hybrid capability (BM25 + Vector + Agentic)",
            CapabilityTier::Semantic => "Semantic capability (Vector + Agentic)",
            CapabilityTier::Keyword => "Keyword capability (BM25 + Agentic)",
            CapabilityTier::Agentic => "Agentic only (TOC navigation)",
        }
    }

    /// Get the best available layer for this tier.
    pub fn best_layer(&self) -> RetrievalLayer {
        match self {
            CapabilityTier::Full => RetrievalLayer::Topics,
            CapabilityTier::Hybrid => RetrievalLayer::Hybrid,
            CapabilityTier::Semantic => RetrievalLayer::Vector,
            CapabilityTier::Keyword => RetrievalLayer::BM25,
            CapabilityTier::Agentic => RetrievalLayer::Agentic,
        }
    }
}

/// Individual retrieval layer identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalLayer {
    /// Topic graph discovery (Layer 5 in cognitive stack)
    Topics,
    /// Hybrid BM25 + Vector search
    Hybrid,
    /// Vector semantic search (Layer 4)
    Vector,
    /// BM25 keyword search (Layer 3)
    BM25,
    /// Agentic TOC navigation (Layer 2) - always available
    Agentic,
}

impl RetrievalLayer {
    /// Returns the display name for this layer.
    pub fn as_str(&self) -> &'static str {
        match self {
            RetrievalLayer::Topics => "topics",
            RetrievalLayer::Hybrid => "hybrid",
            RetrievalLayer::Vector => "vector",
            RetrievalLayer::BM25 => "bm25",
            RetrievalLayer::Agentic => "agentic",
        }
    }

    /// Returns the cognitive layer number.
    pub fn layer_number(&self) -> u8 {
        match self {
            RetrievalLayer::Topics => 5,
            RetrievalLayer::Vector => 4,
            RetrievalLayer::BM25 => 3,
            RetrievalLayer::Hybrid => 3, // Combined BM25+Vector
            RetrievalLayer::Agentic => 2,
        }
    }
}

impl std::fmt::Display for RetrievalLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Stop conditions (safety bounds) for retrieval operations.
///
/// Per PRD Section 5.5: Every retrieval operation MUST respect these bounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopConditions {
    /// Maximum depth levels to traverse (default: 5)
    pub max_depth: u32,

    /// Maximum nodes to visit (default: 100)
    pub max_nodes: u32,

    /// Maximum RPC calls to make (default: 20)
    pub max_rpc_calls: u32,

    /// Maximum token budget for results (default: 4000)
    pub max_tokens: u32,

    /// Timeout in milliseconds (default: 5000)
    pub timeout_ms: u64,

    /// Beam width for parallel operations (default: 1, range: 1-5)
    pub beam_width: u8,

    /// Minimum confidence score to accept results (default: 0.0)
    pub min_confidence: f32,
}

impl Default for StopConditions {
    fn default() -> Self {
        Self {
            max_depth: 5,
            max_nodes: 100,
            max_rpc_calls: 20,
            max_tokens: 4000,
            timeout_ms: 5000,
            beam_width: 1,
            min_confidence: 0.0,
        }
    }
}

impl StopConditions {
    /// Create stop conditions with a custom timeout.
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            timeout_ms: timeout.as_millis() as u64,
            ..Default::default()
        }
    }

    /// Create stop conditions optimized for time-boxed queries.
    pub fn time_boxed(timeout: Duration) -> Self {
        Self {
            timeout_ms: timeout.as_millis() as u64,
            max_depth: 3,
            max_nodes: 50,
            max_rpc_calls: 10,
            beam_width: 1,
            ..Default::default()
        }
    }

    /// Create stop conditions optimized for exploration.
    pub fn exploration() -> Self {
        Self {
            max_depth: 7,
            max_nodes: 200,
            max_rpc_calls: 30,
            max_tokens: 8000,
            timeout_ms: 10000,
            beam_width: 3,
            min_confidence: 0.0,
        }
    }

    /// Builder: set max depth
    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }

    /// Builder: set max nodes
    pub fn with_max_nodes(mut self, nodes: u32) -> Self {
        self.max_nodes = nodes;
        self
    }

    /// Builder: set beam width (clamped to 1-5)
    pub fn with_beam_width(mut self, width: u8) -> Self {
        self.beam_width = width.clamp(1, 5);
        self
    }

    /// Builder: set minimum confidence
    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.min_confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Get the timeout as a Duration.
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}

/// Execution mode for retrieval operations.
///
/// Per PRD Section 5.4: Controls how layers are queried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// One layer at a time, beam width 1.
    /// Lowest cost, best explainability.
    /// Default for most queries.
    #[default]
    Sequential,

    /// Multiple accelerators or siblings at once.
    /// Higher cost, low latency tolerance.
    /// Use when recall is critical.
    Parallel,

    /// Start parallel, cancel losers when one dominates.
    /// Medium cost.
    /// Use for ambiguous queries, weak top-level results.
    Hybrid,
}

impl ExecutionMode {
    /// Returns the display name for this mode.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionMode::Sequential => "sequential",
            ExecutionMode::Parallel => "parallel",
            ExecutionMode::Hybrid => "hybrid",
        }
    }

    /// Returns whether this mode allows concurrent execution.
    pub fn is_concurrent(&self) -> bool {
        matches!(self, ExecutionMode::Parallel | ExecutionMode::Hybrid)
    }
}

/// Health and availability status of a single retrieval layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStatus {
    /// Which layer this status is for
    pub layer: RetrievalLayer,

    /// Whether the layer is enabled in configuration
    pub enabled: bool,

    /// Whether the layer is currently healthy/operational
    pub healthy: bool,

    /// Number of documents/entries in the layer (if applicable)
    pub doc_count: u64,

    /// Additional status message
    pub message: Option<String>,
}

impl LayerStatus {
    /// Create a status for an available layer.
    pub fn available(layer: RetrievalLayer, doc_count: u64) -> Self {
        Self {
            layer,
            enabled: true,
            healthy: true,
            doc_count,
            message: None,
        }
    }

    /// Create a status for a disabled layer.
    pub fn disabled(layer: RetrievalLayer) -> Self {
        Self {
            layer,
            enabled: false,
            healthy: false,
            doc_count: 0,
            message: Some("Layer disabled in configuration".to_string()),
        }
    }

    /// Create a status for an unhealthy layer.
    pub fn unhealthy(layer: RetrievalLayer, reason: &str) -> Self {
        Self {
            layer,
            enabled: true,
            healthy: false,
            doc_count: 0,
            message: Some(reason.to_string()),
        }
    }

    /// Check if this layer is ready for use.
    pub fn is_ready(&self) -> bool {
        self.enabled && self.healthy
    }
}

/// Combined status of all retrieval layers.
///
/// Per PRD Section 5.2: Skills detect the current tier by checking these statuses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedStatus {
    /// BM25 keyword search status
    pub bm25: LayerStatus,

    /// Vector semantic search status
    pub vector: LayerStatus,

    /// Topic graph status
    pub topics: LayerStatus,

    /// Agentic TOC search status (always enabled, always healthy)
    pub agentic: LayerStatus,
}

impl CombinedStatus {
    /// Create a new combined status from individual layer statuses.
    pub fn new(bm25: LayerStatus, vector: LayerStatus, topics: LayerStatus) -> Self {
        Self {
            bm25,
            vector,
            topics,
            // Agentic is always available
            agentic: LayerStatus::available(RetrievalLayer::Agentic, 0),
        }
    }

    /// Create a minimal status where only agentic is available.
    pub fn agentic_only() -> Self {
        Self {
            bm25: LayerStatus::disabled(RetrievalLayer::BM25),
            vector: LayerStatus::disabled(RetrievalLayer::Vector),
            topics: LayerStatus::disabled(RetrievalLayer::Topics),
            agentic: LayerStatus::available(RetrievalLayer::Agentic, 0),
        }
    }

    /// Determine the capability tier from layer statuses.
    ///
    /// Per PRD Section 5.2:
    /// - Full: Topics + Vector + BM25 all ready
    /// - Hybrid: Vector + BM25 ready, Topics unavailable
    /// - Semantic: Vector ready, BM25 unavailable
    /// - Keyword: BM25 ready, Vector unavailable
    /// - Agentic: Nothing else available
    pub fn detect_tier(&self) -> CapabilityTier {
        let bm25_ready = self.bm25.is_ready();
        let vector_ready = self.vector.is_ready();
        let topics_ready = self.topics.is_ready();

        match (topics_ready, vector_ready, bm25_ready) {
            (true, true, true) => CapabilityTier::Full,
            (_, true, true) => CapabilityTier::Hybrid,
            (_, true, false) => CapabilityTier::Semantic,
            (_, false, true) => CapabilityTier::Keyword,
            _ => CapabilityTier::Agentic,
        }
    }

    /// Get the status for a specific layer.
    pub fn get_layer_status(&self, layer: RetrievalLayer) -> &LayerStatus {
        match layer {
            RetrievalLayer::BM25 => &self.bm25,
            RetrievalLayer::Vector => &self.vector,
            RetrievalLayer::Topics => &self.topics,
            RetrievalLayer::Agentic => &self.agentic,
            RetrievalLayer::Hybrid => {
                // For hybrid, return the status of whichever component is NOT ready,
                // or BM25 if both are ready (arbitrary choice when both healthy)
                if self.vector.is_ready() {
                    &self.bm25
                } else {
                    &self.vector
                }
            }
        }
    }

    /// Check if hybrid search is available (both BM25 and Vector ready).
    pub fn hybrid_available(&self) -> bool {
        self.bm25.is_ready() && self.vector.is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_intent_defaults() {
        assert_eq!(QueryIntent::default(), QueryIntent::Answer);
    }

    #[test]
    fn test_query_intent_escalation() {
        assert!(QueryIntent::Explore.allows_escalation());
        assert!(QueryIntent::Answer.allows_escalation());
        assert!(QueryIntent::Locate.allows_escalation());
        assert!(!QueryIntent::TimeBoxed.allows_escalation());
    }

    #[test]
    fn test_query_intent_enforcement() {
        assert!(!QueryIntent::Explore.is_strict_enforcement());
        assert!(!QueryIntent::Answer.is_strict_enforcement());
        assert!(!QueryIntent::Locate.is_strict_enforcement());
        assert!(QueryIntent::TimeBoxed.is_strict_enforcement());
    }

    #[test]
    fn test_capability_tier_supports() {
        // Full tier supports everything
        assert!(CapabilityTier::Full.supports(RetrievalLayer::Topics));
        assert!(CapabilityTier::Full.supports(RetrievalLayer::Vector));
        assert!(CapabilityTier::Full.supports(RetrievalLayer::BM25));
        assert!(CapabilityTier::Full.supports(RetrievalLayer::Agentic));

        // Hybrid tier doesn't support Topics
        assert!(!CapabilityTier::Hybrid.supports(RetrievalLayer::Topics));
        assert!(CapabilityTier::Hybrid.supports(RetrievalLayer::Vector));
        assert!(CapabilityTier::Hybrid.supports(RetrievalLayer::BM25));

        // Semantic tier only supports Vector and Agentic
        assert!(!CapabilityTier::Semantic.supports(RetrievalLayer::BM25));
        assert!(CapabilityTier::Semantic.supports(RetrievalLayer::Vector));
        assert!(CapabilityTier::Semantic.supports(RetrievalLayer::Agentic));

        // Keyword tier only supports BM25 and Agentic
        assert!(CapabilityTier::Keyword.supports(RetrievalLayer::BM25));
        assert!(!CapabilityTier::Keyword.supports(RetrievalLayer::Vector));
        assert!(CapabilityTier::Keyword.supports(RetrievalLayer::Agentic));

        // Agentic tier only supports Agentic
        assert!(!CapabilityTier::Agentic.supports(RetrievalLayer::BM25));
        assert!(!CapabilityTier::Agentic.supports(RetrievalLayer::Vector));
        assert!(CapabilityTier::Agentic.supports(RetrievalLayer::Agentic));
    }

    #[test]
    fn test_capability_tier_ordering() {
        assert!(CapabilityTier::Full < CapabilityTier::Hybrid);
        assert!(CapabilityTier::Hybrid < CapabilityTier::Semantic);
        assert!(CapabilityTier::Semantic < CapabilityTier::Keyword);
        assert!(CapabilityTier::Keyword < CapabilityTier::Agentic);
    }

    #[test]
    fn test_stop_conditions_default() {
        let sc = StopConditions::default();
        assert_eq!(sc.max_depth, 5);
        assert_eq!(sc.max_nodes, 100);
        assert_eq!(sc.max_rpc_calls, 20);
        assert_eq!(sc.max_tokens, 4000);
        assert_eq!(sc.timeout_ms, 5000);
        assert_eq!(sc.beam_width, 1);
    }

    #[test]
    fn test_stop_conditions_builders() {
        let sc = StopConditions::default()
            .with_max_depth(10)
            .with_max_nodes(50)
            .with_beam_width(3);

        assert_eq!(sc.max_depth, 10);
        assert_eq!(sc.max_nodes, 50);
        assert_eq!(sc.beam_width, 3);
    }

    #[test]
    fn test_stop_conditions_beam_width_clamp() {
        let sc = StopConditions::default().with_beam_width(10);
        assert_eq!(sc.beam_width, 5); // Clamped to max 5

        let sc = StopConditions::default().with_beam_width(0);
        assert_eq!(sc.beam_width, 1); // Clamped to min 1
    }

    #[test]
    fn test_execution_mode_concurrent() {
        assert!(!ExecutionMode::Sequential.is_concurrent());
        assert!(ExecutionMode::Parallel.is_concurrent());
        assert!(ExecutionMode::Hybrid.is_concurrent());
    }

    #[test]
    fn test_layer_status_ready() {
        let available = LayerStatus::available(RetrievalLayer::BM25, 100);
        assert!(available.is_ready());

        let disabled = LayerStatus::disabled(RetrievalLayer::Vector);
        assert!(!disabled.is_ready());

        let unhealthy = LayerStatus::unhealthy(RetrievalLayer::Topics, "Index corrupted");
        assert!(!unhealthy.is_ready());
    }

    #[test]
    fn test_combined_status_detect_tier() {
        // All layers ready -> Full
        let status = CombinedStatus::new(
            LayerStatus::available(RetrievalLayer::BM25, 100),
            LayerStatus::available(RetrievalLayer::Vector, 100),
            LayerStatus::available(RetrievalLayer::Topics, 50),
        );
        assert_eq!(status.detect_tier(), CapabilityTier::Full);

        // Topics unavailable -> Hybrid
        let status = CombinedStatus::new(
            LayerStatus::available(RetrievalLayer::BM25, 100),
            LayerStatus::available(RetrievalLayer::Vector, 100),
            LayerStatus::disabled(RetrievalLayer::Topics),
        );
        assert_eq!(status.detect_tier(), CapabilityTier::Hybrid);

        // Only Vector -> Semantic
        let status = CombinedStatus::new(
            LayerStatus::disabled(RetrievalLayer::BM25),
            LayerStatus::available(RetrievalLayer::Vector, 100),
            LayerStatus::disabled(RetrievalLayer::Topics),
        );
        assert_eq!(status.detect_tier(), CapabilityTier::Semantic);

        // Only BM25 -> Keyword
        let status = CombinedStatus::new(
            LayerStatus::available(RetrievalLayer::BM25, 100),
            LayerStatus::disabled(RetrievalLayer::Vector),
            LayerStatus::disabled(RetrievalLayer::Topics),
        );
        assert_eq!(status.detect_tier(), CapabilityTier::Keyword);

        // Nothing -> Agentic
        let status = CombinedStatus::agentic_only();
        assert_eq!(status.detect_tier(), CapabilityTier::Agentic);
    }

    #[test]
    fn test_combined_status_hybrid_available() {
        let status = CombinedStatus::new(
            LayerStatus::available(RetrievalLayer::BM25, 100),
            LayerStatus::available(RetrievalLayer::Vector, 100),
            LayerStatus::disabled(RetrievalLayer::Topics),
        );
        assert!(status.hybrid_available());

        let status = CombinedStatus::new(
            LayerStatus::disabled(RetrievalLayer::BM25),
            LayerStatus::available(RetrievalLayer::Vector, 100),
            LayerStatus::disabled(RetrievalLayer::Topics),
        );
        assert!(!status.hybrid_available());
    }

    #[test]
    fn test_retrieval_layer_display() {
        assert_eq!(RetrievalLayer::Topics.as_str(), "topics");
        assert_eq!(RetrievalLayer::Vector.as_str(), "vector");
        assert_eq!(RetrievalLayer::BM25.as_str(), "bm25");
        assert_eq!(RetrievalLayer::Agentic.as_str(), "agentic");
        assert_eq!(format!("{}", RetrievalLayer::Hybrid), "hybrid");
    }
}
