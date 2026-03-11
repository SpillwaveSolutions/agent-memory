//! Episodic memory types for recording agent task episodes.
//!
//! Episodes capture complete task execution sequences including:
//! - The task goal and plan
//! - Individual actions taken and their results
//! - Outcome scoring and lessons learned
//! - Value scoring for retrieval prioritization

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of an episode's execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeStatus {
    /// Episode is currently being executed.
    InProgress,
    /// Episode completed successfully.
    Completed,
    /// Episode failed during execution.
    Failed,
}

/// Result of an individual action within an episode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status", content = "detail")]
pub enum ActionResult {
    /// Action completed successfully with output.
    Success(String),
    /// Action failed with error description.
    Failure(String),
    /// Action is still pending completion.
    Pending,
}

/// A single action taken during an episode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Type of action performed (e.g., "tool_call", "api_request", "file_edit").
    pub action_type: String,

    /// Input or parameters for the action.
    pub input: String,

    /// Result of the action.
    pub result: ActionResult,

    /// When the action was performed.
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

/// A complete episode recording a task execution sequence.
///
/// Episodes are the core unit of episodic memory. They capture what the agent
/// did, whether it worked, and what was learned. Value scoring determines
/// retrieval priority -- episodes near the midpoint (neither trivial nor
/// catastrophic) are most valuable for future learning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Unique identifier (ULID string).
    pub episode_id: String,

    /// The task or goal being executed.
    pub task: String,

    /// Planned steps for the task.
    #[serde(default)]
    pub plan: Vec<String>,

    /// Actions taken during execution.
    #[serde(default)]
    pub actions: Vec<Action>,

    /// Current status of the episode.
    pub status: EpisodeStatus,

    /// Outcome score (0.0 = total failure, 1.0 = perfect success).
    #[serde(default)]
    pub outcome_score: Option<f32>,

    /// Lessons learned from the episode.
    #[serde(default)]
    pub lessons_learned: Vec<String>,

    /// Failure modes encountered.
    #[serde(default)]
    pub failure_modes: Vec<String>,

    /// Embedding vector for semantic search.
    #[serde(default)]
    pub embedding: Option<Vec<f32>>,

    /// Value score for retrieval prioritization.
    /// Computed from outcome_score using midpoint-distance formula.
    #[serde(default)]
    pub value_score: Option<f32>,

    /// When the episode was created.
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,

    /// When the episode was completed (if finished).
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,

    /// Agent that executed the episode.
    #[serde(default)]
    pub agent: Option<String>,
}

impl Episode {
    /// Create a new in-progress episode.
    pub fn new(episode_id: String, task: String) -> Self {
        Self {
            episode_id,
            task,
            plan: Vec::new(),
            actions: Vec::new(),
            status: EpisodeStatus::InProgress,
            outcome_score: None,
            lessons_learned: Vec::new(),
            failure_modes: Vec::new(),
            embedding: None,
            value_score: None,
            created_at: Utc::now(),
            completed_at: None,
            agent: None,
        }
    }

    /// Set the plan steps.
    pub fn with_plan(mut self, plan: Vec<String>) -> Self {
        self.plan = plan;
        self
    }

    /// Set the agent identifier.
    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    /// Add an action to the episode.
    pub fn add_action(&mut self, action: Action) {
        self.actions.push(action);
    }

    /// Calculate value score from an outcome score.
    ///
    /// Formula: `(1.0 - (outcome_score - midpoint).abs()).max(0.0)`
    ///
    /// Episodes near the midpoint are most valuable for learning:
    /// - Trivial successes (score near 1.0) teach little
    /// - Catastrophic failures (score near 0.0) may be outliers
    /// - Moderate outcomes (near midpoint) are most informative
    pub fn calculate_value_score(outcome_score: f32, midpoint: f32) -> f32 {
        (1.0 - (outcome_score - midpoint).abs()).max(0.0)
    }

    /// Complete the episode with an outcome score, computing the value score.
    pub fn complete(&mut self, outcome_score: f32, midpoint: f32) {
        self.status = EpisodeStatus::Completed;
        self.outcome_score = Some(outcome_score);
        self.value_score = Some(Self::calculate_value_score(outcome_score, midpoint));
        self.completed_at = Some(Utc::now());
    }

    /// Mark the episode as failed with an outcome score.
    pub fn fail(&mut self, outcome_score: f32, midpoint: f32) {
        self.status = EpisodeStatus::Failed;
        self.outcome_score = Some(outcome_score);
        self.value_score = Some(Self::calculate_value_score(outcome_score, midpoint));
        self.completed_at = Some(Utc::now());
    }

