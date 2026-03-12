//! Episode RPC handlers for episodic memory.
//!
//! Implements Phase 44 Episodic Memory RPCs:
//! - StartEpisode: Begin tracking a task execution
//! - RecordAction: Record an action within an episode
//! - CompleteEpisode: Finish an episode with outcome and lessons
//! - GetSimilarEpisodes: Find similar episodes via cosine similarity
//!
//! Follows the AgentDiscoveryHandler/TopicGraphHandler pattern with `Arc<Storage>`.

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use tonic::{Request, Response, Status};
use tracing::{debug, info, warn};

use memory_storage::Storage;
use memory_types::config::EpisodicConfig;
use memory_types::{Action, ActionResult, Episode, EpisodeStatus};

use crate::novelty::EmbedderTrait;
use crate::pb::{
    ActionResultStatus, CompleteEpisodeRequest, CompleteEpisodeResponse, EpisodeAction,
    EpisodeStatusProto, EpisodeSummary, GetSimilarEpisodesRequest, GetSimilarEpisodesResponse,
    RecordActionRequest, RecordActionResponse, StartEpisodeRequest, StartEpisodeResponse,
};

/// Handler for episodic memory RPCs.
pub struct EpisodeHandler {
    storage: Arc<Storage>,
    config: EpisodicConfig,
    embedder: Option<Arc<dyn EmbedderTrait>>,
}

impl EpisodeHandler {
    /// Create a new episode handler.
    pub fn new(storage: Arc<Storage>, config: EpisodicConfig) -> Self {
        Self {
            storage,
            config,
            embedder: None,
        }
    }

