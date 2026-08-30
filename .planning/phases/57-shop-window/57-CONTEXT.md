# Phase 57: Shop Window & Positioning

**Gathered:** 2026-08-30
**Status:** In execution
**Source:** docs/plans/phase-57-shop-window-plan.md

Make the public face of the repo match the reality Phases 54–56 established:
a root README with an honest status table, a LICENSE, a positioning writeup
that leads with the structural differences and declines to make a benchmark
comparison it cannot back, and a supported-surface trim that deletes the
OpenCode stub instead of shipping empty methods.

## What was already true before this phase

- No root `README.md` — the GitHub landing page was empty
- No `LICENSE` file, despite `license = "MIT"` in `Cargo.toml`
- `workspace.package.repository` pointed at `spillwave/agent-memory`, not the
  actual remote `SpillwaveSolutions/agent-memory`
- `docs/README.md` advertised "Passive capture from Claude Code, OpenCode,
  Gemini CLI hooks" and a "Plugin (TypeScript)" OpenCode adapter; the OpenCode
  converter's methods all returned empty and `plugins/memory-opencode-plugin/`
  contained only an archived README
- Five bats CLI suites gated every PR, one of them for a runtime with no
  working converter

## Constraints carried in

- **Benchmark gate (Phase 56):** the only committed results are mock-backend
  and mock-judge, so no comparative accuracy claim may appear anywhere public
- **Execution-evidence rule (v3.1 process change):** the quickstart is a
  run-dependent requirement, so it must be verified by actually executing it
  and committing the transcript — not by the README's existence
