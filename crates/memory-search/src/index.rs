//! Tantivy index management.
//!
//! Handles index creation, opening, and lifecycle.

use std::path::{Path, PathBuf};

use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy};
use tracing::{debug, info};

use crate::error::SearchError;
use crate::schema::{build_teleport_schema, SearchSchema};

/// Default memory budget for IndexWriter (50MB)
const DEFAULT_WRITER_MEMORY_MB: usize = 50;

/// Search index configuration
#[derive(Debug, Clone)]
pub struct SearchIndexConfig {
    /// Path to index directory
    pub index_path: PathBuf,
    /// Memory budget for writer in MB
    pub writer_memory_mb: usize,
}

impl Default for SearchIndexConfig {
    fn default() -> Self {
        Self {
            index_path: PathBuf::from("./bm25-index"),
            writer_memory_mb: DEFAULT_WRITER_MEMORY_MB,
        }
    }
}

impl SearchIndexConfig {
    pub fn new(index_path: impl Into<PathBuf>) -> Self {
        Self {
            index_path: index_path.into(),
            writer_memory_mb: DEFAULT_WRITER_MEMORY_MB,
        }
    }

    pub fn with_memory_mb(mut self, mb: usize) -> Self {
        self.writer_memory_mb = mb;
        self
    }
}

/// Wrapper for Tantivy index with schema access.
pub struct SearchIndex {
    index: Index,
    schema: SearchSchema,
    config: SearchIndexConfig,
}

impl SearchIndex {
    /// Open existing index or create new one.
    pub fn open_or_create(config: SearchIndexConfig) -> Result<Self, SearchError> {
        let index = open_or_create_index(&config.index_path)?;
        let schema = SearchSchema::from_schema(index.schema())?;

        info!(path = ?config.index_path, "Opened search index");

        Ok(Self {
            index,
            schema,
            config,
        })
    }

    /// Get the search schema
    pub fn schema(&self) -> &SearchSchema {
        &self.schema
    }

    /// Get the underlying Tantivy index
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Create an IndexWriter with configured memory budget
    pub fn writer(&self) -> Result<IndexWriter, SearchError> {
        let memory_budget = self.config.writer_memory_mb * 1024 * 1024;
        let writer = self.index.writer(memory_budget)?;
        debug!(
            memory_mb = self.config.writer_memory_mb,
            "Created index writer"
        );
        Ok(writer)
    }

    /// Create an IndexReader with OnCommit reload policy
    pub fn reader(&self) -> Result<IndexReader, SearchError> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        debug!("Created index reader");
        Ok(reader)
    }

    /// Get the index path
    pub fn path(&self) -> &Path {
        &self.config.index_path
    }

    /// Check if index exists at the configured path
    pub fn exists(&self) -> bool {
        self.config.index_path.join("meta.json").exists()
    }
}

/// Open an existing index or create a new one.
///
/// Uses MmapDirectory for persistence.
pub fn open_or_create_index(path: &Path) -> Result<Index, SearchError> {
    if path.join("meta.json").exists() {
        debug!(path = ?path, "Opening existing index");
        let index = Index::open_in_dir(path)?;
        Ok(index)
    } else {
        info!(path = ?path, "Creating new index");
        std::fs::create_dir_all(path)?;
        let schema = build_teleport_schema();
        let index = Index::create_in_dir(path, schema.schema().clone())?;
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_new_index() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());

        let index = SearchIndex::open_or_create(config).unwrap();
        assert!(index.exists());
    }

    #[test]
    fn test_reopen_existing_index() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());

        // Create index
        let _index1 = SearchIndex::open_or_create(config.clone()).unwrap();

        // Reopen
        let index2 = SearchIndex::open_or_create(config).unwrap();
        assert!(index2.exists());
    }

    #[test]
    fn test_create_writer_and_reader() {
        let temp_dir = TempDir::new().unwrap();
        let config = SearchIndexConfig::new(temp_dir.path());
        let index = SearchIndex::open_or_create(config).unwrap();

        let _writer = index.writer().unwrap();
        let _reader = index.reader().unwrap();
    }

    #[test]
    fn test_config_default() {
        let config = SearchIndexConfig::default();
        assert_eq!(config.index_path, PathBuf::from("./bm25-index"));
        assert_eq!(config.writer_memory_mb, DEFAULT_WRITER_MEMORY_MB);
    }

    #[test]
    fn test_config_with_memory() {
        let config = SearchIndexConfig::new("/tmp/test").with_memory_mb(100);
        assert_eq!(config.writer_memory_mb, 100);
    }
}
