---
phase: 37-stale-filter
plan: 03
subsystem: retrieval
tags: [staleness, config-propagation, daemon-wiring, gap-closure]
dependency_graph:
  requires: [StaleFilter, StalenessConfig, stale-filter-wiring]
  provides: [config-driven-staleness, RETRV-04-closed]
  affects: [memory-service, memory-daemon]
tech_stack:
  added: []
  patterns: [config-propagation, constructor-parameter-threading]
key_files:
  created: []
  modified:
    - crates/memory-service/src/server.rs
    - crates/memory-service/src/ingest.rs
    - crates/memory-daemon/src/commands.rs
decisions:
  - All MemoryServiceImpl with_* constructors accept StalenessConfig (no defaults in production)
  - with_scheduler now uses RetrievalHandler::with_services instead of ::new
metrics:
  duration: 4min
  completed: "2026-03-09T21:35:00Z"
---

# Phase 37 Plan 03: StalenessConfig Wiring from Settings to RetrievalHandler Summary

Config-driven staleness propagation from config.toml through daemon startup to RetrievalHandler, closing RETRV-04.

## What Was Built

### server.rs - StalenessConfig Parameter
- `run_server_with_scheduler` now accepts `staleness_config: StalenessConfig` as final parameter
- Forwards config to `MemoryServiceImpl::with_scheduler`
- Added `use memory_types::config::StalenessConfig` import

### ingest.rs - Constructor Updates (7 constructors)
- `with_scheduler`: Now accepts StalenessConfig, uses `RetrievalHandler::with_services` instead of `::new`
- `with_scheduler_and_search`: Accepts StalenessConfig instead of hardcoding default
- `with_search`: Accepts StalenessConfig instead of hardcoding default
- `with_vector`: Accepts StalenessConfig instead of hardcoding default
- `with_topics`: Accepts StalenessConfig instead of hardcoding default
- `with_all_services`: Accepts StalenessConfig instead of hardcoding default
- `with_all_services_and_topics`: Accepts StalenessConfig instead of hardcoding default
- Zero `StalenessConfig::default()` calls remain in production ingest.rs code

### commands.rs - Daemon Startup Wiring
- `start_daemon` passes `settings.staleness.clone()` to `run_server_with_scheduler`
- Added info log line showing staleness filter enabled/half_life/max_penalty at startup
- Complete propagation chain: config.toml -> Settings -> start_daemon -> run_server_with_scheduler -> MemoryServiceImpl::with_scheduler -> RetrievalHandler

## Verification Results

1. `cargo clippy --workspace --all-targets --all-features -- -D warnings` - PASSED
2. `cargo test --workspace --all-features` - PASSED (all tests)
3. Zero `StalenessConfig::default()` in non-test ingest.rs code - VERIFIED
4. `settings.staleness` referenced in commands.rs - VERIFIED
5. `staleness_config` parameter in server.rs - VERIFIED

## Deviations from Plan

None - plan executed exactly as written.

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | 84aca3d | Add StalenessConfig parameter to server and service constructors |
| 2 | 2c96836 | Propagate settings.staleness from daemon startup to RetrievalHandler |

## Self-Check: PASSED
