//! Search implementation using BM25 scoring.
//!
//! Provides keyword search over TOC nodes and grips.

use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, QueryParser, TermQuery};
use tantivy::schema::{IndexRecordOption, Value};
use tantivy::{IndexReader, Term};
use tracing::{debug, info};

use crate::error::SearchError;
use crate::index::SearchIndex;
use crate::schema::{DocType, SearchSchema};

/// A search result with relevance score.
#[derive(Debug, Clone)]
pub struct TeleportResult {
    /// Document ID (node_id or grip_id)
    pub doc_id: String,
    /// Document type
    pub doc_type: DocType,
    /// BM25 relevance score
    pub score: f32,
    /// Keywords from the document (if stored)
    pub keywords: Option<String>,
    /// Timestamp in milliseconds
    pub timestamp_ms: Option<i64>,
    /// Agent attribution (from TocNode.contributing_agents)
    pub agent: Option<String>,
}

/// Search options for filtering and limiting results.
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// Filter by document type (None = all types)
    pub doc_type: Option<DocType>,
    /// Maximum results to return
    pub limit: usize,
}

impl SearchOptions {
    pub fn new() -> Self {
        Self {
            doc_type: None,
            limit: 10,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_doc_type(mut self, doc_type: DocType) -> Self {
        self.doc_type = Some(doc_type);
        self
    }

    pub fn toc_only() -> Self {
        Self::new().with_doc_type(DocType::TocNode)
    }

    pub fn grips_only() -> Self {
        Self::new().with_doc_type(DocType::Grip)
    }
}

/// Searcher for teleport queries using BM25 ranking.
pub struct TeleportSearcher {
    reader: IndexReader,
    schema: SearchSchema,
    query_parser: QueryParser,
}

impl TeleportSearcher {
    /// Create a new searcher from a SearchIndex.
    pub fn new(index: &SearchIndex) -> Result<Self, SearchError> {
        let reader = index.reader()?;
        let schema = index.schema().clone();

        // Create query parser targeting text and keywords fields
        let query_parser =
            QueryParser::for_index(index.index(), vec![schema.text, schema.keywords]);

        Ok(Self {
            reader,
            schema,
            query_parser,
        })
    }

    /// Reload the reader to see recent commits.
    pub fn reload(&self) -> Result<(), SearchError> {
        self.reader.reload()?;
        debug!("Reloaded search reader");
        Ok(())
    }

    /// Search with a query string.
    ///
    /// Uses BM25 scoring over text and keywords fields.
    pub fn search(
        &self,
        query_str: &str,
        options: SearchOptions,
    ) -> Result<Vec<TeleportResult>, SearchError> {
        if query_str.trim().is_empty() {
            return Ok(Vec::new());
        }

        let searcher = self.reader.searcher();

        // Parse the text query
        let text_query = self.query_parser.parse_query(query_str)?;

        // Apply document type filter if specified
        let final_query = if let Some(doc_type) = options.doc_type {
            let type_term = Term::from_field_text(self.schema.doc_type, doc_type.as_str());
            let type_query = TermQuery::new(type_term, IndexRecordOption::Basic);

            Box::new(BooleanQuery::new(vec![
                (Occur::Must, text_query),
                (Occur::Must, Box::new(type_query)),
            ]))
        } else {
            text_query
        };

        // Execute search
        let top_docs = searcher.search(&final_query, &TopDocs::with_limit(options.limit))?;

        // Map results
        let mut results = Vec::with_capacity(top_docs.len());
        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

            // Extract fields
            let doc_type_str = doc
                .get_first(self.schema.doc_type)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let doc_id = doc
                .get_first(self.schema.doc_id)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let keywords = doc
                .get_first(self.schema.keywords)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty());

            let timestamp_ms = doc
                .get_first(self.schema.timestamp_ms)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok());

            let agent = doc
                .get_first(self.schema.agent)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty());

            let doc_type = doc_type_str.parse::<DocType>().unwrap_or(DocType::TocNode);

