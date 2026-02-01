# Phase 11: BM25 Teleport (Tantivy) - Research

**Researched:** 2026-01-31
**Domain:** Full-text search indexing with Tantivy, BM25 scoring, embedded search engine integration
**Confidence:** HIGH

## Summary

Phase 11 adds keyword-based "teleport" search capability using Tantivy, a Lucene-inspired full-text search engine library written in Rust. Tantivy provides BM25 scoring out of the box, embedded index storage via MmapDirectory, and efficient incremental updates through its segment-based architecture.

The research confirms Tantivy version 0.25.0 is the current stable release (August 2025), providing excellent compatibility with the existing Rust ecosystem. The library uses memory-mapped files for low memory footprint and supports concurrent indexing with configurable thread pools. Integration with the existing RocksDB-based storage is straightforward since Tantivy maintains its own separate directory structure.

**Primary recommendation:** Use Tantivy 0.25 with MmapDirectory, storing the index in a subdirectory of the existing storage path (e.g., `~/.local/share/agent-memory/bm25-index/`). Index TOC node summaries (title, bullets, keywords) and grip excerpts as separate document types with a `doc_type` field for filtering.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tantivy | 0.25 | Full-text search with BM25 scoring | Rust-native Lucene alternative, widely used, actively maintained by Quickwit team |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| (workspace) tokio | 1.43 | Async runtime | Already in workspace, Tantivy commit operations can be spawned as blocking tasks |
| (workspace) serde/serde_json | 1.0 | Serialization | Already in workspace, for index metadata |
| (workspace) tracing | 0.1 | Logging | Already in workspace, for search and indexing operations |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Tantivy | MeiliSearch | MeiliSearch is a full server, not embeddable - adds deployment complexity |
| Tantivy | SQLite FTS5 | Less flexible schema, weaker BM25 implementation, not Rust-native |
| MmapDirectory | RamDirectory | RamDirectory loses data on restart, not suitable for persistence |

**Installation:**
```toml
# Add to workspace Cargo.toml
[workspace.dependencies]
tantivy = "0.25"

# In memory-search crate
[dependencies]
tantivy = { workspace = true }
```

## Architecture Patterns

### Recommended Project Structure
```
crates/
  memory-search/           # NEW crate for search functionality
    src/
      lib.rs               # Public API exports
      schema.rs            # Tantivy schema definition
      indexer.rs           # Index writer, document creation
      searcher.rs          # Search queries, result mapping
      error.rs             # Search error types
    Cargo.toml
```

### Pattern 1: Separate Crate for Search
**What:** Create a new `memory-search` crate that depends on `memory-types` and `tantivy`
**When to use:** Always for Phase 11 - keeps search concerns isolated from storage
**Example:**
```rust
// Source: Standard Rust workspace pattern
// crates/memory-search/src/lib.rs
pub mod schema;
pub mod indexer;
pub mod searcher;
pub mod error;

pub use schema::SearchSchema;
pub use indexer::SearchIndexer;
pub use searcher::TeleportSearcher;
pub use error::SearchError;
```

### Pattern 2: Document Type Field for Filtering
**What:** Include a `doc_type` field in the schema to distinguish TOC nodes from grips
**When to use:** When indexing multiple entity types in one index
**Example:**
```rust
// Source: https://docs.rs/tantivy/latest/tantivy/schema/index.html
let mut schema_builder = Schema::builder();
let doc_type = schema_builder.add_text_field("doc_type", STRING | STORED);  // "toc_node" or "grip"
let doc_id = schema_builder.add_text_field("doc_id", STRING | STORED);      // node_id or grip_id
let text = schema_builder.add_text_field("text", TEXT);                      // searchable content
let schema = schema_builder.build();
```

### Pattern 3: IndexWriter as Singleton with Arc
**What:** Share IndexWriter across the application using Arc<Mutex<IndexWriter>>
**When to use:** When multiple components need to add documents
**Example:**
```rust
// Source: https://docs.rs/tantivy/latest/tantivy/struct.IndexWriter.html
use std::sync::{Arc, Mutex};

pub struct SearchIndexer {
    writer: Arc<Mutex<IndexWriter>>,
    schema: SearchSchema,
}

impl SearchIndexer {
    // IndexWriter is Sync, but Mutex provides safe mutable access for commit
    pub fn add_toc_node(&self, node: &TocNode) -> Result<(), SearchError> {
        let doc = self.schema.toc_node_to_doc(node);
        let writer = self.writer.lock().unwrap();
        writer.add_document(doc)?;
        Ok(())
    }
}
```

