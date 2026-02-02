//! Time-decayed importance scoring for topics.
//!
//! Uses an exponential decay model with half-life to calculate topic importance.
//! Recent mentions receive a boost, and scores never decay below a minimum threshold.

use chrono::{DateTime, Utc};

use crate::config::ImportanceConfig;
use crate::types::Topic;

/// Minimum importance score to prevent decay to zero.
const DEFAULT_MIN_SCORE: f64 = 0.01;

/// Days threshold for maximum recency boost.
const RECENCY_BOOST_THRESHOLD_DAYS: f64 = 1.0;

/// Days over which recency boost linearly decays.
const RECENCY_BOOST_DECAY_DAYS: f64 = 7.0;

/// Calculates time-decayed importance scores for topics.
///
/// The scoring formula combines:
/// - Base score from node count (logarithmic scaling)
/// - Exponential decay based on time since last mention
/// - Recency boost for very recent mentions (< 7 days)
///
/// # Example
/// ```
/// use memory_topics::importance::ImportanceScorer;
/// use memory_topics::config::ImportanceConfig;
/// use chrono::Utc;
///
/// let config = ImportanceConfig::default();
/// let scorer = ImportanceScorer::new(config);
///
/// let score = scorer.calculate_score(10, Utc::now(), Utc::now());
/// assert!(score > 0.0);
/// ```
pub struct ImportanceScorer {
    config: ImportanceConfig,
    min_score: f64,
}

impl ImportanceScorer {
    /// Create a new importance scorer with the given configuration.
    pub fn new(config: ImportanceConfig) -> Self {
        Self {
            config,
            min_score: DEFAULT_MIN_SCORE,
        }
    }

    /// Create a scorer with a custom minimum score.
    pub fn with_min_score(config: ImportanceConfig, min_score: f64) -> Self {
        Self { config, min_score }
    }

    /// Get the configured half-life in days.
    pub fn half_life_days(&self) -> u32 {
        self.config.half_life_days
    }

    /// Get the configured recency boost factor.
    pub fn recency_boost_factor(&self) -> f64 {
        self.config.recency_boost
    }

    /// Get the minimum score threshold.
    pub fn min_score(&self) -> f64 {
        self.min_score
    }

