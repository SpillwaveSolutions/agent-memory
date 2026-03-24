---
phase: 56
slug: import-bootstrap
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-03-24
---

# Phase 56 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + tokio::test |
| **Config file** | Workspace Cargo.toml (existing) |
| **Quick run command** | `cargo test -p memory-service --test import_round_trip` |
| **Full suite command** | `cargo test --workspace --all-features` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p memory-service && cargo clippy -p memory-service -- -D warnings`
- **After every plan wave:** Run `task pr-precheck`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| IMPORT-01 | Full restore from backup dir | integration | `cargo test -p memory-service --test import_round_trip` | Wave 0 |
| IMPORT-02 | Round-trip: export→wipe→import→verify | integration | `cargo test -p memory-service --test import_round_trip` | Wave 0 |
| IMPORT-03 | --dry-run shows counts, no writes | unit | `cargo test -p memory-cli import` | Wave 0 |
| IMPORT-04 | Idempotent (dedup by event_id) | integration | `cargo test -p memory-service --test import_round_trip` | Wave 0 |
| IMPORT-05 | Client-streaming RPC | unit | `cargo build -p memory-service` | Wave 0 |
| IMPORT-06 | Events-only import + rebuild-toc ref | unit | `cargo test -p memory-cli import` | Wave 0 |
| GRPC-03 | ImportBackup client-streaming RPC | unit | `cargo build -p memory-service` | Wave 0 |

---

## Wave 0 Requirements

- [ ] `crates/memory-service/src/import.rs` — new handler module
- [ ] `crates/memory-service/tests/import_round_trip.rs` — round-trip integration tests
- [ ] `crates/memory-cli/src/commands/import.rs` — new CLI command module

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Live daemon round-trip | IMPORT-02 | Requires running daemon + real RocksDB | Start daemon, backup, wipe db, import, verify via `memory search` |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-03-24
