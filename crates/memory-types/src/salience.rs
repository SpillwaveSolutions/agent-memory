//! Salience scoring for memory importance calculation.
//!
//! Per Phase 16 Plan 01: Score memories by importance at write time.
//! Salience is computed ONCE at node creation (not on read), respecting
//! the append-only model.
//!
//! ## Components
//!
//! - `MemoryKind`: Classification of memory type (observation, preference, etc.)
//! - `SalienceScorer`: Calculates salience score based on text, kind, and pinned status
//! - `SalienceConfig`: Configuration for scoring weights
//!
//! ## Scoring Formula
//!
//! ```text
//! salience = length_density + kind_boost + pinned_boost
//!
//! where:
//!   length_density = (text.len() / 500.0).min(1.0) * length_density_weight
//!   kind_boost = kind_boost_weight if kind != Observation, else 0.0
//!   pinned_boost = pinned_boost_weight if is_pinned, else 0.0
//! ```

use serde::{Deserialize, Serialize};

/// Classification of memory type for salience scoring.
///
/// Different memory types receive different boosts:
/// - `Observation`: Default type, no boost
/// - `Preference`: User preferences ("prefer", "like", "avoid")
/// - `Procedure`: Steps or instructions ("step", "first", "then")
/// - `Constraint`: Requirements or limitations ("must", "should", "need to")
/// - `Definition`: Definitions or meanings ("is defined as", "means")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    /// Default observation with no boost
    #[default]
    Observation,
    /// User preference (matches: "prefer", "like", "avoid", "hate", "dislike")
    Preference,
    /// Procedural step (matches: "step", "first", "then", "finally", "next")
    Procedure,
    /// Constraint or requirement (matches: "must", "should", "need to", "require", "cannot")
    Constraint,
    /// Definition or meaning (matches: "is defined as", "means", "refers to", "definition")
    Definition,
}

impl std::fmt::Display for MemoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryKind::Observation => write!(f, "observation"),
            MemoryKind::Preference => write!(f, "preference"),
            MemoryKind::Procedure => write!(f, "procedure"),
            MemoryKind::Constraint => write!(f, "constraint"),
            MemoryKind::Definition => write!(f, "definition"),
        }
    }
}

/// Configuration for salience scoring weights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalienceConfig {
    /// Whether salience scoring is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Weight for text length density (0.0-1.0)
    #[serde(default = "default_length_density_weight")]
    pub length_density_weight: f32,

    /// Boost for non-observation memory kinds (0.0-1.0)
    #[serde(default = "default_kind_boost")]
    pub kind_boost: f32,

    /// Boost for pinned memories (0.0-1.0)
    #[serde(default = "default_pinned_boost")]
    pub pinned_boost: f32,
}

fn default_true() -> bool {
    true
}

fn default_length_density_weight() -> f32 {
    0.45
}

fn default_kind_boost() -> f32 {
    0.20
}

fn default_pinned_boost() -> f32 {
    0.20
}

impl Default for SalienceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            length_density_weight: default_length_density_weight(),
            kind_boost: default_kind_boost(),
            pinned_boost: default_pinned_boost(),
        }
    }
}

/// Salience scorer for calculating memory importance at write time.
#[derive(Debug, Clone)]
pub struct SalienceScorer {
    config: SalienceConfig,
}

impl SalienceScorer {
    /// Create a new salience scorer with the given configuration.
    pub fn new(config: SalienceConfig) -> Self {
        Self { config }
    }

