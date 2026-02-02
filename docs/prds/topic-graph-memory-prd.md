# Topic Graph Memory - Product Requirements Document

**Version:** 1.0
**Date:** 2026-02-01
**Status:** Draft for Architecture Review

---

## 1. Executive Summary

### What is Topic Graph Memory?

Topic Graph Memory is a **semantic concept layer** that enriches the agent-memory system with topic-based discovery and navigation. It extracts recurring topics from TOC summaries, tracks their importance over time, and enables agents to discover related conversations through conceptual connections rather than just keywords or time hierarchy.

### Core Philosophy

> "Topics enrich discovery; time remains truth."

This aligns with the agent-memory Progressive Disclosure Architecture (PDA):
- **Primary navigation**: TOC hierarchy (Year → Month → Week → Day → Segment)
- **Acceleration layers**: BM25 + Vector teleport for keyword and semantic search
- **Enrichment layer**: Topic Graph for conceptual discovery and context
- **Fallback**: If topics are unavailable, all other navigation still works

### Why Topics?

Topics provide capabilities that keyword and vector search cannot:

1. **Conceptual Continuity** - Track how topics evolve across weeks and months
2. **Context Bridging** - Link conversations that share concepts but use different terminology
3. **Importance Surfacing** - Identify which topics matter most based on frequency and recency
4. **Discovery Paths** - Navigate from a topic to related topics and their conversations
5. **Contextual Anchors** - Provide semantic landmarks for navigating large memory stores

### How Topics Differ from Existing Search

| Capability | BM25 (Phase 11) | Vector (Phase 12) | Topics (This PRD) |
|------------|-----------------|-------------------|-------------------|
| Find "JWT" | Exact keyword match | Semantic similarity | All nodes about "authentication" |
| Navigate by | Keywords | Meaning | Concepts |
| Surfaces | Documents | Documents | Patterns over time |
| Shows | What matches | What's similar | What's important |
| Links | None | None | Topic relationships |

---

## 2. Optional and Configurable

### Core Principle: Topics are Optional

Topic Graph Memory is **entirely optional**. Users can disable it without losing any functionality - the system falls back to BM25, vector search, or TOC navigation.

This is critical because:
1. **Resource conservation** - Topic extraction requires embedding clustering and LLM calls
2. **Simplicity preference** - Some users may not need conceptual navigation
3. **Privacy concerns** - Some users may prefer no additional analysis of their data
4. **Rebuild scenarios** - Topics can be deleted and system still works

### Configuration

```toml
# ~/.config/agent-memory/config.toml

[topics]
# Master switch for topic graph functionality
enabled = true           # default: false (opt-in feature)

[topics.extraction]
# Embedding clustering for topic discovery
enabled = true
min_cluster_size = 3     # Minimum nodes to form a topic
similarity_threshold = 0.75  # Cosine similarity for clustering

[topics.labeling]
# LLM-based topic naming
enabled = true
model = "default"        # Use configured summarizer
fallback_to_keywords = true  # Use top keywords if LLM unavailable

[topics.importance]
# Time-decayed importance scoring
half_life_days = 30      # Importance decays by 50% every 30 days
recency_boost = 2.0      # Recent mentions get 2x weight

[topics.relationships]
# Topic-to-topic connections
enabled = true
similarity_threshold = 0.6   # Minimum similarity for "similar" topics
hierarchy_enabled = true     # Infer parent/child relationships

[topics.lifecycle]
# Pruning and resurrection
prune_after_days = 180   # Prune topics inactive for 6 months
resurrection_enabled = true  # Allow resurrecting pruned topics
min_importance_score = 0.1   # Prune topics below this score
```

### Agent Skill Behavior

Agent skills MUST handle topics being disabled gracefully:

#### Checking Topic Availability

```
# Agent should first check if topics are available
1. Call GetTopicGraphStatus() RPC
2. Response includes:
   - enabled: bool
   - healthy: bool (topics exist and readable)
   - topic_count: int64
   - last_extraction_ms: int64
```

#### When Topics are DISABLED

Agent skills should:
1. **Not offer topic commands** - Don't suggest `/memory-topics` if disabled
2. **Use alternative navigation** - Fall back to BM25/vector search or TOC navigation
3. **Inform user if asked** - "Topic navigation is disabled. Using keyword search instead."

#### When Topics are ENABLED

Agent skills should:
1. **Offer topic discovery** - Suggest exploring related topics
2. **Surface important topics** - Show which concepts are frequently discussed
3. **Provide topic context** - Explain why a topic might be relevant

### Skill Documentation Requirements

Agent skills (SKILL.md files) MUST document:

1. **Topic dependency** - Whether the command uses topics
2. **Fallback behavior** - What happens when topics are disabled
3. **Configuration guidance** - How users can enable/disable

---

## 3. Goals & Objectives

### Primary Goals

| Goal | Description | Success Metric |
|------|-------------|----------------|
| **G1: Conceptual Discovery** | Surface recurring themes across conversations | >10% of queries use topics |
| **G2: Importance Surfacing** | Identify what matters most to user | Top topics match user perception |
| **G3: Relationship Navigation** | Enable exploring related concepts | >70% of relationships are useful |
| **G4: Graceful Degradation** | Work without topic graph | Falls back to vector/BM25/agentic |
| **G5: Low Overhead** | Minimal storage and computation | <5MB/year, extraction <5 minutes |

### Non-Goals (Out of Scope)

