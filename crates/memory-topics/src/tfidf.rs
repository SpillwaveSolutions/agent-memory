//! TF-IDF (Term Frequency - Inverse Document Frequency) implementation.
//!
//! Pure Rust implementation for keyword extraction without external dependencies.

use std::collections::{HashMap, HashSet};

/// TF-IDF calculator for keyword extraction.
///
/// Computes term importance based on frequency within documents and rarity
/// across the document corpus.
pub struct TfIdf {
    /// Term -> document count (how many documents contain this term)
    doc_frequencies: HashMap<String, usize>,
    /// Term -> total frequency across all documents
    term_frequencies: HashMap<String, usize>,
    /// Number of documents
    doc_count: usize,
}

impl TfIdf {
    /// Create a new TF-IDF calculator from a corpus of documents.
    pub fn new(documents: &[&str]) -> Self {
        let mut doc_frequencies: HashMap<String, usize> = HashMap::new();
        let mut term_frequencies: HashMap<String, usize> = HashMap::new();
        let doc_count = documents.len();

        for doc in documents {
            let terms = tokenize(doc);
            let unique_terms: HashSet<&String> = terms.iter().collect();

            // Count document frequency (each term counted once per doc)
            for term in unique_terms {
                *doc_frequencies.entry(term.clone()).or_insert(0) += 1;
            }

            // Count total term frequency
            for term in terms {
                *term_frequencies.entry(term).or_insert(0) += 1;
            }
        }

        Self {
            doc_frequencies,
            term_frequencies,
            doc_count,
        }
    }

    /// Calculate TF-IDF score for a term.
    ///
    /// TF-IDF = TF * IDF
    /// - TF (Term Frequency) = count of term / total terms
    /// - IDF (Inverse Document Frequency) = log(N / df) where N = doc count, df = doc frequency
    pub fn score(&self, term: &str) -> f32 {
        let tf = self.term_frequency(term);
        let idf = self.inverse_document_frequency(term);
        tf * idf
    }

    /// Calculate term frequency (normalized by total terms).
    fn term_frequency(&self, term: &str) -> f32 {
        let count = *self.term_frequencies.get(term).unwrap_or(&0) as f32;
        let total: usize = self.term_frequencies.values().sum();
        if total == 0 {
            return 0.0;
        }
        count / total as f32
    }

    /// Calculate inverse document frequency.
    ///
    /// Uses smoothed IDF: log((N + 1) / (df + 1)) + 1
    fn inverse_document_frequency(&self, term: &str) -> f32 {
        let df = *self.doc_frequencies.get(term).unwrap_or(&0) as f32;
        let n = self.doc_count as f32;

        if df == 0.0 {
            return 0.0;
        }

        // Smoothed IDF to avoid division by zero and extreme values
        ((n + 1.0) / (df + 1.0)).ln() + 1.0
    }

    /// Get top N terms by TF-IDF score.
    ///
    /// Returns terms sorted by score (highest first).
    pub fn top_terms(&self, n: usize) -> Vec<(String, f32)> {
        let mut scores: Vec<(String, f32)> = self
            .term_frequencies
            .keys()
            .map(|term| (term.clone(), self.score(term)))
            .filter(|(_, score)| *score > 0.0)
            .collect();

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scores.truncate(n);
        scores
    }

    /// Get all terms with their TF-IDF scores.
    pub fn all_scores(&self) -> HashMap<String, f32> {
        self.term_frequencies
            .keys()
            .map(|term| (term.clone(), self.score(term)))
            .collect()
    }

    /// Get document count.
    pub fn doc_count(&self) -> usize {
        self.doc_count
    }

    /// Get unique term count.
    pub fn term_count(&self) -> usize {
        self.term_frequencies.len()
    }
}

/// Tokenize text into lowercase words.
///
/// Filters out:
/// - Stop words (common English words)
/// - Single character tokens
/// - Numbers
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .filter(|s| s.len() > 1)
        .filter(|s| !is_stop_word(s))
        .filter(|s| !s.chars().all(|c| c.is_numeric()))
        .map(String::from)
        .collect()
}