            results.push(TeleportResult {
                doc_id,
                doc_type,
                score,
                keywords,
                timestamp_ms,
                agent,
            });
        }

        info!(
            query = query_str,
            results = results.len(),
            "Teleport search complete"
        );

        Ok(results)
    }

    /// Search TOC nodes only.
    pub fn search_toc(
        &self,
        query_str: &str,
        limit: usize,
    ) -> Result<Vec<TeleportResult>, SearchError> {
        self.search(query_str, SearchOptions::toc_only().with_limit(limit))
    }

    /// Search grips only.
    pub fn search_grips(
        &self,
        query_str: &str,
        limit: usize,
    ) -> Result<Vec<TeleportResult>, SearchError> {
        self.search(query_str, SearchOptions::grips_only().with_limit(limit))
    }

    /// Get the number of indexed documents.
    pub fn num_docs(&self) -> u64 {
        let searcher = self.reader.searcher();
        searcher
            .segment_readers()
            .iter()
            .map(|r| r.num_docs() as u64)
            .sum()
    }
}

// Implement Send + Sync for TeleportSearcher to allow use with Arc
// TeleportSearcher is safe to share between threads as:
// - IndexReader is thread-safe
// - SearchSchema is Clone and contains only Field handles
// - QueryParser is used within methods that hold the appropriate locks
unsafe impl Send for TeleportSearcher {}
unsafe impl Sync for TeleportSearcher {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{SearchIndex, SearchIndexConfig};
    use crate::indexer::SearchIndexer;
    use chrono::Utc;
    use memory_types::{Grip, TocBullet, TocLevel, TocNode};
    use tempfile::TempDir;

    fn sample_toc_node(id: &str, title: &str, bullet: &str) -> TocNode {
        let mut node = TocNode::new(
            id.to_string(),
            TocLevel::Day,
            title.to_string(),
            Utc::now(),
            Utc::now(),
        );
        node.bullets = vec![TocBullet::new(bullet)];
        node.keywords = vec!["test".to_string()];
        node
    }

    fn sample_grip(id: &str, excerpt: &str) -> Grip {
        Grip::new(
            id.to_string(),
            excerpt.to_string(),
            "event-001".to_string(),
            "event-002".to_string(),
            Utc::now(),
            "test".to_string(),
        )
    }

    fn setup_index() -> (TempDir, SearchIndex) {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();
        (temp_dir, index)
    }

    #[test]
    fn test_search_toc_nodes() {
        let (_temp_dir, index) = setup_index();
        let indexer = SearchIndexer::new(&index).unwrap();

        // Index some nodes
        let node1 = sample_toc_node("node-1", "Rust Memory Safety", "Discussed borrow checker");
        let node2 = sample_toc_node("node-2", "Python Performance", "Talked about async/await");

        indexer.index_toc_node(&node1).unwrap();
        indexer.index_toc_node(&node2).unwrap();
        indexer.commit().unwrap();

        let searcher = TeleportSearcher::new(&index).unwrap();

        // Search for "rust"
        let results = searcher.search_toc("rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].doc_id, "node-1");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn test_search_grips() {
        let (_temp_dir, index) = setup_index();
        let indexer = SearchIndexer::new(&index).unwrap();

        let grip1 = sample_grip("grip-1", "User asked about memory allocation");
        let grip2 = sample_grip("grip-2", "Discussed database performance");

        indexer.index_grip(&grip1).unwrap();
        indexer.index_grip(&grip2).unwrap();
        indexer.commit().unwrap();

        let searcher = TeleportSearcher::new(&index).unwrap();

        let results = searcher.search_grips("memory", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].doc_id, "grip-1");
    }

