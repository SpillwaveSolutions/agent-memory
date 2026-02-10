//! Command implementations for the memory daemon.
//!
//! Handles:
//! - start: Load config, open storage, start gRPC server with scheduler
//! - stop: Signal running daemon to stop (via PID file)
//! - status: Check if daemon is running

use std::fs;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use anyhow::{Context, Result};
use chrono::TimeZone;
use tokio::signal;
use tracing::{info, warn};

use memory_client::MemoryClient;
use memory_scheduler::{
    create_compaction_job, create_indexing_job, create_rollup_jobs, CompactionJobConfig,
    IndexingJobConfig, RollupJobConfig, SchedulerConfig, SchedulerService,
};
use memory_service::pb::{
    GetSchedulerStatusRequest, JobResultStatus, PauseJobRequest, ResumeJobRequest,
    SearchChildrenRequest, SearchField as ProtoSearchField, SearchNodeRequest,
    TocLevel as ProtoTocLevel,
};
use memory_service::run_server_with_scheduler;
use memory_storage::Storage;
use memory_toc::summarizer::MockSummarizer;
use memory_types::Settings;

use crate::cli::{
    AdminCommands, AgentsCommand, ClodCliCommand, QueryCommands, RetrievalCommand,
    SchedulerCommands, TeleportCommand, TopicsCommand,
};

/// Get the PID file path
fn pid_file_path() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| {
            // On macOS/Linux, use runtime dir or fall back to cache dir
            #[cfg(unix)]
            {
                dirs.runtime_dir()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| dirs.cache_dir().to_path_buf())
            }
            #[cfg(not(unix))]
            {
                dirs.cache_dir().to_path_buf()
            }
        })
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("agent-memory")
        .join("daemon.pid")
}

/// Write PID to file
fn write_pid_file() -> Result<()> {
    let pid_path = pid_file_path();
    if let Some(parent) = pid_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&pid_path, std::process::id().to_string())?;
    info!("Wrote PID file: {:?}", pid_path);
    Ok(())
}

/// Remove PID file
fn remove_pid_file() {
    let pid_path = pid_file_path();
    if pid_path.exists() {
        if let Err(e) = fs::remove_file(&pid_path) {
            warn!("Failed to remove PID file: {}", e);
        } else {
            info!("Removed PID file");
        }
    }
}

