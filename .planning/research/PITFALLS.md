# Domain Pitfalls: Conversational Memory Systems

**Domain:** Conversational Memory System (Rust + RocksDB + LLM Summarization)
**Researched:** 2026-01-29
**Confidence:** HIGH (based on Context7, academic papers, and production experience reports)

---

## Critical Pitfalls

Mistakes that cause rewrites, data loss, or fundamental architectural failures.

---

### Pitfall 1: Summarization Information Loss Cascade

**What goes wrong:**
LLM-based summarization loses critical details during compression. When summaries are hierarchically aggregated (session -> day -> week -> month), information loss compounds at each level. A user's dietary restriction mentioned once becomes "has preferences" at the day level and disappears entirely at the week level.

**Why it happens:**
- Summarization optimizes for brevity, not retrieval utility
- LLMs cannot distinguish what will be important later
- Hierarchical aggregation amplifies losses exponentially
- No mechanism to preserve "anchor facts" through layers

**Consequences:**
- Agent forgets critical user facts
- Contradictory behavior across sessions
- Users must repeat information endlessly
- Trust erosion in the memory system

**Warning signs:**
- Users repeating information they already provided
- Summaries mentioning "various preferences" without specifics
- Test queries for specific facts returning generic results
- Summary length shrinking faster than expected through hierarchy

**Prevention:**
1. **Fact Extraction Layer**: Before summarization, extract discrete facts (key-value pairs) that bypass summarization entirely
2. **Anchor Tagging**: Mark high-importance facts for preservation through aggregation layers
3. **Dual-Path Storage**: Raw events always accessible; summaries are navigational aids, not truth
4. **Summarization Prompts**: Include explicit instructions to preserve specific details (names, numbers, preferences)
5. **Validation Testing**: Test round-trip fact retrieval before/after summarization

**Which phase should address it:**
Phase 1-2 (Core Storage + TOC Foundation) - Must design fact extraction before building summarization

**Severity:** CRITICAL - Fundamental to memory utility

---

### Pitfall 2: Treating TOC Nodes as Ground Truth

**What goes wrong:**
TOC summaries become the primary retrieval target instead of navigation aids. Queries hit summaries and return stale, incomplete, or hallucinated information without ever touching raw events.

**Why it happens:**
- Summaries are smaller and faster to query
- Developers optimize for speed over accuracy
- LLM hallucinations in summaries look authoritative
- No clear boundary between "navigation" and "retrieval" operations

**Consequences:**
- Hallucinations from summarization propagate as facts
- Stale summaries return outdated information
- No way to verify accuracy against source
- User confusion when agent "remembers" things that didn't happen

**Warning signs:**
- Queries returning summary-generated content as facts
- No audit trail from answer back to source event
- Summaries containing details not in underlying events
- Agent confidently stating incorrect information

**Prevention:**
1. **TOC as Index Only**: TOC summaries used exclusively for navigation to relevant time ranges
2. **Always Verify**: Final answers must cite and retrieve actual events
3. **Provenance Tracking**: Every fact links back to source event ID
4. **Hallucination Detection**: Compare summary claims against raw event content
5. **API Design**: Separate `navigate()` from `retrieve()` operations explicitly

**Which phase should address it:**
Phase 2-3 (TOC Foundation + Query Layer) - Navigation vs retrieval distinction must be architectural

**Severity:** CRITICAL - Core architectural principle

---

### Pitfall 3: RocksDB Write Amplification Explosion

**What goes wrong:**
Append-only workload with level compaction creates 20-80x write amplification. A system ingesting 1GB/day writes 20-80GB to disk. SSDs wear out faster, compaction latency spikes during peak writes.

**Why it happens:**
- Level compaction rewrites data at each level transition
- Default RocksDB config optimized for read-heavy workloads
- Append-only means all writes are new data, maximizing compaction work
- Time-series keys with timestamp prefixes create hot spots

**Consequences:**
- SSD lifespan dramatically reduced
- Latency spikes during compaction
- Write stalls when compaction can't keep up
- Higher operational costs (storage I/O)

**Warning signs:**
- `rocksdb.compaction.bytes.written` far exceeds application write volume
- Write latency percentiles (p99, p999) spike periodically
- Disk I/O utilization high even during low application load
- `level0_slowdown_writes_triggered` increasing

**Prevention:**
1. **FIFO or Universal Compaction**: For append-only time-series, FIFO compaction avoids rewrites entirely; universal reduces amplification
2. **Write Buffer Tuning**: Larger `write_buffer_size` reduces flush frequency
3. **Level Size Ratios**: Increase `max_bytes_for_level_multiplier` to reduce levels
4. **Partition by Time**: Separate column families for time windows; old data can use cheaper compaction
5. **Monitor Write Amplification**: Track ratio continuously; alert on degradation

