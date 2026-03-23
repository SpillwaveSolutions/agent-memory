//! `memory daily` command -- export daily markdown files from memory.

use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;

use memory_client::DayExport;

use crate::cli::{DailyArgs, GlobalArgs};
use crate::client::connect_client;

/// A group of events belonging to one session.
#[allow(dead_code)]
struct SessionGroup {
    session_id: String,
    agent: String,
    start_ms: i64,
    end_ms: i64,
    event_count: usize,
}

/// Run the daily export command.
pub async fn run(args: DailyArgs, global: &GlobalArgs) -> Result<()> {
    let mut client = connect_client(&global.endpoint).await?;
    let (start, end) = compute_date_range(&args.range);
    let result = client.export_daily(&start, &end).await?;

    std::fs::create_dir_all(&args.dir)?;

    if result.days.is_empty() {
        eprintln!("No days with events in range {} to {}", start, end);
        return Ok(());
    }

    for day in &result.days {
        let md = render_day_markdown(day);
        let path = format!("{}/{}.md", args.dir, day.date);
        std::fs::write(&path, &md)?;
        eprintln!("Wrote {}", path);
    }

    eprintln!("Exported {} day(s)", result.days.len());
    Ok(())
}

/// Compute start/end date strings from the --range flag.
///
/// None => today only. Some("7d") => last 7 days. Some("30d") => last 30 days.
fn compute_date_range(range: &Option<String>) -> (String, String) {
    let today = Utc::now().date_naive();
    match range {
        None => {
            let s = today.format("%Y-%m-%d").to_string();
            (s.clone(), s)
        }
        Some(r) => {
            let days = parse_range_to_days(r);
            let start = today - chrono::Duration::days(days - 1);
            (
                start.format("%Y-%m-%d").to_string(),
                today.format("%Y-%m-%d").to_string(),
            )
        }
    }
}

/// Parse a range string like "7d" or "30d" into number of days.
fn parse_range_to_days(range: &str) -> i64 {
    let range = range.trim();
    if let Some(stripped) = range.strip_suffix('d') {
        stripped.parse::<i64>().unwrap_or(7).max(1)
    } else if let Some(stripped) = range.strip_suffix('w') {
        stripped.parse::<i64>().unwrap_or(1).max(1) * 7
    } else {
        range.parse::<i64>().unwrap_or(7).max(1)
    }
}

