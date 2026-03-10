# Phase 36: Ingest Pipeline Wiring - Research

**Researched:** 2026-03-06
**Domain:** Wiring DedupGate into MemoryServiceImpl ingest path, store-and-skip-outbox, structural bypass, proto additions
**Confidence:** HIGH

## Summary

Phase 36 wires the Phase 35 DedupGate (NoveltyChecker + InFlightBuffer) into the actual `MemoryServiceImpl.ingest_event` gRPC handler. The core behavior change: duplicate events are still stored in RocksDB (preserving the append-only invariant) but skip the outbox entry, so they are never picked up by the background indexing pipeline (BM25, HNSW, TOC). Structural events (session_start, session_end, subagent_start, subagent_stop) bypass the dedup gate entirely and always get both storage and outbox entries.

The current `ingest_event` method (lines 316-372 in `ingest.rs`) follows a simple flow: validate -> convert proto -> serialize -> `storage.put_event(event_id, event_bytes, outbox_bytes)` -> return. The `put_event` method always writes both the event and an outbox entry atomically. Phase 36 must introduce an alternative storage path -- `put_event_only` -- that writes the event without an outbox entry, used when the dedup gate marks an event as duplicate. The NoveltyChecker is already fully built with `with_in_flight_buffer` constructor, `should_store()` async method, and `push_to_buffer()` for post-store embedding population.

Additionally, Phase 36 adds a `deduplicated` boolean field to `IngestEventResponse` in the proto, and a `GetDedupStatus` RPC for observability. The `MemoryServiceImpl` struct needs a new `novelty_checker: Option<NoveltyChecker>` field, constructed during daemon startup from `Settings.dedup` config.

**Primary recommendation:** Add `put_event_only` to Storage, inject NoveltyChecker into MemoryServiceImpl via a new constructor, modify `ingest_event` to call `should_store()` and branch on the result (store+outbox vs store-only), add structural event bypass using `EventType` matching, and extend the proto with `deduplicated` field (field number 3) on IngestEventResponse.

## Standard Stack

### Core (Already in Workspace)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| memory-service | workspace | MemoryServiceImpl, NoveltyChecker, ingest pipeline | Primary modification target |
| memory-storage | workspace | Storage.put_event, new put_event_only | Needs new method for store-without-outbox |
| memory-types | workspace | DedupConfig, InFlightBuffer, EventType, OutboxEntry | All types already exist |
| memory-embeddings | workspace | CandleEmbedder (384-dim, all-MiniLM-L6-v2) | Real embedder for production wiring |
| tonic | workspace | gRPC proto generation, Request/Response types | Proto changes auto-generate |
| async-trait | workspace | EmbedderTrait, VectorIndexTrait | Already used in novelty.rs |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tracing | workspace | Logging dedup decisions in ingest path | Every dedup branch logs |
| std::sync::RwLock | stdlib | InFlightBuffer locking | Sub-microsecond ops, no need for tokio RwLock |

### No New Dependencies Required

Phase 36 requires zero new crate dependencies. All code touches existing crates.

## Architecture Patterns

### Where Things Change

```
crates/
  memory-storage/src/
    db.rs               # ADD: put_event_only() method
  memory-service/src/
    ingest.rs           # MODIFY: MemoryServiceImpl fields, ingest_event flow
    novelty.rs          # EXISTING: NoveltyChecker (no changes needed)
  memory-types/src/
    event.rs            # ADD: EventType::is_structural() helper
  memory-daemon/src/
    commands.rs         # MODIFY: Wire NoveltyChecker in start_daemon
proto/
  memory.proto          # MODIFY: IngestEventResponse.deduplicated field, GetDedupStatus RPC
```

### Pattern 1: Store-and-Skip-Outbox

**What:** Duplicate events are stored in RocksDB (append-only preserved) but do NOT get an outbox entry, so the background indexing pipeline (BM25 index, HNSW vector index, TOC rollup) never processes them.

**When to use:** When `NoveltyChecker.should_store()` returns `false` (event is duplicate).

**Implementation approach:**

