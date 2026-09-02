# Phase 59: Guardrails and Inventory

**Gathered:** 2026-09-01
**Status:** In execution
**Source:** docs/plans/v3.2-prove-it-plan.md

v3.1 made the claims true. v3.2 makes them provable. Phase 59 is small,
first, and it protects everything after it: the release pipeline cannot
ship a tag that is not on main, the March orphan line is inventoried
before anyone cherry-picks it, and the planning source of truth matches
the shipped v3.1.0.

## What was already true before this phase

- `v3.1.0` exists as a 5-of-5-platform GitHub Release (2026-09-01 01:45 UTC)
- The first tag push shipped `acc7294` (Cargo.toml `2.7.0`) for 17 minutes
- The four-binary archive guard from #37 held; ancestor/version/changelog
  checks did not exist
- Zero GitHub issues; the backlog lived only in `.planning/`
- `PROJECT.md` still said "Version: v3.0 (In Progress)" and listed OpenCode
- `origin/gsd/phase-56-import-bootstrap` (and the nested 54–58 line) sat
  unmerged, 20 commits behind main, with an add/add war waiting in
  `memory-orchestrator` and `memory-bench`

## Constraints carried in

- Never commit to `main`; feature branch + PR
- Do not delete the `gsd/` branches in this phase — inventory only
- Do not cherry-pick export/import or CREG/META yet (Phase 61)
- Do not run a live `v9.9.9` tag; `dry_run` + unit tests are the verify
- Maintainer decisions (adopted with the plan): cherry-pick by feature;
  blog now; unit files not double-fork
