# Feature Landscape: Conversational Memory System for AI Agents

**Domain:** Conversational Memory for AI Coding Agents (Claude Code, OpenCode, Gemini CLI)
**Researched:** 2026-01-29
**Overall Confidence:** MEDIUM-HIGH (verified against current memory system research and production systems)

## Executive Summary

This document maps the feature landscape for building a conversational memory system with the following characteristics:
- Append-only conversation history storage
- TOC-based navigation (Year/Month/Week/Day/Segment hierarchy)
- Grips for provenance (excerpt + event pointer)
- Teleports for index-based acceleration
- Time-based queries ("what were we talking about last week?")
- Hook-based passive capture (zero token overhead)

The analysis compares against existing systems (Letta/MemGPT, Mem0, LangGraph, Graphiti/Zep) to identify table stakes, differentiators, and anti-features.

---

## Table Stakes

Features users/agents expect. Missing = product feels incomplete or unusable.

| Feature | Why Expected | Complexity | Dependencies | Notes |
|---------|--------------|------------|--------------|-------|
| **Persistent storage across sessions** | Agents like Claude Code already have session resume; without cross-session persistence, the system adds no value | Low | Storage backend | Baseline requirement - every competitor has this |
| **Conversation history append** | Core use case; must capture full conversation including tool calls and results | Low | None | JSONL format common (Claude Code uses .jsonl in ~/.claude/projects/) |
| **Basic retrieval by time** | Users ask "what did we discuss yesterday?" - this is the primary navigation axis | Medium | Time indexing | Most systems support timestamps but not as primary navigation |
| **Full-text search** | Standard expectation for any searchable system | Medium | Search index | Letta, Mem0, LangGraph all provide this |
| **User/agent scoping** | Memory must be partitioned per-user or per-agent; multi-tenancy is expected | Low | Identity model | Mem0 has user_id, session_id, agent_id scopes |
| **Read/query API** | Programmatic access to stored memories | Low | None | REST or tool-based access |
| **Write/ingest API** | Programmatic way to store memories | Low | None | Hook integration point |
| **Session context continuity** | Resume mid-conversation with full context | Medium | State management | Claude Code has --resume; LangGraph has checkpointers |
| **Configurable retention** | Ability to set retention policies (30 days, 90 days, forever) | Low | Lifecycle management | Claude Code deletes after 1 month by default |
| **Privacy controls** | User can view, export, and delete their data | Medium | Identity, storage | GDPR/compliance requirement |

**Minimum Viable Product must include:** Persistent storage, append API, time-based retrieval, and basic search.

---

## Differentiators

Features that set this system apart. Not expected, but create competitive advantage.

### Tier 1: Core Differentiators (Unique to This System)

| Feature | Value Proposition | Complexity | Dependencies | Comparison to Existing |
|---------|-------------------|------------|--------------|------------------------|
| **TOC hierarchy navigation (Year/Month/Week/Day/Segment)** | Deterministic navigation without LLM inference; agents can "drill down" like a file browser | Medium | Index structure | **Unique**: No existing system uses TOC-based navigation. Mem0/Letta rely on vector search. Graphiti uses graph traversal. |
| **Grips (excerpt + event pointer)** | Provenance tracking with verifiable citations; agents can prove "where did I learn this?" | Medium | Event indexing | **Unique**: PROV-AGENT paper addresses provenance but not with excerpts. Vertex AI has grounding but for search, not memory. |
| **Teleports (index-based jumps)** | O(1) access to specific points in history; no scan required | Low-Medium | Pointer system | **Unique**: Vector DBs use ANN (approximate); graphs use traversal. Direct indexing is novel. |
| **Hook-based passive capture** | Zero token overhead during conversation; memory happens asynchronously | Medium | CLI integration | **Unique**: Most systems require explicit memory operations (tool calls) that consume tokens. |
| **Time as primary axis** | Optimized for "last week" / "yesterday" queries that current systems handle poorly | Medium | Temporal indexing | **Differentiated**: TSM paper shows 22.56% improvement over dialogue-time approaches. Most systems treat time as metadata, not navigation. |

### Tier 2: Competitive Differentiators (Better than Existing)