| Non-Goal | Reason |
|----------|--------|
| Causal graphs | Too complex for v1 |
| User-defined topics | Auto-extraction preferred first |
| Real-time extraction | Batch is sufficient |
| Cross-project topics | Per-project stores per requirements |
| Graph database | RocksDB sufficient for simple graph |

---

## 4. Problem Statement

### Current State

The agent-memory system has comprehensive search capabilities:

| Capability | Phase | Description |
|-----------|-------|-------------|
| **Hierarchical TOC** | Phase 2 | Year > Month > Week > Day > Segment organization |
| **Provenance via Grips** | Phase 3 | Excerpts with event pointers for citation |
| **Agentic Search** | Phase 10.5 | Index-free term-overlap search |
| **BM25 Teleport** | Phase 11 | Tantivy full-text search |
| **Vector Teleport** | Phase 12 | HNSW semantic similarity |

However, all these capabilities operate at the **document level**. There's no way to understand:
- What are the recurring themes across conversations?
- How do different discussions connect conceptually?
- Which topics are most important to this user over time?
- What related concepts might be relevant to the current query?

### Pain Points

1. **No Conceptual View**: Users can find individual conversations but can't see the forest for the trees
   - "What are the main themes of my work this month?"
   - "Which topics keep coming up repeatedly?"

2. **No Cross-Conversation Context**: Related discussions are siloed
   - A "security" discussion doesn't link to "authentication" or "authorization" conversations
   - Topics that use different terminology aren't connected

3. **No Importance Signals**: All content is treated equally
   - Frequently discussed topics don't surface naturally
   - Recent topics aren't prioritized over stale ones

4. **Discovery Dead-Ends**: Search finds documents, not conceptual paths
   - Finding one JWT discussion doesn't lead to related auth topics
   - No way to explore "topics similar to this one"

### Opportunity

Add a semantic topic layer that:
- Extracts recurring concepts from TOC summaries
- Tracks topic importance based on frequency and recency
- Connects topics through similarity and hierarchy relationships
- Enables conceptual navigation alongside existing search methods

---

## 5. Solution Overview

### Primary Solution: Topic Extraction and Indexing

Extract topics from TOC node summaries using embedding clustering:

```
TOC Nodes:                          Topic Extraction:
┌──────────────────────┐            ┌─────────────────┐
│ Day 2026-01-28       │ ─────────► │ Cluster         │
│ "JWT auth debugging" │            │ Embeddings      │
├──────────────────────┤            │                 │
│ Day 2026-01-25       │ ─────────► │    ▼            │
│ "OAuth2 setup"       │            │ ┌─────────┐     │
├──────────────────────┤            │ │ Topic:  │     │
│ Day 2026-01-20       │ ─────────► │ │ Auth    │     │
│ "Token refresh fix"  │            │ └─────────┘     │
└──────────────────────┘            └─────────────────┘
```

### Secondary Solution: Topic Importance Scoring

Track topic importance with time-decayed scoring:

```
Importance Score = Σ (mention_weight × decay_factor)

Where:
  mention_weight = 1.0 + recency_boost (if within 7 days)
  decay_factor = 0.5 ^ (days_since_mention / half_life_days)

Example (half_life = 30 days):
  Topic "Authentication" mentioned:
  - Today: 1.0 × 1.0 = 1.0
  - 30 days ago: 1.0 × 0.5 = 0.5
  - 60 days ago: 1.0 × 0.25 = 0.25
  Total importance: 1.75
```

### Tertiary Solution: Topic Relationships

Connect topics through similarity and hierarchy:

```
┌─────────────────────────────────────────────────────────┐
│                    Topic Relationships                   │
├─────────────────────────────────────────────────────────┤
│                                                          │
│                    ┌───────────┐                         │
│                    │ Security  │ (parent)                │
│                    └─────┬─────┘                         │
│          ┌───────────────┼───────────────┐               │
│          ▼               ▼               ▼               │
│   ┌─────────────┐ ┌─────────────┐ ┌─────────────┐       │
│   │Authentication│ │Authorization│ │ Encryption  │       │
│   └──────┬──────┘ └──────┬──────┘ └─────────────┘       │
│          │               │                               │
│          │   similar     │                               │
│          └───────────────┘                               │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Architecture Alignment

```
┌─────────────────────────────────────────────────────────────┐
│                     Agent Memory                             │
├─────────────────────────────────────────────────────────────┤
│  EXISTING:                                                   │
│  ├── Events (CF_EVENTS) ........... Raw conversations       │
│  ├── TocNodes (CF_TOC_NODES) ...... Time hierarchy          │
│  ├── Grips (CF_GRIPS) ............. Provenance anchors      │
│  ├── Tantivy (Phase 11) ........... BM25 teleport           │
│  └── HNSW (Phase 12) .............. Vector teleport         │
│                                                              │
│  NEW (Topic Graph Memory):                                   │
│  ├── Topics (CF_TOPICS) ........... Semantic concepts       │
│  ├── TopicLinks ................... Topic ↔ Node links      │
│  └── TopicRelationships ........... Similar/parent-child    │
└─────────────────────────────────────────────────────────────┘
```

### Navigation Flow with Topics

```
User: "What have I been working on related to security?"

1. GetTopicsByQuery("security")
   └── Returns: [Authentication (0.92), Authorization (0.78),
                 Encryption (0.65), Security Audits (0.55)]

2. Agent selects "Authentication" (highest score)
   └── GetTocNodesForTopic("Authentication")
       └── Returns: [Day 2026-01-28, Day 2026-01-25,
                    Segment 2026-01-20-abc]

