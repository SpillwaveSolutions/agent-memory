# Phase 10.5: Agentic TOC Search - Technical Plan

## Overview

This phase adds the foundational agentic search capability that uses TOC navigation and time-based progressive disclosure. This is the "always works" core that doesn't require any indexes (BM25, vector, graph are optional accelerators built in later phases).

**Phase Type:** Inserted phase (10.5)
**Depends On:** Phase 10 (Background Scheduler)
**Required By:** Phases 11-13 (optional accelerators build on this foundation)

## Goals

1. Enable agents to search within TOC nodes using simple grep/term matching
2. Provide explainable navigation paths from root to evidence
3. Work without any index dependencies (Tantivy, HNSW not required)
4. Support token-budgeted responses for efficient context usage

---

## Foundational Data Architecture

### Layer Hierarchy

```
+---------------------------------------------------------------------+
|  MONTH                                                              |
|  +-- TOC: Points to weeks within this month                         |
|  +-- Summary: Aggregated themes from all weeks                      |
+---------------------------------------------------------------------+
|  WEEK                                                               |
|  +-- TOC: Points to days within this week                           |
|  +-- Summary: Aggregated themes from all days                       |
+---------------------------------------------------------------------+
|  DAY                                                                |
|  +-- TOC: Points to segments within this day                        |
|  +-- Summary: Aggregated themes from all segments                   |
+---------------------------------------------------------------------+
|  SEGMENT (leaf node)                                                |
|  +-- TOC: Points to grips (evidence for each bullet)                |
|  +-- Summary: Bullets with keywords                                 |
|  +-- Context:                                                       |
|      +-- What came before (overlap from previous segment)           |
|      +-- Where this fits (parent day/week/month references)         |
+---------------------------------------------------------------------+
|  RAW EVENTS (base)                                                  |
|  +-- Immutable conversation events (user, assistant, tool, etc.)    |
+---------------------------------------------------------------------+
```

### Segment Structure (Most Important)

Segments are the leaf nodes of the TOC hierarchy. Each segment contains:

| Component | Description | Storage |
|-----------|-------------|---------|
| **TOC** | Bullet points with `grip_ids` linking to evidence | `TocNode.bullets[].grip_ids` |
| **Summary** | Title + keywords describing segment content | `TocNode.title`, `TocNode.keywords` |
| **Context** | | |
| - Overlap | Events from end of previous segment for continuity | Stored in segment creation (overlap window) |
| - Position | Parent node references (day -> week -> month -> year) | Derived from `node_id` format |
| - Time Range | Start/end timestamps for this segment | `TocNode.start_time`, `TocNode.end_time` |

### How Links Work

Each layer's TOC contains explicit pointers to the layer below:

```
Month Node (toc:month:2026-01)
+-- child_node_ids: ["toc:week:2026-W01", "toc:week:2026-W02", ...]
    |
    v
    Week Node (toc:week:2026-W01)
    +-- child_node_ids: ["toc:day:2026-01-01", "toc:day:2026-01-02", ...]
        |
        v
        Day Node (toc:day:2026-01-01)
        +-- child_node_ids: ["toc:segment:2026-01-01:abc123", ...]
            |
            v
            Segment Node (toc:segment:2026-01-01:abc123)
            +-- bullets[0].grip_ids: ["grip:1706745600000:xyz"]
                |
                v
                Grip (grip:1706745600000:xyz)
                +-- event_id_start: "01HN4QX..."
                +-- event_id_end: "01HN4QY..."
                    |
                    v
                    Raw Events (evt:1706745600000:01HN4QX...)
```

These are RocksDB keys:
- `node_id` = key in `CF_TOC_NODES`
- `grip_id` = key in `CF_GRIPS`
- `event_id` = key in `CF_EVENTS`

---

## gRPC API Design

### New RPCs

Add to `MemoryService` in `proto/memory.proto`:

```protobuf
// Search within a specific node's content
rpc SearchNode(SearchNodeRequest) returns (SearchNodeResponse);

// Search across children of a parent node
rpc SearchChildren(SearchChildrenRequest) returns (SearchChildrenResponse);
```

### SearchNode Messages

Grep within a single node's fields (title, summary, bullets, keywords).

