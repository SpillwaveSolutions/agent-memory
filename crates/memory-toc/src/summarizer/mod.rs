//! Summarization trait and implementations.
//!
//! Per SUMM-01: Pluggable Summarizer trait (async, supports API and local LLM).
//! Per SUMM-02: Generates title, bullets, keywords from events.
//! Per SUMM-03: Extracts grips from events during summarization.
//! Per SUMM-04: Rollup summarizer aggregates child node summaries.

mod api;
mod grip_extractor;
mod mock;

pub use api::{ApiSummarizer, ApiSummarizerConfig};
pub use grip_extractor::{extract_grips, ExtractedGrip, GripExtractor, GripExtractorConfig};
pub use mock::MockSummarizer;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use memory_types::Event;

/// Error type for summarization operations.
#[derive(Debug, Error)]
pub enum SummarizerError {
    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Failed to parse API response: {0}")]
    ParseError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("No events to summarize")]
    NoEvents,
}

/// Output from summarization.
///
/// Per SUMM-02: Contains title, bullets, and keywords.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    /// Brief title capturing the main topic (5-10 words)
    pub title: String,

    /// Key points from the conversation (3-5 bullets)
    pub bullets: Vec<String>,

    /// Keywords for search and filtering (3-7 keywords)
    pub keywords: Vec<String>,
}

impl Summary {
    /// Create a new summary.
    pub fn new(title: String, bullets: Vec<String>, keywords: Vec<String>) -> Self {
        Self {
            title,
            bullets,
            keywords,
        }
    }

    /// Create an empty/placeholder summary.
    pub fn empty() -> Self {
        Self {
            title: String::new(),
            bullets: Vec::new(),
            keywords: Vec::new(),
        }
    }
}

/// Pluggable summarizer trait.
///
/// Per SUMM-01: Async trait supporting API and local LLM.
#[async_trait]
pub trait Summarizer: Send + Sync {
    /// Generate a summary from conversation events.
    ///
    /// Per SUMM-02: Generates title, bullets, keywords.
    async fn summarize_events(&self, events: &[Event]) -> Result<Summary, SummarizerError>;

    /// Generate a rollup summary from child summaries.
    ///
    /// Per SUMM-04: Aggregates child node summaries for parent TOC nodes.
    async fn summarize_children(&self, summaries: &[Summary]) -> Result<Summary, SummarizerError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_creation() {
        let summary = Summary::new(
            "Discussed authentication".to_string(),
            vec![
                "Implemented JWT".to_string(),
                "Fixed token refresh".to_string(),
            ],
            vec!["auth".to_string(), "jwt".to_string()],
        );

        assert_eq!(summary.title, "Discussed authentication");
        assert_eq!(summary.bullets.len(), 2);
        assert_eq!(summary.keywords.len(), 2);
    }

    #[test]
    fn test_summary_empty() {
        let summary = Summary::empty();
        assert!(summary.title.is_empty());
        assert!(summary.bullets.is_empty());
        assert!(summary.keywords.is_empty());
    }

    #[test]
    fn test_summary_serialization() {
        let summary = Summary::new(
            "Test".to_string(),
            vec!["Bullet 1".to_string()],
            vec!["keyword".to_string()],
        );

        let json = serde_json::to_string(&summary).unwrap();
        let decoded: Summary = serde_json::from_str(&json).unwrap();

        assert_eq!(summary.title, decoded.title);
    }
}