3. Agent can also explore relationships:
   └── GetRelatedTopics("Authentication")
       └── Returns: [Authorization (similar), Security (parent),
                    JWT Tokens (child)]

4. Agent provides context-rich response:
   "You've discussed authentication extensively - most recently
    on Jan 28 (JWT debugging). Related topics include authorization
    and your broader security work."
```

---

## 6. Terminology Mapping

| Conceptual Term | Agent-Memory Implementation | Notes |
|-----------------|----------------------------|-------|
| Topic | Extracted semantic concept | Has label, embedding, importance |
| Topic Cluster | Group of similar TOC summaries | Formed by embedding clustering |
| Topic Label | Human-readable name | Generated by LLM or keywords |
| Importance Score | Time-decayed frequency metric | Surfaces recent/frequent topics |
| Topic Link | Association between topic and node | Many-to-many relationship |
| Similar Topics | Topics with high embedding similarity | Threshold-based relationship |
| Parent Topic | Broader concept containing children | Inferred from co-occurrence |
| Child Topic | Specific concept under parent | More specific than parent |
| Topic Pruning | Removing low-importance topics | From index, not from storage |
| Topic Resurrection | Re-adding pruned topic | When referenced again |

---

## 7. User Stories

### Agent User Stories

| ID | As a... | I want to... | So that... |
|----|---------|--------------|------------|
| US-01 | Agent | discover recurring themes | I can provide context about user's work patterns |
| US-02 | Agent | find related topics | I can explore conceptual connections |
| US-03 | Agent | see importance scores | I can prioritize which topics to mention |
| US-04 | Agent | navigate from topic to conversations | I can find relevant discussions |
| US-05 | Agent | check if topics are available | I can choose the best navigation method |

### Admin User Stories

| ID | As a... | I want to... | So that... |
|----|---------|--------------|------------|
| US-06 | Admin | enable/disable topics | I can control resource usage |
| US-07 | Admin | view topic graph status | I can monitor health and coverage |
| US-08 | Admin | configure extraction parameters | I can tune topic granularity |
| US-09 | Admin | see pruned vs active topics | I can understand topic lifecycle |

### Developer User Stories

| ID | As a... | I want to... | So that... |
|----|---------|--------------|------------|
| US-10 | Developer | test topic queries via CLI | I can debug topic behavior |
| US-11 | Developer | trigger extraction manually | I can test extraction logic |
| US-12 | Developer | see extraction metrics | I can identify performance issues |

---

## 8. Functional Requirements

### FR-01: Topic Storage (CF_TOPICS)

**Description:** Persist topics in a dedicated RocksDB column family.

**Acceptance Criteria:**
- [ ] New column family `CF_TOPICS` stores topic records
- [ ] Key format: `topic:{topic_id}` where topic_id is a ULID
- [ ] Value includes: label, embedding, importance_score, created_at, last_mentioned_at
- [ ] Topics survive daemon restart
- [ ] Storage is efficient (no duplicate topics)

### FR-02: Topic Extraction

**Description:** Extract topics from TOC node summaries via embedding clustering.

**Acceptance Criteria:**
- [ ] Uses embeddings from Phase 12 infrastructure
- [ ] HDBSCAN or similar clustering algorithm groups similar summaries
- [ ] Configurable minimum cluster size (default: 3 nodes)
- [ ] Configurable similarity threshold (default: 0.75)
- [ ] Extraction runs as scheduled job (not on every ingestion)
- [ ] New topics created for significant clusters

### FR-03: Topic Labeling

**Description:** Generate human-readable labels for topics.

**Acceptance Criteria:**
- [ ] Primary: LLM generates concise label from cluster summaries
- [ ] Fallback: Extract top keywords if LLM unavailable
- [ ] Labels are unique within the topic graph
- [ ] Labels can be updated if cluster composition changes significantly
- [ ] Label length capped at 50 characters

### FR-04: Topic-Node Links

**Description:** Associate topics with TOC nodes.

**Acceptance Criteria:**
- [ ] Many-to-many relationship (node can have multiple topics, topic can span nodes)
- [ ] Links stored in CF_TOPICS or separate CF_TOPIC_LINKS
- [ ] Link includes: topic_id, node_id, relevance_score
- [ ] Links updated when new nodes are created
- [ ] Old links preserved (append-only, like everything else)

### FR-05: Time-Decayed Importance Scoring

**Description:** Track topic importance with recency weighting.

**Acceptance Criteria:**
- [ ] Importance formula: Σ(weight × 0.5^(days/half_life))
- [ ] Configurable half-life (default: 30 days)
- [ ] Configurable recency boost for recent mentions (default: 2.0)
- [ ] Scores recalculated during extraction job
- [ ] GetTopTopics RPC returns topics sorted by importance

### FR-06: Topic Similarity Relationships

**Description:** Connect topics that are semantically similar.

**Acceptance Criteria:**
- [ ] Calculate cosine similarity between topic embeddings
- [ ] Create "similar" relationship above threshold (default: 0.6)
- [ ] Relationships stored in CF_TOPIC_RELATIONSHIPS or within CF_TOPICS
- [ ] Bidirectional (if A similar to B, then B similar to A)
- [ ] GetRelatedTopics RPC returns similar topics

### FR-07: Topic Hierarchy Relationships

**Description:** Infer parent/child relationships between topics.

**Acceptance Criteria:**
- [ ] Parent topic inferred when broader concept contains children
- [ ] Use co-occurrence patterns and label analysis
- [ ] Relationships stored with type: "parent" or "child"
- [ ] GetRelatedTopics includes hierarchy relationships
- [ ] Hierarchy depth limited to 3 levels

### FR-08: GetTopicsByQuery RPC

**Description:** Find topics matching a natural language query.

**Acceptance Criteria:**
- [ ] Input: query text, limit, min_score
- [ ] Query embedded and compared to topic embeddings
- [ ] Returns ranked topics with similarity scores
- [ ] Includes importance_score in response
- [ ] Graceful error if topics disabled

### FR-09: GetTocNodesForTopic RPC

**Description:** Get TOC nodes associated with a topic.

**Acceptance Criteria:**
- [ ] Input: topic_id, limit, min_relevance
- [ ] Returns TOC node summaries linked to topic
- [ ] Sorted by relevance and recency
- [ ] Includes node titles and levels
- [ ] Supports pagination

### FR-10: GetTopTopics RPC

**Description:** Get most important topics overall or in time range.

**Acceptance Criteria:**
- [ ] Input: limit, optional time_range
- [ ] Returns topics sorted by importance score
- [ ] Time range filters by last_mentioned_at
- [ ] Includes importance breakdown (frequency, recency)

### FR-11: GetRelatedTopics RPC

**Description:** Get topics related to a given topic.

**Acceptance Criteria:**
- [ ] Input: topic_id, relationship_types, limit
- [ ] Returns related topics with relationship type and score
- [ ] Relationship types: "similar", "parent", "child"
- [ ] Can filter by specific relationship type

### FR-12: GetTopicGraphStatus RPC

**Description:** Health and configuration status for topic graph.

**Acceptance Criteria:**
- [ ] Reports: enabled state, topic count, last extraction time
- [ ] Reports: configuration settings (half_life, thresholds)
- [ ] Agent skills MUST call this before using topics
- [ ] Returns human-readable status message

### FR-13: Topic Pruning

**Description:** Remove low-importance topics from active index.

**Acceptance Criteria:**
- [ ] Prune topics with importance below threshold
- [ ] Prune topics not mentioned in configurable period (default: 180 days)
- [ ] Pruning runs on schedule (not real-time)
- [ ] Pruned topics stored in archive (can be resurrected)
- [ ] Pruning does NOT delete from CF_TOPICS, only marks inactive

### FR-14: Topic Resurrection

**Description:** Reactivate pruned topics when referenced again.

**Acceptance Criteria:**
- [ ] When new node matches pruned topic, topic is resurrected
- [ ] Resurrection resets importance calculation
- [ ] Preserves original topic_id and label
- [ ] Logs resurrection event for observability

### FR-15: CLI Commands

**Description:** Command-line interface for topic operations.

**Acceptance Criteria:**
- [ ] `memory-daemon topics list [--limit N]` - List top topics
- [ ] `memory-daemon topics search <query>` - Find topics by query
- [ ] `memory-daemon topics show <topic_id>` - Show topic details
- [ ] `memory-daemon topics related <topic_id>` - Show related topics
- [ ] `memory-daemon topics nodes <topic_id>` - Show linked nodes
- [ ] `memory-daemon topics status` - Show topic graph status
- [ ] All commands support JSON output format

---

## 9. Non-Functional Requirements

### NFR-01: Performance

| Metric | Target |
|--------|--------|
| Topic query latency (p50) | < 50ms |
| Topic query latency (p99) | < 200ms |
| Topic extraction (1000 nodes) | < 60s |
| Importance recalculation | < 10s |

### NFR-02: Reliability

- Topic extraction failures don't affect other system operations
- Missing topics falls back gracefully to BM25/vector search
- Corrupted topic index can be rebuilt from TOC nodes
- Daemon starts without topics if extraction hasn't run

### NFR-03: Scalability

- Handles 10,000+ topics efficiently
- Importance scoring scales linearly with topic count
- Clustering scales with embedding infrastructure (Phase 12)

### NFR-04: Observability

- All topic operations logged with tracing spans
- TopicGraphStatus RPC reports: topic count, last extraction, health
- Metrics exposed: extraction duration, query latency, pruning count

---

## 10. Success Metrics

### Adoption

| Metric | Target | Measurement |
|--------|--------|-------------|
| Topic query usage | > 10% of memory queries | gRPC metrics |
| Topic-based navigation success | > 70% find relevant content | Agent feedback |

### Quality

| Metric | Target | Measurement |
|--------|--------|-------------|
| Topic label accuracy | > 80% meaningful labels | Manual sampling |
| Relationship relevance | > 70% useful connections | Manual sampling |
| Importance ranking quality | Top topics match user perception | User feedback |

### Efficiency

| Metric | Target | Measurement |
|--------|--------|-------------|
| Query latency (p99) | < 200ms | Tracing spans |
| Storage overhead per topic | < 5KB | Storage metrics |
| Extraction job duration | < 5 minutes for 10K nodes | Job metrics |

---

## 11. System Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                       memory-daemon                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ memory-      │  │ memory-      │  │ memory-service       │  │
│  │ topics       │  │ embeddings   │  │ (gRPC handlers)      │  │
│  │              │  │ (Phase 12)   │  │                      │  │
│  │ ┌──────────┐ │  │              │  │ ┌──────────────────┐ │  │
│  │ │ Topic    │ │  │ (Reused for  │  │ │ GetTopicsByQuery │ │  │
│  │ │ Extractor│ │  │  clustering) │  │ │ GetTopTopics     │ │  │
│  │ │ HDBSCAN  │ │  │              │  │ │ GetRelatedTopics │ │  │
│  │ └──────────┘ │  └──────────────┘  │ │ GetTocNodesFor   │ │  │
│  │ ┌──────────┐ │                    │ │ GetTopicGraph... │ │  │
│  │ │ Topic    │ │                    │ └──────────────────┘ │  │
│  │ │ Labeler  │ │                    └───────────┬──────────┘  │
│  │ │ (LLM)    │ │                                │             │
│  │ └──────────┘ │                                │             │
│  └──────┬───────┘                                │             │
│         │                                        │             │
│         └────────────────────────────────────────┘             │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    RocksDB Storage                        │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌───────────────┐  │  │
│  │  │CF_EVENTS│ │CF_TOC   │ │CF_EMBED │ │CF_TOPICS      │  │  │
│  │  │         │ │_NODES   │ │_DINGS   │ │(NEW)          │  │  │
│  │  └─────────┘ └─────────┘ └─────────┘ └───────────────┘  │  │
│  │                                       ┌───────────────┐  │  │
│  │                                       │CF_TOPIC_LINKS │  │  │
│  │                                       │(NEW)          │  │  │
│  │                                       └───────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow: Topic Extraction

```
┌──────────────┐     ┌──────────────┐     ┌─────────────────┐
│  Scheduler   │────▶│   Get TOC    │────▶│ Get Embeddings  │
│ (Daily 4AM)  │     │   Nodes      │     │ (Phase 12)      │
└──────────────┘     └──────────────┘     └────────┬────────┘
                                                   │
                                                   ▼
                                          ┌─────────────────┐
                                          │    HDBSCAN      │
                                          │   Clustering    │
                                          └────────┬────────┘
                                                   │
                                          Cluster assignments
                                                   │
                     ┌─────────────────────────────┴────────┐
                     │                                      │
                     ▼                                      ▼
            ┌─────────────────┐                   ┌─────────────────┐
            │  New Cluster?   │                   │ Existing Topic? │
            │  → Create Topic │                   │ → Update Links  │
            └────────┬────────┘                   └────────┬────────┘
                     │                                      │
                     ▼                                      │
            ┌─────────────────┐                             │
            │  LLM Label      │                             │
            │  Generation     │                             │
            └────────┬────────┘                             │
                     │                                      │
                     └──────────────────┬───────────────────┘
                                        │
                                        ▼
                               ┌─────────────────┐
                               │   CF_TOPICS     │
                               │  CF_TOPIC_LINKS │
                               └─────────────────┘
