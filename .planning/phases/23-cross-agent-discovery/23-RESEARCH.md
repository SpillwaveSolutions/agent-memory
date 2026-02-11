# Phase 23: Cross-Agent Discovery + Documentation - Research

**Researched:** 2026-02-10
**Domain:** Cross-agent discovery RPCs, topic aggregation, CLOD format specification, adapter documentation
**Confidence:** HIGH

## Summary

Phase 23 completes the v2.1 Multi-Agent Ecosystem milestone by adding discovery features (which agents contributed memories, when, and what topics), defining a universal command format (CLOD) for cross-agent adapter generation, and producing comprehensive documentation for adapter authoring and cross-agent usage.

The foundation is already solid. Phase 18 added the `Event.agent` field (proto field 8, `optional string`), `TocNode.contributing_agents` (Vec<String> with `serde(default)`), the `AgentAdapter` trait in `memory-adapters`, and `--agent` CLI filters on teleport/retrieval commands. Phases 19-22 delivered four working adapters (Claude, OpenCode, Gemini, Copilot) with hook-based event capture, skills, and per-agent commands. What remains is aggregation/insight RPCs, agent-aware topic queries, format unification, and documentation.

The key technical challenge is efficiently aggregating agent statistics from RocksDB event storage. Events are stored with time-prefixed keys in a `CF_EVENTS` column family. There is no secondary index on agent. Two approaches exist: (a) scan events at query time, which is expensive for large datasets; (b) maintain a lightweight summary (agent metadata) updated during ingestion, which is efficient but requires a new column family or metadata store. The research recommends approach (b) for the `agents list` command and approach (a) with time-bounded scans for the `agents activity` command.

**Primary recommendation:** Add `ListAgents` and `GetAgentActivity` RPCs to `memory.proto`, implement aggregation in `memory-service`, add `Agents` subcommand to CLI, define CLOD as a TOML-based universal command format, and write three documentation guides covering cross-agent usage, adapter authoring, and CLOD specification.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tonic | 0.12+ | gRPC framework | Already used for all RPCs |
| clap | 4.x | CLI argument parsing | Already used for all CLI commands |
| chrono | 0.4 | Time bucketing for activity | Already a workspace dependency |
| serde/serde_json | 1.x | JSON serialization | Already used throughout |
| toml | 0.8+ | CLOD format parsing/generation | Already available in workspace (used by config) |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tabled | 0.16+ | Pretty table output for CLI | Optional: for `agents list` table formatting |
| comfy-table | 7.x | Alternative table formatting | Alternative to tabled if simpler API preferred |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| New CF for agent metadata | Scan events at startup | Scan is O(n) at startup but avoids schema migration; metadata CF is O(1) query but needs ingestion changes |
| TOML for CLOD | YAML or JSON | TOML is natural for Gemini (already uses .toml commands); YAML used by Claude/OpenCode frontmatter; TOML chosen for human readability |
| tabled for CLI output | Manual println formatting | All existing CLI uses manual println; consistency argues for same approach |

## Architecture Patterns

### Recommended Project Structure

```
proto/memory.proto           # Add ListAgents + GetAgentActivity RPCs
crates/memory-service/
  src/agents.rs              # NEW: Agent aggregation logic
  src/lib.rs                 # Register new service module
crates/memory-daemon/
  src/cli.rs                 # Add Agents subcommand enum
  src/commands.rs            # Add handle_agents_command()
  src/main.rs                # Wire Agents command
docs/adapters/
  cross-agent-guide.md       # NEW: Cross-agent usage guide
  authoring-guide.md         # NEW: Plugin authoring guide
  clod-format.md             # NEW: CLOD specification
```

### Pattern 1: New RPC with Aggregation Service

**What:** Add `ListAgents` and `GetAgentActivity` RPCs that aggregate data from existing storage.
**When to use:** When the caller needs pre-computed agent statistics.
**Implementation approach:**

The `ListAgents` RPC scans `TocNode.contributing_agents` across all TOC nodes (fast: typically hundreds of nodes) and aggregates unique agents with first_seen/last_seen timestamps. This avoids scanning potentially millions of events.

The `GetAgentActivity` RPC uses time-bounded event scans (`get_events_in_range`) to count events per agent in time buckets. The caller specifies agent_id, time range, and bucket granularity (day/week).