/// Read PID from file
fn read_pid_file() -> Option<u32> {
    let pid_path = pid_file_path();
    fs::read_to_string(&pid_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Check if a process is running
#[cfg(unix)]
fn is_process_running(pid: u32) -> bool {
    // On Unix, sending signal 0 checks if process exists
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(not(unix))]
fn is_process_running(_pid: u32) -> bool {
    // On Windows, we'd need a different approach
    // For now, assume running if PID file exists
    true
}

/// Register the indexing job if search indexes are available.
///
/// This function attempts to:
/// 1. Open the BM25 search index (required)
/// 2. Create an indexing pipeline with the BM25 updater
/// 3. Register the pipeline with the scheduler
///
/// If the search index doesn't exist, returns an error. Users should
/// run `rebuild-indexes` first to initialize the search index.
async fn register_indexing_job(
    scheduler: &SchedulerService,
    storage: Arc<Storage>,
    db_path: &Path,
) -> Result<()> {
    use memory_indexing::{Bm25IndexUpdater, IndexingPipeline, PipelineConfig};
    use memory_search::{SearchIndex, SearchIndexConfig, SearchIndexer};

    // Check if search index exists
    let search_dir = db_path.join("search");
    if !search_dir.exists() {
        anyhow::bail!("Search index directory not found at {:?}", search_dir);
    }

    // Open search index
    let search_config = SearchIndexConfig::new(&search_dir);
    let search_index =
        SearchIndex::open_or_create(search_config).context("Failed to open search index")?;
    let indexer =
        Arc::new(SearchIndexer::new(&search_index).context("Failed to create search indexer")?);

    // Create BM25 updater
    let bm25_updater = Bm25IndexUpdater::new(indexer, storage.clone());

    // Create indexing pipeline with BM25 updater
    let mut pipeline = IndexingPipeline::new(storage.clone(), PipelineConfig::default());
    pipeline.add_updater(Box::new(bm25_updater));
    pipeline
        .load_checkpoints()
        .context("Failed to load indexing checkpoints")?;

    let pipeline = Arc::new(tokio::sync::Mutex::new(pipeline));

    // Register with scheduler
    create_indexing_job(scheduler, pipeline, IndexingJobConfig::default())
        .await
        .context("Failed to register indexing job")?;

    info!("Indexing job registered with BM25 updater");
    Ok(())
}

/// Register lifecycle prune jobs if indexes are available.
///
/// This function registers:
/// 1. BM25 prune job - prunes old documents from Tantivy index
/// 2. Vector prune job - prunes old vectors from HNSW index
///
/// Both jobs use per-level retention configured in lifecycle settings.
/// BM25 pruning is DISABLED by default (per PRD append-only philosophy).
/// Vector pruning is ENABLED by default.
async fn register_prune_jobs(scheduler: &SchedulerService, db_path: &Path) -> Result<()> {
    use memory_embeddings::EmbeddingModel;
    use memory_scheduler::{
        register_bm25_prune_job, register_vector_prune_job, Bm25PruneJob, Bm25PruneJobConfig,
        VectorPruneJob, VectorPruneJobConfig,
    };
    use memory_search::{SearchIndex, SearchIndexConfig, SearchIndexer};
    use memory_vector::{
        HnswConfig, HnswIndex, PipelineConfig as VectorPipelineConfig, VectorIndexPipeline,
        VectorMetadata,
    };

    let search_dir = db_path.join("search");
    let vector_dir = db_path.join("vector");

    // Register BM25 prune job if search index exists
    if search_dir.exists() {
        let search_config = SearchIndexConfig::new(&search_dir);
        match SearchIndex::open_or_create(search_config) {
            Ok(search_index) => {
                match SearchIndexer::new(&search_index) {
                    Ok(indexer) => {
                        let indexer = Arc::new(indexer);

                        // Create prune job with callback
                        let bm25_job = Bm25PruneJob::with_prune_fn(
                            Bm25PruneJobConfig::default(),
                            move |age_days, level, dry_run| {
                                let idx = Arc::clone(&indexer);
                                async move {
                                    idx.prune_and_commit(age_days, level.as_deref(), dry_run)
                                        .map_err(|e| e.to_string())
                                }
                            },
                        );

                        register_bm25_prune_job(scheduler, bm25_job)
                            .await
                            .context("Failed to register BM25 prune job")?;

                        info!("BM25 prune job registered");
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to create search indexer for BM25 prune job");
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to open search index for BM25 prune job");
            }
        }
    } else {
        info!("Search index not found, skipping BM25 prune job registration");
    }

    // Register vector prune job if vector index exists
    if vector_dir.exists() {
        // Try to create embedder
        match memory_embeddings::CandleEmbedder::load_default() {
            Ok(embedder) => {
                let embedder = Arc::new(embedder);
                let hnsw_config = HnswConfig::new(embedder.info().dimension, &vector_dir);

                match HnswIndex::open_or_create(hnsw_config) {
                    Ok(hnsw_index) => {
                        let hnsw_index = Arc::new(RwLock::new(hnsw_index));

                        // Open metadata store
                        let metadata_path = vector_dir.join("metadata");
                        if metadata_path.exists() {
                            match VectorMetadata::open(&metadata_path) {
                                Ok(metadata) => {
                                    let metadata = Arc::new(metadata);
                                    let pipeline = Arc::new(VectorIndexPipeline::new(
                                        embedder,
                                        hnsw_index,
                                        metadata,
                                        VectorPipelineConfig::default(),
                                    ));

                                    // Create prune job with callback
                                    let vector_job = VectorPruneJob::with_prune_fn(
                                        VectorPruneJobConfig::default(),
                                        move |age_days, level| {
                                            let p = Arc::clone(&pipeline);
                                            async move {
                                                p.prune_level(age_days, level.as_deref())
                                                    .map_err(|e| e.to_string())
                                            }
                                        },
                                    );

                                    register_vector_prune_job(scheduler, vector_job)
                                        .await
                                        .context("Failed to register vector prune job")?;

                                    info!("Vector prune job registered");
                                }
                                Err(e) => {
                                    warn!(error = %e, "Failed to open vector metadata for prune job");
                                }
                            }
                        } else {
                            info!(
                                "Vector metadata not found, skipping vector prune job registration"
                            );
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to open HNSW index for vector prune job");
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to load embedder for vector prune job");
            }
        }
    } else {
        info!("Vector index not found, skipping vector prune job registration");
    }

    Ok(())
}

/// Start the memory daemon.
///
/// 1. Load configuration (CFG-01: defaults -> file -> env -> CLI)
/// 2. Open RocksDB storage
/// 3. Create and start scheduler with rollup and compaction jobs
/// 4. Start gRPC server with scheduler integration
/// 5. Handle graceful shutdown on SIGINT/SIGTERM
pub async fn start_daemon(
    config_path: Option<&str>,
    foreground: bool,
    port_override: Option<u16>,
    db_path_override: Option<&str>,
    log_level_override: Option<&str>,
) -> Result<()> {
    // Load configuration (CFG-01)
    let mut settings = Settings::load(config_path).context("Failed to load configuration")?;

    // Apply CLI overrides (highest precedence per CFG-01)
    if let Some(port) = port_override {
        settings.grpc_port = port;
    }
    if let Some(db_path) = db_path_override {
        settings.db_path = db_path.to_string();
    }
    if let Some(log_level) = log_level_override {
        settings.log_level = log_level.to_string();
    }

    // Initialize logging
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&settings.log_level)),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set tracing subscriber")?;

    info!("Memory daemon starting...");
    info!("Configuration:");
    info!("  Database path: {}", settings.db_path);
    info!("  gRPC address: {}", settings.grpc_addr());
    info!("  Log level: {}", settings.log_level);

    if !foreground {
        // TODO: Implement actual daemonization (double-fork on Unix)
        // For Phase 1, just warn and continue in foreground
        warn!("Background mode not yet implemented, running in foreground");
        warn!("Use a process manager (systemd, launchd) for background operation");
    }

    // Open storage (STOR-04: per-project RocksDB instance)
    let db_path = settings.expanded_db_path();
    info!("Opening storage at {:?}", db_path);

    // Create parent directories if needed
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).context("Failed to create database directory")?;
    }

    let storage = Storage::open(&db_path).context("Failed to open storage")?;
    let storage = Arc::new(storage);

    // Create scheduler
    info!("Initializing scheduler...");
    let scheduler = SchedulerService::new(SchedulerConfig::default())
        .await
        .context("Failed to create scheduler")?;

    // Create summarizer for rollup jobs
    // TODO: Load from config - use ApiSummarizer if OPENAI_API_KEY or ANTHROPIC_API_KEY set
    let summarizer: Arc<dyn memory_toc::summarizer::Summarizer> = Arc::new(MockSummarizer::new());

    // Register rollup jobs (day/week/month)
    create_rollup_jobs(
        &scheduler,
        storage.clone(),
        summarizer,
        RollupJobConfig::default(),
    )
    .await
    .context("Failed to register rollup jobs")?;

    // Register compaction job
    create_compaction_job(&scheduler, storage.clone(), CompactionJobConfig::default())
        .await
        .context("Failed to register compaction job")?;

    // Register indexing job if search index exists
    // The indexing pipeline processes outbox entries into search indexes
    if let Err(e) = register_indexing_job(&scheduler, storage.clone(), &db_path).await {
        warn!("Indexing job not registered: {}", e);
        info!("Run 'rebuild-indexes' to initialize the search index");
    }

    // Register lifecycle prune jobs if indexes exist
    // These jobs prune old documents/vectors based on per-level retention policies
    if let Err(e) = register_prune_jobs(&scheduler, &db_path).await {
        warn!("Prune jobs not fully registered: {}", e);
    }

    info!(
        "Scheduler initialized with {} jobs",
        scheduler.registry().job_count()
    );

    // Write PID file
    write_pid_file()?;

    // Parse address
    let addr: SocketAddr = settings
        .grpc_addr()
        .parse()
        .context("Invalid gRPC address")?;

    // Create shutdown signal handler
    let shutdown_signal = async {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("Received Ctrl+C, shutting down...");
            }
            _ = terminate => {
                info!("Received SIGTERM, shutting down...");
            }
        }
    };

    // Start server with scheduler
    let result = run_server_with_scheduler(addr, storage, scheduler, shutdown_signal).await;

    // Cleanup
    remove_pid_file();

    result.map_err(|e| anyhow::anyhow!("Server error: {}", e))
}

/// Stop the running daemon by sending SIGTERM.
pub fn stop_daemon() -> Result<()> {
    let pid = read_pid_file().context("No PID file found - daemon may not be running")?;

    if !is_process_running(pid) {
        remove_pid_file();
        anyhow::bail!("Daemon not running (stale PID file removed)");
    }

    info!("Stopping daemon (PID {})", pid);

    #[cfg(unix)]
    {
        unsafe {
            if libc::kill(pid as i32, libc::SIGTERM) != 0 {
                anyhow::bail!("Failed to send SIGTERM to daemon");
            }
        }
        println!("Sent SIGTERM to daemon (PID {})", pid);
    }

    #[cfg(not(unix))]
    {
        anyhow::bail!("Stop command not yet implemented on this platform");
    }

    Ok(())
}

/// Show daemon status.
pub fn show_status() -> Result<()> {
    let pid_path = pid_file_path();

    match read_pid_file() {
        Some(pid) if is_process_running(pid) => {
            println!("Memory daemon is running (PID {})", pid);
            println!("PID file: {:?}", pid_path);
            Ok(())
        }
        Some(pid) => {
            println!(
                "Memory daemon is NOT running (stale PID {} in {:?})",
                pid, pid_path
            );
            Ok(())
        }
        None => {
            println!("Memory daemon is NOT running (no PID file)");
            Ok(())
        }
    }
}

/// Handle query commands.
pub async fn handle_query(endpoint: &str, command: QueryCommands) -> Result<()> {
    let mut client = MemoryClient::connect(endpoint)
        .await
        .context("Failed to connect to daemon")?;

    match command {
        QueryCommands::Root => {
            let nodes = client
                .get_toc_root()
                .await
                .context("Failed to get TOC root")?;
            if nodes.is_empty() {
                println!("No TOC nodes found.");
            } else {
                println!("Root TOC Nodes ({} found):\n", nodes.len());
                for node in nodes {
                    let level = level_to_string(node.level);
                    println!("  {} [{}]", node.title, level);
                    println!("    ID: {}", node.node_id);
                    println!("    Children: {}", node.child_node_ids.len());
                    println!();
                }
            }
        }

        QueryCommands::Node { node_id } => {
            match client
                .get_node(&node_id)
                .await
                .context("Failed to get node")?
            {
                Some(node) => {
                    print_node_details(&node);
                }
                None => {
                    println!("Node not found: {}", node_id);
                }
            }
        }

        QueryCommands::Browse {
            parent_id,
            limit,
            token,
        } => {
            let result = client
                .browse_toc(&parent_id, limit, token)
                .await
                .context("Failed to browse TOC")?;

            if result.children.is_empty() {
                println!("No children found for: {}", parent_id);
            } else {
                println!(
                    "Children of {} ({} found):\n",
                    parent_id,
                    result.children.len()
                );
                for child in result.children {
                    let level = level_to_string(child.level);
                    println!("  {} [{}]", child.title, level);
                    println!("    ID: {}", child.node_id);
                }
            }

            if result.has_more {
                if let Some(token) = result.continuation_token {
                    println!("\nMore results available. Use --token {}", token);
                }
            }
        }

        QueryCommands::Events { from, to, limit } => {
            let result = client
                .get_events(from, to, limit)
                .await
                .context("Failed to get events")?;

            if result.events.is_empty() {
                println!("No events found in time range.");
            } else {
                println!("Events ({} found):\n", result.events.len());
                for event in result.events {
                    let role = match event.role {
                        1 => "user",
                        2 => "assistant",
                        3 => "system",
                        4 => "tool",
                        _ => "unknown",
                    };
                    let text_preview = if event.text.len() > 80 {
                        format!("{}...", &event.text[..80])
                    } else {
                        event.text.clone()
                    };
                    println!("  [{}] {}: {}", event.timestamp_ms, role, text_preview);
                }
            }

            if result.has_more {
                println!("\nMore events available. Increase --limit to see more.");
            }
        }

        QueryCommands::Expand {
            grip_id,
            before,
            after,
        } => {
            let result = client
                .expand_grip(&grip_id, Some(before), Some(after))
                .await
                .context("Failed to expand grip")?;

            match result.grip {
                Some(grip) => {
                    println!("Grip: {}\n", grip.grip_id);
                    println!("Excerpt: {}\n", grip.excerpt);

                    if !result.events_before.is_empty() {
                        println!("=== Events Before ({}) ===", result.events_before.len());
                        for event in result.events_before {
                            println!("  {}", truncate_text(&event.text, 100));
                        }
                        println!();
                    }

                    if !result.excerpt_events.is_empty() {
                        println!("=== Excerpt Events ({}) ===", result.excerpt_events.len());
                        for event in result.excerpt_events {
                            println!("  {}", truncate_text(&event.text, 100));
                        }
                        println!();
                    }

                    if !result.events_after.is_empty() {
                        println!("=== Events After ({}) ===", result.events_after.len());
                        for event in result.events_after {
                            println!("  {}", truncate_text(&event.text, 100));
                        }
                    }
                }
                None => {
                    println!("Grip not found: {}", grip_id);
                }
            }
        }

        QueryCommands::Search {
            query,
            node,
            parent,
            fields,
            limit,
        } => handle_search(endpoint, query, node, parent, fields, limit).await?,
    }

    Ok(())
}

fn level_to_string(level: i32) -> &'static str {
    match level {
        l if l == ProtoTocLevel::Year as i32 => "Year",
        l if l == ProtoTocLevel::Month as i32 => "Month",
        l if l == ProtoTocLevel::Week as i32 => "Week",
        l if l == ProtoTocLevel::Day as i32 => "Day",
        l if l == ProtoTocLevel::Segment as i32 => "Segment",
        _ => "Unknown",
    }
}