```

---

## 12. Configuration Schema

```toml
# ~/.config/agent-memory/config.toml

# =============================================================================
# TOPIC GRAPH CONFIGURATION
# =============================================================================

[topics]
# Master switch for topic graph functionality
# When disabled:
#   - No topic extraction runs
#   - No CF_TOPICS data created
#   - Topic RPCs return UNAVAILABLE
#   - Other navigation (TOC, BM25, Vector) unaffected
# Default: false (opt-in feature)
enabled = false

# =============================================================================
# TOPIC EXTRACTION
# =============================================================================

[topics.extraction]
# Enable automatic topic extraction from TOC summaries
enabled = true

# Minimum cluster size to create a topic
# Smaller = more topics (finer grained)
# Larger = fewer topics (broader concepts)
min_cluster_size = 3

# Cosine similarity threshold for clustering
# Lower = more inclusive clusters
# Higher = tighter clusters
similarity_threshold = 0.75

# Schedule for extraction job (cron expression)
schedule = "0 4 * * *"  # 4 AM daily

# Batch size for processing nodes
batch_size = 500

# =============================================================================
# TOPIC LABELING
# =============================================================================

[topics.labeling]
# Enable LLM-based topic naming
enabled = true

# Use default summarizer model, or specify explicitly
model = "default"