### Pattern 4: Background Commit with Scheduler
**What:** Use the existing scheduler (Phase 10) to periodically commit the index
**When to use:** To avoid blocking on every document add
**Example:**
```rust
// Source: Derived from existing scheduler pattern
// Commit job runs every minute, batching document adds
pub async fn create_index_commit_job(
    scheduler: &SchedulerService,
    indexer: Arc<SearchIndexer>,
) -> Result<(), SchedulerError> {
    scheduler.register_job(
        "index-commit",
        "0 * * * * *",  // Every minute
        None,
        OverlapPolicy::Skip,
        JitterConfig::new(10),
        move || {
            let indexer = indexer.clone();
            async move {
                indexer.commit().await
            }
        },
    ).await
}
```

### Anti-Patterns to Avoid
- **Committing after every document:** Commits are expensive; batch them. Commit at most once per minute or after a threshold of documents.
- **Creating multiple IndexWriters:** Only one IndexWriter per index is allowed. Share via Arc.
- **Using RamDirectory for persistence:** RamDirectory is for tests only. Use MmapDirectory.
- **Storing large text in STORED fields:** Store only IDs, fetch full content from RocksDB when needed.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| BM25 scoring | Custom TF-IDF implementation | Tantivy's built-in BM25 | Complex to implement correctly with doc length normalization |
| Query parsing | Custom query parser | Tantivy QueryParser | Handles boolean operators, phrase queries, field targeting |
| Index compression | Custom compression | Tantivy's segment format | Uses FSTs, bitpacking, LZ4 - highly optimized |
| Concurrent indexing | Manual thread management | Tantivy's internal thread pool | Already handles memory budgets and segment creation |
| Document deletion | Manual tombstone tracking | Tantivy's alive bitset | Efficient soft deletes with merge cleanup |

**Key insight:** Tantivy handles all the complexity of segment management, compression, and concurrency. The integration layer should focus on schema design and document mapping, not search internals.

## Common Pitfalls

### Pitfall 1: Forgetting to Commit
**What goes wrong:** Documents added but never searchable
**Why it happens:** `add_document` only queues; documents aren't visible until `commit()`
**How to avoid:** Schedule periodic commits via the background scheduler
**Warning signs:** Search returns stale results or "No results found" for recently added content

### Pitfall 2: Blocking the Async Runtime with Tantivy
**What goes wrong:** gRPC requests timeout during index operations
**Why it happens:** Tantivy's `commit()` and `add_document()` are blocking operations
**How to avoid:** Use `tokio::task::spawn_blocking` for Tantivy operations
**Warning signs:** High latency on all requests during indexing

### Pitfall 3: Index Corruption from Improper Shutdown
**What goes wrong:** Index becomes unreadable after crash
**Why it happens:** Tantivy lockfiles and uncommitted data left in inconsistent state
**How to avoid:** Call `commit()` before shutdown; handle lockfile cleanup on startup
**Warning signs:** Index fails to open with "Lock" errors after restart

### Pitfall 4: Over-indexing Fields
**What goes wrong:** Large index size, slow indexing
**Why it happens:** Storing full document text when only IDs are needed
**How to avoid:** Use STORED only for IDs; fetch full content from RocksDB
**Warning signs:** Index larger than source data, high memory during indexing

### Pitfall 5: Not Reloading the Reader
**What goes wrong:** Search doesn't reflect recent commits
**Why it happens:** IndexReader caches segment state; needs `reload()` after commit
**How to avoid:** Create reader with `ReloadPolicy::OnCommit` or call `reload()` manually
**Warning signs:** Search shows stale results after confirmed commit

## Code Examples

Verified patterns from official sources:

### Schema Definition
```rust
// Source: https://docs.rs/tantivy/latest/tantivy/schema/index.html
use tantivy::schema::{Schema, TEXT, STRING, STORED, SchemaBuilder};

pub fn build_teleport_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // Document type for filtering: "toc_node" or "grip"
    schema_builder.add_text_field("doc_type", STRING | STORED);

    // Primary key - node_id or grip_id
    schema_builder.add_text_field("doc_id", STRING | STORED);

    // TOC level (for toc_node only): "year", "month", "week", "day", "segment"
    schema_builder.add_text_field("level", STRING);

    // Searchable text content (title + bullets for TOC, excerpt for grip)
    schema_builder.add_text_field("text", TEXT);

    // Keywords (stored for retrieval, indexed for search)
    schema_builder.add_text_field("keywords", TEXT | STORED);

    // Timestamp for recency boosting (stored as string for simplicity)
    schema_builder.add_text_field("timestamp_ms", STRING | STORED);

    schema_builder.build()
}
```

