---
phase: 59-guardrails-inventory
verified: 2026-09-01
status: passed
---

# Phase 59: Guardrails and Inventory Verification

**Phase Goal:** the next tag cannot repeat the 2026-09-01 incident; the March
line is inventoried; the planning source of truth matches v3.1.0 shipped.

## Execution evidence

| # | Claim | Status | Evidence |
|---|-------|--------|----------|
| 1 | Tagged commit must be an ancestor of origin/main | **RUN** | `scripts/release-guards-test.sh`: "commit not on main fails"; `release.yml` `guard` job runs the script before any build |
| 2 | Crate version must equal tag minus `v` | **RUN** | same test: "version mismatch fails (the v3.1.0-on-2.7.0 incident)" |
| 3 | Missing CHANGELOG section fails | **RUN** | `changelog-section.sh 9.9.9` exits 1; `changelog-section.sh 3.1.0 CHANGELOG.md` prints the v3.1.0 body and stops before v2.7.0 |
| 4 | Publish requires every matrix build to succeed | FILE | `release.yml` Create Release `if: ${{ success() && ... }}` — a custom `if` without `success()` would reintroduce `always()` |
| 5 | Partial archives cannot ship | FILE | "Require all five platform archives" step lists linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64, windows-x86_64 |
| 6 | Release notes come from CHANGELOG.md | FILE | `generate_release_notes: false` + `body_path: release-notes.md` from `changelog-section.sh` |
| 7 | Tag procedure is documented as explicit-SHA | FILE | `docs/RELEASING.md` — `git tag -a vX.Y.Z <sha>` |
| 8 | Guard tests run on every PR | FILE | `ci.yml` job `Release Guard Scripts`; `CI Success` needs `release-guards` |
| 9 | Orphan-branch inventory exists with a conflict map | FILE | `docs/plans/phase-59-orphan-branch-triage.md` — 35 unmerged files on phase-56, add/add on orchestrator/bench, cherry-pick commit list |
| 10 | PROJECT.md Current State is v3.1 shipped | FILE | `.planning/PROJECT.md` |
| 11 | Known gaps are GitHub issues | **RUN** | #39 LOCOMO, #40 vector/topic, #41 backfill, #42 install-service, #43 TOC rebuild, #44 cross-encoder |
| 12 | v3.2 is on ROADMAP and STATE | FILE | `.planning/ROADMAP.md`, `.planning/STATE.md` |

## Human verification (blockers)

- [x] `bash scripts/release-guards-test.sh` exits 0
- [x] `bash scripts/changelog-section.sh 3.1.0 CHANGELOG.md` extracts the real v3.1.0 section
- [x] Issues #39–#44 exist and link the v3.2 plan
- [ ] Live `workflow_dispatch` + `dry_run: true` with a disagreeing version (maintainer — do not push `v9.9.9`)

## Not done (stated, not waved through)

| Item | Why | Owner |
|---|---|---|
| Live dry-run dispatch against GitHub | Needs `workflow_dispatch` on the merged workflow; unit tests cover the same predicates | Maintainer, after this PR merges |
| Delete the `gsd/` branches | 59-02 inventories; deletion waits until 61 has cherry-picked the kept features | Phase 61 |
| Cherry-pick export/import and CREG/META | Feeds 61-01 and 61-05; not this phase | Phase 61 |
| Blog post | Maintainer launch action; recommended "now" | Maintainer |
