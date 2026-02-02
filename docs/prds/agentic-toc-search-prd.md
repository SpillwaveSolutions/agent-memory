# Agentic TOC Search - Product Requirements Document

**Version:** 1.0
**Date:** 2026-02-01
**Phase:** 10.5

---

## 1. Problem Statement

### Current State

The agent-memory system has a complete TOC (Table of Contents) hierarchy that organizes conversation history into a navigable tree structure (Year > Month > Week > Day > Segment). Agents can traverse this hierarchy using existing RPCs (`GetTocRoot`, `GetNode`, `BrowseToc`), but they lack the ability to efficiently search within or across nodes to find relevant content.

### Pain Points

1. **Blind Navigation**: Agents must manually inspect each node's content to determine relevance, wasting tokens and time
2. **No Search Capability**: There's no way to ask "which month discussed JWT authentication?" without reading every month
3. **Delayed Indexing**: Future BM25/vector indexes (Phases 11-12) add complexity and startup time; users need search now
4. **No Fallback**: If indexes are unavailable or corrupted, there's no backup search mechanism

### Opportunity

Add simple, index-free search that enables agents to efficiently navigate the TOC hierarchy by searching for keywords. This becomes the foundational "always works" capability that indexes can optionally accelerate.

---

## 2. User Stories

### Primary Users

1. **Autonomous Agents** (primary) - Claude Code agents querying past conversations
2. **Plugin Commands** (secondary) - Users invoking `/memory-search` commands
3. **Developers** (tertiary) - Testing via CLI during development

### User Stories

| ID | As a... | I want to... | So that... |
|----|---------|--------------|------------|
| US-01 | Agent | search for keywords within a TOC node | I can determine if this node is relevant without reading everything |
| US-02 | Agent | search across all children of a parent node | I can find which child (week/day/segment) to drill into |
| US-03 | Agent | see which bullets matched and their grip evidence | I can cite specific evidence when answering |
| US-04 | Agent | receive a relevance score for matches | I can prioritize which paths to explore |
| US-05 | Agent | limit response size by token budget | I don't exceed context limits |
| US-06 | User | run `/memory-search jwt authentication` | I get relevant conversations without complex navigation |
| US-07 | Developer | use CLI to test searches | I can debug and verify search behavior |
| US-08 | Agent | understand why a navigation path was chosen | I can explain my reasoning to users |

---

## 3. Functional Requirements

### FR-01: SearchNode RPC

**Description:** Enable searching within a single TOC node's fields.

**Acceptance Criteria:**
- [ ] Search matches against title, summary, bullets, and keywords fields
- [ ] Query terms are space-separated, case-insensitive
- [ ] Short terms (< 3 characters) are filtered out
- [ ] Matches include the field that matched and matching text
- [ ] Bullet matches include associated grip IDs for provenance
- [ ] Relevance score (0.0-1.0) based on term overlap
- [ ] Optional limit parameter caps number of matches
- [ ] Optional token_budget parameter for response size control

**gRPC Definition:**
```protobuf
rpc SearchNode(SearchNodeRequest) returns (SearchNodeResponse);
```

### FR-02: SearchChildren RPC

**Description:** Enable searching across all children of a parent node.

**Acceptance Criteria:**
- [ ] Search all children at a specified level (Year/Month/Week/Day/Segment)
- [ ] Empty parent_id searches root level (Years)
- [ ] Returns list of nodes that matched with their matches
- [ ] Nodes sorted by aggregate relevance score (highest first)
- [ ] Optional limit parameter caps number of nodes returned
- [ ] has_more flag indicates additional results available

**gRPC Definition:**
```protobuf
rpc SearchChildren(SearchChildrenRequest) returns (SearchChildrenResponse);
```

### FR-03: Search Algorithm

**Description:** Simple term-matching algorithm that works without external dependencies.