### Index Creation and Writer
```rust
// Source: https://docs.rs/tantivy/latest/tantivy/index/struct.Index.html
use tantivy::{Index, IndexWriter};
use std::path::Path;

pub fn open_or_create_index(path: &Path, schema: Schema) -> Result<Index, tantivy::TantivyError> {
    if path.join("meta.json").exists() {
        Index::open_in_dir(path)
    } else {
        std::fs::create_dir_all(path)?;
        Index::create_in_dir(path, schema)
    }
}

pub fn create_writer(index: &Index, memory_budget_mb: usize) -> Result<IndexWriter, tantivy::TantivyError> {
    // 50MB is plenty for moderate workloads
    let memory_budget = memory_budget_mb * 1024 * 1024;
    index.writer(memory_budget)
}
```

### Adding Documents
```rust
// Source: https://docs.rs/tantivy/latest/tantivy/ (doc! macro example)
use tantivy::doc;

impl SearchIndexer {
    pub fn index_toc_node(&self, node: &TocNode) -> Result<(), SearchError> {
        let schema = &self.schema;

        // Combine title and bullets for searchable text
        let mut text_parts = vec![node.title.clone()];
        for bullet in &node.bullets {
            text_parts.push(bullet.text.clone());
        }
        let text = text_parts.join(" ");

        let doc = doc!(
            schema.doc_type => "toc_node",
            schema.doc_id => node.node_id.clone(),
            schema.level => node.level.to_string(),
            schema.text => text,
            schema.keywords => node.keywords.join(" "),
            schema.timestamp_ms => node.start_time.timestamp_millis().to_string()
        );

        self.writer.lock().unwrap().add_document(doc)?;
        Ok(())
    }

    pub fn index_grip(&self, grip: &Grip) -> Result<(), SearchError> {
        let schema = &self.schema;

        let doc = doc!(
            schema.doc_type => "grip",
            schema.doc_id => grip.grip_id.clone(),
            schema.text => grip.excerpt.clone(),
            schema.timestamp_ms => grip.timestamp.timestamp_millis().to_string()
        );

        self.writer.lock().unwrap().add_document(doc)?;
        Ok(())
    }
}
```

### Searching with BM25
```rust
// Source: https://docs.rs/tantivy/latest/tantivy/query/struct.QueryParser.html
use tantivy::query::QueryParser;
use tantivy::collector::TopDocs;

pub struct TeleportSearcher {
    index: Index,
    reader: IndexReader,
    query_parser: QueryParser,
}

impl TeleportSearcher {
    pub fn new(index: Index) -> Result<Self, SearchError> {
        let reader = index.reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;

        let text_field = index.schema().get_field("text").unwrap();
        let keywords_field = index.schema().get_field("keywords").unwrap();
        let query_parser = QueryParser::for_index(&index, vec![text_field, keywords_field]);

        Ok(Self { index, reader, query_parser })
    }

    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<TeleportResult>, SearchError> {
        let searcher = self.reader.searcher();
        let query = self.query_parser.parse_query(query_str)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address)?;
            let doc_type = doc.get_first(self.schema.doc_type).unwrap().as_text().unwrap();
            let doc_id = doc.get_first(self.schema.doc_id).unwrap().as_text().unwrap();

            results.push(TeleportResult {
                doc_type: doc_type.to_string(),
                doc_id: doc_id.to_string(),
                score,
            });
        }

        Ok(results)
    }
}
```

### Filtering by Document Type
```rust
// Source: https://docs.rs/tantivy/latest/tantivy/query/struct.BooleanQuery.html
use tantivy::query::{BooleanQuery, TermQuery, Occur};
use tantivy::schema::IndexRecordOption;

impl TeleportSearcher {
    pub fn search_toc_only(&self, query_str: &str, limit: usize) -> Result<Vec<TeleportResult>, SearchError> {
        let text_query = self.query_parser.parse_query(query_str)?;

        // Filter to only TOC nodes
        let doc_type_term = Term::from_field_text(self.schema.doc_type, "toc_node");
        let type_filter = TermQuery::new(doc_type_term, IndexRecordOption::Basic);

        let combined = BooleanQuery::new(vec![
            (Occur::Must, Box::new(text_query)),
            (Occur::Must, Box::new(type_filter)),
        ]);

        let searcher = self.reader.searcher();
        let top_docs = searcher.search(&combined, &TopDocs::with_limit(limit))?;

        // ... map results as before
    }
}
```