# Fall back to keyword extraction if LLM unavailable
fallback_to_keywords = true

# Maximum label length
max_label_length = 50

# =============================================================================
# IMPORTANCE SCORING
# =============================================================================

[topics.importance]
# Half-life for importance decay (days)
# After this many days, a mention contributes 50% of original weight
half_life_days = 30

# Boost multiplier for mentions within 7 days
recency_boost = 2.0

# Minimum importance score to remain active
min_active_score = 0.1

# =============================================================================
# TOPIC RELATIONSHIPS
# =============================================================================

[topics.relationships]
# Enable topic relationship discovery
enabled = true

# Cosine similarity threshold for "similar" relationship
similarity_threshold = 0.6

# Enable parent/child hierarchy inference
hierarchy_enabled = true

# Maximum hierarchy depth
max_hierarchy_depth = 3

# =============================================================================
# LIFECYCLE MANAGEMENT
# =============================================================================

[topics.lifecycle]
# Enable pruning of inactive topics
enabled = true

# Prune topics not mentioned in this many days
prune_after_days = 180

# Enable resurrection of pruned topics
resurrection_enabled = true

# Schedule for pruning job (cron expression)
prune_schedule = "0 5 * * 0"  # 5 AM Sundays
```

---

## 13. API Surface

### gRPC RPCs

| Method | Path | Purpose | Availability |
|--------|------|---------|--------------|
| GetTopicsByQuery | `MemoryService/GetTopicsByQuery` | Find topics by query | This phase |
| GetTocNodesForTopic | `MemoryService/GetTocNodesForTopic` | Get nodes for topic | This phase |
| GetTopTopics | `MemoryService/GetTopTopics` | List important topics | This phase |
| GetRelatedTopics | `MemoryService/GetRelatedTopics` | Get related topics | This phase |
| GetTopicGraphStatus | `MemoryService/GetTopicGraphStatus` | Health and config | This phase |

### CLI Commands

| Command | Description |
|---------|-------------|
| `memory-daemon topics list [--limit N]` | List top topics by importance |
| `memory-daemon topics search <query>` | Find topics matching query |
| `memory-daemon topics show <topic_id>` | Show topic details |
| `memory-daemon topics related <topic_id>` | Show related topics |
| `memory-daemon topics nodes <topic_id>` | Show linked TOC nodes |
| `memory-daemon topics status` | Show topic graph health |

### Skill Integration

Update `skills/memory-query/SKILL.md` to include:

```markdown
## Topic Commands (When Enabled)

