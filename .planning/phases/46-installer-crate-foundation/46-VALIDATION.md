---
phase: 46
slug: installer-crate-foundation
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-03-17
---

# Phase 46 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + cargo test |
| **Config file** | none — standard Rust test runner |
| **Quick run command** | `cargo test -p memory-installer` |
| **Full suite command** | `cargo test --workspace --all-features` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p memory-installer`
- **After every plan wave:** Run `cargo test --workspace --all-features`
- **Before `/gsd:verify-work`:** `task pr-precheck` (format + clippy + test + doc) green
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 46-01-01 | 01 | 1 | INST-01 | unit | `cargo test -p memory-installer cli` | ❌ W0 | ⬜ pending |
| 46-01-01 | 01 | 1 | INST-01 | unit | `cargo test -p memory-installer cli::test_skills_requires_dir` | ❌ W0 | ⬜ pending |
| 46-01-01 | 01 | 1 | INST-01 | unit | `cargo test -p memory-installer cli::test_dry_run_flag` | ❌ W0 | ⬜ pending |
| 46-01-02 | 01 | 1 | INST-03 | compile | `cargo build -p memory-installer` | ❌ W0 | ⬜ pending |
| 46-02-01 | 02 | 2 | INST-02 | integration | `cargo test -p memory-installer parser::test_parse_sources` | ❌ W0 | ⬜ pending |
| 46-02-01 | 02 | 2 | INST-02 | unit | `cargo test -p memory-installer parser::test_parse_command` | ❌ W0 | ⬜ pending |
| 46-02-01 | 02 | 2 | INST-02 | unit | `cargo test -p memory-installer parser::test_parse_no_frontmatter` | ❌ W0 | ⬜ pending |
| 46-02-01 | 02 | 2 | INST-02 | unit | `cargo test -p memory-installer parser::test_parse_skill_dir` | ❌ W0 | ⬜ pending |
| 46-03-01 | 03 | 3 | INST-04 | unit | `cargo test -p memory-installer tool_maps::test_opencode_map` | ❌ W0 | ⬜ pending |
| 46-03-01 | 03 | 3 | INST-04 | unit | `cargo test -p memory-installer tool_maps::test_gemini_map` | ❌ W0 | ⬜ pending |
| 46-03-01 | 03 | 3 | INST-04 | unit | `cargo test -p memory-installer tool_maps::test_task_excluded_gemini` | ❌ W0 | ⬜ pending |
| 46-03-02 | 03 | 3 | INST-05 | unit | `cargo test -p memory-installer writer::test_merge_new_file` | ❌ W0 | ⬜ pending |
| 46-03-02 | 03 | 3 | INST-05 | unit | `cargo test -p memory-installer writer::test_merge_existing_markers` | ❌ W0 | ⬜ pending |
| 46-03-02 | 03 | 3 | INST-05 | unit | `cargo test -p memory-installer writer::test_merge_no_markers` | ❌ W0 | ⬜ pending |
| 46-03-02 | 03 | 3 | INST-06 | unit | `cargo test -p memory-installer writer::test_dry_run_no_write` | ❌ W0 | ⬜ pending |
| 46-03-02 | 03 | 3 | INST-07 | unit | `cargo test -p memory-installer tool_maps::test_unmapped_warns` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/memory-installer/` — entire crate does not exist yet
- [ ] `crates/memory-installer/Cargo.toml` — package definition
- [ ] `gray_matter = { version = "0.3", features = ["yaml"] }` in workspace deps
- [ ] `walkdir = "2.5"` in workspace deps

*All test files are inline `#[cfg(test)]` modules — no separate test directory.*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 5s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
