//! Topic lifecycle management.
//!
//! This module provides tools for managing the lifecycle of topics:
//! - Running extraction cycles to discover new topics
//! - Refreshing importance scores
//! - Pruning stale topics
//! - Merging similar topics
//!
//! ## Usage
//!
//! ```rust,ignore
//! use memory_topics::lifecycle::{TopicLifecycleManager, LifecycleStats};
//! use memory_topics::TopicStorage;
//! use std::sync::Arc;
//!
//! let storage = Arc::new(/* ... */);
//! let topic_storage = TopicStorage::new(storage);
//! let manager = TopicLifecycleManager::new(topic_storage);
//!
//! // Get lifecycle statistics
//! let stats = manager.get_lifecycle_stats()?;
//! println!("Active topics: {}", stats.active_topics);
//!
//! // Prune stale topics (not mentioned in 90 days)
//! let pruned = manager.prune_stale_topics(90)?;
//! println!("Pruned {} topics", pruned);
//! ```

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

use crate::config::ImportanceConfig;
use crate::error::TopicsError;
use crate::importance::ImportanceScorer;
use crate::similarity::cosine_similarity;
use crate::storage::TopicStorage;
use crate::types::{Topic, TopicStatus};

/// Statistics about the topic lifecycle state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifecycleStats {
    /// Number of active topics
    pub active_topics: usize,
    /// Number of archived/pruned topics
    pub archived_topics: usize,
    /// Total number of relationships
    pub total_relationships: usize,
    /// Timestamp of last extraction cycle
    pub last_extraction: Option<DateTime<Utc>>,
    /// Timestamp of last prune operation
    pub last_prune: Option<DateTime<Utc>>,
}

impl LifecycleStats {
    /// Create a new lifecycle stats instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total topic count (active + archived).
    pub fn total_topics(&self) -> usize {
        self.active_topics + self.archived_topics
    }
}

/// Manages the lifecycle of topics.
///
/// The `TopicLifecycleManager` provides operations for maintaining the health
/// and relevance of the topic graph over time.
pub struct TopicLifecycleManager<'a> {
    storage: &'a TopicStorage,
    importance_config: ImportanceConfig,
    /// Timestamp of last extraction cycle (in-memory tracking)
    last_extraction: Option<DateTime<Utc>>,
    /// Timestamp of last prune operation (in-memory tracking)
    last_prune: Option<DateTime<Utc>>,
}

impl<'a> TopicLifecycleManager<'a> {
    /// Create a new lifecycle manager with default importance configuration.
    pub fn new(storage: &'a TopicStorage) -> Self {
        Self {
            storage,
            importance_config: ImportanceConfig::default(),
            last_extraction: None,
            last_prune: None,
        }
    }

    /// Create a lifecycle manager with custom importance configuration.
    pub fn with_importance_config(storage: &'a TopicStorage, config: ImportanceConfig) -> Self {
        Self {
            storage,
            importance_config: config,
            last_extraction: None,
            last_prune: None,
        }
    }

    /// Run a topic extraction cycle.
    ///
    /// This is a placeholder that marks extraction as having run.
    /// The actual extraction logic is handled by the scheduler job.
    ///
    /// # Returns
    ///
    /// Number of new topics extracted (placeholder returns 0).
    #[instrument(skip(self))]
    pub fn run_extraction_cycle(&mut self) -> Result<usize, TopicsError> {
        info!("Running topic extraction cycle");
        self.last_extraction = Some(Utc::now());

        // Note: Actual extraction is performed by the scheduler job using TopicExtractor.
        // This method primarily updates the last_extraction timestamp for tracking.
        // A real implementation would:
        // 1. Query recent TOC nodes since last extraction
        // 2. Get embeddings for those nodes
        // 3. Run HDBSCAN clustering
        // 4. Label and store new topics

        debug!("Extraction cycle complete (placeholder)");
        Ok(0)
    }

    /// Refresh importance scores for all topics.
    ///
    /// Recalculates time-decayed importance scores based on current time.
    /// Topics are updated in storage only if their scores have changed.
    ///
    /// # Returns
    ///
    /// Number of topics whose scores were updated.
    #[instrument(skip(self))]
    pub fn refresh_importance_scores(&mut self) -> Result<u32, TopicsError> {
        let scorer = ImportanceScorer::new(self.importance_config.clone());
        let updated = self.storage.refresh_importance_scores(&scorer)?;
        info!(updated_count = updated, "Refreshed importance scores");
        Ok(updated)
    }

