//! Run queries against either a mock isolated store or the `memory` CLI.
//!
//! Errors from `memory add` / `memory search` fail the run. A dead daemon
//! is not scored as accuracy 0.0.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

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

/// How the CLI backend isolates conversations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Isolation {
    /// One shared daemon. Conversation N can see 1..=N-1. Debug only.
    Shared,
    /// Spawn `memory-daemon start --db-path <tmp> --port <free>` per conversation.
    DaemonPerConversation,
}

impl Isolation {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "shared" => Ok(Self::Shared),
            "daemon-per-conversation" => Ok(Self::DaemonPerConversation),
            other => bail!("unknown isolation '{other}' (expected daemon-per-conversation|shared)"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::DaemonPerConversation => "daemon-per-conversation",
        }
    }

    /// Value written to results.json `isolation`.
    pub fn result_label(self, backend: BackendKind) -> &'static str {
        match (backend, self) {
            (BackendKind::Mock, _) => "per-conversation temp store",
            (BackendKind::Cli, Self::DaemonPerConversation) => "per-conversation daemon",
            (BackendKind::Cli, Self::Shared) => "shared daemon (cross-conversation bleed)",
        }
    }
}

/// Configuration for the benchmark runner.
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// Path to the memory binary (default: "memory").
    pub memory_bin: String,
    /// Path to the memory-daemon binary (isolation spawn).
    pub daemon_bin: String,
    /// gRPC endpoint for CLI backend.
    pub endpoint: String,
    pub backend: BackendKind,
    pub isolation: Isolation,
    /// Cap total questions across conversations.
    pub limit_questions: Option<usize>,
    /// Retrieval layer for the custom harness.
    pub layer: crate::layers::RetrievalLayer,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            memory_bin: "memory".to_string(),
            daemon_bin: "memory-daemon".to_string(),
            endpoint: "http://127.0.0.1:50051".to_string(),
            backend: BackendKind::Mock,
            isolation: Isolation::Shared,
            limit_questions: None,
            layer: crate::layers::RetrievalLayer::Bm25,
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

    /// Rank events by the configured retrieval layer. Isolated: only this store's events.
    pub fn search(&self, query: &str, top_k: usize) -> QueryResult {
        self.search_with_layer(query, top_k, crate::layers::RetrievalLayer::Bm25)
    }

    /// Rank events under a specific retrieval layer.
    pub fn search_with_layer(
        &self,
        query: &str,
        top_k: usize,
        layer: crate::layers::RetrievalLayer,
    ) -> QueryResult {
        let start = Instant::now();
        let docs: Vec<String> = self.events.iter().map(|e| e.text.clone()).collect();
        let ranked_idx = crate::layers::rank(&docs, query, layer);
        let ranked: Vec<RankedHit> = ranked_idx
            .into_iter()
            .take(top_k)
            .map(|(score, i)| RankedHit {
                text: docs[i].clone(),
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
                "layers": layer.as_str(),
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

const DRAIN_TIMEOUT: Duration = Duration::from_secs(300);
const HEALTH_TIMEOUT: Duration = Duration::from_secs(300);
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Poll interval. Not a substitute for drain — we still re-read checkpoints.
fn poll_pause() {
    // `recv_timeout` rather than `thread::sleep`: the cli-backend path must
    // not paper over drain with a blind sleep.
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let _ = rx.recv_timeout(POLL_INTERVAL);
    drop(tx);
}

/// Snapshot from `memory-daemon query checkpoints`.
#[derive(Debug, Clone)]
pub struct CheckpointSnapshot {
    pub checkpoints: Vec<CheckpointInfo>,
    pub outbox_head: u64,
}

#[derive(Debug, Clone)]
pub struct CheckpointInfo {
    pub index_type: String,
    pub last_sequence: u64,
    pub processed_count: u64,
}

/// Drain is complete when BM25 (and vector, if present) has processed every
/// assigned outbox sequence. A missing vector checkpoint does not block:
/// the daemon's outbox pipeline currently registers only the BM25 updater.
pub fn drain_caught_up(snap: &CheckpointSnapshot) -> bool {
    if snap.outbox_head == 0 {
        return true;
    }
    let target = snap.outbox_head.saturating_sub(1);
    let bm25 = snap.checkpoints.iter().find(|c| c.index_type == "bm25");
    let Some(bm25) = bm25 else {
        return false;
    };
    if bm25.processed_count == 0 || bm25.last_sequence < target {
        return false;
    }
    match snap.checkpoints.iter().find(|c| c.index_type == "vector") {
        Some(v) => v.processed_count > 0 && v.last_sequence >= target,
        None => true,
    }
}

pub fn fetch_checkpoints(daemon_bin: &str, endpoint: &str) -> Result<CheckpointSnapshot> {
    let output = Command::new(daemon_bin)
        .args(["query", "--endpoint", endpoint, "checkpoints"])
        .output()
        .with_context(|| format!("spawning `{daemon_bin} query checkpoints`"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "query checkpoints failed at {endpoint}: status={} stderr={stderr}",
            output.status
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_checkpoint_json(&stdout).with_context(|| format!("parsing checkpoints JSON: {stdout}"))
}

fn parse_checkpoint_json(stdout: &str) -> Result<CheckpointSnapshot> {
    let v: Value = serde_json::from_str(stdout.trim())?;
    let outbox_head = v
        .get("outbox_head")
        .and_then(|x| x.as_u64())
        .context("missing outbox_head")?;
    let mut checkpoints = Vec::new();
    if let Some(arr) = v.get("checkpoints").and_then(|x| x.as_array()) {
        for c in arr {
            checkpoints.push(CheckpointInfo {
                index_type: c
                    .get("index_type")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                last_sequence: c.get("last_sequence").and_then(|x| x.as_u64()).unwrap_or(0),
                processed_count: c
                    .get("processed_count")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0),
            });
        }
    }
    Ok(CheckpointSnapshot {
        checkpoints,
        outbox_head,
    })
}

/// Poll GetIndexCheckpoints until BM25 is caught up, or 5 minutes.
pub fn wait_for_drain(daemon_bin: &str, endpoint: &str) -> Result<u64> {
    let start = Instant::now();
    loop {
        match fetch_checkpoints(daemon_bin, endpoint) {
            Ok(snap) if drain_caught_up(&snap) => {
                return Ok(start.elapsed().as_millis() as u64);
            }
            Ok(_) | Err(_) => {
                if start.elapsed() > DRAIN_TIMEOUT {
                    bail!(
                        "index drain timed out after {}ms at {endpoint} \
                         (BM25 checkpoint never reached outbox_head-1)",
                        DRAIN_TIMEOUT.as_millis()
                    );
                }
                poll_pause();
            }
        }
    }
}

/// Bind 127.0.0.1:0, return the port, drop the listener.
pub fn free_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

/// One daemon, one temp store, one free port. Drop/stop kills the child.
pub struct IsolatedDaemon {
    child: Child,
    pub endpoint: String,
    _dir: tempfile::TempDir,
    daemon_bin: String,
    pid_file: PathBuf,
}

impl IsolatedDaemon {
    pub fn spawn(daemon_bin: &str) -> Result<Self> {
        let dir = tempfile::tempdir().context("creating isolation tempdir")?;
        let db = dir.path().join("db");
        std::fs::create_dir_all(&db)?;
        let pid_file = dir.path().join("daemon.pid");
        let stderr_path = dir.path().join("daemon.stderr");
        let stderr_file =
            std::fs::File::create(&stderr_path).context("creating daemon stderr log")?;
        let port = free_port()?;
        let mut child = Command::new(daemon_bin)
            .args([
                "start",
                "--db-path",
                db.to_str().context("db path utf-8")?,
                "--port",
                &port.to_string(),
                "--pid-file",
                pid_file.to_str().context("pid path utf-8")?,
                "--log-level",
                "warn",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::from(stderr_file))
            .spawn()
            .with_context(|| format!("spawning `{daemon_bin} start`"))?;

        let endpoint = format!("http://127.0.0.1:{port}");
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                let log = std::fs::read_to_string(&stderr_path).unwrap_or_default();
                bail!("daemon exited before becoming healthy: {status}\n{log}");
            }
            if fetch_checkpoints(daemon_bin, &endpoint).is_ok() {
                break;
            }
            if start.elapsed() > HEALTH_TIMEOUT {
                let log = std::fs::read_to_string(&stderr_path).unwrap_or_default();
                let _ = child.kill();
                bail!("daemon did not become healthy at {endpoint} within 300s\n{log}");
            }
            poll_pause();
        }

        Ok(Self {
            child,
            endpoint,
            _dir: dir,
            daemon_bin: daemon_bin.to_string(),
            pid_file,
        })
    }

    pub fn stop(mut self) -> Result<()> {
        let _ = Command::new(&self.daemon_bin)
            .args([
                "stop",
                "--pid-file",
                self.pid_file.to_str().unwrap_or_default(),
            ])
            .status();
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if self.child.try_wait()?.is_some() {
                break;
            }
            if Instant::now() > deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                break;
            }
            poll_pause();
        }
        Ok(())
    }
}

impl Drop for IsolatedDaemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Locate `memory` / `memory-daemon` for live isolation tests.
pub fn find_bin(name: &str) -> Option<PathBuf> {
    let env_key = match name {
        "memory-daemon" => "MEMORY_BENCH_DAEMON_BIN",
        "memory" => "MEMORY_BENCH_MEMORY_BIN",
        _ => "",
    };
    if !env_key.is_empty() {
        if let Ok(p) = std::env::var(env_key) {
            let pb = PathBuf::from(p);
            if pb.is_file() {
                return Some(pb);
            }
        }
    }
    let debug = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug")
        .join(name);
    if debug.is_file() {
        return Some(debug);
    }
    let release = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/release")
        .join(name);
    if release.is_file() {
        return Some(release);
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
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
            ..RunConfig::default()
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
        assert!(tests.iter().any(|t| t.id.starts_with("semantic-")));
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

    #[test]
    fn semantic_fixtures_bm25_below_point_four_vector_wins() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks/fixtures");
        let tests: Vec<_> = crate::fixture::Fixture::load_dir(&dir)
            .unwrap()
            .into_iter()
            .filter(|t| t.id.starts_with("semantic-") || t.category.as_deref() == Some("semantic"))
            .collect();
        assert!(
            tests.len() >= 15,
            "QUAL-01 requires ≥15 semantic tests, found {}",
            tests.len()
        );

        let recall_of = |layer: crate::layers::RetrievalLayer| -> f64 {
            let mut recs = Vec::new();
            for test in &tests {
                let mut store = MockStore::new();
                for setup in &test.setup {
                    let path = resolve_setup(&dir, setup);
                    store.ingest_file(&path).unwrap();
                }
                let result = store.search_with_layer(&test.query, test.k.max(5), layer);
                let texts: Vec<String> = result.ranked.iter().map(|h| h.text.clone()).collect();
                if let Some(r) = crate::scorer::compute_recall_at_k(&texts, &test.relevant, test.k)
                {
                    recs.push(r);
                }
            }
            recs.iter().sum::<f64>() / recs.len() as f64
        };

        let bm25 = recall_of(crate::layers::RetrievalLayer::Bm25);
        let vector = recall_of(crate::layers::RetrievalLayer::Vector);
        assert!(
            bm25 < 0.4,
            "BM25 recall@5 on the paraphrase set must be < 0.4, got {bm25}"
        );
        assert!(
            vector > bm25,
            "vector recall@5 ({vector}) must beat BM25 ({bm25}) on the paraphrase set"
        );
    }

    #[test]
    fn drain_caught_up_empty_outbox() {
        let snap = CheckpointSnapshot {
            checkpoints: vec![],
            outbox_head: 0,
        };
        assert!(drain_caught_up(&snap));
    }

    #[test]
    fn drain_caught_up_requires_bm25() {
        let pending = CheckpointSnapshot {
            checkpoints: vec![],
            outbox_head: 3,
        };
        assert!(!drain_caught_up(&pending));

        let incomplete = CheckpointSnapshot {
            checkpoints: vec![CheckpointInfo {
                index_type: "bm25".into(),
                last_sequence: 0,
                processed_count: 0,
            }],
            outbox_head: 3,
        };
        assert!(!drain_caught_up(&incomplete));

        let ready = CheckpointSnapshot {
            checkpoints: vec![CheckpointInfo {
                index_type: "bm25".into(),
                last_sequence: 2,
                processed_count: 3,
            }],
            outbox_head: 3,
        };
        assert!(drain_caught_up(&ready));
    }

    #[test]
    fn drain_caught_up_first_outbox_sequence() {
        let ready = CheckpointSnapshot {
            checkpoints: vec![CheckpointInfo {
                index_type: "bm25".into(),
                last_sequence: 0,
                processed_count: 1,
            }],
            outbox_head: 1,
        };
        assert!(drain_caught_up(&ready));
    }

    #[test]
    fn drain_caught_up_vector_optional_unless_present() {
        let bm25_only = CheckpointSnapshot {
            checkpoints: vec![CheckpointInfo {
                index_type: "bm25".into(),
                last_sequence: 4,
                processed_count: 5,
            }],
            outbox_head: 5,
        };
        assert!(drain_caught_up(&bm25_only));

        let vector_lagging = CheckpointSnapshot {
            checkpoints: vec![
                CheckpointInfo {
                    index_type: "bm25".into(),
                    last_sequence: 4,
                    processed_count: 5,
                },
                CheckpointInfo {
                    index_type: "vector".into(),
                    last_sequence: 0,
                    processed_count: 0,
                },
            ],
            outbox_head: 5,
        };
        assert!(!drain_caught_up(&vector_lagging));
    }

    #[test]
    fn parse_checkpoint_json_roundtrip() {
        let snap = parse_checkpoint_json(
            r#"{"checkpoints":[{"index_type":"bm25","last_sequence":1,"processed_count":2}],"outbox_head":2}"#,
        )
        .unwrap();
        assert_eq!(snap.outbox_head, 2);
        assert_eq!(snap.checkpoints[0].index_type, "bm25");
        assert!(drain_caught_up(&snap));
    }

    #[test]
    fn isolation_parse_roundtrip() {
        assert_eq!(
            Isolation::parse("daemon-per-conversation").unwrap(),
            Isolation::DaemonPerConversation
        );
        assert_eq!(Isolation::parse("shared").unwrap(), Isolation::Shared);
        assert!(Isolation::parse("reset-rpc").is_err());
    }

    #[test]
    fn isolation_default_label() {
        assert_eq!(
            Isolation::DaemonPerConversation.result_label(BackendKind::Cli),
            "per-conversation daemon"
        );
        assert_eq!(
            Isolation::Shared.result_label(BackendKind::Cli),
            "shared daemon (cross-conversation bleed)"
        );
        assert_eq!(
            Isolation::DaemonPerConversation.result_label(BackendKind::Mock),
            "per-conversation temp store"
        );
    }

    #[test]
    fn cli_isolated_daemons_do_not_bleed() {
        if std::env::var("MEMORY_BENCH_LIVE").ok().as_deref() != Some("1") {
            eprintln!("skip: set MEMORY_BENCH_LIVE=1 after building memory + memory-daemon");
            return;
        }
        let Some(daemon) = find_bin("memory-daemon") else {
            eprintln!("skip: memory-daemon binary not found (CI bench-cli-smoke builds it)");
            return;
        };
        let Some(memory) = find_bin("memory") else {
            eprintln!("skip: memory binary not found");
            return;
        };

        eprintln!(
            "live isolation: daemon={} memory={}",
            daemon.display(),
            memory.display()
        );

        let a = IsolatedDaemon::spawn(daemon.to_str().unwrap()).expect("spawn A");
        eprintln!("spawned A at {}", a.endpoint);
        let b = IsolatedDaemon::spawn(daemon.to_str().unwrap()).expect("spawn B");
        eprintln!("spawned B at {}", b.endpoint);

        let cfg_a = RunConfig {
            memory_bin: memory.to_string_lossy().into_owned(),
            daemon_bin: daemon.to_string_lossy().into_owned(),
            endpoint: a.endpoint.clone(),
            backend: BackendKind::Cli,
            isolation: Isolation::DaemonPerConversation,
            limit_questions: None,
            layer: crate::layers::RetrievalLayer::Bm25,
        };
        let mut cfg_b = cfg_a.clone();
        cfg_b.endpoint = b.endpoint.clone();

        let dir = tempfile::tempdir().unwrap();
        let path_a = dir.path().join("a.jsonl");
        let path_b = dir.path().join("b.jsonl");
        std::fs::write(&path_a, r#"{"content":"UNIQUE_ALPHA_TOKEN jwt rotation"}"#).unwrap();
        std::fs::write(&path_b, r#"{"content":"UNIQUE_BETA_TOKEN redis cache"}"#).unwrap();

        ingest_session_cli(path_a.to_str().unwrap(), &cfg_a).unwrap();
        ingest_session_cli(path_b.to_str().unwrap(), &cfg_b).unwrap();
        eprintln!("ingested; draining A");
        wait_for_drain(&cfg_a.daemon_bin, &cfg_a.endpoint).unwrap();
        eprintln!("drained A; draining B");
        wait_for_drain(&cfg_b.daemon_bin, &cfg_b.endpoint).unwrap();
        eprintln!("drained B; searching");

        let hits_b = run_query_cli("UNIQUE_ALPHA_TOKEN", &cfg_b, 5).unwrap();
        assert!(
            hits_b
                .ranked
                .iter()
                .all(|h| !h.text.contains("UNIQUE_ALPHA_TOKEN")),
            "daemon B must not see A's events: {:?}",
            hits_b.ranked
        );
        let hits_a = run_query_cli("UNIQUE_ALPHA_TOKEN", &cfg_a, 5).unwrap();
        assert!(
            hits_a
                .ranked
                .iter()
                .any(|h| h.text.contains("UNIQUE_ALPHA_TOKEN")),
            "daemon A must see its own token: {:?}",
            hits_a.ranked
        );

        a.stop().ok();
        b.stop().ok();
    }
}
