//! Federated cross-project query handler (v3.0).
//!
//! Implements the cross-project unified memory feature: fans out a query to
//! multiple registered project stores, tags each result with its source project
//! path, merges the result lists, and re-ranks by score.
//!
//! Design principles (per v3.0 spec):
//! - Fail-open: if a remote project store is unavailable, it is silently skipped.
//! - Config-driven: additional stores come from `Settings.projects.registered`.
//! - Opt-in: only activated when the caller sets `all_projects = true`.
//! - TOC-search based: cross-project fallback uses TOC keyword scanning so it
//!   works even when BM25/vector indexes are not present for remote stores.
//! - Project attribution: results carry `project` metadata matching the existing
//!   `agent` metadata convention (`serde(default)` for backward compat).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tracing::{debug, warn};

use memory_retrieval::executor::SearchResult;
use memory_retrieval::types::RetrievalLayer;
use memory_storage::Storage;
use memory_types::TocLevel;

/// Search TOC segment-level nodes in a single storage instance for keyword
/// overlap with `query`. Returns up to `limit` results sorted by score.
///
/// Uses an index-free approach so it works without BM25/vector indexes.
/// This is the "Agentic TOC Search" (Layer 2) applied to foreign stores.
fn search_toc_in_store(storage: &Storage, query: &str, limit: usize) -> Vec<SearchResult> {
    let query_terms: Vec<String> = query
        .split_whitespace()
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_lowercase())
        .collect();

    if query_terms.is_empty() {
        return Vec::new();
    }

    // Search segment-level nodes (most granular — always present)
    let nodes = match storage.get_toc_nodes_by_level(TocLevel::Segment, None, None) {
        Ok(n) => n,
        Err(e) => {
            warn!("federated: TOC scan failed: {}", e);
            return Vec::new();
        }
    };

    let mut results: Vec<SearchResult> = nodes
        .into_iter()
        .filter_map(|node| {
            // Compute term overlap score across title, keywords, and bullet text
            let searchable = format!(
                "{} {} {}",
                node.title,
                node.keywords.join(" "),
                node.bullets
                    .iter()
                    .map(|b| b.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .to_lowercase();

            let matched: usize = query_terms
                .iter()
                .filter(|t| searchable.contains(t.as_str()))
                .count();

            if matched == 0 {
                return None;
            }

            let score = matched as f32 / query_terms.len() as f32;

            // Build metadata the same way other layers do
            let mut metadata: HashMap<String, String> = HashMap::new();
            let ts = node.start_time.timestamp_millis();
            metadata.insert("timestamp_ms".to_string(), ts.to_string());
            metadata.insert("memory_kind".to_string(), node.memory_kind.to_string());
            metadata.insert("salience_score".to_string(), node.salience_score.to_string());
            if !node.contributing_agents.is_empty() {
                metadata.insert("agent".to_string(), node.contributing_agents[0].clone());
            }

            Some(SearchResult {
                doc_id: node.node_id.clone(),
                doc_type: "toc_node".to_string(),
                score,
                text_preview: node.title.clone(),
                source_layer: RetrievalLayer::Agentic,
                metadata,
            })
        })
        .collect();

    // Sort by score descending, cap to limit
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    results
}

/// Perform a federated cross-project query.
///
/// Uses `primary_storage` for a TOC-based fallback scan on the primary store
/// when `primary_results` is empty (e.g., when no BM25/vector indexes exist).
/// Opens each registered project store in read-only mode, fans out
/// `search_toc_in_store` to each, tags results with the source project path,
/// merges all result lists, and sorts by score descending.
///
/// Unavailable stores are silently skipped (fail-open semantics).
///
/// # Arguments
/// * `primary_results` – Results already obtained from the primary store pipeline.
/// * `primary_storage` – Reference to the primary store for TOC fallback when results are empty.
/// * `primary_path` – Path string for the primary store (used as attribution).
/// * `registered_paths` – Additional project store paths to query.
/// * `query` – The search query string.
/// * `limit` – Maximum results to return after merging.
pub fn federated_query(
    primary_results: Vec<SearchResult>,
    primary_storage: &Storage,
    primary_path: &str,
    registered_paths: &[PathBuf],
    query: &str,
    limit: usize,
) -> Vec<SearchResult> {
    if registered_paths.is_empty() {
        // Fast path: if no remote stores, still ensure primary is searched via TOC
        // so cross-project mode works even without BM25/vector indexes.
        let primary = if primary_results.is_empty() {
            debug!("federated: primary results empty, falling back to TOC scan on primary");
            search_toc_in_store(primary_storage, query, limit)
        } else {
            primary_results
        };
        return tag_with_project(primary, primary_path);
    }

    // Use primary pipeline results, or fall back to TOC scan on primary if empty.
    let primary_results = if primary_results.is_empty() {
        debug!("federated: primary results empty, falling back to TOC scan on primary");
        search_toc_in_store(primary_storage, query, limit)
    } else {
        primary_results
    };

    // Tag primary results and collect
    let mut merged = tag_with_project(primary_results, primary_path);

    // Fan out to each registered store
    for store_path in registered_paths {
        let path_str = store_path.to_string_lossy().to_string();
        debug!("federated: opening store at {:?}", store_path);

        match Storage::open_read_only(store_path.as_path()) {
            Ok(storage) => {
                let results = search_toc_in_store(&storage, query, limit);
                debug!(
                    "federated: {} results from {:?}",
                    results.len(),
                    store_path
                );
                let tagged = tag_with_project(results, &path_str);
                merged.extend(tagged);
            }
            Err(e) => {
                // fail-open: log warning, skip this store, continue
                warn!(
                    "federated: skipping unavailable store {:?}: {}",
                    store_path, e
                );
            }
        }
    }

    // Re-sort by score descending and cap to limit
    merged.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    merged.truncate(limit);
    merged
}

/// Inject `project` attribution into every result's metadata.
fn tag_with_project(results: Vec<SearchResult>, project_path: &str) -> Vec<SearchResult> {
    results
        .into_iter()
        .map(|mut r| {
            r.metadata
                .insert("project".to_string(), project_path.to_string());
            r
        })
        .collect()
}

/// Open a registered project store read-only, returning `None` on failure (fail-open).
#[allow(dead_code)]
pub fn try_open_store(path: &Path) -> Option<Arc<Storage>> {
    match Storage::open_read_only(path) {
        Ok(s) => Some(Arc::new(s)),
        Err(e) => {
            warn!("federated: cannot open store {:?}: {}", path, e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use memory_types::{MemoryKind, TocBullet, TocLevel as TL, TocNode};
    use tempfile::TempDir;

    fn make_toc_node(node_id: &str, title: &str, keywords: Vec<&str>, agent: &str) -> TocNode {
        let now = Utc::now();
        let mut node = TocNode::new(
            node_id.to_string(),
            TL::Segment,
            title.to_string(),
            now,
            now,
        );
        node.keywords = keywords.into_iter().map(|s| s.to_string()).collect();
        node.bullets = vec![TocBullet::new(format!("{} discussion", title))];
        node.contributing_agents = vec![agent.to_string()];
        node.salience_score = 0.7;
        node.memory_kind = MemoryKind::Observation;
        node
    }

    fn make_search_result(doc_id: &str, score: f32) -> SearchResult {
        SearchResult {
            doc_id: doc_id.to_string(),
            doc_type: "toc_node".to_string(),
            score,
            text_preview: doc_id.to_string(),
            source_layer: RetrievalLayer::Agentic,
            metadata: HashMap::new(),
        }
    }

    /// Test: empty registered list returns primary results with project tag.
    #[test]
    fn test_federated_empty_registered_returns_tagged_primary() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(dir.path()).unwrap();
        let primary = vec![make_search_result("doc1", 0.9)];
        let result = federated_query(primary, &storage, "/primary", &[], "hello", 10);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].doc_id, "doc1");
        assert_eq!(
            result[0].metadata.get("project").map(|s| s.as_str()),
            Some("/primary")
        );
    }

    /// Test: unavailable store is skipped silently (fail-open).
    #[test]
    fn test_federated_unavailable_store_skipped_gracefully() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(dir.path()).unwrap();
        let primary = vec![];
        let missing = vec![PathBuf::from("/nonexistent/path/db_xyz")];
        // Must not panic; should return empty (primary is empty, remote unavailable, storage is empty)
        let result = federated_query(primary, &storage, "/primary", &missing, "query", 10);
        assert!(result.is_empty(), "Expected empty result with unavailable store");
    }

    /// Test: primary results keep project attribution even when remotes are added.
    #[test]
    fn test_federated_project_attribution_primary() {
        let primary = vec![make_search_result("primary-doc", 0.8)];
        // Use a real but empty store for the remote
        let dir_b = TempDir::new().unwrap();
        let _store_b = Storage::open(dir_b.path()).unwrap(); // create the DB

        let dir_primary = TempDir::new().unwrap();
        let storage_primary = Storage::open(dir_primary.path()).unwrap();

        let result = federated_query(
            primary,
            &storage_primary,
            "/my/primary",
            &[dir_b.path().to_path_buf()],
            "test",
            10,
        );
        // At least primary result is present
        let primary_res = result.iter().find(|r| r.doc_id == "primary-doc");
        assert!(primary_res.is_some(), "Primary doc should be in results");
        assert_eq!(
            primary_res.unwrap().metadata.get("project").map(|s| s.as_str()),
            Some("/my/primary"),
        );
    }

    /// Test: two stores merged, results from both present and ranked.
    #[test]
    fn test_federated_two_stores_merged_and_ranked() {
        let dir_a = TempDir::new().unwrap();
        let dir_b = TempDir::new().unwrap();

        // Ensure dir_b has a valid RocksDB
        let store_b = Storage::open(dir_b.path()).unwrap();

        // Put a matching TocNode in store_b
        let node = make_toc_node("toc:segment:node_b_1", "rust ownership discussion", vec!["rust", "ownership"], "claude");
        store_b.put_toc_node(&node).unwrap();
        drop(store_b); // close write handle before read-only open

        // Primary result from store_a
        let primary = vec![make_search_result("primary-doc-1", 0.5)];

        let store_a = Storage::open(dir_a.path()).unwrap();

        let result = federated_query(
            primary,
            &store_a,
            dir_a.path().to_str().unwrap(),
            &[dir_b.path().to_path_buf()],
            "rust ownership",
            20,
        );

        // Should include the primary result
        assert!(
            result.iter().any(|r| r.doc_id == "primary-doc-1"),
            "Primary result should be in merged output"
        );

        // Should include the store_b result
        assert!(
            result.iter().any(|r| r.doc_id == "toc:segment:node_b_1"),
            "Store_b node should be in merged results: {:?}",
            result.iter().map(|r| &r.doc_id).collect::<Vec<_>>()
        );

        // All results should have project attribution
        for r in &result {
            assert!(
                r.metadata.contains_key("project"),
                "Result missing project attribution: {}",
                r.doc_id
            );
        }

        // store_b result attributed to store_b path
        let store_b_result = result.iter().find(|r| r.doc_id == "toc:segment:node_b_1").unwrap();
        assert_eq!(
            store_b_result.metadata.get("project").map(|s| s.as_str()),
            Some(dir_b.path().to_str().unwrap()),
        );
    }

    /// Test: search_toc_in_store returns empty for empty store.
    #[test]
    fn test_search_toc_in_store_empty_store() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(dir.path()).unwrap();
        let results = search_toc_in_store(&storage, "rust ownership", 10);
        assert!(results.is_empty());
    }

    /// Test: search_toc_in_store finds matching nodes.
    #[test]
    fn test_search_toc_in_store_finds_matching_node() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(dir.path()).unwrap();

        let node = make_toc_node("toc:segment:test_1", "Rust ownership and lifetimes", vec!["rust", "ownership"], "claude");
        storage.put_toc_node(&node).unwrap();

        let results = search_toc_in_store(&storage, "rust ownership", 10);
        assert_eq!(results.len(), 1, "Should find the matching node");
        assert_eq!(results[0].doc_id, "toc:segment:test_1");
        assert!(results[0].score > 0.0, "Score should be positive");
    }

    /// Test: search_toc_in_store does not find non-matching nodes.
    #[test]
    fn test_search_toc_in_store_no_match_returns_empty() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(dir.path()).unwrap();

        let node = make_toc_node("toc:segment:test_2", "Python machine learning", vec!["python", "ml"], "claude");
        storage.put_toc_node(&node).unwrap();

        let results = search_toc_in_store(&storage, "rust ownership", 10);
        assert!(results.is_empty(), "Should not match Python node for rust query");
    }

    /// Test: default (single-project) behavior is unchanged when all_projects is false.
    ///
    /// When registered_paths is empty, results are identical to the primary
    /// results (plus project tag), and no additional storage is opened.
    #[test]
    fn test_default_single_project_behavior_unchanged() {
        let primary = vec![
            make_search_result("doc-a", 0.9),
            make_search_result("doc-b", 0.7),
        ];

        // No registered paths = single-project mode
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(dir.path()).unwrap();
        let result = federated_query(primary, &storage, "/project", &[], "query", 10);

        assert_eq!(result.len(), 2);
        // Sorted by score descending
        assert_eq!(result[0].doc_id, "doc-a");
        assert_eq!(result[1].doc_id, "doc-b");
        // Both have project attribution
        for r in &result {
            assert_eq!(r.metadata.get("project").map(|s| s.as_str()), Some("/project"));
        }
    }

    /// Test: try_open_store returns None for missing path.
    #[test]
    fn test_try_open_store_missing_returns_none() {
        let result = try_open_store(Path::new("/nonexistent/path/db_zzz"));
        assert!(result.is_none());
    }
}
