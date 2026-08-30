use clap::Parser;
use std::path::Path;

mod cli;

use memory_bench::judge::{ApiJudge, Judge, MockJudge, ScorerKind};
use memory_bench::runner::{BackendKind, MockStore, RunConfig};
use memory_bench::{baseline, fixture, locomo, report, runner, scorer};
use scorer::BenchmarkReport;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    let backend = BackendKind::parse(&cli.backend)?;
    let config = RunConfig {
        memory_bin: cli.memory_bin.clone(),
        endpoint: cli.endpoint.clone(),
        backend,
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
        } => {
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
        cli::Commands::Locomo {
            dataset,
            output,
            scorer,
            top,
            compare,
            baselines: _baselines,
        } => {
            let kind = ScorerKind::parse(&scorer)?;
            if compare && kind == ScorerKind::Mock {
                anyhow::bail!(
                    "--compare refused for mock scorer: context_hit_rate is not a LOCOMO score \
                     and must not share a table with published LLM-judge numbers"
                );
            }
            let judge: Box<dyn Judge> = match kind {
                ScorerKind::Mock => Box::new(MockJudge),
                ScorerKind::LlmJudge => Box::new(ApiJudge::from_env()?),
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
    let conversations = locomo::load_dataset(Path::new(dataset))?;
    eprintln!(
        "Loaded {} conversations from {} (backend={} scorer={})",
        conversations.len(),
        dataset,
        config.backend.as_str(),
        kind.metric_name()
    );

    let mut results = Vec::new();
    for conv in &conversations {
        // Fresh store per conversation — no shared-store bleed.
        match config.backend {
            BackendKind::Mock => {
                let store = locomo::ingest_sample_mock(conv);
                results.push(locomo::evaluate_sample(conv, &store, judge, top));
            }
            BackendKind::Cli => {
                locomo::ingest_sample_cli(conv, config)?;
                results.push(evaluate_sample_cli(conv, config, judge, top)?);
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
    ))
}

fn evaluate_sample_cli(
    sample: &locomo::LocomoSample,
    config: &RunConfig,
    judge: &dyn Judge,
    top: usize,
) -> anyhow::Result<locomo::LocomoConversationResult> {
    // Build a one-shot mock store from CLI retrieval *per question* by
    // stuffing CLI hits into evaluate_sample would mix questions. Do it
    // question-by-question and reuse locomo types.
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

/// Run all benchmark categories and aggregate into one report.
fn run_all(fixtures_dir: &str, config: &RunConfig) -> anyhow::Result<BenchmarkReport> {
    let tests = fixture::Fixture::load_dir(Path::new(fixtures_dir))?;
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
                store.search(&test.query, test.k.max(5))
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
        caveats.push(
            "backend=mock uses in-process token-overlap retrieval; not a production quality number"
                .into(),
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