**Note:** Topics are optional. Check `GetTopicGraphStatus` before use.

### /memory-topics [query]
List important topics, optionally filtered by query.
**Requires:** `topics.enabled: true`
**Fallback:** Uses BM25 keyword search if disabled

### /memory-explore <topic>
Explore a topic and its relationships.
**Requires:** `topics.enabled: true`
**Fallback:** Uses vector search for similar content if disabled

### Checking Availability
Before using topic features, the skill MUST:
1. Call GetTopicGraphStatus RPC
2. If enabled=false, use BM25/vector search instead
3. If both unavailable, use agentic SearchChildren
```

### Agent Skill Error Codes

| RPC | Status Code | Message | Skill Action |
|-----|-------------|---------|--------------|
| GetTopicsByQuery | UNAVAILABLE | "Topic graph not enabled" | Use vector search |
| GetTocNodesForTopic | NOT_FOUND | "Topic not found" | Inform user, suggest alternatives |
| GetTopicGraphStatus | OK | `enabled: false` | Skip topic features |

---

## 14. Observability

### Prometheus Metrics

```
# Topic graph size
topic_graph_topic_count{status="active"} 245
topic_graph_topic_count{status="pruned"} 58
topic_graph_link_count 4892
topic_graph_relationship_count 612

# Query latency histogram
topic_query_latency_seconds_bucket{op="by_query", le="0.02"} 95
topic_query_latency_seconds_bucket{op="by_query", le="0.05"} 98
topic_query_latency_seconds_bucket{op="by_query", le="0.2"} 99

# Extraction job metrics
topic_extraction_duration_seconds 142.5
topic_extraction_topics_created 12
topic_extraction_topics_updated 45
topic_extraction_last_run_timestamp 1706745600

# Lifecycle metrics
topic_lifecycle_pruned_total 58
topic_lifecycle_resurrected_total 3
```

### Tracing Spans

All topic operations include tracing spans:

- `topic_extraction` - Full extraction job span
  - `fetch_embeddings` - Get embeddings from Phase 12
  - `cluster_nodes` - HDBSCAN clustering
  - `generate_labels` - LLM labeling
  - `update_storage` - Write to CF_TOPICS

- `topic_query` - Query operations
  - `embed_query` - Embed query text
  - `similarity_search` - Find similar topics
  - `enrich_results` - Add metadata

### Health Checks

`GetTopicGraphStatus` returns:
- `enabled: bool` - Configuration state
- `healthy: bool` - Topics exist and readable
- `topic_count: int64` - Total active topics
- `link_count: int64` - Total topic-node links
- `last_extraction_ms: int64` - Last extraction timestamp
- `message: string` - Human-readable status

---

## 15. Data Model

### Topic Record

```rust
pub struct Topic {
    /// Unique identifier (ULID)
    pub topic_id: String,

    /// Human-readable label
    pub label: String,

    /// Embedding vector for similarity calculations
    pub embedding: Vec<f32>,

    /// Time-decayed importance score
    pub importance_score: f64,

    /// Number of linked TOC nodes
    pub node_count: u32,

    /// Timestamp of first occurrence
    pub created_at: DateTime<Utc>,

    /// Timestamp of most recent mention
    pub last_mentioned_at: DateTime<Utc>,

    /// Whether topic is active or pruned
    pub status: TopicStatus,

    /// Keywords extracted from cluster
    pub keywords: Vec<String>,
}

pub enum TopicStatus {
    Active,
    Pruned,
}
```

### Topic Link Record

```rust
pub struct TopicLink {
    /// Topic ID
    pub topic_id: String,

    /// TOC node ID
    pub node_id: String,

    /// Relevance score (0.0-1.0)
    pub relevance: f32,

    /// When link was created
    pub created_at: DateTime<Utc>,
}
```

### Topic Relationship Record

```rust
pub struct TopicRelationship {
    /// Source topic ID
    pub from_topic_id: String,

    /// Target topic ID
    pub to_topic_id: String,

    /// Relationship type
    pub relationship_type: RelationshipType,

    /// Strength/confidence (0.0-1.0)
    pub score: f32,
}

pub enum RelationshipType {
    Similar,
    Parent,
    Child,
}
```

### Storage Keys

```
CF_TOPICS:
  topic:{topic_id} -> Topic (serialized)

CF_TOPIC_LINKS:
  link:{topic_id}:{node_id} -> TopicLink (serialized)
  (Secondary index: node:{node_id}:{topic_id} for reverse lookup)

CF_TOPIC_RELATIONSHIPS:
  rel:{from_topic_id}:{to_topic_id} -> TopicRelationship (serialized)
```

---

## 16. Integration with Existing Phases

### Phase Dependency Chain

```
Phase 10.5: Agentic TOC Search (FOUNDATION)
      │
      │  "Always works" index-free search
      │
      ▼
Phase 11: BM25 Teleport (KEYWORD ACCELERATION)
      │
      │  Tantivy full-text search
      │
      ▼
