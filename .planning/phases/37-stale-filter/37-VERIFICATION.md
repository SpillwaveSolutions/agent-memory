---
phase: 37-stale-filter
verified: 2026-03-09T21:38:56Z
status: passed
score: 10/10 must-haves verified
re_verification:
  previous_status: gaps_found
  previous_score: 9/10
  gaps_closed:
    - "StalenessConfig loaded from config.toml is propagated to RetrievalHandler at daemon startup"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Verify time-decay is applied to live query results"
    expected: "A query returning results with different timestamps should produce lower scores for older results than for newer ones of similar initial score"
    why_human: "Requires a running daemon with indexed data across a time range; not reproducible in unit tests without real storage and timing"
  - test: "Verify supersession marks duplicate content as superseded"
    expected: "Two ingested events with near-identical text and timestamps should result in the older one having superseded_by metadata on query"
    why_human: "Requires real embedding model and populated HNSW index producing >0.80 cosine similarity"
---

# Phase 37: Stale Filter Verification Report

**Phase Goal:** Agents get fresher, more relevant results because outdated and superseded content is downranked at query time
**Verified:** 2026-03-09T21:38:56Z
**Status:** passed
**Re-verification:** Yes — after gap closure plan 37-03

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | Query results with timestamps are downranked via exponential time-decay relative to the newest result | VERIFIED | `StaleFilter.apply_time_decay()` formula `score * (1.0 - max_penalty * (1.0 - exp(-age_days / half_life)))` in stale_filter.rs:95-140. Unit tests at 14/28/42 days confirm decay percentages. |
| 2 | High-salience memory kinds (Constraint, Definition, Procedure, Preference) are exempt from decay | VERIFIED | `is_exempt()` at stale_filter.rs:223-228. `test_kind_exemption` and `test_is_exempt_case_insensitive` confirm all four kinds case-insensitively. |
| 3 | Time-decay half-life is configurable via StalenessConfig with a default of 14 days | VERIFIED | `StalenessConfig` in config.rs:99-155 has `half_life_days: f32` with `default_half_life_days() = 14.0`. Settings.staleness wired through to daemon startup. |
| 4 | Max stale penalty is bounded at 30% (asymptotic, never reached) | VERIFIED | `test_max_penalty_bounded` confirms 365-day result penalty is < 0.30 and > 0.29. Default `max_penalty = 0.30` in config.rs:133. |
| 5 | Results without timestamps receive no penalty (fail-open) | VERIFIED | `apply_time_decay()` returns `r` unchanged when `timestamp_ms` metadata absent. `test_no_timestamps_no_penalty` confirms. |
| 6 | SimpleLayerExecutor enriches SearchResult.metadata with timestamp_ms and memory_kind | VERIFIED | `build_metadata()` helper at retrieval.rs:614-628 populates both fields. All four retrieval layers call it. |
| 7 | StaleFilter is applied post-merge in RetrievalHandler.route_query | VERIFIED | retrieval.rs:276-287: `StaleFilter::new(self.staleness_config.clone())` applied after `retrieval_executor.execute()`, guarded by `self.staleness_config.enabled`. |
| 8 | Older results semantically similar (>=0.80) to newer results are marked superseded with 15% penalty | VERIFIED | `apply_supersession()` in stale_filter.rs:148-218. Threshold from `config.supersession_threshold`, penalty factor `1.0 - config.supersession_penalty`. |
| 9 | Supersession only applies to Observation kind; each result superseded at most once | VERIFIED | `is_exempt()` check at stale_filter.rs:185-187 gates supersession. `break` at stale_filter.rs:214 prevents transitivity. |
| 10 | StalenessConfig loaded from config.toml is propagated to RetrievalHandler at daemon startup | VERIFIED | `start_daemon` passes `settings.staleness.clone()` (commands.rs:510) to `run_server_with_scheduler`, which forwards it (server.rs:152) to `MemoryServiceImpl::with_scheduler`, which passes it to `RetrievalHandler::with_services` (ingest.rs:92-97). Zero `StalenessConfig::default()` calls remain in production paths. |