    /// Create a scorer with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(SalienceConfig::default())
    }

    /// Calculate salience score for a memory.
    ///
    /// Score is based on:
    /// - Text length density (longer = more salient, up to 500 chars)
    /// - Memory kind boost (non-observation types get a boost)
    /// - Pinned status boost
    ///
    /// Returns a score typically in the range 0.0-1.0, though it can exceed 1.0
    /// for pinned, high-value memories.
    pub fn calculate(&self, text: &str, kind: MemoryKind, is_pinned: bool) -> f32 {
        if !self.config.enabled {
            return default_salience();
        }

        // Length density: (len / 500).min(1.0) * weight
        let length_density =
            (text.len() as f32 / 500.0).min(1.0) * self.config.length_density_weight;

        // Kind boost: applies to non-observation types
        let kind_boost = if kind != MemoryKind::Observation {
            self.config.kind_boost
        } else {
            0.0
        };

        // Pinned boost
        let pinned_boost = if is_pinned {
            self.config.pinned_boost
        } else {
            0.0
        };

        // Base score of 0.35 ensures minimum salience + calculated components
        0.35 + length_density + kind_boost + pinned_boost
    }

    /// Classify the memory kind based on text content.
    ///
    /// Uses keyword pattern matching to detect:
    /// - Preferences: "prefer", "like", "avoid", "hate", "dislike"
    /// - Procedures: "step", "first", "then", "finally", "next"
    /// - Constraints: "must", "should", "need to", "require", "cannot"
    /// - Definitions: "is defined as", "means", "refers to", "definition"
    pub fn classify_kind(&self, text: &str) -> MemoryKind {
        let lower = text.to_lowercase();

        // Check for definition patterns first (more specific)
        if lower.contains("is defined as")
            || lower.contains("means that")
            || lower.contains("refers to")
            || lower.contains("definition of")
            || lower.contains("defined as")
        {
            return MemoryKind::Definition;
        }

        // Check for constraint patterns
        if lower.contains("must ")
            || lower.contains("should ")
            || lower.contains("need to")
            || lower.contains("require")
            || lower.contains("cannot ")
            || lower.contains("can't ")
            || lower.contains("must not")
            || lower.contains("should not")
        {
            return MemoryKind::Constraint;
        }

        // Check for preference patterns
        if lower.contains("i prefer")
            || lower.contains("i like")
            || lower.contains("i avoid")
            || lower.contains("i hate")
            || lower.contains("i dislike")
            || lower.contains("prefer to")
            || lower.contains("rather than")
        {
            return MemoryKind::Preference;
        }

        // Check for procedure patterns
        if lower.contains("step ")
            || lower.contains("first,")
            || lower.contains("then,")
            || lower.contains("finally,")
            || lower.contains("next,")
            || lower.contains("step 1")
            || lower.contains("step 2")
            || lower.contains("to do this")
        {
            return MemoryKind::Procedure;
        }

        MemoryKind::Observation
    }

    /// Calculate salience with automatic kind classification.
    pub fn calculate_auto(&self, text: &str, is_pinned: bool) -> (f32, MemoryKind) {
        let kind = self.classify_kind(text);
        let score = self.calculate(text, kind, is_pinned);
        (score, kind)
    }

    /// Get the configuration.
    pub fn config(&self) -> &SalienceConfig {
        &self.config
    }
}

impl Default for SalienceScorer {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Default salience score for existing data without salience fields.
///
/// Returns 0.5 as a neutral midpoint.
pub fn default_salience() -> f32 {
    0.5
}

/// Calculate salience using default configuration.
///
/// Convenience function for simple cases.
pub fn calculate_salience(text: &str, kind: MemoryKind, is_pinned: bool) -> f32 {
    SalienceScorer::with_defaults().calculate(text, kind, is_pinned)
}

/// Classify memory kind from text using default patterns.
pub fn classify_memory_kind(text: &str) -> MemoryKind {
    SalienceScorer::with_defaults().classify_kind(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_kind_default() {
        assert_eq!(MemoryKind::default(), MemoryKind::Observation);
    }

    #[test]
    fn test_memory_kind_display() {
        assert_eq!(MemoryKind::Observation.to_string(), "observation");
        assert_eq!(MemoryKind::Preference.to_string(), "preference");
        assert_eq!(MemoryKind::Procedure.to_string(), "procedure");
        assert_eq!(MemoryKind::Constraint.to_string(), "constraint");
        assert_eq!(MemoryKind::Definition.to_string(), "definition");
    }

    #[test]
    fn test_memory_kind_serialization() {
        let kind = MemoryKind::Preference;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"preference\"");

        let decoded: MemoryKind = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, kind);
    }

    #[test]
    fn test_salience_config_default() {
        let config = SalienceConfig::default();
        assert!(config.enabled);
        assert!((config.length_density_weight - 0.45).abs() < f32::EPSILON);
        assert!((config.kind_boost - 0.20).abs() < f32::EPSILON);
        assert!((config.pinned_boost - 0.20).abs() < f32::EPSILON);
    }