```rust
// In memory-storage/src/db.rs - NEW method
/// Store an event WITHOUT an outbox entry (DEDUP-03).
///
/// Used for duplicate events that should be preserved in the append-only
/// store but excluded from indexing pipelines. The event is still
/// retrievable by event_id.
///
/// Returns (event_key, created) where created=false if event already
/// existed (ING-03 idempotent).
pub fn put_event_only(
    &self,
    event_id: &str,
    event_bytes: &[u8],
) -> Result<(EventKey, bool), StorageError> {
    let events_cf = self.db.cf_handle(CF_EVENTS)
        .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_EVENTS.to_string()))?;

    let event_key = EventKey::from_event_id(event_id)?;

    // Idempotent check (ING-03)
    if self.db.get_cf(&events_cf, event_key.to_bytes())?.is_some() {
        return Ok((event_key, false));
    }

    // Write event only - no outbox entry
    self.db.put_cf(&events_cf, event_key.to_bytes(), event_bytes)?;

    Ok((event_key, true))
}
```

### Pattern 2: Structural Event Bypass (DEDUP-04)

**What:** Session lifecycle events bypass the dedup gate entirely and always get full storage + outbox treatment.

**When to use:** Before calling `should_store()` on the NoveltyChecker.

```rust
// In memory-types/src/event.rs - NEW helper method
impl EventType {
    /// Returns true for structural/lifecycle events that should bypass dedup.
    ///
    /// Per DEDUP-04: session_start, session_end, subagent_start, subagent_stop
    /// are always indexed regardless of content similarity.
    pub fn is_structural(&self) -> bool {
        matches!(
            self,
            EventType::SessionStart
                | EventType::SessionEnd
                | EventType::SubagentStart
                | EventType::SubagentStop
        )
    }
}
```

### Pattern 3: NoveltyChecker Injection into MemoryServiceImpl

**What:** Add NoveltyChecker as an optional field on MemoryServiceImpl, wired during daemon startup.

**When to use:** In the `ingest_event` method, checked before storage.

```rust
// Modified MemoryServiceImpl struct
pub struct MemoryServiceImpl {
    storage: Arc<Storage>,
    scheduler_service: Option<SchedulerGrpcService>,
    teleport_searcher: Option<Arc<TeleportSearcher>>,
    vector_service: Option<Arc<VectorTeleportHandler>>,
    hybrid_service: Option<Arc<HybridSearchHandler>>,
    topic_service: Option<Arc<TopicGraphHandler>>,
    retrieval_service: Option<Arc<RetrievalHandler>>,
    agent_service: Arc<AgentDiscoveryHandler>,
    novelty_checker: Option<Arc<NoveltyChecker>>,  // NEW
}
```

### Pattern 4: Modified Ingest Flow

**What:** The ingest_event method gains a dedup branch between event conversion and storage.

**Flow:**

```
1. Validate + convert proto event  (unchanged)
2. Serialize event                 (unchanged)
3. Check: is event_type structural?
   YES -> store with outbox (normal path)
   NO  -> call novelty_checker.should_store(&event)
          YES (novel) -> store with outbox + push_to_buffer
          NO (duplicate) -> store WITHOUT outbox (put_event_only)
4. Return IngestEventResponse with deduplicated flag
```

```rust
// Simplified ingest_event flow
let event = Self::convert_event(proto_event)?;
let event_id = event.event_id.clone();
let event_bytes = event.to_bytes().map_err(/* ... */)?;

let deduplicated = if event.event_type.is_structural() {
    // DEDUP-04: structural events always indexed
    false
} else if let Some(checker) = &self.novelty_checker {
    !checker.should_store(&event).await
} else {
    false
};

let (_, created) = if deduplicated {
    // Store event but skip outbox (DEDUP-03)
    self.storage.put_event_only(&event_id, &event_bytes)
        .map_err(/* ... */)?
} else {
    // Normal path: store event + outbox entry
    let outbox_entry = OutboxEntry::for_toc(event_id.clone(), timestamp_ms);
    let outbox_bytes = outbox_entry.to_bytes().map_err(/* ... */)?;
    self.storage.put_event(&event_id, &event_bytes, &outbox_bytes)
        .map_err(/* ... */)?
};

// Push novel event embedding to buffer for future dedup checks
// (only if not deduplicated and checker exists)
// NOTE: need the embedding from should_store - see Open Questions

Ok(Response::new(IngestEventResponse {
    event_id,
    created,
    deduplicated,  // NEW proto field
}))
```