| Feature | Value Proposition | Complexity | Dependencies | Comparison to Existing |
|---------|-------------------|------------|--------------|------------------------|
| **Append-only immutability** | Full audit trail; no data loss; conflict-free replication possible | Low | Storage design | Letta supports updates; Mem0 merges facts. Append-only is simpler and more auditable. |
| **Controlled heavy scan as fallback** | When TOC/teleports fail, explicit full-scan with user consent | Medium | Scan limiter | Most systems silently degrade; explicit fallback is more transparent. |
| **Event-centric vs fact-centric** | Stores conversations as events (who said what when) not extracted facts | Low | Data model | Mem0 extracts atomic facts; Letta summarizes. Event-centric preserves context and nuance. |
| **Multi-agent conversation support** | Track which agent said what in multi-agent workflows | Medium | Agent identity | Letta supports multi-agent with shared blocks; this would track full provenance. |
| **Segment-level granularity** | Subdivide days into logical conversation segments (morning, afternoon, by topic) | Medium | Segmentation logic | No existing system has sub-day granularity beyond session IDs. |

### Tier 3: Nice-to-Have Differentiators (Future Phases)

| Feature | Value Proposition | Complexity | Dependencies | Notes |
|---------|-------------------|------------|--------------|-------|
| **Cross-project memory sharing** | "Did I solve this in another project?" | High | Project model, privacy | Interesting but scope creep |
| **Semantic clustering of segments** | Auto-group related conversations | High | ML model | Could layer on later |
| **Memory decay/importance scoring** | Surface frequently-accessed memories | Medium | Usage tracking | Letta has sleep-time agents for this |
| **Compression/summarization** | Reduce storage for old segments | Medium | LLM integration | Adds token cost; conflicts with append-only |

---

## Anti-Features

