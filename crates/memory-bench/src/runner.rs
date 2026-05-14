use std::io::BufRead;
use std::process::Command;
use std::time::Instant;

/// Configuration for the benchmark runner.
pub struct RunConfig {
    /// Path to the memory binary (default: "memory").
    pub memory_bin: String,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            memory_bin: "memory".to_string(),
        }
    }
}

/// Result of running a single query against the memory binary.
pub struct QueryResult {
    /// Raw stdout output from the binary.
    pub raw_output: String,
    /// Elapsed time in milliseconds.
    pub latency_ms: u64,
    /// Token count from meta.tokens_estimated in JSON envelope.
    pub tokens_estimated: usize,
    /// Whether the command exited successfully.
    pub success: bool,
}

/// Run a search query against the memory binary and capture JSON output + latency.
pub fn run_query(query: &str, config: &RunConfig) -> QueryResult {
    let start = Instant::now();
    let output = Command::new(&config.memory_bin)
        .args(["search", query, "--format=json"])
        .output();

    let elapsed = start.elapsed().as_millis() as u64;

    match output {
        Ok(out) => {
            let raw_output = String::from_utf8_lossy(&out.stdout).to_string();
            let tokens_estimated = extract_tokens_estimated(&raw_output);
            QueryResult {
                raw_output,
                latency_ms: elapsed,
                tokens_estimated,
                success: out.status.success(),
            }
        }
        Err(_) => QueryResult {
            raw_output: String::new(),
            latency_ms: elapsed,
            tokens_estimated: 0,
            success: false,
        },
    }
}

/// Extract meta.tokens_estimated from JSON envelope output.
fn extract_tokens_estimated(json_output: &str) -> usize {
    serde_json::from_str::<serde_json::Value>(json_output)
        .ok()
        .and_then(|v| v.get("meta")?.get("tokens_estimated")?.as_u64())
        .unwrap_or(0) as usize
}

/// Ingest a JSONL session file by calling `memory add` for each line.
pub fn ingest_session(session_path: &str, config: &RunConfig) -> anyhow::Result<()> {
    let file = std::fs::File::open(session_path)?;
    let reader = std::io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let _ = Command::new(&config.memory_bin)
            .args(["add", "--content", trimmed, "--kind", "episodic"])
            .output();
    }

    Ok(())
}
