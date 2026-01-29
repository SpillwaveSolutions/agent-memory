# Technology Stack: Agent Memory System

**Project:** Conversational Memory System for AI Agents
**Researched:** 2026-01-29
**Overall Confidence:** HIGH

## Executive Summary

This document recommends the optimal 2026 Rust stack for building a local, append-only conversational memory system. The stack prioritizes production-ready crates with strong cross-platform support (macOS, Linux, Windows), avoiding experimental or unstable dependencies.

**Core principle:** Prefer the Tokio ecosystem for consistency and interoperability. All async code should use Tokio as the runtime, and where possible, prefer crates maintained by tokio-rs.

---

## Recommended Stack

### Core Runtime

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **tokio** | 1.49.0 | Async runtime | Industry standard, LTS releases (1.47.x until Sep 2026), powers tonic/prost ecosystem | HIGH |
| **bytes** | 1.11.0 | Byte buffers | Zero-copy networking, required by prost/tonic, 474M+ downloads | HIGH |

**Rationale:** Tokio is the de facto async runtime for Rust. Using tokio ensures compatibility with tonic (gRPC), tracing, and the broader ecosystem. The LTS policy provides stability guarantees.

**MSRV:** Tokio 1.49.0 requires Rust 1.71+. Prost 0.14.3 requires Rust 1.82+.

---

### Storage Layer (RocksDB)

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **rocksdb** | 0.24.0 | Append-only event storage | Mature, battle-tested LSM-tree, excellent write throughput, 31M+ downloads | HIGH |

**Rationale:** RocksDB via `rust-rocksdb` is the correct choice for append-only event storage with time-prefixed keys. LSM-trees are optimized for write-heavy workloads. The crate wraps Facebook's C++ RocksDB with Rust bindings.

**Cross-Platform Notes:**
- Linux x86_64: Native support, well-tested
- macOS: Works out of the box (both x86_64 and ARM64)
- Windows: Requires MSVC toolchain, some users report build friction

**Configuration Recommendations:**
```toml
[dependencies]
rocksdb = { version = "0.24", features = ["multi-threaded-cf", "zstd"] }
```

- Enable `multi-threaded-cf` for concurrent column family operations (needed for TOC nodes + events + grips)
- Enable `zstd` compression for storage efficiency on historical data

**What NOT to use:**
- **sled**: Alpha stage, unstable on-disk format, rewrite incomplete
- **redb**: B-tree based, not optimized for append-only workloads (better for read-heavy)
- **Fjall**: Winding down active development in 2026

---

### gRPC Layer (tonic/prost)

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **tonic** | 0.14.3 | gRPC server/client | Official Rust gRPC, async/await, TLS via rustls, moving to grpc-io org | HIGH |
| **prost** | 0.14.3 | Protobuf serialization | Generates idiomatic Rust, pairs with tonic, tokio-rs maintained | HIGH |
| **prost-build** | 0.14.3 | Build-time codegen | Compiles .proto files in build.rs | HIGH |
| **tonic-build** | 0.14.3 | gRPC codegen | Generates service traits from .proto | HIGH |

**Rationale:** Tonic is becoming the official gRPC implementation for Rust (partnership with gRPC team announced). It's built on hyper/tokio and provides bi-directional streaming, TLS, load balancing, and health checking.

**Configuration:**
```toml
[dependencies]
tonic = "0.14"
prost = "0.14"

[build-dependencies]
tonic-build = "0.14"
prost-build = "0.14"
```

**Build Requirements:**
- `protoc` must be installed system-wide (prost-build 0.11+ requires it)
- Install via: `brew install protobuf` (macOS), `apt install protobuf-compiler` (Linux), `choco install protoc` (Windows)

**Supporting Crates:**
```toml
tonic-health = "0.14"      # gRPC health checking service
tonic-reflection = "0.14"  # gRPC reflection for debugging
```

**What NOT to use:**
- **grpc-rust** (old): Deprecated, tonic supersedes it
- **grpcio**: Binds to grpc-sys (C++), heavier than pure-Rust tonic