### Pattern 5: Daemon Startup Wiring

**What:** In `start_daemon` (commands.rs), create NoveltyChecker from Settings.dedup config and inject into MemoryServiceImpl.

```rust
// In start_daemon function, after storage is opened:
let novelty_checker = if settings.dedup.enabled {
    let embedder = CandleEmbedder::new()?;
    let buffer = Arc::new(RwLock::new(
        InFlightBuffer::new(settings.dedup.buffer_capacity, 384)
    ));
    Some(Arc::new(NoveltyChecker::with_in_flight_buffer(
        Some(Arc::new(embedder)),
        buffer,
        settings.dedup.clone(),
    )))
} else {
    None
};
```

### Anti-Patterns to Avoid

- **Blocking the ingest RPC on embedding:** The `should_store` method already wraps the check in a timeout (default 50ms). The fail-open behavior means slow embeddings just pass through. Do NOT add additional timeout wrappers.
- **Modifying put_event to conditionally skip outbox:** Keep `put_event` and `put_event_only` as separate methods. Do NOT add a boolean parameter to `put_event` -- that creates a confusing API and loses the clarity of atomic-outbox-write semantics.
- **Storing embeddings redundantly:** The embedding generated for dedup checking is the same one that would be indexed by the HNSW pipeline. Do NOT store it in the event itself. The InFlightBuffer holds it transiently; HNSW indexing generates its own embedding from the outbox pipeline.
- **Breaking the existing constructor API:** MemoryServiceImpl has 8 constructors (`new`, `with_scheduler`, `with_search`, etc.). Do NOT refactor all of them. Add `novelty_checker: None` to all existing constructors and add a `set_novelty_checker` method or builder pattern.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Embedder for dedup | New embedding path | Existing CandleEmbedder via EmbedderTrait | Already validated, 384-dim, normalized output |
| Dedup logic | Custom if/else in ingest | NoveltyChecker.should_store() | Already handles all fail-open gates, timeout, metrics |
| Buffer management | Manual Vec in ingest | InFlightBuffer via NoveltyChecker | Already built with ring buffer semantics |
| Config loading | Manual dedup config | Settings.dedup (already wired) | DedupConfig already in Settings struct with serde alias |

**Key insight:** Phase 35 built everything needed for dedup detection. Phase 36 is pure plumbing -- connecting existing components into the existing ingest pipeline.

## Common Pitfalls

### Pitfall 1: push_to_buffer Requires the Embedding

**What goes wrong:** The `NoveltyChecker.should_store()` method generates an embedding internally but does not expose it. After storing a novel event, the caller needs the embedding to call `push_to_buffer(event_id, embedding)`, but the embedding is trapped inside `check_similarity`.

**Why it happens:** Phase 35 designed `should_store()` to be simple (returns bool). The embedding is generated inside and not returned.

**How to avoid:** Two options: (a) Modify `should_store` to return a `DedupResult` enum that includes the embedding when novel, so the caller can pass it to `push_to_buffer`. (b) Have `should_store` internally push to buffer when novel (auto-push). Option (b) contradicts the Phase 35 decision that "push_to_buffer is explicit (not auto-push in should_store) to avoid pushing for failed stores." So option (a) is correct -- return the embedding alongside the decision.

**Warning signs:** Buffer stays empty because embeddings are never pushed after novel events.

### Pitfall 2: Embedding CandleEmbedder as EmbedderTrait

**What goes wrong:** `CandleEmbedder` is in `memory-embeddings` crate but does not implement `EmbedderTrait` (which is defined in `memory-service/src/novelty.rs`). The trait is local to the novelty module.

**Why it happens:** Phase 35 defined `EmbedderTrait` as a local trait in novelty.rs for testability with mocks. The real `CandleEmbedder` lives in a different crate.

