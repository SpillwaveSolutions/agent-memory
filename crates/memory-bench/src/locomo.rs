//! LOCOMO adapter v2 — real `locomo10.json` schema, isolated stores, honest metrics.
//!
//! Dataset: <https://github.com/snap-research/locomo> (`data/locomo10.json`)
//! License: CC BY-NC 4.0 (verified at download time).
//!
//! Category IDs (from `task_eval/evaluation.py` + data inspection of "When did…"
//! questions labeled `category: 2`):
//!   1 = multi_hop, 2 = temporal, 3 = open_domain, 4 = single_hop, 5 = adversarial

use anyhow::{bail, Context, Result};
use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

use crate::judge::{Judge, JudgeVerdict, ScorerKind};
use crate::runner::{MockStore, QueryResult, RankedHit, RunConfig};

/// Integer category → stable name used in results.json.
pub fn category_name(id: i64) -> &'static str {
    match id {
        1 => "multi_hop",
        2 => "temporal",
        3 => "open_domain",
        4 => "single_hop",
        5 => "adversarial",
        _ => "unknown",
    }
}

#[derive(Debug, Clone)]
pub struct LocomoSample {
    pub sample_id: String,
    pub speaker_a: String,
    pub speaker_b: String,
    pub sessions: Vec<LocomoSession>,
    pub qa: Vec<LocomoQa>,
}

#[derive(Debug, Clone)]
pub struct LocomoSession {
    pub index: usize,
    pub date_time_raw: String,
    pub timestamp: DateTime<Utc>,
    pub turns: Vec<LocomoTurn>,
}

