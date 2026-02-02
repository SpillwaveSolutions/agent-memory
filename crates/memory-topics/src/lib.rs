//! # memory-topics
//!
//! Semantic topic extraction and management for Agent Memory.
//!
//! This crate enables conceptual discovery through topics extracted from
//! TOC summaries using embedding clustering. Topics have time-decayed
//! importance scores and can form relationships (similar, parent/child).
//!
//! ## Features
//! - HDBSCAN clustering for automatic topic detection
//! - TF-IDF keyword extraction for topic labeling
//! - Optional LLM-enhanced labeling with keyword fallback
//! - Time-decayed importance scoring
//! - Topic relationships (similar, parent, child)
//! - Optional feature - disabled by default
//!
//! ## Requirements
//! - TOPIC-01: Topic extraction from TOC summaries
//! - TOPIC-02: Topics stored in CF_TOPICS
//! - TOPIC-07: Optional via configuration
//! - TOPIC-08: GetTopicGraphStatus RPC for discovery

pub mod config;
pub mod error;
pub mod extraction;
pub mod importance;
pub mod labeling;
pub mod lifecycle;
pub mod llm_labeler;
pub mod relationships;
pub mod similarity;
pub mod storage;
pub mod tfidf;
pub mod types;

pub use config::{ImportanceConfig, LabelingConfig, TopicsConfig};
pub use error::TopicsError;
pub use extraction::TopicExtractor;
pub use importance::ImportanceScorer;
pub use labeling::{ClusterDocument, KeywordLabeler, TopicLabel, TopicLabeler};
pub use lifecycle::{LifecycleStats, TopicLifecycleManager};
pub use llm_labeler::{LlmClient, LlmLabeler, NoOpLlmClient};
pub use relationships::{RelationshipBuilder, TopicGraphBuilder};
pub use similarity::{calculate_centroid, cosine_similarity};
pub use storage::TopicStorage;
pub use tfidf::TfIdf;
pub use types::{
    Embedding, RelationshipType, Topic, TopicId, TopicLink, TopicRelationship, TopicStatus,
};
