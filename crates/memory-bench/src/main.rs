use clap::Parser;
use std::path::Path;

mod cli;

use memory_bench::judge::{ApiJudge, Judge, MockJudge, ScorerKind};
use memory_bench::layers::RetrievalLayer;
use memory_bench::runner::{BackendKind, IsolatedDaemon, Isolation, MockStore, RunConfig};
use memory_bench::{baseline, fixture, locomo, report, runner, scorer};
use scorer::BenchmarkReport;

fn resolve_bin(configured: &str) -> String {
    let p = Path::new(configured);
    if p.is_file() {
        return configured.to_string();
    }
    runner::find_bin(configured)
        .map(|pb| pb.to_string_lossy().into_owned())
        .unwrap_or_else(|| configured.to_string())
}

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    let backend = BackendKind::parse(&cli.backend)?;
    let config = RunConfig {
        memory_bin: resolve_bin(&cli.memory_bin),
        daemon_bin: resolve_bin(&cli.daemon_bin),
        endpoint: cli.endpoint.clone(),
        backend,
        isolation: Isolation::Shared,
        limit_questions: None,
        layer: RetrievalLayer::Bm25,
    };

    match cli.command {
        cli::Commands::Temporal { fixtures, output } => {
            let report = run_category("temporal", &fixtures, &config)?;
            print_report(&report, output.as_deref())?;
        }
        cli::Commands::Multisession { fixtures, output } => {
            let report = run_category("multi", &fixtures, &config)?;
            print_report(&report, output.as_deref())?;
        }
        cli::Commands::Compression { fixtures, output } => {
            let report = run_category("compress", &fixtures, &config)?;
            print_report(&report, output.as_deref())?;
        }
        cli::Commands::All {
            fixtures,
            output,
            compare,
            baselines,
            layers,
        } => {
            let mut config = config;
            config.layer = RetrievalLayer::parse(&layers)?;
            let bench_report = run_all(&fixtures, &config)?;
            let baselines_data = if compare {
                Some(baseline::Baselines::load(Path::new(&baselines))?)
            } else {
                None
            };
            let json = report::to_json(&bench_report);
            let md = report::to_markdown(&bench_report, baselines_data.as_ref());
            println!("{md}");
            if let Some(path) = output {
                std::fs::write(&path, &json)?;
                eprintln!("Results written to {path}");
            }
        }
        cli::Commands::Run {
            fixtures,
            output,
            category,
            layers,
        } => {
            let mut config = config;
            config.layer = RetrievalLayer::parse(&layers)?;
            let bench_report = match category {
                Some(prefix) => run_category(&prefix, &fixtures, &config)?,
                None => run_all(&fixtures, &config)?,
            };
            print_report(&bench_report, output.as_deref())?;
        }
        cli::Commands::Semantic {
            fixtures,
            output,
            layers,
        } => {
            let mut config = config;
            config.layer = RetrievalLayer::parse(&layers)?;
            let bench_report = run_category("semantic", &fixtures, &config)?;
            print_report(&bench_report, output.as_deref())?;
        }
        cli::Commands::Locomo {
            dataset,
            output,
            scorer,
            top,
            compare,
            baselines: _baselines,
            isolation,
            limit_questions,
        } => {
            let kind = ScorerKind::parse(&scorer)?;
            if compare && kind == ScorerKind::Mock {
                anyhow::bail!(
                    "--compare refused for mock scorer: context_hit_rate is not a LOCOMO score \
                     and must not share a table with published LLM-judge numbers"
                );
            }
            let isolation = match isolation.as_deref() {
                Some(s) => Isolation::parse(s)?,
                None if backend == BackendKind::Cli => Isolation::DaemonPerConversation,
                None => Isolation::Shared,
            };
            if backend == BackendKind::Cli && isolation == Isolation::Shared {
                eprintln!(
                    "warning: --isolation shared: conversation N can retrieve 1..=N-1 \
                     (cross-conversation bleed). Do not commit this number."
                );
            }
            let judge: Box<dyn Judge> = match kind {
                ScorerKind::Mock => Box::new(MockJudge),
                ScorerKind::LlmJudge => Box::new(ApiJudge::from_env()?),
            };
            let config = RunConfig {
                isolation,
                limit_questions,
                ..config
            };
            let aggregate = run_locomo(&dataset, judge.as_ref(), kind, top, &config)?;
            let json = serde_json::to_string_pretty(&aggregate)?;
            println!("{json}");
            if let Some(path) = output {
                std::fs::write(&path, &json)?;
                eprintln!("Results written to {path}");
            }
        }
        cli::Commands::Smoke { dataset, output } => {
            let judge = MockJudge;
            let aggregate = run_locomo(
                &dataset,
                &judge,
                ScorerKind::Mock,
                5,
                &RunConfig {
                    backend: BackendKind::Mock,
                    ..config
                },
            )?;
            if aggregate.conversations != 1 {
                anyhow::bail!(
                    "smoke fixture must contain exactly 1 conversation, got {}",
                    aggregate.conversations
                );
            }
            if aggregate.total_questions == 0 {
                anyhow::bail!(
                    "smoke fixture produced 0 questions — parse/ingest/score pipeline did not run"
                );
            }
            eprintln!(
                "smoke ok: 1 conversation, {} questions, metric={}",
                aggregate.total_questions, aggregate.metric
            );
            let json = serde_json::to_string_pretty(&aggregate)?;
            println!("{json}");
            if let Some(path) = output {
                std::fs::write(&path, &json)?;
                eprintln!("Results written to {path}");
            }
        }
    }
    Ok(())
}

