//! Run queries against either a mock isolated store or the `memory` CLI.
//!
//! Errors from `memory add` / `memory search` fail the run. A dead daemon
//! is not scored as accuracy 0.0.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// Which retrieval backend to drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    /// In-process token-overlap store. Always isolated per test.
    Mock,
    /// Shell out to the `memory` CLI against a running daemon.
    Cli,
}

impl BackendKind {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "mock" => Ok(Self::Mock),
            "cli" => Ok(Self::Cli),
            other => bail!("unknown backend '{other}' (expected mock|cli)"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::Cli => "cli",
        }
    }
}

/// Configuration for the benchmark runner.
pub struct RunConfig {
    /// Path to the memory binary (default: "memory").
    pub memory_bin: String,
    /// gRPC endpoint for CLI backend.
    pub endpoint: String,
    pub backend: BackendKind,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            memory_bin: "memory".to_string(),
            endpoint: "http://127.0.0.1:50051".to_string(),
            backend: BackendKind::Mock,
        }
    }
}

/// One ranked retrieval hit.
#[derive(Debug, Clone)]
pub struct RankedHit {
    pub text: String,
    pub score: f64,
}

/// Result of running a single query.
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Raw stdout (CLI) or synthesized JSON (mock).
    pub raw_output: String,
    pub latency_ms: u64,
    pub tokens_estimated: usize,
    pub ranked: Vec<RankedHit>,
}

/// A single ingested event sitting in a mock store.
#[derive(Debug, Clone)]
struct StoredEvent {
    text: String,
}

/// Isolated in-memory store. One of these per test / per LOCOMO conversation.
#[derive(Debug, Default)]
pub struct MockStore {
    events: Vec<StoredEvent>,
}

impl MockStore {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn ingest_text(&mut self, _session_id: impl Into<String>, text: impl Into<String>) {
        self.events.push(StoredEvent { text: text.into() });
    }