**How to avoid:** Create an adapter struct `CandleEmbedderAdapter` in `memory-service` that wraps `CandleEmbedder` and implements `EmbedderTrait`. This is a thin wrapper -- one `impl` block.

**Warning signs:** Type mismatch compilation errors when trying to pass CandleEmbedder to NoveltyChecker.

### Pitfall 3: Many MemoryServiceImpl Constructors

**What goes wrong:** MemoryServiceImpl has 8+ constructors (`new`, `with_scheduler`, `with_search`, `with_vector`, `with_topics`, `with_all_services`, `with_all_services_and_topics`, plus the full builder in `start_daemon`). Adding `novelty_checker` to all of them is tedious and error-prone.

**Why it happens:** Constructor proliferation from incremental feature additions across phases.

**How to avoid:** Add `novelty_checker: None` to all existing constructors (preserving backward compat), then add a single `set_novelty_checker(&mut self, checker: Arc<NoveltyChecker>)` method. The daemon startup code calls this after construction. Alternatively, only modify the constructors actually used in production (`with_all_services_and_topics` and variants used by `start_daemon`).

**Warning signs:** Tests failing because test constructors don't compile.

### Pitfall 4: Proto Field Number Collision

**What goes wrong:** Adding `deduplicated` field to `IngestEventResponse` with a field number that conflicts with existing or future fields.

**Why it happens:** Proto field numbers are permanent. The current `IngestEventResponse` has fields 1 (event_id) and 2 (created).

**How to avoid:** Use field number 3 for `deduplicated`. This is the next sequential number. For the `GetDedupStatus` RPC, use field numbers that are clearly in the Phase 36 range.

**Warning signs:** Proto compilation errors, wire format incompatibility.

### Pitfall 5: Score Polarity Inversion in should_store

**What goes wrong:** `NoveltyChecker.check_similarity()` (line 338 of novelty.rs) uses `*score <= self.config.threshold` to mean "novel" (true = store). This means `should_store` returns `true` when score is LOW (event is novel/different) and `false` when score is HIGH (event is duplicate). The `deduplicated` field in the response should be `true` when `should_store` returns `false`.

**Why it happens:** The polarity is inverted from intuition: "should store" = "is novel" = NOT deduplicated.

**How to avoid:** `deduplicated = !should_store()` for non-structural events. Double-check with a unit test: identical text -> `should_store()` returns false -> `deduplicated` = true.

**Warning signs:** Response says `deduplicated: false` for obvious duplicates.

### Pitfall 6: Daemon Startup CandleEmbedder Initialization Failure

**What goes wrong:** CandleEmbedder downloads/loads the ML model on first use. If this fails (disk space, permissions, network), the daemon should still start with dedup disabled (fail-open at startup level).

**Why it happens:** Model loading can fail in production environments.

**How to avoid:** Wrap CandleEmbedder creation in a try block. On failure, log a warning and set `novelty_checker = None`. The NoveltyChecker already handles the `no embedder` case as fail-open.

**Warning signs:** Daemon crashes on startup instead of degrading gracefully.

## Code Examples

### Storage: put_event_only

```rust
// In memory-storage/src/db.rs
/// Store an event WITHOUT writing an outbox entry.
///
/// Used by the dedup gate (DEDUP-03): duplicate events are preserved
/// in the append-only store but excluded from background indexing.
/// Idempotent: returns created=false if event already exists (ING-03).
pub fn put_event_only(
    &self,
    event_id: &str,
    event_bytes: &[u8],
) -> Result<(EventKey, bool), StorageError> {
    let events_cf = self
        .db
        .cf_handle(CF_EVENTS)
        .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_EVENTS.to_string()))?;

    let event_key = EventKey::from_event_id(event_id)?;

    // Idempotent check (ING-03)
    if self.db.get_cf(&events_cf, event_key.to_bytes())?.is_some() {
        debug!("Event {} already exists, skipping", event_id);
        return Ok((event_key, false));
    }

    self.db
        .put_cf(&events_cf, event_key.to_bytes(), event_bytes)?;
    debug!("Stored duplicate event {} (no outbox)", event_id);

    Ok((event_key, true))
}
```

