//! Search error types.

use thiserror::Error;

/// Errors that can occur during search operations.
#[derive(Debug, Error)]
pub enum SearchError {
    /// Tantivy index error
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),

    /// Query parse error
    #[error("Query parse error: {0}")]
    QueryParse(#[from] tantivy::query::QueryParserError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Index not found
    #[error("Index not found at path: {0}")]
    IndexNotFound(String),

    /// Document not found
    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    /// Schema mismatch
    #[error("Schema mismatch: {0}")]
    SchemaMismatch(String),

    /// Index is locked (another process has it open)
    #[error("Index is locked: {0}")]
    IndexLocked(String),
}
