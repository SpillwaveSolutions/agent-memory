//! Search command: queries the daemon via RouteQuery RPC.

use anyhow::Result;
use memory_client::RouteQueryResponse;
use serde_json::{json, Value};

use crate::cli::{GlobalArgs, SearchArgs};
use crate::output::{estimate_tokens, print_output, should_force_json, JsonEnvelope, Meta};

/// Run the search command: connect to daemon, execute RouteQuery, format as JSON envelope.
pub async fn run(args: SearchArgs, global: &GlobalArgs) -> Result<()> {
    let mut client = crate::client::connect_client(&global.endpoint).await?;
    let response = client
        .route_query(&args.query, args.top as i32, None)
        .await?;

    let results_json = build_results_json(&response);
    let meta = build_meta(&response);
    let envelope = JsonEnvelope::ok(&args.query, results_json).with_meta(meta);

    print_output(&envelope, should_force_json(&global.format, &args.format));
    Ok(())
}

/// Map a single proto `RetrievalResult` to a JSON object.
fn map_retrieval_result(r: &memory_client::RetrievalResult) -> Value {
    json!({
        "doc_id": r.doc_id,
        "doc_type": r.doc_type,
        "score": r.score,
        "text_preview": r.text_preview,
        "source_layer": layer_to_string(r.source_layer),
        "metadata": r.metadata,
        "agent": r.agent,
    })
}

/// Convert proto `RetrievalLayer` i32 value to a human-readable string.
fn layer_to_string(layer: i32) -> &'static str {
    // Proto enum values from memory.proto:
    // RETRIEVAL_LAYER_UNSPECIFIED = 0
    // RETRIEVAL_LAYER_TOPICS = 1
    // RETRIEVAL_LAYER_HYBRID = 2
    // RETRIEVAL_LAYER_VECTOR = 3
    // RETRIEVAL_LAYER_BM25 = 4
    // RETRIEVAL_LAYER_AGENTIC = 5
    match layer {
        1 => "topics",
        2 => "hybrid",
        3 => "vector",
        4 => "bm25",
        5 => "agentic",
        _ => "unknown",
    }
}

/// Build the JSON array of search results from the RouteQuery response.
pub fn build_results_json(response: &RouteQueryResponse) -> Value {
    let results: Vec<Value> = response.results.iter().map(map_retrieval_result).collect();
    Value::Array(results)
}

/// Build metadata from the RouteQuery response.
pub fn build_meta(response: &RouteQueryResponse) -> Meta {
    let retrieval_ms = response
        .explanation
        .as_ref()
        .map_or(0, |e| e.total_time_ms);

    let tokens_estimated: usize = response
        .results
        .iter()
        .map(|r| estimate_tokens(&r.text_preview))
        .sum();

    let confidence = response
        .results
        .first()
        .map_or(0.0, |r| f64::from(r.score));

    Meta {
        retrieval_ms,
        tokens_estimated,
        confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_client::{ExplainabilityPayload, RetrievalResult};
    use std::collections::HashMap;

    fn make_result(doc_id: &str, score: f32, text: &str, layer: i32) -> RetrievalResult {
        RetrievalResult {
            doc_id: doc_id.to_string(),
            doc_type: "episodic".to_string(),
            score,
            text_preview: text.to_string(),
            source_layer: layer,
            metadata: HashMap::new(),
            agent: Some("test-agent".to_string()),
        }
    }

    #[test]
    fn test_map_retrieval_result_produces_correct_json() {
        let r = make_result("doc-1", 0.95, "hello world", 3);
        let json = map_retrieval_result(&r);

        assert_eq!(json["doc_id"], "doc-1");
        assert_eq!(json["doc_type"], "episodic");
        assert_eq!(json["score"], 0.95_f32);
        assert_eq!(json["text_preview"], "hello world");
        assert_eq!(json["source_layer"], "vector");
        assert_eq!(json["agent"], "test-agent");
    }

    #[test]
    fn test_build_results_json_maps_all_results() {
        let response = RouteQueryResponse {
            results: vec![
                make_result("a", 0.9, "first", 4),
                make_result("b", 0.8, "second", 3),
            ],
            explanation: None,
            has_results: true,
            layers_attempted: vec![],
        };
        let json = build_results_json(&response);
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["doc_id"], "a");
        assert_eq!(arr[0]["source_layer"], "bm25");
        assert_eq!(arr[1]["doc_id"], "b");
        assert_eq!(arr[1]["source_layer"], "vector");
    }

    #[test]
    fn test_build_meta_extracts_time_tokens_confidence() {
        let response = RouteQueryResponse {
            results: vec![
                make_result("a", 0.92, "hello world", 3),
                make_result("b", 0.80, "goodbye", 4),
            ],
            explanation: Some(ExplainabilityPayload {
                total_time_ms: 42,
                ..Default::default()
            }),
            has_results: true,
            layers_attempted: vec![],
        };
        let meta = build_meta(&response);
        assert_eq!(meta.retrieval_ms, 42);
        // "hello world" = 11 chars -> 11*0.75+50 = 58
        // "goodbye" = 7 chars -> 7*0.75+50 = 55
        assert_eq!(meta.tokens_estimated, 58 + 55);
        assert!((meta.confidence - 0.92_f64).abs() < 0.01);
    }

    #[test]
    fn test_build_meta_no_explanation() {
        let response = RouteQueryResponse {
            results: vec![make_result("a", 0.5, "text", 1)],
            explanation: None,
            has_results: true,
            layers_attempted: vec![],
        };
        let meta = build_meta(&response);
        assert_eq!(meta.retrieval_ms, 0);
    }

    #[test]
    fn test_empty_results_returns_empty_array() {
        let response = RouteQueryResponse {
            results: vec![],
            explanation: None,
            has_results: false,
            layers_attempted: vec![],
        };
        let json = build_results_json(&response);
        assert_eq!(json, json!([]));
        let meta = build_meta(&response);
        assert_eq!(meta.tokens_estimated, 0);
        assert!((meta.confidence - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_layer_to_string_all_variants() {
        assert_eq!(layer_to_string(0), "unknown");
        assert_eq!(layer_to_string(1), "topics");
        assert_eq!(layer_to_string(2), "hybrid");
        assert_eq!(layer_to_string(3), "vector");
        assert_eq!(layer_to_string(4), "bm25");
        assert_eq!(layer_to_string(5), "agentic");
        assert_eq!(layer_to_string(99), "unknown");
    }
}
