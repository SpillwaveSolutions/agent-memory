//! Answer generation + judging for LOCOMO.
//!
//! Substring containment is `context_hit_rate` — never labeled a LOCOMO score.
//! LLM-as-judge is the only path that may be labeled `locomo_llm_judge`.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// How an answer was scored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScorerKind {
    /// Case-insensitive gold-answer substring in retrieved context or predicted text.
    /// Named `context_hit_rate`. Not comparable to published LOCOMO numbers.
    Mock,
    /// Generate an answer then judge it with a pinned LLM at temperature 0.
    LlmJudge,
}

impl ScorerKind {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "mock" | "context_hit_rate" => Ok(Self::Mock),
            "llm-judge" | "llm_judge" => Ok(Self::LlmJudge),
            other => bail!("unknown scorer '{other}' (expected mock|llm-judge)"),
        }
    }

    pub fn metric_name(self) -> &'static str {
        match self {
            Self::Mock => "context_hit_rate",
            Self::LlmJudge => "locomo_llm_judge",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeVerdict {
    pub correct: bool,
    pub rationale: String,
}

pub trait Judge: Send + Sync {
    fn kind(&self) -> ScorerKind;
    fn generate_answer(&self, question: &str, context: &str) -> Result<String>;
    fn judge(
        &self,
        question: &str,
        gold: &str,
        predicted: &str,
        context: &str,
    ) -> Result<JudgeVerdict>;
}

/// Substring judge. Predicted answer is the retrieved context itself.
pub struct MockJudge;

impl Judge for MockJudge {
    fn kind(&self) -> ScorerKind {
        ScorerKind::Mock
    }

    fn generate_answer(&self, _question: &str, context: &str) -> Result<String> {
        Ok(context.to_string())
    }

    fn judge(
        &self,
        _question: &str,
        gold: &str,
        predicted: &str,
        context: &str,
    ) -> Result<JudgeVerdict> {
        let gold_l = gold.to_lowercase();
        let hay_pred = predicted.to_lowercase();
        let hay_ctx = context.to_lowercase();
        let hit = !gold_l.is_empty() && (hay_pred.contains(&gold_l) || hay_ctx.contains(&gold_l));
        Ok(JudgeVerdict {
            correct: hit,
            rationale: if hit {
                "gold substring present in context or predicted text (context_hit_rate)".into()
            } else {
                "gold substring absent (context_hit_rate miss)".into()
            },
        })
    }
}

/// LLM-as-judge using the same OpenAI/Anthropic wire protocols as
/// `memory_toc::summarizer::api::ApiSummarizer`. Temperature is pinned at 0.
pub struct ApiJudge {
    pub model: String,
    pub base_url: String,
    pub api_key: String,
    /// 0.0 — recorded in results.json.
    pub temperature: f64,
}

impl ApiJudge {
    pub fn from_env() -> Result<Self> {
        let model =
            std::env::var("MEMORY_BENCH_JUDGE_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            if !key.is_empty() {
                return Ok(Self {
                    model,
                    base_url: std::env::var("OPENAI_BASE_URL")
                        .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
                    api_key: key,
                    temperature: 0.0,
                });
            }
        }
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            if !key.is_empty() {
                return Ok(Self {
                    model: std::env::var("MEMORY_BENCH_JUDGE_MODEL")
                        .unwrap_or_else(|_| "claude-3-haiku-20240307".into()),
                    base_url: "https://api.anthropic.com/v1".into(),
                    api_key: key,
                    temperature: 0.0,
                });
            }
        }
        bail!(
            "llm-judge requires OPENAI_API_KEY or ANTHROPIC_API_KEY; \
             use --scorer mock for the CI smoke path"
        );
    }

    fn is_anthropic(&self) -> bool {
        self.base_url.contains("anthropic.com")
    }

    fn complete(&self, prompt: &str) -> Result<String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .context("building judge HTTP client")?;
        if self.is_anthropic() {
            let url = format!("{}/messages", self.base_url.trim_end_matches('/'));
            let body = serde_json::json!({
                "model": self.model,
                "max_tokens": 512,
                "temperature": self.temperature,
                "messages": [{"role": "user", "content": prompt}],
            });
            let resp = client
                .post(url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .json(&body)
                .send()
                .context("anthropic judge request")?;
            let status = resp.status();
            let v: serde_json::Value = resp.json().context("anthropic judge json")?;
            if !status.is_success() {
                bail!("anthropic judge HTTP {status}: {v}");
            }
            let text = v
                .get("content")
                .and_then(|c| c.as_array())
                .and_then(|a| a.first())
                .and_then(|m| m.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            return Ok(text);
        }
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.model,
            "temperature": self.temperature,
            "messages": [{"role": "user", "content": prompt}],
        });
        let resp = client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .context("openai judge request")?;
        let status = resp.status();
        let v: serde_json::Value = resp.json().context("openai judge json")?;
        if !status.is_success() {
            bail!("openai judge HTTP {status}: {v}");
        }
        let text = v
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|m| m.get("message")?.get("content"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        Ok(text)
    }
}

