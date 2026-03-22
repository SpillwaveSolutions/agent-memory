---
phase: 52-simple-cli-api
verified: 2026-03-22T06:00:00Z
status: passed
score: 15/15 must-haves verified
re_verification: false
---

# Phase 52: Simple CLI API Verification Report

**Phase Goal:** Users can interact with Agent Memory through a single `memory` binary that provides search, context injection, recall, add, timeline, and summary — with sensible defaults and TTY-aware output
**Verified:** 2026-03-22T06:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | memory binary exists and parses all 6 subcommands via --help | VERIFIED | Binary at target/debug/memory shows: search, context, add, timeline, summary, recall |
| 2 | JsonEnvelope serializes with status, query, results, context, error, meta fields | VERIFIED | output.rs lines 8-19; skip_serializing_if on all Option fields; 14 unit tests passing |
| 3 | TTY detection returns JSON when piped, human-readable when interactive | VERIFIED | output.rs line 99: `std::io::stdout().is_terminal()` gating two code paths |
| 4 | MemoryClient has route_query() method for orchestrated search | VERIFIED | memory-client/src/client.rs lines 297-314; RouteQueryResponse re-exported from lib.rs:47 |
| 5 | connect_client() returns actionable error when daemon is not running | VERIFIED | client.rs line 11: `.context("memory daemon not running -- start with: memory-daemon start (endpoint: {endpoint})")` |
| 6 | memory search 'query' returns ranked results via gRPC RouteQuery RPC | VERIFIED | search.rs lines 11-22: connect_client -> route_query -> build_results_json -> print_output |
| 7 | memory search --format=json produces JSON envelope with results, meta.retrieval_ms, meta.tokens_estimated, meta.confidence | VERIFIED | search.rs build_meta() extracts total_time_ms, sums estimate_tokens, uses first result score |
| 8 | memory recall delegates to search with rerank=llm and top=10 | VERIFIED | recall.rs lines 10-16: constructs SearchArgs{top:10, rerank:Some("llm")} and calls search::run |
| 9 | memory context returns structured context with summary, relevant_events, key_entities | VERIFIED | context.rs lines 25-33: json! with summary, relevant_events, key_entities, open_questions |
| 10 | memory add writes an event via gRPC ingest RPC | VERIFIED | add.rs lines 52-88: connect_client -> build_event -> client.ingest -> envelope |
| 11 | memory add exits non-zero with 'memory daemon not running' when daemon is down | VERIFIED | add.rs lines 52-59: match on connect_client error, print_output(error envelope), process::exit(1) |
| 12 | memory timeline returns events in a time range via get_events RPC | VERIFIED | timeline.rs lines 77-123: parse_range -> client.get_events(from_ms, to_ms, 100) |
| 13 | memory summary returns TOC summaries via get_toc_root and browse_toc RPCs | VERIFIED | summary.rs lines 56-121: get_toc_root -> node_overlaps filter -> browse_toc children |
| 14 | All commands exit 0 on success, non-zero on hard failure | VERIFIED | main.rs line 35-38: Err -> eprintln + process::exit(1); add/timeline/summary also exit(1) in error branches |
| 15 | meta.tokens_estimated present in all JSON envelopes | VERIFIED | search: build_meta sums estimate_tokens; add: estimate_tokens(content); timeline: sums per event; summary: sums per summary text |

