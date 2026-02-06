//! # memory-search
//!
//! Full-text search for Agent Memory using Tantivy.
//!
//! This crate provides BM25 keyword search for "teleporting" directly to
//! relevant TOC nodes or grips without traversing the hierarchy.
//!
//! ## Features
//! - Embedded Tantivy index with MmapDirectory for persistence
//! - Schema for indexing TOC node summaries and grip excerpts
//! - BM25 scoring for relevance ranking
//! - Document type filtering (toc_node vs grip)
//!
//! ## Requirements
//! - TEL-01: Tantivy embedded index
//! - TEL-02: BM25 search returns ranked results
//! - TEL-03: Relevance scores for agent decision-making
//! - TEL-04: Incremental index updates

pub mod document;
pub mod error;
pub mod index;
pub mod indexer;
pub mod lifecycle;
pub mod schema;
pub mod searcher;

pub use document::{extract_toc_text, grip_to_doc, toc_node_to_doc};
pub use error::SearchError;
pub use index::{open_or_create_index, SearchIndex, SearchIndexConfig};
pub use indexer::SearchIndexer;
pub use lifecycle::{
    is_protected_level, retention_map, Bm25LifecycleConfig, Bm25MaintenanceConfig, Bm25PruneStats,
};
pub use schema::{build_teleport_schema, DocType, SearchSchema};
pub use searcher::{SearchOptions, TeleportResult, TeleportSearcher};
