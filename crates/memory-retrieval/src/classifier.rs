//! Intent classification for queries.
//!
//! This module implements the `IntentClassifier` which analyzes query text
//! to determine the user's intent (Explore, Answer, Locate, TimeBoxed).
//!
//! Per PRD Section 3: Query Intent Classification

use std::collections::HashSet;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::types::QueryIntent;

/// Result of intent classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// The classified intent
    pub intent: QueryIntent,

    /// Confidence score (0.0-1.0)
    pub confidence: f32,

    /// Explanation of why this intent was chosen
    pub reason: String,

    /// Extracted time constraint, if any (for TimeBoxed)
    pub time_constraint: Option<TimeConstraint>,

    /// Keywords that influenced the classification
    pub matched_keywords: Vec<String>,
}

/// Time constraint extracted from query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeConstraint {
    /// Deadline in milliseconds (relative to now)
    pub deadline_ms: Option<u64>,

    /// Time range lookback (e.g., "yesterday" -> 1 day)
    pub lookback: Option<Duration>,

    /// Raw text that indicated the constraint
    pub source: String,
}

/// Configuration for intent classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifierConfig {
    /// Keywords that indicate Explore intent
    pub explore_keywords: Vec<String>,

    /// Keywords that indicate Answer intent
    pub answer_keywords: Vec<String>,

    /// Keywords that indicate Locate intent
    pub locate_keywords: Vec<String>,

    /// Time-related patterns
    pub time_patterns: Vec<String>,

    /// Default intent when no strong signal
    pub default_intent: QueryIntent,

    /// Minimum confidence to report a match
    pub min_confidence: f32,
}

impl Default for ClassifierConfig {
    fn default() -> Self {
        Self {
            explore_keywords: vec![
                // Pattern words per PRD
                "themes".to_string(),
                "topics".to_string(),
                "working on".to_string(),
                "been doing".to_string(),
                // Additional explore signals
                "explore".to_string(),
                "discover".to_string(),
                "patterns".to_string(),
                "recurring".to_string(),
                "overview".to_string(),
                "summary".to_string(),
                "show me".to_string(),
                "what have".to_string(),
                "related".to_string(),
                "connections".to_string(),
            ],
            answer_keywords: vec![
                // Question words per PRD
                "how".to_string(),
                "why".to_string(),
                "what was".to_string(),
                "what is".to_string(),
                // Additional answer signals
                "explain".to_string(),
                "describe".to_string(),
                "tell me".to_string(),
                "when did".to_string(),
                "who".to_string(),
                "decided".to_string(),
                "solution".to_string(),
                "fix".to_string(),
                "resolve".to_string(),
            ],
            locate_keywords: vec![
                // Location words per PRD
                "where".to_string(),
                "find".to_string(),
                "locate".to_string(),
                // Additional locate signals
                "exact".to_string(),
                "specific".to_string(),
                "quote".to_string(),
                "snippet".to_string(),
                "definition".to_string(),
                "defined".to_string(),
                "config".to_string(),
                "error message".to_string(),
                "line".to_string(),
                "search for".to_string(),
            ],
            time_patterns: vec![
                "yesterday".to_string(),
                "today".to_string(),
                "last week".to_string(),
                "last month".to_string(),
                "this week".to_string(),
                "this month".to_string(),
                "recent".to_string(),
                "latest".to_string(),
                "in the past".to_string(),
                "hours ago".to_string(),
                "days ago".to_string(),
                "minutes ago".to_string(),
            ],
            default_intent: QueryIntent::Answer,
            min_confidence: 0.3,
        }
    }
}

/// Intent classifier using keyword heuristics.
///
/// Per PRD Section 3: Classifies queries as Explore/Answer/Locate/TimeBoxed.
pub struct IntentClassifier {
    config: ClassifierConfig,
    explore_set: HashSet<String>,
    answer_set: HashSet<String>,
    locate_set: HashSet<String>,
    time_set: HashSet<String>,
}

impl IntentClassifier {
    /// Create a new classifier with default configuration.
    pub fn new() -> Self {
        Self::with_config(ClassifierConfig::default())
    }

    /// Create a classifier with custom configuration.
    pub fn with_config(config: ClassifierConfig) -> Self {
        let explore_set: HashSet<String> = config
            .explore_keywords
            .iter()
            .map(|s| s.to_lowercase())
            .collect();
        let answer_set: HashSet<String> = config
            .answer_keywords
            .iter()
            .map(|s| s.to_lowercase())
            .collect();
        let locate_set: HashSet<String> = config
            .locate_keywords
            .iter()
            .map(|s| s.to_lowercase())
            .collect();
        let time_set: HashSet<String> = config
            .time_patterns
            .iter()
            .map(|s| s.to_lowercase())
            .collect();

        Self {
            config,
            explore_set,
            answer_set,
            locate_set,
            time_set,
        }
    }