Phase 12: Vector Teleport (SEMANTIC ACCELERATION)
      │
      │  HNSW semantic similarity
      │   └── Embeddings reused for topic clustering
      │
      ▼
Topic Graph Memory (THIS PRD - CONCEPTUAL ENRICHMENT)
      │
      │  Topic extraction, relationships, navigation
      │
      ▼
   Complete Navigation Stack
```

### Integration Points

| Phase | Component | Integration |
|-------|-----------|-------------|
| 2 | TOC Nodes | Source data for topic extraction |
| 3 | Grips | Can be linked to topics for provenance |
| 10.5 | SearchChildren | Fallback when topics unavailable |
| 11 | BM25 | Topic labels indexed for keyword search |
| 12 | HNSW | Embeddings reused for clustering |
| 12 | CF_EMBEDDINGS | Topic embeddings stored similarly |

### Search Method Comparison and Fallback Chain

The complete search method comparison table and fallback chain are defined in the **Agent Retrieval Policy PRD** — the single source of truth for retrieval layer selection.

**See:**
- [Agent Retrieval Policy PRD](agent-retrieval-policy-prd.md) - Fallback chains, capability tiers, skill contracts
- [Cognitive Architecture Manifesto](../COGNITIVE_ARCHITECTURE.md) - Philosophy and layer stack
- [BM25 Teleport PRD](bm25-teleport-prd.md)
- [Hierarchical Vector Indexing PRD](hierarchical-vector-indexing-prd.md)

---

## 17. Out of Scope

| Item | Reason | Future Consideration |
|------|--------|---------------------|
| Causal graphs | Adds significant complexity | Future enhancement |
| Multi-agent federation | Out of scope per requirements | Future version |
| Real-time extraction | Batch is sufficient, simpler | If needed for latency |
| User-defined topics | Auto-extraction preferred first | Future enhancement |
| Topic merging/splitting | Manual curation adds complexity | Future enhancement |
| Cross-project topics | Per-project stores per REQUIREMENTS.md | Out of scope |
| Graph database | Topics are simple tree/graph in RocksDB | Not needed |

---

## 18. Open Questions

| Question | Status | Resolution |
|----------|--------|------------|
| Optimal clustering algorithm? | Decided | HDBSCAN for automatic cluster count |
| Minimum cluster size? | Decided | 3 nodes (configurable) |
| Similarity threshold for "similar" topics? | Decided | 0.6 (configurable) |
| How to infer parent/child relationships? | Decided | Co-occurrence + label analysis |
| LLM model for labeling? | Decided | Use configured summarizer model |
| Topic pruning threshold? | Decided | 180 days inactive + importance < 0.1 |

---

## 19. Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Topic extraction quality poor | Useless topics | Medium | Tune clustering params, use LLM labels |
| Importance scoring not useful | Wrong topics surfaced | Medium | User feedback loop, configurable half-life |
| Too many topics | Navigation overwhelm | Medium | Min cluster size, pruning |
| Too few topics | Missing concepts | Medium | Lower similarity threshold |
| LLM labeling expensive | Cost concerns | Low | Fallback to keywords, batch processing |
| Circular relationships | Graph traversal issues | Low | Validate relationships, depth limits |
| Resurrection spam | Constant topic churn | Low | Minimum stability period |

---

## 20. Implementation Waves

| Wave | Focus | Key Deliverables |
|------|-------|------------------|
| Wave 1 | Topic Extraction | CF_TOPICS, embedding clustering, basic Topic struct |
| Wave 2 | Topic Labeling | LLM integration, keyword fallback, label generation |
| Wave 3 | Importance Scoring | Time decay calculation, GetTopTopics RPC |
| Wave 4 | Topic Relationships | Similar topics, parent/child hierarchy |
| Wave 5 | Navigation RPCs | GetTopicsByQuery, GetTocNodesForTopic, GetRelatedTopics |
| Wave 6 | Lifecycle Management | Pruning, resurrection, CLI commands, status RPC |

---

## Appendix A: Proto Definitions

```protobuf
// Topic entity
message Topic {
  string topic_id = 1;
  string label = 2;
  double importance_score = 3;
  int32 node_count = 4;
  int64 created_at_ms = 5;
  int64 last_mentioned_at_ms = 6;
  repeated string keywords = 7;
}

// Topic relationship types
enum RelationshipType {
  RELATIONSHIP_TYPE_UNSPECIFIED = 0;
  RELATIONSHIP_TYPE_SIMILAR = 1;
  RELATIONSHIP_TYPE_PARENT = 2;
  RELATIONSHIP_TYPE_CHILD = 3;
}

// Related topic with relationship info
message RelatedTopic {
  Topic topic = 1;
  RelationshipType relationship_type = 2;
  float score = 3;
}

// ============================================
// GetTopicsByQuery
// ============================================

message GetTopicsByQueryRequest {
  string query = 1;
  int32 limit = 2;
  float min_score = 3;
}

message GetTopicsByQueryResponse {
  repeated Topic topics = 1;
  repeated float scores = 2;
}

// ============================================
// GetTocNodesForTopic
// ============================================

message GetTocNodesForTopicRequest {
  string topic_id = 1;
  int32 limit = 2;
  float min_relevance = 3;
}

message TopicNodeLink {
  string node_id = 1;
  string title = 2;
  TocLevel level = 3;
  float relevance = 4;
  int64 timestamp_ms = 5;
}

message GetTocNodesForTopicResponse {
  repeated TopicNodeLink nodes = 1;
  bool has_more = 2;
}

