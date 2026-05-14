use clap::Parser;

mod cli;

use memory_bench::{baseline, fixture, locomo, report, runner, scorer};
use scorer::BenchmarkReport;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    let config = runner::RunConfig {
        memory_bin: cli.memory_bin.clone(),
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
                Some(baseline::Baselines::load(std::path::Path::new(&baselines))?)
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
            compare,
            baselines,
        } => {
            let conversations = locomo::load_dataset(std::path::Path::new(&dataset))?;
            eprintln!(
                "Loaded {} conversations from {}",
                conversations.len(),
                dataset
            );

            let mut results = Vec::new();
            for conv in &conversations {
                // Convert turns to JSONL and ingest via runner
                let temp_dir = tempfile::tempdir()?;
                let session_path = temp_dir.path().join("session.jsonl");
                let mut lines = Vec::new();
                for turn in &conv.turns {
                    lines.push(format!(
                        "{{\"role\":\"{}\",\"content\":\"{}\"}}",
                        turn.role,
                        turn.content.replace('\\', "\\\\").replace('"', "\\\"")
                    ));
                }
                std::fs::write(&session_path, lines.join("\n"))?;
                let _ = runner::ingest_session(session_path.to_str().unwrap_or_default(), &config);

                // Run each question through runner and collect answers
                let mut answers = Vec::new();
                for q in &conv.questions {
                    let result = runner::run_query(&q.question, &config);
                    answers.push(result.raw_output);
                }

                let result = locomo::score_conversation(conv, &answers);
                results.push(result);
            }

            let aggregate = locomo::aggregate_results(&results);

            if compare {
                let _baselines_data = baseline::Baselines::load(std::path::Path::new(&baselines))?;
                eprintln!("Loaded baselines for comparison");
            }

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

/// Run benchmarks for a single category by filtering test case IDs by prefix.
fn run_category(
    category_prefix: &str,
    fixtures_dir: &str,
    config: &runner::RunConfig,
) -> anyhow::Result<BenchmarkReport> {
    let all_tests = fixture::Fixture::load_dir(std::path::Path::new(fixtures_dir))?;
    let tests: Vec<_> = all_tests
        .into_iter()
        .filter(|t| t.id.starts_with(category_prefix))
        .collect();

    run_tests(&tests, config)
}

/// Run all benchmark categories and aggregate into one report.
fn run_all(fixtures_dir: &str, config: &runner::RunConfig) -> anyhow::Result<BenchmarkReport> {
    let tests = fixture::Fixture::load_dir(std::path::Path::new(fixtures_dir))?;
    run_tests(&tests, config)
}

/// Execute a set of test cases and produce a benchmark report.
fn run_tests(
    tests: &[fixture::TestCase],
    config: &runner::RunConfig,
) -> anyhow::Result<BenchmarkReport> {
    let mut hits = Vec::new();
    let mut latencies = Vec::new();
    let mut total_tokens = 0usize;
    let mut compression_ratios = Vec::new();

    for test in tests {
        // Ingest setup session files
        for setup_path in &test.setup {
            let _ = runner::ingest_session(setup_path, config);
        }

        // Run the query
        let result = runner::run_query(&test.query, config);
        let hit = scorer::score_result(&result.raw_output, &test.expected_contains);
        hits.push(hit);
        latencies.push(result.latency_ms);
        total_tokens += result.tokens_estimated;

        // Compute compression ratio
        let raw_tokens = scorer::estimate_raw_tokens(&test.setup);
        if raw_tokens > 0 {
            let ratio = scorer::compute_compression_ratio(result.tokens_estimated, raw_tokens);
            compression_ratios.push(ratio);
        }
    }

    latencies.sort();

    let test_count = tests.len();
    let pass_count = hits.iter().filter(|&&h| h).count();
    let accuracy = scorer::compute_accuracy(&hits);
    let recall_at_5 = scorer::compute_recall_at_k(&hits, test_count);
    let token_usage_avg = total_tokens.checked_div(test_count).unwrap_or(0);
    let latency_p50_ms = scorer::percentile(&latencies, 50.0);
    let latency_p95_ms = scorer::percentile(&latencies, 95.0);
    let compression_ratio = if compression_ratios.is_empty() {
        0.0
    } else {
        compression_ratios.iter().sum::<f64>() / compression_ratios.len() as f64
    };

    Ok(BenchmarkReport {
        accuracy,
        recall_at_5,
        token_usage_avg,
        latency_p50_ms,
        latency_p95_ms,
        compression_ratio,
        test_count,
        pass_count,
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