**Which phase should address it:**
Phase 1 (Core Storage) - Compaction strategy is foundational configuration

**Severity:** CRITICAL - Affects system longevity and cost

---

### Pitfall 4: Embedding Model Version Drift

**What goes wrong:**
Vector index contains embeddings from multiple model versions. Query embeddings from the current model don't match stored embeddings from older versions. Retrieval quality silently degrades.

**Why it happens:**
- Model updates change embedding space geometry
- Partial re-embedding (some docs, not all)
- No version tracking on stored vectors
- "Minor" model version bumps are assumed compatible

**Consequences:**
- Relevant documents not retrieved
- Irrelevant documents returned with high similarity
- Silent degradation (no errors, just bad results)
- Debugging is extremely difficult

**Warning signs:**
- Retrieval precision drops without obvious cause
- Same query returns different results after model update
- Nearest neighbor consistency tests failing
- Documents embedded at different times cluster poorly

**Prevention:**
1. **Version Metadata**: Every vector stores model version, embedding date, preprocessing hash
2. **Atomic Re-indexing**: All-or-nothing index rebuilds when model changes
3. **Index as Disposable**: Treat vector indexes as rebuildable accelerators (per your core principles)
4. **Drift Detection**: Periodic nearest-neighbor consistency checks
5. **Pin Model Versions**: Explicit version pinning, no automatic updates

**Which phase should address it:**
Phase 4+ (Teleport Indexes) - Vector indexes are optional accelerators; version discipline from start

**Severity:** CRITICAL - Silent quality degradation is worst failure mode

---

## Moderate Pitfalls

Mistakes that cause delays, performance issues, or accumulated technical debt.

---

### Pitfall 5: Key Design Preventing Efficient Time Scans

**What goes wrong:**
RocksDB key structure doesn't support efficient time-range queries. Retrieving "all events from last Tuesday" requires full database scan instead of prefix scan.

**Why it happens:**
- Keys designed for point lookups, not range scans
- Timestamp not in key prefix position
- UUID-first keys scatter time-adjacent events across key space
- Prefix bloom filters can't help

**Consequences:**
- TOC regeneration is prohibitively slow
- Time-based queries hit every SST file
- System doesn't scale with history length
- Heavy scan fallback becomes too heavy

**Warning signs:**
- Time-range query latency grows linearly with total data
- High read amplification for bounded time queries
- Iterator seeks touching all levels
- Prefix bloom filter hit rate near 0%

**Prevention:**
1. **Time-Prefix Keys**: Structure as `{source_id}:{timestamp}:{event_id}`
2. **Prefix Extractor**: Configure RocksDB prefix extractor for source+time prefixes
3. **Bloom Filters**: Enable prefix bloom filters for time-range acceleration
4. **Test with Scale**: Benchmark time queries with realistic data volumes early
5. **Partition by Source**: Separate key prefixes or column families per agent source

**Which phase should address it:**
Phase 1 (Core Storage) - Key schema is immutable once data exists

**Severity:** HIGH - Directly impacts your time-first architecture

---

### Pitfall 6: Recency Bias in Retrieval Obscuring Important Old Facts