    /// Ingest a JSONL session file. Each line is either a JSON object with
    /// `content` (and optional `role`) or raw text.
    pub fn ingest_file(&mut self, path: &Path) -> Result<usize> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("opening session file {}", path.display()))?;
        let reader = std::io::BufReader::new(file);
        let session_id = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "session".to_string());
        let mut n = 0usize;
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let text = parse_jsonl_line(trimmed);
            if text.is_empty() {
                continue;
            }
            self.ingest_text(&session_id, text);
            n += 1;
        }
        Ok(n)
    }

    /// Rank events by query-term overlap. Isolated: only this store's events.
    pub fn search(&self, query: &str, top_k: usize) -> QueryResult {
        let start = Instant::now();
        let terms = tokenize(query);
        let mut scored: Vec<(f64, &StoredEvent)> = self
            .events
            .iter()
            .map(|e| {
                let hay = e.text.to_lowercase();
                let score = terms.iter().filter(|t| hay.contains(t.as_str())).count() as f64;
                (score, e)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        let ranked: Vec<RankedHit> = scored
            .into_iter()
            .map(|(score, e)| RankedHit {
                text: e.text.clone(),
                score,
            })
            .collect();

        let joined = ranked
            .iter()
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let tokens_estimated = crate::scorer::estimate_tokens_from_text(&joined);
        let results: Vec<Value> = ranked
            .iter()
            .map(|h| {
                serde_json::json!({
                    "text_preview": h.text,
                    "score": h.score,
                })
            })
            .collect();
        let envelope = serde_json::json!({
            "status": "ok",
            "query": query,
            "results": results,
            "meta": {
                "retrieval_ms": start.elapsed().as_millis() as u64,
                "tokens_estimated": tokens_estimated,
                "confidence": ranked.first().map(|h| h.score).unwrap_or(0.0),
                "backend": "mock",
            }
        });

        QueryResult {
            raw_output: envelope.to_string(),
            latency_ms: start.elapsed().as_millis() as u64,
            tokens_estimated,
            ranked,
        }
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

fn tokenize(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 1)
        .map(|s| s.to_lowercase())
        .collect()
}

fn parse_jsonl_line(line: &str) -> String {
    if let Ok(v) = serde_json::from_str::<Value>(line) {
        if let Some(c) = v.get("content").and_then(|x| x.as_str()) {
            let role = v.get("role").and_then(|x| x.as_str()).unwrap_or("");
            if role.is_empty() {
                return c.to_string();
            }
            return format!("{role}: {c}");
        }
        if let Some(t) = v.get("text").and_then(|x| x.as_str()) {
            return t.to_string();
        }
    }
    line.to_string()
}

/// Parse ranked hits out of a `memory search --format=json` envelope.
pub fn parse_ranked_hits(json_output: &str) -> Vec<RankedHit> {
    let Ok(v) = serde_json::from_str::<Value>(json_output) else {
        return Vec::new();
    };
    let Some(results) = v.get("results").and_then(|r| r.as_array()) else {
        return Vec::new();
    };
    results
        .iter()
        .map(|r| RankedHit {
            text: r
                .get("text_preview")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string(),
            score: r.get("score").and_then(|s| s.as_f64()).unwrap_or(0.0),
        })
        .collect()
}

/// Extract meta.tokens_estimated from JSON envelope output.
pub fn extract_tokens_estimated(json_output: &str) -> usize {
    serde_json::from_str::<Value>(json_output)
        .ok()
        .and_then(|v| v.get("meta")?.get("tokens_estimated")?.as_u64())
        .unwrap_or(0) as usize
}

/// Resolve a fixture setup path against the fixtures directory.
pub fn resolve_setup(fixtures_dir: &Path, setup: &str) -> PathBuf {
    let p = Path::new(setup);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    let from_fixtures = fixtures_dir.join(setup);
    if from_fixtures.exists() {
        return from_fixtures;
    }
    p.to_path_buf()
}

/// Ingest a JSONL session file by calling `memory add` for each line.
/// Failures (missing binary, non-zero exit, daemon down) abort the run.
pub fn ingest_session_cli(session_path: &str, config: &RunConfig) -> Result<usize> {
    let file = std::fs::File::open(session_path)
        .with_context(|| format!("opening session file {session_path}"))?;
    let reader = std::io::BufReader::new(file);
    let mut n = 0usize;

    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let content = parse_jsonl_line(trimmed);
        let output = Command::new(&config.memory_bin)
            .args([
                "add",
                "--content",
                &content,
                "--kind",
                "episodic",
                "--endpoint",
                &config.endpoint,
            ])
            .output()
            .with_context(|| {
                format!(
                    "spawning `{} add` (is the memory binary on PATH?)",
                    config.memory_bin
                )
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            bail!(
                "memory add failed on {session_path}:{idx}: status={} stderr={stderr} stdout={stdout}",
                output.status
            );
        }
        n += 1;
    }
    Ok(n)
}

/// Run a search query against the memory CLI. Non-zero exit fails the run.
pub fn run_query_cli(query: &str, config: &RunConfig, top: usize) -> Result<QueryResult> {
    let start = Instant::now();
    let output = Command::new(&config.memory_bin)
        .args([
            "search",
            query,
            "--format",
            "json",
            "--top",
            &top.to_string(),
            "--endpoint",
            &config.endpoint,
        ])
        .output()
        .with_context(|| {
            format!(
                "spawning `{} search` (is the memory binary on PATH?)",
                config.memory_bin
            )
        })?;
    let elapsed = start.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        bail!(
            "memory search failed for query {query:?}: status={} stderr={stderr} stdout={stdout}",
            output.status
        );
    }
    if stdout.trim().is_empty() {
        bail!("memory search returned empty stdout for query {query:?} (daemon down?)");
    }
    let ranked = parse_ranked_hits(&stdout);
    let tokens_estimated = extract_tokens_estimated(&stdout);
    Ok(QueryResult {
        raw_output: stdout,
        latency_ms: elapsed,
        tokens_estimated,
        ranked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_stores_do_not_bleed() {
        let mut a = MockStore::new();
        a.ingest_text("s1", "UNIQUE_ALPHA_TOKEN jwt rotation");
        let mut b = MockStore::new();
        b.ingest_text("s2", "UNIQUE_BETA_TOKEN redis cache");

        let hits = b.search("UNIQUE_ALPHA_TOKEN", 5);
        assert!(
            hits.ranked
                .iter()
                .all(|h| !h.text.contains("UNIQUE_ALPHA_TOKEN")),
            "store B must not see store A's events: {:?}",
            hits.ranked
        );
        let hits_a = a.search("UNIQUE_ALPHA_TOKEN", 5);
        assert!(hits_a
            .ranked
            .iter()
            .any(|h| h.text.contains("UNIQUE_ALPHA_TOKEN")));
    }

    #[test]
    fn mock_search_ranks_term_overlap() {
        let mut s = MockStore::new();
        s.ingest_text("s", "the weather is nice today");
        s.ingest_text("s", "we chose JWT with refresh token rotation for auth");
        s.ingest_text("s", "redis caching hit rate");
        let hits = s.search("JWT refresh rotation", 5);
        assert!(!hits.ranked.is_empty());
        assert!(hits.ranked[0].text.contains("JWT"));
    }

    #[test]
    fn ingest_file_reads_jsonl_content_not_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.jsonl");
        std::fs::write(
            &path,
            r#"{"role":"user","content":"pick JWT please"}
{"role":"assistant","content":"JWT with rotation it is"}
"#,
        )
        .unwrap();
        let mut s = MockStore::new();
        let n = s.ingest_file(&path).unwrap();
        assert_eq!(n, 2);
        let hits = s.search("JWT", 5);
        assert!(hits.ranked.iter().any(|h| h.text.contains("JWT")));
    }

    #[test]
    fn cli_ingest_fails_loud_when_binary_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.jsonl");
        std::fs::write(&path, "{\"content\":\"hi\"}\n").unwrap();
        let cfg = RunConfig {
            memory_bin: "/definitely/not/a/memory/binary".into(),
            endpoint: "http://127.0.0.1:1".into(),
            backend: BackendKind::Cli,
        };
        let err = ingest_session_cli(path.to_str().unwrap(), &cfg).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("spawning") || msg.contains("No such file") || msg.contains("failed"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn committed_fixtures_are_at_least_25() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks/fixtures");
        let tests = crate::fixture::Fixture::load_dir(&dir).unwrap();
        assert!(
            tests.len() >= 25,
            "Phase 56 requires ≥25 custom-harness tests, found {}",
            tests.len()
        );
        assert!(tests.iter().any(|t| t.id.starts_with("temporal-")));
        assert!(tests.iter().any(|t| t.id.starts_with("multi-")));
        assert!(tests.iter().any(|t| t.id.starts_with("compress-")));
        assert!(tests.iter().all(|t| !t.relevant.is_empty()));
    }

    #[test]
    fn run_committed_fixtures_isolated_mock() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks/fixtures");
        let tests = crate::fixture::Fixture::load_dir(&dir).unwrap();
        let mut pass = 0usize;
        let mut recalls = Vec::new();
        for test in &tests {
            let mut store = MockStore::new();
            for setup in &test.setup {
                let path = resolve_setup(&dir, setup);
                store.ingest_file(&path).unwrap();
            }
            let result = store.search(&test.query, test.k.max(5));
            let texts: Vec<String> = result.ranked.iter().map(|h| h.text.clone()).collect();
            if crate::scorer::score_result(&texts.join("\n"), &test.expected_contains) {
                pass += 1;
            }
            if let Some(r) = crate::scorer::compute_recall_at_k(&texts, &test.relevant, test.k) {
                recalls.push(r);
            }
        }
        assert!(pass > 0, "mock retrieval should hit at least one fixture");
        assert_eq!(recalls.len(), tests.len());
        // Isolation: running the suite must not require a daemon and must
        // not panic. Accuracy is a mock number — asserted only as a pipeline.
        assert!(tests.len() >= 25);
    }
}
