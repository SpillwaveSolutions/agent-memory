---
phase: 49
slug: copilot-skills-hooks
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 49 — Validation Strategy

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
| 49-01-01 | 01 | 1 | COP-01,COP-02,COP-03 | unit | `cargo test -p memory-installer -- copilot` | ✅ | ⬜ pending |
| 49-02-01 | 02 | 1 | SKL-01,SKL-02,SKL-03 | unit | `cargo test -p memory-installer -- skills` | ✅ | ⬜ pending |
| 49-03-01 | 03 | 2 | HOOK-01,HOOK-02,HOOK-03 | unit | `cargo test -p memory-installer -- hook` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. The memory-installer crate already has test infrastructure from Phases 46-48.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Hook scripts execute correctly with fail-open behavior | HOOK-03 | Requires real daemon process | Run installer, trigger hook event, verify background execution |
| All 6 --agent flags produce valid output | SC-4 | Integration across all converters | Run `memory-installer install --agent {each}` and inspect output |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