/// Render a DayExport into markdown.
fn render_day_markdown(day: &DayExport) -> String {
    let mut md = String::new();

    // Title
    md.push_str(&format!("# {}\n\n", day.date));

    // Summary section (from day node if rollup complete)
    if day.has_rollup {
        if let Some(ref node) = day.day_node {
            // Summary bullets
            if !node.bullets.is_empty() {
                md.push_str("## Summary\n\n");
                for bullet in &node.bullets {
                    md.push_str(&format!("- {}\n", bullet.text));
                }
                md.push('\n');
            }
            // Keywords
            if !node.keywords.is_empty() {
                md.push_str(&format!("**Keywords:** {}\n\n", node.keywords.join(", ")));
            }
        }
    } else {
        md.push_str("*Summary pending -- day rollup not yet complete*\n\n");
    }

    // Sessions section (grouped from events by session_id)
    let sessions = group_events_by_session(&day.events);
    if !sessions.is_empty() {
        md.push_str("## Sessions\n\n");
        for (i, session) in sessions.iter().enumerate() {
            let start_time = format_timestamp_ms(session.start_ms);
            let end_time = format_timestamp_ms(session.end_ms);
            let agent_label = if session.agent.is_empty() {
                String::new()
            } else {
                format!(" [agent: {}]", session.agent)
            };
            md.push_str(&format!(
                "### Session {} ({} -- {}, {} events){}\n\n",
                i + 1,
                start_time,
                end_time,
                session.event_count,
                agent_label,
            ));

            // Find matching segment summary for this session's time range
            for seg in &day.segments {
                if seg.start_time_ms <= session.end_ms && seg.end_time_ms >= session.start_ms {
                    if let Some(ref summary) = seg.summary {
                        if !summary.is_empty() {
                            md.push_str(&format!("**Summary:** {}\n\n", summary));
                        }
                    }
                    // Segment bullets as key points
                    if !seg.bullets.is_empty() {
                        md.push_str("**Key points:**\n");
                        for bullet in &seg.bullets {
                            md.push_str(&format!("- {}\n", bullet.text));
                        }
                        md.push('\n');
                    }
                    // Segment keywords
                    if !seg.keywords.is_empty() {
                        md.push_str(&format!("**Keywords:** {}\n\n", seg.keywords.join(", ")));
                    }
                }
            }
        }
    }

    // Grip excerpts (key moments)
    if !day.grips.is_empty() {
        md.push_str("## Key Moments\n\n");
        for grip in &day.grips {
            if !grip.excerpt.is_empty() {
                md.push_str(&format!("> {}\n> -- {}\n\n", grip.excerpt, grip.grip_id));
            }
        }
    }

    // Footer (DAILY-05)
    md.push_str("---\n\n");
    md.push_str(&format!(
        "*Exported from agent-memory at {} -- this file is a derived view*\n",
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    md
}

/// Group proto events by session_id, tracking start/end times and agent.
fn group_events_by_session(events: &[memory_service::pb::Event]) -> Vec<SessionGroup> {
    // Use Vec to preserve insertion order (first-seen session_id order)
    let mut session_map: HashMap<String, usize> = HashMap::new();
    let mut sessions: Vec<SessionGroup> = Vec::new();

    for event in events {
        let sid = &event.session_id;
        if let Some(&idx) = session_map.get(sid) {
            let s = &mut sessions[idx];
            s.start_ms = s.start_ms.min(event.timestamp_ms);
            s.end_ms = s.end_ms.max(event.timestamp_ms);
            s.event_count += 1;
            // Use first non-empty agent
            if s.agent.is_empty() {
                if let Some(ref agent) = event.agent {
                    if !agent.is_empty() {
                        s.agent.clone_from(agent);
                    }
                }
            }
        } else {
            let idx = sessions.len();
            session_map.insert(sid.clone(), idx);
            sessions.push(SessionGroup {
                session_id: sid.clone(),
                agent: event.agent.clone().unwrap_or_default(),
                start_ms: event.timestamp_ms,
                end_ms: event.timestamp_ms,
                event_count: 1,
            });
        }
    }

    sessions
}

/// Format a timestamp_ms as "HH:MM" for display.
fn format_timestamp_ms(ms: i64) -> String {
    use chrono::TimeZone;
    match Utc.timestamp_millis_opt(ms) {
        chrono::LocalResult::Single(dt) => dt.format("%H:%M").to_string(),
        _ => "??:??".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory_service::pb::{
        Event as ProtoEvent, Grip as ProtoGrip, TocBullet as ProtoTocBullet,
        TocNode as ProtoTocNode,
    };

    fn make_day_export(has_rollup: bool) -> DayExport {
        let day_node = if has_rollup {
            Some(ProtoTocNode {
                node_id: "toc:day:2026-03-23".into(),
                title: "2026-03-23".into(),
                bullets: vec![
                    ProtoTocBullet {
                        text: "Implemented RRF fusion".into(),
                        grip_ids: vec!["grip:001".into()],
                    },
                    ProtoTocBullet {
                        text: "Added CLI binary".into(),
                        grip_ids: vec![],
                    },
                ],
                keywords: vec!["RRF".into(), "CLI".into()],
                ..Default::default()
            })
        } else {
            None
        };

        DayExport {
            date: "2026-03-23".into(),
            day_node,
            segments: vec![],
            events: vec![
                ProtoEvent {
                    event_id: "evt-1".into(),
                    session_id: "sess-1".into(),
                    timestamp_ms: 1_711_180_800_000, // 2026-03-23 08:00 UTC
                    agent: Some("claude".into()),
                    ..Default::default()
                },
                ProtoEvent {
                    event_id: "evt-2".into(),
                    session_id: "sess-1".into(),
                    timestamp_ms: 1_711_184_400_000, // 2026-03-23 09:00 UTC
                    agent: Some("claude".into()),
                    ..Default::default()
                },
            ],
            grips: vec![ProtoGrip {
                grip_id: "grip:001".into(),
                excerpt: "RRF is parameter-free".into(),
                ..Default::default()
            }],
            has_rollup,
        }
    }

    #[test]
    fn test_render_day_with_rollup() {
        let day = make_day_export(true);
        let md = render_day_markdown(&day);
        assert!(md.contains("# 2026-03-23"));
        assert!(md.contains("## Summary"));
        assert!(md.contains("- Implemented RRF fusion"));
        assert!(md.contains("- Added CLI binary"));
        assert!(md.contains("**Keywords:** RRF, CLI"));
        assert!(md.contains("## Key Moments"));
        assert!(md.contains("> RRF is parameter-free"));
        assert!(md.contains("derived view"));
        assert!(md.contains("Exported from agent-memory"));
    }

    #[test]
    fn test_render_day_without_rollup() {
        let day = make_day_export(false);
        let md = render_day_markdown(&day);
        assert!(md.contains("# 2026-03-23"));
        assert!(md.contains("Summary pending"));
        assert!(!md.contains("## Summary"));
        assert!(md.contains("derived view"));
    }

    #[test]
    fn test_footer_contains_derived_view() {
        let day = make_day_export(true);
        let md = render_day_markdown(&day);
        assert!(md.contains("this file is a derived view"));
        assert!(md.contains("Exported from agent-memory at"));
    }

    #[test]
    fn test_group_events_by_session() {
        let events = vec![
            ProtoEvent {
                session_id: "sess-1".into(),
                agent: Some("claude".into()),
                timestamp_ms: 1000,
                ..Default::default()
            },
            ProtoEvent {
                session_id: "sess-1".into(),
                agent: Some("claude".into()),
                timestamp_ms: 2000,
                ..Default::default()
            },
            ProtoEvent {
                session_id: "sess-2".into(),
                agent: Some("gemini".into()),
                timestamp_ms: 3000,
                ..Default::default()
            },
        ];
        let sessions = group_events_by_session(&events);
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].session_id, "sess-1");
        assert_eq!(sessions[0].agent, "claude");
        assert_eq!(sessions[0].start_ms, 1000);
        assert_eq!(sessions[0].end_ms, 2000);
        assert_eq!(sessions[0].event_count, 2);
        assert_eq!(sessions[1].session_id, "sess-2");
        assert_eq!(sessions[1].agent, "gemini");
        assert_eq!(sessions[1].event_count, 1);
    }

    #[test]
    fn test_compute_date_range_today() {
        let (start, end) = compute_date_range(&None);
        assert_eq!(start, end);
        // Should be today's date
        let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
        assert_eq!(start, today);
    }

    #[test]
    fn test_compute_date_range_7d() {
        let (start, end) = compute_date_range(&Some("7d".to_string()));
        let today = Utc::now().date_naive();
        let expected_start = (today - chrono::Duration::days(6))
            .format("%Y-%m-%d")
            .to_string();
        assert_eq!(start, expected_start);
        assert_eq!(end, today.format("%Y-%m-%d").to_string());
    }

    #[test]
    fn test_parse_range_to_days() {
        assert_eq!(parse_range_to_days("7d"), 7);
        assert_eq!(parse_range_to_days("30d"), 30);
        assert_eq!(parse_range_to_days("1w"), 7);
        assert_eq!(parse_range_to_days("2w"), 14);
    }
}