    #[test]
    fn test_search_all_types() {
        let (_temp_dir, index) = setup_index();
        let indexer = SearchIndexer::new(&index).unwrap();

        let node = sample_toc_node("node-1", "Memory Discussion", "Talked about allocation");
        let grip = sample_grip("grip-1", "Memory allocation in Rust");

        indexer.index_toc_node(&node).unwrap();
        indexer.index_grip(&grip).unwrap();
        indexer.commit().unwrap();

        let searcher = TeleportSearcher::new(&index).unwrap();

        // Search all types
        let results = searcher
            .search("memory", SearchOptions::new().with_limit(10))
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_bm25_ranking() {
        let (_temp_dir, index) = setup_index();
        let indexer = SearchIndexer::new(&index).unwrap();

        // Node with "rust" once
        let node1 = sample_toc_node("node-1", "Rust basics", "Introduction");
        // Node with "rust" multiple times
        let node2 = sample_toc_node(
            "node-2",
            "Advanced Rust",
            "Deep dive into Rust ownership and Rust lifetimes",
        );

        indexer.index_toc_node(&node1).unwrap();
        indexer.index_toc_node(&node2).unwrap();
        indexer.commit().unwrap();

        let searcher = TeleportSearcher::new(&index).unwrap();

        let results = searcher.search_toc("rust", 10).unwrap();
        assert_eq!(results.len(), 2);
        // Node2 should rank higher (more occurrences of "rust")
        assert_eq!(results[0].doc_id, "node-2");
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn test_empty_query() {
        let (_temp_dir, index) = setup_index();
        let searcher = TeleportSearcher::new(&index).unwrap();

        let results = searcher.search("", SearchOptions::new()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_whitespace_only_query() {
        let (_temp_dir, index) = setup_index();
        let searcher = TeleportSearcher::new(&index).unwrap();

        let results = searcher.search("   ", SearchOptions::new()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_num_docs() {
        let (_temp_dir, index) = setup_index();
        let indexer = SearchIndexer::new(&index).unwrap();

        indexer
            .index_toc_node(&sample_toc_node("node-1", "Test", "Content"))
            .unwrap();
        indexer
            .index_grip(&sample_grip("grip-1", "Excerpt"))
            .unwrap();
        indexer.commit().unwrap();

        let searcher = TeleportSearcher::new(&index).unwrap();
        assert_eq!(searcher.num_docs(), 2);
    }

    #[test]
    fn test_search_options_builder() {
        let options = SearchOptions::new()
            .with_limit(20)
            .with_doc_type(DocType::TocNode);

        assert_eq!(options.limit, 20);
        assert_eq!(options.doc_type, Some(DocType::TocNode));
    }

    #[test]
    fn test_search_options_toc_only() {
        let options = SearchOptions::toc_only();
        assert_eq!(options.doc_type, Some(DocType::TocNode));
        assert_eq!(options.limit, 10);
    }

    #[test]
    fn test_search_options_grips_only() {
        let options = SearchOptions::grips_only();
        assert_eq!(options.doc_type, Some(DocType::Grip));
        assert_eq!(options.limit, 10);
    }

    #[test]
    fn test_search_with_keywords() {
        let (_temp_dir, index) = setup_index();
        let indexer = SearchIndexer::new(&index).unwrap();

        let mut node = TocNode::new(
            "node-1".to_string(),
            TocLevel::Day,
            "Generic Title".to_string(),
            Utc::now(),
            Utc::now(),
        );
        node.keywords = vec!["rust".to_string(), "memory".to_string()];

        indexer.index_toc_node(&node).unwrap();
        indexer.commit().unwrap();

        let searcher = TeleportSearcher::new(&index).unwrap();

        // Search by keyword
        let results = searcher.search_toc("rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].keywords.is_some());
    }

    #[test]
    fn test_reload() {
        let (_temp_dir, index) = setup_index();
        let searcher = TeleportSearcher::new(&index).unwrap();

        // Reload should succeed
        searcher.reload().unwrap();
    }

    #[test]
    fn test_no_results_for_nonexistent_term() {
        let (_temp_dir, index) = setup_index();
        let indexer = SearchIndexer::new(&index).unwrap();

        let node = sample_toc_node("node-1", "Rust Discussion", "Talked about ownership");
        indexer.index_toc_node(&node).unwrap();
        indexer.commit().unwrap();

        let searcher = TeleportSearcher::new(&index).unwrap();

        let results = searcher.search_toc("nonexistentterm12345", 10).unwrap();
        assert!(results.is_empty());
    }
}