fn print_node_details(node: &memory_service::pb::TocNode) {
    let level = level_to_string(node.level);
    println!("TOC Node: {}", node.title);
    println!("  ID: {}", node.node_id);
    println!("  Level: {}", level);
    println!("  Version: {}", node.version);
    println!(
        "  Time Range: {} - {}",
        node.start_time_ms, node.end_time_ms
    );

    if let Some(summary) = &node.summary {
        println!("\nSummary: {}", summary);
    }

    if !node.bullets.is_empty() {
        println!("\nBullets:");
        for bullet in &node.bullets {
            println!("  • {}", bullet.text);
            if !bullet.grip_ids.is_empty() {
                println!("    Grips: {}", bullet.grip_ids.join(", "));
            }
        }
    }

    if !node.keywords.is_empty() {
        println!("\nKeywords: {}", node.keywords.join(", "));
    }

    if !node.child_node_ids.is_empty() {
        println!("\nChildren ({}):", node.child_node_ids.len());
        for (i, child_id) in node.child_node_ids.iter().enumerate() {
            if i >= 10 {
                println!("  ... and {} more", node.child_node_ids.len() - 10);
                break;
            }
            println!("  {}", child_id);
        }
    }
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() > max_len {
        format!("{}...", &text[..max_len])
    } else {
        text.to_string()
    }
}

