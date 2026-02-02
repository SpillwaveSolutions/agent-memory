//! RocksDB compaction job.
//!
//! Triggers manual compaction to optimize storage by:
//! - Reclaiming deleted space
//! - Merging SST files
//! - Reducing read amplification
//!
//! By default runs weekly at 4 AM Sunday to minimize impact
//! on normal operations.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::info;

use memory_storage::Storage;

use crate::{JitterConfig, OverlapPolicy, SchedulerError, SchedulerService, TimeoutConfig};

/// Configuration for the compaction job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionJobConfig {
    /// Cron expression (default: "0 0 4 * * 0" = 4 AM Sunday)
    pub cron: String,

    /// Timezone (default: "UTC")
    pub timezone: String,

    /// Max jitter in seconds (default: 600 = 10 min)
    pub jitter_secs: u64,

    /// Timeout in seconds (default: 3600 = 1 hour)
    pub timeout_secs: u64,
}

impl Default for CompactionJobConfig {
    fn default() -> Self {
        Self {
            cron: "0 0 4 * * 0".to_string(),
            timezone: "UTC".to_string(),
            jitter_secs: 600,
            timeout_secs: 3600, // 1 hour
        }
    }
}

/// Register compaction job with the scheduler.
///
/// Creates a job that triggers RocksDB compaction on all column families.
/// Uses OverlapPolicy::Skip to prevent concurrent compaction runs.
///
/// # Arguments
///
/// * `scheduler` - The scheduler service to register the job with
/// * `storage` - Storage instance to compact
/// * `config` - Configuration for job schedule
///
/// # Errors
///
/// Returns error if job registration fails (invalid cron, invalid timezone).
pub async fn create_compaction_job(
    scheduler: &SchedulerService,
    storage: Arc<Storage>,
    config: CompactionJobConfig,
) -> Result<(), SchedulerError> {
    scheduler
        .register_job(
            "rocksdb_compaction",
            &config.cron,
            Some(&config.timezone),
            OverlapPolicy::Skip,
            JitterConfig::new(config.jitter_secs),
            TimeoutConfig::new(config.timeout_secs),
            move || {
                let storage = storage.clone();
                async move {
                    info!("Starting manual compaction");
                    storage
                        .compact()
                        .map(|_| info!("Compaction complete"))
                        .map_err(|e| e.to_string())
                }
            },
        )
        .await?;

    info!("Registered compaction job");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compaction_config_default() {
        let config = CompactionJobConfig::default();

        assert_eq!(config.cron, "0 0 4 * * 0");
        assert_eq!(config.timezone, "UTC");
        assert_eq!(config.jitter_secs, 600);
        assert_eq!(config.timeout_secs, 3600);
    }

    #[test]
    fn test_compaction_config_serialization() {
        let config = CompactionJobConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: CompactionJobConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.cron, decoded.cron);
        assert_eq!(config.timezone, decoded.timezone);
    }
}
