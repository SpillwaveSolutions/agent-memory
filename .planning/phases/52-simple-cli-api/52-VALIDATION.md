---
phase: 52
slug: simple-cli-api
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-03-22
---

# Phase 52 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + tokio::test |
| **Config file** | Workspace Cargo.toml (existing) |
| **Quick run command** | `cargo test -p memory-cli` |
| **Full suite command** | `cargo test --workspace --all-features` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p memory-cli && cargo clippy -p memory-cli -- -D warnings`
- **After every plan wave:** Run `task pr-precheck`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CLI-01 | 6 subcommands parse correctly | unit | `cargo test -p memory-cli cli` | Wave 0 |
| CLI-02 | search returns JSON envelope | unit | `cargo test -p memory-cli search` | Wave 0 |
| CLI-03 | recall delegates to search | unit | `cargo test -p memory-cli recall` | Wave 0 |
| CLI-04 | add errors when daemon down | unit | `cargo test -p memory-cli add` | Wave 0 |
| CLI-05 | TTY detection logic | unit | `cargo test -p memory-cli output` | Wave 0 |
| CLI-06 | context returns structured JSON | unit | `cargo test -p memory-cli context` | Wave 0 |
| CLI-07 | timeline/summary query TOC | unit | `cargo test -p memory-cli timeline` | Wave 0 |
| CLI-08 | daemon binary unchanged | manual | Verify no changes to memory-daemon crate | N/A |
| CLI-09 | Exit codes 0/non-zero | unit | `cargo test -p memory-cli exit` | Wave 0 |
| CLI-10 | tokens_estimated in meta | unit | `cargo test -p memory-cli meta` | Wave 0 |

*Status: ⬜ pending*

---

## Wave 0 Requirements

- [ ] `crates/memory-cli/` — entire crate does not exist yet
- [ ] Unit tests for `JsonEnvelope` serialization
- [ ] Unit tests for CLI argument parsing
- [ ] Unit tests for range parsing utility
- [ ] Note: Integration tests requiring running daemon should be in `crates/e2e-tests/` or marked `#[ignore]`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| daemon binary unchanged | CLI-08 | File comparison, not unit testable | Verify no changes to memory-daemon crate via git diff |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-03-22
