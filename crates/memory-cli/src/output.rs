//! JSON envelope output formatting with TTY-aware printing.

use serde::{Deserialize, Serialize};
use std::io::IsTerminal;

/// JSON envelope for all CLI output.
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonEnvelope {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub meta: Meta,
}

/// Metadata about the retrieval operation.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Meta {
    pub retrieval_ms: u64,
    pub tokens_estimated: usize,
    pub confidence: f64,
}

impl JsonEnvelope {
    /// Create a successful envelope with query results.
    pub fn ok(query: &str, results: serde_json::Value) -> Self {
        Self {
            status: "ok".to_string(),
            query: Some(query.to_string()),
            results: Some(results),
            context: None,
            error: None,
            meta: Meta::default(),
        }
    }

    /// Create a successful envelope with context payload.
    pub fn context_ok(query: &str, context: serde_json::Value) -> Self {
        Self {
            status: "ok".to_string(),
            query: Some(query.to_string()),
            results: None,
            context: Some(context),
            error: None,
            meta: Meta::default(),
        }
    }

    /// Create an error envelope.
    pub fn error(msg: &str) -> Self {
        Self {
            status: "error".to_string(),
            query: None,
            results: None,
            context: None,
            error: Some(msg.to_string()),
            meta: Meta::default(),
        }
    }

    /// Set metadata on the envelope (builder pattern).
    pub fn with_meta(mut self, meta: Meta) -> Self {
        self.meta = meta;
        self
    }

    /// Serialize this envelope to a JSON string.
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Estimate the token count for a text string.
///
/// Uses the heuristic: `chars * 0.75 + 50` (overhead for framing).
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() as f64 * 0.75 + 50.0) as usize
}

/// Determine whether to force JSON output based on format arguments.
///
/// Returns `true` if either the global `--format` or command-level `--format` is `"json"`.
pub fn should_force_json(global_format: &Option<String>, cmd_format: &Option<String>) -> bool {
    matches!(global_format.as_deref(), Some("json"))
        || matches!(cmd_format.as_deref(), Some("json"))
}

/// Print the envelope to stdout, choosing format based on TTY detection.
///
/// - If `force_json` is true or stdout is not a terminal: compact JSON
/// - If TTY and status is "ok": human-readable pretty-print
/// - If TTY and status is "error": print error to stderr
pub fn print_output(envelope: &JsonEnvelope, force_json: bool) {
    let is_tty = std::io::stdout().is_terminal();

    if force_json || !is_tty {
        println!("{}", serde_json::to_string(envelope).unwrap_or_default());
        return;
    }

    // Human-readable TTY output
    if envelope.status == "error" {
        if let Some(ref err) = envelope.error {
            eprintln!("Error: {err}");
        }
        return;
    }

    // Print query header
    if let Some(ref query) = envelope.query {
        println!("Query: {query}");
        println!();
    }

    // Print results or context
    if let Some(ref results) = envelope.results {
        println!(
            "{}",
            serde_json::to_string_pretty(results).unwrap_or_default()
        );
    } else if let Some(ref context) = envelope.context {
        println!(
            "{}",
            serde_json::to_string_pretty(context).unwrap_or_default()
        );
    }

    // Print meta footer
    let meta = &envelope.meta;
    if meta.retrieval_ms > 0 || meta.confidence > 0.0 {
        println!();
        println!(
            "({} ms, ~{} tokens, confidence: {:.2})",
            meta.retrieval_ms, meta.tokens_estimated, meta.confidence
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_envelope_ok_serializes() {
        let env = JsonEnvelope::ok("search", json!([]));
        let json_str = env.to_json_string();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["status"], "ok");
        assert_eq!(parsed["query"], "search");
        assert_eq!(parsed["results"], json!([]));
    }

    #[test]
    fn test_json_envelope_error_serializes() {
        let env = JsonEnvelope::error("daemon down");
        let json_str = env.to_json_string();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["status"], "error");
        assert_eq!(parsed["error"], "daemon down");
    }

    #[test]
    fn test_json_envelope_skips_none_fields() {
        let env = JsonEnvelope::error("fail");
        let json_str = env.to_json_string();

        // None fields should not appear in JSON
        assert!(!json_str.contains("\"query\""));
        assert!(!json_str.contains("\"results\""));
        assert!(!json_str.contains("\"context\""));
    }

    #[test]
    fn test_json_envelope_ok_skips_error_and_context() {
        let env = JsonEnvelope::ok("test", json!([1, 2, 3]));
        let json_str = env.to_json_string();

        assert!(!json_str.contains("\"error\""));
        assert!(!json_str.contains("\"context\""));
    }

    #[test]
    fn test_meta_default() {
        let meta = Meta::default();
        assert_eq!(meta.retrieval_ms, 0);
        assert_eq!(meta.tokens_estimated, 0);
        assert!((meta.confidence - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_with_meta() {
        let meta = Meta {
            retrieval_ms: 42,
            tokens_estimated: 100,
            confidence: 0.95,
        };
        let env = JsonEnvelope::ok("q", json!([])).with_meta(meta);
        assert_eq!(env.meta.retrieval_ms, 42);
        assert_eq!(env.meta.tokens_estimated, 100);
        assert!((env.meta.confidence - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn test_estimate_tokens() {
        // "hello world" = 11 chars, 11 * 0.75 + 50 = 58.25 -> 58
        assert_eq!(estimate_tokens("hello world"), 58);
    }

    #[test]
    fn test_estimate_tokens_empty() {
        // 0 * 0.75 + 50 = 50
        assert_eq!(estimate_tokens(""), 50);
    }

    #[test]
    fn test_should_force_json_global() {
        assert!(should_force_json(&Some("json".to_string()), &None));
    }

    #[test]
    fn test_should_force_json_cmd() {
        assert!(should_force_json(&None, &Some("json".to_string())));
    }

    #[test]
    fn test_should_force_json_neither() {
        assert!(!should_force_json(&None, &None));
    }

    #[test]
    fn test_should_force_json_non_json() {
        assert!(!should_force_json(&Some("table".to_string()), &None));
    }

    #[test]
    fn test_context_ok() {
        let env = JsonEnvelope::context_ok("what happened", json!({"summary": "things"}));
        let json_str = env.to_json_string();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["status"], "ok");
        assert_eq!(parsed["query"], "what happened");
        assert_eq!(parsed["context"]["summary"], "things");
        assert!(!json_str.contains("\"results\""));
    }

    #[test]
    fn test_force_json_produces_json_string() {
        // When force_json=true, print_output writes JSON to stdout.
        // We verify the envelope serializes to valid JSON.
        let env = JsonEnvelope::ok("test", json!(["a", "b"]));
        let output = serde_json::to_string(&env).unwrap();
        let reparsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(reparsed["status"], "ok");
        assert_eq!(reparsed["results"], json!(["a", "b"]));
    }
}
