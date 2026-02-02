//! Search functionality for TOC nodes.
//!
//! Provides term-overlap scoring to search within TOC node content
//! without external index dependencies.
//!
//! Per Phase 10.5: This is the "always works" foundation that
//! later phases (BM25, vector) build upon.

use memory_types::TocNode;

/// Represents searchable fields in a TOC node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchField {
    /// The node title
    Title,
    /// Summary derived from bullets
    Summary,
    /// Individual bullet points
    Bullets,
    /// Keywords associated with the node
    Keywords,
}

/// Represents a match result from searching a TOC node.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// Which field matched
    pub field: SearchField,
    /// The text that matched
    pub text: String,
    /// Grip IDs associated with this match (for provenance)
    pub grip_ids: Vec<String>,
    /// Relevance score (0.0 to 1.0)
    pub score: f32,
}

impl SearchMatch {
    /// Create a new search match.
    pub fn new(field: SearchField, text: String, grip_ids: Vec<String>, score: f32) -> Self {
        Self {
            field,
            text,
            grip_ids,
            score,
        }
    }
}

/// Calculate term overlap score (0.0-1.0).
///
/// Returns the ratio of matched terms to total terms.
/// Returns None if no terms match or if terms list is empty.
///
/// # Arguments
/// * `text` - The text to search within
/// * `terms` - Search terms (should be lowercase, >= 3 chars)
///
/// # Example
/// ```
/// use memory_toc::search::term_overlap_score;
///
/// let terms = vec!["jwt".to_string(), "token".to_string()];
/// let score = term_overlap_score("JWT authentication with token refresh", &terms);
/// assert_eq!(score, Some(1.0)); // Both terms match
/// ```
pub fn term_overlap_score(text: &str, terms: &[String]) -> Option<f32> {
    if terms.is_empty() {
        return None;
    }

    let text_lower = text.to_lowercase();
    let matched_count = terms
        .iter()
        .filter(|term| text_lower.contains(term.as_str()))
        .count();

    if matched_count == 0 {
        None
    } else {
        Some(matched_count as f32 / terms.len() as f32)
    }
}

/// Parse query string into normalized search terms.
///
/// - Splits on whitespace
/// - Filters terms shorter than 3 characters
/// - Converts to lowercase
fn parse_query(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .filter(|w| w.len() >= 3)
        .map(|w| w.to_lowercase())
        .collect()
}

/// Check if a search field is enabled.
fn field_enabled(fields: &[SearchField], target: SearchField) -> bool {
    fields.is_empty() || fields.contains(&target)
}

