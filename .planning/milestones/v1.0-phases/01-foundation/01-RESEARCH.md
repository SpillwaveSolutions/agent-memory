# Phase 1: Foundation - Research

**Researched:** 2026-01-29
**Domain:** Rust storage layer (RocksDB), gRPC services (tonic), workspace organization, daemon process management
**Confidence:** HIGH

## Summary

Phase 1 establishes the foundation for the agent-memory system: storage layer with RocksDB, gRPC service with tonic, daemon binary with CLI, and layered configuration. Research focused on five areas: RocksDB setup patterns (column families, compaction, key encoding), tonic gRPC service setup (build.rs, proto compilation, health/reflection), Rust workspace organization, configuration patterns (config crate), and daemon process management.

The standard approach uses a multi-crate workspace with flat `crates/` layout, RocksDB with FIFO or Universal compaction for append-only workloads, tonic 0.14 for gRPC with tonic-health and tonic-reflection, config-rs for layered configuration, and clap for CLI with subcommands. Key encoding follows `evt:{ts_ms}:{ulid}` format where ULID bytes are naturally lexicographically sortable.

**Primary recommendation:** Start with workspace scaffolding, then RocksDB storage abstraction with column families, then gRPC proto definitions and tonic service, then config loading, and finally daemon CLI with start/stop/status commands.

## Standard Stack

The established libraries/tools for this phase:

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rocksdb | 0.24.0 | Embedded key-value storage | Mature LSM-tree, excellent write throughput, column family isolation, 31M+ downloads |
| tonic | 0.14.3 | gRPC server framework | Official Rust gRPC (partnership with gRPC team), async/await, tokio-native |
| prost | 0.14.3 | Protobuf serialization | Generates idiomatic Rust, pairs with tonic, tokio-rs maintained |
| config | 0.15.x | Layered configuration | 12-factor support, file/env/CLI sources, serde integration |
| clap | 4.x | CLI argument parsing | Derive macro for subcommands, industry standard for Rust CLIs |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tonic-build | 0.14.3 | Proto compilation in build.rs | Always - generates service traits from .proto files |
| tonic-health | 0.14.x | gRPC health checking | GRPC-03 requirement - standard health check protocol |
| tonic-reflection | 0.14.x | gRPC reflection for debugging | GRPC-04 requirement - service discovery for clients |
| ulid | 1.2.1 | Time-sortable unique IDs | Event IDs - lexicographically sortable, timestamp-encoded |
| thiserror | 2.0 | Error type definitions | Library crates - matchable error enums |
| anyhow | 2.0 | Error propagation | Binary crates - error context aggregation |
| tracing | 0.1 | Structured logging | All crates - async-aware, span-based observability |
| serde | 1.0.228 | Serialization framework | All data types - derive macros for config and storage |
| chrono | 0.4.x | Timestamp handling | Event timestamps - UTC milliseconds, serde support |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| rocksdb | sled | sled is alpha stage, unstable on-disk format, rewrite incomplete |
| rocksdb | redb | B-tree based, not optimized for append-only workloads |
| config | figment | figment is more flexible but config-rs has stronger 12-factor patterns |
| clap derive | structopt | structopt merged into clap 3+, clap is the successor |
| ulid | uuid v7 | UUID v7 works but ULID has better Rust ecosystem tooling |

**Installation:**
```toml
[workspace.dependencies]
# Core
rocksdb = { version = "0.24", features = ["multi-threaded-cf", "zstd"] }
tonic = "0.14"
prost = "0.14"
tonic-health = "0.14"
tonic-reflection = "0.14"
config = "0.15"
clap = { version = "4", features = ["derive"] }

# Supporting
ulid = { version = "1.2", features = ["serde"] }
thiserror = "2.0"
anyhow = "2.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
tokio = { version = "1.49", features = ["full"] }

[workspace.build-dependencies]
tonic-build = "0.14"
prost-build = "0.14"
```

## Architecture Patterns

### Recommended Project Structure