### EventType::is_structural

```rust
// In memory-types/src/event.rs
impl EventType {
    /// Returns true for structural lifecycle events that bypass dedup (DEDUP-04).
    pub fn is_structural(&self) -> bool {
        matches!(
            self,
            EventType::SessionStart
                | EventType::SessionEnd
                | EventType::SubagentStart
                | EventType::SubagentStop
        )
    }
}
```

### CandleEmbedderAdapter

```rust
// In memory-service/src/novelty.rs (or a new adapter module)
use memory_embeddings::CandleEmbedder;

/// Adapter bridging CandleEmbedder to the EmbedderTrait interface.
pub struct CandleEmbedderAdapter {
    embedder: CandleEmbedder,
}

impl CandleEmbedderAdapter {
    pub fn new(embedder: CandleEmbedder) -> Self {
        Self { embedder }
    }
}

#[async_trait::async_trait]
impl EmbedderTrait for CandleEmbedderAdapter {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        self.embedder
            .embed(text)
            .map(|embedding| embedding.values().to_vec())
            .map_err(|e| e.to_string())
    }
}
```

### Proto Changes

```protobuf
// In proto/memory.proto

// Response from event ingestion
message IngestEventResponse {
    string event_id = 1;
    bool created = 2;
    // True if event was detected as a duplicate and stored without indexing (DEDUP-03)
    bool deduplicated = 3;
}

// Dedup status (observability)
message GetDedupStatusRequest {}

message GetDedupStatusResponse {
    bool enabled = 1;
    float threshold = 2;
    uint64 events_checked = 3;
    uint64 events_deduplicated = 4;
    uint64 events_skipped = 5;
    uint32 buffer_size = 6;
    uint32 buffer_capacity = 7;
}
```

### Modified ingest_event (sketch)