// ============================================
// GetTopTopics
// ============================================

message GetTopTopicsRequest {
  int32 limit = 1;
  optional int64 start_time_ms = 2;
  optional int64 end_time_ms = 3;
}

message GetTopTopicsResponse {
  repeated Topic topics = 1;
}

// ============================================
// GetRelatedTopics
// ============================================

message GetRelatedTopicsRequest {
  string topic_id = 1;
  repeated RelationshipType relationship_types = 2;
  int32 limit = 3;
}

message GetRelatedTopicsResponse {
  repeated RelatedTopic related = 1;
}

// ============================================
// GetTopicGraphStatus
// ============================================

message GetTopicGraphStatusRequest {}

message GetTopicGraphStatusResponse {
  bool enabled = 1;
  bool healthy = 2;
  int64 topic_count = 3;
  int64 link_count = 4;
  int64 last_extraction_ms = 5;
  string message = 6;

  // Configuration summary
  int32 half_life_days = 7;
  float similarity_threshold = 8;
}
```

---

## Appendix B: Example Usage Scenarios

### Scenario 1: Discover Main Themes

```
User: "What have I been working on this month?"

Agent:
1. GetTopTopics(limit=5, start_time=month_start)
   → [Authentication (0.85), API Design (0.72),
      Database (0.65), Testing (0.58), Security (0.52)]

2. Present: "Your main themes this month:
   1. Authentication (8 conversations)
   2. API Design (6 conversations)
   3. Database work (5 conversations)
   Would you like me to explore any of these?"
```

### Scenario 2: Explore Related Concepts

```
User: "Tell me more about the security work"

Agent:
1. GetTopicsByQuery("security")
   → [Security (0.95)]

2. GetRelatedTopics("topic:security", types=[SIMILAR, CHILD])
   → [Authentication (similar, 0.82),
      Authorization (similar, 0.78),
      OWASP Compliance (child, 0.70)]

3. GetTocNodesForTopic("topic:security", limit=3)
   → [Day 2026-01-28 "Security audit prep",
      Day 2026-01-22 "OWASP review",
      Segment 2026-01-15-abc "Input validation"]

4. Present: "Your security work includes authentication,
   authorization, and OWASP compliance. Recent discussions:
   - Jan 28: Security audit preparation
   - Jan 22: OWASP review
   Want me to expand on any of these?"
```

### Scenario 3: Connect Conversations

```
User: "Is the JWT discussion related to the OAuth work?"

Agent:
1. GetTopicsByQuery("JWT")
   → [JWT Tokens (0.92)]

2. GetRelatedTopics("topic:jwt-tokens")
   → [Authentication (parent, 0.88),
      OAuth2 (similar, 0.75),
      Token Refresh (similar, 0.70)]

3. Present: "Yes, JWT and OAuth are related through your
   Authentication topic. Both appear in your auth work:
   - JWT: 4 conversations about token handling
   - OAuth2: 3 conversations about OAuth2 setup
   They share the same parent theme (Authentication)."
```

---

## Appendix C: Storage Estimates

| Data Point | Count (1 year) | Size per Record | Total |
|------------|----------------|-----------------|-------|
| Topics | ~500 | 4KB (embedding + metadata) | 2MB |
| Topic Links | ~10,000 | 100 bytes | 1MB |
| Topic Relationships | ~2,500 | 50 bytes | 125KB |
| **Total** | | | **~3.5MB/year** |

With pruning (180-day retention for inactive topics):
- Active topics: ~200 at any time
- Active index: ~1MB

---

## Appendix D: Agent Skill Integration Guide

### Pattern 1: Check-Then-Use

```rust
async fn discover_topics(query: &str) -> Result<Vec<Topic>> {
    // First, check if topics are available
    let status = client.get_topic_graph_status().await?;

    if !status.enabled || !status.healthy {
        // Fall back to vector search for similar content
        return fallback_to_vector_search(query).await;
    }

    // Topics available - use them
    client.get_topics_by_query(GetTopicsByQueryRequest {
        query: query.to_string(),
        limit: 10,
        min_score: 0.5,
    }).await
}
```

### Pattern 2: Progressive Enhancement

```markdown
## Search Capability Tiers

This skill supports four search tiers with automatic fallback:

### Tier 1: Topic-Guided Search (Best)
**When:** Topics enabled and healthy
**Method:** GetTopicsByQuery → GetTocNodesForTopic
**Capability:** Finds conceptual themes and related conversations

### Tier 2: Semantic Search (Great)
**When:** Topics disabled, vector available
**Method:** VectorTeleport or HybridSearch
**Capability:** Finds semantically similar content

### Tier 3: Keyword Search (Good)
**When:** Topics and vector disabled, BM25 available
**Method:** TeleportSearch RPC
**Capability:** Finds exact keywords and stems

### Tier 4: Agentic Navigation (Always Works)
**When:** No indexes available
**Method:** SearchChildren RPC
**Capability:** Traverses TOC hierarchy with term matching

The skill automatically selects the best available tier.
```

### Pattern 3: User Communication

```markdown
## When Topics are Disabled

If a user requests topic exploration but it's disabled:

1. **Inform clearly:** "Topic navigation is not enabled. Using search instead."
2. **Suggest enabling:** "Enable with: `topics.enabled: true` in config"
3. **Show results:** Provide search results as alternative
4. **Don't fail silently:** Always tell user which method was used
```

---

*PRD Created: 2026-02-01*
*Last Updated: 2026-02-01*
*Author: Agent Memory Team*
*Status: Draft for Architecture Review*