```
agent-memory/
├── Cargo.toml                    # Workspace root (virtual manifest)
├── proto/
│   └── memory.proto              # gRPC service definitions
├── crates/
│   ├── memory-types/             # Shared types (Event, TocNode, etc.)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── event.rs
│   │       ├── config.rs
│   │       └── error.rs
│   │
│   ├── memory-storage/           # RocksDB abstraction layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── db.rs             # RocksDB wrapper
│   │       ├── keys.rs           # Key encoding/decoding
│   │       ├── column_families.rs
│   │       └── checkpoint.rs
│   │
│   ├── memory-service/           # gRPC service implementation
│   │   ├── Cargo.toml
│   │   ├── build.rs              # tonic-build for proto
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs
│   │       └── ingest.rs
│   │
│   └── memory-daemon/            # Binary: the daemon
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs           # CLI, config loading, startup
│           └── commands.rs       # start/stop/status
│
└── tests/
    └── integration/              # Integration tests
```

### Pattern 1: Virtual Manifest Workspace

**What:** Root Cargo.toml is a virtual manifest (no `[package]` section), only `[workspace]`.
**When to use:** Always for multi-crate projects.
**Example:**
```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "crates/memory-types",
    "crates/memory-storage",
    "crates/memory-service",
    "crates/memory-daemon",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.82"
license = "MIT"

[workspace.dependencies]
# Centralized dependency versions (see Installation above)
```

