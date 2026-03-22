//! JSON envelope output formatting with TTY-aware printing.
//!
//! Placeholder -- will be fully implemented in Task 2.

use serde::{Deserialize, Serialize};

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
