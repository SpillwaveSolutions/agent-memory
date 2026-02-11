//! Vector metadata storage.
//!
//! Maps internal vector IDs (u64) to document IDs (node_id or grip_id).
//! Stored in RocksDB for persistence and atomic updates.

use std::path::Path;

use rocksdb::{ColumnFamily, ColumnFamilyDescriptor, Options, DB};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::VectorError;

/// Column family name for vector metadata
pub const CF_VECTOR_META: &str = "vector_meta";

/// Document type for vectors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocType {
    /// TOC node summary
    TocNode,
    /// Grip excerpt
    Grip,
}

impl DocType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocType::TocNode => "toc_node",
            DocType::Grip => "grip",
        }
    }
}

/// Vector entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    /// Internal vector ID (key in HNSW index)
    pub vector_id: u64,
    /// Document type
    pub doc_type: DocType,
    /// Document ID (node_id or grip_id)
    pub doc_id: String,
    /// Timestamp when vector was created (ms since epoch)
    pub created_at: i64,
    /// Text that was embedded (truncated for storage)
    pub text_preview: String,
    /// Agent attribution (from TocNode.contributing_agents or event metadata)
    #[serde(default)]
    pub agent: Option<String>,
}

impl VectorEntry {
    pub fn new(
        vector_id: u64,
        doc_type: DocType,
        doc_id: impl Into<String>,
        created_at: i64,
        text: &str,
    ) -> Self {
        const MAX_PREVIEW: usize = 200;
        let text_preview = if text.len() > MAX_PREVIEW {
            format!("{}...", &text[..MAX_PREVIEW])
        } else {
            text.to_string()
        };

        Self {
            vector_id,
            doc_type,
            doc_id: doc_id.into(),
            created_at,
            text_preview,
            agent: None,
        }
    }

    /// Set agent attribution (builder pattern).
    pub fn with_agent(mut self, agent: Option<String>) -> Self {
        self.agent = agent;
        self
    }
}

/// Vector metadata storage using RocksDB.
pub struct VectorMetadata {
    db: DB,
}

impl VectorMetadata {
    /// Open or create metadata storage.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, VectorError> {
        let path = path.as_ref();

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_opts = Options::default();
        let cf = ColumnFamilyDescriptor::new(CF_VECTOR_META, cf_opts);

        let db = DB::open_cf_descriptors(&opts, path, vec![cf])?;