    /// Classify the intent of a query.
    pub fn classify(&self, query: &str) -> ClassificationResult {
        let query_lower = query.to_lowercase();

        // Extract time constraint first
        let time_constraint = self.extract_time_constraint(&query_lower);

        // Count keyword matches for each intent
        let mut explore_matches = Vec::new();
        let mut answer_matches = Vec::new();
        let mut locate_matches = Vec::new();

        for keyword in &self.explore_set {
            if query_lower.contains(keyword) {
                explore_matches.push(keyword.clone());
            }
        }

        for keyword in &self.answer_set {
            if query_lower.contains(keyword) {
                answer_matches.push(keyword.clone());
            }
        }

        for keyword in &self.locate_set {
            if query_lower.contains(keyword) {
                locate_matches.push(keyword.clone());
            }
        }

        // Calculate scores (weighted by specificity)
        let explore_score = self.calculate_score(&explore_matches);
        let answer_score = self.calculate_score(&answer_matches);
        let locate_score = self.calculate_score(&locate_matches);

        debug!(
            query = query,
            explore_score = explore_score,
            answer_score = answer_score,
            locate_score = locate_score,
            "Intent classification scores"
        );

        // Determine winner
        let (intent, confidence, matched, reason) = self.determine_intent(
            explore_score,
            answer_score,
            locate_score,
            &explore_matches,
            &answer_matches,
            &locate_matches,
            &time_constraint,
        );

        ClassificationResult {
            intent,
            confidence,
            reason,
            time_constraint,
            matched_keywords: matched,
        }
    }

    /// Classify with an explicit timeout constraint (force TimeBoxed).
    pub fn classify_with_timeout(&self, query: &str, timeout: Duration) -> ClassificationResult {
        let mut result = self.classify(query);

        // Override to TimeBoxed if timeout is specified
        result.intent = QueryIntent::TimeBoxed;
        result.time_constraint = Some(TimeConstraint {
            deadline_ms: Some(timeout.as_millis() as u64),
            lookback: None,
            source: format!("explicit timeout: {}ms", timeout.as_millis()),
        });
        result.reason = format!(
            "TimeBoxed due to explicit timeout constraint ({}ms)",
            timeout.as_millis()
        );

        result
    }

    fn calculate_score(&self, matches: &[String]) -> f32 {
        if matches.is_empty() {
            return 0.0;
        }

        // Base score from number of matches
        let base = (matches.len() as f32).min(3.0) / 3.0;

        // Bonus for longer/more specific keywords
        let specificity_bonus: f32 = matches
            .iter()
            .map(|k| if k.len() > 5 { 0.1 } else { 0.0 })
            .sum();

        (base + specificity_bonus).min(1.0)
    }

    #[allow(clippy::too_many_arguments)]
    fn determine_intent(
        &self,
        explore_score: f32,
        answer_score: f32,
        locate_score: f32,
        explore_matches: &[String],
        answer_matches: &[String],
        locate_matches: &[String],
        time_constraint: &Option<TimeConstraint>,
    ) -> (QueryIntent, f32, Vec<String>, String) {
        let max_score = explore_score.max(answer_score).max(locate_score);

        // If no strong signal, use default
        if max_score < self.config.min_confidence {
            return (
                self.config.default_intent,
                0.5, // Medium confidence for default
                vec![],
                "No strong intent signal; defaulting to Answer".to_string(),
            );
        }

        // Check if time-boxed (deadline constraint from skill context)
        // This is typically set by the caller, not extracted from query
        if let Some(tc) = time_constraint {
            if tc.deadline_ms.is_some() {
                return (
                    QueryIntent::TimeBoxed,
                    0.9,
                    vec![tc.source.clone()],
                    format!("TimeBoxed due to time constraint: {}", tc.source),
                );
            }
        }

        // Determine winner based on scores
        if explore_score >= answer_score && explore_score >= locate_score {
            (
                QueryIntent::Explore,
                explore_score,
                explore_matches.to_vec(),
                format!(
                    "Explore intent: matched keywords [{}]",
                    explore_matches.join(", ")
                ),
            )
        } else if locate_score >= answer_score {
            (
                QueryIntent::Locate,
                locate_score,
                locate_matches.to_vec(),
                format!(
                    "Locate intent: matched keywords [{}]",
                    locate_matches.join(", ")
                ),
            )
        } else {
            (
                QueryIntent::Answer,
                answer_score,
                answer_matches.to_vec(),
                format!(
                    "Answer intent: matched keywords [{}]",
                    answer_matches.join(", ")
                ),
            )
        }
    }

    fn extract_time_constraint(&self, query_lower: &str) -> Option<TimeConstraint> {
        // Check for "N days/hours/minutes ago" patterns first (more specific)
        if let Some(duration) = self.extract_relative_time(query_lower) {
            return Some(TimeConstraint {
                deadline_ms: None,
                lookback: Some(duration),
                source: "relative time expression".to_string(),
            });
        }

        // Check for general time patterns
        for pattern in &self.time_set {
            if query_lower.contains(pattern) {
                let lookback = self.pattern_to_duration(pattern);
                return Some(TimeConstraint {
                    deadline_ms: None, // Lookback constraint, not deadline
                    lookback,
                    source: pattern.clone(),
                });
            }
        }

        None
    }