```rust
async fn ingest_event(
    &self,
    request: Request<IngestEventRequest>,
) -> Result<Response<IngestEventResponse>, Status> {
    let req = request.into_inner();
    let proto_event = req.event
        .ok_or_else(|| Status::invalid_argument("Event is required"))?;

    if proto_event.event_id.is_empty() {
        return Err(Status::invalid_argument("event_id is required"));
    }
    if proto_event.session_id.is_empty() {
        return Err(Status::invalid_argument("session_id is required"));
    }

    let event = Self::convert_event(proto_event)?;
    let event_id = event.event_id.clone();
    let timestamp_ms = event.timestamp_ms();

    let event_bytes = event.to_bytes().map_err(|e| {
        error!("Failed to serialize event: {}", e);
        Status::internal("Failed to serialize event")
    })?;

    // Dedup gate: structural events bypass, others get checked
    let deduplicated = if event.event_type.is_structural() {
        false  // DEDUP-04: always index structural events
    } else if let Some(ref checker) = self.novelty_checker {
        !checker.should_store(&event).await
    } else {
        false  // No checker configured = no dedup
    };

    let (_, created) = if deduplicated {
        // DEDUP-03: store event but skip outbox (no indexing)
        info!(event_id = %event_id, "Dedup: storing without outbox");
        self.storage.put_event_only(&event_id, &event_bytes)
            .map_err(|e| {
                error!("Failed to store event: {}", e);
                Status::internal(format!("Storage error: {}", e))
            })?
    } else {
        // Normal path: store + outbox
        let outbox_entry = OutboxEntry::for_toc(event_id.clone(), timestamp_ms);
        let outbox_bytes = outbox_entry.to_bytes().map_err(|e| {
            error!("Failed to serialize outbox entry: {}", e);
            Status::internal("Failed to serialize outbox entry")
        })?;
        self.storage.put_event(&event_id, &event_bytes, &outbox_bytes)
            .map_err(|e| {
                error!("Failed to store event: {}", e);
                Status::internal(format!("Storage error: {}", e))
            })?
    };

    if created {
        info!("Stored event: {} (deduplicated: {})", event_id, deduplicated);
    }

    Ok(Response::new(IngestEventResponse {
        event_id,
        created,
        deduplicated,
    }))
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| All events get outbox entry | Dedup events skip outbox | Phase 36 | Indexes stay clean of duplicates |
| NoveltyChecker not wired | Injected into MemoryServiceImpl | Phase 36 | Dedup active in production |
| IngestEventResponse has 2 fields | Gains `deduplicated` field | Phase 36 | Clients know dedup status |
| No CandleEmbedder adapter | CandleEmbedderAdapter bridges to EmbedderTrait | Phase 36 | Real embeddings in production |

## Open Questions

1. **How does push_to_buffer get the embedding after should_store?**
   - What we know: `should_store()` returns `bool`. The embedding is generated internally. `push_to_buffer(event_id, embedding)` needs the embedding vector.
   - What's unclear: Whether to modify `should_store` to return the embedding or auto-push internally.
   - Recommendation: Add a `should_store_with_embedding()` method that returns `(bool, Option<Vec<f32>>)` -- the bool is the store decision, the Option<Vec<f32>> is the embedding (present when embedding succeeded). Alternatively, rename to return a `DedupResult` struct. The caller then calls `push_to_buffer` only after confirmed storage. This preserves the Phase 35 decision of explicit push.

2. **Should CandleEmbedder.embed() be sync or async?**
   - What we know: CandleEmbedder likely runs inference synchronously on CPU. The `EmbedderTrait::embed` is async.
   - What's unclear: Whether wrapping sync CPU inference in async adds meaningful overhead.
   - Recommendation: Use `tokio::task::spawn_blocking` inside the adapter if CandleEmbedder::embed is CPU-intensive, to avoid blocking the tokio runtime. Verify by checking the CandleEmbedder implementation.

3. **How to handle existing tests that construct MemoryServiceImpl?**
   - What we know: Many integration tests use `MemoryServiceImpl::new(storage)` and other constructors.
   - What's unclear: How many tests need updating.
   - Recommendation: Add `novelty_checker: None` to all existing constructors. Tests continue to work unchanged. Only new tests exercise the dedup path.

## Sources

### Primary (HIGH confidence)
- `crates/memory-service/src/ingest.rs` (lines 48-372) -- MemoryServiceImpl struct and ingest_event implementation
- `crates/memory-service/src/novelty.rs` -- NoveltyChecker with should_store(), push_to_buffer(), with_in_flight_buffer()
- `crates/memory-storage/src/db.rs` (lines 84-122) -- Storage.put_event() with atomic outbox write
- `crates/memory-types/src/config.rs` -- DedupConfig in Settings struct, already wired with serde alias
- `crates/memory-types/src/event.rs` -- EventType enum (SessionStart, SessionEnd, SubagentStart, SubagentStop)
- `crates/memory-types/src/outbox.rs` -- OutboxEntry for_toc/for_index constructors
- `proto/memory.proto` (lines 195-201) -- IngestEventResponse with fields 1 (event_id) and 2 (created)
- `crates/memory-daemon/src/commands.rs` (lines 290-435) -- start_daemon wiring, currently no NoveltyChecker
- `crates/memory-types/src/dedup.rs` -- InFlightBuffer implementation
- `.planning/phases/35-dedup-gate-foundation/35-RESEARCH.md` -- Phase 35 research findings

### Secondary (MEDIUM confidence)
- `.planning/STATE.md` -- Prior decisions on store-and-skip-outbox, explicit push_to_buffer, std::sync::RwLock
- `.planning/REQUIREMENTS.md` -- DEDUP-02, DEDUP-03, DEDUP-04 requirement definitions

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all code is local, no external dependencies, all crates verified
- Architecture: HIGH -- ingest pipeline is simple and well-understood, modification points clearly identified
- Pitfalls: HIGH -- embedding accessibility gap (Pitfall 1) and EmbedderTrait adapter need (Pitfall 2) verified from source code
- Proto changes: HIGH -- field numbers verified, IngestEventResponse currently has fields 1 and 2

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable -- all code is local, no external API changes)