    /// Set the embedder for generating episode embeddings.
    pub fn with_embedder(mut self, embedder: Arc<dyn EmbedderTrait>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    /// Handle StartEpisode RPC.
    pub async fn start_episode(
        &self,
        request: Request<StartEpisodeRequest>,
    ) -> Result<Response<StartEpisodeResponse>, Status> {
        if !self.config.enabled {
            return Err(Status::failed_precondition(
                "Episodic memory is not enabled",
            ));
        }

        let req = request.into_inner();

        if req.task.is_empty() {
            return Err(Status::invalid_argument("task is required"));
        }

        let episode_id = ulid::Ulid::new().to_string();
        let mut episode = Episode::new(episode_id.clone(), req.task).with_plan(req.plan);

        if let Some(agent) = req.agent {
            episode = episode.with_agent(agent);
        }

        self.storage
            .store_episode(&episode)
            .map_err(|e| Status::internal(format!("Failed to store episode: {e}")))?;

        info!(episode_id = %episode_id, "Started episode");

        Ok(Response::new(StartEpisodeResponse {
            episode_id,
            created: true,
        }))
    }

    /// Handle RecordAction RPC.
    pub async fn record_action(
        &self,
        request: Request<RecordActionRequest>,
    ) -> Result<Response<RecordActionResponse>, Status> {
        if !self.config.enabled {
            return Err(Status::failed_precondition(
                "Episodic memory is not enabled",
            ));
        }

        let req = request.into_inner();

        if req.episode_id.is_empty() {
            return Err(Status::invalid_argument("episode_id is required"));
        }

        let proto_action = req
            .action
            .ok_or_else(|| Status::invalid_argument("action is required"))?;

        let mut episode = self
            .storage
            .get_episode(&req.episode_id)
            .map_err(|e| Status::internal(format!("Failed to get episode: {e}")))?
            .ok_or_else(|| Status::not_found("Episode not found"))?;

        if episode.status != EpisodeStatus::InProgress {
            return Err(Status::failed_precondition(
                "Cannot record actions on a completed or failed episode",
            ));
        }

        let action = convert_proto_action(proto_action)?;
        episode.add_action(action);

        self.storage
            .update_episode(&episode)
            .map_err(|e| Status::internal(format!("Failed to update episode: {e}")))?;

        let action_count = episode.actions.len() as u32;
        debug!(episode_id = %req.episode_id, action_count, "Recorded action");

        Ok(Response::new(RecordActionResponse {
            recorded: true,
            action_count,
        }))
    }

    /// Handle CompleteEpisode RPC.
    pub async fn complete_episode(
        &self,
        request: Request<CompleteEpisodeRequest>,
    ) -> Result<Response<CompleteEpisodeResponse>, Status> {
        if !self.config.enabled {
            return Err(Status::failed_precondition(
                "Episodic memory is not enabled",
            ));
        }

        let req = request.into_inner();

        if req.episode_id.is_empty() {
            return Err(Status::invalid_argument("episode_id is required"));
        }

        if !(0.0..=1.0).contains(&req.outcome_score) {
            return Err(Status::invalid_argument(
                "outcome_score must be between 0.0 and 1.0",
            ));
        }

        let mut episode = self
            .storage
            .get_episode(&req.episode_id)
            .map_err(|e| Status::internal(format!("Failed to get episode: {e}")))?
            .ok_or_else(|| Status::not_found("Episode not found"))?;

        if episode.status != EpisodeStatus::InProgress {
            return Err(Status::failed_precondition(
                "Episode is already completed or failed",
            ));
        }

        // Complete or fail the episode
        let midpoint = self.config.midpoint_target;
        if req.failed {
            episode.fail(req.outcome_score, midpoint);
        } else {
            episode.complete(req.outcome_score, midpoint);
        }

        episode.lessons_learned = req.lessons_learned;
        episode.failure_modes = req.failure_modes;

        // Generate embedding from task + lessons
        if let Some(ref embedder) = self.embedder {
            let text = build_embedding_text(&episode);
            match embedder.embed(&text).await {
                Ok(embedding) => {
                    episode.embedding = Some(embedding);
                }
                Err(e) => {
                    warn!(episode_id = %req.episode_id, "Failed to generate episode embedding: {e}");
                    // Fail-open: continue without embedding
                }
            }
        }

        let value_score = episode.value_score.unwrap_or(0.0);

        self.storage
            .update_episode(&episode)
            .map_err(|e| Status::internal(format!("Failed to update episode: {e}")))?;

        // Value-based retention pruning
        let episodes_pruned = self.prune_if_over_limit()?;

        info!(
            episode_id = %req.episode_id,
            value_score,
            episodes_pruned,
            "Completed episode"
        );

        Ok(Response::new(CompleteEpisodeResponse {
            completed: true,
            value_score,
            episodes_pruned,
        }))
    }

    /// Handle GetSimilarEpisodes RPC.
    pub async fn get_similar_episodes(
        &self,
        request: Request<GetSimilarEpisodesRequest>,
    ) -> Result<Response<GetSimilarEpisodesResponse>, Status> {
        if !self.config.enabled {
            return Err(Status::failed_precondition(
                "Episodic memory is not enabled",
            ));
        }

        let req = request.into_inner();

        if req.query.is_empty() {
            return Err(Status::invalid_argument("query is required"));
        }

        let top_k = if req.top_k == 0 { 5 } else { req.top_k } as usize;
        let min_score = req.min_score;

        // Embed the query
        let embedder = self.embedder.as_ref().ok_or_else(|| {
            Status::unavailable("Embedder not configured for episode similarity search")
        })?;

        let query_embedding = embedder
            .embed(&req.query)
            .await
            .map_err(|e| Status::internal(format!("Failed to embed query: {e}")))?;

        // Load all episodes and compute cosine similarity
        let episodes = self
            .storage
            .list_episodes(self.config.max_episodes)
            .map_err(|e| Status::internal(format!("Failed to list episodes: {e}")))?;

        let mut scored: Vec<(f32, &Episode)> = episodes
            .iter()
            .filter_map(|ep| {
                let embedding = ep.embedding.as_ref()?;
                let sim = cosine_similarity(&query_embedding, embedding);
                if sim >= min_score {
                    Some((sim, ep))
                } else {
                    None
                }
            })
            .collect();

        // Sort by similarity descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        let summaries: Vec<EpisodeSummary> = scored
            .iter()
            .map(|(sim, ep)| episode_to_summary(ep, *sim))
            .collect();

        debug!(results = summaries.len(), "Found similar episodes");

        Ok(Response::new(GetSimilarEpisodesResponse {
            episodes: summaries,
        }))
    }

    /// Prune lowest-value episodes if total exceeds max_episodes.
    #[allow(clippy::result_large_err)]
    fn prune_if_over_limit(&self) -> Result<u32, Status> {
        let all_episodes = self
            .storage
            .list_episodes(self.config.max_episodes + 100) // fetch a bit more
            .map_err(|e| Status::internal(format!("Failed to list episodes: {e}")))?;

        if all_episodes.len() <= self.config.max_episodes {
            return Ok(0);
        }

        let excess = all_episodes.len() - self.config.max_episodes;

        // Sort by value_score ascending (lowest first) to find prune candidates
        let mut sortable: Vec<&Episode> = all_episodes.iter().collect();
        sortable.sort_by(|a, b| {
            let va = a.value_score.unwrap_or(0.0);
            let vb = b.value_score.unwrap_or(0.0);
            va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut pruned = 0u32;
        for ep in sortable.iter().take(excess) {
            if let Err(e) = self.storage.delete_episode(&ep.episode_id) {
                warn!(episode_id = %ep.episode_id, "Failed to prune episode: {e}");
                continue;
            }
            pruned += 1;
        }

        if pruned > 0 {
            info!(pruned, "Pruned low-value episodes");
        }

        Ok(pruned)
    }
}

/// Convert a proto EpisodeAction to a domain Action.
#[allow(clippy::result_large_err)]
fn convert_proto_action(proto: EpisodeAction) -> Result<Action, Status> {
    let result = match ActionResultStatus::try_from(proto.result_status) {
        Ok(ActionResultStatus::ActionResultSuccess) => ActionResult::Success(proto.result_detail),
        Ok(ActionResultStatus::ActionResultFailure) => ActionResult::Failure(proto.result_detail),
        Ok(ActionResultStatus::ActionResultPending)
        | Ok(ActionResultStatus::ActionResultUnspecified) => ActionResult::Pending,
        Err(_) => ActionResult::Pending,
    };

    let timestamp = if proto.timestamp_ms > 0 {
        Utc.timestamp_millis_opt(proto.timestamp_ms)
            .single()
            .unwrap_or_else(Utc::now)
    } else {
        Utc::now()
    };

    Ok(Action {
        action_type: proto.action_type,
        input: proto.input,
        result,
        timestamp,
    })
}

/// Build embedding text from episode task + lessons.
fn build_embedding_text(episode: &Episode) -> String {
    let mut parts = vec![episode.task.clone()];
    for lesson in &episode.lessons_learned {
        parts.push(lesson.clone());
    }
    for mode in &episode.failure_modes {
        parts.push(mode.clone());
    }
    parts.join(". ")
}

/// Compute cosine similarity between two vectors.
///
/// Assumes vectors are pre-normalized (dot product = cosine similarity).
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Convert an Episode to a proto EpisodeSummary.
fn episode_to_summary(episode: &Episode, similarity_score: f32) -> EpisodeSummary {
    let status = match episode.status {
        EpisodeStatus::InProgress => EpisodeStatusProto::EpisodeStatusInProgress,
        EpisodeStatus::Completed => EpisodeStatusProto::EpisodeStatusCompleted,
        EpisodeStatus::Failed => EpisodeStatusProto::EpisodeStatusFailed,
    };

    EpisodeSummary {
        episode_id: episode.episode_id.clone(),
        task: episode.task.clone(),
        status: status.into(),
        outcome_score: episode.outcome_score.unwrap_or(0.0),
        value_score: episode.value_score.unwrap_or(0.0),
        similarity_score,
        lessons_learned: episode.lessons_learned.clone(),
        failure_modes: episode.failure_modes.clone(),
        action_count: episode.actions.len() as u32,
        created_at_ms: episode.created_at.timestamp_millis(),
        agent: episode.agent.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_types::config::EpisodicConfig;
    use tempfile::TempDir;

    fn create_test_handler() -> (EpisodeHandler, Arc<Storage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::open(temp_dir.path()).unwrap());
        let config = EpisodicConfig {
            enabled: true,
            ..Default::default()
        };
        let handler = EpisodeHandler::new(storage.clone(), config);
        (handler, storage, temp_dir)
    }

    fn create_disabled_handler() -> (EpisodeHandler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::open(temp_dir.path()).unwrap());
        let config = EpisodicConfig::default(); // disabled
        let handler = EpisodeHandler::new(storage, config);
        (handler, temp_dir)
    }

    #[tokio::test]
    async fn test_start_episode() {
        let (handler, _, _temp) = create_test_handler();

        let response = handler
            .start_episode(Request::new(StartEpisodeRequest {
                task: "Build auth system".to_string(),
                plan: vec!["Design schema".to_string(), "Implement JWT".to_string()],
                agent: Some("claude".to_string()),
            }))
            .await
            .unwrap();

        let resp = response.into_inner();
        assert!(resp.created);
        assert!(!resp.episode_id.is_empty());
    }

    #[tokio::test]
    async fn test_start_episode_disabled() {
        let (handler, _temp) = create_disabled_handler();

        let result = handler
            .start_episode(Request::new(StartEpisodeRequest {
                task: "test".to_string(),
                plan: vec![],
                agent: None,
            }))
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::FailedPrecondition);
    }

    #[tokio::test]
    async fn test_start_episode_empty_task() {
        let (handler, _, _temp) = create_test_handler();

        let result = handler
            .start_episode(Request::new(StartEpisodeRequest {
                task: "".to_string(),
                plan: vec![],
                agent: None,
            }))
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_record_action() {
        let (handler, _, _temp) = create_test_handler();

        // Start episode
        let start_resp = handler
            .start_episode(Request::new(StartEpisodeRequest {
                task: "test task".to_string(),
                plan: vec![],
                agent: None,
            }))
            .await
            .unwrap()
            .into_inner();

        // Record action
        let response = handler
            .record_action(Request::new(RecordActionRequest {
                episode_id: start_resp.episode_id.clone(),
                action: Some(EpisodeAction {
                    action_type: "tool_call".to_string(),
                    input: "read file".to_string(),
                    result_status: ActionResultStatus::ActionResultSuccess.into(),
                    result_detail: "file contents".to_string(),
                    timestamp_ms: Utc::now().timestamp_millis(),
                }),
            }))
            .await
            .unwrap();

        let resp = response.into_inner();
        assert!(resp.recorded);
        assert_eq!(resp.action_count, 1);
    }

    #[tokio::test]
    async fn test_record_action_not_found() {
        let (handler, _, _temp) = create_test_handler();

        let result = handler
            .record_action(Request::new(RecordActionRequest {
                episode_id: "nonexistent".to_string(),
                action: Some(EpisodeAction {
                    action_type: "tool_call".to_string(),
                    input: "test".to_string(),
                    result_status: ActionResultStatus::ActionResultSuccess.into(),
                    result_detail: "ok".to_string(),
                    timestamp_ms: 0,
                }),
            }))
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_complete_episode() {
        let (handler, storage, _temp) = create_test_handler();

        // Start episode
        let start_resp = handler
            .start_episode(Request::new(StartEpisodeRequest {
                task: "test task".to_string(),
                plan: vec![],
                agent: None,
            }))
            .await
            .unwrap()
            .into_inner();

        // Complete episode
        let response = handler
            .complete_episode(Request::new(CompleteEpisodeRequest {
                episode_id: start_resp.episode_id.clone(),
                outcome_score: 0.65,
                failed: false,
                lessons_learned: vec!["Always test first".to_string()],
                failure_modes: vec![],
            }))
            .await
            .unwrap();

        let resp = response.into_inner();
        assert!(resp.completed);
        // At midpoint (0.65), value score = 1.0
        assert!((resp.value_score - 1.0).abs() < f32::EPSILON);

        // Verify storage
        let stored = storage
            .get_episode(&start_resp.episode_id)
            .unwrap()
            .unwrap();
        assert_eq!(stored.status, EpisodeStatus::Completed);
        assert_eq!(stored.lessons_learned, vec!["Always test first"]);
    }

    #[tokio::test]
    async fn test_complete_episode_failed() {
        let (handler, storage, _temp) = create_test_handler();

        let start_resp = handler
            .start_episode(Request::new(StartEpisodeRequest {
                task: "failing task".to_string(),
                plan: vec![],
                agent: None,
            }))
            .await
            .unwrap()
            .into_inner();

        let response = handler
            .complete_episode(Request::new(CompleteEpisodeRequest {
                episode_id: start_resp.episode_id.clone(),
                outcome_score: 0.2,
                failed: true,
                lessons_learned: vec![],
                failure_modes: vec!["Timeout on API".to_string()],
            }))
            .await
            .unwrap();

        let resp = response.into_inner();
        assert!(resp.completed);

        let stored = storage
            .get_episode(&start_resp.episode_id)
            .unwrap()
            .unwrap();
        assert_eq!(stored.status, EpisodeStatus::Failed);
        assert_eq!(stored.failure_modes, vec!["Timeout on API"]);
    }

    #[tokio::test]
    async fn test_complete_episode_invalid_score() {
        let (handler, _, _temp) = create_test_handler();

        let start_resp = handler
            .start_episode(Request::new(StartEpisodeRequest {
                task: "test".to_string(),
                plan: vec![],
                agent: None,
            }))
            .await
            .unwrap()
            .into_inner();

        let result = handler
            .complete_episode(Request::new(CompleteEpisodeRequest {
                episode_id: start_resp.episode_id,
                outcome_score: 1.5,
                failed: false,
                lessons_learned: vec![],
                failure_modes: vec![],
            }))
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_complete_already_completed() {
        let (handler, _, _temp) = create_test_handler();

        let start_resp = handler
            .start_episode(Request::new(StartEpisodeRequest {
                task: "test".to_string(),
                plan: vec![],
                agent: None,
            }))
            .await
            .unwrap()
            .into_inner();

        // Complete once
        handler
            .complete_episode(Request::new(CompleteEpisodeRequest {
                episode_id: start_resp.episode_id.clone(),
                outcome_score: 0.5,
                failed: false,
                lessons_learned: vec![],
                failure_modes: vec![],
            }))
            .await
            .unwrap();

        // Try again
        let result = handler
            .complete_episode(Request::new(CompleteEpisodeRequest {
                episode_id: start_resp.episode_id,
                outcome_score: 0.8,
                failed: false,
                lessons_learned: vec![],
                failure_modes: vec![],
            }))
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::FailedPrecondition);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < f32::EPSILON);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < f32::EPSILON);

        // Empty or mismatched
        assert!((cosine_similarity(&[], &[]) - 0.0).abs() < f32::EPSILON);
        assert!((cosine_similarity(&[1.0], &[1.0, 2.0]) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_build_embedding_text() {
        let mut episode = Episode::new("test".to_string(), "Build auth".to_string());
        episode.lessons_learned = vec!["Use JWT".to_string()];
        episode.failure_modes = vec!["Timeout".to_string()];

        let text = build_embedding_text(&episode);
        assert_eq!(text, "Build auth. Use JWT. Timeout");
    }

    #[tokio::test]
    async fn test_value_based_pruning() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::open(temp_dir.path()).unwrap());
        let config = EpisodicConfig {
            enabled: true,
            max_episodes: 3,
            ..Default::default()
        };
        let handler = EpisodeHandler::new(storage.clone(), config);

        // Create 4 episodes with different value scores
        for (i, score) in [0.1, 0.9, 0.5, 0.65].iter().enumerate() {
            let start_resp = handler
                .start_episode(Request::new(StartEpisodeRequest {
                    task: format!("task {i}"),
                    plan: vec![],
                    agent: None,
                }))
                .await
                .unwrap()
                .into_inner();

            // Small delay so ULIDs are distinct
            std::thread::sleep(std::time::Duration::from_millis(2));

            handler
                .complete_episode(Request::new(CompleteEpisodeRequest {
                    episode_id: start_resp.episode_id,
                    outcome_score: *score,
                    failed: false,
                    lessons_learned: vec![],
                    failure_modes: vec![],
                }))
                .await
                .unwrap();
        }

        // After 4th episode, pruning should have removed 1
        let remaining = storage.list_episodes(100).unwrap();
        assert_eq!(remaining.len(), 3);
    }
}
