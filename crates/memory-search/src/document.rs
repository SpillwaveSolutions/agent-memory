//! Document mapping from domain types to Tantivy documents.
//!
//! Converts TocNode and Grip into indexable Tantivy documents.

use tantivy::doc;
use tantivy::TantivyDocument;

use memory_types::{Grip, TocNode};

use crate::schema::{DocType, SearchSchema};

/// Convert a TocNode to a Tantivy document.
///
/// Text field contains: title + all bullet texts
/// Keywords field contains: joined keywords
pub fn toc_node_to_doc(schema: &SearchSchema, node: &TocNode) -> TantivyDocument {
    // Combine title and bullets for searchable text
    let mut text_parts = vec![node.title.clone()];
    for bullet in &node.bullets {
        text_parts.push(bullet.text.clone());
    }
    let text = text_parts.join(" ");

    // Join keywords with space
    let keywords = node.keywords.join(" ");

    // Timestamp in milliseconds
    let timestamp = node.start_time.timestamp_millis().to_string();

    // Use first contributing agent as the primary agent
    let agent = node
        .contributing_agents
        .first()
        .cloned()
        .unwrap_or_default();

    doc!(
        schema.doc_type => DocType::TocNode.as_str(),
        schema.doc_id => node.node_id.clone(),
        schema.level => node.level.to_string(),
        schema.text => text,
        schema.keywords => keywords,
        schema.timestamp_ms => timestamp,
        schema.agent => agent
    )
}

/// Convert a Grip to a Tantivy document.
///
/// Text field contains: excerpt
/// Level field is empty (not applicable to grips)
pub fn grip_to_doc(schema: &SearchSchema, grip: &Grip) -> TantivyDocument {
    let timestamp = grip.timestamp.timestamp_millis().to_string();

    doc!(
        schema.doc_type => DocType::Grip.as_str(),
        schema.doc_id => grip.grip_id.clone(),
        schema.level => "",  // Not applicable for grips
        schema.text => grip.excerpt.clone(),
        schema.keywords => "",  // Grips don't have keywords
        schema.timestamp_ms => timestamp,
        schema.agent => ""  // Grips inherit agent from parent node
    )
}

/// Extract text content from a TocNode for indexing.
///
/// Returns combined title and bullet text.
pub fn extract_toc_text(node: &TocNode) -> String {
    let mut parts = vec![node.title.clone()];
    for bullet in &node.bullets {
        parts.push(bullet.text.clone());
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::build_teleport_schema;
    use chrono::Utc;
    use memory_types::{TocBullet, TocLevel};
    use tantivy::schema::Value;

    fn sample_toc_node() -> TocNode {
        let mut node = TocNode::new(
            "toc:day:2024-01-15".to_string(),
            TocLevel::Day,
            "Monday, January 15, 2024".to_string(),
            Utc::now(),
            Utc::now(),
        );
        node.bullets = vec![
            TocBullet::new("Discussed Rust memory safety"),
            TocBullet::new("Implemented authentication flow"),
        ];
        node.keywords = vec!["rust".to_string(), "memory".to_string(), "auth".to_string()];
        node
    }

    fn sample_grip() -> Grip {
        Grip::new(
            "grip-12345".to_string(),
            "User asked about borrow checker semantics".to_string(),
            "event-001".to_string(),
            "event-003".to_string(),
            Utc::now(),
            "segment_summarizer".to_string(),
        )
    }

    #[test]
    fn test_toc_node_to_doc() {
        let schema = build_teleport_schema();
        let node = sample_toc_node();

        let doc = toc_node_to_doc(&schema, &node);

        // Verify doc_type
        let doc_type = doc.get_first(schema.doc_type).unwrap();
        assert_eq!(doc_type.as_str(), Some("toc_node"));

        // Verify doc_id
        let doc_id = doc.get_first(schema.doc_id).unwrap();
        assert_eq!(doc_id.as_str(), Some("toc:day:2024-01-15"));

        // Verify text contains title and bullets
        let text = doc.get_first(schema.text).unwrap();
        let text_str = text.as_str().unwrap();
        assert!(text_str.contains("Monday, January 15, 2024"));
        assert!(text_str.contains("Rust memory safety"));
    }

    #[test]
    fn test_grip_to_doc() {
        let schema = build_teleport_schema();
        let grip = sample_grip();

        let doc = grip_to_doc(&schema, &grip);

        let doc_type = doc.get_first(schema.doc_type).unwrap();
        assert_eq!(doc_type.as_str(), Some("grip"));

        let text = doc.get_first(schema.text).unwrap();
        assert!(text.as_str().unwrap().contains("borrow checker"));
    }

    #[test]
    fn test_extract_toc_text() {
        let node = sample_toc_node();
        let text = extract_toc_text(&node);

        assert!(text.contains("Monday, January 15, 2024"));
        assert!(text.contains("Discussed Rust memory safety"));
        assert!(text.contains("Implemented authentication flow"));
    }

    #[test]
    fn test_toc_node_with_empty_bullets() {
        let schema = build_teleport_schema();
        let node = TocNode::new(
            "toc:day:2024-01-15".to_string(),
            TocLevel::Day,
            "A simple title".to_string(),
            Utc::now(),
            Utc::now(),
        );

        let doc = toc_node_to_doc(&schema, &node);

        let text = doc.get_first(schema.text).unwrap();
        assert_eq!(text.as_str(), Some("A simple title"));
    }

    #[test]
    fn test_toc_node_with_empty_keywords() {
        let schema = build_teleport_schema();
        let mut node = TocNode::new(
            "toc:day:2024-01-15".to_string(),
            TocLevel::Day,
            "A simple title".to_string(),
            Utc::now(),
            Utc::now(),
        );
        node.keywords = vec![];

        let doc = toc_node_to_doc(&schema, &node);

        let keywords = doc.get_first(schema.keywords).unwrap();
        assert_eq!(keywords.as_str(), Some(""));
    }

    #[test]
    fn test_grip_doc_level_is_empty() {
        let schema = build_teleport_schema();
        let grip = sample_grip();

        let doc = grip_to_doc(&schema, &grip);

        let level = doc.get_first(schema.level).unwrap();
        assert_eq!(level.as_str(), Some(""));
    }
}
