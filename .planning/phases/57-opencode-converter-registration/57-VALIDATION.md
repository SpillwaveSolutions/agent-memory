---
phase: 57
slug: opencode-converter-registration
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-03-25
---

# Phase 57 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test |
| **Config file** | Workspace Cargo.toml (existing) |
| **Quick run command** | `cargo test -p memory-installer opencode` |
| **Full suite command** | `cargo test --workspace --all-features` |
| **Estimated runtime** | ~20 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p memory-installer opencode && cargo clippy -p memory-installer -- -D warnings`
- **After every plan wave:** Run `task pr-precheck`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 20 seconds

---

## Per-Task Verification Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| OC-01 | Commands in singular command/ dir | unit + E2E | `cargo test -p memory-installer opencode` | Wave 0 |
| OC-02 | Agent tools: object format | unit | `cargo test -p memory-installer opencode` | Wave 0 |
| OC-03 | Tool name lowercase + special mappings | unit | `cargo test -p memory-installer opencode` | Wave 0 |
| OC-04 | Color names → hex values | unit | `cargo test -p memory-installer opencode` | Wave 0 |
| OC-05 | Path rewriting ~/.claude/ → ~/.config/opencode/ | unit + E2E | `cargo test -p memory-installer opencode` | Wave 0 |
| OC-06 | opencode.json permissions generated | unit + E2E | `cargo test -p memory-installer opencode` | Wave 0 |
| OREG-01 | Writes opencode.json with permissions | E2E | `cargo test -p memory-installer e2e` | Wave 0 |
| OREG-02 | Glob patterns match installed paths | unit | `cargo test -p memory-installer opencode` | Wave 0 |
| OREG-03 | Merge existing opencode.json | unit | `cargo test -p memory-installer opencode` | Wave 0 |

---

## Wave 0 Requirements

- [ ] `crates/memory-installer/src/converters/opencode.rs` — replace stub with real implementation
- [ ] Unit tests for convert_command, convert_agent, convert_skill, generate_guidance
- [ ] E2E test updated from stub assertions to real output verification

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 20s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-03-25
