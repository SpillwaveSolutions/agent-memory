//! Topic labeling using keyword extraction.
//!
//! Provides label generation for topic clusters using TF-IDF keyword extraction.

use crate::config::LabelingConfig;
use crate::error::TopicsError;
use crate::tfidf::TfIdf;

/// Document within a cluster for labeling.
#[derive(Debug, Clone)]
pub struct ClusterDocument {
    /// Document identifier
    pub doc_id: String,
    /// Full text content
    pub text: String,
    /// Pre-extracted keywords (optional)
    pub keywords: Vec<String>,
}

impl ClusterDocument {
    /// Create a new cluster document.
    pub fn new(doc_id: String, text: String) -> Self {
        Self {
            doc_id,
            text,
            keywords: Vec::new(),
        }
    }

    /// Create a cluster document with pre-extracted keywords.
    pub fn with_keywords(doc_id: String, text: String, keywords: Vec<String>) -> Self {
        Self {
            doc_id,
            text,
            keywords,
        }
    }
}

/// Generated topic label with metadata.
#[derive(Debug, Clone)]
pub struct TopicLabel {
    /// Human-readable label (2-5 words)
    pub label: String,
    /// Top keywords for this topic
    pub keywords: Vec<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}

impl TopicLabel {
    /// Create a new topic label.
    pub fn new(label: String, keywords: Vec<String>, confidence: f32) -> Self {
        Self {
            label,
            keywords,
            confidence,
        }
    }
}

/// Trait for generating topic labels from cluster documents.
pub trait TopicLabeler: Send + Sync {
    /// Generate a label for a cluster of documents.
    fn label_cluster(&self, documents: &[ClusterDocument]) -> Result<TopicLabel, TopicsError>;
}

/// Keyword-based topic labeler using TF-IDF.
///
/// This is the default labeler that requires no external dependencies.
/// It extracts top keywords using TF-IDF scoring and generates a concise label.
pub struct KeywordLabeler {
    config: LabelingConfig,
}

impl KeywordLabeler {
    /// Create a new keyword labeler.
    pub fn new(config: LabelingConfig) -> Self {
        Self { config }
    }

    /// Extract keywords using TF-IDF scoring.
    ///
    /// Returns keywords sorted by TF-IDF score (highest first).
    fn extract_keywords(&self, documents: &[ClusterDocument]) -> Vec<(String, f32)> {
        if documents.is_empty() {
            return Vec::new();
        }

        // Collect all texts
        let texts: Vec<&str> = documents.iter().map(|d| d.text.as_str()).collect();

        // Use TF-IDF to extract keywords
        let tfidf = TfIdf::new(&texts);

        // Get aggregated scores across all documents in the cluster
        tfidf.top_terms(self.config.top_keywords * 2) // Get extra for filtering
    }

    /// Generate a label from top keywords.
    ///
    /// Creates a concise 2-5 word label from the most important keywords.
    fn generate_label(&self, keywords: &[(String, f32)]) -> String {
        if keywords.is_empty() {
            return "Unknown Topic".to_string();
        }

        // Take top keywords for label (max 5)
        let label_words: Vec<&str> = keywords
            .iter()
            .take(5)
            .map(|(word, _)| word.as_str())
            .collect();

        // Join with spaces and truncate if needed
        let label = label_words.join(" ");
        self.truncate_label(&label)
    }

    /// Truncate label to max length, breaking at word boundary.
    fn truncate_label(&self, label: &str) -> String {
        if label.len() <= self.config.max_label_length {
            return label.to_string();
        }

        // Find last space before max length
        let truncated = &label[..self.config.max_label_length];
        if let Some(last_space) = truncated.rfind(' ') {
            truncated[..last_space].to_string()
        } else {
            truncated.to_string()
        }
    }

    /// Calculate confidence based on keyword distribution.
    ///
    /// Higher confidence when keywords have distinct high scores.
    fn calculate_confidence(&self, keywords: &[(String, f32)]) -> f32 {
        if keywords.is_empty() {
            return 0.0;
        }
        if keywords.len() == 1 {
            return keywords[0].1.min(1.0);
        }

        // Confidence based on top keyword score relative to sum
        let top_score = keywords[0].1;
        let total_score: f32 = keywords.iter().map(|(_, s)| s).sum();

        if total_score == 0.0 {
            return 0.0;
        }

        // Normalize: if top keyword dominates, confidence is higher
        let ratio = top_score / total_score;

        // Scale to 0.5-1.0 range (having any keywords gives at least 0.5)
        0.5 + (ratio * 0.5)
    }
}

impl TopicLabeler for KeywordLabeler {
    fn label_cluster(&self, documents: &[ClusterDocument]) -> Result<TopicLabel, TopicsError> {
        if documents.is_empty() {
            return Err(TopicsError::InvalidInput(
                "Cannot label empty cluster".to_string(),
            ));
        }

        // Extract keywords using TF-IDF
        let keywords = self.extract_keywords(documents);

        // Generate label from top keywords
        let label = self.generate_label(&keywords);

        // Calculate confidence
        let confidence = self.calculate_confidence(&keywords);

        // Return top N keywords as configured
        let top_keywords: Vec<String> = keywords
            .into_iter()
            .take(self.config.top_keywords)
            .map(|(word, _)| word)
            .collect();

        Ok(TopicLabel::new(label, top_keywords, confidence))
    }
}