**What goes wrong:**
Memory systems weight recent events too heavily. Critical facts from early conversations (user's name, core preferences, important context) get buried by recency decay and never surface.

**Why it happens:**
- Simple temporal decay functions (e.g., 0.995^hours)
- Importance scoring is noisy/inconsistent
- No distinction between "old but foundational" vs "old and stale"
- Recency is easy to compute; importance is hard

**Consequences:**
- Agent forgets user's name after a week
- Early-established facts get lost
- Users re-explain core context repeatedly
- Memory feels "goldfish-like"

**Warning signs:**
- Queries for foundational facts failing after time passes
- High-importance facts not retrieved despite matching
- Users explicitly re-stating information with phrases like "as I mentioned before"
- Retrieval test suite regressing on older test cases

**Prevention:**
1. **Fact Type Classification**: Distinguish ephemeral (weather today) from persistent (user's name) facts
2. **Importance Anchoring**: High-importance facts decay slower or not at all
3. **Explicit Persistence**: Allow facts to be marked as "always relevant"
4. **Test Temporal Coverage**: Query benchmarks must include facts from all time periods
5. **Usage-Based Reinforcement**: Facts that get retrieved frequently resist decay

**Which phase should address it:**
Phase 3-4 (Query Layer + Teleport Indexes) - Retrieval ranking is query-layer concern

**Severity:** HIGH - Core to long-term memory value

---

### Pitfall 7: Hook Ingestion Race Conditions and Out-of-Order Events

**What goes wrong:**
Events from multiple agent sources arrive out of order. A response event arrives before its prompt event. Deduplication fails. State becomes inconsistent.

**Why it happens:**
- Network latency variance between hooks
- No global ordering across sources
- Retry logic creates duplicates
- Webhook receivers don't coordinate

**Consequences:**
- Orphaned response events without prompts
- Duplicate events in storage
- TOC summarization references missing context
- Inconsistent state across consumers

**Warning signs:**
- Events with references to non-existent prior events
- Duplicate event IDs in storage
- Summarization errors citing "missing context"
- Event counts don't match source system counts

**Prevention:**
1. **Idempotent Writes**: Use event ID as key; writes are upserts, not inserts
2. **Source Timestamps**: Trust source event time, not ingestion time, for ordering
3. **Deduplication Window**: Track seen event IDs for configurable lookback period
4. **Late Event Handling**: Events can arrive late; TOC must handle backfills
5. **Reconciliation Jobs**: Periodic comparison against source systems
6. **Queue-First Architecture**: Ingest to durable queue before processing

**Which phase should address it:**
Phase 1 (Core Storage) - Ingestion guarantees are foundational

**Severity:** HIGH - Data integrity baseline

---

### Pitfall 8: RocksDB Memory Consumption During Compaction

**What goes wrong:**
Compaction doubles memory usage temporarily. System OOMs during compaction spikes. Or, to prevent OOM, compaction is throttled so aggressively that write stalls occur.

**Why it happens:**
- Universal compaction holds old + new data during merge
- Memory limits not configured for peak, only steady state
- Block cache too large relative to system memory
- Multiple concurrent compactions

**Consequences:**
- OOM kills during compaction
- Severe latency spikes
- Write stalls blocking ingestion
- Unpredictable system behavior under load

**Warning signs:**
- Memory usage spikes correlating with compaction
- OOM killer activity in system logs
- Write stalls during batch ingestion
- High memory pressure during off-peak (compaction catching up)

**Prevention:**
1. **Memory Budget**: Allocate only 50-60% of system memory to RocksDB for headroom
2. **Compaction Concurrency**: Limit `max_background_compactions` to control parallelism
3. **Block Cache Sizing**: Size block cache for steady state, not maximum
4. **Rate Limiting**: Use `rate_limiter` to throttle compaction I/O
5. **Monitoring**: Alert on memory usage percentiles, not just averages

**Which phase should address it:**
Phase 1 (Core Storage) - Memory configuration is deployment concern

**Severity:** MEDIUM - Operational, usually caught in staging

---

## Minor Pitfalls

Mistakes that cause annoyance but are fixable without major rework.

---

### Pitfall 9: Inconsistent Timestamp Handling

**What goes wrong:**
Different parts of the system use different timestamp formats, timezones, or precision. UTC vs local, seconds vs milliseconds, string vs integer.

**Why it happens:**
- Multiple developers, no standard established
- External sources use different formats
- "Just get it working" mentality
- Timezone handling is annoying

**Consequences:**
- Off-by-one-hour errors in queries
- Events appearing in wrong TOC buckets
- Sorting anomalies at day boundaries
- Confusing debug output

**Prevention:**
1. **Single Canonical Format**: Milliseconds-since-Unix-epoch UTC everywhere internal
2. **Conversion at Boundaries**: Parse to canonical immediately on ingestion; format only on output
3. **Type System**: Rust newtype wrappers prevent mixing timestamp types
4. **Test Around Boundaries**: Midnight, DST transitions, timezone edges

**Which phase should address it:**
Phase 1 (Core Storage) - Define once, enforce everywhere

**Severity:** LOW - Annoying but fixable incrementally

---

### Pitfall 10: Over-Engineering the First TOC Level

**What goes wrong:**
Building sophisticated month/quarter/year aggregations before validating the session->day level works correctly. Complexity without proven value.

**Why it happens:**
- Exciting to build the "complete" system
- Premature optimization (your explicit non-goal!)
- Assuming higher levels work if lower levels work
- Underestimating LLM summarization edge cases

**Consequences:**
- Time spent on features that may not be needed
- Bugs hidden in rarely-exercised code paths
- More complex debugging
- Delayed validation of core functionality

**Prevention:**
1. **Start with Two Levels**: Session and day only until proven useful
2. **Demand-Driven Expansion**: Add hierarchy levels when queries need them
3. **Metrics First**: Measure what queries actually need before building
4. **Vertical Slice**: Complete one level well before adding more

**Which phase should address it:**
Phase 2 (TOC Foundation) - Start minimal, expand based on evidence

**Severity:** LOW - Course correction is cheap

---

### Pitfall 11: BM25 vs Vector Index Preprocessing Mismatch

**What goes wrong:**
BM25 index and vector index use different text preprocessing. BM25 lowercases; embeddings don't. BM25 stems; embeddings see full words. Hybrid search returns inconsistent results.

**Why it happens:**
- Indexes built by different code paths
- Copy-paste with modifications
- Preprocessing seems like minor detail
- Tested separately, not together

**Consequences:**
- Same query returns different docs from each index
- Hybrid fusion produces nonsensical rankings
- Hard to debug which index is "wrong"
- User confusion at result variance

**Prevention:**
1. **Shared Preprocessing Module**: Single source of truth for text normalization
2. **Document Canonical Form**: Store preprocessed form; both indexes read same source
3. **Test Hybrid End-to-End**: Query benchmarks cover both paths
4. **Preprocessing Hash**: Track preprocessing version in index metadata

**Which phase should address it:**
Phase 4 (Teleport Indexes) - When adding second index type

**Severity:** LOW - Fixable when building teleport layer

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Core Storage (RocksDB) | Write amplification explosion | Configure FIFO/Universal compaction from start |
| Core Storage (RocksDB) | Key design preventing time scans | Time-prefix keys with prefix extractors |
| Core Storage (Ingestion) | Out-of-order events, duplicates | Idempotent writes, source timestamps |
| TOC Foundation | Summarization information loss | Fact extraction layer before summarization |
| TOC Foundation | TOC as ground truth | Explicit navigation-only API design |
| Query Layer | Recency bias burying old facts | Fact type classification, importance anchoring |
| Query Layer | Over-engineering TOC levels | Start with 2 levels, demand-driven expansion |
| Teleport Indexes | Embedding version drift | Version metadata, atomic re-indexing |
| Teleport Indexes | BM25/vector preprocessing mismatch | Shared preprocessing module |

---

## Your Non-Goals as Protection

Your explicit non-goals naturally prevent several common pitfalls:

| Non-Goal | Pitfalls Prevented |
|----------|-------------------|
| No graph database | Over-engineering relationships, graph query complexity |
| No multi-tenant | Permission/isolation bugs, key collision schemes |
| No deletes/mutable history | Consistency bugs, tombstone accumulation |
| No "search everything all the time" | Index maintenance overhead, cold query spikes |
| No premature optimization | Over-engineering, wasted effort on unvalidated features |

Your core principles also provide natural guardrails:

| Principle | Protection Provided |
|-----------|-------------------|
| Append-only truth | Data integrity, audit trail, no corruption from updates |
| TOC never goes away | Navigation always possible even if indexes fail |
| Time is primary axis | Natural partitioning, efficient range queries |
| Indexes are disposable | Embedding drift is recoverable; rebuild, don't repair |
| Heavy scan is controlled fallback | Always have a correct (if slow) answer |

---

## Sources

### Academic & Research Papers
- [Memory in the Age of AI Agents (arXiv 2512.13564)](https://arxiv.org/abs/2512.13564) - Comprehensive agent memory survey
- [Drift-Adapter (EMNLP 2025)](https://aclanthology.org/2025.emnlp-main.805/) - Embedding model migration
- [LLM Chat History Summarization Guide](https://mem0.ai/blog/llm-chat-history-summarization-guide-2025) - Summarization failure modes
- [ACL 2025 Long-Term Memory Evaluation](https://aclanthology.org/2025.findings-acl.1014.pdf) - Memory retrieval challenges

### RocksDB Documentation & Issues
- [RocksDB Tuning Guide](https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide) - Write amplification, compaction configuration
- [RocksDB Troubleshooting Guide](https://github.com/facebook/rocksdb/wiki/RocksDB-Troubleshooting-Guide) - Common production issues
- [Time-Aware Tiered Storage](https://rocksdb.org/blog/2022/11/09/time-aware-tiered-storage.html) - Time-based data handling

### Production Experience
- [Embedding Drift: The Quiet Killer (DEV Community)](https://dev.to/dowhatmatters/embedding-drift-the-quiet-killer-of-retrieval-quality-in-rag-systems-4l5m) - Drift detection and prevention
- [Webhooks Best Practices (Medium)](https://medium.com/@xsronhou/webhooks-best-practices-lessons-from-the-trenches-57ade2871b33) - Ingestion race conditions
- [Hierarchical Summarization for Monitoring (Anthropic)](https://alignment.anthropic.com/2025/summarization-for-monitoring/) - Hierarchical aggregation challenges

### Vector Search
- [Milvus BM25 Integration](https://milvus.io/ai-quick-reference/how-do-i-implement-bm25-alongside-vector-search) - Hybrid search implementation
- [Exa BM25 Optimization](https://exa.ai/blog/bm25-optimization) - Scale challenges
