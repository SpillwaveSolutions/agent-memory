//! ImportBackup client-streaming RPC implementation.
//!
//! Per IMPORT-01 through IMPORT-06, GRPC-03:
//! Receives JSONL chunks via client-side streaming, writes to RocksDB.

use std::sync::Arc;
use std::time::Instant;

use tonic::{Request, Status, Streaming};
use tracing::{debug, error, warn};

use memory_storage::Storage;
use memory_types::{Episode, Event, Grip, OutboxEntry, TocNode};

use crate::pb::{BackupChunkType, ImportChunk, ImportResult};

/// Counters for import progress tracking.
#[derive(Default)]
struct ImportCounts {
    events_imported: u64,
    events_skipped: u64,
    toc_nodes_imported: u64,
    grips_imported: u64,
    episodes_imported: u64,
    errors: u64,
}

/// Handle ImportBackup client-streaming RPC.
///
/// Receives a stream of `ImportChunk` messages and writes each record to the
/// appropriate RocksDB column family. Events use `put_event` for idempotency
/// (existing event_ids are skipped). Outbox entries are created for imported
/// events so they get re-indexed.
pub async fn import_backup(
    storage: Arc<Storage>,
    request: Request<Streaming<ImportChunk>>,
) -> Result<tonic::Response<ImportResult>, Status> {
    let start = Instant::now();
    let mut stream = request.into_inner();
    let mut counts = ImportCounts::default();
    let mut dry_run = false;
    let mut first_chunk = true;

    while let Some(chunk) = stream.message().await.map_err(|e| {
        error!("Import stream error: {e}");
        Status::internal(format!("Stream error: {e}"))
    })? {
        if first_chunk {
            dry_run = chunk.dry_run;
            first_chunk = false;
        }

        let chunk_type = chunk.chunk_type();
        match chunk_type {
            BackupChunkType::Events => {
                import_events(&storage, &chunk.jsonl_data, dry_run, &mut counts);
            }
            BackupChunkType::TocSegments
            | BackupChunkType::TocDays
            | BackupChunkType::TocWeeks
            | BackupChunkType::TocMonths
            | BackupChunkType::TocYears => {
                import_toc_nodes(&storage, &chunk.jsonl_data, dry_run, &mut counts);
            }
            BackupChunkType::Grips => {
                import_grips(&storage, &chunk.jsonl_data, dry_run, &mut counts);
            }
            BackupChunkType::Episodes => {
                import_episodes(&storage, &chunk.jsonl_data, dry_run, &mut counts);
            }
            _ => {
                debug!("Skipping chunk type {:?} during import", chunk_type);
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    debug!(
        "Import complete: {} events ({} skipped), {} toc, {} grips, {} episodes, {} errors in {:.2}s",
        counts.events_imported,
        counts.events_skipped,
        counts.toc_nodes_imported,
        counts.grips_imported,
        counts.episodes_imported,
        counts.errors,
        elapsed,
    );

    Ok(tonic::Response::new(ImportResult {
        events_imported: counts.events_imported,
        events_skipped: counts.events_skipped,
        toc_nodes_imported: counts.toc_nodes_imported,
        grips_imported: counts.grips_imported,
        episodes_imported: counts.episodes_imported,
        errors: counts.errors,
        elapsed_seconds: elapsed,
        dry_run,
    }))
}

/// Import events from JSONL data with idempotency and outbox entries.
fn import_events(storage: &Storage, jsonl_data: &str, dry_run: bool, counts: &mut ImportCounts) {
    for line in jsonl_data.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let event: Event = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to deserialize event: {e}");
                counts.errors += 1;
                continue;
            }
        };

        if dry_run {
            counts.events_imported += 1;
            continue;
        }

        let event_bytes = match event.to_bytes() {
            Ok(b) => b,
            Err(e) => {
                warn!("Failed to serialize event bytes: {e}");
                counts.errors += 1;
                continue;
            }
        };

        let outbox_entry = OutboxEntry::for_toc(event.event_id.clone(), event.timestamp_ms());
        let outbox_bytes = match outbox_entry.to_bytes() {
            Ok(b) => b,
            Err(e) => {
                warn!("Failed to serialize outbox entry: {e}");
                counts.errors += 1;
                continue;
            }
        };

        match storage.put_event(&event.event_id, &event_bytes, &outbox_bytes) {
            Ok((_key, true)) => counts.events_imported += 1,
            Ok((_key, false)) => counts.events_skipped += 1,
            Err(e) => {
                warn!("Failed to store event {}: {e}", event.event_id);
                counts.errors += 1;
            }
        }
    }
}

/// Import TOC nodes from JSONL data.
fn import_toc_nodes(
    storage: &Storage,
    jsonl_data: &str,
    dry_run: bool,
    counts: &mut ImportCounts,
) {
    for line in jsonl_data.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let node: TocNode = match serde_json::from_str(line) {
            Ok(n) => n,
            Err(e) => {
                warn!("Failed to deserialize TOC node: {e}");
                counts.errors += 1;
                continue;
            }
        };

        if dry_run {
            counts.toc_nodes_imported += 1;
            continue;
        }

        match storage.put_toc_node(&node) {
            Ok(()) => counts.toc_nodes_imported += 1,
            Err(e) => {
                warn!("Failed to store TOC node: {e}");
                counts.errors += 1;
            }
        }
    }
}

/// Import grips from JSONL data.
fn import_grips(storage: &Storage, jsonl_data: &str, dry_run: bool, counts: &mut ImportCounts) {
    for line in jsonl_data.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let grip: Grip = match serde_json::from_str(line) {
            Ok(g) => g,
            Err(e) => {
                warn!("Failed to deserialize grip: {e}");
                counts.errors += 1;
                continue;
            }
        };

        if dry_run {
            counts.grips_imported += 1;
            continue;
        }

        match storage.put_grip(&grip) {
            Ok(()) => counts.grips_imported += 1,
            Err(e) => {
                warn!("Failed to store grip: {e}");
                counts.errors += 1;
            }
        }
    }
}

/// Import episodes from JSONL data.
fn import_episodes(
    storage: &Storage,
    jsonl_data: &str,
    dry_run: bool,
    counts: &mut ImportCounts,
) {
    for line in jsonl_data.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let episode: Episode = match serde_json::from_str(line) {
            Ok(ep) => ep,
            Err(e) => {
                warn!("Failed to deserialize episode: {e}");
                counts.errors += 1;
                continue;
            }
        };

        if dry_run {
            counts.episodes_imported += 1;
            continue;
        }

        match storage.store_episode(&episode) {
            Ok(()) => counts.episodes_imported += 1,
            Err(e) => {
                warn!("Failed to store episode: {e}");
                counts.errors += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_counts_default() {
        let counts = ImportCounts::default();
        assert_eq!(counts.events_imported, 0);
        assert_eq!(counts.events_skipped, 0);
        assert_eq!(counts.toc_nodes_imported, 0);
        assert_eq!(counts.grips_imported, 0);
        assert_eq!(counts.episodes_imported, 0);
        assert_eq!(counts.errors, 0);
    }
}