const GENERATE_PROMPT: &str = "You answer questions using ONLY the retrieved context. \
If the context is insufficient, say you don't know. Do not invent facts.\n\n\
Context:\n{context}\n\nQuestion: {question}\n\nAnswer:";

const JUDGE_PROMPT: &str = "You are a strict binary grader. Compare the predicted answer to the gold answer.\n\
The predicted answer is correct if it contains the same key facts as the gold answer, allowing for \
paraphrase and extra context. Reply with JSON only: {\"correct\": true|false, \"rationale\": \"...\"}\n\n\
Question: {question}\nGold: {gold}\nPredicted: {predicted}\n";

impl Judge for ApiJudge {
    fn kind(&self) -> ScorerKind {
        ScorerKind::LlmJudge
    }

    fn generate_answer(&self, question: &str, context: &str) -> Result<String> {
        let prompt = GENERATE_PROMPT
            .replace("{context}", context)
            .replace("{question}", question);
        self.complete(&prompt)
    }

    fn judge(
        &self,
        question: &str,
        gold: &str,
        predicted: &str,
        _context: &str,
    ) -> Result<JudgeVerdict> {
        let prompt = JUDGE_PROMPT
            .replace("{question}", question)
            .replace("{gold}", gold)
            .replace("{predicted}", predicted);
        let raw = self.complete(&prompt)?;
        parse_verdict(&raw)
    }
}

fn parse_verdict(raw: &str) -> Result<JudgeVerdict> {
    let start = raw.find('{');
    let end = raw.rfind('}');
    if let (Some(s), Some(e)) = (start, end) {
        if e >= s {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw[s..=e]) {
                let correct = v.get("correct").and_then(|c| c.as_bool()).unwrap_or(false);
                let rationale = v
                    .get("rationale")
                    .and_then(|r| r.as_str())
                    .unwrap_or("")
                    .to_string();
                return Ok(JudgeVerdict { correct, rationale });
            }
        }
    }
    let lower = raw.to_lowercase();
    Ok(JudgeVerdict {
        correct: lower.contains("\"correct\": true") || lower.contains("correct: true"),
        rationale: raw.chars().take(240).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_judge_hits_on_substring() {
        let j = MockJudge;
        let v = j
            .judge("when?", "7 May 2023", "irrelevant", "Went on 7 May 2023")
            .unwrap();
        assert!(v.correct);
        assert!(j.kind().metric_name() == "context_hit_rate");
    }

    #[test]
    fn mock_judge_misses_when_absent() {
        let j = MockJudge;
        let v = j
            .judge("when?", "7 May 2023", "no date here", "also no date")
            .unwrap();
        assert!(!v.correct);
    }

    #[test]
    fn parse_verdict_json() {
        let v = parse_verdict("sure\n{\"correct\": true, \"rationale\": \"matches\"}\n").unwrap();
        assert!(v.correct);
        assert_eq!(v.rationale, "matches");
    }

    #[test]
    fn scorer_kind_never_aliases_substring_as_locomo() {
        assert_eq!(ScorerKind::Mock.metric_name(), "context_hit_rate");
        assert_eq!(ScorerKind::LlmJudge.metric_name(), "locomo_llm_judge");
    }
}
