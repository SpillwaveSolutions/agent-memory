# Phase 52: Simple CLI API - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning
**Source:** PRD Express Path (docs/superpowers/specs/2026-03-21-v3-competitive-parity-design.md + docs/superpowers/plans/2026-03-21-v3-phase-b-simple-cli-api.md)

<domain>
## Phase Boundary

This phase creates a new `memory` binary with 6 structured-JSON commands (`add`, `search`, `context`, `timeline`, `summary`, `recall`) wired to the Phase 51 `memory-orchestrator`. The existing `memory-daemon` binary and all skill hooks remain unchanged. The binary is developer-facing — designed to be called from agent skills with zero context pollution.

</domain>

<decisions>
## Implementation Decisions

### Architecture
- New crate `crates/memory-cli/` with `[[bin]]` entry producing `memory` binary
- Each subcommand calls `MemoryOrchestrator` (Phase 51) via in-process call or `MemoryClient` gRPC for writes
- `memory-daemon` binary and existing skill hooks unchanged
- `memory recall` is a named alias for `memory search --rerank=llm --top=10` (same code path)

### Binary Strategy
- New `memory` binary — NOT renaming `memory-daemon`
- `memory-daemon` continues to serve daemon management commands
- `memory` binary exposes developer-facing API commands
- Existing skill hooks that call `memory-daemon` subcommands are unchanged

### JSON Envelope (output.rs)
- Every command returns consistent `JsonEnvelope`: status, query, results, context, error, meta
- `meta` includes `retrieval_ms`, `tokens_estimated`, `confidence`
- `--format=json` is default when stdout is not a TTY (piped); human-readable when interactive
- Uses `std::io::IsTerminal` for TTY detection (stable in Rust 1.70+, no `atty` dep needed)
- All commands exit 0 on success, non-zero on hard failure

### CLI Structs (cli.rs)
- `clap` derive API with `Cli`, `Commands` enum, `GlobalArgs`
- Global args: `--format`, `--endpoint` (gRPC endpoint, default `http://127.0.0.1:50051`)
- `SearchArgs`: query (positional), --top (10), --rerank (llm|heuristic), --format
- `RecallArgs`: query (positional), --format — delegates to SearchArgs internally
- `AddArgs`: --content, --kind (episodic default), --agent
- `ContextArgs`: query (positional), --format
- `TimelineArgs`: --entity, --range (7d default), --format
- `SummaryArgs`: --range (week default), --format

### Write Path (add command)
- `memory add` routes through `MemoryClient` over gRPC — daemon must be running
- If daemon not running, exits non-zero: `"memory daemon not running — start with: memory-daemon start"`
- `client.rs` wraps gRPC connection with clear error context

### Read Path (search/context/recall)
- search/context/recall call `MemoryOrchestrator.query()` from Phase 51
- Orchestrator runs in-process (not over gRPC) for read commands
- Need to construct orchestrator with real `LayerExecutor` — requires access to storage/indexes

### Claude's Discretion
- How to construct `MemoryOrchestrator` with real storage for read commands (may need gRPC client or direct storage access)
- Whether `timeline` and `summary` call orchestrator or directly query TOC gRPC RPCs
- Error handling strategy for partial failures (e.g., orchestrator returns results but with degraded indexes)
- Whether to add `--verbose` or `--debug` flag for tracing output

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Spec & Plans
- `docs/superpowers/specs/2026-03-21-v3-competitive-parity-design.md` — Full v3.0 design spec (Phase B section)
- `docs/superpowers/plans/2026-03-21-v3-phase-b-simple-cli-api.md` — Detailed implementation plan with code snippets

### Phase 51 Orchestrator (Dependency)
- `crates/memory-orchestrator/src/orchestrator.rs` — `MemoryOrchestrator<E: LayerExecutor>` with `query()` method
- `crates/memory-orchestrator/src/types.rs` — `OrchestratorConfig`, `MemoryContext`, `RankedResult`, `RerankMode`
- `crates/memory-orchestrator/src/lib.rs` — Public API re-exports

### Existing gRPC Client
- `crates/memory-client/src/lib.rs` — `MemoryClient` for gRPC communication with daemon
- `proto/memory.proto` — gRPC service definition (IngestEvent, GetTocRoot, GetNode, etc.)

### Existing Daemon Binary
- `crates/memory-daemon/src/main.rs` — DO NOT MODIFY; existing daemon management commands
- `crates/memory-service/src/handlers/` — gRPC handler implementations

### Existing Retrieval
- `crates/memory-retrieval/src/executor.rs` — `LayerExecutor` trait needed for orchestrator construction

</canonical_refs>

<specifics>
## Specific Ideas

- The implementation plan has 5 tasks with complete Rust code snippets for all structs and command handlers
- `JsonEnvelope` pattern specified with `ok()` and `error()` constructors
- TTY detection uses `std::io::IsTerminal` (stable Rust, no external dep)
- `memory recall` implementation is trivial — constructs `SearchArgs` with rerank=llm, top=10 and delegates
- Integration smoke test defined: start daemon → add event → search → verify JSON envelope

</specifics>

<deferred>
## Deferred Ideas

- REST/HTTP endpoint (CLI-F01) — future milestone
- Python SDK (CLI-F02) — wraps CLI binary, future milestone
- `--verbose` / `--debug` tracing flags — nice to have, not required
- Updated canonical plugin source to reference `memory` binary in new hooks — future integration task

</deferred>

---

*Phase: 52-simple-cli-api*
*Context gathered: 2026-03-22 via PRD Express Path*