**Score:** 15/15 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/memory-cli/Cargo.toml` | Crate manifest with `[[bin]] name = "memory"` | VERIFIED | Line 9: `name = "memory"`; memory-cli in workspace Cargo.toml line 22 |
| `crates/memory-cli/src/cli.rs` | Clap derive structs for all 6 subcommands | VERIFIED | 286 lines; Commands enum with Search/Context/Add/Timeline/Summary/Recall; 11 unit tests |
| `crates/memory-cli/src/output.rs` | JsonEnvelope, Meta, print_output with TTY detection | VERIFIED | 266 lines; all constructors, TTY detection via IsTerminal, 14 unit tests |
| `crates/memory-cli/src/client.rs` | connect_client helper with actionable error | VERIFIED | 14 lines; MemoryClient::connect + .context() with "daemon not running" message |
| `crates/memory-client/src/client.rs` | route_query() method on MemoryClient | VERIFIED | Lines 297-314; follows hybrid_search pattern; RouteQueryRequest/Response used correctly |
| `crates/memory-cli/src/commands/search.rs` | Search command calling RouteQuery RPC | VERIFIED | 193 lines; route_query, map_retrieval_result, build_results_json, build_meta; 6 unit tests |
| `crates/memory-cli/src/commands/recall.rs` | Recall command delegating to search with llm rerank | VERIFIED | 63 lines; constructs SearchArgs{rerank:"llm", top:10}, calls search::run; 2 unit tests |
| `crates/memory-cli/src/commands/context.rs` | Context command returning structured MemoryContext-style output | VERIFIED | 127 lines; route_query -> context_ok with summary/relevant_events/key_entities; 2 unit tests |
| `crates/memory-cli/src/commands/add.rs` | Add command writing events via gRPC ingest | VERIFIED | 184 lines; build_event, kind_to_event_type, ULID IDs, exit(1) on failure; 9 unit tests |
| `crates/memory-cli/src/commands/timeline.rs` | Timeline command querying events by time range | VERIFIED | 213 lines; parse_range, map_proto_event, get_events RPC, entity filter; 7 unit tests |
| `crates/memory-cli/src/commands/summary.rs` | Summary command querying TOC hierarchy | VERIFIED | 209 lines; parse_summary_range, node_overlaps, get_toc_root + browse_toc; 8 unit tests |
| `crates/memory-cli/src/commands/mod.rs` | Module declarations for all 6 commands | VERIFIED | 6 pub mod declarations, all present |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `output.rs` | `std::io::IsTerminal` | TTY detection in print_output | WIRED | Line 4 import; line 99 `stdout().is_terminal()` branches correctly |
| `client.rs` | `memory_client::MemoryClient` | gRPC connection wrapper | WIRED | Line 4: `use memory_client::MemoryClient`; line 11: `MemoryClient::connect(endpoint).await` |
| `commands/search.rs` | `crates/memory-client/src/client.rs` | connect_client() -> client.route_query() | WIRED | Line 12: `crate::client::connect_client`; line 13-15: `client.route_query(...)` |
| `commands/recall.rs` | `crates/memory-cli/src/commands/search.rs` | delegates to search::run with llm rerank args | WIRED | Line 6: `use crate::commands::search`; line 16: `search::run(search_args, global).await` |
| `commands/context.rs` | `crates/memory-client/src/client.rs` | connect_client() -> client.route_query() | WIRED | Line 12: connect_client; line 13: `client.route_query(&args.query, 10, None)` |
| `commands/add.rs` | `crates/memory-client/src/client.rs` | connect_client() -> client.ingest() | WIRED | Lines 52-65: connect_client then `client.ingest(event).await` |
| `commands/timeline.rs` | `crates/memory-client/src/client.rs` | connect_client() -> client.get_events() | WIRED | Lines 80-91: connect_client then `client.get_events(from_ms, to_ms, 100).await` |
| `commands/summary.rs` | `crates/memory-client/src/client.rs` | connect_client() -> client.get_toc_root() + browse_toc() | WIRED | Lines 59-80: connect_client; line 70: get_toc_root; line 80: browse_toc |

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| CLI-01 | 52-01 | New `memory` binary with search, context, recall, add, timeline, summary subcommands | SATISFIED | Binary confirmed; --help shows all 6 subcommands; 11 CLI parse tests pass |
| CLI-02 | 52-02 | `memory search --format=json` returns JSON envelope with results, meta, confidence | SATISFIED | search.rs: route_query -> build_results_json + build_meta -> JsonEnvelope::ok |
| CLI-03 | 52-02 | `memory recall` delegates to search with `--rerank=llm --top=10` | SATISFIED | recall.rs: SearchArgs{top:10, rerank:Some("llm")} -> search::run |
| CLI-04 | 52-03 | `memory add` writes via gRPC MemoryClient, exits non-zero with clear error if daemon not running | SATISFIED | add.rs: connect_client error -> JsonEnvelope::error + process::exit(1) |
| CLI-05 | 52-01 | TTY detection: JSON when piped, human-readable when interactive | SATISFIED | output.rs: IsTerminal import + is_terminal() in print_output |
| CLI-06 | 52-02 | `memory context` returns structured context for prompt injection | SATISFIED | context.rs: summary/relevant_events/key_entities/open_questions JSON shape |
| CLI-07 | 52-03 | `memory timeline` and `memory summary` query TOC by entity/range | SATISFIED | timeline.rs: get_events with parse_range; summary.rs: get_toc_root + browse_toc |
| CLI-08 | 52-02, 52-03 | `memory-daemon` binary and existing skill hooks unchanged | SATISFIED | `git diff --name-only crates/memory-daemon/` returned empty output |
| CLI-09 | 52-01, 52-03 | All commands exit 0 on success, non-zero on hard failure | SATISFIED | main.rs exit(1) on Err; add/timeline/summary also call process::exit(1) in error branches |
| CLI-10 | 52-01, 52-03 | `meta.tokens_estimated` included in JSON envelope for context budget decisions | SATISFIED | All 4 write/query commands compute tokens_estimated via estimate_tokens(); search uses build_meta sum |

### Anti-Patterns Found

No anti-patterns detected.

- No `todo!()` or `unimplemented!()` macros found in any command file
- No placeholder return values (`return null`, `return {}`, `return []` without logic)
- No empty handlers or stub implementations
- All 6 commands have substantive implementations with real gRPC calls
- Clippy passes clean with `-D warnings` on memory-cli crate

### Human Verification Required

#### 1. TTY-aware Human-Readable Output Visual Check

**Test:** Run `memory search "test" --endpoint http://127.0.0.1:50051` in an interactive terminal with daemon running
**Expected:** Displays "Query: test" header, pretty-printed JSON results, and "(X ms, ~Y tokens, confidence: Z.ZZ)" footer
**Why human:** TTY detection behavior cannot be verified programmatically; requires an interactive terminal session with a live daemon

