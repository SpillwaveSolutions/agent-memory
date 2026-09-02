# Phase 59: Guardrails and Inventory — Plan

**Milestone:** v3.2 Prove It
**Goal:** the next tag cannot repeat the 2026-09-01 stale-ref incident; the
March `gsd/` line is inventoried before anyone cherry-picks it; PROJECT.md
and the public backlog match v3.1.0 shipped.

See: [v3.2-prove-it-plan.md](v3.2-prove-it-plan.md)

## 59-01: Release pipeline checks

- `scripts/release-guards.sh` — ancestor of `origin/main` + crate version
- `scripts/changelog-section.sh` — `## vX.Y.Z` section or fail
- `release.yml` `guard` job before any platform build
- Publish `if: success()` (not `always()`); all five archives required
- Notes from CHANGELOG, not GitHub's PR auto-list
- `docs/RELEASING.md` — `git tag -a vX.Y.Z <sha>`
- Unit tests on every PR (`Release Guard Scripts`)

## 59-02: Orphan branch triage

Inventory: [phase-59-orphan-branch-triage.md](phase-59-orphan-branch-triage.md).

Keep (cherry-pick in Phase 61): export/import streaming RPCs; Claude Code
CREG/META. Skip: OpenCode converter. Do not merge any `gsd/phase-*` branch
wholesale — add/add on `memory-orchestrator` and `memory-bench` would
regress Make It True.

## 59-03: Planning truth

- PROJECT.md Current State is v3.1.0 shipped / v3.2 executing
- OpenCode removed from the adapter list; API summarizer moved out of Deferred
- ROADMAP/STATE/MILESTONES know about v3.2
- Issues #39–#44 are the public backlog

## Exit criteria

1. `bash scripts/release-guards-test.sh` exits 0
2. A disagreeing version or a commit not on main cannot get past `guard`
3. Issues #39–#44 exist
4. `task pr-precheck` green
