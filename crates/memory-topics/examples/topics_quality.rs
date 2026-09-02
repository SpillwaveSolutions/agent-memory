//! Emit `benchmarks/results/topics-quality.json`.
//!
//! ```text
//! cargo run -p memory-topics --example topics_quality -- benchmarks/results/topics-quality.json
//! ```

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("benchmarks/results/topics-quality.json"));
    let report = memory_topics::evaluate_labelled_corpus(5).expect("cluster labelled corpus");
    let json = serde_json::to_string_pretty(&report).expect("serialize");
    if let Some(parent) = out.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&out, format!("{json}\n")).expect("write report");
    eprintln!("wrote {}", out.display());
}
