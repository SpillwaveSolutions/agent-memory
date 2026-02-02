//! LLM-enhanced topic labeling with keyword fallback.
//!
//! Provides LLM-based label generation with automatic fallback to
//! keyword-based labeling when LLM is unavailable or fails.

use crate::config::LabelingConfig;
use crate::error::TopicsError;
use crate::labeling::{ClusterDocument, KeywordLabeler, TopicLabel, TopicLabeler};

/// Trait for LLM completion.
///
/// Implement this trait to provide LLM-based label generation.
/// The implementation should handle API calls, rate limiting, and error handling.
pub trait LlmClient: Send + Sync {
    /// Generate a completion for the given prompt.
    ///
    /// Returns the generated text or an error.
    fn complete(&self, prompt: &str) -> Result<String, TopicsError>;
}

/// LLM-enhanced topic labeler with keyword fallback.
///
/// Uses an optional LLM client for sophisticated label generation,
/// falling back to keyword-based labeling when:
/// - LLM client is not provided
/// - LLM call fails
/// - `fallback_to_keywords` is configured
pub struct LlmLabeler<L: LlmClient> {
    /// Optional LLM client
    llm: Option<L>,
    /// Keyword-based fallback labeler
    keyword_fallback: KeywordLabeler,
    /// Configuration
    config: LabelingConfig,
}

impl<L: LlmClient> LlmLabeler<L> {
    /// Create a new LLM labeler with an optional client.
    pub fn new(llm: Option<L>, config: LabelingConfig) -> Self {
        let keyword_fallback = KeywordLabeler::new(config.clone());
        Self {
            llm,
            keyword_fallback,
            config,
        }
    }

    /// Create an LLM labeler with a client.
    pub fn with_llm(llm: L, config: LabelingConfig) -> Self {
        Self::new(Some(llm), config)
    }

    /// Create an LLM labeler without a client (keyword-only).
    pub fn without_llm(config: LabelingConfig) -> Self {
        Self::new(None, config)
    }

    /// Generate a prompt for the LLM.
    fn generate_prompt(&self, documents: &[ClusterDocument]) -> String {
        let samples: Vec<&str> = documents
            .iter()
            .take(5) // Limit context size
            .map(|d| d.text.as_str())
            .collect();

        let sample_text = samples.join("\n---\n");

        format!(
            r#"Generate a concise topic label (2-5 words) for the following cluster of related documents.
The label should capture the main theme or concept.

Documents:
{}

Respond with ONLY the topic label, nothing else."#,
            sample_text
        )
    }

    /// Parse LLM response into a label.
    fn parse_response(&self, response: &str) -> String {
        // Clean up response: trim, remove quotes, limit length
        let cleaned = response.trim().trim_matches('"').trim_matches('\'').trim();

        // Truncate if needed
        if cleaned.len() > self.config.max_label_length {
            if let Some(last_space) = cleaned[..self.config.max_label_length].rfind(' ') {
                return cleaned[..last_space].to_string();
            }
            return cleaned[..self.config.max_label_length].to_string();
        }

        cleaned.to_string()
    }

    /// Label using LLM.
    fn label_with_llm(
        &self,
        llm: &L,
        documents: &[ClusterDocument],
    ) -> Result<TopicLabel, TopicsError> {
        let prompt = self.generate_prompt(documents);
        let response = llm.complete(&prompt)?;
        let label = self.parse_response(&response);

        // Get keywords from fallback for metadata
        let keyword_result = self.keyword_fallback.label_cluster(documents)?;

        Ok(TopicLabel::new(
            label,
            keyword_result.keywords,
            0.85, // LLM labels get higher default confidence
        ))
    }
}

impl<L: LlmClient> TopicLabeler for LlmLabeler<L> {
    fn label_cluster(&self, documents: &[ClusterDocument]) -> Result<TopicLabel, TopicsError> {
        // If LLM is not enabled or not available, use keywords
        if !self.config.use_llm || self.llm.is_none() {
            return self.keyword_fallback.label_cluster(documents);
        }

        // Try LLM labeling
        if let Some(ref llm) = self.llm {
            match self.label_with_llm(llm, documents) {
                Ok(label) => return Ok(label),
                Err(e) => {
                    tracing::warn!("LLM labeling failed: {}, falling back to keywords", e);

                    // Fall back to keywords if configured
                    if self.config.fallback_to_keywords {
                        return self.keyword_fallback.label_cluster(documents);
                    }

                    return Err(e);
                }
            }
        }

        // Should not reach here, but fallback just in case
        self.keyword_fallback.label_cluster(documents)
    }
}

/// A no-op LLM client for testing and keyword-only mode.
///
/// Always returns an error, forcing fallback to keyword labeling.
pub struct NoOpLlmClient;