**Example proto additions:**
```protobuf
// Agent discovery RPCs
rpc ListAgents(ListAgentsRequest) returns (ListAgentsResponse);
rpc GetAgentActivity(GetAgentActivityRequest) returns (GetAgentActivityResponse);

message AgentSummary {
    string agent_id = 1;
    uint64 event_count = 2;
    uint64 session_count = 3;
    int64 first_seen_ms = 4;
    int64 last_seen_ms = 5;
}

message ListAgentsRequest {}
message ListAgentsResponse {
    repeated AgentSummary agents = 1;
}

message GetAgentActivityRequest {
    optional string agent_id = 1;
    optional int64 from_ms = 2;
    optional int64 to_ms = 3;
    string bucket = 4; // "day" or "week"
}

message ActivityBucket {
    int64 start_ms = 1;
    int64 end_ms = 2;
    uint64 event_count = 3;
    string agent_id = 4;
}

message GetAgentActivityResponse {
    repeated ActivityBucket buckets = 1;
}
```

### Pattern 2: CLI Subcommand Group (matching existing patterns)

**What:** Add `Agents` as a new top-level subcommand with `list` and `activity` sub-subcommands.
**When to use:** All cross-agent discovery CLI features.

The CLI pattern should match the existing `Topics`, `Teleport`, `Retrieval` command groups:

```rust
// In cli.rs, add to Commands enum:
/// Agent discovery commands
#[command(subcommand)]
Agents(AgentsCommand),

#[derive(Subcommand, Debug, Clone)]
pub enum AgentsCommand {
    /// List all contributing agents
    List {
        #[arg(long, default_value = "http://[::1]:50051")]
        addr: String,
    },
    /// Show agent activity timeline
    Activity {
        /// Agent ID to show activity for (all agents if omitted)
        #[arg(long, short = 'a')]
        agent: Option<String>,
        /// Start time (YYYY-MM-DD or Unix ms)
        #[arg(long)]
        from: Option<String>,
        /// End time (YYYY-MM-DD or Unix ms)
        #[arg(long)]
        to: Option<String>,
        /// Bucket granularity: day, week
        #[arg(long, default_value = "day")]
        bucket: String,
        #[arg(long, default_value = "http://[::1]:50051")]
        addr: String,
    },
    /// Show top topics for an agent
    Topics {
        /// Agent ID to show topics for
        agent: String,
        #[arg(long, short = 'n', default_value = "10")]
        limit: u32,
        #[arg(long, default_value = "http://[::1]:50051")]
        addr: String,
    },
}
```

### Pattern 3: Agent-Aware Topic Queries

**What:** Extend existing topic RPCs to support agent filtering.
**When to use:** Cross-agent topic linking (R4.3.3).

Topic data currently lacks an agent dimension. TocNode has `contributing_agents`, and topics link to TocNodes via `TopicLink`. To get topics-by-agent:

1. Get all TopicLinks for all topics
2. For each linked TocNode, check if `contributing_agents` contains the target agent
3. Filter topics to only those with links to agent-contributing nodes
4. Return with per-agent relevance scores

This can be added as:
- A new RPC `GetTopicsByAgent` that takes agent_id and limit
- Or an `agent_filter` field on existing `GetTopTopicsRequest`

**Recommendation:** Add `optional string agent_filter` to `GetTopTopicsRequest` (backward compatible, simpler, consistent with existing filter pattern).

### Pattern 4: CLOD Universal Command Format

**What:** A TOML-based format that describes an agent-memory command generically, enabling generation of Claude `.md`, OpenCode `.md`, Gemini `.toml`, and Copilot `.md` files.
**When to use:** When adding a new command that must work across all agents.

**CLOD (Cross-Language Operation Definition) structure:**

```toml
[command]
name = "memory-search"
description = "Search past conversations by topic or keyword"
version = "1.0"

[[command.parameters]]
name = "topic"
description = "Topic or keyword to search"
required = true
position = 1

[[command.parameters]]
name = "period"
description = "Time period filter"
required = false
flag = "--period"

[[command.parameters]]
name = "agent"
description = "Filter by agent"
required = false
flag = "--agent"

[process]
steps = [
    "Check daemon status: `memory-daemon status`",
    "Check retrieval capabilities: `memory-daemon retrieval status`",
    "Route query: `memory-daemon retrieval route \"<topic>\" [--agent <agent>]`",
    "Fallback to TOC navigation if no results",
]

[output]
format = """
## Memory Search: [topic]

### [Time Period]
**Summary:** [matching bullet points]

**Excerpts:**
- "[excerpt text]" `grip:ID`
  _Source: [timestamp]_

---
Expand any excerpt: /memory-context grip:ID
"""