Features to explicitly NOT build. Common mistakes in this domain.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Vector search as primary retrieval** | Semantic similarity fails for temporal queries ("yesterday" doesn't embed well); 5-18% worse than structured approaches per research | Use TOC navigation + teleports; offer semantic search as optional enhancement |
| **Automatic fact extraction** | LLM extracts facts = token cost + hallucination risk + lost context | Store raw events; let querying agent extract meaning at query time |
| **Self-modifying memory** | Memory that edits itself is unpredictable; leads to ZombieAgent-style attacks | Append-only; deletions are tombstones if needed |
| **Always-on context injection** | Injecting memories into every prompt wastes tokens and may inject irrelevant info | On-demand retrieval; agent asks when needed |
| **Complex graph relationships** | Knowledge graphs require schema design, maintenance, and add query complexity | Simple parent-child hierarchy (Year > Month > Week > Day > Segment) |
| **Real-time synchronization** | Eventual consistency is fine for memory; real-time adds latency and complexity | Async append; reads see committed state |
| **LLM-in-the-loop for storage** | Using LLM to decide what to store adds token overhead and latency | Rule-based capture via hooks; store everything |
| **Embedding-only storage** | Losing original text makes debugging impossible | Store original text; generate embeddings optionally as secondary index |
| **Global memory sharing** | Privacy nightmare; mixing users' memories | Strict tenant isolation; sharing must be explicit |
| **Heartbeat/continuous reasoning** | MemGPT's heartbeat pattern consumes tokens during idle time | Only process during explicit queries |

---

## Feature Dependencies

```
                    ┌─────────────────┐
                    │ Storage Backend │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              v              v              v
        ┌─────────┐   ┌───────────┐   ┌──────────┐
        │ Append  │   │ Indexing  │   │ Identity │
        │   API   │   │  System   │   │  Model   │
        └────┬────┘   └─────┬─────┘   └────┬─────┘
             │              │              │
             │         ┌────┴────┐         │
             │         │         │         │
             v         v         v         v
        ┌─────────┐ ┌──────┐ ┌───────┐ ┌───────┐
        │  Hooks  │ │ TOC  │ │Teleport│ │Scoping│
        └────┬────┘ │ Hier │ │ Index │ └───┬───┘
             │      └──┬───┘ └───┬───┘     │
             │         │         │         │
             └─────────┼─────────┼─────────┘
                       │         │
                       v         v
                 ┌──────────────────┐
                 │  Query Engine    │
                 │  (TOC + Search)  │
                 └────────┬─────────┘
                          │
                          v
                 ┌──────────────────┐
                 │     Grips        │
                 │   (Provenance)   │
                 └──────────────────┘
```

**Critical Path:**
1. Storage Backend (prerequisite for everything)
2. Append API + Identity Model (enable basic writes)
3. Indexing System (enable TOC hierarchy)
4. Query Engine (enable reads)
5. Hooks (enable passive capture)
6. Grips (enable provenance)

---

## Comparison to Existing Memory Systems

### Letta (formerly MemGPT)

| Capability | Letta | This System | Notes |
|------------|-------|-------------|-------|
| Storage | Vector DB (Chroma, pgvector) + Memory Blocks | Append-only event log | Letta summarizes; we preserve raw events |
| Navigation | Semantic search + conversation search | TOC hierarchy + teleports | We offer deterministic navigation |
| Temporal queries | Limited (timestamp metadata) | Primary axis | Key differentiator |
| Provenance | None explicit | Grips | Key differentiator |
| Token overhead | High (heartbeats, tool calls for memory) | Zero (hooks) | Key differentiator |
| Multi-agent | Shared memory blocks | Per-agent scopes + cross-reference | Similar capability |
| Maturity | Production (2+ years) | Greenfield | Letta has ecosystem |

**Verdict:** Letta is feature-rich but token-expensive. Our system trades sophistication for efficiency.

### Mem0

| Capability | Mem0 | This System | Notes |
|------------|------|-------------|-------|
| Storage | Vector + Graph + Key-Value hybrid | Append-only event log | Mem0 extracts facts; we store events |
| Navigation | Semantic search + graph traversal | TOC hierarchy | Different paradigms |
| Temporal queries | Supports but not primary | Primary axis | We optimize for this |
| Provenance | Entity linking | Grips (excerpt pointers) | Both support but differently |
| Token overhead | Moderate (extraction cost) | Zero (hooks) | Key differentiator |
| Graph features | Entity relationships | None (explicit anti-feature) | Simpler is better for our use case |

**Verdict:** Mem0 is more sophisticated for relationship tracking. We're better for "when did we discuss X?"

### Graphiti/Zep

| Capability | Graphiti | This System | Notes |
|------------|----------|-------------|-------|
| Storage | Neo4j/FalkorDB temporal graph | Append-only event log | Different paradigms |
| Navigation | Graph traversal + hybrid search | TOC hierarchy | Graphiti requires graph queries |
| Temporal queries | Bi-temporal model (excellent) | Time hierarchy (simpler) | Both strong; Graphiti more sophisticated |
| Provenance | Timestamp tracking | Grips (richer) | We have excerpt-level provenance |
| Token overhead | Low (no LLM for storage) | Zero (hooks) | Both efficient |
| Complexity | High (graph schema design) | Low (hierarchy is fixed) | Key differentiator |

**Verdict:** Graphiti is technically impressive but operationally complex. Our system is simpler to deploy and reason about.

### LangGraph Memory

| Capability | LangGraph | This System | Notes |
|------------|-----------|-------------|-------|
| Storage | Checkpointers (SQLite, Postgres) | Append-only event log | LangGraph stores state; we store events |
| Navigation | Thread-based retrieval | TOC hierarchy | Different scoping |
| Temporal queries | Weak (session-based) | Strong (primary axis) | Key differentiator |
| Provenance | None | Grips | Key differentiator |
| Integration | LangChain ecosystem | Standalone + hooks | LangGraph requires LangChain buy-in |

**Verdict:** LangGraph is for LangChain users. Our system is agent-agnostic.

---

## MVP Recommendation

### Phase 1: Foundation (Must Have)

1. **Append-only storage backend** (table stakes)
   - JSONL files or SQLite
   - User/agent scoping
   - Retention policies

2. **TOC hierarchy indexing** (core differentiator)
   - Year/Month/Week/Day/Segment structure
   - Fast navigation API

3. **Basic query engine** (table stakes)
   - Navigate by TOC
   - Full-text search within scope

4. **Hook integration for Claude Code** (core differentiator)
   - Passive capture of conversations
   - Zero token overhead

### Phase 2: Enhanced Retrieval (Should Have)

5. **Teleports** (differentiator)
   - Direct pointers to specific events
   - O(1) lookup

6. **Grips** (differentiator)
   - Excerpt + event pointer
   - Provenance for agent responses

7. **Time-based query DSL** (differentiator)
   - "last week", "yesterday", "Tuesday morning"
   - Relative and absolute time support

### Phase 3: Polish (Nice to Have)

8. **Multi-agent support**
9. **Cross-session context handoff**
10. **Optional semantic search enhancement**

### Defer to Post-MVP

- Knowledge graph relationships (anti-feature for this use case)
- Automatic summarization (adds token cost)
- Real-time sync (unnecessary complexity)
- Cross-project memory sharing (privacy concerns)

---

## Sources

### Research Papers
- [Memory in the Age of AI Agents (arXiv:2512.13564)](https://arxiv.org/abs/2512.13564) - Survey of agent memory systems
- [Agentic Memory (arXiv:2601.01885)](https://arxiv.org/pdf/2601.01885) - Unified LTM/STM management
- [Mem0 Paper (arXiv:2504.19413)](https://arxiv.org/pdf/2504.19413) - Production memory architecture
- [Temporal Semantic Memory (arXiv:2601.07468)](https://arxiv.org/html/2601.07468v1) - 22.56% improvement in temporal accuracy
- [PROV-AGENT (arXiv:2508.02866)](https://arxiv.org/abs/2508.02866) - Provenance for agent interactions
- [Zep/Graphiti (arXiv:2501.13956)](https://arxiv.org/abs/2501.13956) - Temporal knowledge graph architecture

### Production Systems
- [Letta Documentation](https://docs.letta.com/concepts/memgpt/) - MemGPT concepts and memory architecture
- [Mem0 Platform](https://mem0.ai/) - Universal memory layer
- [LangGraph Memory](https://docs.langchain.com/oss/python/langgraph/memory) - Checkpointer-based persistence
- [Graphiti GitHub](https://github.com/getzep/graphiti) - Temporal knowledge graphs

### Claude Code Memory
- [Claude Code Memory Docs](https://code.claude.com/docs/en/memory) - CLAUDE.md hierarchy
- [Claude Memory Tool API](https://platform.claude.com/docs/en/agents-and-tools/tool-use/memory-tool) - Beta memory API

### Architecture
- [Survey of AI Agent Memory Frameworks](https://www.graphlit.com/blog/survey-of-ai-agent-memory-frameworks) - Comparison of approaches
- [AI Memory Layer Guide](https://mem0.ai/blog/ai-memory-layer-guide) - Implementation patterns
- [Building Smarter AI Agents (AWS)](https://aws.amazon.com/blogs/machine-learning/building-smarter-ai-agents-agentcore-long-term-memory-deep-dive/) - Production considerations

---

## Confidence Assessment

| Area | Confidence | Reason |
|------|------------|--------|
| Table Stakes | HIGH | Verified against multiple production systems (Letta, Mem0, LangGraph) |
| Differentiators | MEDIUM-HIGH | TOC/Grips/Teleports are novel; validated that no existing system uses this approach |
| Anti-Features | HIGH | Clear research evidence on vector search limitations and token overhead concerns |
| Comparisons | MEDIUM | Based on documentation and papers; no hands-on testing of competitors |
| MVP Recommendation | MEDIUM | Logical sequencing but may need adjustment based on implementation complexity |

---

## Open Questions for Later Research

1. **Segment boundary detection**: How to automatically identify conversation segment breaks within a day?
2. **Hook implementation details**: What's the exact integration point for Claude Code, OpenCode, Gemini CLI?
3. **Storage scaling**: What happens at 1M+ events? Need to validate indexing performance.
4. **Cross-agent queries**: How should "did any agent discuss X?" work across tenant boundaries?
5. **Conflict resolution**: If same event captured twice (redundant hooks), how to deduplicate?