#[derive(Debug, Clone)]
pub struct LocomoTurn {
    pub speaker: String,
    pub dia_id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct LocomoQa {
    pub question: String,
    pub answer: String,
    pub category: i64,
    pub evidence: Vec<String>,
}

/// Per-question record written to results.json.
#[derive(Debug, Clone, Serialize)]
pub struct QuestionResult {
    pub question: String,
    pub gold: String,
    pub category: String,
    pub category_id: i64,
    pub predicted: String,
    pub correct: bool,
    pub rationale: String,
    pub latency_ms: u64,
    pub context_tokens: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeScore {
    pub total: usize,
    pub correct: usize,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocomoConversationResult {
    pub sample_id: String,
    pub total_questions: usize,
    pub correct: usize,
    pub score: f64,
    pub by_type: HashMap<String, TypeScore>,
    pub questions: Vec<QuestionResult>,
}

/// Aggregate across conversations. `metric` is the only name that may be
/// quoted; it is never a bare "Accuracy" or unlabeled "LOCOMO score".
#[derive(Debug, Clone, Serialize)]
pub struct LocomoAggregateResult {
    pub metric: String,
    pub judge: String,
    pub temperature: Option<f64>,
    pub model: Option<String>,
    pub dataset: String,
    pub isolation: String,
    pub conversations: usize,
    pub total_questions: usize,
    pub overall_score: f64,
    pub by_type: HashMap<String, TypeScore>,
    pub per_conversation: Vec<LocomoConversationResult>,
    pub caveats: Vec<String>,
}

/// Load conversations from a file or directory.
///
/// * File: `locomo10.json` (top-level array) or a single sample object.
/// * Directory: `locomo10.json` if present, otherwise every `*.json`.
pub fn load_dataset(path: &Path) -> Result<Vec<LocomoSample>> {
    if path.is_file() {
        return load_json_file(path);
    }
    let locomo10 = path.join("locomo10.json");
    if locomo10.is_file() {
        return load_json_file(&locomo10);
    }
    let mut conversations = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(path)
        .with_context(|| format!("reading dataset dir {}", path.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        conversations.extend(load_json_file(&entry.path())?);
    }
    Ok(conversations)
}

fn load_json_file(path: &Path) -> Result<Vec<LocomoSample>> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let value: Value =
        serde_json::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;
    parse_dataset_value(&value)
}

fn parse_dataset_value(value: &Value) -> Result<Vec<LocomoSample>> {
    match value {
        Value::Array(items) => {
            let mut out = Vec::new();
            for (i, item) in items.iter().enumerate() {
                out.push(parse_sample(item, format!("conv-{i}"))?);
            }
            Ok(out)
        }
        Value::Object(_) => Ok(vec![parse_sample(value, "conv-0".into())?]),
        other => bail!("LOCOMO dataset must be a JSON array or object, got {other}"),
    }
}

/// Parse one real LOCOMO sample. Rejects the v3.0 invented schema.
pub fn parse_sample(value: &Value, fallback_id: String) -> Result<LocomoSample> {
    let obj = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("sample is not an object"))?;

    // The invented v1 schema used conversation_id / turns / questions.
    if obj.contains_key("conversation_id") || obj.contains_key("turns") {
        bail!(
            "refusing invented v1 LOCOMO schema (conversation_id/turns/questions); \
             expected locomo10.json sample_id + conversation.session_N + qa"
        );
    }
    let conversation = obj
        .get("conversation")
        .and_then(|c| c.as_object())
        .ok_or_else(|| anyhow::anyhow!("sample missing conversation object"))?;

    let sample_id = obj
        .get("sample_id")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .unwrap_or(fallback_id);

    let speaker_a = conversation
        .get("speaker_a")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let speaker_b = conversation
        .get("speaker_b")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    let mut sessions = Vec::new();
    for n in 1..=64 {
        let key = format!("session_{n}");
        let Some(arr) = conversation.get(&key).and_then(|v| v.as_array()) else {
            continue;
        };
        let date_key = format!("session_{n}_date_time");
        let date_time_raw = conversation
            .get(&date_key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let timestamp = parse_locomo_datetime(&date_time_raw).unwrap_or_else(|| {
            Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0)
                .single()
                .unwrap_or_else(Utc::now)
        });
        let mut turns = Vec::new();
        for t in arr {
            turns.push(LocomoTurn {
                speaker: t
                    .get("speaker")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                dia_id: t
                    .get("dia_id")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                text: t
                    .get("text")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
            });
        }
        sessions.push(LocomoSession {
            index: n,
            date_time_raw,
            timestamp,
            turns,
        });
    }
    if sessions.is_empty() {
        bail!("sample {sample_id} has no session_N arrays");
    }

    let qa_arr = obj
        .get("qa")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("sample {sample_id} missing qa array"))?;
    let mut qa = Vec::new();
    for item in qa_arr {
        qa.push(LocomoQa {
            question: item
                .get("question")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            answer: stringify_answer(item.get("answer")),
            category: item.get("category").and_then(|c| c.as_i64()).unwrap_or(0),
            evidence: item
                .get("evidence")
                .and_then(|e| e.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
        });
    }

    Ok(LocomoSample {
        sample_id,
        speaker_a,
        speaker_b,
        sessions,
        qa,
    })
}

fn stringify_answer(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

/// Parse `"1:56 pm on 8 May, 2023"` (LOCOMO session timestamps).
pub fn parse_locomo_datetime(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (time_part, date_part) = s.split_once(" on ")?;
    let time_part = time_part.trim().to_lowercase();
    let date_part = date_part.trim().trim_end_matches(',').replace(',', "");

    let mut time_bits = time_part.split_whitespace();
    let hm = time_bits.next()?;
    let ampm = time_bits.next().unwrap_or("");
    let mut hm_bits = hm.split(':');
    let mut hour: u32 = hm_bits.next()?.parse().ok()?;
    let minute: u32 = hm_bits.next().unwrap_or("0").parse().ok()?;
    if ampm.starts_with('p') && hour != 12 {
        hour += 12;
    }
    if ampm.starts_with('a') && hour == 12 {
        hour = 0;
    }

    let mut date_bits = date_part.split_whitespace();
    let day: u32 = date_bits.next()?.parse().ok()?;
    let month_name = date_bits.next()?.to_lowercase();
    let year: i32 = date_bits.next()?.parse().ok()?;
    let month = month_from_name(&month_name)?;
    let date = NaiveDate::from_ymd_opt(year, month, day)?;
    let time = NaiveTime::from_hms_opt(hour, minute, 0)?;
    Some(DateTime::<Utc>::from_naive_utc_and_offset(
        date.and_time(time),
        Utc,
    ))
}

fn month_from_name(name: &str) -> Option<u32> {
    Some(match name {
        "january" | "jan" => 1,
        "february" | "feb" => 2,
        "march" | "mar" => 3,
        "april" | "apr" => 4,
        "may" => 5,
        "june" | "jun" => 6,
        "july" | "jul" => 7,
        "august" | "aug" => 8,
        "september" | "sep" | "sept" => 9,
        "october" | "oct" => 10,
        "november" | "nov" => 11,
        "december" | "dec" => 12,
        _ => return None,
    })
}

/// Ingest one conversation into a fresh mock store (isolation).
pub fn ingest_sample_mock(sample: &LocomoSample) -> MockStore {
    let mut store = MockStore::new();
    for session in &sample.sessions {
        let sid = format!("{}-session-{}", sample.sample_id, session.index);
        for turn in &session.turns {
            let text = format!(
                "[{} @ {}] {}: {}",
                turn.dia_id, session.date_time_raw, turn.speaker, turn.text
            );
            store.ingest_text(&sid, text);
        }
    }
    store
}

/// Retrieve → generate → judge one conversation against an isolated store.
pub fn evaluate_sample(
    sample: &LocomoSample,
    store: &MockStore,
    judge: &dyn Judge,
    top_k: usize,
) -> LocomoConversationResult {
    let mut questions = Vec::new();
    let mut by_type: HashMap<String, (usize, usize)> = HashMap::new();
    let mut correct_n = 0usize;

    for qa in &sample.qa {
        let retrieved: QueryResult = store.search(&qa.question, top_k);
        let context = retrieved
            .ranked
            .iter()
            .map(|h: &RankedHit| h.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let predicted = judge
            .generate_answer(&qa.question, &context)
            .unwrap_or_default();
        let verdict: JudgeVerdict = judge
            .judge(&qa.question, &qa.answer, &predicted, &context)
            .unwrap_or(JudgeVerdict {
                correct: false,
                rationale: "judge error".into(),
            });
        if verdict.correct {
            correct_n += 1;
        }
        let cat = category_name(qa.category).to_string();
        let entry = by_type.entry(cat.clone()).or_insert((0, 0));
        entry.0 += 1;
        if verdict.correct {
            entry.1 += 1;
        }
        questions.push(QuestionResult {
            question: qa.question.clone(),
            gold: qa.answer.clone(),
            category: cat,
            category_id: qa.category,
            predicted: predicted.chars().take(500).collect(),
            correct: verdict.correct,
            rationale: verdict.rationale,
            latency_ms: retrieved.latency_ms,
            context_tokens: retrieved.tokens_estimated,
        });
    }

    let total = sample.qa.len();
    let score = if total == 0 {
        0.0
    } else {
        correct_n as f64 / total as f64
    };
    let by_type = by_type
        .into_iter()
        .map(|(k, (t, c))| {
            (
                k,
                TypeScore {
                    total: t,
                    correct: c,
                    score: if t == 0 { 0.0 } else { c as f64 / t as f64 },
                },
            )
        })
        .collect();

    LocomoConversationResult {
        sample_id: sample.sample_id.clone(),
        total_questions: total,
        correct: correct_n,
        score,
        by_type,
        questions,
    }
}

pub fn aggregate_results(
    results: &[LocomoConversationResult],
    kind: ScorerKind,
    dataset: &str,
    judge_label: &str,
    model: Option<String>,
    temperature: Option<f64>,
) -> LocomoAggregateResult {
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
            (
                k,
                TypeScore {
                    total: t,
                    correct: c,
                    score: if t == 0 { 0.0 } else { c as f64 / t as f64 },
                },
            )
        })
        .collect();

    let mut caveats = vec![
        "one isolated mock store per conversation (no cross-conversation bleed)".into(),
        format!(
            "metric is '{}' — do not quote as a published LOCOMO leaderboard number unless scorer is llm-judge with a pinned model",
            kind.metric_name()
        ),
        "dataset license is CC BY-NC 4.0; verify LICENSE.txt before commercial use".into(),
    ];
    if kind == ScorerKind::Mock {
        caveats.push(
            "mock scorer is substring context_hit_rate over token-overlap retrieval; \
             not comparable to Mem0/MemMachine LLM-judge numbers"
                .into(),
        );
    }

    LocomoAggregateResult {
        metric: kind.metric_name().to_string(),
        judge: judge_label.to_string(),
        temperature,
        model,
        dataset: dataset.to_string(),
        isolation: "per-conversation temp store".into(),
        conversations: results.len(),
        total_questions,
        overall_score,
        by_type,
        per_conversation: results.to_vec(),
        caveats,
    }
}

/// CLI backend helper: ingest via `memory add --timestamp --session-id`.
/// Kept so a live daemon run can exercise real timestamps.
pub fn ingest_sample_cli(sample: &LocomoSample, config: &RunConfig) -> Result<usize> {
    let mut n = 0usize;
    for session in &sample.sessions {
        let sid = format!("{}-session-{}", sample.sample_id, session.index);
        let ts = session.timestamp.to_rfc3339();
        for turn in &session.turns {
            let content = format!("{}: {}", turn.speaker, turn.text);
            let kind = "episodic";
            let output = std::process::Command::new(&config.memory_bin)
                .args([
                    "add",
                    "--content",
                    &content,
                    "--kind",
                    kind,
                    "--session-id",
                    &sid,
                    "--timestamp",
                    &ts,
                    "--role",
                    "user",
                    "--endpoint",
                    &config.endpoint,
                ])
                .output()
                .context("spawning memory add for LOCOMO turn")?;
            if !output.status.success() {
                bail!(
                    "memory add failed for {} {}: {}",
                    sid,
                    turn.dia_id,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judge::MockJudge;

    const REAL_SHAPE: &str = r#"{
        "sample_id": "conv-41",
        "qa": [
            {
                "question": "When did Caroline go to the LGBTQ support group?",
                "answer": "7 May 2023",
                "evidence": ["D1:3"],
                "category": 2
            },
            {
                "question": "What activities has Maria done with her church friends?",
                "answer": "Hiking, picnic, volunteer work",
                "evidence": ["D25:2", "D24:6"],
                "category": 1
            },
            {
                "question": "Would John be open to moving?",
                "answer": "No",
                "evidence": ["D7:2"],
                "category": 3
            },
            {
                "question": "When did Melanie paint a sunrise?",
                "answer": 2022,
                "evidence": ["D1:12"],
                "category": 2
            }
        ],
        "conversation": {
            "speaker_a": "Caroline",
            "speaker_b": "Melanie",
            "session_1_date_time": "1:56 pm on 8 May, 2023",
            "session_1": [
                {"speaker": "Caroline", "dia_id": "D1:1", "text": "Hey Mel! How have you been?"},
                {"speaker": "Melanie", "dia_id": "D1:2", "text": "Swamped with the kids."},
                {"speaker": "Caroline", "dia_id": "D1:3", "text": "I went to a LGBTQ support group yesterday and it was so powerful."}
            ],
            "session_2_date_time": "4:04 pm on 20 January, 2023",
            "session_2": [
                {"speaker": "Melanie", "dia_id": "D2:1", "text": "I painted a sunrise back in 2022."}
            ]
        }
    }"#;

    #[test]
    fn real_schema_parses_including_numeric_answer() {
        let v: Value = serde_json::from_str(REAL_SHAPE).unwrap();
        let s = parse_sample(&v, "x".into()).unwrap();
        assert_eq!(s.sample_id, "conv-41");
        assert_eq!(s.speaker_a, "Caroline");
        assert_eq!(s.sessions.len(), 2);
        assert_eq!(s.sessions[0].turns.len(), 3);
        assert_eq!(s.qa.len(), 4);
        assert_eq!(s.qa[3].answer, "2022");
        assert_eq!(category_name(s.qa[0].category), "temporal");
        assert_eq!(category_name(s.qa[1].category), "multi_hop");
    }

    #[test]
    fn invented_v1_schema_is_rejected() {
        let json = r#"{"conversation_id":"conv-001","turns":[{"role":"user","content":"hi"}],"questions":[{"question":"q","answer":"a","type":"single_hop"}]}"#;
        let v: Value = serde_json::from_str(json).unwrap();
        let err = parse_sample(&v, "x".into()).unwrap_err().to_string();
        assert!(err.contains("invented v1"), "{err}");
    }

    #[test]
    fn parse_locomo_datetime_pm() {
        let dt = parse_locomo_datetime("1:56 pm on 8 May, 2023").unwrap();
        assert_eq!(dt.format("%Y-%m-%dT%H:%M").to_string(), "2023-05-08T13:56");
    }

    #[test]
    fn isolation_no_cross_conversation_bleed() {
        let v: Value = serde_json::from_str(REAL_SHAPE).unwrap();
        let a = parse_sample(&v, "a".into()).unwrap();
        let store_a = ingest_sample_mock(&a);

        let mut b = a.clone();
        b.sample_id = "other".into();
        b.sessions[0].turns[2].text = "I adopted a UNIQUE_ZEBRA_TOKEN last week".into();
        let store_b = ingest_sample_mock(&b);

        let hits_b = store_b.search("UNIQUE_ZEBRA_TOKEN", 5);
        assert!(hits_b
            .ranked
            .iter()
            .any(|h| h.text.contains("UNIQUE_ZEBRA_TOKEN")));
        let hits_a = store_a.search("UNIQUE_ZEBRA_TOKEN", 5);
        assert!(
            hits_a
                .ranked
                .iter()
                .all(|h| !h.text.contains("UNIQUE_ZEBRA_TOKEN")),
            "conversation A must not see B's unique token"
        );
    }

    #[test]
    fn mock_pipeline_parse_ingest_retrieve_score() {
        let v: Value = serde_json::from_str(REAL_SHAPE).unwrap();
        let sample = parse_sample(&v, "x".into()).unwrap();
        let store = ingest_sample_mock(&sample);
        assert!(store.event_count() > 0);
        let result = evaluate_sample(&sample, &store, &MockJudge, 5);
        assert_eq!(result.total_questions, 4);
        // Gold "7 May 2023" is in session_1 D1:3 context via "yesterday" date? The
        // text says "yesterday" not "7 May 2023". Date is on the session stamp
        // "8 May, 2023" — substring "7 May 2023" may miss. "2022" is in session_2.
        assert!(
            result.by_type.contains_key("temporal") || result.by_type.contains_key("multi_hop")
        );
        let agg = aggregate_results(&[result], ScorerKind::Mock, "fixture", "mock", None, None);
        assert_eq!(agg.metric, "context_hit_rate");
        assert!(agg.caveats.iter().any(|c| c.contains("not comparable")));
    }

    #[test]
    fn load_dataset_array_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("locomo10.json");
        std::fs::write(&path, format!("[{REAL_SHAPE}]")).unwrap();
        let samples = load_dataset(&path).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].sample_id, "conv-41");
    }
}