    /// Prune topics that haven't been mentioned in the specified number of days.
    ///
    /// Topics are marked as `Pruned` status rather than deleted, allowing
    /// potential resurrection if they are mentioned again.
    ///
    /// # Arguments
    ///
    /// * `days` - Number of days of inactivity before pruning
    ///
    /// # Returns
    ///
    /// Number of topics pruned.
    #[instrument(skip(self))]
    pub fn prune_stale_topics(&mut self, days: u32) -> Result<usize, TopicsError> {
        let now = Utc::now();
        let threshold = now - Duration::days(i64::from(days));
        self.last_prune = Some(now);

        let topics = self.storage.list_topics()?;
        let mut pruned_count = 0;

        for topic in topics {
            if topic.last_mentioned_at < threshold {
                let mut pruned_topic = topic.clone();
                pruned_topic.status = TopicStatus::Pruned;
                self.storage.save_topic(&pruned_topic)?;
                pruned_count += 1;
                debug!(
                    topic_id = %topic.topic_id,
                    last_mentioned = %topic.last_mentioned_at,
                    "Pruned stale topic"
                );
            }
        }

        info!(
            days = days,
            pruned_count = pruned_count,
            "Pruned stale topics"
        );
        Ok(pruned_count)
    }

    /// Merge topics that are highly similar.
    ///
    /// Topics with embedding similarity above the threshold are merged.
    /// The topic with higher importance is kept, and relationships from
    /// the merged topic are transferred.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Minimum cosine similarity for merging (0.0 - 1.0)
    ///
    /// # Returns
    ///
    /// Number of topic pairs merged.
    #[instrument(skip(self))]
    pub fn merge_similar_topics(&mut self, threshold: f32) -> Result<usize, TopicsError> {
        if !(0.0..=1.0).contains(&threshold) {
            return Err(TopicsError::InvalidInput(format!(
                "Threshold must be between 0.0 and 1.0, got {}",
                threshold
            )));
        }

        let topics = self.storage.list_topics()?;
        let n = topics.len();
        let mut merged_count = 0;
        let mut merged_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Find similar topic pairs
        for i in 0..n {
            if merged_ids.contains(&topics[i].topic_id) {
                continue;
            }

            for j in (i + 1)..n {
                if merged_ids.contains(&topics[j].topic_id) {
                    continue;
                }

                let similarity = cosine_similarity(&topics[i].embedding, &topics[j].embedding);

                if similarity >= threshold {
                    // Determine which topic to keep (higher importance)
                    let (keeper, merged) =
                        if topics[i].importance_score >= topics[j].importance_score {
                            (&topics[i], &topics[j])
                        } else {
                            (&topics[j], &topics[i])
                        };

                    // Mark merged topic as pruned
                    let mut pruned = merged.clone();
                    pruned.status = TopicStatus::Pruned;
                    self.storage.save_topic(&pruned)?;

                    // Update keeper with combined node count and keywords
                    let mut updated_keeper = keeper.clone();
                    updated_keeper.node_count += merged.node_count;

                    // Merge keywords (deduplicate)
                    for keyword in &merged.keywords {
                        if !updated_keeper.keywords.contains(keyword) {
                            updated_keeper.keywords.push(keyword.clone());
                        }
                    }

                    self.storage.save_topic(&updated_keeper)?;
                    merged_ids.insert(merged.topic_id.clone());
                    merged_count += 1;

                    info!(
                        keeper_id = %keeper.topic_id,
                        merged_id = %merged.topic_id,
                        similarity = similarity,
                        "Merged similar topics"
                    );
                }
            }
        }

        info!(
            threshold = threshold,
            merged_count = merged_count,
            "Merged similar topics"
        );
        Ok(merged_count)
    }

    /// Get lifecycle statistics.
    ///
    /// # Returns
    ///
    /// Current lifecycle statistics including topic counts and timestamps.
    #[instrument(skip(self))]
    pub fn get_lifecycle_stats(&self) -> Result<LifecycleStats, TopicsError> {
        let stats = self.storage.get_stats()?;

        // Count active vs pruned topics
        let all_topics = self.list_all_topics()?;
        let active_count = all_topics.iter().filter(|t| t.is_active()).count();
        let archived_count = all_topics.len() - active_count;

        Ok(LifecycleStats {
            active_topics: active_count,
            archived_topics: archived_count,
            total_relationships: stats.relationship_count as usize,
            last_extraction: self.last_extraction,
            last_prune: self.last_prune,
        })
    }