---

### Full-Text Search (Tantivy/BM25)

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **tantivy** | 0.25.0 | BM25 search index | Lucene-inspired, 2x faster than Lucene in benchmarks, pure Rust | HIGH |

**Rationale:** Tantivy is the standard for embedded full-text search in Rust. It provides BM25 scoring, phrase queries, fuzzy matching, and boolean logic. Used by ParadeDB, Memgraph, Quickwit, and others in production.

**Cross-Platform Notes:**
- Linux (x86_64, i686): Fully supported
- macOS: Works well (ARM64 support confirmed)
- Windows: Works, though less frequently tested

**Configuration:**
```toml
[dependencies]
tantivy = "0.25"
```

**Key Features for This Project:**
- Schema-based field definitions (map to conversation segments)
- Segment-based architecture (efficient for append patterns)
- Custom tokenizers (for agent-specific content)
- Thread-safe for concurrent queries

**What NOT to use:**
- **MeiliSearch** (server): Overkill, requires separate process
- **sonic**: Less mature, fewer features

---

### Vector Similarity (HNSW)

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **hnsw_rs** | 0.3.3 | HNSW vector index | Pure Rust, excellent cross-platform support, memory-mapped data, filtering | HIGH |

**Rationale:** `hnsw_rs` provides a pure Rust HNSW implementation with broad platform support including ARM macOS, ARM Linux, and Windows. It supports L1, L2, Cosine, Jaccard, Hamming, and other distance metrics.

**Cross-Platform Notes:**
- aarch64-apple-darwin (ARM macOS): Verified
- aarch64-unknown-linux-gnu (ARM Linux): Verified
- i686-pc-windows-msvc (32-bit Windows): Verified
- x86_64-pc-windows-msvc (64-bit Windows): Verified
- x86_64-unknown-linux-gnu (64-bit Linux): Verified

**Configuration:**
```toml
[dependencies]
hnsw_rs = "0.3"
```

**Alternatives Considered:**

| Crate | Why Not |
|-------|---------|
| **hnswlib-rs** | Pure Rust, but decouples graph from vector storage (more complexity) |
| **usearch** | Bindings to C++ library, less Rust-native |
| **SWARC** | Newer, less battle-tested |
| **LanceDB** | Full database, overkill for embedded index |

**Note on Embedding Generation:** This stack is for storage/retrieval only. Embedding generation requires an external LLM API or local model (e.g., via `llama-cpp-rs` or API calls).

---

### Serialization

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **serde** | 1.0.228 | Serialization framework | De facto standard, 1B+ downloads | HIGH |
| **serde_json** | 1.x | JSON (config, debug) | Human-readable configs | HIGH |
| **rkyv** | 0.8.x | Zero-copy binary (optional) | Performance-critical paths | MEDIUM |

**Configuration:**
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**Rationale:** Serde is non-negotiable for Rust serialization. Use JSON for configuration and debugging, protobuf for wire format (via prost). Consider rkyv for internal high-performance paths if benchmarks warrant it.

---

### Identifiers

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **ulid** | 1.2.1 | Time-sortable IDs | Lexicographically sortable, 128-bit, UUID-compatible | HIGH |

**Rationale:** ULIDs are perfect for append-only event stores with time-prefixed keys. They're:
- Lexicographically sortable (key ordering in RocksDB)
- Timestamp-encoded (natural time ordering)
- UUID-compatible (easy interop)
- Monotonic generation supported

**Configuration:**
```toml
[dependencies]
ulid = { version = "1.2", features = ["serde"] }
```

**What NOT to use:**
- **uuid v4**: Not time-sortable
- **uuid v7**: Good alternative, but ULID has broader Rust ecosystem support
- **snowflake**: Requires coordination, overkill for local-first

---

### Error Handling

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **thiserror** | 2.0 | Error type definitions | Library-style matchable errors | HIGH |
| **anyhow** | 2.0 | Error propagation | Application-level error context | HIGH |