**Score:** 10/10 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/memory-types/src/config.rs` | StalenessConfig struct with serde defaults, added to Settings | VERIFIED | Lines 99-155: struct with 5 fields, all with serde defaults. Line 284: `pub staleness: StalenessConfig` in Settings. Default 14d half-life, 0.30 max_penalty. |
| `crates/memory-retrieval/src/stale_filter.rs` | StaleFilter with time-decay, kind exemption, apply_with_supersession | VERIFIED | All required methods present. 18+ unit tests confirmed in initial verification. |
| `crates/memory-service/src/retrieval.rs` | StaleFilter wired into route_query with config-derived StalenessConfig | VERIFIED | Lines 276-287: StaleFilter created from `self.staleness_config`, applied post-merge with supersession support. |
| `crates/memory-service/src/server.rs` | run_server_with_scheduler accepts StalenessConfig parameter | VERIFIED | Lines 116-123: function signature includes `staleness_config: StalenessConfig`. Line 152: forwarded to `MemoryServiceImpl::with_scheduler`. Import at line 17. |
| `crates/memory-service/src/ingest.rs` | All with_* constructors accept StalenessConfig, no default hardcoding | VERIFIED | 7 constructors (with_scheduler, with_scheduler_and_search, with_search, with_vector, with_topics, with_all_services, with_all_services_and_topics) each accept `staleness_config: StalenessConfig` and pass it to `RetrievalHandler::with_services`. Zero `StalenessConfig::default()` in production code paths. |
| `crates/memory-daemon/src/commands.rs` | start_daemon passes settings.staleness to run_server_with_scheduler | VERIFIED | Line 500: info log records enabled/half_life/max_penalty. Line 510: `settings.staleness.clone()` passed as final argument to `run_server_with_scheduler`. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `stale_filter.rs` | `config.rs` | `use memory_types::config::StalenessConfig` | WIRED | Confirmed in initial verification (line 17 of stale_filter.rs). No regression. |
| `retrieval.rs` | `stale_filter.rs` | `stale_filter.apply_with_supersession` call | WIRED | retrieval.rs:284 calls `stale_filter.apply_with_supersession(result.results, embeddings.as_ref())` |
| `commands.rs` | `server.rs` | `settings.staleness.clone()` passed to `run_server_with_scheduler` | WIRED | commands.rs:504-511 — `run_server_with_scheduler(..., settings.staleness.clone())`. Was NOT WIRED in previous verification; now closed. |
| `server.rs` | `ingest.rs` | `staleness_config` forwarded to `MemoryServiceImpl::with_scheduler` | WIRED | server.rs:152: `MemoryServiceImpl::with_scheduler(storage, scheduler.clone(), staleness_config)` |
| `ingest.rs` | `retrieval.rs` | `with_scheduler` creates `RetrievalHandler::with_services(... staleness_config)` | WIRED | ingest.rs:92-97: `RetrievalHandler::with_services(storage.clone(), None, None, None, staleness_config)` — was using `RetrievalHandler::new` before; now uses `with_services`. |

### Requirements Coverage

| Requirement | Status | Blocking Issue |
|-------------|--------|---------------|
| RETRV-01: Stale results downranked via time-decay relative to newest | SATISFIED | Time-decay formula implemented, wired in route_query, config flows from config.toml. |
| RETRV-02: Supersession detection marks older semantically similar results | SATISFIED | `apply_with_supersession` with pairwise cosine similarity, `superseded_by` metadata, 15% penalty. |
| RETRV-03: High-salience events exempt from time-decay | SATISFIED | `is_exempt()` covers Constraint, Definition, Procedure, Preference with case-insensitive matching. |
| RETRV-04: Time-decay half-life configurable (default 14 days) via config.toml | SATISFIED | Full chain verified: config.toml -> Settings.staleness -> start_daemon -> run_server_with_scheduler -> MemoryServiceImpl::with_scheduler -> RetrievalHandler. No hardcoded defaults remain in production paths. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/memory-service/src/server.rs` | 50, 91 | `MemoryServiceImpl::new(storage)` used by `run_server` and `run_server_with_shutdown` | Info | These simpler server variants are not called from memory-daemon startup — the daemon always uses `run_server_with_scheduler`. Not a production concern. |

No blockers. No warnings. The previously identified warning-level anti-patterns (hardcoded `StalenessConfig::default()` in ingest.rs) have been fully resolved.

### Re-Verification: Gaps Closed

**Previous gap (RETRV-04 partial):**
- `run_server_with_scheduler` had no `StalenessConfig` parameter (FIXED: parameter added at server.rs:122)
- `start_daemon` did not pass `settings.staleness` (FIXED: commands.rs:510 now passes `settings.staleness.clone()`)
- All `with_services()` call sites in ingest.rs hardcoded `StalenessConfig::default()` (FIXED: all 7 constructors now accept and forward the parameter)
- `with_scheduler` used `RetrievalHandler::new` which ignored staleness (FIXED: now uses `RetrievalHandler::with_services` with the config)

**Regression check:** Truths 1-9 from initial verification were confirmed via re-reads of stale_filter.rs, retrieval.rs, and config.rs. No regressions detected.

**Commits verified:**
- `84aca3d` — Add StalenessConfig parameter to server and service constructors
- `2c96836` — Propagate settings.staleness from daemon startup to RetrievalHandler

### Human Verification Required

#### 1. End-to-End Time Decay on Live Queries

**Test:** Ingest 10 events spread across the last 60 days. Query with a phrase that matches all of them. Observe that result scores decrease monotonically with age.
**Expected:** Events 14 days old score approximately 19% lower than newest. Events 28 days old score approximately 26% lower.
**Why human:** Requires running daemon, real storage, and timed ingest — not reproducible in unit tests.

#### 2. End-to-End Supersession Detection

**Test:** Ingest two semantically near-identical events 24 hours apart. Query with a matching phrase. Verify the older result has `superseded_by` in its metadata.
**Expected:** Older result has `superseded_by: <doc_id_of_newer>` and its score is reduced by approximately 15% on top of time-decay.
**Why human:** Requires real CandleEmbedder and populated HNSW index producing >0.80 cosine similarity.

#### 3. Verify config.toml staleness section takes effect

**Test:** Set `half_life_days = 7` in config.toml staleness section. Start daemon. Ingest two events, one 7 days old. Query. Confirm the 7-day-old result is penalized more than with the default 14-day half-life.
**Expected:** With half_life=7, a 7-day-old result should be penalized roughly 50% of max_penalty (~15%). With default half_life=14, the same result would show ~26% of max_penalty (~7.8%). The difference is observable in scores.
**Why human:** Requires daemon restart with modified config.toml and live data — validates the end-to-end RETRV-04 config-to-behavior chain.

---

_Verified: 2026-03-09T21:38:56Z_
_Verifier: Claude (gsd-verifier)_