    fn pattern_to_duration(&self, pattern: &str) -> Option<Duration> {
        match pattern {
            "yesterday" | "today" => Some(Duration::from_secs(24 * 60 * 60)),
            "last week" | "this week" => Some(Duration::from_secs(7 * 24 * 60 * 60)),
            "last month" | "this month" => Some(Duration::from_secs(30 * 24 * 60 * 60)),
            "recent" | "latest" => Some(Duration::from_secs(3 * 24 * 60 * 60)), // 3 days
            _ => None,
        }
    }

    fn extract_relative_time(&self, query: &str) -> Option<Duration> {
        // Simple regex-like pattern matching for "N units ago"
        let patterns = [
            ("minutes ago", 60u64),
            ("hours ago", 3600),
            ("days ago", 86400),
        ];

        for (suffix, multiplier) in patterns {
            if let Some(pos) = query.find(suffix) {
                // Look for a number before the suffix
                let before = &query[..pos].trim_end();
                if let Some(last_word) = before.split_whitespace().last() {
                    if let Ok(n) = last_word.parse::<u64>() {
                        return Some(Duration::from_secs(n * multiplier));
                    }
                }
            }
        }

        None
    }
}

impl Default for IntentClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_explore() {
        let classifier = IntentClassifier::new();

        let result = classifier.classify("What have I been working on lately?");
        assert_eq!(result.intent, QueryIntent::Explore);
        assert!(!result.matched_keywords.is_empty());

        let result = classifier.classify("Show me the themes in my conversations");
        assert_eq!(result.intent, QueryIntent::Explore);

        let result = classifier.classify("What topics have been recurring?");
        assert_eq!(result.intent, QueryIntent::Explore);
    }

    #[test]
    fn test_classify_answer() {
        let classifier = IntentClassifier::new();

        let result = classifier.classify("How did we fix the JWT bug?");
        assert_eq!(result.intent, QueryIntent::Answer);
        assert!(!result.matched_keywords.is_empty());

        let result = classifier.classify("Why was that decision made?");
        assert_eq!(result.intent, QueryIntent::Answer);

        let result = classifier.classify("What was the solution to the auth issue?");
        assert_eq!(result.intent, QueryIntent::Answer);
    }

    #[test]
    fn test_classify_locate() {
        let classifier = IntentClassifier::new();

        let result = classifier.classify("Where did I define the config?");
        assert_eq!(result.intent, QueryIntent::Locate);
        assert!(!result.matched_keywords.is_empty());

        let result = classifier.classify("Find the exact error message");
        assert_eq!(result.intent, QueryIntent::Locate);

        let result = classifier.classify("Locate the database config snippet");
        assert_eq!(result.intent, QueryIntent::Locate);
    }

    #[test]
    fn test_classify_default() {
        let classifier = IntentClassifier::new();

        // Ambiguous query should default to Answer
        let result = classifier.classify("memory stuff");
        assert_eq!(result.intent, QueryIntent::Answer);
    }

    #[test]
    fn test_classify_with_timeout() {
        let classifier = IntentClassifier::new();

        let result = classifier.classify_with_timeout("Find something", Duration::from_millis(500));
        assert_eq!(result.intent, QueryIntent::TimeBoxed);
        assert!(result.time_constraint.is_some());
        assert_eq!(result.time_constraint.unwrap().deadline_ms, Some(500));
    }

    #[test]
    fn test_time_constraint_extraction() {
        let classifier = IntentClassifier::new();

        let result = classifier.classify("What did we discuss yesterday?");
        assert!(result.time_constraint.is_some());
        assert_eq!(result.time_constraint.as_ref().unwrap().source, "yesterday");

        let result = classifier.classify("Find conversations from last week");
        assert!(result.time_constraint.is_some());
        assert_eq!(result.time_constraint.as_ref().unwrap().source, "last week");
    }

    #[test]
    fn test_relative_time_extraction() {
        let classifier = IntentClassifier::new();

        let result = classifier.classify("What happened 5 hours ago?");
        assert!(result.time_constraint.is_some());
        let tc = result.time_constraint.unwrap();
        assert_eq!(tc.lookback, Some(Duration::from_secs(5 * 3600)));

        let result = classifier.classify("Find stuff from 3 days ago");
        assert!(result.time_constraint.is_some());
        let tc = result.time_constraint.unwrap();
        assert_eq!(tc.lookback, Some(Duration::from_secs(3 * 86400)));
    }

    #[test]
    fn test_classification_confidence() {
        let classifier = IntentClassifier::new();

        // Strong signal should have high confidence
        let result = classifier.classify("Where can I find and locate the config definition?");
        assert!(result.confidence >= 0.5);

        // Weak signal should have lower confidence
        let result = classifier.classify("stuff");
        assert!(result.confidence <= 0.6);
    }

    #[test]
    fn test_custom_config() {
        let mut config = ClassifierConfig::default();
        config.explore_keywords.push("investigate".to_string());

        let classifier = IntentClassifier::with_config(config);

        let result = classifier.classify("I want to investigate the patterns");
        assert_eq!(result.intent, QueryIntent::Explore);
    }
}