fn run_locomo(
    dataset: &str,
    judge: &dyn Judge,
    kind: ScorerKind,
    top: usize,
    config: &RunConfig,
) -> anyhow::Result<locomo::LocomoAggregateResult> {
    let mut conversations = locomo::load_dataset(Path::new(dataset))?;
    if let Some(limit) = config.limit_questions {
        let mut left = limit;
        for conv in &mut conversations {
            if left == 0 {
                conv.qa.clear();
            } else if conv.qa.len() > left {
                conv.qa.truncate(left);
                left = 0;
            } else {
                left -= conv.qa.len();
            }
        }
        conversations.retain(|c| !c.qa.is_empty());
    }
    eprintln!(
        "Loaded {} conversations from {} (backend={} scorer={} isolation={})",
        conversations.len(),
        dataset,
        config.backend.as_str(),
        kind.metric_name(),
        config.isolation.result_label(config.backend),
    );

    let mut results = Vec::new();
    for conv in &conversations {
        match config.backend {
            BackendKind::Mock => {
                let store = locomo::ingest_sample_mock(conv);
                results.push(locomo::evaluate_sample(conv, &store, judge, top));
            }
            BackendKind::Cli => {
                results.push(run_locomo_cli_conversation(conv, config, judge, top)?);
            }
        }
    }

    let (judge_label, model, temperature) = match kind {
        ScorerKind::Mock => ("mock".to_string(), None, None),
        ScorerKind::LlmJudge => {
            let model =
                std::env::var("MEMORY_BENCH_JUDGE_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into());
            ("api".to_string(), Some(model), Some(0.0))
        }
    };

    Ok(locomo::aggregate_results(
        &results,
        kind,
        dataset,
        &judge_label,
        model,
        temperature,
        config.isolation.result_label(config.backend),
    ))
}

fn run_locomo_cli_conversation(
    conv: &locomo::LocomoSample,
    config: &RunConfig,
    judge: &dyn Judge,
    top: usize,
) -> anyhow::Result<locomo::LocomoConversationResult> {
    match config.isolation {
        Isolation::DaemonPerConversation => {
            let daemon = IsolatedDaemon::spawn(&config.daemon_bin)?;
            let mut isolated = config.clone();
            isolated.endpoint = daemon.endpoint.clone();
            locomo::ingest_sample_cli(conv, &isolated)?;
            let drain_wait_ms = runner::wait_for_drain(&isolated.daemon_bin, &isolated.endpoint)?;
            let mut result = evaluate_sample_cli(conv, &isolated, judge, top)?;
            result.drain_wait_ms = drain_wait_ms;
            daemon.stop()?;
            Ok(result)
        }
        Isolation::Shared => {
            locomo::ingest_sample_cli(conv, config)?;
            let drain_wait_ms = runner::wait_for_drain(&config.daemon_bin, &config.endpoint)?;
            let mut result = evaluate_sample_cli(conv, config, judge, top)?;
            result.drain_wait_ms = drain_wait_ms;
            Ok(result)
        }
    }
}

fn evaluate_sample_cli(
    sample: &locomo::LocomoSample,
    config: &RunConfig,
    judge: &dyn Judge,
    top: usize,
) -> anyhow::Result<locomo::LocomoConversationResult> {
    use locomo::{QuestionResult, TypeScore};
    use std::collections::HashMap;

    let mut questions = Vec::new();
    let mut by_type: HashMap<String, (usize, usize)> = HashMap::new();
    let mut correct_n = 0usize;
    for qa in &sample.qa {
        let retrieved = runner::run_query_cli(&qa.question, config, top)?;
        let context = retrieved
            .ranked
            .iter()
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let predicted = judge.generate_answer(&qa.question, &context)?;
        let verdict = judge.judge(&qa.question, &qa.answer, &predicted, &context)?;
        if verdict.correct {
            correct_n += 1;
        }
        let cat = locomo::category_name(qa.category).to_string();
        let entry = by_type.entry(cat.clone()).or_insert((0, 0));
        entry.0 += 1;
        if verdict.correct {
            entry.1 += 1;
        }
        questions.push(QuestionResult {
            question: qa.question.clone(),
            gold: qa.answer.clone(),
            category: cat,
            category_id: qa.category,
            predicted: predicted.chars().take(500).collect(),
            correct: verdict.correct,
            rationale: verdict.rationale,
            latency_ms: retrieved.latency_ms,
            context_tokens: retrieved.tokens_estimated,
        });
    }
    let total = sample.qa.len();
    let by_type = by_type
        .into_iter()
        .map(|(k, (t, c))| {
            (
                k,
                TypeScore {
                    total: t,
                    correct: c,
                    score: if t == 0 { 0.0 } else { c as f64 / t as f64 },
                },
            )
        })
        .collect();
    Ok(locomo::LocomoConversationResult {
        sample_id: sample.sample_id.clone(),
        total_questions: total,
        correct: correct_n,
        score: if total == 0 {
            0.0
        } else {
            correct_n as f64 / total as f64
        },
        by_type,
        questions,
        drain_wait_ms: 0,
    })
}

/// Run benchmarks for a single category by filtering test case IDs by prefix.
fn run_category(
    category_prefix: &str,
    fixtures_dir: &str,
    config: &RunConfig,
) -> anyhow::Result<BenchmarkReport> {
    let all_tests = fixture::Fixture::load_dir(Path::new(fixtures_dir))?;
    let tests: Vec<_> = all_tests
        .into_iter()
        .filter(|t| {
            t.id.starts_with(category_prefix)
                || t.category
                    .as_deref()
                    .is_some_and(|c| c.starts_with(category_prefix))
        })
        .collect();

    run_tests(&tests, Path::new(fixtures_dir), config)
}

fn is_semantic(t: &fixture::TestCase) -> bool {
    t.id.starts_with("semantic")
        || t.category
            .as_deref()
            .is_some_and(|c| c.starts_with("semantic"))
}

/// Run all benchmark categories except semantic (paraphrase set tanks BM25).
fn run_all(fixtures_dir: &str, config: &RunConfig) -> anyhow::Result<BenchmarkReport> {
    let tests: Vec<_> = fixture::Fixture::load_dir(Path::new(fixtures_dir))?
        .into_iter()
        .filter(|t| !is_semantic(t))
        .collect();
    run_tests(&tests, Path::new(fixtures_dir), config)
}

/// Execute a set of test cases. Each test gets a fresh mock store.
fn run_tests(
    tests: &[fixture::TestCase],
    fixtures_dir: &Path,
    config: &RunConfig,
) -> anyhow::Result<BenchmarkReport> {
    let mut hits = Vec::new();
    let mut failed_ids = Vec::new();
    let mut latencies = Vec::new();
    let mut total_tokens = 0usize;
    let mut compression_ratios = Vec::new();
    let mut recalls = Vec::new();
    let mut k = 5usize;

    for test in tests {
        k = test.k;
        let result = match config.backend {
            BackendKind::Mock => {
                let mut store = MockStore::new();
                for setup_path in &test.setup {
                    let path = runner::resolve_setup(fixtures_dir, setup_path);
                    store.ingest_file(&path)?;
                }
                store.search_with_layer(&test.query, test.k.max(5), config.layer)
            }
            BackendKind::Cli => {
                for setup_path in &test.setup {
                    let path = runner::resolve_setup(fixtures_dir, setup_path);
                    runner::ingest_session_cli(path.to_str().unwrap_or_default(), config)?;
                }
                runner::run_query_cli(&test.query, config, test.k.max(5))?
            }
        };

        let ranked_texts: Vec<String> = result.ranked.iter().map(|h| h.text.clone()).collect();
        let blob = ranked_texts.join("\n");
        let hit = scorer::score_result(&blob, &test.expected_contains);
        hits.push(hit);
        if !hit {
            failed_ids.push(test.id.clone());
        }
        latencies.push(result.latency_ms);
        total_tokens += result.tokens_estimated;

        if let Some(r) = scorer::compute_recall_at_k(&ranked_texts, &test.relevant, test.k) {
            recalls.push(r);
        }

        let paths: Vec<_> = test
            .setup
            .iter()
            .map(|s| runner::resolve_setup(fixtures_dir, s))
            .collect();
        let raw_tokens = scorer::estimate_raw_tokens_from_files(&paths)?;
        if raw_tokens > 0 {
            compression_ratios.push(scorer::compute_compression_ratio(
                result.tokens_estimated,
                raw_tokens,
            ));
        }
    }

    latencies.sort();

    let test_count = tests.len();
    let pass_count = hits.iter().filter(|&&h| h).count();
    let accuracy = scorer::compute_accuracy(&hits);
    let recall_at_k = if recalls.is_empty() {
        None
    } else {
        Some(recalls.iter().sum::<f64>() / recalls.len() as f64)
    };
    let token_usage_avg = total_tokens.checked_div(test_count).unwrap_or(0);
    let latency_p50_ms = scorer::percentile(&latencies, 50.0);
    let latency_p95_ms = scorer::percentile(&latencies, 95.0);
    let compression_ratio = if compression_ratios.is_empty() {
        0.0
    } else {
        compression_ratios.iter().sum::<f64>() / compression_ratios.len() as f64
    };

    let mut caveats = vec![
        "accuracy is expected_contains over this fixture suite, not LOCOMO".into(),
        "recall@k uses labeled relevant items in top-k, not accuracy under another name".into(),
        "compression_ratio compares retrieved tokens to setup file contents (not path strings)"
            .into(),
        "each test ran against a fresh store (mock isolation; cli shares the daemon unless you restart it)"
            .into(),
    ];
    if config.backend == BackendKind::Mock {
        match config.layer {
            RetrievalLayer::Bm25 => caveats.push(
                "backend=mock uses in-process token-overlap retrieval; not a production quality number"
                    .into(),
            ),
            RetrievalLayer::Vector => caveats.push(
                "backend=mock vector is a committed paraphrase lexicon plus TF-IDF cosine, not Candle/HNSW"
                    .into(),
            ),
            RetrievalLayer::Hybrid => caveats.push(
                "backend=mock hybrid is RRF (k=60) of token-overlap and lexicon TF-IDF; not RouteQuery"
                    .into(),
            ),
        }
        caveats.push(
            "--layers is a mock-backend switch; CLI search is always RouteQuery hybrid".into(),
        );
    }

    Ok(BenchmarkReport {
        backend: config.backend.as_str().to_string(),
        accuracy,
        recall_at_k,
        k,
        token_usage_avg,
        latency_p50_ms,
        latency_p95_ms,
        compression_ratio,
        test_count,
        pass_count,
        failed_ids,
        caveats,
        layers: config.layer.as_str().to_string(),
    })
}

/// Print a report as markdown to stdout and optionally write JSON to file.
fn print_report(bench_report: &BenchmarkReport, output: Option<&str>) -> anyhow::Result<()> {
    let md = report::to_markdown(bench_report, None);
    println!("{md}");
    if let Some(path) = output {
        let json = report::to_json(bench_report);
        std::fs::write(path, &json)?;
        eprintln!("Results written to {path}");
    }
    Ok(())
}