impl LlmClient for NoOpLlmClient {
    fn complete(&self, _prompt: &str) -> Result<String, TopicsError> {
        Err(TopicsError::InvalidConfig("No LLM configured".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test LLM client that returns a fixed response.
    struct MockLlmClient {
        response: String,
    }

    impl MockLlmClient {
        fn new(response: &str) -> Self {
            Self {
                response: response.to_string(),
            }
        }
    }

    impl LlmClient for MockLlmClient {
        fn complete(&self, _prompt: &str) -> Result<String, TopicsError> {
            Ok(self.response.clone())
        }
    }

    /// Test LLM client that always fails.
    struct FailingLlmClient;

    impl LlmClient for FailingLlmClient {
        fn complete(&self, _prompt: &str) -> Result<String, TopicsError> {
            Err(TopicsError::Embedding("LLM API error".to_string()))
        }
    }

    fn make_doc(id: &str, text: &str) -> ClusterDocument {
        ClusterDocument::new(id.to_string(), text.to_string())
    }

    #[test]
    fn test_llm_labeler_with_mock() {
        let config = LabelingConfig::default();
        let mock = MockLlmClient::new("Machine Learning");
        let labeler = LlmLabeler::with_llm(mock, config);

        let docs = vec![
            make_doc("d1", "deep learning neural networks"),
            make_doc("d2", "machine learning algorithms"),
        ];

        let result = labeler.label_cluster(&docs).unwrap();
        assert_eq!(result.label, "Machine Learning");
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn test_llm_labeler_fallback_on_failure() {
        let config = LabelingConfig {
            use_llm: true,
            fallback_to_keywords: true,
            ..Default::default()
        };
        let labeler = LlmLabeler::with_llm(FailingLlmClient, config);

        let docs = vec![
            make_doc("d1", "rust programming systems"),
            make_doc("d2", "rust memory safety"),
        ];

        // Should fall back to keywords and succeed
        let result = labeler.label_cluster(&docs).unwrap();
        assert!(!result.label.is_empty());
    }

    #[test]
    fn test_llm_labeler_without_llm() {
        let config = LabelingConfig::default();
        let labeler: LlmLabeler<NoOpLlmClient> = LlmLabeler::without_llm(config);

        let docs = vec![make_doc("d1", "rust programming language")];

        // Should use keyword labeling
        let result = labeler.label_cluster(&docs).unwrap();
        assert!(!result.label.is_empty());
    }

    #[test]
    fn test_llm_labeler_disabled() {
        let config = LabelingConfig {
            use_llm: false,
            ..Default::default()
        };
        let mock = MockLlmClient::new("Should Not Use This");
        let labeler = LlmLabeler::with_llm(mock, config);

        let docs = vec![make_doc("d1", "python scripting automation")];

        // Should use keyword labeling since LLM is disabled
        let result = labeler.label_cluster(&docs).unwrap();
        assert_ne!(result.label, "Should Not Use This");
    }

    #[test]
    fn test_llm_labeler_no_fallback() {
        let config = LabelingConfig {
            use_llm: true,
            fallback_to_keywords: false,
            ..Default::default()
        };
        let labeler = LlmLabeler::with_llm(FailingLlmClient, config);

        let docs = vec![make_doc("d1", "some text")];

        // Should fail without fallback
        let result = labeler.label_cluster(&docs);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_prompt() {
        let config = LabelingConfig::default();
        let labeler: LlmLabeler<NoOpLlmClient> = LlmLabeler::without_llm(config);

        let docs = vec![
            make_doc("d1", "First document about rust"),
            make_doc("d2", "Second document about programming"),
        ];

        let prompt = labeler.generate_prompt(&docs);
        assert!(prompt.contains("First document"));
        assert!(prompt.contains("Second document"));
        assert!(prompt.contains("2-5 words"));
    }

    #[test]
    fn test_parse_response_clean() {
        let config = LabelingConfig::default();
        let labeler: LlmLabeler<NoOpLlmClient> = LlmLabeler::without_llm(config);

        assert_eq!(
            labeler.parse_response("Machine Learning"),
            "Machine Learning"
        );
        assert_eq!(
            labeler.parse_response("  Rust Programming  "),
            "Rust Programming"
        );
        assert_eq!(labeler.parse_response("\"Quoted Label\""), "Quoted Label");
    }

    #[test]
    fn test_parse_response_truncate() {
        let config = LabelingConfig {
            max_label_length: 20,
            ..Default::default()
        };
        let labeler: LlmLabeler<NoOpLlmClient> = LlmLabeler::without_llm(config);

        let long_response = "This is a very long topic label that needs truncation";
        let parsed = labeler.parse_response(long_response);
        assert!(parsed.len() <= 20);
    }

    #[test]
    fn test_noop_client() {
        let client = NoOpLlmClient;
        let result = client.complete("test prompt");
        assert!(result.is_err());
    }
}