```protobuf
// Fields to search within a TOC node
enum SearchField {
  SEARCH_FIELD_UNSPECIFIED = 0;
  SEARCH_FIELD_TITLE = 1;
  SEARCH_FIELD_SUMMARY = 2;
  SEARCH_FIELD_BULLETS = 3;
  SEARCH_FIELD_KEYWORDS = 4;
}

// Request to search within a single node
message SearchNodeRequest {
  // Node to search within
  string node_id = 1;
  // Search terms (space-separated, OR matching)
  string query = 2;
  // Fields to search (default: all)
  repeated SearchField fields = 3;
  // Max matches to return (default: 10)
  int32 limit = 4;
  // Optional: limit response tokens for budget control
  int32 token_budget = 5;
}

// A match within a node
message SearchMatch {
  // Which field matched
  SearchField field = 1;
  // Matching text snippet
  string text = 2;
  // If bullet, the supporting grip IDs
  repeated string grip_ids = 3;
  // Simple term overlap score (0.0-1.0)
  float score = 4;
}

// Response from node search
message SearchNodeResponse {
  // Whether any matches found
  bool matched = 1;
  // Individual matches
  repeated SearchMatch matches = 2;
  // Node that was searched
  string node_id = 3;
  // Level of the searched node
  TocLevel level = 4;
}
```

### SearchChildren Messages

Grep across all children of a parent at a specific level.

```protobuf
// Request to search children of a parent node
message SearchChildrenRequest {
  // Parent node ID (empty string for root/year level)
  string parent_id = 1;
  // Search terms (space-separated, OR matching)
  string query = 2;
  // Level to search at
  TocLevel child_level = 3;
  // Fields to search (default: all)
  repeated SearchField fields = 4;
  // Max nodes to return (default: 10)
  int32 limit = 5;
  // Optional: limit response tokens for budget control
  int32 token_budget = 6;
}

// A node that matched with its matches
message SearchNodeResult {
  // Node ID of the matching node
  string node_id = 1;
  // Node title for display
  string title = 2;
  // Level of this node
  TocLevel level = 3;
  // Matches within this node
  repeated SearchMatch matches = 4;
  // Aggregate relevance score
  float relevance_score = 5;
}

// Response from children search
message SearchChildrenResponse {
  // Matching nodes with their matches
  repeated SearchNodeResult results = 1;
  // Whether more results available
  bool has_more = 2;
}
```

---

## Implementation Components

### Component Layout

| Component | Crate | File | Purpose |
|-----------|-------|------|---------|
| Search logic | `memory-toc` | `src/search.rs` (new) | Core search algorithms |
| Search types | `memory-toc` | `src/search.rs` | SearchMatch, SearchField enums |
| gRPC handlers | `memory-service` | `src/search_service.rs` (new) | RPC implementations |
| Proto definitions | `proto` | `memory.proto` (extend) | Message definitions |
| CLI commands | `memory-daemon` | `src/cli.rs` (extend) | `search` and `navigate` commands |

### Search Algorithm

Simple term-overlap matching (no BM25 dependencies):

```rust
// memory-toc/src/search.rs

/// Fields that can be searched within a TOC node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchField {
    Title,
    Summary,
    Bullets,
    Keywords,
}

/// A match found during search
#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub field: SearchField,
    pub text: String,
    pub grip_ids: Vec<String>,
    pub score: f32,
}

/// Search within a single node's fields
pub fn search_node(
    node: &TocNode,
    query: &str,
    fields: &[SearchField],
) -> Vec<SearchMatch> {
    let terms: Vec<&str> = query
        .split_whitespace()
        .filter(|t| t.len() > 2)  // Skip short words
        .map(|t| t.to_lowercase())
        .collect();

    if terms.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();

    // Search title
    if fields.contains(&SearchField::Title) || fields.is_empty() {
        if let Some(score) = term_overlap_score(&node.title, &terms) {
            matches.push(SearchMatch {
                field: SearchField::Title,
                text: node.title.clone(),
                grip_ids: Vec::new(),
                score,
            });
        }
    }

    // Search summary
    if fields.contains(&SearchField::Summary) || fields.is_empty() {
        if let Some(ref summary) = node.summary {
            if let Some(score) = term_overlap_score(summary, &terms) {
                matches.push(SearchMatch {
                    field: SearchField::Summary,
                    text: summary.clone(),
                    grip_ids: Vec::new(),
                    score,
                });
            }
        }
    }

    // Search bullets (with grip links)
    if fields.contains(&SearchField::Bullets) || fields.is_empty() {
        for bullet in &node.bullets {
            if let Some(score) = term_overlap_score(&bullet.text, &terms) {
                matches.push(SearchMatch {
                    field: SearchField::Bullets,
                    text: bullet.text.clone(),
                    grip_ids: bullet.grip_ids.clone(),
                    score,
                });
            }
        }
    }

    // Search keywords
    if fields.contains(&SearchField::Keywords) || fields.is_empty() {
        for kw in &node.keywords {
            if terms.iter().any(|t| kw.to_lowercase().contains(t)) {
                matches.push(SearchMatch {
                    field: SearchField::Keywords,
                    text: kw.clone(),
                    grip_ids: Vec::new(),
                    score: 1.0,
                });
            }
        }
    }

    // Sort by score descending
    matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    matches
}

/// Calculate term overlap score (0.0-1.0)
fn term_overlap_score(text: &str, terms: &[&str]) -> Option<f32> {
    let text_lower = text.to_lowercase();
    let matched_count = terms.iter().filter(|t| text_lower.contains(*t)).count();

    if matched_count > 0 {
        Some(matched_count as f32 / terms.len() as f32)
    } else {
        None
    }
}
```