/// Check if a word is a stop word.
fn is_stop_word(word: &str) -> bool {
    const STOP_WORDS: &[&str] = &[
        "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "he", "in", "is",
        "it", "its", "of", "on", "or", "that", "the", "to", "was", "were", "will", "with", "this",
        "they", "but", "have", "had", "what", "when", "where", "who", "which", "why", "how", "all",
        "each", "every", "both", "few", "more", "most", "other", "some", "such", "no", "nor",
        "not", "only", "own", "same", "so", "than", "too", "very", "can", "just", "should", "now",
        "also", "been", "being", "do", "does", "did", "doing", "would", "could", "might", "must",
        "shall", "about", "above", "after", "again", "against", "am", "any", "before", "below",
        "between", "into", "through", "during", "out", "over", "under", "up", "down", "then",
        "once", "here", "there", "if", "else", "while", "because", "until", "we", "you", "your",
        "our", "their", "him", "her", "them", "me", "my", "myself", "itself", "those", "these",
        "his",
    ];

    STOP_WORDS.contains(&word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("Hello World");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn test_tokenize_removes_stop_words() {
        let tokens = tokenize("the quick brown fox");
        assert!(!tokens.contains(&"the".to_string()));
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"brown".to_string()));
        assert!(tokens.contains(&"fox".to_string()));
    }

    #[test]
    fn test_tokenize_removes_single_chars() {
        let tokens = tokenize("a b c rust");
        assert_eq!(tokens, vec!["rust"]);
    }

    #[test]
    fn test_tokenize_removes_numbers() {
        let tokens = tokenize("rust 123 456 programming");
        assert_eq!(tokens, vec!["rust", "programming"]);
    }

    #[test]
    fn test_tokenize_handles_punctuation() {
        let tokens = tokenize("rust, python, and java!");
        assert!(tokens.contains(&"rust".to_string()));
        assert!(tokens.contains(&"python".to_string()));
        assert!(tokens.contains(&"java".to_string()));
    }

    #[test]
    fn test_is_stop_word() {
        assert!(is_stop_word("the"));
        assert!(is_stop_word("and"));
        assert!(is_stop_word("is"));
        assert!(!is_stop_word("rust"));
        assert!(!is_stop_word("programming"));
    }

    #[test]
    fn test_tfidf_new() {
        let docs = vec!["rust programming", "python programming", "rust systems"];
        let tfidf = TfIdf::new(&docs);

        assert_eq!(tfidf.doc_count(), 3);
        assert!(tfidf.term_count() > 0);
    }

    #[test]
    fn test_tfidf_score_common_term() {
        let docs = vec!["rust programming", "python programming", "java programming"];
        let tfidf = TfIdf::new(&docs);

        // "programming" appears in all docs (3x), rust/python/java each appear once
        // All have same TF (1/6 for each single occurrence, 3/6 for programming)
        // programming: TF=0.5, IDF=ln(4/4)+1 = 1.0, score = 0.5
        // rust: TF=1/6, IDF=ln(4/2)+1 = 1.69, score = 0.28
        let prog_score = tfidf.score("programming");
        let rust_score = tfidf.score("rust");

        // programming has higher TF which outweighs the IDF difference
        assert!(prog_score > 0.0);
        assert!(rust_score > 0.0);
        // High frequency term dominates in TF-IDF
        assert!(prog_score > rust_score);
    }

    #[test]
    fn test_tfidf_score_rare_term() {
        let docs = vec![
            "machine learning algorithms",
            "deep learning neural networks",
            "machine learning models",
        ];
        let tfidf = TfIdf::new(&docs);

        // "neural" only in one doc, "learning" in all
        let neural_score = tfidf.score("neural");
        let learning_score = tfidf.score("learning");

        // Neural should have higher IDF component
        assert!(neural_score > 0.0);
        assert!(learning_score > 0.0);
    }

    #[test]
    fn test_tfidf_score_nonexistent_term() {
        let docs = vec!["rust programming"];
        let tfidf = TfIdf::new(&docs);

        let score = tfidf.score("nonexistent");
        assert!((score - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_top_terms() {
        let docs = vec![
            "rust rust rust systems",
            "python scripting",
            "rust memory safety",
        ];
        let tfidf = TfIdf::new(&docs);

        let top = tfidf.top_terms(3);
        assert!(!top.is_empty());
        assert!(top.len() <= 3);

        // Top terms should be sorted by score
        for i in 1..top.len() {
            assert!(top[i - 1].1 >= top[i].1);
        }
    }

    #[test]
    fn test_top_terms_includes_rust() {
        let docs = vec![
            "rust rust rust programming",
            "rust systems programming",
            "python scripting language",
        ];
        let tfidf = TfIdf::new(&docs);

        let top = tfidf.top_terms(5);
        let top_words: Vec<&str> = top.iter().map(|(w, _)| w.as_str()).collect();

        // "rust" appears frequently but not in all docs, should rank high
        assert!(top_words.contains(&"rust"));
    }

    #[test]
    fn test_all_scores() {
        let docs = vec!["rust programming"];
        let tfidf = TfIdf::new(&docs);

        let scores = tfidf.all_scores();
        assert!(scores.contains_key("rust"));
        assert!(scores.contains_key("programming"));
    }

    #[test]
    fn test_empty_corpus() {
        let docs: Vec<&str> = vec![];
        let tfidf = TfIdf::new(&docs);

        assert_eq!(tfidf.doc_count(), 0);
        assert_eq!(tfidf.term_count(), 0);
        assert!(tfidf.top_terms(5).is_empty());
    }

    #[test]
    fn test_single_document() {
        let docs = vec!["rust memory safety ownership borrowing"];
        let tfidf = TfIdf::new(&docs);

        assert_eq!(tfidf.doc_count(), 1);
        let top = tfidf.top_terms(5);
        assert!(!top.is_empty());
    }

    #[test]
    fn test_repeated_terms() {
        let docs = vec!["rust rust rust rust", "python"];
        let tfidf = TfIdf::new(&docs);

        // "rust" repeated multiple times should have high TF
        let rust_score = tfidf.score("rust");
        let python_score = tfidf.score("python");

        // Both appear in only one doc, but rust has higher TF
        assert!(rust_score > python_score);
    }

    #[test]
    fn test_idf_calculation() {
        let docs = vec!["term1 term2", "term1 term3", "term1 term4"];
        let tfidf = TfIdf::new(&docs);

        // term1 in all 3 docs (TF=3/6=0.5), term2/3/4 in only 1 each (TF=1/6)
        // IDF for term1: ln(4/4)+1 = 1.0
        // IDF for term2: ln(4/2)+1 = 1.69
        let score_common = tfidf.score("term1");
        let score_rare = tfidf.score("term2");

        // term1 has 3x the TF with lower IDF, term2 has 1x TF with higher IDF
        // 0.5 * 1.0 = 0.5 vs 0.167 * 1.69 = 0.28
        assert!(score_common > 0.0);
        assert!(score_rare > 0.0);

        // In TF-IDF with global TF, frequent terms dominate
        assert!(score_common > score_rare);
    }
}
