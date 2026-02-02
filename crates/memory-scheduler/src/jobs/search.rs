//! Search index scheduled jobs.
//!
//! Periodic commit job to make indexed documents searchable.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use memory_search::SearchIndexer;

use crate::{JitterConfig, OverlapPolicy, SchedulerError, SchedulerService, TimeoutConfig};

/// Configuration for index commit job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexCommitJobConfig {
    /// Cron expression (default: "0 * * * * *" = every minute)
    pub cron: String,
    /// Timezone (default: "UTC")
    pub timezone: String,
    /// Max jitter in seconds (default: 10)
    pub jitter_secs: u64,
    /// Timeout in seconds (default: 60 = 1 minute)
    pub timeout_secs: u64,
}

impl Default for IndexCommitJobConfig {
    fn default() -> Self {
        Self {
            cron: "0 * * * * *".to_string(), // Every minute
            timezone: "UTC".to_string(),
            jitter_secs: 10,
            timeout_secs: 60, // 1 minute
        }
    }
}

/// Register the index commit job with the scheduler.
///
/// This job periodically commits the search index to make
/// newly indexed documents visible to search queries.
pub async fn create_index_commit_job(
    scheduler: &SchedulerService,
    indexer: Arc<SearchIndexer>,
    config: IndexCommitJobConfig,
) -> Result<(), SchedulerError> {
    scheduler
        .register_job(
            "search_index_commit",
            &config.cron,
            Some(&config.timezone),
            OverlapPolicy::Skip,
            JitterConfig::new(config.jitter_secs),
            TimeoutConfig::new(config.timeout_secs),
            move || {
                let indexer = indexer.clone();
                async move {
                    match indexer.commit() {
                        Ok(opstamp) => {
                            tracing::debug!(opstamp, "Search index committed");
                            Ok(())
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "Search index commit failed");
                            Err(e.to_string())
                        }
                    }
                }
            },
        )
        .await?;

    tracing::info!("Registered search index commit job");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = IndexCommitJobConfig::default();
        assert_eq!(config.cron, "0 * * * * *");
        assert_eq!(config.timezone, "UTC");
        assert_eq!(config.jitter_secs, 10);
        assert_eq!(config.timeout_secs, 60);
    }

    #[test]
    fn test_config_serialization() {
        let config = IndexCommitJobConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: IndexCommitJobConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.cron, decoded.cron);
        assert_eq!(config.timezone, decoded.timezone);
        assert_eq!(config.jitter_secs, decoded.jitter_secs);
    }
}
