//! E2E tests for episodic memory (Phase 44).
//!
//! Validates:
//! - Episode lifecycle: start -> record actions -> complete -> verify storage
//! - Value-based retention: multiple episodes with varying scores, verify pruning
//! - Disabled config: RPCs return appropriate error when episodic memory is disabled

use std::sync::Arc;

use pretty_assertions::assert_eq;
use tonic::Request;

use e2e_tests::TestHarness;
use memory_service::pb::memory_service_server::MemoryService;
use memory_service::pb::{
    ActionResultStatus, CompleteEpisodeRequest, EpisodeAction, RecordActionRequest,
    StartEpisodeRequest,
};
use memory_service::{EpisodeHandler, MemoryServiceImpl};
use memory_types::config::EpisodicConfig;

/// Create a MemoryServiceImpl with episodic memory enabled.
fn create_episodic_service(harness: &TestHarness, config: EpisodicConfig) -> MemoryServiceImpl {
    let handler = Arc::new(EpisodeHandler::new(harness.storage.clone(), config));
    let mut service = MemoryServiceImpl::new(harness.storage.clone());
    service.set_episode_handler(handler);
    service
}

/// E2E test: Full episode lifecycle through gRPC service layer.
///
/// Validates: StartEpisode -> RecordAction (x2) -> CompleteEpisode -> verify storage.
#[tokio::test]
async fn test_episode_lifecycle_e2e() {
    let harness = TestHarness::new();
    let config = EpisodicConfig {
        enabled: true,
        ..Default::default()
    };
    let service = create_episodic_service(&harness, config);

    // 1. Start episode
    let start_resp = service
        .start_episode(Request::new(StartEpisodeRequest {
            task: "Implement authentication module".to_string(),
            plan: vec![
                "Design JWT schema".to_string(),
                "Implement token validation".to_string(),
                "Add refresh token rotation".to_string(),
            ],
            agent: Some("claude".to_string()),
        }))
        .await
        .unwrap()
        .into_inner();

    assert!(start_resp.created);
    let episode_id = start_resp.episode_id.clone();
    assert!(!episode_id.is_empty());

    // 2. Record first action (success)
    let action1_resp = service
        .record_action(Request::new(RecordActionRequest {
            episode_id: episode_id.clone(),
            action: Some(EpisodeAction {
                action_type: "tool_call".to_string(),
                input: "Read existing auth code".to_string(),
                result_status: ActionResultStatus::ActionResultSuccess.into(),
                result_detail: "Found existing JWT utils".to_string(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
            }),
        }))
        .await
        .unwrap()
        .into_inner();

    assert!(action1_resp.recorded);
    assert_eq!(action1_resp.action_count, 1);

    // 3. Record second action (failure then retry)
    let action2_resp = service
        .record_action(Request::new(RecordActionRequest {
            episode_id: episode_id.clone(),
            action: Some(EpisodeAction {
                action_type: "api_request".to_string(),
                input: "Test token endpoint".to_string(),
                result_status: ActionResultStatus::ActionResultFailure.into(),
                result_detail: "Connection refused".to_string(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
            }),
        }))
        .await
        .unwrap()
        .into_inner();

    assert!(action2_resp.recorded);
    assert_eq!(action2_resp.action_count, 2);

    // 4. Complete episode with moderate success
    let complete_resp = service
        .complete_episode(Request::new(CompleteEpisodeRequest {
            episode_id: episode_id.clone(),
            outcome_score: 0.65,
            failed: false,
            lessons_learned: vec![
                "JWT refresh rotation prevents token theft".to_string(),
                "Always test endpoints before deploying".to_string(),
            ],
            failure_modes: vec!["API connectivity issues in test environment".to_string()],
        }))
        .await
        .unwrap()
        .into_inner();

    assert!(complete_resp.completed);
    // At midpoint (0.65), value score = 1.0
    assert!(
        (complete_resp.value_score - 1.0).abs() < f32::EPSILON,
        "Expected value_score 1.0 at midpoint, got {}",
        complete_resp.value_score
    );

    // 5. Verify episode in storage
    let stored = harness
        .storage
        .get_episode(&episode_id)
        .unwrap()
        .expect("Episode should be in storage");

    assert_eq!(stored.task, "Implement authentication module");
    assert_eq!(stored.plan.len(), 3);
    assert_eq!(stored.actions.len(), 2);
    assert_eq!(stored.status, memory_types::EpisodeStatus::Completed);
    assert_eq!(stored.lessons_learned.len(), 2);
    assert_eq!(stored.failure_modes.len(), 1);
    assert_eq!(stored.agent, Some("claude".to_string()));
    assert!(stored.outcome_score.is_some());
    assert!(stored.value_score.is_some());
    assert!(stored.completed_at.is_some());
}

/// E2E test: Value-based retention pruning.
///
/// Creates episodes exceeding max_episodes limit and verifies lowest-value
/// episodes are pruned after completion.
#[tokio::test]
async fn test_value_based_retention_pruning_e2e() {
    let harness = TestHarness::new();
    let config = EpisodicConfig {
        enabled: true,
        max_episodes: 3,
        midpoint_target: 0.65,
        ..Default::default()
    };
    let service = create_episodic_service(&harness, config);

    // Create episodes with different outcome scores (and thus different value scores)
    // Score 0.1 -> far from midpoint -> low value
    // Score 0.65 -> at midpoint -> highest value
    // Score 0.9 -> far from midpoint -> medium value
    // Score 0.5 -> near midpoint -> high value
    let scores = [0.1, 0.65, 0.9, 0.5];
    let mut episode_ids = Vec::new();

    for (i, score) in scores.iter().enumerate() {
        let start_resp = service
            .start_episode(Request::new(StartEpisodeRequest {
                task: format!("Task {} with score {}", i, score),
                plan: vec![],
                agent: None,
            }))
            .await
            .unwrap()
            .into_inner();

        episode_ids.push(start_resp.episode_id.clone());

        // Small delay to ensure distinct ULIDs
        std::thread::sleep(std::time::Duration::from_millis(2));

        let complete_resp = service
            .complete_episode(Request::new(CompleteEpisodeRequest {
                episode_id: start_resp.episode_id,
                outcome_score: *score,
                failed: false,
                lessons_learned: vec![],
                failure_modes: vec![],
            }))
            .await
            .unwrap()
            .into_inner();

        assert!(complete_resp.completed);
    }

    // After 4th episode completion, should have pruned 1 (down to max_episodes=3)
    let remaining = harness.storage.list_episodes(100).unwrap();
    assert_eq!(
        remaining.len(),
        3,
        "Should have pruned to max_episodes=3, got {}",
        remaining.len()
    );

    // The episode with score=0.1 (value_score = 1.0 - |0.1 - 0.65| = 0.45) should be pruned
    // because it has the lowest value score among all four.
    // Score 0.65 -> value 1.0 (highest)
    // Score 0.5 -> value 1.0 - |0.5 - 0.65| = 0.85
    // Score 0.9 -> value 1.0 - |0.9 - 0.65| = 0.75
    // Score 0.1 -> value 1.0 - |0.1 - 0.65| = 0.45 (lowest -- pruned)
    let remaining_ids: Vec<&str> = remaining.iter().map(|e| e.episode_id.as_str()).collect();
    assert!(
        !remaining_ids.contains(&episode_ids[0].as_str()),
        "Episode with lowest value (score=0.1) should have been pruned"
    );
}

/// E2E test: Disabled episodic memory returns FailedPrecondition.
///
/// When EpisodicConfig.enabled=false, all episode RPCs should return
/// appropriate error status.
#[tokio::test]
async fn test_episodic_disabled_returns_error() {
    let harness = TestHarness::new();
    let config = EpisodicConfig::default(); // disabled by default
    let service = create_episodic_service(&harness, config);

    let start_result = service
        .start_episode(Request::new(StartEpisodeRequest {
            task: "should fail".to_string(),
            plan: vec![],
            agent: None,
        }))
        .await;

    assert!(start_result.is_err());
    assert_eq!(
        start_result.unwrap_err().code(),
        tonic::Code::FailedPrecondition
    );
}

/// E2E test: No episode handler returns FailedPrecondition.
///
/// When episode_handler is None (not configured), all episode RPCs should return
/// appropriate error status.
#[tokio::test]
async fn test_episodic_no_handler_returns_error() {
    let harness = TestHarness::new();
    let service = MemoryServiceImpl::new(harness.storage.clone());

    let start_result = service
        .start_episode(Request::new(StartEpisodeRequest {
            task: "should fail".to_string(),
            plan: vec![],
            agent: None,
        }))
        .await;

    assert!(start_result.is_err());
    assert_eq!(
        start_result.unwrap_err().code(),
        tonic::Code::FailedPrecondition
    );
}

/// E2E test: Cannot record action on completed episode.
#[tokio::test]
async fn test_record_action_on_completed_episode() {
    let harness = TestHarness::new();
    let config = EpisodicConfig {
        enabled: true,
        ..Default::default()
    };
    let service = create_episodic_service(&harness, config);

    let start_resp = service
        .start_episode(Request::new(StartEpisodeRequest {
            task: "test task".to_string(),
            plan: vec![],
            agent: None,
        }))
        .await
        .unwrap()
        .into_inner();

    // Complete it
    service
        .complete_episode(Request::new(CompleteEpisodeRequest {
            episode_id: start_resp.episode_id.clone(),
            outcome_score: 0.5,
            failed: false,
            lessons_learned: vec![],
            failure_modes: vec![],
        }))
        .await
        .unwrap();

    // Try to record action on completed episode
    let result = service
        .record_action(Request::new(RecordActionRequest {
            episode_id: start_resp.episode_id,
            action: Some(EpisodeAction {
                action_type: "tool_call".to_string(),
                input: "should fail".to_string(),
                result_status: ActionResultStatus::ActionResultSuccess.into(),
                result_detail: "ok".to_string(),
                timestamp_ms: 0,
            }),
        }))
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::FailedPrecondition);
}

/// E2E test: Failed episode has correct status.
#[tokio::test]
async fn test_episode_failure_status() {
    let harness = TestHarness::new();
    let config = EpisodicConfig {
        enabled: true,
        ..Default::default()
    };
    let service = create_episodic_service(&harness, config);

    let start_resp = service
        .start_episode(Request::new(StartEpisodeRequest {
            task: "risky operation".to_string(),
            plan: vec![],
            agent: Some("opencode".to_string()),
        }))
        .await
        .unwrap()
        .into_inner();

    let complete_resp = service
        .complete_episode(Request::new(CompleteEpisodeRequest {
            episode_id: start_resp.episode_id.clone(),
            outcome_score: 0.15,
            failed: true,
            lessons_learned: vec!["Need better error handling".to_string()],
            failure_modes: vec!["Unhandled null pointer".to_string()],
        }))
        .await
        .unwrap()
        .into_inner();

    assert!(complete_resp.completed);

    let stored = harness
        .storage
        .get_episode(&start_resp.episode_id)
        .unwrap()
        .expect("Episode should exist");

    assert_eq!(stored.status, memory_types::EpisodeStatus::Failed);
    assert_eq!(stored.agent, Some("opencode".to_string()));
}