impl Default for KeywordLabeler {
    fn default() -> Self {
        Self::new(LabelingConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc(id: &str, text: &str) -> ClusterDocument {
        ClusterDocument::new(id.to_string(), text.to_string())
    }

    #[test]
    fn test_cluster_document_new() {
        let doc = ClusterDocument::new("doc1".to_string(), "Some text".to_string());
        assert_eq!(doc.doc_id, "doc1");
        assert_eq!(doc.text, "Some text");
        assert!(doc.keywords.is_empty());
    }

    #[test]
    fn test_cluster_document_with_keywords() {
        let doc = ClusterDocument::with_keywords(
            "doc1".to_string(),
            "Some text".to_string(),
            vec!["keyword1".to_string(), "keyword2".to_string()],
        );
        assert_eq!(doc.keywords.len(), 2);
    }

    #[test]
    fn test_topic_label_new() {
        let label = TopicLabel::new("Test Label".to_string(), vec!["keyword1".to_string()], 0.85);
        assert_eq!(label.label, "Test Label");
        assert_eq!(label.keywords.len(), 1);
        assert!((label.confidence - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn test_keyword_labeler_empty_documents() {
        let labeler = KeywordLabeler::default();
        let result = labeler.label_cluster(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_keyword_labeler_single_document() {
        let labeler = KeywordLabeler::default();
        let docs = vec![make_doc(
            "d1",
            "machine learning algorithms neural networks deep learning",
        )];

        let result = labeler.label_cluster(&docs).unwrap();
        assert!(!result.label.is_empty());
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn test_keyword_labeler_multiple_documents() {
        let labeler = KeywordLabeler::default();
        let docs = vec![
            make_doc("d1", "machine learning algorithms for classification"),
            make_doc("d2", "deep learning neural networks training"),
            make_doc("d3", "machine learning model optimization techniques"),
        ];

        let result = labeler.label_cluster(&docs).unwrap();
        assert!(!result.label.is_empty());
        assert!(!result.keywords.is_empty());
        assert!(result.confidence >= 0.5);
    }

    #[test]
    fn test_truncate_label_short() {
        let config = LabelingConfig {
            max_label_length: 50,
            ..Default::default()
        };
        let labeler = KeywordLabeler::new(config);

        let label = labeler.truncate_label("short label");
        assert_eq!(label, "short label");
    }

    #[test]
    fn test_truncate_label_long() {
        let config = LabelingConfig {
            max_label_length: 20,
            ..Default::default()
        };
        let labeler = KeywordLabeler::new(config);

        let label = labeler.truncate_label("this is a very long label that needs truncation");
        assert!(label.len() <= 20);
        assert!(!label.ends_with(' ')); // Should break at word boundary
    }

    #[test]
    fn test_generate_label_empty_keywords() {
        let labeler = KeywordLabeler::default();
        let label = labeler.generate_label(&[]);
        assert_eq!(label, "Unknown Topic");
    }

    #[test]
    fn test_calculate_confidence_empty() {
        let labeler = KeywordLabeler::default();
        let confidence = labeler.calculate_confidence(&[]);
        assert!((confidence - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_calculate_confidence_single() {
        let labeler = KeywordLabeler::default();
        let keywords = vec![("word".to_string(), 0.8)];
        let confidence = labeler.calculate_confidence(&keywords);
        assert!((confidence - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_calculate_confidence_dominant_keyword() {
        let labeler = KeywordLabeler::default();
        let keywords = vec![("dominant".to_string(), 0.9), ("minor".to_string(), 0.1)];
        let confidence = labeler.calculate_confidence(&keywords);
        // 0.9 / 1.0 = 0.9, scaled to 0.5 + 0.45 = 0.95
        assert!(confidence > 0.9);
    }

    #[test]
    fn test_calculate_confidence_even_distribution() {
        let labeler = KeywordLabeler::default();
        let keywords = vec![
            ("word1".to_string(), 0.25),
            ("word2".to_string(), 0.25),
            ("word3".to_string(), 0.25),
            ("word4".to_string(), 0.25),
        ];
        let confidence = labeler.calculate_confidence(&keywords);
        // 0.25 / 1.0 = 0.25, scaled to 0.5 + 0.125 = 0.625
        assert!((confidence - 0.625).abs() < 0.01);
    }

    #[test]
    fn test_extract_keywords_returns_scored_terms() {
        let labeler = KeywordLabeler::default();
        let docs = vec![
            make_doc("d1", "rust programming language systems"),
            make_doc("d2", "rust memory safety ownership"),
        ];

        let keywords = labeler.extract_keywords(&docs);
        assert!(!keywords.is_empty());
        // Keywords should be sorted by score (descending)
        for i in 1..keywords.len() {
            assert!(keywords[i - 1].1 >= keywords[i].1);
        }
    }
}
