//! End-to-end vector semantic search tests for agent-memory.
//!
//! E2E-03: Vector ingest -> index -> semantic search pipeline
//! E2E-03b: Agent attribution on vector results
//!
//! NOTE: These tests require the all-MiniLM-L6-v2 model (~80MB download on first run).
//! The model is cached locally after the first download. Run with:
//!   cargo test -p e2e-tests --test vector_search_test -- --ignored --nocapture

use std::sync::{Arc, OnceLock};

use pretty_assertions::assert_eq;

use e2e_tests::TestHarness;
use memory_embeddings::{CandleEmbedder, EmbeddingModel};
use memory_vector::{DocType, HnswConfig, HnswIndex, VectorEntry, VectorIndex, VectorMetadata};

/// Shared embedder across tests to avoid concurrent model loading.
/// The model is loaded once and reused by all tests in this file.
static EMBEDDER: OnceLock<Arc<CandleEmbedder>> = OnceLock::new();

/// Get or initialize the shared embedder (thread-safe, loads once).
fn get_embedder() -> Arc<CandleEmbedder> {
    EMBEDDER
        .get_or_init(|| {
            let embedder =
                CandleEmbedder::load_default().expect("Failed to load embedding model");
            Arc::new(embedder)
        })
        .clone()
}

/// E2E-03: Verify that vector semantic search returns semantically similar results
/// ordered by relevance score. Ingests events across 3 distinct topic groups,
/// embeds and indexes them, then searches to verify the closest matching topic
/// ranks first with proper score ordering.
#[tokio::test]
#[ignore = "requires model download (~80MB on first run)"]
async fn test_vector_ingest_index_search_semantic() {
    // 1. Create a TestHarness
    let harness = TestHarness::new();

    // 2. Define 3 groups of text about distinct topics
    let group_a_texts = [
        "Rust ownership system ensures memory safety without garbage collection",
        "Borrowing rules in Rust prevent data races at compile time",
        "Lifetimes in Rust track how long references are valid",
        "The borrow checker enforces ownership and borrowing rules statically",
        "Move semantics in Rust transfer ownership of values between variables",
    ];

    let group_b_texts = [
        "Italian pasta recipes include classic carbonara and amatriciana",
        "Making fresh pasta dough requires flour eggs and olive oil",
        "Cooking al dente pasta means boiling until firm to the bite",
        "Traditional bolognese sauce simmers for hours with meat and tomatoes",
        "Homemade ravioli are filled with ricotta cheese and spinach",
    ];

    let group_c_texts = [
        "Neural networks learn patterns through layers of connected nodes",
        "Deep learning uses backpropagation to train multi-layer models",
        "Convolutional neural networks excel at image recognition tasks",
        "Machine learning models generalize from training data to new inputs",
        "Gradient descent optimizes neural network weights during training",
    ];

    // 3. Load the embedding model (shared across tests via OnceLock)
    let embedder = tokio::task::spawn_blocking(get_embedder)
        .await
        .expect("Embedding model load task panicked");

    // 4. Create HnswIndex at harness.vector_index_path with dimension 384
    let hnsw_config = HnswConfig::new(384, &harness.vector_index_path).with_capacity(100);
    let mut hnsw_index =
        HnswIndex::open_or_create(hnsw_config).expect("Failed to create HNSW index");

    // 5. Create VectorMetadata backed by storage
    let metadata_path = harness.vector_index_path.join("metadata");
    let metadata =
        VectorMetadata::open(&metadata_path).expect("Failed to open vector metadata storage");

    // 6. Embed and index all texts, tracking which group each belongs to
    let all_texts: Vec<(&str, &str)> = group_a_texts
        .iter()
        .map(|t| (*t, "group_a"))
        .chain(group_b_texts.iter().map(|t| (*t, "group_b")))
        .chain(group_c_texts.iter().map(|t| (*t, "group_c")))
        .collect();

    let mut doc_id_to_group: Vec<(String, String)> = Vec::new();

    for (i, (text, group)) in all_texts.iter().enumerate() {
        let vector_id = (i + 1) as u64;
        let doc_id = format!("toc:segment:test-{}", i);

        // Embed text using spawn_blocking since it is CPU-bound
        let embedder_clone = embedder.clone();
        let text_owned = text.to_string();
        let embedding = tokio::task::spawn_blocking(move || {
            embedder_clone
                .embed(&text_owned)
                .expect("Failed to embed text")
        })
        .await
        .expect("Embed task panicked");

        // Add to HNSW index
        hnsw_index
            .add(vector_id, &embedding)
            .expect("Failed to add vector to index");

        // Store metadata
        let entry = VectorEntry::new(
            vector_id,
            DocType::TocNode,
            &doc_id,
            chrono::Utc::now().timestamp_millis(),
            text,
        )
        .with_agent(Some("claude".to_string()));
        metadata.put(&entry).expect("Failed to store vector entry");

        doc_id_to_group.push((doc_id, group.to_string()));
    }

    assert_eq!(hnsw_index.len(), 15, "Should have 15 vectors indexed");

    // 7. Wrap for VectorTeleportHandler
    let index_lock = Arc::new(std::sync::RwLock::new(hnsw_index));
    let metadata = Arc::new(metadata);

    let handler =
        memory_service::VectorTeleportHandler::new(embedder.clone(), index_lock, metadata);

    // 8. Search for "Rust memory management and borrowing"
    let results = handler
        .search("Rust memory management and borrowing", 10, 0.0)
        .await
        .expect("Vector search failed");

    // 9. Verify results are non-empty
    assert!(
        !results.is_empty(),
        "Vector search should return non-empty results"
    );

    // 10. Verify first result is from Group A (Rust topic)
    let first_doc_id = &results[0].doc_id;
    let first_group = doc_id_to_group
        .iter()
        .find(|(id, _)| id == first_doc_id)
        .map(|(_, g)| g.as_str())
        .expect("First result doc_id not found in mapping");

    assert_eq!(
        first_group, "group_a",
        "First result for 'Rust memory management' should be from Group A (Rust topic), got doc_id={}",
        first_doc_id
    );

    // 11. Verify results are ordered by descending score
    for i in 1..results.len() {
        assert!(
            results[i - 1].score >= results[i].score,
            "Results should be ordered by descending score: result[{}].score={} >= result[{}].score={}",
            i - 1,
            results[i - 1].score,
            i,
            results[i].score
        );
    }

    // 12. Verify Group A result has higher score than Group B (cooking) result
    let group_a_max_score = results
        .iter()
        .filter(|r| {
            doc_id_to_group
                .iter()
                .any(|(id, g)| id == &r.doc_id && g == "group_a")
        })
        .map(|r| r.score)
        .fold(f32::NEG_INFINITY, f32::max);

    let group_b_max_score = results
        .iter()
        .filter(|r| {
            doc_id_to_group
                .iter()
                .any(|(id, g)| id == &r.doc_id && g == "group_b")
        })
        .map(|r| r.score)
        .fold(f32::NEG_INFINITY, f32::max);

    assert!(
        group_a_max_score > group_b_max_score,
        "Group A (Rust) max score {} should be higher than Group B (cooking) max score {} for query 'Rust memory management'",
        group_a_max_score,
        group_b_max_score
    );

    // 13. Search for "pasta cooking recipes" and verify Group B ranks first
    let pasta_results = handler
        .search("pasta cooking recipes", 10, 0.0)
        .await
        .expect("Pasta search failed");

    assert!(
        !pasta_results.is_empty(),
        "Pasta search should return results"
    );

    let pasta_first_doc_id = &pasta_results[0].doc_id;
    let pasta_first_group = doc_id_to_group
        .iter()
        .find(|(id, _)| id == pasta_first_doc_id)
        .map(|(_, g)| g.as_str())
        .expect("Pasta first result doc_id not found");

    assert_eq!(
        pasta_first_group, "group_b",
        "First result for 'pasta cooking recipes' should be from Group B (cooking), got doc_id={}",
        pasta_first_doc_id
    );
}