**Rationale:** Use `thiserror` for defining error enums in library code (gRPC service errors, storage errors). Use `anyhow` in binary/application code for aggregating errors with context.

**Configuration:**
```toml
[dependencies]
thiserror = "2.0"
anyhow = "2.0"
```

---

### Observability

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **tracing** | 0.1 | Structured logging/spans | Tokio ecosystem standard, async-aware | HIGH |
| **tracing-subscriber** | 0.3 | Log output formatting | Pluggable subscribers | HIGH |
| **opentelemetry** | 0.28+ | OTLP export (optional) | Production observability | MEDIUM |

**Rationale:** `tracing` is the standard for Rust observability. It provides structured logging with spans (not just log lines), which is essential for debugging async code. Integrates with OpenTelemetry for production deployments.

**Configuration:**
```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

---

### Time Handling

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **chrono** | 0.4.x | Date/time operations | Mature, widely used, fixed historical issues | HIGH |
| **chrono-tz** | 0.10+ | Timezone support | When timezone handling needed | MEDIUM |

**Rationale:** Chrono is the standard for date/time in Rust. For UTC-only timestamps (likely for this project), it's the clear choice. If complex timezone handling is needed, consider `jiff` as a newer alternative.

**Configuration:**
```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
```

---

### Configuration Management

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **config** | 0.15.x | Layered configuration | 12-factor app support, multiple formats | HIGH |

**Rationale:** The `config` crate provides layered configuration (files, env vars, defaults) with type-safe deserialization via serde.

**Configuration:**
```toml
[dependencies]
config = "0.15"
```

---

### Middleware (for gRPC interceptors)

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **tower** | 0.5.x | Service abstraction | Composable middleware, tonic integration | HIGH |
| **tower-http** | 0.6.x | HTTP-specific middleware | Logging, tracing, compression | HIGH |

**Rationale:** Tower provides the `Service` trait that tonic uses internally. Use it for building gRPC interceptors (auth, logging, rate limiting).

**Configuration:**
```toml
[dependencies]
tower = "0.5"
tower-http = { version = "0.6", features = ["trace"] }
```

---

### Testing

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **rstest** | 0.26.x | Fixture-based testing | Parameterized tests, async support | HIGH |
| **proptest** | 1.x | Property-based testing | Fuzzing-like coverage | MEDIUM |
| **tokio-test** | 0.4.x | Async test utilities | Tokio runtime for tests | HIGH |

**Configuration:**
```toml
[dev-dependencies]
rstest = "0.26"
proptest = "1"
tokio-test = "0.4"
tokio = { version = "1", features = ["test-util", "macros", "rt-multi-thread"] }
```

---

## Full Cargo.toml Template

```toml
[package]
name = "agent-memory"
version = "0.1.0"
edition = "2024"
rust-version = "1.82"  # Required by prost 0.14

[dependencies]
# Async Runtime
tokio = { version = "1.49", features = ["full"] }
bytes = "1.11"

# Storage
rocksdb = { version = "0.24", features = ["multi-threaded-cf", "zstd"] }

# gRPC
tonic = "0.14"
prost = "0.14"
tonic-health = "0.14"
tonic-reflection = "0.14"

# Search
tantivy = "0.25"

# Vector Similarity
hnsw_rs = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Identifiers
ulid = { version = "1.2", features = ["serde"] }

# Error Handling
thiserror = "2.0"
anyhow = "2.0"

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Time
chrono = { version = "0.4", features = ["serde"] }

# Configuration
config = "0.15"

# Middleware
tower = "0.5"
tower-http = { version = "0.6", features = ["trace"] }

[build-dependencies]
tonic-build = "0.14"
prost-build = "0.14"