/// Search within a single node's fields for matching terms.
///
/// # Arguments
/// * `node` - The TOC node to search
/// * `query` - Space-separated search terms
/// * `fields` - Which fields to search (empty = all fields)
///
/// # Returns
/// Vector of SearchMatch sorted by score descending
///
/// # Example
/// ```
/// use memory_toc::search::{search_node, SearchField};
/// use memory_types::{TocNode, TocLevel, TocBullet};
/// use chrono::Utc;
///
/// let mut node = TocNode::new(
///     "node:1".to_string(),
///     TocLevel::Day,
///     "JWT Debugging Session".to_string(),
///     Utc::now(),
///     Utc::now(),
/// );
/// node.bullets = vec![TocBullet::new("Fixed token expiration bug")];
///
/// let matches = search_node(&node, "jwt token", &[]);
/// assert!(!matches.is_empty());
/// ```
pub fn search_node(node: &TocNode, query: &str, fields: &[SearchField]) -> Vec<SearchMatch> {
    let terms = parse_query(query);
    if terms.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();

    // Search title
    if field_enabled(fields, SearchField::Title) {
        if let Some(score) = term_overlap_score(&node.title, &terms) {
            matches.push(SearchMatch::new(
                SearchField::Title,
                node.title.clone(),
                Vec::new(),
                score,
            ));
        }
    }

    // Search summary (derived from bullets)
    if field_enabled(fields, SearchField::Summary) {
        let summary: String = node
            .bullets
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        if !summary.is_empty() {
            if let Some(score) = term_overlap_score(&summary, &terms) {
                // Collect all grip IDs from bullets for the summary match
                let grip_ids: Vec<String> = node
                    .bullets
                    .iter()
                    .flat_map(|b| b.grip_ids.clone())
                    .collect();

                matches.push(SearchMatch::new(
                    SearchField::Summary,
                    summary,
                    grip_ids,
                    score,
                ));
            }
        }
    }

    // Search individual bullets
    if field_enabled(fields, SearchField::Bullets) {
        for bullet in &node.bullets {
            if let Some(score) = term_overlap_score(&bullet.text, &terms) {
                matches.push(SearchMatch::new(
                    SearchField::Bullets,
                    bullet.text.clone(),
                    bullet.grip_ids.clone(),
                    score,
                ));
            }
        }
    }

    // Search keywords
    if field_enabled(fields, SearchField::Keywords) {
        for keyword in &node.keywords {
            // Keyword matching: if any term matches the keyword exactly
            // (case-insensitive), score is 1.0
            let keyword_lower = keyword.to_lowercase();
            if terms.iter().any(|term| term == &keyword_lower) {
                matches.push(SearchMatch::new(
                    SearchField::Keywords,
                    keyword.clone(),
                    Vec::new(),
                    1.0,
                ));
            }
        }
    }

    // Sort by score descending
    matches.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use memory_types::{TocBullet, TocLevel};

    fn make_test_node(
        title: &str,
        bullets: Vec<(&str, Vec<&str>)>,
        keywords: Vec<&str>,
    ) -> TocNode {
        let mut node = TocNode::new(
            "test:node:1".to_string(),
            TocLevel::Segment,
            title.to_string(),
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 1, 1, 23, 59, 59).unwrap(),
        );
        node.bullets = bullets
            .into_iter()
            .map(|(text, grips)| {
                TocBullet::new(text).with_grips(grips.into_iter().map(|s| s.to_string()).collect())
            })
            .collect();
        node.keywords = keywords.into_iter().map(|s| s.to_string()).collect();
        node
    }

    #[test]
    fn test_term_overlap_single_match() {
        let terms = vec!["jwt".to_string()];
        let score = term_overlap_score("JWT authentication system", &terms);
        assert!(score.is_some());
        assert_eq!(score.unwrap(), 1.0);
    }

    #[test]
    fn test_term_overlap_partial_match() {
        let terms = vec!["jwt".to_string(), "debugging".to_string()];
        let score = term_overlap_score("JWT authentication", &terms);
        assert!(score.is_some());
        assert_eq!(score.unwrap(), 0.5);
    }

    #[test]
    fn test_term_overlap_no_match() {
        let terms = vec!["vector".to_string(), "embedding".to_string()];
        let score = term_overlap_score("JWT authentication", &terms);
        assert!(score.is_none());
    }

    #[test]
    fn test_term_overlap_empty_terms() {
        let terms: Vec<String> = vec![];
        let score = term_overlap_score("JWT authentication", &terms);
        assert!(score.is_none());
    }

    #[test]
    fn test_term_overlap_case_insensitive() {
        let terms = vec!["jwt".to_string(), "token".to_string()];
        let score = term_overlap_score("JWT TOKEN Authentication", &terms);
        assert!(score.is_some());
        assert_eq!(score.unwrap(), 1.0);
    }

    #[test]
    fn test_parse_query_filters_short_terms() {
        let terms = parse_query("to jwt is the token");
        // "to" (2) and "is" (2) should be filtered, "jwt" (3) and "the" (3) and "token" (5) kept
        assert!(terms.contains(&"jwt".to_string()));
        assert!(terms.contains(&"the".to_string()));
        assert!(terms.contains(&"token".to_string()));
        assert!(!terms.contains(&"to".to_string()));
        assert!(!terms.contains(&"is".to_string()));
    }

    #[test]
    fn test_parse_query_lowercase() {
        let terms = parse_query("JWT Token");
        assert!(terms.contains(&"jwt".to_string()));
        assert!(terms.contains(&"token".to_string()));
    }

    #[test]
    fn test_search_node_title_match() {
        let node = make_test_node("JWT Token Debugging Session", vec![], vec![]);
        let matches = search_node(&node, "jwt debugging", &[SearchField::Title]);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].field, SearchField::Title);
        assert_eq!(matches[0].score, 1.0); // Both terms match
    }

    #[test]
    fn test_search_node_title_partial_match() {
        let node = make_test_node("JWT Token Session", vec![], vec![]);
        let matches = search_node(&node, "jwt debugging", &[SearchField::Title]);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].field, SearchField::Title);
        assert_eq!(matches[0].score, 0.5); // Only "jwt" matches
    }

    #[test]
    fn test_search_node_title_no_match() {
        let node = make_test_node("Database Migration", vec![], vec![]);
        let matches = search_node(&node, "jwt authentication", &[SearchField::Title]);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_search_node_bullet_with_grips() {
        let node = make_test_node(
            "Session",
            vec![("Fixed JWT expiration bug", vec!["grip:123"])],
            vec![],
        );
        let matches = search_node(&node, "jwt bug", &[SearchField::Bullets]);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].field, SearchField::Bullets);
        assert_eq!(matches[0].grip_ids, vec!["grip:123"]);
        assert_eq!(matches[0].score, 1.0);
    }

    #[test]
    fn test_search_node_multiple_bullets() {
        let node = make_test_node(
            "Session",
            vec![
                ("Fixed JWT expiration bug", vec!["grip:1"]),
                ("Added token refresh", vec!["grip:2"]),
                ("Updated documentation", vec!["grip:3"]),
            ],
            vec![],
        );
        let matches = search_node(&node, "jwt token", &[SearchField::Bullets]);
        // First bullet matches "jwt", second matches "token"
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_search_node_keyword_match() {
        let node = make_test_node("Session", vec![], vec!["authentication", "JWT"]);
        let matches = search_node(&node, "jwt", &[SearchField::Keywords]);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].field, SearchField::Keywords);
        assert_eq!(matches[0].score, 1.0);
        assert_eq!(matches[0].text, "JWT");
    }

    #[test]
    fn test_search_node_keyword_no_partial_match() {
        let node = make_test_node("Session", vec![], vec!["authentication"]);
        // "auth" should not match "authentication" for keywords (exact match required)
        let matches = search_node(&node, "auth", &[SearchField::Keywords]);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_search_node_summary_match() {
        let node = make_test_node(
            "Session",
            vec![
                ("Fixed JWT bug", vec!["grip:1"]),
                ("Added token refresh", vec!["grip:2"]),
            ],
            vec![],
        );
        let matches = search_node(&node, "jwt token", &[SearchField::Summary]);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].field, SearchField::Summary);
        // Summary should include both grips
        assert!(matches[0].grip_ids.contains(&"grip:1".to_string()));
        assert!(matches[0].grip_ids.contains(&"grip:2".to_string()));
        assert_eq!(matches[0].score, 1.0);
    }

    #[test]
    fn test_search_node_short_terms_filtered() {
        let node = make_test_node("The JWT Token", vec![], vec![]);
        // "to" (2 chars) is filtered, "jwt" (3 chars) kept
        let matches = search_node(&node, "to jwt", &[SearchField::Title]);
        assert_eq!(matches.len(), 1);
        // Only "jwt" should be used, and it matches
        assert_eq!(matches[0].score, 1.0);
    }

    #[test]
    fn test_search_node_all_terms_filtered() {
        let node = make_test_node("JWT Token", vec![], vec![]);
        // All terms < 3 chars
        let matches = search_node(&node, "to is a", &[SearchField::Title]);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_search_node_all_fields() {
        let node = make_test_node(
            "JWT Session",
            vec![("Implemented authentication", vec!["grip:1"])],
            vec!["token"],
        );
        // Empty fields means search all
        let matches = search_node(&node, "jwt token authentication", &[]);
        // Should find matches in:
        // - Title: "jwt" matches
        // - Summary: "authentication" matches
        // - Bullets: "authentication" matches
        // - Keywords: "token" matches exactly
        assert!(!matches.is_empty());

        let field_types: Vec<SearchField> = matches.iter().map(|m| m.field).collect();
        assert!(field_types.contains(&SearchField::Title));
        assert!(field_types.contains(&SearchField::Summary));
        assert!(field_types.contains(&SearchField::Bullets));
        assert!(field_types.contains(&SearchField::Keywords));
    }

    #[test]
    fn test_search_node_sorted_by_score() {
        let node = make_test_node(
            "Session", // No match
            vec![
                ("JWT debugging today", vec!["grip:1"]), // 1 of 2 = 0.5
                ("JWT authentication and token refresh", vec!["grip:2"]), // 2 of 2 = 1.0
            ],
            vec![],
        );
        let matches = search_node(&node, "jwt authentication", &[SearchField::Bullets]);
        assert_eq!(matches.len(), 2);
        // Higher score should be first
        assert!(matches[0].score >= matches[1].score);
        assert_eq!(matches[0].score, 1.0);
        assert_eq!(matches[1].score, 0.5);
    }

    #[test]
    fn test_search_node_empty_query() {
        let node = make_test_node("JWT Session", vec![], vec![]);
        let matches = search_node(&node, "", &[]);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_search_node_empty_node() {
        let node = make_test_node("", vec![], vec![]);
        let matches = search_node(&node, "jwt token", &[]);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_search_node_grips_propagated_from_bullets() {
        let node = make_test_node(
            "Session",
            vec![
                ("Fixed JWT bug", vec!["grip:aaa", "grip:bbb"]),
                ("Added token refresh", vec!["grip:ccc"]),
            ],
            vec![],
        );

        // Test bullet match includes its grips
        let bullet_matches = search_node(&node, "jwt bug", &[SearchField::Bullets]);
        assert_eq!(bullet_matches.len(), 1);
        assert_eq!(bullet_matches[0].grip_ids.len(), 2);
        assert!(bullet_matches[0].grip_ids.contains(&"grip:aaa".to_string()));
        assert!(bullet_matches[0].grip_ids.contains(&"grip:bbb".to_string()));

        // Test summary match includes all grips
        let summary_matches = search_node(&node, "fixed added", &[SearchField::Summary]);
        assert_eq!(summary_matches.len(), 1);
        assert_eq!(summary_matches[0].grip_ids.len(), 3);
    }
}
