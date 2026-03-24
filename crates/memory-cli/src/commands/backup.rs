//! `memory backup` command -- structured JSONL backup of all memory layers.

use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use memory_client::BackupChunkType;

use crate::cli::{BackupArgs, GlobalArgs};
use crate::client::connect_client;

/// Run the backup command.
pub async fn run(args: BackupArgs, global: &GlobalArgs) -> Result<()> {
    let mut client = connect_client(&global.endpoint).await?;

    let (since_ms, until_ms) = parse_time_range(&args.since, &args.until)?;

    let base = PathBuf::from(&args.dir);
    create_backup_dirs(&base, args.events_only)?;

    let mut stream = client
        .export_backup(args.events_only, since_ms, until_ms)
        .await
        .context("Failed to start backup stream")?;

    // Track per-day event buffers for overwrite semantics (BACKUP-04)
    let mut day_events: HashMap<String, Vec<String>> = HashMap::new();
    let mut toc_buffers: HashMap<String, Vec<String>> = HashMap::new();
    let mut grip_lines: Vec<String> = Vec::new();
    let mut episode_lines: Vec<String> = Vec::new();
    let mut manifest_json: Option<String> = None;
    let mut total_chunks = 0u64;

    while let Some(chunk) = stream.message().await? {
        total_chunks += 1;
        let chunk_type = chunk.chunk_type();

        match chunk_type {
            BackupChunkType::Events => {
                for line in chunk.jsonl_data.split('\n') {
                    if line.is_empty() {
                        continue;
                    }
                    if let Some(day_str) = extract_day_from_jsonl(line) {
                        day_events
                            .entry(day_str)
                            .or_default()
                            .push(line.to_string());
                    }
                }
            }
            BackupChunkType::TocSegments => {
                toc_buffers
                    .entry("segments".to_string())
                    .or_default()
                    .extend(
                        chunk
                            .jsonl_data
                            .split('\n')
                            .filter(|l| !l.is_empty())
                            .map(String::from),
                    );
            }
            BackupChunkType::TocDays => {
                toc_buffers.entry("days".to_string()).or_default().extend(
                    chunk
                        .jsonl_data
                        .split('\n')
                        .filter(|l| !l.is_empty())
                        .map(String::from),
                );
            }
            BackupChunkType::TocWeeks => {
                toc_buffers.entry("weeks".to_string()).or_default().extend(
                    chunk
                        .jsonl_data
                        .split('\n')
                        .filter(|l| !l.is_empty())
                        .map(String::from),
                );
            }
            BackupChunkType::TocMonths => {
                toc_buffers.entry("months".to_string()).or_default().extend(
                    chunk
                        .jsonl_data
                        .split('\n')
                        .filter(|l| !l.is_empty())
                        .map(String::from),
                );
            }
            BackupChunkType::TocYears => {
                toc_buffers.entry("years".to_string()).or_default().extend(
                    chunk
                        .jsonl_data
                        .split('\n')
                        .filter(|l| !l.is_empty())
                        .map(String::from),
                );
            }
            BackupChunkType::Grips => {
                grip_lines.extend(
                    chunk
                        .jsonl_data
                        .split('\n')
                        .filter(|l| !l.is_empty())
                        .map(String::from),
                );
            }
            BackupChunkType::Episodes => {
                episode_lines.extend(
                    chunk
                        .jsonl_data
                        .split('\n')
                        .filter(|l| !l.is_empty())
                        .map(String::from),
                );
            }
            BackupChunkType::Manifest => {
                manifest_json = Some(chunk.jsonl_data);
            }
            _ => {
                eprintln!("Warning: unknown chunk type {}", chunk.chunk_type);
            }
        }
    }

    // Write all files (overwrite semantics per BACKUP-04)
    write_event_files(&base, &day_events)?;

    if !args.events_only {
        write_toc_files(&base, &toc_buffers)?;
        write_jsonl_file(&base.join("grips.jsonl"), &grip_lines)?;
        write_jsonl_file(&base.join("episodes.jsonl"), &episode_lines)?;
    }

    if let Some(manifest) = manifest_json {
        // Pretty-print the manifest
        let parsed: serde_json::Value = serde_json::from_str(&manifest)
            .unwrap_or_else(|_| serde_json::Value::String(manifest.clone()));
        let pretty = serde_json::to_string_pretty(&parsed)?;
        std::fs::write(base.join("manifest.json"), pretty)?;
        eprintln!("Wrote manifest.json");
    }

    eprintln!(
        "Backup complete: {} chunks received, {} day file(s)",
        total_chunks,
        day_events.len()
    );
    Ok(())
}

/// Create backup directory structure.
fn create_backup_dirs(base: &Path, events_only: bool) -> Result<()> {
    std::fs::create_dir_all(base.join("events"))?;
    if !events_only {
        std::fs::create_dir_all(base.join("toc"))?;
    }
    Ok(())
}

