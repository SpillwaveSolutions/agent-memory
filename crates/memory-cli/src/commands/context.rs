//! Context command: builds a structured context window from memory.

use anyhow::Result;
use serde_json::{json, Value};

use crate::cli::{ContextArgs, GlobalArgs};
use crate::commands::search::{build_meta, build_results_json};
use crate::output::{print_output, should_force_json, JsonEnvelope};

/// Run the context command: query daemon and return structured MemoryContext-shaped JSON.
pub async fn run(args: ContextArgs, global: &GlobalArgs) -> Result<()> {
    let mut client = crate::client::connect_client(&global.endpoint).await?;
    let response = client.route_query(&args.query, 10, None).await?;

    let results_json = build_results_json(&response);
    let meta = build_meta(&response);

    // Extract unique doc_ids as entity references (simple heuristic)
    let key_entities: Vec<Value> = response
        .results
        .iter()
        .map(|r| json!({ "id": r.doc_id, "type": r.doc_type }))
        .collect();

    let context = json!({
        "summary": format!("Memory context for: {}", args.query),
        "relevant_events": results_json,
        "key_entities": key_entities,
        "open_questions": [],
        "retrieval_ms": meta.retrieval_ms,
        "tokens_estimated": meta.tokens_estimated,
        "confidence": meta.confidence,
    });

    let envelope = JsonEnvelope::context_ok(&args.query, context).with_meta(meta);
    print_output(&envelope, should_force_json(&global.format, &args.format));
    Ok(())
}

#[cfg(test)]
mod tests {
    use memory_client::{RetrievalResult, RouteQueryResponse};
    use serde_json::json;
    use std::collections::HashMap;

    use crate::commands::search::{build_meta, build_results_json};

    fn make_result(doc_id: &str, doc_type: &str) -> RetrievalResult {
        RetrievalResult {
            doc_id: doc_id.to_string(),
            doc_type: doc_type.to_string(),
            score: 0.85,
            text_preview: "some text".to_string(),
            source_layer: 3,
            metadata: HashMap::new(),
            agent: None,
            project: None,
        }
    }

    #[test]
    fn test_context_json_shape() {
        let response = RouteQueryResponse {
            results: vec![
                make_result("doc-1", "episodic"),
                make_result("doc-2", "semantic"),
            ],
            explanation: None,
            has_results: true,
            layers_attempted: vec![],
        };

        let results_json = build_results_json(&response);
        let meta = build_meta(&response);

        let key_entities: Vec<serde_json::Value> = response
            .results
            .iter()
            .map(|r| json!({ "id": r.doc_id, "type": r.doc_type }))
            .collect();

        let context = json!({
            "summary": "Memory context for: test query",
            "relevant_events": results_json,
            "key_entities": key_entities,
            "open_questions": [],
            "retrieval_ms": meta.retrieval_ms,
            "tokens_estimated": meta.tokens_estimated,
            "confidence": meta.confidence,
        });

        // Verify required fields
        assert_eq!(context["summary"], "Memory context for: test query");
        assert!(context["relevant_events"].is_array());
        assert_eq!(context["relevant_events"].as_array().unwrap().len(), 2);
        assert!(context["key_entities"].is_array());
        assert_eq!(context["key_entities"].as_array().unwrap().len(), 2);
        assert_eq!(context["key_entities"][0]["id"], "doc-1");
        assert_eq!(context["key_entities"][0]["type"], "episodic");
        assert_eq!(context["key_entities"][1]["type"], "semantic");
        assert!(context["open_questions"].is_array());
        assert_eq!(context["open_questions"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_context_empty_results() {
        let response = RouteQueryResponse {
            results: vec![],
            explanation: None,
            has_results: false,
            layers_attempted: vec![],
        };

        let results_json = build_results_json(&response);
        let key_entities: Vec<serde_json::Value> = vec![];

        let context = json!({
            "summary": "Memory context for: nothing",
            "relevant_events": results_json,
            "key_entities": key_entities,
            "open_questions": [],
        });

        assert_eq!(context["relevant_events"], json!([]));
        assert_eq!(context["key_entities"], json!([]));
    }
}