    /// List all topics including pruned ones.
    fn list_all_topics(&self) -> Result<Vec<Topic>, TopicsError> {
        // TopicStorage.list_topics() only returns active topics,
        // so we need to scan the storage directly
        let prefix = b"topic:";
        let mut topics = Vec::new();

        for (_, value) in self
            .storage
            .storage()
            .prefix_iterator(crate::storage::CF_TOPICS, prefix)?
        {
            let topic: Topic = serde_json::from_slice(&value)?;
            topics.push(topic);
        }

        Ok(topics)
    }

    /// Get the timestamp of the last extraction cycle.
    pub fn last_extraction(&self) -> Option<DateTime<Utc>> {
        self.last_extraction
    }

    /// Get the timestamp of the last prune operation.
    pub fn last_prune(&self) -> Option<DateTime<Utc>> {
        self.last_prune
    }

    /// Set the last extraction timestamp (for restoring state).
    pub fn set_last_extraction(&mut self, timestamp: DateTime<Utc>) {
        self.last_extraction = Some(timestamp);
    }

    /// Set the last prune timestamp (for restoring state).
    pub fn set_last_prune(&mut self, timestamp: DateTime<Utc>) {
        self.last_prune = Some(timestamp);
    }

    /// Resurrect a pruned topic (set status back to Active).
    ///
    /// # Arguments
    ///
    /// * `topic_id` - ID of the topic to resurrect
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the topic was resurrected, `Ok(false)` if it was already active,
    /// or an error if the topic was not found.
    #[instrument(skip(self))]
    pub fn resurrect_topic(&self, topic_id: &str) -> Result<bool, TopicsError> {
        let topic = self
            .storage
            .get_topic(topic_id)?
            .ok_or_else(|| TopicsError::NotFound(topic_id.to_string()))?;

        if topic.is_active() {
            debug!(topic_id = %topic_id, "Topic already active");
            return Ok(false);
        }

        let mut resurrected = topic;
        resurrected.status = TopicStatus::Active;
        resurrected.last_mentioned_at = Utc::now();
        self.storage.save_topic(&resurrected)?;

        info!(topic_id = %topic_id, "Resurrected topic");
        Ok(true)
    }