### gRPC Service Implementation

```rust
// memory-service/src/search_service.rs

impl MemoryService {
    pub async fn search_node(
        &self,
        request: Request<SearchNodeRequest>,
    ) -> Result<Response<SearchNodeResponse>, Status> {
        let req = request.into_inner();

        // Load the node
        let node = self.storage.get_toc_node(&req.node_id)
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("Node not found"))?;

        // Convert proto fields to domain fields
        let fields: Vec<SearchField> = req.fields
            .iter()
            .filter_map(|f| proto_to_search_field(*f))
            .collect();

        // Execute search
        let matches = search_node(&node, &req.query, &fields);

        // Apply limit
        let limit = if req.limit > 0 { req.limit as usize } else { 10 };
        let matches: Vec<_> = matches.into_iter().take(limit).collect();

        Ok(Response::new(SearchNodeResponse {
            matched: !matches.is_empty(),
            matches: matches.into_iter().map(|m| m.into()).collect(),
            node_id: req.node_id,
            level: node.level.into(),
        }))
    }

    pub async fn search_children(
        &self,
        request: Request<SearchChildrenRequest>,
    ) -> Result<Response<SearchChildrenResponse>, Status> {
        let req = request.into_inner();

        // Get children of parent
        let children = if req.parent_id.is_empty() {
            // Search root level (years)
            self.storage.get_toc_root()?
        } else {
            self.storage.get_child_nodes(&req.parent_id)?
        };

        // Convert proto fields to domain fields
        let fields: Vec<SearchField> = req.fields
            .iter()
            .filter_map(|f| proto_to_search_field(*f))
            .collect();

        // Search each child and collect results
        let mut results: Vec<SearchNodeResult> = Vec::new();
        for child in children {
            let matches = search_node(&child, &req.query, &fields);
            if !matches.is_empty() {
                let relevance = matches.iter().map(|m| m.score).sum::<f32>()
                    / matches.len() as f32;
                results.push(SearchNodeResult {
                    node_id: child.node_id.clone(),
                    title: child.title.clone(),
                    level: child.level.into(),
                    matches: matches.into_iter().map(|m| m.into()).collect(),
                    relevance_score: relevance,
                });
            }
        }

        // Sort by relevance and apply limit
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score)
            .unwrap_or(Ordering::Equal));

        let limit = if req.limit > 0 { req.limit as usize } else { 10 };
        let has_more = results.len() > limit;
        let results: Vec<_> = results.into_iter().take(limit).collect();

        Ok(Response::new(SearchChildrenResponse {
            results,
            has_more,
        }))
    }
}
```

---

## Navigator Agent Skill

### Skill Structure

Create/extend the memory-query plugin with navigator capabilities:

```
skills/memory-query/
+-- SKILL.md           # Updated with navigator capabilities
+-- references/
|   +-- search-api.md  # API reference for SearchNode/SearchChildren
|   +-- patterns.md    # Common search patterns
+-- agents/
    +-- memory-navigator.md   # Agent definition (update existing)
```

### Agent Navigation Loop

```
1. Parse user query for time hints and topics
   - Time hints: "yesterday", "last week", "in January"
   - Topics: keywords, technical terms

2. Start at root (GetTocRoot) or time-hinted level

3. Navigation loop:
   a. SearchChildren(current_level, query)
   b. If strong match found (score > 0.5):
      - Explain: "Found relevant content in [Month X]: [match text]"
      - Drill down: GetNode(matched_id)
   c. If at Segment level:
      - Return bullets with grip_ids
      - Offer to ExpandGrip for verification
   d. If no matches and budget remaining:
      - Try sibling nodes (adjacent time periods)
      - Try broader time range
   e. If budget exhausted:
      - Return best partial results with explanation

4. Return path taken + evidence with citations
```

### Explainability Output Format

