//! LOCOMO dataset adapter for benchmark evaluation.
//!
//! Loads conversations from the Snap Research LOCOMO dataset format,
//! scores answers against gold-standard questions, and aggregates
//! results with per-question-type breakdowns (single_hop, multi_hop,
//! temporal, open_domain).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

/// A single LOCOMO conversation containing turns and evaluation questions.
#[derive(Debug, Deserialize, Clone)]
pub struct LocomoConversation {
    /// Unique identifier for the conversation.
    pub conversation_id: String,
    /// Ordered dialogue turns.
    pub turns: Vec<Turn>,
    /// Gold-standard questions for evaluation.
    pub questions: Vec<Question>,
}

/// A single dialogue turn in a LOCOMO conversation.
#[derive(Debug, Deserialize, Clone)]
pub struct Turn {
    /// Speaker role (e.g., "user", "assistant").
    pub role: String,
    /// Text content of the turn.
    pub content: String,
}

/// A gold-standard evaluation question with expected answer and type.
#[derive(Debug, Deserialize, Clone)]
pub struct Question {
    /// The question text.
    pub question: String,
    /// The expected gold-standard answer.
    pub answer: String,
    /// Question type: single_hop, multi_hop, temporal, or open_domain.
    #[serde(rename = "type")]
    pub question_type: String,
}

/// Evaluation result for a single conversation.
#[derive(Debug, Serialize, Clone)]
pub struct LocomoResult {
    /// Conversation identifier.
    pub conversation_id: String,
    /// Total number of questions evaluated.
    pub total_questions: usize,
    /// Number of correct answers.
    pub correct: usize,
    /// Overall score (correct / total).
    pub score: f64,
    /// Scores broken down by question type.
    pub by_type: HashMap<String, TypeScore>,
}

/// Score for a specific question type.
#[derive(Debug, Serialize, Clone)]
pub struct TypeScore {
    /// Total questions of this type.
    pub total: usize,
    /// Correct answers of this type.
    pub correct: usize,
    /// Score for this type (correct / total).
    pub score: f64,
}

/// Aggregate result across all conversations.
#[derive(Debug, Serialize)]
pub struct LocomoAggregateResult {
    /// Number of conversations evaluated.
    pub conversations: usize,
    /// Total questions across all conversations.
    pub total_questions: usize,
    /// Overall score across all conversations.
    pub overall_score: f64,
    /// Aggregated scores by question type.
    pub by_type: HashMap<String, TypeScore>,
    /// Per-conversation results.
    pub per_conversation: Vec<LocomoResult>,
}

/// Load all LOCOMO conversations from a dataset directory.
///
/// Reads all `.json` files in the directory and deserializes them
/// into `LocomoConversation` structs.
pub fn load_dataset(dir: &Path) -> Result<Vec<LocomoConversation>> {
    let mut conversations = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let content = std::fs::read_to_string(&path)?;
            let conv: LocomoConversation = serde_json::from_str(&content)?;
            conversations.push(conv);
        }
    }
    Ok(conversations)
}

/// Score a single conversation's questions against retrieved answers.
///
/// Uses case-insensitive substring matching: an answer is correct if
/// the retrieved text contains the gold answer (case-insensitive).
pub fn score_conversation(conv: &LocomoConversation, answers: &[String]) -> LocomoResult {
    let mut by_type: HashMap<String, (usize, usize)> = HashMap::new();
    let mut total_correct = 0;

    for (i, q) in conv.questions.iter().enumerate() {
        let answer = answers.get(i).map(|s| s.as_str()).unwrap_or("");
        let is_correct = answer.to_lowercase().contains(&q.answer.to_lowercase());

        if is_correct {
            total_correct += 1;
        }

        let entry = by_type.entry(q.question_type.clone()).or_insert((0, 0));
        entry.0 += 1; // total
        if is_correct {
            entry.1 += 1; // correct
        }
    }

    let total = conv.questions.len();
    let score = if total == 0 {
        0.0
    } else {
        total_correct as f64 / total as f64
    };

    let by_type = by_type
        .into_iter()
        .map(|(k, (t, c))| {
            let s = if t == 0 { 0.0 } else { c as f64 / t as f64 };
            (k, TypeScore { total: t, correct: c, score: s })
        })
        .collect();

    LocomoResult {
        conversation_id: conv.conversation_id.clone(),
        total_questions: total,
        correct: total_correct,
        score,
        by_type,
    }
}

