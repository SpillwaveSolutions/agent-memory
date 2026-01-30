//! Mock summarizer for testing.

use async_trait::async_trait;

use memory_types::Event;

use super::{Summary, Summarizer, SummarizerError};

/// Mock summarizer that generates deterministic summaries.
///
/// Useful for testing without making API calls.
pub struct MockSummarizer {
    /// Prefix for generated titles
    title_prefix: String,
}

impl MockSummarizer {
    /// Create a new mock summarizer.
    pub fn new() -> Self {
        Self {
            title_prefix: "Summary of".to_string(),
        }
    }

    /// Create with custom title prefix.
    pub fn with_title_prefix(prefix: impl Into<String>) -> Self {
        Self {
            title_prefix: prefix.into(),
        }
    }
}

impl Default for MockSummarizer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Summarizer for MockSummarizer {
    async fn summarize_events(&self, events: &[Event]) -> Result<Summary, SummarizerError> {
        if events.is_empty() {
            return Err(SummarizerError::NoEvents);
        }

        // Extract some info from events for mock summary
        let first_event = &events[0];
        let last_event = &events[events.len() - 1];

        let title = format!(
            "{} {} events",
            self.title_prefix,
            events.len()
        );

        let bullets = vec![
            format!("First message: {}", truncate(&first_event.text, 50)),
            format!("Last message: {}", truncate(&last_event.text, 50)),
            format!("Total events: {}", events.len()),
        ];

        // Extract keywords from event text
        let keywords = extract_mock_keywords(events);

        Ok(Summary::new(title, bullets, keywords))
    }

    async fn summarize_children(&self, summaries: &[Summary]) -> Result<Summary, SummarizerError> {
        if summaries.is_empty() {
            return Err(SummarizerError::NoEvents);
        }

        let title = format!(
            "{} {} child summaries",
            self.title_prefix,
            summaries.len()
        );

        // Collect bullets from children (first bullet from each)
        let bullets: Vec<String> = summaries
            .iter()
            .filter_map(|s| s.bullets.first().cloned())
            .take(5)
            .collect();

        // Merge keywords from all children
        let mut all_keywords: Vec<String> = summaries
            .iter()
            .flat_map(|s| s.keywords.clone())
            .collect();
        all_keywords.sort();
        all_keywords.dedup();
        let keywords = all_keywords.into_iter().take(7).collect();

        Ok(Summary::new(title, bullets, keywords))
    }
}

/// Truncate text to max length, adding "..." if truncated.
fn truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}

/// Extract mock keywords from events (simple word extraction).
fn extract_mock_keywords(events: &[Event]) -> Vec<String> {
    let all_text: String = events.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join(" ");

    // Simple keyword extraction: split by whitespace, filter short words
    let words: Vec<String> = all_text
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .map(|w| w.to_lowercase())
        .filter(|w| !is_stopword(w))
        .collect();

    // Count and sort by frequency
    let mut word_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for word in words {
        *word_counts.entry(word).or_insert(0) += 1;
    }

    let mut sorted: Vec<_> = word_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    sorted.into_iter().take(5).map(|(w, _)| w).collect()
}

/// Check if word is a common stopword.
fn is_stopword(word: &str) -> bool {
    const STOPWORDS: &[&str] = &[
        "the", "and", "for", "that", "this", "with", "from", "have", "has",
        "been", "were", "will", "would", "could", "should", "there", "their",
        "what", "when", "where", "which", "about", "into", "through",
    ];
    STOPWORDS.contains(&word)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use memory_types::{EventRole, EventType};

    fn create_test_event(text: &str) -> Event {
        Event::new(
            ulid::Ulid::new().to_string(),
            "session-123".to_string(),
            Utc::now(),
            EventType::UserMessage,
            EventRole::User,
            text.to_string(),
        )
    }

    #[tokio::test]
    async fn test_mock_summarize_events() {
        let summarizer = MockSummarizer::new();
        let events = vec![
            create_test_event("How do I implement authentication?"),
            create_test_event("Use JWT tokens for stateless auth"),
        ];

        let summary = summarizer.summarize_events(&events).await.unwrap();

        assert!(summary.title.contains("2 events"));
        assert_eq!(summary.bullets.len(), 3);
        assert!(!summary.keywords.is_empty());
    }

    #[tokio::test]
    async fn test_mock_summarize_empty() {
        let summarizer = MockSummarizer::new();
        let result = summarizer.summarize_events(&[]).await;
        assert!(matches!(result, Err(SummarizerError::NoEvents)));
    }

    #[tokio::test]
    async fn test_mock_summarize_children() {
        let summarizer = MockSummarizer::new();
        let summaries = vec![
            Summary::new(
                "Day 1".to_string(),
                vec!["Worked on auth".to_string()],
                vec!["auth".to_string()],
            ),
            Summary::new(
                "Day 2".to_string(),
                vec!["Fixed bugs".to_string()],
                vec!["bugs".to_string()],
            ),
        ];

        let rollup = summarizer.summarize_children(&summaries).await.unwrap();

        assert!(rollup.title.contains("2 child summaries"));
        assert!(rollup.keywords.contains(&"auth".to_string()));
    }

    #[tokio::test]
    async fn test_mock_custom_prefix() {
        let summarizer = MockSummarizer::with_title_prefix("Overview of");
        let events = vec![create_test_event("Test event")];

        let summary = summarizer.summarize_events(&events).await.unwrap();

        assert!(summary.title.starts_with("Overview of"));
    }
}
