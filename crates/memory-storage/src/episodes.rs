//! Episode storage operations for episodic memory.
//!
//! Provides CRUD operations for episodes in the CF_EPISODES column family.
//! Episodes are stored as JSON-serialized values keyed by episode_id.

use crate::column_families::CF_EPISODES;
use crate::error::StorageError;
use crate::Storage;
use memory_types::Episode;
use tracing::debug;

impl Storage {
    /// Store an episode in the episodes column family.
    ///
    /// The episode is serialized to JSON and stored with its episode_id as key.
    pub fn store_episode(&self, episode: &Episode) -> Result<(), StorageError> {
        let bytes = serde_json::to_vec(episode)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        self.put(CF_EPISODES, episode.episode_id.as_bytes(), &bytes)?;
        debug!(episode_id = %episode.episode_id, "Stored episode");
        Ok(())
    }

    /// Get an episode by its ID.
    pub fn get_episode(&self, episode_id: &str) -> Result<Option<Episode>, StorageError> {
        match self.get(CF_EPISODES, episode_id.as_bytes())? {
            Some(bytes) => {
                let episode: Episode = serde_json::from_slice(&bytes)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(episode))
            }
            None => Ok(None),
        }
    }

    /// List episodes, newest first (by ULID lexicographic order, reversed).
    ///
    /// Returns up to `limit` episodes. Uses reverse iteration over the
    /// CF_EPISODES column family, so ULID-keyed episodes come out newest first.
    pub fn list_episodes(&self, limit: usize) -> Result<Vec<Episode>, StorageError> {
        let cf = self
            .db
            .cf_handle(CF_EPISODES)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_EPISODES.to_string()))?;

        let mut episodes = Vec::new();
        let iter = self
            .db
            .iterator_cf(&cf, rocksdb::IteratorMode::End);

        for item in iter.take(limit) {
            let (_, value) = item?;
            let episode: Episode = serde_json::from_slice(&value)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            episodes.push(episode);
        }

        Ok(episodes)
    }

    /// Update an episode (overwrite by ID).
    ///
    /// This is equivalent to store_episode but semantically indicates an update.
    pub fn update_episode(&self, episode: &Episode) -> Result<(), StorageError> {
        self.store_episode(episode)
    }

    /// Delete an episode by its ID.
    pub fn delete_episode(&self, episode_id: &str) -> Result<(), StorageError> {
        self.delete(CF_EPISODES, episode_id.as_bytes())?;
        debug!(episode_id = %episode_id, "Deleted episode");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use memory_types::{Action, ActionResult, Episode, EpisodeStatus};
    use tempfile::TempDir;

    use crate::Storage;

    fn create_test_storage() -> (Storage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::open(temp_dir.path()).unwrap();
        (storage, temp_dir)
    }

    #[test]
    fn test_episode_store_and_get() {
        let (storage, _tmp) = create_test_storage();

        let episode = Episode::new(
            ulid::Ulid::new().to_string(),
            "Build auth system".to_string(),
        )
        .with_plan(vec![
            "Design schema".to_string(),
            "Implement JWT".to_string(),
        ])
        .with_agent("claude");

        storage.store_episode(&episode).unwrap();

        let retrieved = storage.get_episode(&episode.episode_id).unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.episode_id, episode.episode_id);
        assert_eq!(retrieved.task, "Build auth system");
        assert_eq!(retrieved.plan.len(), 2);
        assert_eq!(retrieved.agent, Some("claude".to_string()));
    }

    #[test]
    fn test_episode_get_not_found() {
        let (storage, _tmp) = create_test_storage();

        let result = storage.get_episode("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_episode_update() {
        let (storage, _tmp) = create_test_storage();

        let mut episode = Episode::new(
            ulid::Ulid::new().to_string(),
            "Build auth system".to_string(),
        );

        storage.store_episode(&episode).unwrap();

        // Update with action and completion
        episode.add_action(Action {
            action_type: "tool_call".to_string(),
            input: "read auth.rs".to_string(),
            result: ActionResult::Success("file contents".to_string()),
            timestamp: chrono::Utc::now(),
        });
        episode.complete(0.8, 0.65);

        storage.update_episode(&episode).unwrap();

        let retrieved = storage.get_episode(&episode.episode_id).unwrap().unwrap();
        assert_eq!(retrieved.status, EpisodeStatus::Completed);
        assert_eq!(retrieved.actions.len(), 1);
        assert!(retrieved.outcome_score.is_some());
        assert!(retrieved.value_score.is_some());
    }

    #[test]
    fn test_episode_delete() {
        let (storage, _tmp) = create_test_storage();

        let episode = Episode::new(ulid::Ulid::new().to_string(), "test task".to_string());

        storage.store_episode(&episode).unwrap();
        assert!(storage.get_episode(&episode.episode_id).unwrap().is_some());

        storage.delete_episode(&episode.episode_id).unwrap();
        assert!(storage.get_episode(&episode.episode_id).unwrap().is_none());
    }

    #[test]
    fn test_episode_list_newest_first() {
        let (storage, _tmp) = create_test_storage();

        // Create episodes with sequential ULIDs (newer = lexicographically later)
        let ids: Vec<String> = (0..5)
            .map(|_| {
                let id = ulid::Ulid::new().to_string();
                std::thread::sleep(std::time::Duration::from_millis(2));
                id
            })
            .collect();

        for (i, id) in ids.iter().enumerate() {
            let episode = Episode::new(id.clone(), format!("task {i}"));
            storage.store_episode(&episode).unwrap();
        }

        let listed = storage.list_episodes(3).unwrap();
        assert_eq!(listed.len(), 3);

        // Should be newest first (reverse ULID order)
        assert_eq!(listed[0].episode_id, ids[4]);
        assert_eq!(listed[1].episode_id, ids[3]);
        assert_eq!(listed[2].episode_id, ids[2]);
    }

    #[test]
    fn test_episode_list_empty() {
        let (storage, _tmp) = create_test_storage();

        let listed = storage.list_episodes(10).unwrap();
        assert!(listed.is_empty());
    }

    #[test]
    fn test_episode_roundtrip_with_actions() {
        let (storage, _tmp) = create_test_storage();

        let mut episode =
            Episode::new(ulid::Ulid::new().to_string(), "Complex task".to_string())
                .with_agent("claude");

        episode.add_action(Action {
            action_type: "tool_call".to_string(),
            input: "read file".to_string(),
            result: ActionResult::Success("contents".to_string()),
            timestamp: chrono::Utc::now(),
        });
        episode.add_action(Action {
            action_type: "api_call".to_string(),
            input: "create resource".to_string(),
            result: ActionResult::Failure("timeout".to_string()),
            timestamp: chrono::Utc::now(),
        });
        episode.add_action(Action {
            action_type: "retry".to_string(),
            input: "create resource".to_string(),
            result: ActionResult::Pending,
            timestamp: chrono::Utc::now(),
        });

        episode
            .lessons_learned
            .push("Always set timeouts".to_string());
        episode
            .failure_modes
            .push("API timeout under load".to_string());

        storage.store_episode(&episode).unwrap();

        let retrieved = storage.get_episode(&episode.episode_id).unwrap().unwrap();
        assert_eq!(retrieved.actions.len(), 3);
        assert_eq!(retrieved.lessons_learned.len(), 1);
        assert_eq!(retrieved.failure_modes.len(), 1);
        assert_eq!(
            retrieved.actions[0].result,
            ActionResult::Success("contents".to_string())
        );
        assert_eq!(
            retrieved.actions[1].result,
            ActionResult::Failure("timeout".to_string())
        );
        assert_eq!(retrieved.actions[2].result, ActionResult::Pending);
    }
}