[dev-dependencies]
rstest = "0.26"
proptest = "1"
tokio-test = "0.4"
```

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not Recommended |
|----------|-------------|-------------|---------------------|
| Storage | rocksdb | sled | Alpha, unstable format, rewrite incomplete |
| Storage | rocksdb | redb | B-tree, not optimized for append-only |
| Storage | rocksdb | Fjall | Development winding down 2026 |
| gRPC | tonic | grpcio | C++ bindings, heavier |
| Search | tantivy | MeiliSearch | Server-based, overkill |
| HNSW | hnsw_rs | usearch | C++ bindings |
| Time | chrono | time 0.3 | chrono more widely used |
| IDs | ulid | uuid v7 | ULID has better Rust tooling |

---

## Cross-Platform Build Considerations

### Linux
- **Target:** x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu
- **Notes:** Primary development target, all crates well-tested

### macOS
- **Target:** x86_64-apple-darwin, aarch64-apple-darwin (M1/M2/M3)
- **Notes:** RocksDB builds natively. Tantivy and hnsw_rs work well.

### Windows
- **Target:** x86_64-pc-windows-msvc
- **Notes:**
  - Requires Visual Studio Build Tools (C++ workload)
  - RocksDB may need `VCPKG` or manual LLVM setup for some builds
  - Test thoroughly in CI
  - Consider using `cargo-zigbuild` for cross-compilation

### CI Recommendations
```yaml
# GitHub Actions matrix
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest, windows-latest]
    include:
      - os: ubuntu-latest
        target: x86_64-unknown-linux-gnu
      - os: macos-latest
        target: aarch64-apple-darwin
      - os: windows-latest
        target: x86_64-pc-windows-msvc
```

---

## Sources

### Official Documentation
- [Tokio](https://tokio.rs/) - Async runtime
- [Tonic](https://docs.rs/tonic) - gRPC implementation
- [Prost](https://docs.rs/prost) - Protocol Buffers
- [RocksDB Rust](https://docs.rs/rocksdb) - Storage bindings
- [Tantivy](https://docs.rs/tantivy) - Full-text search
- [hnsw_rs](https://docs.rs/hnsw_rs) - Vector similarity

### Verification Sources
- [crates.io/rocksdb](https://crates.io/crates/rocksdb) - Version 0.24.0 (Aug 2025)
- [crates.io/tonic](https://crates.io/crates/tonic) - Version 0.14.3 (Jan 2026)
- [crates.io/tantivy](https://crates.io/crates/tantivy) - Version 0.25.0
- [crates.io/hnsw_rs](https://crates.io/crates/hnsw_rs) - Version 0.3.3
- [GitHub rust-rocksdb](https://github.com/rust-rocksdb/rust-rocksdb)
- [GitHub hyperium/tonic](https://github.com/hyperium/tonic)

### Community References
- [Rust Error Handling Guide 2025](https://markaicode.com/rust-error-handling-2025-guide/)
- [State of the Crates 2025](https://ohadravid.github.io/posts/2024-12-state-of-the-crates/)
- [gRPC-Rust Announcement](https://groups.google.com/g/grpc-io/c/ExbWWLaGHjI)

---

## Confidence Assessment

| Area | Level | Reason |
|------|-------|--------|
| Core Runtime (tokio) | HIGH | LTS releases, industry standard, verified docs.rs |
| Storage (rocksdb) | HIGH | 31M downloads, mature C++ backing, verified version |
| gRPC (tonic/prost) | HIGH | Official gRPC partnership, verified version |
| Search (tantivy) | HIGH | Used by Quickwit/ParadeDB in production |
| Vector (hnsw_rs) | HIGH | Cross-platform verified, pure Rust |
| Supporting crates | HIGH | All widely used, serde has 1B+ downloads |
| Cross-platform Windows | MEDIUM | RocksDB build friction reported, needs CI testing |

---

## Gaps to Address

1. **Embedding Generation:** This stack covers storage/retrieval but not embedding creation. Phase-specific research needed for local embedding options (llama-cpp-rs, ort, candle).

2. **Windows CI:** RocksDB on Windows needs explicit CI testing. Consider fallback to redb if Windows support is critical and RocksDB proves problematic.

3. **LLM Summarization Integration:** Pluggable LLM interface design needed. Consider `async-openai`, `llm-chain`, or direct HTTP clients.
