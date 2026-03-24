//! ExportBackup streaming RPC implementation.
//!
//! Per BACKUP-01 through BACKUP-07, GRPC-02, GRPC-04:
//! Streams backup data as JSONL chunks via server-side streaming.

use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::{debug, error, warn};

use memory_storage::Storage;
use memory_types::{Event, TocLevel as DomainTocLevel};

use crate::pb::{BackupChunk, BackupChunkType, BackupOptions};

/// Stream type for the ExportBackup RPC.
pub type ExportBackupStream = ReceiverStream<Result<BackupChunk, Status>>;

/// Channel buffer size for streaming chunks.
const CHANNEL_BUFFER: usize = 64;

/// Number of records to batch per chunk.
const CHUNK_SIZE: usize = 100;

/// Handle ExportBackup streaming RPC.
pub async fn export_backup(
    storage: Arc<Storage>,
    request: Request<BackupOptions>,
) -> Result<Response<ExportBackupStream>, Status> {
    let opts = request.into_inner();
    debug!(
        "ExportBackup request: events_only={}, since_ms={}, until_ms={}",
        opts.events_only, opts.since_ms, opts.until_ms
    );

    let (tx, rx) = mpsc::channel(CHANNEL_BUFFER);

    tokio::spawn(async move {
        if let Err(e) = stream_backup(&storage, &opts, &tx).await {
            error!("Backup stream error: {e}");
            let _ = tx.send(Err(Status::internal(e.to_string()))).await;
        }
    });

    Ok(Response::new(ReceiverStream::new(rx)))
}

#[derive(Default)]
struct ManifestCounts {
    events: u64,
    toc_segments: u64,
    toc_days: u64,
    toc_weeks: u64,
    toc_months: u64,
    toc_years: u64,
    grips: u64,
    episodes: u64,
}

async fn stream_backup(
    storage: &Arc<Storage>,
    opts: &BackupOptions,
    tx: &mpsc::Sender<Result<BackupChunk, Status>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut counts = ManifestCounts::default();

    // Determine time range
    let since_ms = if opts.since_ms > 0 {
        opts.since_ms
    } else {
        0
    };
    let until_ms = if opts.until_ms > 0 {
        opts.until_ms
    } else {
        i64::MAX
    };

    // 1. Stream events (always included)
    counts.events = stream_events(storage, since_ms, until_ms, tx).await?;

    // 2. Stream derived layers (unless events_only)
    if !opts.events_only {
        counts.toc_segments = stream_toc_level(
            storage,
            DomainTocLevel::Segment,
            BackupChunkType::TocSegments,
            tx,
        )
        .await?;
        counts.toc_days =
            stream_toc_level(storage, DomainTocLevel::Day, BackupChunkType::TocDays, tx).await?;
        counts.toc_weeks =
            stream_toc_level(storage, DomainTocLevel::Week, BackupChunkType::TocWeeks, tx).await?;
        counts.toc_months = stream_toc_level(
            storage,
            DomainTocLevel::Month,
            BackupChunkType::TocMonths,
            tx,
        )
        .await?;
        counts.toc_years =
            stream_toc_level(storage, DomainTocLevel::Year, BackupChunkType::TocYears, tx).await?;
        counts.grips = stream_grips(storage, tx).await?;
        counts.episodes = stream_episodes(storage, tx).await?;
    }

    // 3. Stream manifest LAST (signals backup complete)
    stream_manifest(opts, &counts, tx).await?;

    Ok(())
}

async fn stream_events(
    storage: &Arc<Storage>,
    since_ms: i64,
    until_ms: i64,
    tx: &mpsc::Sender<Result<BackupChunk, Status>>,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let raw_events = storage.get_events_in_range(since_ms, until_ms)?;
    let mut total: u64 = 0;
    let mut lines = Vec::with_capacity(CHUNK_SIZE);

    for (_key, bytes) in &raw_events {
        match Event::from_bytes(bytes) {
            Ok(event) => {
                let json = serde_json::to_string(&event)?;
                lines.push(json);
                total += 1;

                if lines.len() >= CHUNK_SIZE {
                    tx.send(Ok(build_chunk(BackupChunkType::Events, &lines)))
                        .await?;
                    lines.clear();
                }
            }
            Err(e) => {
                warn!("Skipping undeserializable event during backup: {e}");
            }
        }
    }

    // Flush remaining
    if !lines.is_empty() {
        tx.send(Ok(build_chunk(BackupChunkType::Events, &lines)))
            .await?;
    }

    debug!("Streamed {total} events");
    Ok(total)
}