```markdown
## Search Path

Query: "JWT authentication debugging"

1. **Year 2026** - Searched 1 year node
   - Matched: "authentication infrastructure work" (score: 0.75)
   - Drilling into January (strongest month match)

2. **Month 2026-01** - Searched 4 week nodes
   - Matched: "JWT implementation and testing" (score: 0.82)
   - Drilling into Week 4 (multiple JWT references)

3. **Week 2026-W04** - Searched 5 day nodes
   - Matched: "token refresh debugging" (score: 0.91)
   - Drilling into Thursday (debugging session)

4. **Day 2026-01-30** - Searched 3 segment nodes
   - Matched: "JWT expiry bug fixed" (score: 0.95)
   - Found relevant segment

## Evidence

**Segment: toc:segment:2026-01-30:abc123**
- "Debugged JWT token expiration issue" [grip:abc123]
- "Fixed refresh token rotation logic" [grip:def456]

Would you like me to expand a grip for the full conversation context?
```

---

## CLI Commands

### New Commands

```bash
# Search at a specific level
memory-daemon search --level month --query "authentication"

# Search within a specific node
memory-daemon search --node "toc:month:2026-01" --query "JWT"

# Search children of a node
memory-daemon search --parent "toc:week:2026-W04" --query "debugging"

# Interactive navigator (uses agent loop, shows path)
memory-daemon navigate "find discussions about JWT debugging"
```

### CLI Implementation

```rust
// memory-daemon/src/cli.rs

#[derive(Parser)]
enum Commands {
    // ... existing commands ...

    /// Search TOC nodes for matching content
    Search {
        /// Search query terms
        #[arg(long)]
        query: String,

        /// Level to search (year, month, week, day, segment)
        #[arg(long)]
        level: Option<String>,

        /// Specific node ID to search within
        #[arg(long)]
        node: Option<String>,

        /// Parent node ID to search children of
        #[arg(long)]
        parent: Option<String>,

        /// Maximum results to return
        #[arg(long, default_value = "10")]
        limit: u32,
    },

    /// Navigate TOC hierarchy to find relevant content
    Navigate {
        /// Natural language query
        query: String,

        /// Show detailed navigation path
        #[arg(long)]
        verbose: bool,
    },
}
```

---

## Testing Strategy

### Unit Tests

```rust
// memory-toc/src/search.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_overlap_single_match() {
        let score = term_overlap_score("JWT authentication", &["jwt"]);
        assert!(score.is_some());
        assert_eq!(score.unwrap(), 1.0);
    }

    #[test]
    fn test_term_overlap_partial_match() {
        let score = term_overlap_score("JWT authentication", &["jwt", "debugging"]);
        assert!(score.is_some());
        assert_eq!(score.unwrap(), 0.5);  // 1 of 2 terms matched
    }

    #[test]
    fn test_term_overlap_no_match() {
        let score = term_overlap_score("JWT authentication", &["vector", "embedding"]);
        assert!(score.is_none());
    }

    #[test]
    fn test_search_node_title_match() {
        let node = TocNode {
            node_id: "test".to_string(),
            title: "JWT Token Debugging Session".to_string(),
            summary: None,
            bullets: vec![],
            keywords: vec![],
            // ... other fields
        };

        let matches = search_node(&node, "jwt debugging", &[SearchField::Title]);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].field, SearchField::Title);
    }

    #[test]
    fn test_search_node_bullet_with_grips() {
        let node = TocNode {
            node_id: "test".to_string(),
            title: "Session".to_string(),
            summary: None,
            bullets: vec![
                TocBullet {
                    text: "Fixed JWT expiration bug".to_string(),
                    grip_ids: vec!["grip:123".to_string()],
                },
            ],
            keywords: vec![],
            // ... other fields
        };

        let matches = search_node(&node, "jwt bug", &[SearchField::Bullets]);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].grip_ids, vec!["grip:123"]);
    }

    #[test]
    fn test_short_terms_filtered() {
        let node = TocNode {
            node_id: "test".to_string(),
            title: "The JWT Token".to_string(),
            // ... other fields
        };

        // "the" and "jwt" - "the" should be filtered (< 3 chars)
        let matches = search_node(&node, "the jwt", &[SearchField::Title]);
        assert!(!matches.is_empty());
    }
}
```

### Integration Tests

