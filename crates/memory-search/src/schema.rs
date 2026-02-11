//! Tantivy schema definition for teleport search.
//!
//! Indexes two document types:
//! - TOC nodes: title + bullets + keywords
//! - Grips: excerpt text

use tantivy::schema::{Field, Schema, STORED, STRING, TEXT};

use crate::SearchError;

/// Document types stored in the index
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocType {
    TocNode,
    Grip,
}

impl DocType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocType::TocNode => "toc_node",
            DocType::Grip => "grip",
        }
    }

    /// Parse from string, returning None for unknown types.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "toc_node" => Some(DocType::TocNode),
            "grip" => Some(DocType::Grip),
            _ => None,
        }
    }
}

impl std::str::FromStr for DocType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("unknown doc type: {}", s))
    }
}

/// Schema field handles for efficient access
#[derive(Debug, Clone)]
pub struct SearchSchema {
    schema: Schema,
    /// Document type: "toc_node" or "grip" (STRING | STORED)
    pub doc_type: Field,
    /// Primary key: node_id or grip_id (STRING | STORED)
    pub doc_id: Field,
    /// TOC level for toc_node: "year", "month", etc. (STRING)
    pub level: Field,
    /// Searchable text: title+bullets for TOC, excerpt for grip (TEXT)
    pub text: Field,
    /// Keywords/tags (TEXT | STORED)
    pub keywords: Field,
    /// Timestamp in milliseconds (STRING | STORED for recency)
    pub timestamp_ms: Field,
    /// Agent attribution (STRING | STORED) - from TocNode.contributing_agents
    pub agent: Field,
}

impl SearchSchema {
    /// Get the underlying Tantivy schema
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Create a SearchSchema from an existing Tantivy Schema
    pub fn from_schema(schema: Schema) -> Result<Self, SearchError> {
        let doc_type = schema
            .get_field("doc_type")
            .map_err(|_| SearchError::SchemaMismatch("missing doc_type field".into()))?;
        let doc_id = schema
            .get_field("doc_id")
            .map_err(|_| SearchError::SchemaMismatch("missing doc_id field".into()))?;
        let level = schema
            .get_field("level")
            .map_err(|_| SearchError::SchemaMismatch("missing level field".into()))?;
        let text = schema
            .get_field("text")
            .map_err(|_| SearchError::SchemaMismatch("missing text field".into()))?;
        let keywords = schema
            .get_field("keywords")
            .map_err(|_| SearchError::SchemaMismatch("missing keywords field".into()))?;
        let timestamp_ms = schema
            .get_field("timestamp_ms")
            .map_err(|_| SearchError::SchemaMismatch("missing timestamp_ms field".into()))?;
        let agent = schema
            .get_field("agent")
            .map_err(|_| SearchError::SchemaMismatch("missing agent field".into()))?;

        Ok(Self {
            schema,
            doc_type,
            doc_id,
            level,
            text,
            keywords,
            timestamp_ms,
            agent,
        })
    }
}

/// Build the teleport search schema.
///
/// Schema fields:
/// - doc_type: STRING | STORED - "toc_node" or "grip"
/// - doc_id: STRING | STORED - node_id or grip_id
/// - level: STRING - TOC level (for filtering)
/// - text: TEXT - searchable content
/// - keywords: TEXT | STORED - keywords/tags
/// - timestamp_ms: STRING | STORED - for recency info
pub fn build_teleport_schema() -> SearchSchema {
    let mut schema_builder = Schema::builder();

    // Document type for filtering: "toc_node" or "grip"
    let doc_type = schema_builder.add_text_field("doc_type", STRING | STORED);

    // Primary key - node_id or grip_id
    let doc_id = schema_builder.add_text_field("doc_id", STRING | STORED);

    // TOC level (for toc_node only): "year", "month", "week", "day", "segment"
    let level = schema_builder.add_text_field("level", STRING | STORED);

    // Searchable text content (title + bullets for TOC, excerpt for grip)
    let text = schema_builder.add_text_field("text", TEXT);

    // Keywords (indexed and stored for retrieval)
    let keywords = schema_builder.add_text_field("keywords", TEXT | STORED);

    // Timestamp for recency (stored as string for simplicity)
    let timestamp_ms = schema_builder.add_text_field("timestamp_ms", STRING | STORED);

    // Agent attribution (from TocNode.contributing_agents)
    let agent = schema_builder.add_text_field("agent", STRING | STORED);

    let schema = schema_builder.build();

    SearchSchema {
        schema,
        doc_type,
        doc_id,
        level,
        text,
        keywords,
        timestamp_ms,
        agent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_schema() {
        let schema = build_teleport_schema();
        assert!(schema.schema.get_field("doc_type").is_ok());
        assert!(schema.schema.get_field("doc_id").is_ok());
        assert!(schema.schema.get_field("text").is_ok());
    }

    #[test]
    fn test_doc_type_conversion() {
        assert_eq!(DocType::TocNode.as_str(), "toc_node");
        assert_eq!(DocType::parse("grip"), Some(DocType::Grip));
        assert_eq!(DocType::parse("invalid"), None);
        // Test FromStr trait
        assert_eq!("toc_node".parse::<DocType>().unwrap(), DocType::TocNode);
        assert!("invalid".parse::<DocType>().is_err());
    }

    #[test]
    fn test_from_schema() {
        let original = build_teleport_schema();
        let rebuilt = SearchSchema::from_schema(original.schema().clone()).unwrap();
        assert_eq!(rebuilt.doc_type, original.doc_type);
        assert_eq!(rebuilt.doc_id, original.doc_id);
        assert_eq!(rebuilt.text, original.text);
    }
}
