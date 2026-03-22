//! `memory summary` command -- navigate the TOC hierarchy for compressed summaries.

use anyhow::Result;
use chrono::Utc;
use serde_json::json;

use memory_service::pb::TocNode as ProtoTocNode;

use crate::cli::{GlobalArgs, SummaryArgs};
use crate::output::{estimate_tokens, print_output, should_force_json, JsonEnvelope, Meta};

/// Parse a summary range keyword into `(from_ms, to_ms)`.
pub(crate) fn parse_summary_range(range: &str) -> (i64, i64) {
    let now = Utc::now().timestamp_millis();
    let range = range.trim();

    // Support named ranges
    let duration_ms = match range {
        "day" => 86_400_000,
        "week" => 7 * 86_400_000,
        "month" => 30 * 86_400_000,
        "year" => 365 * 86_400_000,
        _ => {
            // Delegate "Nd" / "Nw" format to parse_range logic
            if range.ends_with('d') || range.ends_with('w') {
                let (from, to) = super::timeline::parse_range(range);
                return (from, to);
            }
            // Default: week
            7 * 86_400_000
        }
    };

    (now - duration_ms, now)
}

/// Check whether a TOC node's time range overlaps with the query range.
fn node_overlaps(node: &ProtoTocNode, from_ms: i64, to_ms: i64) -> bool {
    node.start_time_ms <= to_ms && node.end_time_ms >= from_ms
}

/// Map a TocLevel enum value to a readable string.
fn level_to_string(level: i32) -> &'static str {
    match level {
        0 => "unknown",
        1 => "year",
        2 => "month",
        3 => "week",
        4 => "day",
        5 => "session",
        _ => "unknown",
    }
}

/// Run the `memory summary` command.
pub async fn run(args: SummaryArgs, global: &GlobalArgs) -> Result<()> {
    let force_json = should_force_json(&global.format, &args.format);

    let mut client = match crate::client::connect_client(&global.endpoint).await {
        Ok(c) => c,
        Err(err) => {
            let envelope = JsonEnvelope::error(&format!("{err:#}"));
            print_output(&envelope, force_json);
            std::process::exit(1);
        }
    };

    let (from_ms, to_ms) = parse_summary_range(&args.range);

    match client.get_toc_root().await {
        Ok(root_nodes) => {
            let mut summaries = Vec::new();

            for node in &root_nodes {
                if !node_overlaps(node, from_ms, to_ms) {
                    continue;
                }

                // Browse children of this root node
                match client.browse_toc(&node.node_id, 50, None).await {
                    Ok(browse_result) => {
                        for child in &browse_result.children {
                            if node_overlaps(child, from_ms, to_ms) {
                                summaries.push(json!({
                                    "node_id": child.node_id,
                                    "level": level_to_string(child.level),
                                    "label": child.title,
                                    "summary": child.summary.as_deref().unwrap_or(""),
                                    "event_count": child.child_node_ids.len(),
                                    "start_ms": child.start_time_ms,
                                    "end_ms": child.end_time_ms,
                                }));
                            }
                        }
                    }
                    Err(err) => {
                        tracing::warn!("Failed to browse TOC node {}: {err}", node.node_id);
                    }
                }
            }

            let total_tokens: usize = summaries
                .iter()
                .map(|s| {
                    estimate_tokens(s["summary"].as_str().unwrap_or(""))
                })
                .sum();

            let envelope =
                JsonEnvelope::ok("summary", json!(summaries)).with_meta(Meta {
                    retrieval_ms: 0,
                    tokens_estimated: total_tokens,
                    confidence: 1.0,
                });
            print_output(&envelope, force_json);
            Ok(())
        }
        Err(err) => {
            let envelope = JsonEnvelope::error(&format!("{err:#}"));
            print_output(&envelope, force_json);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_summary_range_day() {
        let now = Utc::now().timestamp_millis();
        let (from, to) = parse_summary_range("day");
        assert!((to - now).abs() < 1000);
        assert!((from - (now - 86_400_000)).abs() < 1000);
    }

    #[test]
    fn test_parse_summary_range_week() {
        let now = Utc::now().timestamp_millis();
        let (from, to) = parse_summary_range("week");
        assert!((to - now).abs() < 1000);
        assert!((from - (now - 7 * 86_400_000)).abs() < 1000);
    }

    #[test]
    fn test_parse_summary_range_month() {
        let now = Utc::now().timestamp_millis();
        let (from, to) = parse_summary_range("month");
        assert!((to - now).abs() < 1000);
        assert!((from - (now - 30 * 86_400_000)).abs() < 1000);
    }

    #[test]
    fn test_parse_summary_range_year() {
        let now = Utc::now().timestamp_millis();
        let (from, to) = parse_summary_range("year");
        assert!((to - now).abs() < 1000);
        assert!((from - (now - 365 * 86_400_000)).abs() < 1000);
    }

    #[test]
    fn test_parse_summary_range_delegates_nd() {
        let now = Utc::now().timestamp_millis();
        let (from, to) = parse_summary_range("14d");
        assert!((to - now).abs() < 1000);
        assert!((from - (now - 14 * 86_400_000)).abs() < 1000);
    }

    #[test]
    fn test_parse_summary_range_unknown_defaults_to_week() {
        let now = Utc::now().timestamp_millis();
        let (from, to) = parse_summary_range("unknown");
        assert!((to - now).abs() < 1000);
        assert!((from - (now - 7 * 86_400_000)).abs() < 1000);
    }

    #[test]
    fn test_node_overlaps_within_range() {
        let node = ProtoTocNode {
            start_time_ms: 100,
            end_time_ms: 200,
            ..Default::default()
        };
        assert!(node_overlaps(&node, 50, 150));
        assert!(node_overlaps(&node, 100, 200));
        assert!(node_overlaps(&node, 150, 250));
    }

    #[test]
    fn test_node_overlaps_outside_range() {
        let node = ProtoTocNode {
            start_time_ms: 100,
            end_time_ms: 200,
            ..Default::default()
        };
        assert!(!node_overlaps(&node, 201, 300));
        assert!(!node_overlaps(&node, 0, 99));
    }

    #[test]
    fn test_level_to_string() {
        assert_eq!(level_to_string(1), "year");
        assert_eq!(level_to_string(2), "month");
        assert_eq!(level_to_string(3), "week");
        assert_eq!(level_to_string(4), "day");
        assert_eq!(level_to_string(5), "session");
        assert_eq!(level_to_string(0), "unknown");
        assert_eq!(level_to_string(99), "unknown");
    }
}