/// Aggregate results across all conversations.
///
/// Computes overall totals, correct counts, and per-type breakdowns.
pub fn aggregate_results(results: &[LocomoResult]) -> LocomoAggregateResult {
    let mut total_questions = 0;
    let mut total_correct = 0;
    let mut by_type: HashMap<String, (usize, usize)> = HashMap::new();

    for r in results {
        total_questions += r.total_questions;
        total_correct += r.correct;
        for (k, ts) in &r.by_type {
            let entry = by_type.entry(k.clone()).or_insert((0, 0));
            entry.0 += ts.total;
            entry.1 += ts.correct;
        }
    }

    let overall_score = if total_questions == 0 {
        0.0
    } else {
        total_correct as f64 / total_questions as f64
    };

    let by_type = by_type
        .into_iter()
        .map(|(k, (t, c))| {
            let s = if t == 0 { 0.0 } else { c as f64 / t as f64 };
            (k, TypeScore { total: t, correct: c, score: s })
        })
        .collect();

    LocomoAggregateResult {
        conversations: results.len(),
        total_questions,
        overall_score,
        by_type,
        per_conversation: results.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locomo_conversation_parses() {
        let json = r#"{
            "conversation_id": "conv-001",
            "turns": [
                {"role": "user", "content": "Hello"},
                {"role": "assistant", "content": "Hi there"}
            ],
            "questions": [
                {"question": "What did the user say?", "answer": "Hello", "type": "single_hop"}
            ]
        }"#;
        let conv: LocomoConversation = serde_json::from_str(json).unwrap();
        assert_eq!(conv.conversation_id, "conv-001");
        assert_eq!(conv.turns.len(), 2);
        assert_eq!(conv.questions.len(), 1);
        assert_eq!(conv.questions[0].question_type, "single_hop");
    }

    #[test]
    fn test_locomo_conversation_multiple_types() {
        let json = r#"{
            "conversation_id": "conv-002",
            "turns": [{"role": "user", "content": "test"}],
            "questions": [
                {"question": "q1", "answer": "a1", "type": "single_hop"},
                {"question": "q2", "answer": "a2", "type": "multi_hop"},
                {"question": "q3", "answer": "a3", "type": "temporal"},
                {"question": "q4", "answer": "a4", "type": "open_domain"}
            ]
        }"#;
        let conv: LocomoConversation = serde_json::from_str(json).unwrap();
        assert_eq!(conv.questions.len(), 4);
        let types: Vec<&str> = conv.questions.iter().map(|q| q.question_type.as_str()).collect();
        assert!(types.contains(&"single_hop"));
        assert!(types.contains(&"multi_hop"));
        assert!(types.contains(&"temporal"));
        assert!(types.contains(&"open_domain"));
    }

    #[test]
    fn test_score_conversation_all_correct() {
        let conv = LocomoConversation {
            conversation_id: "test".to_string(),
            turns: vec![],
            questions: vec![
                Question { question: "q1".into(), answer: "alpha".into(), question_type: "single_hop".into() },
                Question { question: "q2".into(), answer: "beta".into(), question_type: "multi_hop".into() },
            ],
        };
        let answers = vec![
            "The answer is Alpha obviously".to_string(),
            "It was beta all along".to_string(),
        ];
        let result = score_conversation(&conv, &answers);
        assert!((result.score - 1.0).abs() < f64::EPSILON);
        assert_eq!(result.correct, 2);
        assert_eq!(result.total_questions, 2);
    }

    #[test]
    fn test_score_conversation_partial() {
        let conv = LocomoConversation {
            conversation_id: "test".to_string(),
            turns: vec![],
            questions: vec![
                Question { question: "q1".into(), answer: "alpha".into(), question_type: "single_hop".into() },
                Question { question: "q2".into(), answer: "beta".into(), question_type: "single_hop".into() },
                Question { question: "q3".into(), answer: "gamma".into(), question_type: "temporal".into() },
                Question { question: "q4".into(), answer: "delta".into(), question_type: "temporal".into() },
            ],
        };
        let answers = vec![
            "alpha is here".to_string(),
            "no match".to_string(),
            "gamma found".to_string(),
            "wrong answer".to_string(),
        ];
        let result = score_conversation(&conv, &answers);
        assert!((result.score - 0.5).abs() < f64::EPSILON);
        assert_eq!(result.correct, 2);
    }

    #[test]
    fn test_aggregate_results() {
        let r1 = LocomoResult {
            conversation_id: "c1".into(),
            total_questions: 4,
            correct: 3,
            score: 0.75,
            by_type: HashMap::from([
                ("single_hop".into(), TypeScore { total: 2, correct: 2, score: 1.0 }),
                ("temporal".into(), TypeScore { total: 2, correct: 1, score: 0.5 }),
            ]),
        };
        let r2 = LocomoResult {
            conversation_id: "c2".into(),
            total_questions: 2,
            correct: 1,
            score: 0.5,
            by_type: HashMap::from([
                ("single_hop".into(), TypeScore { total: 1, correct: 0, score: 0.0 }),
                ("temporal".into(), TypeScore { total: 1, correct: 1, score: 1.0 }),
            ]),
        };
        let agg = aggregate_results(&[r1, r2]);
        assert_eq!(agg.conversations, 2);
        assert_eq!(agg.total_questions, 6);
        assert_eq!(agg.by_type["single_hop"].total, 3);
        assert_eq!(agg.by_type["single_hop"].correct, 2);
        assert_eq!(agg.by_type["temporal"].total, 3);
        assert_eq!(agg.by_type["temporal"].correct, 2);
        // overall: 4/6
        assert!((agg.overall_score - 4.0 / 6.0).abs() < 0.001);
    }

    #[test]
    fn test_load_dataset_from_dir() {
        let dir = tempfile::tempdir().unwrap();
        let conv1 = r#"{"conversation_id":"c1","turns":[{"role":"user","content":"hi"}],"questions":[{"question":"q","answer":"a","type":"single_hop"}]}"#;
        let conv2 = r#"{"conversation_id":"c2","turns":[{"role":"user","content":"bye"}],"questions":[{"question":"q2","answer":"a2","type":"temporal"}]}"#;
        std::fs::write(dir.path().join("conv1.json"), conv1).unwrap();
        std::fs::write(dir.path().join("conv2.json"), conv2).unwrap();
        // Non-json file should be ignored
        std::fs::write(dir.path().join("readme.txt"), "ignore me").unwrap();

        let convs = load_dataset(dir.path()).unwrap();
        assert_eq!(convs.len(), 2);
    }
}
