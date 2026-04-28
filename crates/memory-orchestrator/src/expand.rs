//! Heuristic query expansion.
//!
//! Generates multiple query variants to improve recall across
//! BM25 and vector indexes without requiring an LLM call.

/// Expand a query into 1-3 heuristic variants.
///
/// Always includes the original. Adds simple rewrites:
/// - lowercase variant if original has uppercase
/// - drops leading question words for keyword bias
pub fn expand_query(query: &str) -> Vec<String> {
    if query.is_empty() {
        return vec![query.to_string()];
    }

    let mut variants = vec![query.to_string()];

    // Lowercase variant (helps BM25 match case-insensitive terms)
    let lower = query.to_lowercase();
    if lower != query {
        variants.push(lower.clone());
    }

    // Strip leading question words to produce a keyword-biased variant
    let stripped = lower
        .trim_start_matches("what ")
        .trim_start_matches("how ")
        .trim_start_matches("why ")
        .trim_start_matches("when ")
        .trim_start_matches("where ")
        .trim_start_matches("did we ")
        .trim_start_matches("do we ")
        .to_string();

    if stripped != lower && !stripped.is_empty() {
        variants.push(stripped);
    }

    variants.dedup();
    variants
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expansion_always_includes_original() {
        let expanded = expand_query("JWT authentication bug");
        assert!(expanded.contains(&"JWT authentication bug".to_string()));
    }

    #[test]
    fn test_expansion_returns_multiple_variants() {
        let expanded = expand_query("what did we decide");
        assert!(expanded.len() >= 2);
    }

    #[test]
    fn test_expansion_empty_query() {
        let expanded = expand_query("");
        assert_eq!(expanded, vec!["".to_string()]);
    }

    #[test]
    fn test_expansion_lowercase_variant() {
        let expanded = expand_query("What Happened");
        assert!(expanded.contains(&"What Happened".to_string()));
        assert!(expanded.contains(&"what happened".to_string()));
    }

    #[test]
    fn test_expansion_strips_question_words() {
        let expanded = expand_query("how does authentication work");
        assert!(expanded.contains(&"how does authentication work".to_string()));
        // Should have keyword variant with "how " stripped
        assert!(expanded.contains(&"does authentication work".to_string()));
    }

    #[test]
    fn test_expansion_no_duplicate_for_lowercase_input() {
        let expanded = expand_query("simple query");
        // Already lowercase, no question words => only the original
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0], "simple query");
    }
}