#### 2. Pipe Mode Produces Compact JSON

**Test:** Run `memory search "test" | cat` with daemon running
**Expected:** Single-line compact JSON on stdout (no human-readable decoration)
**Why human:** Requires live daemon and a real pipe to verify IsTerminal returns false correctly

#### 3. Non-zero Exit on Daemon Down

**Test:** Run `memory add --content "test"` when no daemon is running; check `echo $?`
**Expected:** Exit code 1, stderr/stdout shows JSON error envelope with "daemon not running" message
**Why human:** Cannot start/stop daemon in test environment to verify actual exit code behavior end-to-end

## Summary

Phase 52 goal is fully achieved. The `memory` binary provides all 6 subcommands (search, context, recall, add, timeline, summary) with complete implementations:

- **CLI scaffold** (Plan 01): 286-line cli.rs with clap derive for all 6 subcommands, 266-line output.rs with JsonEnvelope/TTY detection, connect_client helper, route_query() added to MemoryClient. 25 unit tests.
- **Read commands** (Plan 02): search.rs (RouteQuery -> ranked results with meta), context.rs (MemoryContext-shaped JSON), recall.rs (delegates to search with rerank=llm). 12 unit tests.
- **Write/query commands** (Plan 03): add.rs (ULID event creation via ingest RPC), timeline.rs (get_events with time-range parsing), summary.rs (get_toc_root + browse_toc with overlap filtering). 25 unit tests.

All 62 unit tests pass. Clippy is clean with -D warnings. The memory-daemon crate was not modified (CLI-08 confirmed). All 10 requirement IDs (CLI-01 through CLI-10) are satisfied with code evidence.

Three items flagged for human verification are behavioral checks that require a live daemon and an interactive TTY — they are not gaps in implementation.

---

_Verified: 2026-03-22T06:00:00Z_
_Verifier: Claude (gsd-verifier)_