    /// Serialize episode to JSON bytes for storage.
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize episode from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episode_serialization_roundtrip() {
        let mut episode = Episode::new("01TEST".to_string(), "Build auth system".to_string())
            .with_plan(vec!["Design schema".to_string(), "Implement JWT".to_string()])
            .with_agent("claude");

        episode.add_action(Action {
            action_type: "tool_call".to_string(),
            input: "read auth.rs".to_string(),
            result: ActionResult::Success("file contents".to_string()),
            timestamp: Utc::now(),
        });

        let bytes = episode.to_bytes().unwrap();
        let decoded = Episode::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.episode_id, "01TEST");
        assert_eq!(decoded.task, "Build auth system");
        assert_eq!(decoded.plan.len(), 2);
        assert_eq!(decoded.actions.len(), 1);
        assert_eq!(decoded.status, EpisodeStatus::InProgress);
        assert_eq!(decoded.agent, Some("claude".to_string()));
    }

    #[test]
    fn test_episode_backward_compat_no_optional_fields() {
        let json = r#"{
            "episode_id": "01TEST",
            "task": "test task",
            "status": "in_progress",
            "created_at": 1704067200000
        }"#;

        let episode: Episode = serde_json::from_str(json).unwrap();
        assert_eq!(episode.episode_id, "01TEST");
        assert!(episode.plan.is_empty());
        assert!(episode.actions.is_empty());
        assert!(episode.outcome_score.is_none());
        assert!(episode.agent.is_none());
    }

    #[test]
    fn test_episode_complete() {
        let mut episode = Episode::new("01TEST".to_string(), "task".to_string());
        episode.complete(0.65, 0.65);

        assert_eq!(episode.status, EpisodeStatus::Completed);
        assert!(episode.completed_at.is_some());
        // At midpoint, value score should be 1.0
        assert!((episode.value_score.unwrap() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_episode_fail() {
        let mut episode = Episode::new("01TEST".to_string(), "task".to_string());
        episode.fail(0.0, 0.65);

        assert_eq!(episode.status, EpisodeStatus::Failed);
        assert!(episode.completed_at.is_some());
        // Far from midpoint, value score should be 1.0 - 0.65 = 0.35
        assert!((episode.value_score.unwrap() - 0.35).abs() < f32::EPSILON);
    }

    #[test]
    fn test_action_result_serialization() {
        let success = ActionResult::Success("done".to_string());
        let failure = ActionResult::Failure("error".to_string());
        let pending = ActionResult::Pending;

        let s_json = serde_json::to_string(&success).unwrap();
        let f_json = serde_json::to_string(&failure).unwrap();
        let p_json = serde_json::to_string(&pending).unwrap();

        let s_decoded: ActionResult = serde_json::from_str(&s_json).unwrap();
        let f_decoded: ActionResult = serde_json::from_str(&f_json).unwrap();
        let p_decoded: ActionResult = serde_json::from_str(&p_json).unwrap();

        assert_eq!(s_decoded, ActionResult::Success("done".to_string()));
        assert_eq!(f_decoded, ActionResult::Failure("error".to_string()));
        assert_eq!(p_decoded, ActionResult::Pending);
    }

    #[test]
    fn test_calculate_value_score_at_midpoint() {
        // At midpoint: distance = 0, value = 1.0
        let score = Episode::calculate_value_score(0.65, 0.65);
        assert!((score - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_calculate_value_score_perfect_success() {
        // Perfect success far from midpoint
        let score = Episode::calculate_value_score(1.0, 0.65);
        assert!((score - 0.65).abs() < f32::EPSILON);
    }

    #[test]
    fn test_calculate_value_score_total_failure() {
        // Total failure far from midpoint
        let score = Episode::calculate_value_score(0.0, 0.65);
        assert!((score - 0.35).abs() < f32::EPSILON);
    }

    #[test]
    fn test_calculate_value_score_clamps_to_zero() {
        // Edge case: outcome very far from midpoint with high midpoint
        // outcome=0.0, midpoint=0.0 => distance=0 => value=1.0
        let score = Episode::calculate_value_score(0.0, 0.0);
        assert!((score - 1.0).abs() < f32::EPSILON);

        // outcome=2.0 (out of range), midpoint=0.5 => distance=1.5 => value=max(1.0-1.5, 0) = 0
        let score = Episode::calculate_value_score(2.0, 0.5);
        assert!((score - 0.0).abs() < f32::EPSILON);
    }
}