    /// Calculate importance score for a topic.
    ///
    /// Formula: `score = base_score * decay_factor * recency_boost`
    ///
    /// - `base_score = ln(1 + node_count)` (logarithmic to prevent large clusters from dominating)
    /// - `decay_factor = 2^(-days_since_mention / half_life)` (exponential decay)
    /// - `recency_boost` = boost factor for mentions within 7 days
    ///
    /// # Arguments
    /// * `node_count` - Number of nodes linked to the topic
    /// * `last_mentioned_at` - Timestamp of last topic mention
    /// * `now` - Current timestamp for decay calculation
    ///
    /// # Returns
    /// Importance score (always >= min_score)
    pub fn calculate_score(
        &self,
        node_count: u32,
        last_mentioned_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> f64 {
        let days_since = self.days_between(last_mentioned_at, now);
        let base = self.base_score(node_count);
        let decay = self.decay_factor(days_since);
        let boost = self.recency_boost(days_since);

        (base * decay * boost).max(self.min_score)
    }

    /// Update topic importance based on new mention.
    ///
    /// Updates the topic's `last_mentioned_at` timestamp and recalculates
    /// the importance score. Also increments `node_count` by 1.
    ///
    /// # Arguments
    /// * `topic` - Topic to update (mutated in place)
    /// * `now` - Current timestamp
    pub fn on_topic_mentioned(&self, topic: &mut Topic, now: DateTime<Utc>) {
        topic.last_mentioned_at = now;
        topic.node_count += 1;
        topic.importance_score =
            self.calculate_score(topic.node_count, topic.last_mentioned_at, now);
    }

    /// Touch a topic without incrementing node count.
    ///
    /// Updates the `last_mentioned_at` timestamp and recalculates importance
    /// without changing the node count. Useful for re-references to existing links.
    ///
    /// # Arguments
    /// * `topic` - Topic to update (mutated in place)
    /// * `now` - Current timestamp
    pub fn touch_topic(&self, topic: &mut Topic, now: DateTime<Utc>) {
        topic.last_mentioned_at = now;
        topic.importance_score =
            self.calculate_score(topic.node_count, topic.last_mentioned_at, now);
    }

    /// Batch recalculate all topic scores.
    ///
    /// Useful for periodic refresh jobs to update decayed scores.
    ///
    /// # Arguments
    /// * `topics` - Slice of topics to update (mutated in place)
    /// * `now` - Current timestamp for decay calculation
    ///
    /// # Returns
    /// Number of topics updated
    pub fn recalculate_all(&self, topics: &mut [Topic], now: DateTime<Utc>) -> u32 {
        let mut count = 0;
        for topic in topics.iter_mut() {
            let new_score = self.calculate_score(topic.node_count, topic.last_mentioned_at, now);
            if (new_score - topic.importance_score).abs() > f64::EPSILON {
                topic.importance_score = new_score;
                count += 1;
            }
        }
        count
    }

    /// Calculate exponential decay factor.
    ///
    /// Uses the half-life formula: `decay = 2^(-days / half_life)`
    ///
    /// # Returns
    /// Value between min_score and 1.0
    fn decay_factor(&self, days_since: f64) -> f64 {
        let half_life = f64::from(self.config.half_life_days);
        let factor = 2.0_f64.powf(-days_since / half_life);
        factor.max(self.min_score)
    }

    /// Calculate base score from node count using logarithmic scaling.
    ///
    /// Uses `ln(1 + node_count)` to prevent topics with many nodes
    /// from dominating while still rewarding larger clusters.
    fn base_score(&self, node_count: u32) -> f64 {
        (1.0 + f64::from(node_count)).ln()
    }

    /// Calculate recency boost for very recent mentions.
    ///
    /// - Mentions < 1 day ago: full boost
    /// - Mentions 1-7 days ago: linear decay from boost to 1.0
    /// - Mentions > 7 days ago: no boost (1.0)
    fn recency_boost(&self, days_since: f64) -> f64 {
        if days_since < RECENCY_BOOST_THRESHOLD_DAYS {
            // Full boost for very recent mentions
            self.config.recency_boost
        } else if days_since < RECENCY_BOOST_DECAY_DAYS {
            // Linear decay from boost to 1.0 over the decay period
            let remaining_boost = self.config.recency_boost - 1.0;
            let decay_progress = (days_since - RECENCY_BOOST_THRESHOLD_DAYS)
                / (RECENCY_BOOST_DECAY_DAYS - RECENCY_BOOST_THRESHOLD_DAYS);
            1.0 + remaining_boost * (1.0 - decay_progress)
        } else {
            // No boost for older mentions
            1.0
        }
    }

    /// Calculate days between two timestamps.
    fn days_between(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> f64 {
        let duration = to.signed_duration_since(from);
        duration.num_seconds() as f64 / 86400.0
    }
}

impl Default for ImportanceScorer {
    fn default() -> Self {
        Self::new(ImportanceConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_topic() -> Topic {
        Topic::new(
            "test-topic-id".to_string(),
            "Test Topic".to_string(),
            vec![0.1, 0.2, 0.3],
        )
    }

    #[test]
    fn test_scorer_defaults() {
        let scorer = ImportanceScorer::default();
        assert_eq!(scorer.half_life_days(), 30);
        assert!((scorer.recency_boost_factor() - 2.0).abs() < f64::EPSILON);
        assert!((scorer.min_score() - DEFAULT_MIN_SCORE).abs() < f64::EPSILON);
    }

    #[test]
    fn test_scorer_with_custom_min_score() {
        let config = ImportanceConfig::default();
        let scorer = ImportanceScorer::with_min_score(config, 0.05);
        assert!((scorer.min_score() - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn test_base_score_logarithmic() {
        let scorer = ImportanceScorer::default();

        // Base score uses ln(1 + node_count)
        let score_0 = scorer.base_score(0);
        let score_1 = scorer.base_score(1);
        let score_10 = scorer.base_score(10);
        let score_100 = scorer.base_score(100);

        // Verify logarithmic scaling
        assert!((score_0 - 1.0_f64.ln()).abs() < f64::EPSILON); // ln(1) = 0
        assert!((score_1 - 2.0_f64.ln()).abs() < f64::EPSILON); // ln(2)
        assert!((score_10 - 11.0_f64.ln()).abs() < f64::EPSILON); // ln(11)
        assert!((score_100 - 101.0_f64.ln()).abs() < f64::EPSILON); // ln(101)

        // Verify diminishing returns
        assert!(score_10 < score_100);
        assert!((score_100 - score_10) < (score_10 - score_0));
    }

    #[test]
    fn test_decay_factor_at_half_life() {
        let scorer = ImportanceScorer::default();

        // At exactly one half-life, decay should be 0.5
        let decay = scorer.decay_factor(30.0);
        assert!((decay - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_decay_factor_at_zero() {
        let scorer = ImportanceScorer::default();

        // At time zero, no decay
        let decay = scorer.decay_factor(0.0);
        assert!((decay - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_decay_factor_at_two_half_lives() {
        let scorer = ImportanceScorer::default();

        // At two half-lives, decay should be 0.25
        let decay = scorer.decay_factor(60.0);
        assert!((decay - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_decay_factor_never_below_min() {
        let scorer = ImportanceScorer::default();

        // Even after many half-lives, score stays above minimum
        let decay = scorer.decay_factor(365.0); // ~12 half-lives
        assert!(decay >= scorer.min_score());
    }

    #[test]
    fn test_recency_boost_very_recent() {
        let scorer = ImportanceScorer::default();

        // Less than 1 day: full boost
        let boost = scorer.recency_boost(0.5);
        assert!((boost - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_recency_boost_within_week() {
        let scorer = ImportanceScorer::default();

        // At 4 days (midpoint of 1-7 day range): partial boost
        let boost = scorer.recency_boost(4.0);
        // Should be halfway between 2.0 and 1.0
        assert!(boost > 1.0);
        assert!(boost < 2.0);
        assert!((boost - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_recency_boost_after_week() {
        let scorer = ImportanceScorer::default();

        // After 7 days: no boost
        let boost = scorer.recency_boost(10.0);
        assert!((boost - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_score_combines_factors() {
        let scorer = ImportanceScorer::default();
        let now = Utc::now();
        let mentioned_now = now;
        let mentioned_30_days_ago = now - Duration::days(30);

        // Score for recent mention with 10 nodes
        let recent_score = scorer.calculate_score(10, mentioned_now, now);

        // Score for old mention with same nodes
        let old_score = scorer.calculate_score(10, mentioned_30_days_ago, now);

        // Recent should be higher due to recency boost and no decay
        assert!(recent_score > old_score);

        // Old score should be approximately half (one half-life) * no recency boost
        // recent = ln(11) * 1.0 * 2.0 = ~4.79
        // old = ln(11) * 0.5 * 1.0 = ~1.20
        assert!(recent_score > old_score * 2.0);
    }

    #[test]
    fn test_calculate_score_minimum() {
        let scorer = ImportanceScorer::default();
        let now = Utc::now();
        let ancient = now - Duration::days(365 * 10); // 10 years ago

        // Even very old topics have minimum score
        let score = scorer.calculate_score(0, ancient, now);
        assert!(score >= scorer.min_score());
    }

    #[test]
    fn test_on_topic_mentioned() {
        let scorer = ImportanceScorer::default();
        let mut topic = create_test_topic();
        let initial_node_count = topic.node_count;
        let initial_score = topic.importance_score;

        let now = Utc::now();
        scorer.on_topic_mentioned(&mut topic, now);

        // Node count should increase
        assert_eq!(topic.node_count, initial_node_count + 1);
        // Timestamp should update
        assert_eq!(topic.last_mentioned_at, now);
        // Score should be recalculated (and likely increase due to recency boost)
        assert!(topic.importance_score > 0.0);
        assert_ne!(topic.importance_score, initial_score);
    }

    #[test]
    fn test_touch_topic_no_node_increment() {
        let scorer = ImportanceScorer::default();
        let mut topic = create_test_topic();
        topic.node_count = 5;
        let initial_node_count = topic.node_count;

        let now = Utc::now();
        scorer.touch_topic(&mut topic, now);

        // Node count should NOT increase
        assert_eq!(topic.node_count, initial_node_count);
        // Timestamp should update
        assert_eq!(topic.last_mentioned_at, now);
    }

    #[test]
    fn test_recalculate_all() {
        let scorer = ImportanceScorer::default();
        let base_time = Utc::now() - Duration::days(15);

        let mut topics = vec![
            Topic::new("t1".to_string(), "Topic 1".to_string(), vec![0.1]),
            Topic::new("t2".to_string(), "Topic 2".to_string(), vec![0.2]),
        ];

        // Set different ages
        topics[0].node_count = 5;
        topics[0].last_mentioned_at = base_time;
        topics[0].importance_score = 0.0;

        topics[1].node_count = 10;
        topics[1].last_mentioned_at = base_time - Duration::days(30);
        topics[1].importance_score = 0.0;

        let now = Utc::now();
        let updated = scorer.recalculate_all(&mut topics, now);

        assert_eq!(updated, 2);
        assert!(topics[0].importance_score > 0.0);
        assert!(topics[1].importance_score > 0.0);
        // Topic 0 (more recent) should have higher score despite fewer nodes
        assert!(topics[0].importance_score > topics[1].importance_score);
    }

    #[test]
    fn test_recalculate_all_skips_unchanged() {
        let scorer = ImportanceScorer::default();
        let now = Utc::now();

        let mut topics = vec![Topic::new(
            "t1".to_string(),
            "Topic 1".to_string(),
            vec![0.1],
        )];

        // Pre-calculate correct score
        topics[0].node_count = 5;
        topics[0].last_mentioned_at = now;
        topics[0].importance_score =
            scorer.calculate_score(topics[0].node_count, topics[0].last_mentioned_at, now);

        // Should skip since score is already correct
        let updated = scorer.recalculate_all(&mut topics, now);
        assert_eq!(updated, 0);
    }

    #[test]
    fn test_days_between() {
        let scorer = ImportanceScorer::default();
        let now = Utc::now();
        let yesterday = now - Duration::days(1);
        let week_ago = now - Duration::days(7);

        assert!((scorer.days_between(yesterday, now) - 1.0).abs() < 0.001);
        assert!((scorer.days_between(week_ago, now) - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_days_between_fractional() {
        let scorer = ImportanceScorer::default();
        let now = Utc::now();
        let twelve_hours_ago = now - Duration::hours(12);

        let days = scorer.days_between(twelve_hours_ago, now);
        assert!((days - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_importance_ordering() {
        let scorer = ImportanceScorer::default();
        let now = Utc::now();

        // Create scenarios with different combinations
        let scenarios = [
            (10, now, "recent-many"),                   // Recent, many nodes
            (10, now - Duration::days(30), "old-many"), // Old, many nodes
            (1, now, "recent-few"),                     // Recent, few nodes
            (1, now - Duration::days(30), "old-few"),   // Old, few nodes
        ];

        let scores: Vec<(f64, &str)> = scenarios
            .iter()
            .map(|(nodes, time, label)| (scorer.calculate_score(*nodes, *time, now), *label))
            .collect();

        // Verify expected ordering: recent-many > old-many >= recent-few > old-few
        assert!(
            scores.iter().find(|(_, l)| *l == "recent-many").unwrap().0
                > scores.iter().find(|(_, l)| *l == "old-many").unwrap().0,
            "Recent with many nodes should beat old with many nodes"
        );

        assert!(
            scores.iter().find(|(_, l)| *l == "recent-few").unwrap().0
                > scores.iter().find(|(_, l)| *l == "old-few").unwrap().0,
            "Recent with few nodes should beat old with few nodes"
        );
    }
}