Source: [Cargo Workspaces - The Rust Programming Language](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html), [Large Rust Workspaces](https://matklad.github.io/2021/08/22/large-rust-workspaces.html)

### Pattern 2: Column Family Isolation

**What:** Separate RocksDB column families for different data types with different access patterns.
**When to use:** Always - core architectural decision per ARCHITECTURE.md.
**Example:**
```rust
// Source: Context7 /websites/rs_rocksdb_0_24_0
use rocksdb::{DB, ColumnFamilyDescriptor, Options};

pub const CF_EVENTS: &str = "events";
pub const CF_TOC_NODES: &str = "toc_nodes";
pub const CF_TOC_LATEST: &str = "toc_latest";
pub const CF_GRIPS: &str = "grips";
pub const CF_OUTBOX: &str = "outbox";
pub const CF_CHECKPOINTS: &str = "checkpoints";

pub fn open_db(path: &Path) -> Result<DB, Error> {
    let mut db_opts = Options::default();
    db_opts.create_if_missing(true);
    db_opts.create_missing_column_families(true);

    // Per PITFALLS.md: Use Universal or FIFO compaction for append-only
    db_opts.set_compaction_style(rocksdb::DBCompactionStyle::Universal);

    let cf_descriptors = vec![
        ColumnFamilyDescriptor::new(CF_EVENTS, events_options()),
        ColumnFamilyDescriptor::new(CF_TOC_NODES, Options::default()),
        ColumnFamilyDescriptor::new(CF_TOC_LATEST, Options::default()),
        ColumnFamilyDescriptor::new(CF_GRIPS, Options::default()),
        ColumnFamilyDescriptor::new(CF_OUTBOX, outbox_options()),
        ColumnFamilyDescriptor::new(CF_CHECKPOINTS, Options::default()),
    ];

    DB::open_cf_descriptors(&db_opts, path, cf_descriptors)
}

fn events_options() -> Options {
    let mut opts = Options::default();
    // Append-only, enable compression
    opts.set_compression_type(rocksdb::DBCompressionType::Zstd);
    opts
}

fn outbox_options() -> Options {
    let mut opts = Options::default();
    // FIFO for queue-like behavior
    opts.set_compaction_style(rocksdb::DBCompactionStyle::Fifo);
    opts
}
```

Source: [RocksDB Column Families Wiki](https://github.com/facebook/rocksdb/wiki/column-families), Context7 `/websites/rs_rocksdb_0_24_0`

### Pattern 3: Time-Prefixed Key Encoding

**What:** Keys structured as `{prefix}:{timestamp_ms}:{ulid}` for efficient time-range scans.
**When to use:** All event storage (STOR-01 requirement).
**Example:**
```rust
use ulid::Ulid;

/// Key format: evt:{timestamp_ms}:{ulid}
/// Example: evt:1706540400000:01HN4QXKN6YWXVKZ3JMHP4BCDE
pub struct EventKey {
    pub timestamp_ms: i64,
    pub ulid: Ulid,
}

impl EventKey {
    pub fn new(timestamp_ms: i64) -> Self {
        Self {
            timestamp_ms,
            ulid: Ulid::new(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // Format: "evt:" + 13-byte timestamp + ":" + 26-byte ulid
        // ULID is already lexicographically sortable in string form
        format!("evt:{}:{}", self.timestamp_ms, self.ulid).into_bytes()
    }

    pub fn prefix_for_time_range(start_ms: i64, end_ms: i64) -> (Vec<u8>, Vec<u8>) {
        let start = format!("evt:{}:", start_ms).into_bytes();
        let end = format!("evt:{}:", end_ms).into_bytes();
        (start, end)
    }
}
```

Source: [Storing data in order](https://cornerwings.github.io/2019/10/lexical-sorting/), [ulid crate docs](https://docs.rs/ulid)

### Pattern 4: Atomic WriteBatch for Event + Outbox

**What:** Write event and outbox entry in single atomic batch (transactional outbox pattern).
**When to use:** All ingestion (ING-05 requirement).
**Example:**
```rust
// Source: Context7 /websites/rs_rocksdb_0_24_0
use rocksdb::WriteBatch;

pub fn ingest_event(
    db: &DB,
    event: &Event,
    outbox_entry: &OutboxEntry,
) -> Result<(), Error> {
    let events_cf = db.cf_handle(CF_EVENTS).unwrap();
    let outbox_cf = db.cf_handle(CF_OUTBOX).unwrap();

    let event_key = EventKey::new(event.timestamp_ms);
    let outbox_key = OutboxKey::next_sequence();

    let mut batch = WriteBatch::default();
    batch.put_cf(&events_cf, event_key.to_bytes(), event.encode()?);
    batch.put_cf(&outbox_cf, outbox_key.to_bytes(), outbox_entry.encode()?);

    // Atomic write - both succeed or both fail
    db.write(batch)?;
    Ok(())
}
```

Source: [Transactional Outbox Pattern](https://microservices.io/patterns/data/transactional-outbox.html), Context7 `/websites/rs_rocksdb_0_24_0`

### Pattern 5: Layered Configuration with config-rs

**What:** Load config from defaults, file, env vars, CLI flags in precedence order.
**When to use:** Daemon startup (CFG-01, CFG-02, CFG-03 requirements).
**Example:**
```rust
// Source: Context7 /rust-cli/config-rs
use config::{Config, File, Environment};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub db_path: String,
    pub grpc_port: u16,
    pub summarizer: SummarizerSettings,
}

#[derive(Debug, Deserialize)]
pub struct SummarizerSettings {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl Settings {
    pub fn load(cli_config_path: Option<&str>) -> Result<Self, config::ConfigError> {
        let config_dir = dirs::config_dir()
            .map(|p| p.join("agent-memory"))
            .unwrap_or_else(|| PathBuf::from("."));

        let builder = Config::builder()
            // 1. Defaults
            .set_default("db_path", "~/.local/share/agent-memory/db")?
            .set_default("grpc_port", 50051)?
            .set_default("summarizer.provider", "openai")?
            .set_default("summarizer.model", "gpt-4o-mini")?

            // 2. Config file (~/.config/agent-memory/config.toml)
            .add_source(
                File::with_name(&config_dir.join("config").to_string_lossy())
                    .required(false)
            )

            // 3. CLI-specified config file (optional)
            .add_source(
                cli_config_path
                    .map(|p| File::with_name(p).required(true))
                    .unwrap_or_else(|| File::with_name("").required(false))
            )

            // 4. Environment variables (MEMORY_DB_PATH, MEMORY_GRPC_PORT, etc.)
            .add_source(
                Environment::with_prefix("MEMORY")
                    .separator("_")
                    .try_parsing(true)
            );

        builder.build()?.try_deserialize()
    }
}
```

Source: Context7 `/rust-cli/config-rs`

### Pattern 6: CLI Subcommands with clap derive

**What:** Use clap derive macros for start/stop/status subcommands.
**When to use:** Daemon binary (CLI-01 requirement).
**Example:**
```rust
// Source: Context7 /websites/rs_clap
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "memory-daemon")]
#[command(about = "Agent memory daemon", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, global = true)]
    pub config: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the daemon
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,
    },
    /// Stop the running daemon
    Stop,
    /// Show daemon status
    Status,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { foreground } => {
            let settings = Settings::load(cli.config.as_deref()).unwrap();
            start_daemon(settings, foreground);
        }
        Commands::Stop => stop_daemon(),
        Commands::Status => show_status(),
    }
}
```

Source: Context7 `/websites/rs_clap`

### Pattern 7: tonic gRPC Service Setup

**What:** Proto compilation in build.rs, service trait implementation, health and reflection.
**When to use:** gRPC layer (GRPC-01 through GRPC-04 requirements).

**build.rs:**
```rust
// Source: Context7 /websites/rs_tonic
use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("memory_descriptor.bin"))
        .compile_protos(&["../../proto/memory.proto"], &["../../proto"])?;

    Ok(())
}
```

**lib.rs:**
```rust
pub mod pb {
    tonic::include_proto!("memory");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("memory_descriptor");
}
```

**server.rs:**
```rust
use tonic::transport::Server;
use tonic_health::server::health_reporter;
use tonic_reflection::server::Builder as ReflectionBuilder;

pub async fn run_server(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    // Health check service (GRPC-03)
    let (mut health_reporter, health_service) = health_reporter();
    health_reporter
        .set_serving::<MemoryServiceServer<MemoryServiceImpl>>()
        .await;

    // Reflection service (GRPC-04)
    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(pb::FILE_DESCRIPTOR_SET)
        .build()?;

    // Main service
    let memory_service = MemoryServiceImpl::new(db);

    Server::builder()
        .add_service(health_service)
        .add_service(reflection_service)
        .add_service(MemoryServiceServer::new(memory_service))
        .serve(addr)
        .await?;

    Ok(())
}
```

Source: Context7 `/websites/rs_tonic`, [tonic-reflection setup guide](https://medium.com/@drewjaja/how-to-add-grpc-reflection-with-rust-tonic-reflection-1f4e14e6750e)

### Anti-Patterns to Avoid

- **Single Column Family for All Data:** Cannot tune compaction per workload; range scans include irrelevant data. Use CF isolation.
- **UUID v4 Keys:** Not time-sortable; scatters time-adjacent events. Use ULID or timestamp-prefixed keys.
- **Synchronous Index Updates:** Slows ingestion. Use outbox pattern for async index updates.
- **Level Compaction for Append-Only:** Creates 20-80x write amplification. Use FIFO or Universal.
- **Mutable Events:** Complicates crash recovery. Events are append-only per ARCHITECTURE.md.
- **Nested Crate Folder Structure:** Creates navigation friction. Use flat `crates/` layout.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Layered config loading | Custom file+env parsing | config-rs | Edge cases in precedence, env var parsing, type coercion |
| CLI argument parsing | Manual arg iteration | clap derive | Subcommands, help generation, validation, shell completions |
| Time-sortable IDs | Custom timestamp+random | ulid crate | Monotonic generation, proper encoding, proven algorithm |
| gRPC health checks | Custom health endpoint | tonic-health | Follows official gRPC health protocol, client compatibility |
| gRPC reflection | Manual service listing | tonic-reflection | Standard protocol, works with grpcurl/Postman/etc. |
| Key encoding | String concatenation | Dedicated keys module | Prefix extraction, range bounds, type safety |
| Error types | String errors | thiserror/anyhow | Matchable errors, context chains, ?-operator ergonomics |

**Key insight:** Foundation phase is about wiring together proven crates, not inventing new patterns. Every custom solution here adds maintenance burden without unique value.

## Common Pitfalls

### Pitfall 1: RocksDB Write Amplification Explosion

**What goes wrong:** Level compaction with append-only workload creates 20-80x write amplification. SSD wear, latency spikes, write stalls.

**Why it happens:** Default RocksDB config optimized for read-heavy workloads with updates.

**How to avoid:** Configure FIFO or Universal compaction from the start.
```rust
db_opts.set_compaction_style(rocksdb::DBCompactionStyle::Universal);
// Or for outbox (queue-like):
cf_opts.set_compaction_style(rocksdb::DBCompactionStyle::Fifo);
```

**Warning signs:** `rocksdb.compaction.bytes.written` far exceeds application write volume.

Source: [PITFALLS.md - Pitfall 3](../research/PITFALLS.md), [RocksDB Universal Compaction](https://github.com/facebook/rocksdb/wiki/Universal-Compaction)

### Pitfall 2: Key Design Preventing Efficient Time Scans

**What goes wrong:** Keys without timestamp prefix require full database scan for time-range queries.

**Why it happens:** UUID-first keys scatter time-adjacent events across key space.

**How to avoid:** Time-prefix keys: `evt:{timestamp_ms}:{ulid}`. Configure prefix extractor.
```rust
// Enable prefix bloom filters
db_opts.set_prefix_extractor(rocksdb::SliceTransform::create_fixed_prefix(17)); // "evt:" + 13-digit timestamp
```

**Warning signs:** Time-range query latency grows linearly with total data.

Source: [PITFALLS.md - Pitfall 5](../research/PITFALLS.md)

### Pitfall 3: Ingestion Race Conditions

**What goes wrong:** Events from multiple sources arrive out of order; duplicates created.

**Why it happens:** Network latency variance, retry logic.

**How to avoid:** Idempotent writes using event_id as key (ING-03). Use source timestamp for ordering (ING-04).
```rust
// Check if event already exists before writing
if db.get_cf(&events_cf, event_key.to_bytes())?.is_some() {
    return Ok(()); // Idempotent - already ingested
}
```

**Warning signs:** Duplicate event IDs in storage.

Source: [PITFALLS.md - Pitfall 7](../research/PITFALLS.md)

### Pitfall 4: Memory Consumption During Compaction

**What goes wrong:** Compaction doubles memory usage temporarily, causing OOM.

**Why it happens:** Universal compaction holds old + new data during merge.

**How to avoid:** Allocate only 50-60% of system memory to RocksDB. Limit concurrent compactions.
```rust
db_opts.set_max_background_jobs(4);
db_opts.set_max_subcompactions(2);
// Block cache sizing (not full system memory)
let mut block_opts = rocksdb::BlockBasedOptions::default();
block_opts.set_block_cache(&rocksdb::Cache::new_lru_cache(256 * 1024 * 1024)); // 256MB
```

**Warning signs:** Memory spikes correlating with compaction.

Source: [PITFALLS.md - Pitfall 8](../research/PITFALLS.md)

### Pitfall 5: Inconsistent Timestamp Handling

**What goes wrong:** Different parts use different timestamp formats (UTC vs local, seconds vs milliseconds).

**Why it happens:** No standard established early.

**How to avoid:** Define canonical format once: milliseconds-since-Unix-epoch UTC everywhere.
```rust
pub type TimestampMs = i64;

pub fn now_ms() -> TimestampMs {
    chrono::Utc::now().timestamp_millis()
}
```

**Warning signs:** Off-by-one-hour errors in queries.

Source: [PITFALLS.md - Pitfall 9](../research/PITFALLS.md)

## Code Examples

Verified patterns from official sources:

### Proto Definition (memory.proto)

```protobuf
syntax = "proto3";

package memory;

service MemoryService {
    // Ingestion
    rpc IngestEvent(IngestEventRequest) returns (IngestEventResponse);
}

message Event {
    string event_id = 1;
    string session_id = 2;
    int64 timestamp_ms = 3;
    string role = 4;  // "user", "assistant", "system", "tool"
    string text = 5;
    map<string, string> metadata = 6;
}

message IngestEventRequest {
    Event event = 1;
}

message IngestEventResponse {
    string event_id = 1;
    bool created = 2;  // false if idempotent hit
}
```

### Storage Layer Init

```rust
// crates/memory-storage/src/lib.rs
use rocksdb::{DB, Options, ColumnFamilyDescriptor};
use std::path::Path;

pub struct Storage {
    db: DB,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);
        db_opts.set_compaction_style(rocksdb::DBCompactionStyle::Universal);

        let cfs = vec![
            ColumnFamilyDescriptor::new("events", Self::events_options()),
            ColumnFamilyDescriptor::new("toc_nodes", Options::default()),
            ColumnFamilyDescriptor::new("toc_latest", Options::default()),
            ColumnFamilyDescriptor::new("grips", Options::default()),
            ColumnFamilyDescriptor::new("outbox", Self::outbox_options()),
            ColumnFamilyDescriptor::new("checkpoints", Options::default()),
        ];

        let db = DB::open_cf_descriptors(&db_opts, path, cfs)?;
        Ok(Self { db })
    }

    fn events_options() -> Options {
        let mut opts = Options::default();
        opts.set_compression_type(rocksdb::DBCompressionType::Zstd);
        opts
    }

    fn outbox_options() -> Options {
        let mut opts = Options::default();
        opts.set_compaction_style(rocksdb::DBCompactionStyle::Fifo);
        opts
    }
}
```

### Daemon Main Entry Point

```rust
// crates/memory-daemon/src/main.rs
use clap::Parser;
use memory_daemon::{Cli, Commands};
use memory_service::run_server;
use memory_types::Settings;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let settings = Settings::load(cli.config.as_deref())?;

    match cli.command {
        Commands::Start { foreground } => {
            let addr: SocketAddr = format!("0.0.0.0:{}", settings.grpc_port).parse()?;
            tracing::info!("Starting memory daemon on {}", addr);

            if !foreground {
                // TODO: Daemonize (Phase 1 can start with foreground-only)
                tracing::warn!("Background mode not yet implemented, running in foreground");
            }

            run_server(addr, &settings).await?;
        }
        Commands::Stop => {
            // TODO: Send signal to running daemon
            tracing::info!("Stop command - not yet implemented");
        }
        Commands::Status => {
            // TODO: Check if daemon is running
            tracing::info!("Status command - not yet implemented");
        }
    }

    Ok(())
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| grpc-rust | tonic | 2020+ | tonic is now official Rust gRPC with async/await |
| structopt | clap derive | clap 3.0 (2022) | structopt merged into clap |
| failure | thiserror + anyhow | 2019-2020 | failure deprecated |
| Level compaction for logs | FIFO/Universal | Always was better | Level causes write amplification |
| Custom daemonization | systemd/launchd service | Modern practice | Let OS manage lifecycle |

**Deprecated/outdated:**
- **sled**: Still alpha in 2026, on-disk format unstable, not production-ready
- **grpcio**: C++ bindings, heavier than pure-Rust tonic
- **failure crate**: Deprecated, use thiserror for library errors
- **Double-fork daemonization**: Modern approach is to let systemd/launchd manage the process as a service

## Open Questions

Things that couldn't be fully resolved:

1. **Daemon Background Mode Implementation**
   - What we know: Can use `proc-daemon` crate or rely on systemd/launchd
   - What's unclear: Whether Phase 1 needs true daemonization or just foreground mode
   - Recommendation: Start with foreground-only for Phase 1; add daemonization if explicitly needed. Most modern deployments use systemd anyway.

2. **PID File Location**
   - What we know: Standard locations are `/var/run/memory-daemon.pid` or `~/.local/run/memory-daemon.pid`
   - What's unclear: Permission model for single-user vs system-wide installation
   - Recommendation: Use XDG base directory spec: `~/.local/run/agent-memory/daemon.pid`

3. **Graceful Shutdown Signal Handling**
   - What we know: `signal-hook` crate is standard for SIGINT/SIGTERM handling
   - What's unclear: Exact cleanup sequence (flush RocksDB WAL, close gRPC connections)
   - Recommendation: tokio::signal for async signal handling; RocksDB auto-flushes on close

## Sources

### Primary (HIGH confidence)

- Context7 `/websites/rs_rocksdb_0_24_0` - RocksDB Rust bindings API
- Context7 `/facebook/rocksdb` - RocksDB compaction and tuning
- Context7 `/websites/rs_tonic` - tonic gRPC framework
- Context7 `/rust-cli/config-rs` - Layered configuration
- Context7 `/websites/rs_clap` - CLI argument parsing
- [RocksDB Universal Compaction](https://github.com/facebook/rocksdb/wiki/Universal-Compaction)
- [Cargo Workspaces - The Rust Programming Language](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
- [ulid crate documentation](https://docs.rs/ulid)

### Secondary (MEDIUM confidence)

- [Large Rust Workspaces](https://matklad.github.io/2021/08/22/large-rust-workspaces.html) - Workspace organization patterns
- [tonic-reflection setup guide](https://medium.com/@drewjaja/how-to-add-grpc-reflection-with-rust-tonic-reflection-1f4e14e6750e) - gRPC reflection configuration
- [Signal handling - Command Line Applications in Rust](https://rust-cli.github.io/book/in-depth/signals.html) - Signal handling patterns
- [Building a Daemon using Rust](https://tuttlem.github.io/2024/11/16/building-a-daemon-using-rust.html) - Daemon process patterns

### Tertiary (LOW confidence)

- [Storing data in order](https://cornerwings.github.io/2019/10/lexical-sorting/) - Lexicographic key encoding (older article, concepts still valid)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Versions verified via Context7 and crates.io, patterns from official docs
- Architecture: HIGH - Patterns from RocksDB wiki, tonic examples, Rust book
- Pitfalls: HIGH - Documented in PITFALLS.md, verified with RocksDB tuning guide

**Research date:** 2026-01-29
**Valid until:** 2026-03-01 (stable stack, 30-day validity)

---
*Generated by GSD Phase Researcher, 2026-01-29*