[adapters.claude]
directory = "commands/"
extension = ".md"
template = "yaml-frontmatter"

[adapters.opencode]
directory = ".opencode/command/"
extension = ".md"
template = "arguments-substitution"

[adapters.gemini]
directory = ".gemini/commands/"
extension = ".toml"
template = "toml-prompt"

[adapters.copilot]
directory = ".github/skills/"
extension = ".md"
template = "skill-embedded"
```

### Anti-Patterns to Avoid

- **Scanning all events for `agents list`:** This is O(n) where n = total events (could be millions). Use TOC contributing_agents aggregation instead (O(k) where k = TOC nodes, typically hundreds).
- **Adding agent field to existing proto messages without `optional`:** Always use `optional string` for backward compatibility.
- **Breaking existing topic RPCs:** Add new fields with defaults, never change existing field semantics.
- **Separate CLI binary for CLOD:** CLOD convert should be a subcommand of `memory-daemon`, not a separate binary.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Agent stats from events | Full event scan on every call | Aggregate from TocNode.contributing_agents | TOC nodes already track agents; scanning events is O(n) |
| Time bucketing | Custom date arithmetic | chrono::NaiveDate + Duration | Already in workspace, handles edge cases |
| Proto backward compat | Version negotiation | `optional` fields + `serde(default)` | Existing pattern from Phase 18 |
| Table formatting | Custom column alignment | Match existing println! patterns | Consistency with existing CLI output |
| Command format conversion | Per-adapter custom code | Template-based CLOD generator | Templates are easier to maintain than code |

**Key insight:** The cross-agent discovery features can be built almost entirely on existing infrastructure. TocNode.contributing_agents, Event.agent, and the --agent filter pattern are already in place. The main new code is aggregation logic and CLI presentation.

## Common Pitfalls

### Pitfall 1: Event Scan Performance for Agent Statistics

**What goes wrong:** Scanning all events in RocksDB to count per-agent statistics takes seconds on large datasets.
**Why it happens:** Events are keyed by timestamp, not by agent. No secondary index exists.
**How to avoid:** Derive agent statistics from TocNode.contributing_agents (fast, O(k) where k = number of TOC nodes). For detailed activity (event counts per bucket), use time-bounded scans with `get_events_in_range` and parse only the agent field.
**Warning signs:** `agents list` takes more than 100ms; events CF has >100K entries.

### Pitfall 2: Proto Field Number Conflicts

**What goes wrong:** New proto fields conflict with existing field numbers.
**Why it happens:** Multiple phases add fields to the same message.
**How to avoid:** Use field numbers > 200 for Phase 23 additions, or add new messages entirely. Always check existing proto for used field numbers before adding.
**Warning signs:** Proto compilation warnings about duplicate field numbers.

### Pitfall 3: CLOD Scope Creep

**What goes wrong:** CLOD format becomes overly complex trying to capture every adapter difference.
**Why it happens:** Each adapter has unique features (Claude parameters vs OpenCode $ARGUMENTS vs Gemini {{args}} vs Copilot skills-only).
**How to avoid:** Define CLOD as a minimal specification of what a command does (name, params, process, output). Generation templates handle adapter-specific quirks. CLOD describes intent; templates produce files.
**Warning signs:** CLOD format has adapter-specific fields; conversion logic exceeds 200 lines per adapter.

### Pitfall 4: Backward Compatibility for Topic Agent Filtering

**What goes wrong:** Adding agent_filter to topic RPCs changes default behavior.
**Why it happens:** Empty string vs None semantics differ between proto3 optional and default empty.
**How to avoid:** Use `optional string agent_filter` (proto3 optional). When absent, return all topics (current behavior). When present, filter to matching agent contributions.
**Warning signs:** Existing topic queries return fewer results after Phase 23 changes.

### Pitfall 5: Documentation Staleness

**What goes wrong:** Documentation references outdated command syntax or configuration paths.
**Why it happens:** Commands evolved across Phases 19-22 but docs were not centralized.
**How to avoid:** Cross-reference all CLI command syntax against current `cli.rs`. Include version numbers in docs. Create a single source of truth for adapter comparison.
**Warning signs:** Users report commands that don't work as documented.

## Code Examples

Verified patterns from existing codebase:

### Adding a New CLI Subcommand (Pattern from Topics)

```rust
// In cli.rs - add to Commands enum following TopicsCommand pattern
/// Agent discovery commands
#[command(subcommand)]
Agents(AgentsCommand),