### Delete and Update Pattern
```rust
// Source: https://github.com/quickwit-oss/tantivy/blob/main/examples/deleting_updating_documents.rs
impl SearchIndexer {
    /// Re-index a TOC node (delete old, add new)
    pub fn update_toc_node(&self, node: &TocNode) -> Result<(), SearchError> {
        let doc_id_term = Term::from_field_text(self.schema.doc_id, &node.node_id);

        let mut writer = self.writer.lock().unwrap();
        writer.delete_term(doc_id_term);

        // Add updated document
        let doc = self.node_to_doc(node);
        writer.add_document(doc)?;

        Ok(())
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Tantivy 0.21 | Tantivy 0.25 | August 2025 | Better index compatibility, performance improvements |
| Manual thread management | Tantivy internal thread pool | Since 0.15+ | Simpler API, better memory management |
| Explicit merge calls | Automatic merge policy | Since 0.10+ | Less configuration needed |

**Deprecated/outdated:**
- `index_writer.garbage_collect_files()` - merged into automatic segment management
- Custom BM25 implementations - use Tantivy's built-in scorer

## Open Questions

Things that couldn't be fully resolved:

1. **Exact memory budget for agent-memory workload**
   - What we know: 50MB is "plenty for basic use cases" per documentation
   - What's unclear: Optimal value for typical TOC (hundreds of nodes) vs. many grips (thousands)
   - Recommendation: Start with 50MB, make configurable, tune based on real usage

2. **Index rebuild vs. incremental from outbox**
   - What we know: Phase 13 will add outbox-driven indexing for crash recovery
   - What's unclear: Whether Phase 11 should include basic rebuild or defer entirely
   - Recommendation: Implement simple full rebuild command in Phase 11; outbox-driven incremental in Phase 13

3. **Concurrent reader vs. single reader**
   - What we know: "Most projects should create at most one reader"
   - What's unclear: Impact of multiple concurrent gRPC search requests
   - Recommendation: Use single IndexReader with `reload()` on commit; searcher is already thread-safe

## Sources

### Primary (HIGH confidence)
- [Tantivy docs.rs](https://docs.rs/tantivy/latest/tantivy/) - Schema, Index, IndexWriter, QueryParser APIs
- [Tantivy ARCHITECTURE.md](https://github.com/quickwit-oss/tantivy/blob/main/ARCHITECTURE.md) - Segment model, indexing internals
- [Tantivy examples](https://github.com/quickwit-oss/tantivy/tree/main/examples) - Document update/delete patterns

### Secondary (MEDIUM confidence)
- [ParadeDB Tantivy Introduction](https://www.paradedb.com/learn/tantivy/introduction) - Overview and use cases
- [Fulmicoton blog on Tantivy indexing](https://fulmicoton.com/posts/behold-tantivy-part2/) - Threading and performance insights

### Tertiary (LOW confidence)
- Version 0.25.0 release date (August 2025) from web search - verify on crates.io if critical

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Tantivy is well-documented with active maintenance
- Architecture: HIGH - Patterns derived from official docs and examples
- Pitfalls: MEDIUM - Based on documentation and community discussions
- Integration approach: HIGH - Clear separation between Tantivy files and RocksDB

**Research date:** 2026-01-31
**Valid until:** 2026-03-31 (60 days - Tantivy is stable)

---

## Recommended Plan Breakdown

Based on this research, Phase 11 should be split into 4 plans:

### Plan 11-01: Tantivy Integration (Schema and Index Setup)
**Focus:** Create `memory-search` crate, define schema, implement index open/create
**Tasks:**
- Add tantivy 0.25 to workspace dependencies
- Create memory-search crate with proper structure
- Define SearchSchema with doc_type, doc_id, text, keywords, timestamp fields
- Implement Index open_or_create with MmapDirectory
- Add index path configuration to Settings
- Unit tests for schema and index creation

### Plan 11-02: Indexing Pipeline (Document Mapping)
**Focus:** Index writer, document creation for TOC nodes and grips
**Tasks:**
- Implement SearchIndexer with Arc<Mutex<IndexWriter>>
- Add toc_node_to_doc and grip_to_doc mapping functions
- Implement index_toc_node and index_grip methods
- Implement update (delete + add) for versioned TOC nodes
- Add commit method with tokio::spawn_blocking
- Integration tests with mock data

### Plan 11-03: Search API (gRPC TeleportSearch RPC)
**Focus:** Search implementation and gRPC exposure
**Tasks:**
- Implement TeleportSearcher with BM25 query
- Add TeleportSearch RPC to memory.proto
- Create TeleportSearchRequest (query, doc_type filter, limit)
- Create TeleportSearchResponse (results with doc_id, doc_type, score)
- Implement gRPC handler using spawn_blocking
- Add ReloadPolicy::OnCommit for reader freshness
- Integration tests for search functionality

### Plan 11-04: CLI and Background Jobs
**Focus:** CLI command and scheduler integration
**Tasks:**
- Add `teleport search <query>` CLI command
- Implement index-commit scheduled job (every minute)
- Add index rebuild admin command
- Add index stats to storage stats
- End-to-end tests with real TOC data
- Documentation and usage examples