```rust
// memory-service/tests/search_integration.rs

#[tokio::test]
async fn test_search_children_rpc() {
    // Setup test server with sample data
    let server = setup_test_server().await;

    // Ingest test events and build TOC
    ingest_test_conversations(&server).await;
    trigger_toc_build(&server).await;

    // Search for content
    let response = server
        .search_children(SearchChildrenRequest {
            parent_id: "".to_string(),  // Root level
            query: "authentication".to_string(),
            child_level: TocLevel::Month as i32,
            fields: vec![],
            limit: 5,
            token_budget: 0,
        })
        .await
        .unwrap();

    assert!(!response.results.is_empty());
    assert!(response.results[0].relevance_score > 0.0);
}

#[tokio::test]
async fn test_search_drill_down() {
    let server = setup_test_server().await;

    // Ingest and build TOC
    ingest_test_conversations(&server).await;
    trigger_toc_build(&server).await;

    // Find month with matches
    let months = server.search_children(SearchChildrenRequest {
        parent_id: "".to_string(),
        query: "jwt".to_string(),
        child_level: TocLevel::Month as i32,
        ..Default::default()
    }).await.unwrap();

    assert!(!months.results.is_empty());

    // Drill into best month
    let best_month = &months.results[0];
    let weeks = server.search_children(SearchChildrenRequest {
        parent_id: best_month.node_id.clone(),
        query: "jwt".to_string(),
        child_level: TocLevel::Week as i32,
        ..Default::default()
    }).await.unwrap();

    // Should find more specific matches at week level
    assert!(!weeks.results.is_empty());
}
```

---

## Success Criteria

| Criterion | Verification |
|-----------|--------------|
| Agent can answer "find X" using ONLY TOC + summaries + grips | Integration test with agent loop |
| No index dependencies (works on fresh install) | Test without Tantivy/HNSW crates |
| Explainable path: agent shows why it chose each navigation step | Output includes path with match explanations |
| Token-efficient: stays within budget, drills only when needed | Token budget parameter respected |
| Graceful degradation: returns partial results if query too broad | Test with broad queries, verify partial results returned |
| SearchNode RPC works for single-node searches | Unit and integration tests |
| SearchChildren RPC works for level-wide searches | Unit and integration tests |
| CLI search command works | Manual testing via `memory-daemon search` |

---

## Integration Points

### Existing Code Integration

| Existing Component | Integration Point |
|-------------------|-------------------|
| `Storage::get_toc_node` | Used by SearchNode to load single nodes |
| `Storage::get_child_nodes` | Used by SearchChildren to enumerate children |
| `Storage::get_toc_root` | Used for root-level searches |
| `TocNode.bullets[].grip_ids` | Returned in search results for provenance |
| `ExpandGrip` RPC | Used by agent to verify claims (existing) |
| `memory-query` plugin | Extended with search commands and navigator updates |

### Future Phase Integration

| Future Phase | How 10.5 Helps |
|-------------|----------------|
| Phase 11: BM25 Teleport | Can "teleport" to node, then use 10.5's SearchNode for local matches |
| Phase 12: Vector Teleport | Same pattern - teleport to semantic cluster, use 10.5 for final navigation |
| Phase 13: Outbox Ingestion | Indexes just accelerate; 10.5 is always available as fallback |

---

## Implementation Plan

### Wave 1: Core Search (Plan 10.5-01)

**Files Modified:**
- `proto/memory.proto` - Add search messages and RPCs
- `crates/memory-toc/src/search.rs` (new) - Search algorithms
- `crates/memory-toc/src/lib.rs` - Export search module

**Tasks:**
1. Add proto definitions for SearchNode and SearchChildren
2. Implement `search_node()` and `term_overlap_score()` functions
3. Add comprehensive unit tests

### Wave 2: gRPC Service (Plan 10.5-02)

**Files Modified:**
- `crates/memory-service/src/search_service.rs` (new) - RPC handlers
- `crates/memory-service/src/lib.rs` - Wire up service

**Tasks:**
1. Implement SearchNode RPC handler
2. Implement SearchChildren RPC handler
3. Add integration tests

### Wave 3: CLI & Agent (Plan 10.5-03)

**Files Modified:**
- `crates/memory-daemon/src/cli.rs` - Add search/navigate commands
- `skills/memory-query/SKILL.md` - Update with search capabilities
- `skills/memory-query/agents/memory-navigator.md` - Update agent with search loop

**Tasks:**
1. Add `search` CLI command
2. Add `navigate` CLI command (optional)
3. Update memory-navigator agent with search-based navigation
4. Add search API reference documentation

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Simple grep too slow for large TOC | Performance degradation | Early pagination, limit searches to relevant time windows |
| Term matching misses semantic similarity | Lower recall | Documented limitation; Phase 11-12 add semantic search |
| Token budget estimation inaccurate | Over/under budget | Conservative estimates, configurable factor |
| Agent loops infinitely | Resource waste | Max iteration limit, visited node tracking |

---

*Plan created: 2026-02-01*
*Target: Phase 10.5 - Agentic TOC Search*