async fn stream_toc_level(
    storage: &Arc<Storage>,
    level: DomainTocLevel,
    chunk_type: BackupChunkType,
    tx: &mpsc::Sender<Result<BackupChunk, Status>>,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let nodes = storage.get_toc_nodes_by_level(level, None, None)?;
    let mut total: u64 = 0;
    let mut lines = Vec::with_capacity(CHUNK_SIZE);

    for node in &nodes {
        let json = serde_json::to_string(node)?;
        lines.push(json);
        total += 1;

        if lines.len() >= CHUNK_SIZE {
            tx.send(Ok(build_chunk(chunk_type, &lines))).await?;
            lines.clear();
        }
    }

    if !lines.is_empty() {
        tx.send(Ok(build_chunk(chunk_type, &lines))).await?;
    }

    debug!("Streamed {total} TOC nodes at level {level}");
    Ok(total)
}

async fn stream_grips(
    storage: &Arc<Storage>,
    tx: &mpsc::Sender<Result<BackupChunk, Status>>,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let grips = storage.list_all_grips()?;
    let mut total: u64 = 0;
    let mut lines = Vec::with_capacity(CHUNK_SIZE);

    for grip in &grips {
        let json = serde_json::to_string(grip)?;
        lines.push(json);
        total += 1;

        if lines.len() >= CHUNK_SIZE {
            tx.send(Ok(build_chunk(BackupChunkType::Grips, &lines)))
                .await?;
            lines.clear();
        }
    }

    if !lines.is_empty() {
        tx.send(Ok(build_chunk(BackupChunkType::Grips, &lines)))
            .await?;
    }

    debug!("Streamed {total} grips");
    Ok(total)
}

async fn stream_episodes(
    storage: &Arc<Storage>,
    tx: &mpsc::Sender<Result<BackupChunk, Status>>,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let episodes = storage.list_all_episodes()?;
    let mut total: u64 = 0;
    let mut lines = Vec::with_capacity(CHUNK_SIZE);

    for episode in &episodes {
        let json = serde_json::to_string(episode)?;
        lines.push(json);
        total += 1;

        if lines.len() >= CHUNK_SIZE {
            tx.send(Ok(build_chunk(BackupChunkType::Episodes, &lines)))
                .await?;
            lines.clear();
        }
    }

    if !lines.is_empty() {
        tx.send(Ok(build_chunk(BackupChunkType::Episodes, &lines)))
            .await?;
    }

    debug!("Streamed {total} episodes");
    Ok(total)
}

async fn stream_manifest(
    opts: &BackupOptions,
    counts: &ManifestCounts,
    tx: &mpsc::Sender<Result<BackupChunk, Status>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let incremental = opts.since_ms > 0;
    let now = chrono::Utc::now().to_rfc3339();

    let mut manifest = serde_json::json!({
        "version": "1.0",
        "agent_memory_version": env!("CARGO_PKG_VERSION"),
        "export_date": now,
        "incremental": incremental,
        "events_only": opts.events_only,
        "counts": {
            "events": counts.events,
            "toc_segments": counts.toc_segments,
            "toc_days": counts.toc_days,
            "toc_weeks": counts.toc_weeks,
            "toc_months": counts.toc_months,
            "toc_years": counts.toc_years,
            "grips": counts.grips,
            "episodes": counts.episodes,
        }
    });

    if incremental {
        manifest["since_ms"] = serde_json::json!(opts.since_ms);
        if opts.until_ms > 0 {
            manifest["until_ms"] = serde_json::json!(opts.until_ms);
        }
    }

    let json = serde_json::to_string(&manifest)?;
    tx.send(Ok(build_chunk(BackupChunkType::Manifest, &[json])))
        .await?;

    debug!("Streamed manifest");
    Ok(())
}

fn build_chunk(chunk_type: BackupChunkType, lines: &[String]) -> BackupChunk {
    BackupChunk {
        chunk_type: chunk_type as i32,
        jsonl_data: lines.join("\n"),
        record_count: lines.len() as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_chunk() {
        let lines = vec!["line1".to_string(), "line2".to_string()];
        let chunk = build_chunk(BackupChunkType::Events, &lines);
        assert_eq!(chunk.record_count, 2);
        assert_eq!(chunk.jsonl_data, "line1\nline2");
        assert_eq!(chunk.chunk_type, BackupChunkType::Events as i32);
    }

    #[test]
    fn test_build_chunk_single_line() {
        let lines = vec!["only-line".to_string()];
        let chunk = build_chunk(BackupChunkType::Manifest, &lines);
        assert_eq!(chunk.record_count, 1);
        assert_eq!(chunk.jsonl_data, "only-line");
        assert_eq!(chunk.chunk_type, BackupChunkType::Manifest as i32);
    }
}
