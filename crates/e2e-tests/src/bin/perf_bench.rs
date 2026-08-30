use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use chrono::{DateTime, TimeZone, Utc};
use clap::{Parser, ValueEnum};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use tonic::Request;

use e2e_tests::{build_toc_segment, ingest_events, TestHarness};
use memory_embeddings::{CandleEmbedder, EmbeddingModel};
use memory_search::{
    SearchIndex, SearchIndexConfig, SearchIndexer, SearchOptions, TeleportSearcher,
};
use memory_service::pb::RouteQueryRequest;
use memory_service::{RetrievalHandler, TopicGraphHandler, VectorTeleportHandler};
use memory_topics::{Topic, TopicStatus, TopicStorage};
use memory_types::{Event, EventRole, EventType, TocNode};
use memory_vector::{DocType, HnswConfig, HnswIndex, VectorEntry, VectorIndex, VectorMetadata};

static EMBEDDER: OnceLock<Arc<CandleEmbedder>> = OnceLock::new();

const SMALL_EVENT_COUNT: usize = 60;
const MEDIUM_EVENT_COUNT: usize = 240;
/// Query-step default. p99 is only reported at this count.
const DEFAULT_ITERATIONS: usize = 30;
const MIN_SAMPLES_P90: usize = 10;
const MIN_SAMPLES_P99: usize = 30;
const SCHEMA_VERSION: u32 = 2;