/// Extract YYYY-MM-DD from a JSONL event line by parsing `timestamp_ms`.
fn extract_day_from_jsonl(line: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let ts_ms = v.get("timestamp_ms")?.as_i64().or_else(|| {
        // Handle the case where timestamp is an ISO string
        v.get("timestamp")?
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.timestamp_millis())
    })?;
    let dt = chrono::DateTime::from_timestamp_millis(ts_ms)?;
    Some(dt.format("%Y-%m-%d").to_string())
}

/// Write per-day event JSONL files (overwrite, not append).
fn write_event_files(base: &Path, day_events: &HashMap<String, Vec<String>>) -> Result<()> {
    for (day, lines) in day_events {
        let path = base.join("events").join(format!("{day}.jsonl"));
        let content = lines.join("\n") + "\n";
        std::fs::write(&path, content)?;
        eprintln!("Wrote events/{day}.jsonl ({} events)", lines.len());
    }
    Ok(())
}

/// Write TOC JSONL files.
fn write_toc_files(base: &Path, toc_buffers: &HashMap<String, Vec<String>>) -> Result<()> {
    for (level, lines) in toc_buffers {
        let path = base.join("toc").join(format!("{level}.jsonl"));
        if !lines.is_empty() {
            let content = lines.join("\n") + "\n";
            std::fs::write(&path, content)?;
            eprintln!("Wrote toc/{level}.jsonl ({} nodes)", lines.len());
        }
    }
    Ok(())
}

/// Write a JSONL file from lines.
fn write_jsonl_file(path: &Path, lines: &[String]) -> Result<()> {
    if !lines.is_empty() {
        let content = lines.join("\n") + "\n";
        std::fs::write(path, content)?;
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        eprintln!("Wrote {filename} ({} records)", lines.len());
    }
    Ok(())
}

/// Parse `--since` and `--until` into millisecond timestamps.
fn parse_time_range(since: &Option<String>, until: &Option<String>) -> Result<(i64, i64)> {
    let since_ms = match since {
        None => 0,
        Some(s) => parse_time_spec(s)?,
    };
    let until_ms = match until {
        None => 0, // 0 means "now" on server side
        Some(u) => parse_time_spec(u)?,
    };
    Ok((since_ms, until_ms))
}

/// Parse a time specification: "24h", "7d", or "YYYY-MM-DD".
fn parse_time_spec(spec: &str) -> Result<i64> {
    let now = Utc::now();
    if let Some(hours) = spec.strip_suffix('h') {
        let h: i64 = hours.parse().context("Invalid hour value")?;
        Ok((now - chrono::Duration::hours(h)).timestamp_millis())
    } else if let Some(days) = spec.strip_suffix('d') {
        let d: i64 = days.parse().context("Invalid day value")?;
        Ok((now - chrono::Duration::days(d)).timestamp_millis())
    } else {
        // Try YYYY-MM-DD
        let date = NaiveDate::parse_from_str(spec, "%Y-%m-%d")
            .context("Expected format: 24h, 7d, or YYYY-MM-DD")?;
        Ok(date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_day_from_jsonl_with_timestamp_ms() {
        let line = r#"{"event_id":"e1","timestamp_ms":1711152000000,"text":"hello"}"#;
        let day = extract_day_from_jsonl(line);
        assert!(day.is_some());
        // 1711152000000 = 2024-03-23 00:00:00 UTC
        assert_eq!(day.unwrap(), "2024-03-23");
    }

    #[test]
    fn test_extract_day_from_jsonl_missing_timestamp() {
        let line = r#"{"event_id":"e1","text":"hello"}"#;
        let day = extract_day_from_jsonl(line);
        assert!(day.is_none());
    }

    #[test]
    fn test_parse_time_spec_hours() {
        let ms = parse_time_spec("24h").unwrap();
        let now_ms = Utc::now().timestamp_millis();
        let diff = now_ms - ms;
        // Should be approximately 24 hours ago (86_400_000 ms)
        assert!(diff > 86_000_000 && diff < 87_000_000);
    }

    #[test]
    fn test_parse_time_spec_days() {
        let ms = parse_time_spec("7d").unwrap();
        let now_ms = Utc::now().timestamp_millis();
        let diff = now_ms - ms;
        let seven_days_ms = 7 * 86_400_000;
        assert!(diff > (seven_days_ms - 1000) && diff < (seven_days_ms + 1000));
    }

    #[test]
    fn test_parse_time_spec_date() {
        let ms = parse_time_spec("2026-03-22").unwrap();
        assert!(ms > 0);
    }

    #[test]
    fn test_parse_time_spec_invalid() {
        assert!(parse_time_spec("not-a-date").is_err());
    }

    #[test]
    fn test_parse_time_range_defaults() {
        let (since, until) = parse_time_range(&None, &None).unwrap();
        assert_eq!(since, 0);
        assert_eq!(until, 0);
    }

    #[test]
    fn test_parse_time_range_with_since() {
        let (since, until) = parse_time_range(&Some("24h".to_string()), &None).unwrap();
        assert!(since > 0);
        assert_eq!(until, 0);
    }
}