// In commands.rs - add handler following handle_topics_command pattern
pub async fn handle_agents_command(cmd: AgentsCommand) -> Result<()> {
    match cmd {
        AgentsCommand::List { addr } => agents_list(&addr).await,
        AgentsCommand::Activity { agent, from, to, bucket, addr } => {
            agents_activity(agent.as_deref(), from.as_deref(), to.as_deref(), &bucket, &addr).await
        },
        AgentsCommand::Topics { agent, limit, addr } => {
            agents_topics(&agent, limit, &addr).await
        },
    }
}

// In main.rs - add arm to match
Commands::Agents(cmd) => {
    handle_agents_command(cmd).await?;
}
```

### Aggregating Agents from TocNode (Source: memory-types/src/toc.rs)

```rust
// TocNode already has contributing_agents field
// To aggregate across all TOC nodes:
fn aggregate_agents(storage: &Storage) -> Result<Vec<AgentSummary>> {
    let mut agents: HashMap<String, AgentSummary> = HashMap::new();
    // Iterate TOC nodes (hundreds, not millions)
    for node in storage.iter_toc_nodes()? {
        for agent_id in &node.contributing_agents {
            let entry = agents.entry(agent_id.clone()).or_insert_with(|| {
                AgentSummary {
                    agent_id: agent_id.clone(),
                    first_seen_ms: node.start_time.timestamp_millis(),
                    last_seen_ms: node.end_time.timestamp_millis(),
                    ..Default::default()
                }
            });
            entry.first_seen_ms = entry.first_seen_ms.min(node.start_time.timestamp_millis());
            entry.last_seen_ms = entry.last_seen_ms.max(node.end_time.timestamp_millis());
        }
    }
    Ok(agents.into_values().collect())
}
```

### Adding Agent Filter to Topic Query (Source: proto/memory.proto pattern)

```protobuf
// Add to existing GetTopTopicsRequest:
message GetTopTopicsRequest {
    uint32 limit = 1;
    uint32 days = 2;
    // Phase 23: Filter topics by contributing agent
    optional string agent_filter = 3;
}
```

### CLOD CLI Subcommand Pattern

```rust
/// CLOD format commands
#[derive(Subcommand, Debug, Clone)]
pub enum ClodCommand {
    /// Convert CLOD definition to adapter-specific files
    Convert {
        /// Path to CLOD definition file
        #[arg(long)]
        input: String,
        /// Target adapter: claude, opencode, gemini, copilot, all
        #[arg(long)]
        target: String,
        /// Output directory
        #[arg(long)]
        out: String,
    },
    /// Validate a CLOD definition file
    Validate {
        /// Path to CLOD definition file
        input: String,
    },
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No agent field on events | Event.agent optional string | Phase 18 (v2.1) | Enables all cross-agent features |
| No agent filter on queries | --agent flag on teleport/retrieval | Phase 18/20 | Enables single-agent queries |
| Manual adapter copy | Skills portable across agents | Phase 19-22 | Skills are the shared format |
| No AGENTS.md standard | AGENTS.md emerging standard | 2025-2026 | External format for agent config; CLOD is internal equivalent |

**Deprecated/outdated:**
- None relevant. All existing code is current (Phase 22 just completed).

## Open Questions

1. **Agent Statistics: Scan vs. Metadata Store**
   - What we know: TocNode.contributing_agents gives us which agents exist and rough time ranges. For precise event counts, we need to scan events (bounded by time range).
   - What's unclear: Whether a dedicated agent metadata CF in RocksDB is worth the ingestion overhead for this phase.
   - Recommendation: Use TocNode aggregation for `agents list` (fast, approximate). Use time-bounded event scans for `agents activity` (exact but bounded). Defer dedicated metadata CF to a future phase if performance is insufficient.

2. **CLOD Format: Internal vs. External Standard**
   - What we know: No external "CLOD" standard exists. This is a project-internal format. AGENTS.md is an emerging external standard but covers agent config, not command definitions.
   - What's unclear: Whether CLOD should be a minimal internal tool or aim for broader adoption.
   - Recommendation: Design CLOD as a minimal, practical internal format. Keep it simple enough to potentially externalize later, but don't over-engineer for hypothetical community adoption. The R5.1 requirements say "optional."

3. **Topic-Agent Linking Granularity**
   - What we know: TopicLinks connect topics to TocNodes. TocNodes have contributing_agents. This gives us topic-to-agent linkage indirectly.
   - What's unclear: Whether we need direct topic-to-agent storage for performance.
   - Recommendation: Use the indirect path (topic -> TopicLink -> TocNode -> contributing_agents). This avoids new storage and works because the topic graph is typically small (<1000 topics). Add a direct index only if query latency exceeds 100ms.

4. **Storage API: iter_toc_nodes**
   - What we know: `Storage` has `get_events_in_range` but no public `iter_toc_nodes`. The `get_stats` method uses `count_cf_entries` which iterates all entries.
   - What's unclear: Whether a TOC node iterator already exists or needs to be added.
   - Recommendation: Add `iter_toc_nodes() -> Result<Vec<TocNode>>` to Storage if it doesn't exist. This is straightforward (iterate CF_TOC_NODES, deserialize each). Required for agent aggregation.

5. **Documentation Scope**
   - What we know: Four adapters exist with README.md each. No centralized cross-agent guide or authoring guide exists. `docs/adapters/` directory doesn't exist yet.
   - What's unclear: How much adapter internals to expose in the authoring guide vs. keeping it high-level.
   - Recommendation: Three docs: (1) cross-agent-guide.md covers end-user cross-agent queries, (2) authoring-guide.md covers the `AgentAdapter` trait + hook patterns + skill format for developers building new adapters, (3) clod-format.md defines the CLOD spec with examples. All three go in `docs/adapters/`.

## Sources

### Primary (HIGH confidence)

- **Codebase analysis** - proto/memory.proto (Event.agent field 8, query agent_filter fields), crates/memory-types (Event, TocNode with contributing_agents), crates/memory-adapters (AgentAdapter trait), crates/memory-daemon (cli.rs, commands.rs, main.rs), crates/memory-service (retrieval.rs, ingest.rs), crates/memory-topics (types.rs)
- **Existing plans** - .planning/phases/23-cross-agent-discovery/23-01-PLAN.md, 23-02-PLAN.md, 23-03-PLAN.md
- **Requirements** - .planning/REQUIREMENTS.md (R4.3.1-R4.3.3, R5.1.1-R5.1.3, R5.3.1-R5.3.3)
- **State** - .planning/STATE.md (Phase 22 complete, Phase 23 ready)
- **ROADMAP** - .planning/ROADMAP.md (Phase 23 definition, dependency graph)
- **Adapter plugins** - plugins/memory-query-plugin, plugins/memory-opencode-plugin, plugins/memory-gemini-adapter, plugins/memory-copilot-adapter (four complete adapter implementations)

### Secondary (MEDIUM confidence)

- **AGENTS.md standard** - [layer5.io blog](https://layer5.io/blog/ai/agentsmd-one-file-to-guide-them-all/) - Emerging standard for AI agent configuration files; relevant context for CLOD positioning but not directly applicable
- **Arxiv paper on AI coding agent config** - [arxiv.org/pdf/2511.09268](https://arxiv.org/pdf/2511.09268) - Research on configuration patterns across AI coding agents

### Tertiary (LOW confidence)

- **CLOD as a format name** - Web search returned no results for "CLOD format." This appears to be a project-internal term. The name is defined in the requirements but has no external specification. Treat as a greenfield design opportunity.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All libraries already in workspace
- Architecture: HIGH - Patterns directly derived from existing codebase (cli.rs, commands.rs, proto)
- Agent aggregation strategy: HIGH - Based on actual storage inspection (Event.agent, TocNode.contributing_agents, CF_EVENTS/CF_TOC_NODES)
- CLOD format design: MEDIUM - No external precedent; based on analysis of four adapter formats
- Documentation scope: HIGH - Based on inventory of existing adapter READMEs and missing centralized docs
- Pitfalls: HIGH - Based on actual code inspection (storage scan cost, proto field numbers, backward compat patterns)

**Plan alignment check:**
- **23-01-PLAN.md** (Agent insights RPC/CLI): Aligns well. Proto additions, storage aggregation, CLI commands all match research findings. One gap: plan mentions `ListAgents` returns session_count which requires event scan; recommend using TocNode aggregation instead for the default case.
- **23-02-PLAN.md** (Agent-aware topics): Aligns well. Adding agent filter to topic RPCs, CLI surfacing. Research confirms indirect path (topic -> TopicLink -> TocNode -> contributing_agents) is viable.
- **23-03-PLAN.md** (CLOD + docs): Aligns well. CLOD converter CLI, three documentation files. Research confirms CLOD is a greenfield design opportunity. Plan's `memory-daemon clod convert` command matches recommended architecture.

**Research date:** 2026-02-10
**Valid until:** 2026-03-10 (stable: this is internal code with known architecture)