#[derive(Parser, Debug)]
#[command(
    name = "perf_bench",
    about = "Agent-memory performance benchmark harness"
)]
struct Args {
    #[arg(long, value_enum, default_value = "small")]
    tier: DatasetTier,
    #[arg(long, value_enum, default_value = "cold")]
    mode: RunMode,
    #[arg(long, default_value_t = DEFAULT_ITERATIONS)]
    iterations: usize,
    #[arg(long)]
    trace: Option<PathBuf>,
    #[arg(long, default_value = "crates/e2e-tests/benchmarks")]
    out_dir: PathBuf,
    #[arg(long)]
    write_baseline: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, ValueEnum, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DatasetTier {
    Small,
    Medium,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, ValueEnum, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RunMode {
    Cold,
    Warm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    Single,
    Multi,
}

impl Scenario {
    fn label(&self) -> &'static str {
        match self {
            Scenario::Single => "single",
            Scenario::Multi => "multi",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum StepKind {
    Setup,
    Query,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ThroughputMetrics {
    p50_eps: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    p90_eps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    p99_eps: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StepMetrics {
    kind: StepKind,
    samples: usize,
    min_ms: f64,
    p50_ms: f64,
    max_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    p90_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    p99_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    throughput_eps: Option<ThroughputMetrics>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CorpusInfo {
    tier: DatasetTier,
    event_count: usize,
    kind: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HardwareInfo {
    os: String,
    arch: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkOutput {
    schema_version: u32,
    tier: DatasetTier,
    mode: RunMode,
    iterations: usize,
    generated_at: String,
    corpus: CorpusInfo,
    hardware: HardwareInfo,
    caveats: Vec<String>,
    steps: BTreeMap<String, StepMetrics>,
    comparison: Option<ComparisonSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ComparisonSummary {
    warnings: Vec<Regression>,
    severe: Vec<Regression>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Regression {
    step: String,
    metric: String,
    baseline: f64,
    current: f64,
    severity: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BaselineFile {
    #[serde(default = "default_baseline_label")]
    baseline: String,
    version: u32,
    thresholds: Thresholds,
    runs: Vec<BaselineRun>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Thresholds {
    warning: Threshold,
    severe: Threshold,
}

#[derive(Debug, Serialize, Deserialize)]
struct Threshold {
    relative: f64,
    absolute_ms: f64,
    throughput_relative: f64,
    throughput_absolute: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct BaselineRun {
    tier: DatasetTier,
    mode: RunMode,
    steps: BTreeMap<String, StepMetrics>,
}

struct PreparedStore {
    harness: TestHarness,
    toc_node: TocNode,
    bm25_searcher: Arc<TeleportSearcher>,
    vector_handler: Arc<VectorTeleportHandler>,
    topic_handler: Arc<TopicGraphHandler>,
}

#[derive(Default)]
struct SampleCollector {
    durations: HashMap<String, Vec<f64>>,
    throughput: HashMap<String, Vec<f64>>,
    kinds: HashMap<String, StepKind>,
}

impl SampleCollector {
    fn record(&mut self, step: String, kind: StepKind, duration_ms: f64) {
        self.kinds.insert(step.clone(), kind);
        self.durations.entry(step).or_default().push(duration_ms);
    }

    fn record_throughput(&mut self, step: &str, eps: f64) {
        self.throughput
            .entry(step.to_string())
            .or_default()
            .push(eps);
    }
}

fn caveats() -> Vec<String> {
    vec![
        "Query steps (toc, bm25, vector, topics, route_query) exclude index build, rollup, and model load.".into(),
        "toc_build is ingest-time MockSummarizer rollup + grip extract + parent-node writes — not TOC navigation.".into(),
        "vector_index includes Candle embedding of TOC bullets; vector_model_load is a one-shot process cost.".into(),
        "bm25_index is Tantivy add+commit; bm25 is search only.".into(),
        "p90 is omitted when samples < 10; p99 is omitted when samples < 30.".into(),
        "Warm mode: one setup, one discarded warmup query, then N query samples. Cold mode: new store per iteration.".into(),
        "The 2026-02-12 latest.json (single.toc p50 ≈ 64.6s, samples=3) measured toc_build, not navigation.".into(),
    ]
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let args = Args::parse();
    fs::create_dir_all(&args.out_dir).map_err(|e| format!("Failed to create out dir: {e}"))?;

    let trace_events = if let Some(path) = &args.trace {
        Some(load_trace_events(path)?)
    } else {
        None
    };

    let event_count = match (trace_events.as_ref(), args.tier) {
        (Some(events), _) => events.len(),
        (None, DatasetTier::Small) => SMALL_EVENT_COUNT,
        (None, DatasetTier::Medium) => MEDIUM_EVENT_COUNT,
    };

    let mut collector = SampleCollector::default();

    let model_start = Instant::now();
    eprintln!("perf_bench: loading embedding model...");
    let _ = get_embedder();
    let model_ms = model_start.elapsed().as_secs_f64() * 1000.0;
    eprintln!("perf_bench: embedding model ready ({model_ms:.0} ms)");
    collector.record("vector_model_load".to_string(), StepKind::Setup, model_ms);

    for scenario in [Scenario::Single, Scenario::Multi] {
        eprintln!(
            "perf_bench: {} / {} starting",
            mode_label(args.mode),
            scenario.label()
        );
        match args.mode {
            RunMode::Warm => {
                let store = prepare_store(
                    args.tier,
                    scenario,
                    0,
                    trace_events.as_deref(),
                    &mut collector,
                )
                .await?;
                // Discard one warmup query so the N samples are warm.
                let _ = run_queries(&store, scenario).await?;
                for _iteration in 0..args.iterations {
                    let query_durations = run_queries(&store, scenario).await?;
                    for (step, ms) in query_durations {
                        collector.record(step, StepKind::Query, ms);
                    }
                }
            }
            RunMode::Cold => {
                for iteration in 0..args.iterations {
                    let store = prepare_store(
                        args.tier,
                        scenario,
                        iteration,
                        trace_events.as_deref(),
                        &mut collector,
                    )
                    .await?;
                    let query_durations = run_queries(&store, scenario).await?;
                    for (step, ms) in query_durations {
                        collector.record(step, StepKind::Query, ms);
                    }
                    drop(store);
                }
            }
        }
    }

    let step_metrics = build_metrics(&collector);
    let mut output = BenchmarkOutput {
        schema_version: SCHEMA_VERSION,
        tier: args.tier,
        mode: args.mode,
        iterations: args.iterations,
        generated_at: Utc::now().to_rfc3339(),
        corpus: CorpusInfo {
            tier: args.tier,
            event_count,
            kind: if args.trace.is_some() {
                "trace".into()
            } else {
                "synthetic".into()
            },
        },
        hardware: HardwareInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        },
        caveats: caveats(),
        steps: step_metrics,
        comparison: None,
    };

    let baseline_path = PathBuf::from("crates/e2e-tests/benchmarks/baseline.json");

    if args.write_baseline {
        let _baseline = update_baseline(&baseline_path, &output)?;
        output.comparison = Some(ComparisonSummary {
            warnings: Vec::new(),
            severe: Vec::new(),
        });
        let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
        write_outputs(&args.out_dir, &json, &render_table(&output))?;
        println!("{}", render_table(&output));
        println!("\n{}", json);
        println!("Baseline updated: {}", baseline_path.display());
        return Ok(());
    }

    let comparison = compare_with_baseline(&baseline_path, &output)?;
    output.comparison = Some(comparison.clone());

    let json = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    let table = render_table(&output);
    write_outputs(&args.out_dir, &json, &table)?;

    println!("{}", table);
    println!("\n{}", json);

    if !comparison.warnings.is_empty() || !comparison.severe.is_empty() {
        println!("\nRegression summary:");
        for warning in &comparison.warnings {
            println!(
                "WARNING: {} {} baseline={} current={}",
                warning.step, warning.metric, warning.baseline, warning.current
            );
        }
        for severe in &comparison.severe {
            println!(
                "SEVERE: {} {} baseline={} current={}",
                severe.step, severe.metric, severe.baseline, severe.current
            );
        }
    }

    if !comparison.severe.is_empty() {
        return Err("Severe regressions detected".to_string());
    }

    Ok(())
}

async fn prepare_store(
    tier: DatasetTier,
    scenario: Scenario,
    iteration: usize,
    trace_events: Option<&[Event]>,
    collector: &mut SampleCollector,
) -> Result<PreparedStore, String> {
    let harness = TestHarness::new();
    let iteration_tag = format!("{}-{}-{}", tier_label(tier), scenario.label(), iteration);
    let base_events = if let Some(trace) = trace_events {
        prepare_trace_events(trace, scenario, &iteration_tag)
    } else {
        synthetic_events(tier, scenario, iteration)
    };

    let prefix = scenario.label();

    let ingest_start = Instant::now();
    ingest_events(&harness.storage, &base_events);
    let ingest_ms = ingest_start.elapsed().as_secs_f64() * 1000.0;
    collector.record(format!("{prefix}.ingest"), StepKind::Setup, ingest_ms);
    collector.record_throughput(
        &format!("{prefix}.ingest"),
        events_per_second(base_events.len(), ingest_ms),
    );

    let toc_build_start = Instant::now();
    let mut toc_node = build_toc_segment(harness.storage.clone(), base_events).await;
    if scenario == Scenario::Multi {
        for agent in ["claude", "copilot"] {
            if !toc_node.contributing_agents.contains(&agent.to_string()) {
                toc_node.contributing_agents.push(agent.to_string());
            }
        }
    }
    collector.record(
        format!("{prefix}.toc_build"),
        StepKind::Setup,
        toc_build_start.elapsed().as_secs_f64() * 1000.0,
    );
    eprintln!(
        "perf_bench: {prefix}.toc_build {:.0} ms",
        toc_build_start.elapsed().as_secs_f64() * 1000.0
    );

    let bm25_start = Instant::now();
    let bm25_searcher = build_bm25_index(&harness, &toc_node)?;
    collector.record(
        format!("{prefix}.bm25_index"),
        StepKind::Setup,
        bm25_start.elapsed().as_secs_f64() * 1000.0,
    );

    let vector_start = Instant::now();
    let vector_handler = build_vector_index(&harness, &toc_node, scenario, iteration).await?;
    collector.record(
        format!("{prefix}.vector_index"),
        StepKind::Setup,
        vector_start.elapsed().as_secs_f64() * 1000.0,
    );

    let topics_start = Instant::now();
    let topic_handler = build_topic_graph(harness.storage.clone(), &toc_node, iteration).await?;
    collector.record(
        format!("{prefix}.topics_index"),
        StepKind::Setup,
        topics_start.elapsed().as_secs_f64() * 1000.0,
    );

    Ok(PreparedStore {
        harness,
        toc_node,
        bm25_searcher,
        vector_handler,
        topic_handler,
    })
}

async fn run_queries(
    store: &PreparedStore,
    scenario: Scenario,
) -> Result<Vec<(String, f64)>, String> {
    let prefix = scenario.label();
    let mut durations = Vec::new();

    let toc_start = Instant::now();
    navigate_toc(store.harness.storage.as_ref(), &store.toc_node);
    durations.push((
        format!("{prefix}.toc"),
        toc_start.elapsed().as_secs_f64() * 1000.0,
    ));

    let bm25_start = Instant::now();
    let _ = store
        .bm25_searcher
        .search("rust memory safety", SearchOptions::new().with_limit(10));
    durations.push((
        format!("{prefix}.bm25"),
        bm25_start.elapsed().as_secs_f64() * 1000.0,
    ));

    let vector_start = Instant::now();
    let _ = store
        .vector_handler
        .search("vector embedding retrieval", 10, 0.0)
        .await;
    durations.push((
        format!("{prefix}.vector"),
        vector_start.elapsed().as_secs_f64() * 1000.0,
    ));

    let topics_start = Instant::now();
    let _ = store
        .topic_handler
        .get_top_topics(Request::new(memory_service::pb::GetTopTopicsRequest {
            limit: 3,
            days: 30,
            agent_filter: None,
        }))
        .await;
    let _ = store
        .topic_handler
        .search_topics("memory retrieval", 5)
        .await;
    durations.push((
        format!("{prefix}.topics"),
        topics_start.elapsed().as_secs_f64() * 1000.0,
    ));

    let route_start = Instant::now();
    run_route_query(
        store.harness.storage.clone(),
        store.bm25_searcher.clone(),
        store.vector_handler.clone(),
        store.topic_handler.clone(),
    )
    .await?;
    durations.push((
        format!("{prefix}.route_query"),
        route_start.elapsed().as_secs_f64() * 1000.0,
    ));

    Ok(durations)
}

fn events_per_second(count: usize, duration_ms: f64) -> f64 {
    if duration_ms <= 0.0 {
        return 0.0;
    }
    count as f64 / (duration_ms / 1000.0)
}

fn load_trace_events(path: &Path) -> Result<Vec<Event>, String> {
    let file = fs::File::open(path).map_err(|e| format!("Trace open failed: {e}"))?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let event: Event = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        events.push(event);
    }
    if events.is_empty() {
        return Err("Trace file produced zero events".to_string());
    }
    Ok(events)
}

fn prepare_trace_events(events: &[Event], scenario: Scenario, tag: &str) -> Vec<Event> {
    events
        .iter()
        .enumerate()
        .map(|(idx, event)| {
            let mut cloned = event.clone();
            cloned.event_id = format!("{}-{}", tag, event.event_id);
            cloned.session_id = format!("{}-{}", event.session_id, tag);
            cloned.agent = Some(match scenario {
                Scenario::Single => "claude".to_string(),
                Scenario::Multi => {
                    if idx % 2 == 0 {
                        "claude".to_string()
                    } else {
                        "copilot".to_string()
                    }
                }
            });
            cloned
        })
        .collect()
}

fn synthetic_events(tier: DatasetTier, scenario: Scenario, iteration: usize) -> Vec<Event> {
    let count = match tier {
        DatasetTier::Small => SMALL_EVENT_COUNT,
        DatasetTier::Medium => MEDIUM_EVENT_COUNT,
    };
    let seed = 1337_u64
        + iteration as u64
        + (match scenario {
            Scenario::Single => 0,
            Scenario::Multi => 10,
        });
    let mut rng = StdRng::seed_from_u64(seed);
    let base_ts: i64 = 1_706_540_400_000;

    let topics = [
        "rust ownership and borrow checker",
        "vector search embeddings",
        "topic graph clustering",
        "bm25 lexical search",
        "multi agent routing",
        "toc navigation summaries",
    ];
    let agents = ["claude", "copilot"];

    (0..count)
        .map(|i| {
            let ts_ms = base_ts + (i as i64 * 100);
            let ulid = ulid::Ulid::from_parts(ts_ms as u64, rng.random());
            let timestamp: DateTime<Utc> = Utc.timestamp_millis_opt(ts_ms).unwrap();
            let topic = topics[rng.random_range(0..topics.len())];
            let detail = rng.random_range(1..1000);
            let text = format!("{} detail {}", topic, detail);
            let (event_type, role) = if i % 2 == 0 {
                (EventType::UserMessage, EventRole::User)
            } else {
                (EventType::AssistantMessage, EventRole::Assistant)
            };

            let mut event = Event::new(
                ulid.to_string(),
                format!("perf-{}-{}", tier_label(tier), scenario.label()),
                timestamp,
                event_type,
                role,
                text,
            );

            event.agent = Some(match scenario {
                Scenario::Single => "claude".to_string(),
                Scenario::Multi => agents[i % agents.len()].to_string(),
            });
            event
        })
        .collect()
}

fn navigate_toc(storage: &memory_storage::Storage, toc_node: &TocNode) {
    let day_id = toc_node.start_time.format("%Y-%m-%d").to_string();
    let year_id = toc_node.start_time.format("%Y").to_string();
    let day_node = format!("toc:day:{day_id}");
    let year_node = format!("toc:year:{year_id}");
    let _ = storage.get_toc_node(&year_node);
    let _ = storage.get_toc_node(&day_node);
    let _ = storage.get_toc_node(&toc_node.node_id);
}

fn build_bm25_index(
    harness: &TestHarness,
    toc_node: &TocNode,
) -> Result<Arc<TeleportSearcher>, String> {
    let bm25_config = SearchIndexConfig::new(&harness.bm25_index_path);
    let bm25_index = SearchIndex::open_or_create(bm25_config).map_err(|e| e.to_string())?;
    let indexer = SearchIndexer::new(&bm25_index).map_err(|e| e.to_string())?;
    indexer
        .index_toc_node(toc_node)
        .map_err(|e| e.to_string())?;

    let grip_ids: Vec<String> = toc_node
        .bullets
        .iter()
        .flat_map(|b| b.grip_ids.iter().cloned())
        .collect();
    for grip_id in &grip_ids {
        if let Some(grip) = harness
            .storage
            .get_grip(grip_id)
            .map_err(|e| e.to_string())?
        {
            indexer.index_grip(&grip).map_err(|e| e.to_string())?;
        }
    }
    indexer.commit().map_err(|e| e.to_string())?;
    TeleportSearcher::new(&bm25_index)
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

async fn build_vector_index(
    harness: &TestHarness,
    toc_node: &TocNode,
    scenario: Scenario,
    iteration: usize,
) -> Result<Arc<VectorTeleportHandler>, String> {
    let embedder = get_embedder();
    let capacity = toc_node.bullets.len().max(10) + 32;
    let hnsw_config = HnswConfig::new(384, &harness.vector_index_path).with_capacity(capacity);
    let mut hnsw_index = HnswIndex::open_or_create(hnsw_config).map_err(|e| e.to_string())?;
    let metadata_path = harness.vector_index_path.join("metadata");
    let metadata = VectorMetadata::open(&metadata_path).map_err(|e| e.to_string())?;

    let mut texts: Vec<(String, Option<String>, i64)> = toc_node
        .bullets
        .iter()
        .map(|bullet| {
            (
                bullet.text.clone(),
                Some(agent_for_scenario(scenario).to_string()),
                toc_node.created_at.timestamp_millis(),
            )
        })
        .collect();
    if texts.is_empty() {
        texts.push((
            toc_node.title.clone(),
            Some(agent_for_scenario(scenario).to_string()),
            toc_node.created_at.timestamp_millis(),
        ));
    }

    for (idx, (text, agent, timestamp_ms)) in texts.iter().enumerate() {
        let vector_id = idx as u64 + 1;
        let embedder_clone = embedder.clone();
        let text_owned = text.clone();
        let embedding = tokio::task::spawn_blocking(move || embedder_clone.embed(&text_owned))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;

        hnsw_index
            .add(vector_id, &embedding)
            .map_err(|e| e.to_string())?;
        let doc_id = format!(
            "toc:segment:perf:{}:{}:{}",
            scenario.label(),
            iteration,
            idx
        );
        let entry = VectorEntry::new(vector_id, DocType::TocNode, doc_id, *timestamp_ms, text)
            .with_agent(agent.clone());
        metadata.put(&entry).map_err(|e| e.to_string())?;
    }

    let index_lock = Arc::new(std::sync::RwLock::new(hnsw_index));
    let metadata = Arc::new(metadata);
    Ok(Arc::new(VectorTeleportHandler::new(
        embedder, index_lock, metadata,
    )))
}

async fn build_topic_graph(
    storage: Arc<memory_storage::Storage>,
    toc_node: &TocNode,
    iteration: usize,
) -> Result<Arc<TopicGraphHandler>, String> {
    let topic_storage = TopicStorage::new(storage.clone());
    let fallback_topics = [
        "memory",
        "retrieval",
        "toc",
        "teleport",
        "topics",
        "routing",
    ];
    let mut idx = 0;
    let keywords = if toc_node.keywords.is_empty() {
        fallback_topics.iter().map(|s| s.to_string()).collect()
    } else {
        toc_node.keywords.clone()
    };
    for keyword in &keywords {
        let topic = create_topic(
            &format!("topic-{}-{}", iteration, idx),
            keyword,
            &[keyword.as_str()],
            0.5 + (idx as f64 * 0.01),
        );
        topic_storage
            .save_topic(&topic)
            .map_err(|e| e.to_string())?;
        idx += 1;
        if idx >= 5 {
            break;
        }
    }

    Ok(Arc::new(TopicGraphHandler::new(
        Arc::new(topic_storage),
        storage,
    )))
}

async fn run_route_query(
    storage: Arc<memory_storage::Storage>,
    bm25_searcher: Arc<TeleportSearcher>,
    vector_handler: Arc<VectorTeleportHandler>,
    topic_handler: Arc<TopicGraphHandler>,
) -> Result<(), String> {
    let handler = RetrievalHandler::with_services(
        storage,
        Some(bm25_searcher),
        Some(vector_handler),
        Some(topic_handler),
        Default::default(),
    );
    let _ = handler
        .route_query(Request::new(RouteQueryRequest {
            query: "memory safety retrieval".to_string(),
            intent_override: None,
            stop_conditions: None,
            mode_override: None,
            limit: 10,
            agent_filter: None,
            all_projects: false,
            rerank_mode: None,
            expand_query: false,
        }))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn create_topic(id: &str, label: &str, keywords: &[&str], importance: f64) -> Topic {
    let mut topic = Topic::new(id.to_string(), label.to_string(), vec![0.0_f32; 384]);
    topic.importance_score = importance;
    topic.keywords = keywords.iter().map(|k| k.to_string()).collect();
    topic.status = TopicStatus::Active;
    topic
}

fn build_metrics(collector: &SampleCollector) -> BTreeMap<String, StepMetrics> {
    let mut steps = BTreeMap::new();
    for (step, durations) in &collector.durations {
        let kind = collector
            .kinds
            .get(step)
            .copied()
            .unwrap_or(StepKind::Query);
        let stats = summarize_samples(durations);
        let throughput = collector.throughput.get(step).map(|values| {
            let t = summarize_samples(values);
            ThroughputMetrics {
                p50_eps: t.p50,
                p90_eps: t.p90,
                p99_eps: t.p99,
            }
        });

        steps.insert(
            step.clone(),
            StepMetrics {
                kind,
                samples: stats.samples,
                min_ms: stats.min,
                p50_ms: stats.p50,
                max_ms: stats.max,
                p90_ms: stats.p90,
                p99_ms: stats.p99,
                throughput_eps: throughput,
            },
        );
    }
    steps
}

struct SampleStats {
    samples: usize,
    min: f64,
    p50: f64,
    max: f64,
    p90: Option<f64>,
    p99: Option<f64>,
}

fn summarize_samples(values: &[f64]) -> SampleStats {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    SampleStats {
        samples: n,
        min: sorted.first().copied().unwrap_or(0.0),
        p50: percentile(&sorted, 50.0).unwrap_or(0.0),
        max: sorted.last().copied().unwrap_or(0.0),
        p90: if n >= MIN_SAMPLES_P90 {
            percentile(&sorted, 90.0)
        } else {
            None
        },
        p99: if n >= MIN_SAMPLES_P99 {
            percentile(&sorted, 99.0)
        } else {
            None
        },
    }
}

/// Linear-interpolated percentile. `values` must be sorted ascending.
fn percentile(values: &[f64], percentile: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let rank = (percentile / 100.0) * (values.len() as f64 - 1.0);
    let low = rank.floor() as usize;
    let high = rank.ceil() as usize;
    if low == high {
        Some(values[low])
    } else {
        let weight = rank - low as f64;
        Some(values[low] + (values[high] - values[low]) * weight)
    }
}

fn render_table(output: &BenchmarkOutput) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "Benchmark Results (schema={}, tier={}, mode={}, iterations={}, corpus={} {}, {}/{})",
        output.schema_version,
        tier_label(output.tier),
        mode_label(output.mode),
        output.iterations,
        output.corpus.event_count,
        output.corpus.kind,
        output.hardware.os,
        output.hardware.arch
    ));
    lines.push(
        "kind\tstep\tsamples\tmin_ms\tp50_ms\tp90_ms\tp99_ms\tmax_ms\tthroughput_eps".to_string(),
    );

    for (step, metrics) in &output.steps {
        let kind = match metrics.kind {
            StepKind::Setup => "setup",
            StepKind::Query => "query",
        };
        let p90 = metrics
            .p90_ms
            .map(|v| format!("{v:.2}"))
            .unwrap_or_else(|| "-".into());
        let p99 = metrics
            .p99_ms
            .map(|v| format!("{v:.2}"))
            .unwrap_or_else(|| "-".into());
        let throughput = metrics
            .throughput_eps
            .as_ref()
            .map(|t| format!("{:.2}", t.p50_eps))
            .unwrap_or_else(|| "-".to_string());
        lines.push(format!(
            "{kind}\t{step}\t{}\t{:.2}\t{:.2}\t{p90}\t{p99}\t{:.2}\t{throughput}",
            metrics.samples, metrics.min_ms, metrics.p50_ms, metrics.max_ms
        ));
    }
    lines.join("\n")
}

fn write_outputs(out_dir: &Path, json: &str, table: &str) -> Result<(), String> {
    fs::write(out_dir.join("latest.json"), json).map_err(|e| e.to_string())?;
    fs::write(out_dir.join("latest.txt"), table).map_err(|e| e.to_string())?;
    Ok(())
}

fn compare_with_baseline(
    baseline_path: &Path,
    output: &BenchmarkOutput,
) -> Result<ComparisonSummary, String> {
    if !baseline_path.exists() {
        return Ok(ComparisonSummary {
            warnings: Vec::new(),
            severe: Vec::new(),
        });
    }
    let baseline_data = fs::read_to_string(baseline_path).map_err(|e| e.to_string())?;
    let baseline: BaselineFile = match serde_json::from_str::<BaselineFile>(&baseline_data) {
        Ok(b) if b.version == SCHEMA_VERSION => b,
        _ => {
            // v1 files mixed setup+query under query names; do not compare.
            return Ok(ComparisonSummary {
                warnings: Vec::new(),
                severe: Vec::new(),
            });
        }
    };

    let run = baseline
        .runs
        .iter()
        .find(|r| r.tier == output.tier && r.mode == output.mode);

    let Some(run) = run else {
        return Ok(ComparisonSummary {
            warnings: Vec::new(),
            severe: Vec::new(),
        });
    };

    let mut warnings = Vec::new();
    let mut severe = Vec::new();

    for (step, current) in &output.steps {
        if current.kind != StepKind::Query {
            continue;
        }
        let Some(baseline_step) = run.steps.get(step) else {
            continue;
        };

        apply_duration_regression(
            step,
            "p50_ms",
            baseline_step.p50_ms,
            current.p50_ms,
            &baseline.thresholds,
            &mut warnings,
            &mut severe,
        );

        if let (Some(base_tp), Some(cur_tp)) =
            (&baseline_step.throughput_eps, &current.throughput_eps)
        {
            apply_throughput_regression(
                step,
                "throughput_p50_eps",
                base_tp.p50_eps,
                cur_tp.p50_eps,
                &baseline.thresholds,
                &mut warnings,
                &mut severe,
            );
        }
    }

    Ok(ComparisonSummary { warnings, severe })
}

fn apply_duration_regression(
    step: &str,
    metric: &str,
    baseline: f64,
    current: f64,
    thresholds: &Thresholds,
    warnings: &mut Vec<Regression>,
    severe: &mut Vec<Regression>,
) {
    if baseline == 0.0 {
        return;
    }
    let delta = current - baseline;
    let ratio = (current / baseline) - 1.0;

    if delta >= thresholds.severe.absolute_ms || ratio >= thresholds.severe.relative {
        severe.push(Regression {
            step: step.to_string(),
            metric: metric.to_string(),
            baseline,
            current,
            severity: "severe".to_string(),
        });
        return;
    }

    if delta >= thresholds.warning.absolute_ms || ratio >= thresholds.warning.relative {
        warnings.push(Regression {
            step: step.to_string(),
            metric: metric.to_string(),
            baseline,
            current,
            severity: "warning".to_string(),
        });
    }
}

fn apply_throughput_regression(
    step: &str,
    metric: &str,
    baseline: f64,
    current: f64,
    thresholds: &Thresholds,
    warnings: &mut Vec<Regression>,
    severe: &mut Vec<Regression>,
) {
    if baseline == 0.0 {
        return;
    }
    let delta = baseline - current;
    let ratio = 1.0 - (current / baseline);

    if delta >= thresholds.severe.throughput_absolute
        || ratio >= thresholds.severe.throughput_relative
    {
        severe.push(Regression {
            step: step.to_string(),
            metric: metric.to_string(),
            baseline,
            current,
            severity: "severe".to_string(),
        });
        return;
    }

    if delta >= thresholds.warning.throughput_absolute
        || ratio >= thresholds.warning.throughput_relative
    {
        warnings.push(Regression {
            step: step.to_string(),
            metric: metric.to_string(),
            baseline,
            current,
            severity: "warning".to_string(),
        });
    }
}

fn update_baseline(path: &Path, output: &BenchmarkOutput) -> Result<BaselineFile, String> {
    let mut baseline = if path.exists() {
        let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
        match serde_json::from_str::<BaselineFile>(&data) {
            Ok(existing) if existing.version == SCHEMA_VERSION => existing,
            _ => BaselineFile {
                baseline: default_baseline_label(),
                version: SCHEMA_VERSION,
                thresholds: default_thresholds(),
                runs: Vec::new(),
            },
        }
    } else {
        BaselineFile {
            baseline: default_baseline_label(),
            version: SCHEMA_VERSION,
            thresholds: default_thresholds(),
            runs: Vec::new(),
        }
    };

    if baseline.baseline.is_empty() {
        baseline.baseline = default_baseline_label();
    }
    baseline.version = SCHEMA_VERSION;

    let run = BaselineRun {
        tier: output.tier,
        mode: output.mode,
        steps: output.steps.clone(),
    };

    if let Some(existing) = baseline
        .runs
        .iter_mut()
        .find(|r| r.tier == output.tier && r.mode == output.mode)
    {
        existing.steps = run.steps;
    } else {
        baseline.runs.push(run);
    }

    let json = serde_json::to_string_pretty(&baseline).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(baseline)
}

fn default_thresholds() -> Thresholds {
    Thresholds {
        warning: Threshold {
            relative: 0.15,
            absolute_ms: 25.0,
            throughput_relative: 0.15,
            throughput_absolute: 50.0,
        },
        severe: Threshold {
            relative: 0.30,
            absolute_ms: 50.0,
            throughput_relative: 0.30,
            throughput_absolute: 100.0,
        },
    }
}

fn default_baseline_label() -> String {
    "perf_bench".to_string()
}

fn get_embedder() -> Arc<CandleEmbedder> {
    EMBEDDER
        .get_or_init(|| {
            let cache = memory_embeddings::ModelCache::default();
            eprintln!(
                "perf_bench: model cache dir = {} (cached={})",
                cache.model_dir().display(),
                cache.is_cached()
            );
            let embedder = CandleEmbedder::load(&cache).expect("Failed to load embedding model");
            Arc::new(embedder)
        })
        .clone()
}

fn tier_label(tier: DatasetTier) -> &'static str {
    match tier {
        DatasetTier::Small => "small",
        DatasetTier::Medium => "medium",
    }
}

fn mode_label(mode: RunMode) -> &'static str {
    match mode {
        RunMode::Cold => "cold",
        RunMode::Warm => "warm",
    }
}

fn agent_for_scenario(scenario: Scenario) -> &'static str {
    match scenario {
        Scenario::Single => "claude",
        Scenario::Multi => "copilot",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_empty_is_none() {
        assert_eq!(percentile(&[], 50.0), None);
    }

    #[test]
    fn percentile_single_value() {
        assert_eq!(percentile(&[10.0], 50.0), Some(10.0));
        assert_eq!(percentile(&[10.0], 99.0), Some(10.0));
    }

    #[test]
    fn summarize_omits_p90_below_10_samples() {
        let stats = summarize_samples(&[1.0, 2.0, 3.0]);
        assert_eq!(stats.samples, 3);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 3.0);
        assert_eq!(stats.p50, 2.0);
        assert!(stats.p90.is_none());
        assert!(stats.p99.is_none());
    }

    #[test]
    fn summarize_includes_p90_at_10_samples() {
        let values: Vec<f64> = (1..=10).map(|i| i as f64).collect();
        let stats = summarize_samples(&values);
        assert!(stats.p90.is_some());
        assert!(stats.p99.is_none());
    }

    #[test]
    fn summarize_includes_p99_at_30_samples() {
        let values: Vec<f64> = (1..=30).map(|i| i as f64).collect();
        let stats = summarize_samples(&values);
        assert!(stats.p90.is_some());
        assert!(stats.p99.is_some());
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 30.0);
    }

    #[test]
    fn three_sample_p99_would_be_an_artifact() {
        // The retired 2026-02-12 methodology interpolated p99 from 3 samples.
        let sorted = [64576.0, 65000.0, 65451.0];
        let fake_p99 = percentile(&sorted, 99.0).unwrap();
        assert!(
            (fake_p99 - sorted[2]).abs() < 50.0,
            "p99 of 3 samples collapses onto max ({fake_p99} vs {})",
            sorted[2]
        );
        let honest = summarize_samples(&sorted);
        assert!(honest.p99.is_none());
    }
}