    /// Archive a topic (set status to Pruned).
    ///
    /// # Arguments
    ///
    /// * `topic_id` - ID of the topic to archive
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the topic was archived, `Ok(false)` if it was already archived,
    /// or an error if the topic was not found.
    #[instrument(skip(self))]
    pub fn archive_topic(&self, topic_id: &str) -> Result<bool, TopicsError> {
        let topic = self
            .storage
            .get_topic(topic_id)?
            .ok_or_else(|| TopicsError::NotFound(topic_id.to_string()))?;

        if topic.status == TopicStatus::Pruned {
            debug!(topic_id = %topic_id, "Topic already archived");
            return Ok(false);
        }

        let mut archived = topic;
        archived.status = TopicStatus::Pruned;
        self.storage.save_topic(&archived)?;

        info!(topic_id = %topic_id, "Archived topic");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Topic;
    use memory_storage::Storage;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_storage() -> (TempDir, Arc<Storage>) {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(dir.path()).unwrap();
        (dir, Arc::new(storage))
    }

    fn create_test_topic(id: &str, label: &str, embedding: Vec<f32>) -> Topic {
        Topic::new(id.to_string(), label.to_string(), embedding)
    }

    #[test]
    fn test_lifecycle_stats_default() {
        let stats = LifecycleStats::default();
        assert_eq!(stats.active_topics, 0);
        assert_eq!(stats.archived_topics, 0);
        assert_eq!(stats.total_relationships, 0);
        assert!(stats.last_extraction.is_none());
        assert!(stats.last_prune.is_none());
    }

    #[test]
    fn test_lifecycle_stats_total_topics() {
        let stats = LifecycleStats {
            active_topics: 10,
            archived_topics: 5,
            total_relationships: 20,
            last_extraction: None,
            last_prune: None,
        };
        assert_eq!(stats.total_topics(), 15);
    }

    #[test]
    fn test_manager_new() {
        let (_dir, storage) = create_test_storage();
        let topic_storage = TopicStorage::new(storage);
        let manager = TopicLifecycleManager::new(&topic_storage);

        assert!(manager.last_extraction().is_none());
        assert!(manager.last_prune().is_none());
    }

    #[test]
    fn test_manager_with_importance_config() {
        let (_dir, storage) = create_test_storage();
        let topic_storage = TopicStorage::new(storage);
        let config = ImportanceConfig {
            half_life_days: 60,
            recency_boost: 3.0,
        };
        let manager = TopicLifecycleManager::with_importance_config(&topic_storage, config);

        assert!(manager.last_extraction().is_none());
    }

    #[test]
    fn test_run_extraction_cycle() {
        let (_dir, storage) = create_test_storage();
        let topic_storage = TopicStorage::new(storage);
        let mut manager = TopicLifecycleManager::new(&topic_storage);

        let result = manager.run_extraction_cycle();
        assert!(result.is_ok());
        assert!(manager.last_extraction().is_some());
    }

    #[test]
    fn test_refresh_importance_scores() {
        let (_dir, storage) = create_test_storage();
        let topic_storage = TopicStorage::new(storage);

        // Add a topic
        let topic = create_test_topic("t1", "Test Topic", vec![0.1, 0.2, 0.3]);
        topic_storage.save_topic(&topic).unwrap();

        let mut manager = TopicLifecycleManager::new(&topic_storage);
        let result = manager.refresh_importance_scores();

        assert!(result.is_ok());
    }

    #[test]
    fn test_prune_stale_topics() {
        let (_dir, storage) = create_test_storage();
        let topic_storage = TopicStorage::new(storage);

        // Create an old topic
        let mut old_topic = create_test_topic("t1", "Old Topic", vec![0.1, 0.2, 0.3]);
        old_topic.last_mentioned_at = Utc::now() - Duration::days(100);
        topic_storage.save_topic(&old_topic).unwrap();

        // Create a recent topic
        let recent_topic = create_test_topic("t2", "Recent Topic", vec![0.4, 0.5, 0.6]);
        topic_storage.save_topic(&recent_topic).unwrap();

        let mut manager = TopicLifecycleManager::new(&topic_storage);
        let pruned = manager.prune_stale_topics(90).unwrap();

        assert_eq!(pruned, 1);
        assert!(manager.last_prune().is_some());

        // Verify the old topic is pruned
        let topic = topic_storage.get_topic("t1").unwrap().unwrap();
        assert_eq!(topic.status, TopicStatus::Pruned);

        // Verify the recent topic is still active
        let topic = topic_storage.get_topic("t2").unwrap().unwrap();
        assert_eq!(topic.status, TopicStatus::Active);
    }

    #[test]
    fn test_merge_similar_topics() {
        let (_dir, storage) = create_test_storage();
        let topic_storage = TopicStorage::new(storage);

        // Create similar topics (same embedding = similarity 1.0)
        let mut topic1 = create_test_topic("t1", "Topic One", vec![1.0, 0.0, 0.0]);
        topic1.importance_score = 0.9;
        topic1.node_count = 5;
        topic1.keywords = vec!["rust".to_string()];
        topic_storage.save_topic(&topic1).unwrap();

        let mut topic2 = create_test_topic("t2", "Topic Two", vec![1.0, 0.0, 0.0]);
        topic2.importance_score = 0.5;
        topic2.node_count = 3;
        topic2.keywords = vec!["memory".to_string()];
        topic_storage.save_topic(&topic2).unwrap();

        // Create a different topic
        let topic3 = create_test_topic("t3", "Different", vec![0.0, 0.0, 1.0]);
        topic_storage.save_topic(&topic3).unwrap();

        let mut manager = TopicLifecycleManager::new(&topic_storage);
        let merged = manager.merge_similar_topics(0.95).unwrap();

        assert_eq!(merged, 1);

        // Topic1 should be kept (higher importance)
        let kept = topic_storage.get_topic("t1").unwrap().unwrap();
        assert_eq!(kept.status, TopicStatus::Active);
        assert_eq!(kept.node_count, 8); // 5 + 3
        assert!(kept.keywords.contains(&"rust".to_string()));
        assert!(kept.keywords.contains(&"memory".to_string()));

        // Topic2 should be pruned
        let pruned = topic_storage.get_topic("t2").unwrap().unwrap();
        assert_eq!(pruned.status, TopicStatus::Pruned);

        // Topic3 should be unchanged
        let different = topic_storage.get_topic("t3").unwrap().unwrap();
        assert_eq!(different.status, TopicStatus::Active);
    }

    #[test]
    fn test_merge_similar_topics_invalid_threshold() {
        let (_dir, storage) = create_test_storage();
        let topic_storage = TopicStorage::new(storage);
        let mut manager = TopicLifecycleManager::new(&topic_storage);

        let result = manager.merge_similar_topics(1.5);
        assert!(result.is_err());

        let result = manager.merge_similar_topics(-0.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_lifecycle_stats() {
        let (_dir, storage) = create_test_storage();
        let topic_storage = TopicStorage::new(storage);

        // Add topics
        let active = create_test_topic("t1", "Active", vec![0.1, 0.2]);
        topic_storage.save_topic(&active).unwrap();

        let mut pruned = create_test_topic("t2", "Pruned", vec![0.3, 0.4]);
        pruned.status = TopicStatus::Pruned;
        topic_storage.save_topic(&pruned).unwrap();

        let manager = TopicLifecycleManager::new(&topic_storage);
        let stats = manager.get_lifecycle_stats().unwrap();

        assert_eq!(stats.active_topics, 1);
        assert_eq!(stats.archived_topics, 1);
        assert_eq!(stats.total_topics(), 2);
    }

    #[test]
    fn test_resurrect_topic() {
        let (_dir, storage) = create_test_storage();
        let topic_storage = TopicStorage::new(storage);

        let mut pruned = create_test_topic("t1", "Pruned Topic", vec![0.1, 0.2]);
        pruned.status = TopicStatus::Pruned;
        topic_storage.save_topic(&pruned).unwrap();

        let manager = TopicLifecycleManager::new(&topic_storage);
        let resurrected = manager.resurrect_topic("t1").unwrap();

        assert!(resurrected);

        let topic = topic_storage.get_topic("t1").unwrap().unwrap();
        assert_eq!(topic.status, TopicStatus::Active);
    }

    #[test]
    fn test_resurrect_active_topic() {
        let (_dir, storage) = create_test_storage();
        let topic_storage = TopicStorage::new(storage);

        let active = create_test_topic("t1", "Active Topic", vec![0.1, 0.2]);
        topic_storage.save_topic(&active).unwrap();

        let manager = TopicLifecycleManager::new(&topic_storage);
        let resurrected = manager.resurrect_topic("t1").unwrap();

        assert!(!resurrected); // Already active
    }

    #[test]
    fn test_resurrect_nonexistent_topic() {
        let (_dir, storage) = create_test_storage();
        let topic_storage = TopicStorage::new(storage);
        let manager = TopicLifecycleManager::new(&topic_storage);

        let result = manager.resurrect_topic("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_archive_topic() {
        let (_dir, storage) = create_test_storage();
        let topic_storage = TopicStorage::new(storage);

        let active = create_test_topic("t1", "Active Topic", vec![0.1, 0.2]);
        topic_storage.save_topic(&active).unwrap();

        let manager = TopicLifecycleManager::new(&topic_storage);
        let archived = manager.archive_topic("t1").unwrap();

        assert!(archived);

        let topic = topic_storage.get_topic("t1").unwrap().unwrap();
        assert_eq!(topic.status, TopicStatus::Pruned);
    }

    #[test]
    fn test_archive_already_archived_topic() {
        let (_dir, storage) = create_test_storage();
        let topic_storage = TopicStorage::new(storage);

        let mut pruned = create_test_topic("t1", "Pruned Topic", vec![0.1, 0.2]);
        pruned.status = TopicStatus::Pruned;
        topic_storage.save_topic(&pruned).unwrap();

        let manager = TopicLifecycleManager::new(&topic_storage);
        let archived = manager.archive_topic("t1").unwrap();

        assert!(!archived); // Already archived
    }

    #[test]
    fn test_set_timestamps() {
        let (_dir, storage) = create_test_storage();
        let topic_storage = TopicStorage::new(storage);
        let mut manager = TopicLifecycleManager::new(&topic_storage);

        let extraction_time = Utc::now() - Duration::hours(1);
        let prune_time = Utc::now() - Duration::hours(2);

        manager.set_last_extraction(extraction_time);
        manager.set_last_prune(prune_time);

        assert_eq!(manager.last_extraction(), Some(extraction_time));
        assert_eq!(manager.last_prune(), Some(prune_time));
    }
}