    #[test]
    fn test_default_salience() {
        assert!((default_salience() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_calculate_salience_observation() {
        let scorer = SalienceScorer::with_defaults();

        // Short observation
        let score = scorer.calculate("Hello", MemoryKind::Observation, false);
        assert!(score > 0.35);
        assert!(score < 0.5);

        // Long observation (500+ chars gets max length density)
        let long_text = "x".repeat(600);
        let score = scorer.calculate(&long_text, MemoryKind::Observation, false);
        assert!((score - 0.80).abs() < 0.01); // 0.35 + 0.45 = 0.80
    }

    #[test]
    fn test_calculate_salience_kind_boost() {
        let scorer = SalienceScorer::with_defaults();
        let text = "test";

        let obs_score = scorer.calculate(text, MemoryKind::Observation, false);
        let pref_score = scorer.calculate(text, MemoryKind::Preference, false);
        let proc_score = scorer.calculate(text, MemoryKind::Procedure, false);
        let const_score = scorer.calculate(text, MemoryKind::Constraint, false);
        let def_score = scorer.calculate(text, MemoryKind::Definition, false);

        // Non-observation kinds should have higher scores
        assert!(pref_score > obs_score);
        assert!(proc_score > obs_score);
        assert!(const_score > obs_score);
        assert!(def_score > obs_score);

        // All non-observation kinds should have same boost
        assert!((pref_score - proc_score).abs() < f32::EPSILON);
        assert!((proc_score - const_score).abs() < f32::EPSILON);
        assert!((const_score - def_score).abs() < f32::EPSILON);
    }

    #[test]
    fn test_calculate_salience_pinned_boost() {
        let scorer = SalienceScorer::with_defaults();
        let text = "test";

        let unpinned = scorer.calculate(text, MemoryKind::Observation, false);
        let pinned = scorer.calculate(text, MemoryKind::Observation, true);

        assert!(pinned > unpinned);
        assert!((pinned - unpinned - 0.20).abs() < f32::EPSILON);
    }

    #[test]
    fn test_calculate_salience_combined() {
        let scorer = SalienceScorer::with_defaults();

        // Long text + non-observation + pinned = maximum salience
        let long_text = "x".repeat(600);
        let score = scorer.calculate(&long_text, MemoryKind::Preference, true);

        // 0.35 (base) + 0.45 (length) + 0.20 (kind) + 0.20 (pinned) = 1.20
        assert!((score - 1.20).abs() < 0.01);
    }

    #[test]
    fn test_calculate_salience_disabled() {
        let config = SalienceConfig {
            enabled: false,
            ..Default::default()
        };
        let scorer = SalienceScorer::new(config);

        // When disabled, should return default
        let score = scorer.calculate("long text here", MemoryKind::Preference, true);
        assert!((score - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_classify_kind_preference() {
        let scorer = SalienceScorer::with_defaults();

        assert_eq!(
            scorer.classify_kind("I prefer to use Rust for systems programming"),
            MemoryKind::Preference
        );
        assert_eq!(
            scorer.classify_kind("I like async/await patterns"),
            MemoryKind::Preference
        );
        assert_eq!(
            scorer.classify_kind("I avoid using global state"),
            MemoryKind::Preference
        );
        assert_eq!(
            scorer.classify_kind("I dislike mutable references"),
            MemoryKind::Preference
        );
    }

    #[test]
    fn test_classify_kind_procedure() {
        let scorer = SalienceScorer::with_defaults();

        assert_eq!(
            scorer.classify_kind("Step 1: Install dependencies"),
            MemoryKind::Procedure
        );
        assert_eq!(
            scorer.classify_kind("First, clone the repository"),
            MemoryKind::Procedure
        );
        assert_eq!(
            scorer.classify_kind("Then, run the build command"),
            MemoryKind::Procedure
        );
        assert_eq!(
            scorer.classify_kind("Finally, deploy to production"),
            MemoryKind::Procedure
        );
    }

    #[test]
    fn test_classify_kind_constraint() {
        let scorer = SalienceScorer::with_defaults();

        assert_eq!(
            scorer.classify_kind("You must use UTF-8 encoding"),
            MemoryKind::Constraint
        );
        assert_eq!(
            scorer.classify_kind("You should handle errors gracefully"),
            MemoryKind::Constraint
        );
        assert_eq!(
            scorer.classify_kind("We need to support backwards compatibility"),
            MemoryKind::Constraint
        );
        assert_eq!(
            scorer.classify_kind("The system requires authentication"),
            MemoryKind::Constraint
        );
        assert_eq!(
            scorer.classify_kind("You cannot modify immutable data"),
            MemoryKind::Constraint
        );
    }

    #[test]
    fn test_classify_kind_definition() {
        let scorer = SalienceScorer::with_defaults();

        assert_eq!(
            scorer.classify_kind("A monad is defined as a type that wraps values"),
            MemoryKind::Definition
        );
        assert_eq!(
            scorer.classify_kind("This means that the operation is atomic"),
            MemoryKind::Definition
        );
        assert_eq!(
            scorer.classify_kind("'ACID' refers to atomicity, consistency, isolation, durability"),
            MemoryKind::Definition
        );
        assert_eq!(
            scorer.classify_kind("The definition of ownership in Rust"),
            MemoryKind::Definition
        );
    }

    #[test]
    fn test_classify_kind_observation_default() {
        let scorer = SalienceScorer::with_defaults();

        assert_eq!(
            scorer.classify_kind("The weather is nice today"),
            MemoryKind::Observation
        );
        assert_eq!(
            scorer.classify_kind("I went to the store"),
            MemoryKind::Observation
        );
        assert_eq!(
            scorer.classify_kind("The code compiles successfully"),
            MemoryKind::Observation
        );
    }

    #[test]
    fn test_calculate_auto() {
        let scorer = SalienceScorer::with_defaults();

        let (score, kind) = scorer.calculate_auto("I prefer Rust over C++", false);
        assert_eq!(kind, MemoryKind::Preference);
        assert!(score > 0.5); // Has kind boost

        let (score, kind) = scorer.calculate_auto("Regular observation", false);
        assert_eq!(kind, MemoryKind::Observation);
        assert!(score < 0.5); // No boosts, short text
    }

    #[test]
    fn test_convenience_functions() {
        let score = calculate_salience("test", MemoryKind::Preference, false);
        let scorer = SalienceScorer::with_defaults();
        let expected = scorer.calculate("test", MemoryKind::Preference, false);
        assert!((score - expected).abs() < f32::EPSILON);

        let kind = classify_memory_kind("I prefer this approach");
        assert_eq!(kind, MemoryKind::Preference);
    }
}