/// Handle search command.
///
/// Per SEARCH-01, SEARCH-02: Search TOC nodes for matching content.
async fn handle_search(
    endpoint: &str,
    query: String,
    node: Option<String>,
    parent: Option<String>,
    fields_str: Option<String>,
    limit: u32,
) -> Result<()> {
    use memory_service::pb::memory_service_client::MemoryServiceClient;

    let mut client = MemoryServiceClient::connect(endpoint.to_string())
        .await
        .context("Failed to connect to daemon")?;

    // Parse fields from comma-separated string
    let fields: Vec<i32> = fields_str
        .map(|s| {
            s.split(',')
                .filter_map(|f| match f.trim().to_lowercase().as_str() {
                    "title" => Some(ProtoSearchField::Title as i32),
                    "summary" => Some(ProtoSearchField::Summary as i32),
                    "bullets" => Some(ProtoSearchField::Bullets as i32),
                    "keywords" => Some(ProtoSearchField::Keywords as i32),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();

    if let Some(node_id) = node {
        // Search within single node
        let response = client
            .search_node(SearchNodeRequest {
                node_id: node_id.clone(),
                query: query.clone(),
                fields: fields.clone(),
                limit: limit as i32,
                token_budget: 0,
            })
            .await
            .context("SearchNode RPC failed")?;

        let resp = response.into_inner();
        println!("Search Results for node: {}", node_id);
        println!("Query: \"{}\"", query);
        println!("Matched: {}", resp.matched);
        println!();

        if resp.matches.is_empty() {
            println!("No matches found.");
        } else {
            for (i, m) in resp.matches.iter().enumerate() {
                let field_name = match ProtoSearchField::try_from(m.field) {
                    Ok(ProtoSearchField::Title) => "title",
                    Ok(ProtoSearchField::Summary) => "summary",
                    Ok(ProtoSearchField::Bullets) => "bullets",
                    Ok(ProtoSearchField::Keywords) => "keywords",
                    _ => "unknown",
                };
                println!("{}. [{}] score={:.2}", i + 1, field_name, m.score);
                println!("   Text: {}", truncate_text(&m.text, 100));
                if !m.grip_ids.is_empty() {
                    println!("   Grips: {}", m.grip_ids.join(", "));
                }
            }
        }
    } else {
        // Search children of parent (or root if no parent)
        let parent_id = parent.unwrap_or_default();
        let response = client
            .search_children(SearchChildrenRequest {
                parent_id: parent_id.clone(),
                query: query.clone(),
                child_level: 0, // Ignored when parent_id is provided
                fields: fields.clone(),
                limit: limit as i32,
                token_budget: 0,
            })
            .await
            .context("SearchChildren RPC failed")?;

        let resp = response.into_inner();
        let scope = if parent_id.is_empty() {
            "root level".to_string()
        } else {
            format!("children of {}", parent_id)
        };
        println!("Search Results for {}", scope);
        println!("Query: \"{}\"", query);
        println!("Found: {} nodes", resp.results.len());
        if resp.has_more {
            println!("(more results available, increase --limit)");
        }
        println!();

        if resp.results.is_empty() {
            println!("No matching nodes found.");
        } else {
            for result in resp.results {
                println!(
                    "Node: {} (score={:.2})",
                    result.node_id, result.relevance_score
                );
                println!("  Title: {}", result.title);
                println!("  Matches:");
                for m in result.matches.iter().take(3) {
                    let field_name = match ProtoSearchField::try_from(m.field) {
                        Ok(ProtoSearchField::Title) => "title",
                        Ok(ProtoSearchField::Summary) => "summary",
                        Ok(ProtoSearchField::Bullets) => "bullets",
                        Ok(ProtoSearchField::Keywords) => "keywords",
                        _ => "unknown",
                    };
                    println!("    - [{}] {}", field_name, truncate_text(&m.text, 80));
                }
                if result.matches.len() > 3 {
                    println!("    ... and {} more matches", result.matches.len() - 3);
                }
                println!();
            }
        }
    }

    Ok(())
}

/// Handle admin commands.
///
/// Per CLI-03: Admin commands include rebuild-toc, compact, status, rebuild-indexes.
pub fn handle_admin(db_path: Option<String>, command: AdminCommands) -> Result<()> {
    // Load settings to get default db_path if not provided
    let settings = Settings::load(None).context("Failed to load configuration")?;
    let db_path = db_path.unwrap_or_else(|| settings.db_path.clone());
    let expanded_path = shellexpand::tilde(&db_path).to_string();

    // Open storage directly (not via gRPC)
    let storage = Storage::open(std::path::Path::new(&expanded_path))
        .context(format!("Failed to open storage at {}", expanded_path))?;
    let storage = Arc::new(storage);

    match command {
        AdminCommands::Stats => {
            let stats = storage.get_stats().context("Failed to get stats")?;

            println!("Database Statistics");
            println!("===================");
            println!("Path: {}", expanded_path);
            println!();
            println!("Events:       {:>10}", stats.event_count);
            println!("TOC Nodes:    {:>10}", stats.toc_node_count);
            println!("Grips:        {:>10}", stats.grip_count);
            println!("Outbox:       {:>10}", stats.outbox_count);
            println!();
            println!("Disk Usage:   {:>10}", format_bytes(stats.disk_usage_bytes));
        }

        AdminCommands::Compact { cf } => match cf {
            Some(cf_name) => {
                println!("Compacting column family: {}", cf_name);
                storage
                    .compact_cf(&cf_name)
                    .context(format!("Failed to compact {}", cf_name))?;
                println!("Compaction complete.");
            }
            None => {
                println!("Compacting all column families...");
                storage.compact().context("Failed to compact")?;
                println!("Compaction complete.");
            }
        },

        AdminCommands::RebuildToc { from_date, dry_run } => {
            if dry_run {
                println!("DRY RUN - No changes will be made");
                println!();
            }

            let from_timestamp = if let Some(date_str) = from_date {
                // Parse YYYY-MM-DD date
                let date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                    .context(format!("Invalid date format: {}. Use YYYY-MM-DD", date_str))?;
                let datetime = date.and_hms_opt(0, 0, 0).unwrap();
                Some(chrono::Utc.from_utc_datetime(&datetime).timestamp_millis())
            } else {
                None
            };

            // Get events to process
            let start_ms = from_timestamp.unwrap_or(0);
            let end_ms = chrono::Utc::now().timestamp_millis();

            let events = storage
                .get_events_in_range(start_ms, end_ms)
                .context("Failed to query events")?;

            println!("Found {} events to process", events.len());

            if events.is_empty() {
                println!("No events to rebuild TOC from.");
                return Ok(());
            }

            if dry_run {
                println!();
                println!("Would process events from {} to {}", start_ms, end_ms);
                println!(
                    "First event timestamp: {}",
                    events.first().map(|(k, _)| k.timestamp_ms).unwrap_or(0)
                );
                println!(
                    "Last event timestamp: {}",
                    events.last().map(|(k, _)| k.timestamp_ms).unwrap_or(0)
                );
                println!();
                println!("To actually rebuild, run without --dry-run");
            } else {
                // TODO: Full TOC rebuild would require integrating with memory-toc
                // For now, just report what would be done
                println!();
                println!("TOC rebuild not yet fully implemented.");
                println!("This would require re-running segmentation and summarization.");
                println!("Events are intact and can be manually processed.");
            }
        }

        AdminCommands::RebuildIndexes {
            index,
            batch_size,
            force,
            search_path,
            vector_path,
        } => {
            handle_rebuild_indexes(
                storage,
                &expanded_path,
                &index,
                batch_size,
                force,
                search_path,
                vector_path,
            )?;
        }

        AdminCommands::IndexStats {
            search_path,
            vector_path,
        } => {
            handle_index_stats(&expanded_path, search_path, vector_path)?;
        }

        AdminCommands::ClearIndex {
            index,
            force,
            search_path,
            vector_path,
        } => {
            handle_clear_index(&index, force, search_path, vector_path, &expanded_path)?;
        }
    }

    Ok(())
}

/// Handle the rebuild-indexes command.
fn handle_rebuild_indexes(
    storage: Arc<Storage>,
    db_path: &str,
    index: &str,
    batch_size: usize,
    force: bool,
    search_path: Option<String>,
    vector_path: Option<String>,
) -> Result<()> {
    use memory_embeddings::EmbeddingModel;
    use memory_indexing::{
        rebuild_bm25_index, rebuild_vector_index, Bm25IndexUpdater, RebuildConfig,
        VectorIndexUpdater,
    };
    use memory_search::{SearchIndex, SearchIndexConfig, SearchIndexer};
    use memory_vector::{HnswConfig, HnswIndex, VectorMetadata};

    // Determine which indexes to rebuild
    let rebuild_bm25 = index == "all" || index == "bm25";
    let rebuild_vector = index == "all" || index == "vector";

    if !rebuild_bm25 && !rebuild_vector {
        anyhow::bail!("Invalid index type: {}. Use bm25, vector, or all.", index);
    }

    // Count documents to process
    let stats = storage.get_stats().context("Failed to get stats")?;
    let total_docs = stats.toc_node_count + stats.grip_count;

    if total_docs == 0 {
        println!("No documents found in storage to index.");
        return Ok(());
    }

    println!("Index Rebuild");
    println!("=============");
    println!("Storage path: {}", db_path);
    println!("Index type:   {}", index);
    println!(
        "Documents:    {} ({} TOC nodes, {} grips)",
        total_docs, stats.toc_node_count, stats.grip_count
    );
    println!("Batch size:   {}", batch_size);
    println!();

    // Confirmation prompt
    if !force {
        print!("This will rebuild the index from scratch. Continue? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let start_time = Instant::now();
    let config = RebuildConfig::default().with_batch_size(batch_size);

    // Progress callback that prints to console
    let progress_callback = ConsoleProgressCallback::new(batch_size);

    // Rebuild BM25 index
    if rebuild_bm25 {
        let search_dir = search_path
            .clone()
            .unwrap_or_else(|| format!("{}/search", db_path));
        let search_dir = shellexpand::tilde(&search_dir).to_string();
        let search_path = Path::new(&search_dir);

        println!("Rebuilding BM25 index at: {}", search_dir);

        // Create search directory if needed
        std::fs::create_dir_all(search_path).context("Failed to create search index directory")?;

        // Open or create search index
        let search_config = SearchIndexConfig::new(search_path);
        let search_index =
            SearchIndex::open_or_create(search_config).context("Failed to open search index")?;
        let indexer =
            Arc::new(SearchIndexer::new(&search_index).context("Failed to create search indexer")?);

        let updater = Bm25IndexUpdater::new(indexer, storage.clone());

        let progress = rebuild_bm25_index(storage.clone(), &updater, &config, &progress_callback)
            .map_err(|e| anyhow::anyhow!("BM25 rebuild failed: {}", e))?;

        println!();
        println!("BM25 index rebuilt:");
        println!("  TOC nodes: {}", progress.toc_nodes_indexed);
        println!("  Grips:     {}", progress.grips_indexed);
        println!("  Errors:    {}", progress.errors);
    }

    // Rebuild vector index
    if rebuild_vector {
        let vector_dir = vector_path
            .clone()
            .unwrap_or_else(|| format!("{}/vector", db_path));
        let vector_dir = shellexpand::tilde(&vector_dir).to_string();
        let vector_path = Path::new(&vector_dir);

        println!("Rebuilding vector index at: {}", vector_dir);

        // Create vector directory if needed
        std::fs::create_dir_all(vector_path).context("Failed to create vector index directory")?;

        // Create embedder
        let embedder = Arc::new(
            memory_embeddings::CandleEmbedder::load_default()
                .context("Failed to create embedder")?,
        );

        // Open or create HNSW index
        let hnsw_config = HnswConfig::new(embedder.info().dimension, vector_path);
        let hnsw_index = Arc::new(RwLock::new(
            HnswIndex::open_or_create(hnsw_config).context("Failed to open HNSW index")?,
        ));

        // Open metadata store
        let metadata_path = vector_path.join("metadata");
        std::fs::create_dir_all(&metadata_path).context("Failed to create metadata directory")?;
        let metadata = Arc::new(
            VectorMetadata::open(&metadata_path).context("Failed to open vector metadata")?,
        );

        let updater = VectorIndexUpdater::new(hnsw_index, embedder, metadata, storage.clone());

        let progress = rebuild_vector_index(storage.clone(), &updater, &config, &progress_callback)
            .map_err(|e| anyhow::anyhow!("Vector rebuild failed: {}", e))?;

        println!();
        println!("Vector index rebuilt:");
        println!("  TOC nodes: {}", progress.toc_nodes_indexed);
        println!("  Grips:     {}", progress.grips_indexed);
        println!("  Skipped:   {}", progress.skipped);
        println!("  Errors:    {}", progress.errors);
    }

    let elapsed = start_time.elapsed();
    println!();
    println!("Rebuild complete in {:.2}s", elapsed.as_secs_f64());

    Ok(())
}

/// Handle the index-stats command.
fn handle_index_stats(
    db_path: &str,
    search_path: Option<String>,
    vector_path: Option<String>,
) -> Result<()> {
    use memory_search::{SearchIndex, SearchIndexConfig};
    use memory_vector::{HnswConfig, HnswIndex, VectorIndex, VectorMetadata};

    println!("Search Index Statistics");
    println!("=======================");
    println!();

    // BM25 index stats
    let search_dir = search_path.unwrap_or_else(|| format!("{}/search", db_path));
    let search_dir = shellexpand::tilde(&search_dir).to_string();
    let search_path = Path::new(&search_dir);

    println!("BM25 Index:");
    println!("  Path: {}", search_dir);

    if search_path.exists() {
        match SearchIndex::open_or_create(SearchIndexConfig::new(search_path)) {
            Ok(index) => {
                // Create a searcher to get doc count
                match memory_search::TeleportSearcher::new(&index) {
                    Ok(searcher) => {
                        let doc_count = searcher.num_docs();
                        println!("  Documents: {}", doc_count);
                        println!("  Status:    Available");
                    }
                    Err(e) => {
                        println!("  Status:    Error creating searcher - {}", e);
                    }
                }
            }
            Err(e) => {
                println!("  Status:    Error - {}", e);
            }
        }
    } else {
        println!("  Status:    Not found");
    }

    println!();

    // Vector index stats
    let vector_dir = vector_path.unwrap_or_else(|| format!("{}/vector", db_path));
    let vector_dir = shellexpand::tilde(&vector_dir).to_string();
    let vector_path = Path::new(&vector_dir);

    println!("Vector Index:");
    println!("  Path: {}", vector_dir);

    if vector_path.exists() {
        // Try to get dimension from an existing index
        // Default to 384 for all-MiniLM-L6-v2
        let dimension = 384;
        let hnsw_config = HnswConfig::new(dimension, vector_path);

        match HnswIndex::open_or_create(hnsw_config) {
            Ok(index) => {
                println!("  Vectors:   {}", index.len());
                println!("  Dimension: {}", dimension);
                println!("  Status:    Available");
            }
            Err(e) => {
                println!("  Status:    Error - {}", e);
            }
        }

        // Metadata stats
        let metadata_path = vector_path.join("metadata");
        if metadata_path.exists() {
            match VectorMetadata::open(&metadata_path) {
                Ok(metadata) => {
                    println!("  Metadata:  Available");
                    if let Ok(count) = metadata.count() {
                        println!("  Entries:   {}", count);
                    }
                }
                Err(e) => {
                    println!("  Metadata:  Error - {}", e);
                }
            }
        }
    } else {
        println!("  Status:    Not found");
    }

    Ok(())
}

/// Handle the clear-index command.
fn handle_clear_index(
    index: &str,
    force: bool,
    search_path: Option<String>,
    vector_path: Option<String>,
    db_path: &str,
) -> Result<()> {
    let clear_bm25 = index == "all" || index == "bm25";
    let clear_vector = index == "all" || index == "vector";

    if !clear_bm25 && !clear_vector {
        anyhow::bail!("Invalid index type: {}. Use bm25, vector, or all.", index);
    }

    println!("Clear Index");
    println!("===========");

    // Confirmation prompt
    if !force {
        print!(
            "This will PERMANENTLY DELETE the {} index. Continue? [y/N] ",
            index
        );
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Clear BM25 index
    if clear_bm25 {
        let search_dir = search_path
            .clone()
            .unwrap_or_else(|| format!("{}/search", db_path));
        let search_dir = shellexpand::tilde(&search_dir).to_string();
        let search_path = Path::new(&search_dir);

        if search_path.exists() {
            println!("Removing BM25 index at: {}", search_dir);
            std::fs::remove_dir_all(search_path)
                .context("Failed to remove BM25 index directory")?;
            println!("BM25 index cleared.");
        } else {
            println!("BM25 index not found at: {}", search_dir);
        }
    }

    // Clear vector index
    if clear_vector {
        let vector_dir = vector_path
            .clone()
            .unwrap_or_else(|| format!("{}/vector", db_path));
        let vector_dir = shellexpand::tilde(&vector_dir).to_string();
        let vector_path = Path::new(&vector_dir);

        if vector_path.exists() {
            println!("Removing vector index at: {}", vector_dir);
            std::fs::remove_dir_all(vector_path)
                .context("Failed to remove vector index directory")?;
            println!("Vector index cleared.");
        } else {
            println!("Vector index not found at: {}", vector_dir);
        }
    }

    Ok(())
}

/// Console progress callback for rebuild operations.
struct ConsoleProgressCallback {
    batch_size: usize,
}

impl ConsoleProgressCallback {
    fn new(batch_size: usize) -> Self {
        Self { batch_size }
    }
}

impl memory_indexing::ProgressCallback for ConsoleProgressCallback {
    fn on_progress(&self, progress: &memory_indexing::RebuildProgress) {
        if progress
            .total_processed
            .is_multiple_of(self.batch_size as u64)
            && progress.total_processed > 0
        {
            println!(
                "  Progress: {} documents ({} TOC nodes, {} grips, {} errors)",
                progress.total_processed,
                progress.toc_nodes_indexed,
                progress.grips_indexed,
                progress.errors
            );
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Handle scheduler commands.
///
/// Per SCHED-05: Job status observable via CLI.
pub async fn handle_scheduler(endpoint: &str, command: SchedulerCommands) -> Result<()> {
    use memory_service::pb::memory_service_client::MemoryServiceClient;

    let mut client = MemoryServiceClient::connect(endpoint.to_string())
        .await
        .context("Failed to connect to daemon")?;

    match command {
        SchedulerCommands::Status => {
            let response = client
                .get_scheduler_status(GetSchedulerStatusRequest {})
                .await
                .context("Failed to get scheduler status")?
                .into_inner();

            let status_str = if response.scheduler_running {
                "RUNNING"
            } else {
                "STOPPED"
            };
            println!("Scheduler: {}", status_str);
            println!();

            if response.jobs.is_empty() {
                println!("No jobs registered.");
            } else {
                println!(
                    "{:<20} {:<12} {:<20} {:<20} {:<10} {:<10}",
                    "JOB", "STATUS", "LAST RUN", "NEXT RUN", "RUNS", "ERRORS"
                );
                println!("{}", "-".repeat(92));

                for job in response.jobs {
                    let status = if job.is_paused {
                        "PAUSED"
                    } else if job.is_running {
                        "RUNNING"
                    } else {
                        "IDLE"
                    };

                    let last_run = if job.last_run_ms > 0 {
                        format_timestamp(job.last_run_ms)
                    } else {
                        "Never".to_string()
                    };

                    let next_run = if job.next_run_ms > 0 && !job.is_paused {
                        format_timestamp(job.next_run_ms)
                    } else {
                        "-".to_string()
                    };

                    println!(
                        "{:<20} {:<12} {:<20} {:<20} {:<10} {:<10}",
                        job.job_name, status, last_run, next_run, job.run_count, job.error_count
                    );

                    // Show last result if there was an error
                    if job.last_result == JobResultStatus::Failed as i32 {
                        if let Some(error) = &job.last_error {
                            println!("  Last error: {}", error);
                        }
                    }
                }
            }
        }

        SchedulerCommands::Pause { job_name } => {
            let response = client
                .pause_job(PauseJobRequest {
                    job_name: job_name.clone(),
                })
                .await
                .context("Failed to pause job")?
                .into_inner();

            if response.success {
                println!("Job '{}' paused.", job_name);
            } else {
                println!(
                    "Failed to pause '{}': {}",
                    job_name,
                    response.error.unwrap_or_default()
                );
            }
        }

        SchedulerCommands::Resume { job_name } => {
            let response = client
                .resume_job(ResumeJobRequest {
                    job_name: job_name.clone(),
                })
                .await
                .context("Failed to resume job")?
                .into_inner();

            if response.success {
                println!("Job '{}' resumed.", job_name);
            } else {
                println!(
                    "Failed to resume '{}': {}",
                    job_name,
                    response.error.unwrap_or_default()
                );
            }
        }
    }

    Ok(())
}

/// Format a timestamp in milliseconds as a local time string.
fn format_timestamp(ms: i64) -> String {
    use chrono::{DateTime, Local, Utc};

    DateTime::<Utc>::from_timestamp_millis(ms)
        .map(|t| t.with_timezone(&Local))
        .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "Invalid".to_string())
}

/// Handle teleport commands.
///
/// Per TEL-01 through TEL-04: BM25 keyword search for teleporting to content.
/// Per VEC-01 through VEC-03: Vector semantic search for teleporting to content.
pub async fn handle_teleport_command(cmd: TeleportCommand) -> Result<()> {
    match cmd {
        TeleportCommand::Search {
            query,
            doc_type,
            limit,
            addr,
            ..
        } => teleport_search(&query, &doc_type, limit, &addr).await,
        TeleportCommand::VectorSearch {
            query,
            top_k,
            min_score,
            target,
            addr,
            ..
        } => vector_search(&query, top_k, min_score, &target, &addr).await,
        TeleportCommand::HybridSearch {
            query,
            top_k,
            mode,
            bm25_weight,
            vector_weight,
            target,
            addr,
            ..
        } => {
            hybrid_search(
                &query,
                top_k,
                &mode,
                bm25_weight,
                vector_weight,
                &target,
                &addr,
            )
            .await
        }
        TeleportCommand::Stats { addr } => teleport_stats(&addr).await,
        TeleportCommand::VectorStats { addr } => vector_stats(&addr).await,
        TeleportCommand::Rebuild { addr } => teleport_rebuild(&addr).await,
    }
}

/// Execute teleport search via gRPC.
async fn teleport_search(query: &str, doc_type: &str, limit: usize, addr: &str) -> Result<()> {
    println!("Searching for: \"{}\"", query);
    println!("Filter: {}, Limit: {}", doc_type, limit);
    println!();

    let mut client = MemoryClient::connect(addr)
        .await
        .context("Failed to connect to daemon")?;

    // Map doc_type string to enum value
    let doc_type_value = match doc_type.to_lowercase().as_str() {
        "toc" | "toc_node" => 1, // TeleportDocType::TocNode
        "grip" | "grips" => 2,   // TeleportDocType::Grip
        _ => 0,                  // TeleportDocType::Unspecified (all)
    };

    let response = client
        .teleport_search(query, doc_type_value, limit as i32)
        .await
        .context("Teleport search failed")?;

    if response.results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    println!("Found {} results:", response.results.len());
    println!("{:-<60}", "");

    for (i, result) in response.results.iter().enumerate() {
        let type_str = match result.doc_type {
            1 => "TOC",
            2 => "Grip",
            _ => "?",
        };

        println!(
            "{}. [{}] {} (score: {:.4})",
            i + 1,
            type_str,
            result.doc_id,
            result.score
        );

        if let Some(ref keywords) = result.keywords {
            if !keywords.is_empty() {
                println!("   Keywords: {}", keywords);
            }
        }
    }

    println!("{:-<60}", "");
    println!("Total documents in index: {}", response.total_docs);

    Ok(())
}

/// Show teleport index statistics.
async fn teleport_stats(addr: &str) -> Result<()> {
    let mut client = MemoryClient::connect(addr)
        .await
        .context("Failed to connect to daemon")?;

    // Use empty search to get total_docs
    let response = client
        .teleport_search("", 0, 0)
        .await
        .context("Failed to get index stats")?;

    println!("Teleport Index Statistics");
    println!("{:-<40}", "");
    println!("Total documents: {}", response.total_docs);

    Ok(())
}

/// Trigger index rebuild (placeholder - will be implemented in Phase 13).
async fn teleport_rebuild(_addr: &str) -> Result<()> {
    println!("Index rebuild not yet implemented.");
    println!("This will be available in Phase 13 (Outbox Index Ingestion).");
    Ok(())
}

/// Execute vector semantic search via gRPC.
async fn vector_search(
    query: &str,
    top_k: i32,
    min_score: f32,
    target: &str,
    addr: &str,
) -> Result<()> {
    println!("Vector Search: \"{}\"", query);
    println!(
        "Top-K: {}, Min Score: {:.2}, Target: {}",
        top_k, min_score, target
    );
    println!();

    let mut client = MemoryClient::connect(addr)
        .await
        .context("Failed to connect to daemon")?;

    // Map target string to enum value
    let target_value = match target.to_lowercase().as_str() {
        "toc" | "toc_node" => 1, // VectorTargetType::TocNode
        "grip" | "grips" => 2,   // VectorTargetType::Grip
        "all" => 3,              // VectorTargetType::All
        _ => 0,                  // VectorTargetType::Unspecified
    };

    let response = client
        .vector_teleport(query, top_k, min_score, target_value)
        .await
        .context("Vector search failed")?;

    if response.matches.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    println!("Found {} results:", response.matches.len());
    println!("{:-<70}", "");

    for (i, m) in response.matches.iter().enumerate() {
        println!(
            "{}. [{}] {} (score: {:.4})",
            i + 1,
            m.doc_type,
            m.doc_id,
            m.score
        );

        // Show text preview (truncated)
        let preview = truncate_text(&m.text_preview, 80);
        println!("   {}", preview);

        // Show timestamp if available
        if m.timestamp_ms > 0 {
            println!("   Time: {}", format_timestamp(m.timestamp_ms));
        }

        println!();
    }

    // Show index status if available
    if let Some(status) = &response.index_status {
        println!("{:-<70}", "");
        println!(
            "Index: {} vectors, dim={}, last updated: {}",
            status.vector_count, status.dimension, status.last_indexed
        );
    }

    Ok(())
}

/// Execute hybrid BM25 + vector search via gRPC.
async fn hybrid_search(
    query: &str,
    top_k: i32,
    mode: &str,
    bm25_weight: f32,
    vector_weight: f32,
    target: &str,
    addr: &str,
) -> Result<()> {
    println!("Hybrid Search: \"{}\"", query);
    println!(
        "Mode: {}, BM25 Weight: {:.2}, Vector Weight: {:.2}",
        mode, bm25_weight, vector_weight
    );
    println!();

    let mut client = MemoryClient::connect(addr)
        .await
        .context("Failed to connect to daemon")?;

    // Map mode string to enum value
    let mode_value = match mode.to_lowercase().as_str() {
        "vector-only" | "vector" => 1, // HybridMode::VectorOnly
        "bm25-only" | "bm25" => 2,     // HybridMode::Bm25Only
        "hybrid" => 3,                 // HybridMode::Hybrid
        _ => 0,                        // HybridMode::Unspecified
    };

    // Map target string to enum value
    let target_value = match target.to_lowercase().as_str() {
        "toc" | "toc_node" => 1, // VectorTargetType::TocNode
        "grip" | "grips" => 2,   // VectorTargetType::Grip
        "all" => 3,              // VectorTargetType::All
        _ => 0,                  // VectorTargetType::Unspecified
    };

    let response = client
        .hybrid_search(
            query,
            top_k,
            mode_value,
            bm25_weight,
            vector_weight,
            target_value,
        )
        .await
        .context("Hybrid search failed")?;

    // Show mode used and availability
    let mode_used = match response.mode_used {
        1 => "vector-only",
        2 => "bm25-only",
        3 => "hybrid",
        _ => "unknown",
    };
    println!(
        "Mode used: {} (BM25: {}, Vector: {})",
        mode_used,
        if response.bm25_available { "yes" } else { "no" },
        if response.vector_available {
            "yes"
        } else {
            "no"
        }
    );
    println!();

    if response.matches.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    println!("Found {} results:", response.matches.len());
    println!("{:-<70}", "");

    for (i, m) in response.matches.iter().enumerate() {
        println!(
            "{}. [{}] {} (score: {:.4})",
            i + 1,
            m.doc_type,
            m.doc_id,
            m.score
        );

        // Show text preview (truncated)
        let preview = truncate_text(&m.text_preview, 80);
        println!("   {}", preview);

        // Show timestamp if available
        if m.timestamp_ms > 0 {
            println!("   Time: {}", format_timestamp(m.timestamp_ms));
        }

        println!();
    }

    Ok(())
}

/// Show vector index statistics.
async fn vector_stats(addr: &str) -> Result<()> {
    let mut client = MemoryClient::connect(addr)
        .await
        .context("Failed to connect to daemon")?;

    let status = client
        .get_vector_index_status()
        .await
        .context("Failed to get vector index status")?;

    println!("Vector Index Statistics");
    println!("{:-<40}", "");
    println!(
        "Status:        {}",
        if status.available {
            "Available"
        } else {
            "Unavailable"
        }
    );
    println!("Vectors:       {}", status.vector_count);
    println!("Dimension:     {}", status.dimension);
    println!("Last Indexed:  {}", status.last_indexed);
    println!("Index Path:    {}", status.index_path);
    println!("Index Size:    {}", format_bytes(status.size_bytes as u64));

    Ok(())
}

/// Handle topics commands.
///
/// Per TOPIC-08: Topic graph discovery and navigation.
pub async fn handle_topics_command(cmd: TopicsCommand) -> Result<()> {
    match cmd {
        TopicsCommand::Status { addr } => topics_status(&addr).await,
        TopicsCommand::Explore { query, limit, addr } => topics_explore(&query, limit, &addr).await,
        TopicsCommand::Related {
            topic_id,
            rel_type,
            limit,
            addr,
        } => topics_related(&topic_id, rel_type.as_deref(), limit, &addr).await,
        TopicsCommand::Top { limit, days, addr } => topics_top(limit, days, &addr).await,
        TopicsCommand::RefreshScores { db_path } => topics_refresh_scores(db_path).await,
        TopicsCommand::Prune {
            days,
            force,
            db_path,
        } => topics_prune(days, force, db_path).await,
    }
}

/// Show topic graph status.
async fn topics_status(addr: &str) -> Result<()> {
    let mut client = MemoryClient::connect(addr)
        .await
        .context("Failed to connect to daemon")?;

    let status = client
        .get_topic_graph_status()
        .await
        .context("Failed to get topic graph status")?;

    println!("Topic Graph Status");
    println!("{:-<40}", "");
    println!(
        "Status:         {}",
        if status.available {
            "Available"
        } else {
            "Unavailable"
        }
    );
    println!("Topics:         {}", status.topic_count);
    println!("Relationships:  {}", status.relationship_count);
    println!("Last Updated:   {}", status.last_updated);

    Ok(())
}

/// Explore topics matching a query.
async fn topics_explore(query: &str, limit: u32, addr: &str) -> Result<()> {
    let mut client = MemoryClient::connect(addr)
        .await
        .context("Failed to connect to daemon")?;

    println!("Searching for topics: \"{}\"", query);
    println!();

    let topics = client
        .get_topics_by_query(query, limit)
        .await
        .context("Failed to search topics")?;

    if topics.is_empty() {
        println!("No topics found matching query.");
        return Ok(());
    }

    println!("Found {} topics:", topics.len());
    println!("{:-<70}", "");

    for (i, topic) in topics.iter().enumerate() {
        println!(
            "{}. {} (importance: {:.4})",
            i + 1,
            topic.label,
            topic.importance_score
        );
        println!("   ID: {}", topic.id);
        if !topic.keywords.is_empty() {
            println!("   Keywords: {}", topic.keywords.join(", "));
        }
        println!("   Created: {}", topic.created_at);
        println!("   Last Mention: {}", topic.last_mention);
        println!();
    }

    Ok(())
}

/// Show related topics.
async fn topics_related(
    topic_id: &str,
    rel_type: Option<&str>,
    limit: u32,
    addr: &str,
) -> Result<()> {
    let mut client = MemoryClient::connect(addr)
        .await
        .context("Failed to connect to daemon")?;

    println!("Finding topics related to: {}", topic_id);
    if let Some(rt) = rel_type {
        println!("Filtering by relationship type: {}", rt);
    }
    println!();

    let response = client
        .get_related_topics(topic_id, rel_type, limit)
        .await
        .context("Failed to get related topics")?;

    if response.related_topics.is_empty() {
        println!("No related topics found.");
        return Ok(());
    }

    println!("Found {} related topics:", response.related_topics.len());
    println!("{:-<70}", "");

    for (i, topic) in response.related_topics.iter().enumerate() {
        // Find the relationship for this topic
        let rel = response
            .relationships
            .iter()
            .find(|r| r.target_id == topic.id);

        let rel_info = rel
            .map(|r| format!("{} (strength: {:.2})", r.relationship_type, r.strength))
            .unwrap_or_default();

        println!(
            "{}. {} (importance: {:.4})",
            i + 1,
            topic.label,
            topic.importance_score
        );
        println!("   ID: {}", topic.id);
        if !rel_info.is_empty() {
            println!("   Relationship: {}", rel_info);
        }
        println!();
    }

    Ok(())
}

/// Show top topics by importance.
async fn topics_top(limit: u32, days: u32, addr: &str) -> Result<()> {
    let mut client = MemoryClient::connect(addr)
        .await
        .context("Failed to connect to daemon")?;

    println!("Top {} topics (last {} days):", limit, days);
    println!();

    let topics = client
        .get_top_topics(limit, days)
        .await
        .context("Failed to get top topics")?;

    if topics.is_empty() {
        println!("No topics found.");
        return Ok(());
    }

    println!("{:-<70}", "");

    for (i, topic) in topics.iter().enumerate() {
        println!(
            "{}. {} (importance: {:.4})",
            i + 1,
            topic.label,
            topic.importance_score
        );
        println!("   ID: {}", topic.id);
        if !topic.keywords.is_empty() {
            println!("   Keywords: {}", topic.keywords.join(", "));
        }
        println!("   Last Mention: {}", topic.last_mention);
        println!();
    }

    Ok(())
}

/// Refresh topic importance scores.
async fn topics_refresh_scores(db_path: Option<String>) -> Result<()> {
    use memory_topics::{config::ImportanceConfig, ImportanceScorer, TopicStorage};

    // Load settings to get default db_path if not provided
    let settings = Settings::load(None).context("Failed to load configuration")?;
    let db_path = db_path.unwrap_or_else(|| settings.db_path.clone());
    let expanded_path = shellexpand::tilde(&db_path).to_string();

    println!("Refreshing topic importance scores...");
    println!("Database: {}", expanded_path);
    println!();

    // Open storage directly
    let storage = Storage::open(std::path::Path::new(&expanded_path))
        .context(format!("Failed to open storage at {}", expanded_path))?;
    let storage = Arc::new(storage);
    let topic_storage = TopicStorage::new(storage);

    let scorer = ImportanceScorer::new(ImportanceConfig::default());
    let updated = topic_storage
        .refresh_importance_scores(&scorer)
        .context("Failed to refresh importance scores")?;

    println!("Refreshed {} topic importance scores.", updated);

    Ok(())
}

/// Prune stale topics.
async fn topics_prune(days: u32, force: bool, db_path: Option<String>) -> Result<()> {
    use memory_topics::{TopicLifecycleManager, TopicStorage};

    // Load settings to get default db_path if not provided
    let settings = Settings::load(None).context("Failed to load configuration")?;
    let db_path = db_path.unwrap_or_else(|| settings.db_path.clone());
    let expanded_path = shellexpand::tilde(&db_path).to_string();

    println!("Pruning stale topics...");
    println!("Database: {}", expanded_path);
    println!("Inactivity threshold: {} days", days);
    println!();

    // Confirmation prompt
    if !force {
        print!(
            "This will archive topics not mentioned in {} days. Continue? [y/N] ",
            days
        );
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Open storage directly
    let storage = Storage::open(std::path::Path::new(&expanded_path))
        .context(format!("Failed to open storage at {}", expanded_path))?;
    let storage = Arc::new(storage);
    let topic_storage = TopicStorage::new(storage);

    let mut manager = TopicLifecycleManager::new(&topic_storage);
    let pruned = manager
        .prune_stale_topics(days)
        .context("Failed to prune topics")?;

    println!("Pruned {} stale topics.", pruned);

    Ok(())
}

/// Handle retrieval commands.
///
/// Per Phase 17: Retrieval policy status, intent classification, and query routing.
pub async fn handle_retrieval_command(cmd: RetrievalCommand) -> Result<()> {
    match cmd {
        RetrievalCommand::Status { addr } => retrieval_status(&addr).await,
        RetrievalCommand::Classify {
            query,
            timeout_ms,
            addr,
        } => retrieval_classify(&query, timeout_ms, &addr).await,
        RetrievalCommand::Route {
            query,
            intent,
            limit,
            mode,
            timeout_ms,
            agent,
            addr,
        } => {
            retrieval_route(
                &query,
                intent.as_deref(),
                limit,
                mode.as_deref(),
                timeout_ms,
                agent.as_deref(),
                &addr,
            )
            .await
        }
    }
}

/// Show retrieval tier and layer availability.
async fn retrieval_status(addr: &str) -> Result<()> {
    use memory_service::pb::memory_service_client::MemoryServiceClient;
    use memory_service::pb::GetRetrievalCapabilitiesRequest;

    let mut client = MemoryServiceClient::connect(addr.to_string())
        .await
        .context("Failed to connect to daemon")?;

    let response = client
        .get_retrieval_capabilities(GetRetrievalCapabilitiesRequest {})
        .await
        .context("Failed to get retrieval capabilities")?
        .into_inner();

    // Map tier to string
    let tier_str = match response.tier {
        1 => "Full (Topics + Hybrid + Agentic)",
        2 => "Hybrid (BM25 + Vector + Agentic)",
        3 => "Semantic (Vector + Agentic)",
        4 => "Keyword (BM25 + Agentic)",
        5 => "Agentic (TOC only)",
        _ => "Unknown",
    };

    println!("Retrieval Capabilities");
    println!("{:-<50}", "");
    println!("Tier: {}", tier_str);
    println!();

    // Print layer statuses
    println!("Layer Availability:");
    if let Some(status) = response.bm25_status {
        let emoji = if status.healthy { "[ok]" } else { "[--]" };
        println!(
            "  {} BM25:    {} docs - {}",
            emoji,
            status.doc_count,
            status.message.unwrap_or_default()
        );
    }
    if let Some(status) = response.vector_status {
        let emoji = if status.healthy { "[ok]" } else { "[--]" };
        println!(
            "  {} Vector:  {} docs - {}",
            emoji,
            status.doc_count,
            status.message.unwrap_or_default()
        );
    }
    if let Some(status) = response.topics_status {
        let emoji = if status.healthy { "[ok]" } else { "[--]" };
        println!(
            "  {} Topics:  {} docs - {}",
            emoji,
            status.doc_count,
            status.message.unwrap_or_default()
        );
    }
    if let Some(status) = response.agentic_status {
        let emoji = if status.healthy { "[ok]" } else { "[--]" };
        println!(
            "  {} Agentic: {}",
            emoji,
            status.message.unwrap_or_default()
        );
    }

    println!();
    println!("Detection time: {}ms", response.detection_time_ms);

    if !response.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in response.warnings {
            println!("  - {}", warning);
        }
    }

    Ok(())
}

/// Classify query intent.
async fn retrieval_classify(query: &str, timeout_ms: Option<u64>, addr: &str) -> Result<()> {
    use memory_service::pb::memory_service_client::MemoryServiceClient;
    use memory_service::pb::ClassifyQueryIntentRequest;

    let mut client = MemoryServiceClient::connect(addr.to_string())
        .await
        .context("Failed to connect to daemon")?;

    let response = client
        .classify_query_intent(ClassifyQueryIntentRequest {
            query: query.to_string(),
            timeout_ms,
        })
        .await
        .context("Failed to classify query intent")?
        .into_inner();

    // Map intent to string
    let intent_str = match response.intent {
        1 => "Explore (discover patterns/themes)",
        2 => "Answer (evidence-backed result)",
        3 => "Locate (find exact snippet)",
        4 => "Time-boxed (best partial in N ms)",
        _ => "Unknown",
    };

    println!("Query Classification");
    println!("{:-<50}", "");
    println!("Query:      \"{}\"", query);
    println!("Intent:     {}", intent_str);
    println!("Confidence: {:.2}", response.confidence);
    println!("Reason:     {}", response.reason);

    if !response.matched_keywords.is_empty() {
        println!("Keywords:   {}", response.matched_keywords.join(", "));
    }

    if let Some(lookback) = response.lookback_ms {
        if lookback > 0 {
            let hours = lookback / 3_600_000;
            let days = hours / 24;
            if days > 0 {
                println!("Lookback:   {} days", days);
            } else if hours > 0 {
                println!("Lookback:   {} hours", hours);
            } else {
                println!("Lookback:   {} ms", lookback);
            }
        }
    }

    Ok(())
}

/// Route query through optimal layers.
async fn retrieval_route(
    query: &str,
    intent_override: Option<&str>,
    limit: u32,
    mode_override: Option<&str>,
    timeout_ms: Option<u64>,
    agent_filter: Option<&str>,
    addr: &str,
) -> Result<()> {
    use memory_service::pb::memory_service_client::MemoryServiceClient;
    use memory_service::pb::{
        ExecutionMode as ProtoExecMode, QueryIntent as ProtoIntent, RouteQueryRequest,
        StopConditions as ProtoStopConditions,
    };

    let mut client = MemoryServiceClient::connect(addr.to_string())
        .await
        .context("Failed to connect to daemon")?;

    // Parse intent override
    let intent_override = intent_override.map(|s| match s.to_lowercase().as_str() {
        "explore" => ProtoIntent::Explore as i32,
        "answer" => ProtoIntent::Answer as i32,
        "locate" => ProtoIntent::Locate as i32,
        "time-boxed" | "timeboxed" => ProtoIntent::TimeBoxed as i32,
        _ => ProtoIntent::Unspecified as i32,
    });

    // Parse mode override
    let mode_override = mode_override.map(|s| match s.to_lowercase().as_str() {
        "sequential" => ProtoExecMode::Sequential as i32,
        "parallel" => ProtoExecMode::Parallel as i32,
        "hybrid" => ProtoExecMode::Hybrid as i32,
        _ => ProtoExecMode::Unspecified as i32,
    });

    // Build stop conditions
    let stop_conditions = timeout_ms.map(|timeout| ProtoStopConditions {
        max_depth: 0,
        max_nodes: 0,
        max_rpc_calls: 0,
        max_tokens: 0,
        timeout_ms: timeout,
        beam_width: 0,
        min_confidence: 0.0,
    });

    let response = client
        .route_query(RouteQueryRequest {
            query: query.to_string(),
            intent_override,
            stop_conditions,
            mode_override,
            limit: limit as i32,
            agent_filter: agent_filter.map(|s| s.to_string()),
        })
        .await
        .context("Failed to route query")?
        .into_inner();

    println!("Query Routing");
    println!("{:-<70}", "");
    println!("Query: \"{}\"", query);

    // Print explanation
    if let Some(exp) = &response.explanation {
        let intent_str = match exp.intent {
            1 => "Explore",
            2 => "Answer",
            3 => "Locate",
            4 => "Time-boxed",
            _ => "Unknown",
        };
        let tier_str = match exp.tier {
            1 => "Full",
            2 => "Hybrid",
            3 => "Semantic",
            4 => "Keyword",
            5 => "Agentic",
            _ => "Unknown",
        };
        let mode_str = match exp.mode {
            1 => "Sequential",
            2 => "Parallel",
            3 => "Hybrid",
            _ => "Unknown",
        };
        let winner_str = match exp.winner {
            1 => "Topics",
            2 => "Hybrid",
            3 => "Vector",
            4 => "BM25",
            5 => "Agentic",
            _ => "Unknown",
        };

        println!();
        println!("Execution:");
        println!(
            "  Intent: {} | Tier: {} | Mode: {}",
            intent_str, tier_str, mode_str
        );
        println!("  Winner: {} - {}", winner_str, exp.why_winner);

        if exp.fallback_occurred {
            if let Some(reason) = &exp.fallback_reason {
                println!("  Fallback: {}", reason);
            }
        }

        println!("  Time: {}ms", exp.total_time_ms);
    }

    // Print results
    println!();
    if response.results.is_empty() {
        println!("No results found.");
    } else {
        println!("Results ({} found):", response.results.len());
        println!("{:-<70}", "");

        for (i, result) in response.results.iter().enumerate() {
            let layer_str = match result.source_layer {
                1 => "Topics",
                2 => "Hybrid",
                3 => "Vector",
                4 => "BM25",
                5 => "Agentic",
                _ => "?",
            };

            println!(
                "{}. [{}] {} (score: {:.4})",
                i + 1,
                layer_str,
                result.doc_id,
                result.score
            );

            if !result.text_preview.is_empty() {
                let preview = truncate_text(&result.text_preview, 80);
                println!("   {}", preview);
            }

            println!("   Type: {}", result.doc_type);
            if let Some(ref agent) = result.agent {
                println!("   Agent: {}", agent);
            }
            println!();
        }
    }

    // Print layers attempted
    if !response.layers_attempted.is_empty() {
        let layers: Vec<&str> = response
            .layers_attempted
            .iter()
            .map(|l| match *l {
                1 => "Topics",
                2 => "Hybrid",
                3 => "Vector",
                4 => "BM25",
                5 => "Agentic",
                _ => "?",
            })
            .collect();
        println!("Layers attempted: {}", layers.join(" -> "));
    }

    Ok(())
}

/// Handle agent discovery commands.
///
/// Per Phase 23: Cross-agent discovery.
pub async fn handle_agents_command(cmd: AgentsCommand) -> Result<()> {
    match cmd {
        AgentsCommand::List { addr } => agents_list(&addr).await,
        AgentsCommand::Activity {
            agent,
            from,
            to,
            bucket,
            addr,
        } => {
            agents_activity(
                agent.as_deref(),
                from.as_deref(),
                to.as_deref(),
                &bucket,
                &addr,
            )
            .await
        }
        AgentsCommand::Topics { agent, limit, addr } => {
            agents_topics(&agent, limit, &addr).await
        }
    }
}

/// List all contributing agents with summary statistics.
async fn agents_list(addr: &str) -> Result<()> {
    use memory_service::pb::memory_service_client::MemoryServiceClient;
    use memory_service::pb::ListAgentsRequest;

    let mut client = MemoryServiceClient::connect(addr.to_string())
        .await
        .context("Failed to connect to daemon")?;

    let response = client
        .list_agents(ListAgentsRequest {})
        .await
        .context("ListAgents RPC failed")?
        .into_inner();

    if response.agents.is_empty() {
        println!("No contributing agents found.");
        return Ok(());
    }

    println!("Contributing Agents:");
    println!(
        "  {:<16} {:<24} {:<24} {:>6}",
        "AGENT", "FIRST SEEN", "LAST SEEN", "NODES"
    );

    for agent in &response.agents {
        let first_seen = format_utc_timestamp(agent.first_seen_ms);
        let last_seen = format_utc_timestamp(agent.last_seen_ms);

        println!(
            "  {:<16} {:<24} {:<24} {:>6}",
            agent.agent_id, first_seen, last_seen, agent.event_count
        );
    }

    Ok(())
}

/// Show agent activity timeline.
async fn agents_activity(
    agent: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
    bucket: &str,
    addr: &str,
) -> Result<()> {
    use memory_service::pb::memory_service_client::MemoryServiceClient;
    use memory_service::pb::GetAgentActivityRequest;

    let mut client = MemoryServiceClient::connect(addr.to_string())
        .await
        .context("Failed to connect to daemon")?;

    // Parse --from/--to: if matches YYYY-MM-DD, convert to epoch ms; if numeric, use as-is
    let from_ms = from.map(parse_time_arg).transpose()?;
    let to_ms = to.map(parse_time_arg).transpose()?;

    let response = client
        .get_agent_activity(GetAgentActivityRequest {
            agent_id: agent.map(|s| s.to_string()),
            from_ms,
            to_ms,
            bucket: bucket.to_string(),
        })
        .await
        .context("GetAgentActivity RPC failed")?
        .into_inner();

    if response.buckets.is_empty() {
        println!("No agent activity found.");
        return Ok(());
    }

    println!("Agent Activity ({} buckets):", bucket);
    println!(
        "  {:<14} {:<16} {:>8}",
        "DATE", "AGENT", "EVENTS"
    );

    for b in &response.buckets {
        let date_str = format_utc_date(b.start_ms);
        println!(
            "  {:<14} {:<16} {:>8}",
            date_str, b.agent_id, b.event_count
        );
    }

    Ok(())
}

/// Show top topics for a specific agent.
async fn agents_topics(agent: &str, limit: u32, addr: &str) -> Result<()> {
    let mut client = MemoryClient::connect(addr)
        .await
        .context("Failed to connect to daemon")?;

    let topics = client
        .get_top_topics_for_agent(limit, 30, agent)
        .await
        .context("Failed to get topics for agent")?;

    if topics.is_empty() {
        println!("No topics found for agent '{}'.", agent);
        return Ok(());
    }

    println!("Top Topics for agent \"{}\":", agent);
    println!("  {:<4} {:<30} {:>10}  KEYWORDS", "#", "TOPIC", "IMPORTANCE");

    for (i, topic) in topics.iter().enumerate() {
        let keywords = if topic.keywords.is_empty() {
            String::new()
        } else {
            topic.keywords.join(", ")
        };
        println!(
            "  {:<4} {:<30} {:>10.4}  {}",
            i + 1,
            truncate_text(&topic.label, 28),
            topic.importance_score,
            truncate_text(&keywords, 40),
        );
    }

    Ok(())
}

/// Parse a time argument that can be either YYYY-MM-DD or Unix epoch milliseconds.
fn parse_time_arg(s: &str) -> Result<i64> {
    // Try parsing as integer (epoch ms) first
    if let Ok(ms) = s.parse::<i64>() {
        return Ok(ms);
    }

    // Try parsing as YYYY-MM-DD
    let date = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .context(format!("Invalid time format: {}. Use YYYY-MM-DD or epoch ms", s))?;
    let datetime = date.and_hms_opt(0, 0, 0).unwrap();
    Ok(chrono::Utc.from_utc_datetime(&datetime).timestamp_millis())
}

/// Format a Unix timestamp in milliseconds as a human-readable UTC string.
fn format_utc_timestamp(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "Invalid".to_string())
}

/// Handle CLOD format commands (convert and validate).
pub async fn handle_clod_command(cmd: ClodCliCommand) -> Result<()> {
    use crate::clod;
    use std::path::Path;

    match cmd {
        ClodCliCommand::Convert { input, target, out } => {
            let input_path = Path::new(&input);
            let out_path = Path::new(&out);

            let def = clod::parse_clod(input_path)?;

            let files = match target.to_lowercase().as_str() {
                "claude" => vec![clod::generate_claude(&def, out_path)?],
                "opencode" => vec![clod::generate_opencode(&def, out_path)?],
                "gemini" => vec![clod::generate_gemini(&def, out_path)?],
                "copilot" => vec![clod::generate_copilot(&def, out_path)?],
                "all" => clod::generate_all(&def, out_path)?,
                other => anyhow::bail!(
                    "Unknown target '{}'. Use: claude, opencode, gemini, copilot, all",
                    other
                ),
            };

            println!(
                "Generated {} file(s) from CLOD definition '{}':",
                files.len(),
                def.command.name
            );
            for f in &files {
                println!("  {}", f);
            }
        }
        ClodCliCommand::Validate { input } => {
            let input_path = Path::new(&input);
            let def = clod::parse_clod(input_path)?;

            let required_count = def.command.parameters.iter().filter(|p| p.required).count();
            let optional_count = def.command.parameters.len() - required_count;
            let step_count = def.process.as_ref().map_or(0, |p| p.steps.len());

            // Collect configured adapters
            let mut adapter_names = Vec::new();
            if let Some(ref adapters) = def.adapters {
                if adapters.claude.is_some() {
                    adapter_names.push("claude");
                }
                if adapters.opencode.is_some() {
                    adapter_names.push("opencode");
                }
                if adapters.gemini.is_some() {
                    adapter_names.push("gemini");
                }
                if adapters.copilot.is_some() {
                    adapter_names.push("copilot");
                }
            }

            println!(
                "Valid CLOD definition: {} v{}",
                def.command.name, def.command.version
            );
            println!(
                "  Parameters: {} ({} required, {} optional)",
                def.command.parameters.len(),
                required_count,
                optional_count
            );
            println!("  Steps: {}", step_count);
            if !adapter_names.is_empty() {
                println!("  Adapters: {}", adapter_names.join(", "));
            }
        }
    }

    Ok(())
}

/// Format a Unix timestamp in milliseconds as a date-only UTC string.
fn format_utc_date(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "Invalid".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_file_path() {
        let path = pid_file_path();
        assert!(path.ends_with("daemon.pid"));
        assert!(path
            .parent()
            .unwrap()
            .to_string_lossy()
            .contains("agent-memory"));
    }

    #[test]
    fn test_status_no_daemon() {
        // Just verify it doesn't panic
        let result = show_status();
        assert!(result.is_ok());
    }

    #[test]
    fn test_level_to_string() {
        assert_eq!(level_to_string(ProtoTocLevel::Year as i32), "Year");
        assert_eq!(level_to_string(ProtoTocLevel::Month as i32), "Month");
        assert_eq!(level_to_string(ProtoTocLevel::Segment as i32), "Segment");
    }

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("hello", 10), "hello");
        assert_eq!(truncate_text("hello world!", 5), "hello...");
    }

    #[test]
    fn test_parse_time_arg_epoch_ms() {
        assert_eq!(parse_time_arg("1707350400000").unwrap(), 1707350400000);
    }

    #[test]
    fn test_parse_time_arg_date() {
        let ms = parse_time_arg("2024-02-08").unwrap();
        // 2024-02-08 00:00:00 UTC = 1707350400000
        assert_eq!(ms, 1707350400000);
    }

    #[test]
    fn test_parse_time_arg_invalid() {
        assert!(parse_time_arg("not-a-date").is_err());
    }

    #[test]
    fn test_format_utc_timestamp() {
        let s = format_utc_timestamp(1707350400000);
        assert_eq!(s, "2024-02-08 00:00 UTC");
    }

    #[test]
    fn test_format_utc_date() {
        let s = format_utc_date(1707350400000);
        assert_eq!(s, "2024-02-08");
    }
}