**Acceptance Criteria:**
- [ ] Uses term overlap scoring (matched_terms / total_terms)
- [ ] Case-insensitive matching
- [ ] Filters terms shorter than 3 characters
- [ ] Works without Tantivy or any other index
- [ ] Performance: < 100ms for searching 100 nodes

### FR-04: CLI Search Command

**Description:** Command-line interface for testing and direct search.

**Acceptance Criteria:**
- [ ] `memory-daemon search --query "terms"` searches at default level
- [ ] `--level` flag specifies hierarchy level (year/month/week/day/segment)
- [ ] `--node` flag searches within a specific node
- [ ] `--parent` flag searches children of a parent
- [ ] `--limit` flag caps results
- [ ] Output shows node IDs, titles, matches, and scores

### FR-05: Agent Navigation Loop

**Description:** Navigator agent uses search to efficiently traverse hierarchy.

**Acceptance Criteria:**
- [ ] Agent starts at appropriate level based on query time hints
- [ ] Uses SearchChildren to find relevant nodes at each level
- [ ] Drills into highest-scoring matches
- [ ] Stops at Segment level and returns bullet evidence
- [ ] Tracks visited nodes to avoid loops
- [ ] Respects max iteration limit
- [ ] Outputs explainable path showing navigation decisions

### FR-06: Explainability

**Description:** Search results and navigation paths are explainable.

**Acceptance Criteria:**
- [ ] Each match includes which field matched
- [ ] Navigation path shows why each level was chosen
- [ ] Match scores visible to agents and users
- [ ] Grip IDs provided for evidence verification

---

## 4. Non-Functional Requirements

### NFR-01: Performance

| Metric | Target |
|--------|--------|
| Single node search latency | < 10ms |
| Children search (100 nodes) | < 100ms |
| Full hierarchy navigation | < 500ms |

### NFR-02: Reliability

- Search must work without any index dependencies
- Graceful degradation if query is too broad (return partial results)
- No crashes on empty nodes or missing fields

### NFR-03: Scalability

- Should handle TOC with 1,000+ nodes efficiently
- Token budget support for bounded responses

### NFR-04: Compatibility

- Integrates with existing Storage layer
- Works with existing TocNode structure
- Compatible with future BM25/vector teleport phases

### NFR-05: Observability

- Search operations logged with tracing
- Duration metrics for performance monitoring
- Query and result counts tracked

---

## 5. Success Metrics

### Adoption Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Search RPC usage | > 50% of memory queries use search | gRPC metrics |
| Agent navigation success | > 80% find relevant content | Agent feedback |

### Performance Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Search latency p50 | < 20ms | Tracing spans |
| Search latency p99 | < 100ms | Tracing spans |
| Token efficiency | < 1000 tokens to reach answer | Agent token tracking |

### Quality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Precision (relevant results / returned results) | > 70% | Manual sampling |
| Recall (found answers / answerable queries) | > 60% | Manual sampling |
| User satisfaction | No complaints about search quality | User feedback |

---

## 6. Out of Scope

The following are explicitly NOT part of Phase 10.5:

| Item | Reason | Future Phase |
|------|--------|--------------|
| BM25 ranking | Adds Tantivy dependency | Phase 11 |
| Semantic/vector search | Adds HNSW + embedding model | Phase 12 |
| Real-time index updates | Requires outbox consumer | Phase 13 |
| Fuzzy matching | Simple term overlap sufficient for MVP | Future |
| Query parsing (AND/OR/NOT) | Simple space-separated terms for MVP | Future |
| Highlighting | Not needed for agent consumption | Future |
| Search result caching | Optimize later if needed | Future |

---

## 7. Technical Constraints

### Must Use

- Existing `Storage` trait for node access
- Existing `TocNode` structure (no schema changes)
- Existing gRPC server infrastructure
- Rust with async/await

### Must Not

- Add new external dependencies for search (no Tantivy, no meilisearch)
- Modify existing RPC signatures
- Break existing TOC navigation flows

