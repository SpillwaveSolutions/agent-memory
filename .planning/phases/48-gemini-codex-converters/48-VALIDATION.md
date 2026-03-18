---
phase: 48
slug: gemini-codex-converters
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 48 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml workspace |
| **Quick run command** | `cargo test -p memory-installer` |
| **Full suite command** | `cargo test -p memory-installer && cargo clippy -p memory-installer --all-targets --all-features -- -D warnings` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p memory-installer`
- **After every plan wave:** Run `cargo test -p memory-installer && cargo clippy -p memory-installer --all-targets --all-features -- -D warnings`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 48-01-01 | 01 | 1 | GEM-01,GEM-02 | unit | `cargo test -p memory-installer -- gemini` | ✅ | ⬜ pending |
| 48-02-01 | 02 | 1 | CDX-01,CDX-02,CDX-03,CDX-04 | unit | `cargo test -p memory-installer -- codex` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. The memory-installer crate already has test infrastructure from Phase 46-47.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Gemini settings.json merge preserves user settings | GEM-05 | Requires real .gemini/settings.json | Create sample settings.json, run installer, verify merge |
| Codex AGENTS.md renders correctly | CDX-03 | Visual formatting check | Run installer, inspect generated AGENTS.md |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
