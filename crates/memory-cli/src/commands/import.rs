//! `memory import` command -- restore memory from a backup directory.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

use memory_client::{BackupChunkType, ImportChunk};

use crate::cli::{GlobalArgs, ImportArgs};
use crate::client::connect_client;

/// Batch size for JSONL lines per ImportChunk (matches CHUNK_SIZE in backup.rs).
const IMPORT_BATCH_SIZE: usize = 100;

/// Run the import command.
pub async fn run(args: ImportArgs, global: &GlobalArgs) -> Result<()> {
    let mut client = connect_client(&global.endpoint).await?;

    let base = PathBuf::from(&args.dir);
    validate_manifest(&base)?;

    let dry_run = args.dry_run;
    let events_only = args.events_only;

    let mut chunks: Vec<ImportChunk> = Vec::new();

    // 1. Read events/*.jsonl files sorted by filename (chronological)
    let events_dir = base.join("events");
    if events_dir.is_dir() {
        let mut event_files: Vec<_> = std::fs::read_dir(&events_dir)
            .context("Failed to read events directory")?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext == "jsonl")
            })
            .collect();
        event_files.sort_by_key(|e| e.file_name());

        for entry in &event_files {
            let mut file_chunks = read_jsonl_chunks(
                &entry.path(),
                BackupChunkType::Events,
                dry_run,
                events_only,
                IMPORT_BATCH_SIZE,
            )?;
            chunks.append(&mut file_chunks);
        }
    }

    // 2. If NOT events_only: read TOC files
    if !events_only {
        let toc_files = [
            ("segments.jsonl", BackupChunkType::TocSegments),
            ("days.jsonl", BackupChunkType::TocDays),
            ("weeks.jsonl", BackupChunkType::TocWeeks),
            ("months.jsonl", BackupChunkType::TocMonths),
            ("years.jsonl", BackupChunkType::TocYears),
        ];
        for (filename, chunk_type) in &toc_files {
            let path = base.join("toc").join(filename);
            let mut file_chunks =
                read_jsonl_chunks(&path, *chunk_type, dry_run, events_only, IMPORT_BATCH_SIZE)?;
            chunks.append(&mut file_chunks);
        }

        // 3. Read grips and episodes
        let mut grip_chunks = read_jsonl_chunks(
            &base.join("grips.jsonl"),
            BackupChunkType::Grips,
            dry_run,
            events_only,
            IMPORT_BATCH_SIZE,
        )?;
        chunks.append(&mut grip_chunks);

        let mut episode_chunks = read_jsonl_chunks(
            &base.join("episodes.jsonl"),
            BackupChunkType::Episodes,
            dry_run,
            events_only,
            IMPORT_BATCH_SIZE,
        )?;
        chunks.append(&mut episode_chunks);
    }

    if chunks.is_empty() {
        eprintln!("No data found to import in {}", base.display());
        return Ok(());
    }

    // Stream chunks to daemon
    let stream = tokio_stream::iter(chunks);
    let result = client.import_backup(stream).await.context("Import failed")?;

    // Report results
    report_result(&result, dry_run, events_only);

    Ok(())
}

/// Validate the manifest.json in the backup directory.
fn validate_manifest(base: &Path) -> Result<serde_json::Value> {
    let manifest_path = base.join("manifest.json");
    let content = std::fs::read_to_string(&manifest_path)
        .context("Failed to read manifest.json -- is this a valid backup directory?")?;
    let manifest: serde_json::Value =
        serde_json::from_str(&content).context("Failed to parse manifest.json")?;
    let version = manifest
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if version != "1.0" {
        bail!(
            "Unsupported backup version: '{}' (expected '1.0')",
            version
        );
    }
    Ok(manifest)
}

/// Read a JSONL file and split into batched ImportChunk messages.
fn read_jsonl_chunks(
    path: &Path,
    chunk_type: BackupChunkType,
    dry_run: bool,
    events_only: bool,
    batch_size: usize,
) -> Result<Vec<ImportChunk>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
    let mut chunks = Vec::new();
    for batch in lines.chunks(batch_size) {
        chunks.push(ImportChunk {
            chunk_type: chunk_type as i32,
            jsonl_data: batch.join("\n"),
            record_count: batch.len() as u32,
            dry_run,
            events_only,
        });
    }
    Ok(chunks)
}

/// Report import results to stderr.
fn report_result(
    result: &memory_client::ImportResult,
    dry_run: bool,
    events_only: bool,
) {
    let prefix = if dry_run { "[DRY RUN] " } else { "" };

    eprintln!("{prefix}Import results:");
    eprintln!("  Events imported:  {}", result.events_imported);
    eprintln!("  Events skipped:   {}", result.events_skipped);
    eprintln!("  TOC nodes:        {}", result.toc_nodes_imported);
    eprintln!("  Grips:            {}", result.grips_imported);
    eprintln!("  Episodes:         {}", result.episodes_imported);
    eprintln!("  Errors:           {}", result.errors);
    eprintln!("  Elapsed:          {:.2}s", result.elapsed_seconds);

    if events_only {
        eprintln!();
        eprintln!("Note: Only events were imported. TOC may need rebuild.");
        eprintln!("Run `memory admin rebuild-toc` to regenerate the table of contents.");
    }
}