/// E2E-03b: Verify agent attribution propagates through vector results.
/// Creates events with agent = "opencode", embeds and indexes them,
/// then verifies the search result carries agent = Some("opencode").
#[tokio::test]
#[ignore = "requires model download (~80MB on first run)"]
async fn test_vector_search_with_agent_attribution() {
    // 1. Create harness
    let harness = TestHarness::new();

    // 2. Load embedding model (shared via OnceLock)
    let embedder = tokio::task::spawn_blocking(get_embedder)
        .await
        .expect("Embedding model load task panicked");

    // 3. Create index and metadata
    let hnsw_config = HnswConfig::new(384, &harness.vector_index_path).with_capacity(10);
    let mut hnsw_index =
        HnswIndex::open_or_create(hnsw_config).expect("Failed to create HNSW index");

    let metadata_path = harness.vector_index_path.join("metadata");
    let metadata = VectorMetadata::open(&metadata_path).expect("Failed to open metadata");

    // 4. Embed a text and store with agent = "opencode"
    let text = "OpenCode agent performing code analysis and review";
    let embedder_clone = embedder.clone();
    let text_owned = text.to_string();
    let embedding = tokio::task::spawn_blocking(move || {
        embedder_clone
            .embed(&text_owned)
            .expect("Failed to embed text")
    })
    .await
    .expect("Embed task panicked");

    hnsw_index.add(1, &embedding).expect("Failed to add vector");

    let entry = VectorEntry::new(
        1,
        DocType::TocNode,
        "toc:segment:agent-test-1",
        chrono::Utc::now().timestamp_millis(),
        text,
    )
    .with_agent(Some("opencode".to_string()));
    metadata.put(&entry).expect("Failed to store entry");

    // 5. Create handler and search
    let index_lock = Arc::new(std::sync::RwLock::new(hnsw_index));
    let metadata = Arc::new(metadata);

    let handler =
        memory_service::VectorTeleportHandler::new(embedder.clone(), index_lock, metadata);

    let results = handler
        .search("code analysis review", 5, 0.0)
        .await
        .expect("Search failed");

    // 6. Verify agent attribution
    assert!(!results.is_empty(), "Should have search results");
    assert_eq!(
        results[0].agent,
        Some("opencode".to_string()),
        "Result should have agent = 'opencode'"
    );
}
