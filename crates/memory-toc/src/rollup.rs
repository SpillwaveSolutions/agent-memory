//! Rollup jobs for aggregating child TOC nodes.
//!
//! Per TOC-05: Day/Week/Month rollup jobs with checkpointing.
//! Per SUMM-04: Rollup summarizer aggregates child node summaries.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

use memory_storage::Storage;
use memory_types::{TocBullet, TocLevel, TocNode};

use crate::summarizer::{Summarizer, SummarizerError, Summary};

/// Checkpoint for rollup job crash recovery.
///
/// Per STOR-03 and TOC-05: Enables crash recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollupCheckpoint {
    /// Job identifier
    pub job_name: String,

    /// Level being processed
    pub level: TocLevel,

    /// Last successfully processed time period
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub last_processed_time: DateTime<Utc>,

    /// Number of nodes processed in current run
    pub processed_count: usize,

    /// When this checkpoint was created
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
}

impl RollupCheckpoint {
    pub fn new(job_name: String, level: TocLevel) -> Self {
        Self {
            job_name,
            level,
            last_processed_time: DateTime::<Utc>::MIN_UTC,
            processed_count: 0,
            created_at: Utc::now(),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// Error type for rollup operations.
#[derive(Debug, thiserror::Error)]
pub enum RollupError {
    #[error("Storage error: {0}")]
    Storage(#[from] memory_storage::StorageError),

    #[error("Summarization error: {0}")]
    Summarizer(#[from] SummarizerError),

    #[error("No child level for {0}")]
    NoChildLevel(TocLevel),

    #[error("Checkpoint error: {0}")]
    Checkpoint(String),
}

/// Rollup job for aggregating child nodes into parent summaries.
pub struct RollupJob {
    storage: Arc<Storage>,
    summarizer: Arc<dyn Summarizer>,
    level: TocLevel,
    /// Minimum age of period before rollup (avoids rolling up incomplete periods)
    min_age: Duration,
}

impl RollupJob {
    /// Create a new rollup job for the specified level.
    ///
    /// min_age: Minimum age of a period before it can be rolled up.
    /// This prevents rolling up periods that are still receiving events.
    pub fn new(
        storage: Arc<Storage>,
        summarizer: Arc<dyn Summarizer>,
        level: TocLevel,
        min_age: Duration,
    ) -> Self {
        Self {
            storage,
            summarizer,
            level,
            min_age,
        }
    }

    /// Create rollup jobs for all levels.
    pub fn create_all(storage: Arc<Storage>, summarizer: Arc<dyn Summarizer>) -> Vec<Self> {
        vec![
            Self::new(
                storage.clone(),
                summarizer.clone(),
                TocLevel::Day,
                Duration::hours(1),
            ),
            Self::new(
                storage.clone(),
                summarizer.clone(),
                TocLevel::Week,
                Duration::hours(24),
            ),
            Self::new(
                storage.clone(),
                summarizer.clone(),
                TocLevel::Month,
                Duration::hours(24),
            ),
            Self::new(
                storage.clone(),
                summarizer.clone(),
                TocLevel::Year,
                Duration::days(7),
            ),
        ]
    }

    /// Run the rollup job.
    ///
    /// Processes nodes that need rollup since the last checkpoint.
    pub async fn run(&self) -> Result<usize, RollupError> {
        let job_name = format!("rollup_{}", self.level);
        info!(job = %job_name, level = %self.level, "Starting rollup job");

        // Load checkpoint
        let checkpoint = self.load_checkpoint(&job_name)?;
        let start_time = checkpoint
            .map(|c| c.last_processed_time)
            .unwrap_or(DateTime::<Utc>::MIN_UTC);

        // Get nodes at this level that need rollup
        let cutoff_time = Utc::now() - self.min_age;
        let nodes =
            self.storage
                .get_toc_nodes_by_level(self.level, Some(start_time), Some(cutoff_time))?;

        let mut processed = 0;

        for node in nodes {
            // Skip if period is too recent
            if node.end_time > cutoff_time {
                debug!(
                    node_id = %node.node_id,
                    "Skipping node - period not yet closed"
                );
                continue;
            }

            // Get children
            let children = self.storage.get_child_nodes(&node.node_id)?;
            if children.is_empty() {
                debug!(node_id = %node.node_id, "Skipping node - no children");
                continue;
            }

            // Convert children to summaries
            let summaries: Vec<Summary> = children
                .iter()
                .map(|c| {
                    Summary::new(
                        c.title.clone(),
                        c.bullets.iter().map(|b| b.text.clone()).collect(),
                        c.keywords.clone(),
                    )
                })
                .collect();

            // Generate rollup summary
            let rollup_summary = self.summarizer.summarize_children(&summaries).await?;

            // Update node with rollup summary
            let mut updated_node = node.clone();
            updated_node.title = rollup_summary.title;
            updated_node.bullets = rollup_summary
                .bullets
                .into_iter()
                .map(TocBullet::new)
                .collect();
            updated_node.keywords = rollup_summary.keywords;

            // Ensure child IDs are up to date
            updated_node.child_node_ids = children.iter().map(|c| c.node_id.clone()).collect();

            self.storage.put_toc_node(&updated_node)?;

            // Save checkpoint after each node
            self.save_checkpoint(&job_name, &updated_node)?;

            processed += 1;
            debug!(
                node_id = %updated_node.node_id,
                children = children.len(),
                "Rolled up node"
            );
        }

        info!(
            job = %job_name,
            processed = processed,
            "Rollup job complete"
        );

        Ok(processed)
    }

    /// Load checkpoint from storage.
    fn load_checkpoint(&self, job_name: &str) -> Result<Option<RollupCheckpoint>, RollupError> {
        match self.storage.get_checkpoint(job_name)? {
            Some(bytes) => {
                let checkpoint = RollupCheckpoint::from_bytes(&bytes)
                    .map_err(|e| RollupError::Checkpoint(e.to_string()))?;
                Ok(Some(checkpoint))
            }
            None => Ok(None),
        }
    }

    /// Save checkpoint to storage.
    fn save_checkpoint(&self, job_name: &str, node: &TocNode) -> Result<(), RollupError> {
        let checkpoint = RollupCheckpoint {
            job_name: job_name.to_string(),
            level: self.level,
            last_processed_time: node.end_time,
            processed_count: 1,
            created_at: Utc::now(),
        };

        let bytes = checkpoint
            .to_bytes()
            .map_err(|e| RollupError::Checkpoint(e.to_string()))?;

        self.storage.put_checkpoint(job_name, &bytes)?;
        Ok(())
    }
}

/// Run all rollup jobs in sequence.
pub async fn run_all_rollups(
    storage: Arc<Storage>,
    summarizer: Arc<dyn Summarizer>,
) -> Result<usize, RollupError> {
    let jobs = RollupJob::create_all(storage, summarizer);
    let mut total = 0;

    for job in jobs {
        total += job.run().await?;
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::TocBuilder;
    use crate::summarizer::MockSummarizer;
    use chrono::TimeZone;
    use memory_types::{Event, EventRole, EventType, Segment};
    use tempfile::TempDir;

    fn create_test_storage() -> (Arc<Storage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::open(temp_dir.path()).unwrap());
        (storage, temp_dir)
    }

    #[allow(dead_code)]
    fn create_test_event(text: &str, timestamp_ms: i64) -> Event {
        let ulid = ulid::Ulid::from_parts(timestamp_ms as u64, rand::random());
        Event::new(
            ulid.to_string(),
            "session-123".to_string(),
            Utc.timestamp_millis_opt(timestamp_ms).unwrap(),
            EventType::UserMessage,
            EventRole::User,
            text.to_string(),
        )
    }

    #[test]
    fn test_checkpoint_serialization() {
        let checkpoint = RollupCheckpoint::new("test_job".to_string(), TocLevel::Day);
        let bytes = checkpoint.to_bytes().unwrap();
        let decoded = RollupCheckpoint::from_bytes(&bytes).unwrap();

        assert_eq!(checkpoint.job_name, decoded.job_name);
        assert_eq!(checkpoint.level, decoded.level);
    }

    #[tokio::test]
    async fn test_rollup_job_no_children() {
        let (storage, _temp) = create_test_storage();
        let summarizer = Arc::new(MockSummarizer::new());

        let job = RollupJob::new(
            storage,
            summarizer,
            TocLevel::Day,
            Duration::zero(), // No min age for testing
        );

        let result = job.run().await.unwrap();
        assert_eq!(result, 0); // No nodes to process
    }

    #[tokio::test]
    async fn test_rollup_job_with_segments() {
        let (storage, _temp) = create_test_storage();
        let summarizer = Arc::new(MockSummarizer::new());

        // First, create some segments using TocBuilder
        let builder = TocBuilder::new(storage.clone(), summarizer.clone());

        // Create segment in the past
        let past_time = Utc::now() - Duration::days(2);
        let events = vec![Event::new(
            ulid::Ulid::new().to_string(),
            "session".to_string(),
            past_time,
            EventType::UserMessage,
            EventRole::User,
            "Test event".to_string(),
        )];
        let segment = Segment::new(
            "seg:test".to_string(),
            events.clone(),
            past_time,
            past_time,
            50,
        );

        builder.process_segment(&segment).await.unwrap();

        // Run rollup job
        let job = RollupJob::new(
            storage.clone(),
            summarizer,
            TocLevel::Day,
            Duration::hours(1),
        );

        let result = job.run().await.unwrap();
        // Result depends on whether the day node has children
        // This tests the basic flow works without errors
        // result is a count of nodes processed
        let _ = result;
    }
}