### Assumptions

- TOC nodes already contain meaningful titles, summaries, bullets, and keywords
- Summarization quality is sufficient for term matching
- Most queries involve keywords that appear in summaries

---

## 8. Dependencies

### Internal Dependencies

| Dependency | Reason | Status |
|------------|--------|--------|
| Phase 10 (Scheduler) | Phase ordering | Complete |
| TOC Building (Phase 2) | Nodes must exist to search | Complete |
| Grips (Phase 3) | Provenance links in bullets | Complete |

### External Dependencies

None. This phase is intentionally index-free.

---

## 9. Rollout Plan

### Phase 10.5 Execution

1. **Wave 1: Core Search Logic**
   - Implement `search_node()` function in memory-toc
   - Add unit tests
   - Duration: 1 plan

2. **Wave 2: gRPC Integration**
   - Add proto definitions
   - Implement RPC handlers
   - Add integration tests
   - Duration: 1 plan

3. **Wave 3: CLI & Agent**
   - Add CLI commands
   - Update memory-navigator agent
   - Update skill documentation
   - Duration: 1 plan

### Verification

- All unit tests pass
- All integration tests pass
- CLI `search` command works
- Agent can find content using search RPCs
- No performance regressions

---

## 10. Open Questions

| Question | Status | Resolution |
|----------|--------|------------|
| Should search support regex patterns? | Decided | No - simple term matching for MVP |
| What's the max nodes to search before timeout? | Decided | 1000 nodes, with pagination |
| Should we cache search results? | Decided | No - optimize later if needed |

---

## 11. Appendix

### A. Proto Message Reference

```protobuf
// Search field enumeration
enum SearchField {
  SEARCH_FIELD_UNSPECIFIED = 0;
  SEARCH_FIELD_TITLE = 1;
  SEARCH_FIELD_SUMMARY = 2;
  SEARCH_FIELD_BULLETS = 3;
  SEARCH_FIELD_KEYWORDS = 4;
}

// Search within single node
message SearchNodeRequest {
  string node_id = 1;
  string query = 2;
  repeated SearchField fields = 3;
  int32 limit = 4;
  int32 token_budget = 5;
}

message SearchMatch {
  SearchField field = 1;
  string text = 2;
  repeated string grip_ids = 3;
  float score = 4;
}

message SearchNodeResponse {
  bool matched = 1;
  repeated SearchMatch matches = 2;
  string node_id = 3;
  TocLevel level = 4;
}

// Search children of parent
message SearchChildrenRequest {
  string parent_id = 1;
  string query = 2;
  TocLevel child_level = 3;
  repeated SearchField fields = 4;
  int32 limit = 5;
  int32 token_budget = 6;
}

message SearchNodeResult {
  string node_id = 1;
  string title = 2;
  TocLevel level = 3;
  repeated SearchMatch matches = 4;
  float relevance_score = 5;
}

message SearchChildrenResponse {
  repeated SearchNodeResult results = 1;
  bool has_more = 2;
}
```

### B. Example Agent Navigation

```
User Query: "What did we discuss about JWT tokens last week?"

Agent Path:
1. Parse time hint: "last week" -> Week 2026-W04
2. SearchChildren(parent="toc:week:2026-W04", query="JWT tokens", level=Day)
   -> Results: [Day 2026-01-30 (score: 0.85), Day 2026-01-28 (score: 0.62)]
3. Drill into Day 2026-01-30
4. SearchChildren(parent="toc:day:2026-01-30", query="JWT tokens", level=Segment)
   -> Results: [Segment abc123 (score: 0.92)]
5. Return bullets from Segment abc123 with grip IDs
6. Offer: "Found 2 relevant points. Expand grip:xyz for full context?"
```

---

*PRD Created: 2026-02-01*
*Last Updated: 2026-02-01*
*Author: Agent Memory Team*