        info!(path = ?path, "Opened vector metadata storage");
        Ok(Self { db })
    }

    /// Get the column family handle
    fn cf(&self) -> &ColumnFamily {
        self.db
            .cf_handle(CF_VECTOR_META)
            .expect("CF_VECTOR_META missing")
    }

    /// Store vector entry metadata.
    pub fn put(&self, entry: &VectorEntry) -> Result<(), VectorError> {
        let key = entry.vector_id.to_be_bytes();
        let value =
            serde_json::to_vec(entry).map_err(|e| VectorError::Serialization(e.to_string()))?;

        self.db.put_cf(self.cf(), key, value)?;
        debug!(vector_id = entry.vector_id, doc_id = %entry.doc_id, "Stored metadata");
        Ok(())
    }

    /// Get vector entry by vector ID.
    pub fn get(&self, vector_id: u64) -> Result<Option<VectorEntry>, VectorError> {
        let key = vector_id.to_be_bytes();
        match self.db.get_cf(self.cf(), key)? {
            Some(bytes) => {
                let entry: VectorEntry = serde_json::from_slice(&bytes)
                    .map_err(|e| VectorError::Serialization(e.to_string()))?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Delete vector entry by vector ID.
    pub fn delete(&self, vector_id: u64) -> Result<(), VectorError> {
        let key = vector_id.to_be_bytes();
        self.db.delete_cf(self.cf(), key)?;
        Ok(())
    }

    /// Get all entries for a document type.
    pub fn get_by_type(&self, doc_type: DocType) -> Result<Vec<VectorEntry>, VectorError> {
        let mut entries = Vec::new();
        let iter = self.db.iterator_cf(self.cf(), rocksdb::IteratorMode::Start);

        for item in iter {
            let (_, value) = item?;
            let entry: VectorEntry = serde_json::from_slice(&value)
                .map_err(|e| VectorError::Serialization(e.to_string()))?;
            if entry.doc_type == doc_type {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Find vector ID for a document ID.
    pub fn find_by_doc_id(&self, doc_id: &str) -> Result<Option<VectorEntry>, VectorError> {
        let iter = self.db.iterator_cf(self.cf(), rocksdb::IteratorMode::Start);

        for item in iter {
            let (_, value) = item?;
            let entry: VectorEntry = serde_json::from_slice(&value)
                .map_err(|e| VectorError::Serialization(e.to_string()))?;
            if entry.doc_id == doc_id {
                return Ok(Some(entry));
            }
        }

        Ok(None)
    }

    /// Count total entries
    pub fn count(&self) -> Result<usize, VectorError> {
        let iter = self.db.iterator_cf(self.cf(), rocksdb::IteratorMode::Start);
        Ok(iter.count())
    }

    /// Get all entries.
    ///
    /// Returns all vector metadata entries in the store.
    /// Use with caution on large indexes.
    pub fn get_all(&self) -> Result<Vec<VectorEntry>, VectorError> {
        let mut entries = Vec::new();
        let iter = self.db.iterator_cf(self.cf(), rocksdb::IteratorMode::Start);

        for item in iter {
            let (_, value) = item?;
            let entry: VectorEntry = serde_json::from_slice(&value)
                .map_err(|e| VectorError::Serialization(e.to_string()))?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Clear all entries from metadata storage.
    ///
    /// Used during full rebuild operations.
    pub fn clear(&self) -> Result<(), VectorError> {
        // Collect all keys first to avoid iterator invalidation
        let keys: Vec<Vec<u8>> = self
            .db
            .iterator_cf(self.cf(), rocksdb::IteratorMode::Start)
            .filter_map(|item| item.ok().map(|(k, _)| k.to_vec()))
            .collect();

        // Delete each key
        for key in keys {
            self.db.delete_cf(self.cf(), &key)?;
        }

        debug!("Cleared all vector metadata entries");
        Ok(())
    }

    /// Get the next available vector ID
    pub fn next_vector_id(&self) -> Result<u64, VectorError> {
        let iter = self.db.iterator_cf(self.cf(), rocksdb::IteratorMode::End);

        if let Some(Ok((key, _))) = iter.into_iter().next() {
            let id = u64::from_be_bytes(key[..8].try_into().unwrap());
            Ok(id + 1)
        } else {
            Ok(1) // Start from 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_put_and_get() {
        let temp = TempDir::new().unwrap();
        let meta = VectorMetadata::open(temp.path()).unwrap();

        let entry = VectorEntry::new(
            1,
            DocType::TocNode,
            "toc:day:2024-01-15",
            1705320000000,
            "This is a test summary for the day",
        );

        meta.put(&entry).unwrap();

        let retrieved = meta.get(1).unwrap().unwrap();
        assert_eq!(retrieved.vector_id, 1);
        assert_eq!(retrieved.doc_id, "toc:day:2024-01-15");
        assert_eq!(retrieved.doc_type, DocType::TocNode);
    }

    #[test]
    fn test_find_by_doc_id() {
        let temp = TempDir::new().unwrap();
        let meta = VectorMetadata::open(temp.path()).unwrap();

        for i in 0..5 {
            let entry = VectorEntry::new(
                i,
                DocType::Grip,
                format!("grip:123456:{}", i),
                1705320000000,
                "Test excerpt",
            );
            meta.put(&entry).unwrap();
        }

        let found = meta.find_by_doc_id("grip:123456:3").unwrap().unwrap();
        assert_eq!(found.vector_id, 3);
    }

    #[test]
    fn test_next_vector_id() {
        let temp = TempDir::new().unwrap();
        let meta = VectorMetadata::open(temp.path()).unwrap();

        assert_eq!(meta.next_vector_id().unwrap(), 1);

        let entry = VectorEntry::new(42, DocType::TocNode, "test", 0, "test");
        meta.put(&entry).unwrap();

        assert_eq!(meta.next_vector_id().unwrap(), 43);
    }

    #[test]
    fn test_get_by_type() {
        let temp = TempDir::new().unwrap();
        let meta = VectorMetadata::open(temp.path()).unwrap();

        // Add mixed entries
        meta.put(&VectorEntry::new(1, DocType::TocNode, "toc:1", 0, "toc"))
            .unwrap();
        meta.put(&VectorEntry::new(2, DocType::Grip, "grip:1", 0, "grip"))
            .unwrap();
        meta.put(&VectorEntry::new(3, DocType::TocNode, "toc:2", 0, "toc"))
            .unwrap();

        let toc_entries = meta.get_by_type(DocType::TocNode).unwrap();
        assert_eq!(toc_entries.len(), 2);

        let grip_entries = meta.get_by_type(DocType::Grip).unwrap();
        assert_eq!(grip_entries.len(), 1);
    }

    #[test]
    fn test_delete() {
        let temp = TempDir::new().unwrap();
        let meta = VectorMetadata::open(temp.path()).unwrap();

        let entry = VectorEntry::new(1, DocType::TocNode, "test", 0, "test");
        meta.put(&entry).unwrap();
        assert!(meta.get(1).unwrap().is_some());

        meta.delete(1).unwrap();
        assert!(meta.get(1).unwrap().is_none());
    }

    #[test]
    fn test_count() {
        let temp = TempDir::new().unwrap();
        let meta = VectorMetadata::open(temp.path()).unwrap();

        assert_eq!(meta.count().unwrap(), 0);

        for i in 0..5 {
            let entry = VectorEntry::new(i, DocType::Grip, format!("grip:{}", i), 0, "test");
            meta.put(&entry).unwrap();
        }

        assert_eq!(meta.count().unwrap(), 5);
    }

    #[test]
    fn test_text_preview_truncation() {
        let long_text = "x".repeat(500);
        let entry = VectorEntry::new(1, DocType::TocNode, "test", 0, &long_text);
        assert!(entry.text_preview.len() < 250); // 200 + "..."
        assert!(entry.text_preview.ends_with("..."));
    }

    #[test]
    fn test_get_all() {
        let temp = TempDir::new().unwrap();
        let meta = VectorMetadata::open(temp.path()).unwrap();

        // Add some entries
        for i in 0..3 {
            let entry = VectorEntry::new(i, DocType::TocNode, format!("toc:{}", i), 0, "test");
            meta.put(&entry).unwrap();
        }

        let all = meta.get_all().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_clear() {
        let temp = TempDir::new().unwrap();
        let meta = VectorMetadata::open(temp.path()).unwrap();

        // Add some entries
        for i in 0..5 {
            let entry = VectorEntry::new(i, DocType::Grip, format!("grip:{}", i), 0, "test");
            meta.put(&entry).unwrap();
        }
        assert_eq!(meta.count().unwrap(), 5);

        // Clear all
        meta.clear().unwrap();
        assert_eq!(meta.count().unwrap(), 0);
        assert!(meta.get_all().unwrap().is_empty());
    }
}
