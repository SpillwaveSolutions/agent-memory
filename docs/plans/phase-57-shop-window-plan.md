# Phase 57: Shop Window & Positioning — Plan

**Milestone:** v3.1 Make It True
**Goal:** a stranger landing on the repo understands what it is, trusts it, and
can run it — and the project's public claims match Phases 54–56 reality.

## 57-01: Repo hygiene

| Item | Decision |
|---|---|
| Root `README.md` | New. One-paragraph local-first pitch, ASCII architecture diagram, 5-minute quickstart, honest status table, supported-surface tiers, docs index |
| `LICENSE` | MIT, matching `workspace.package.license` |
| `workspace.package.repository` | `spillwave/agent-memory` → `SpillwaveSolutions/agent-memory` (matches the git remote) |
| GitHub description / topics / Discussions | Cannot be set from a PR — repo settings. Listed as a maintainer action in the verification doc rather than claimed as done |
| Demo recording | Not produced. A real executed quickstart transcript is committed instead; an asciinema/GIF is left open and recorded as not done |

## 57-02: Positioning writeup

`docs/positioning/agent-memory-vs-competition.md`, leading with the three
structural differences (passive zero-token capture, local-first, cross-CLI),
head-to-head table vs Mem0 / Zep / MemMachine / Letta, an explicit "where they
are ahead of us" section, the platform-risk answer, and a claims ledger with
sources and check dates.

**Benchmark gate honored:** the only committed results are mock-backend and
mock-judge, so the doc makes no comparative accuracy claim and says why.

## 57-03: Scope trim — supported-surface tiering

- **Tier 1 (PR gate):** Claude Code, Codex CLI
- **Tier 2 (weekly schedule):** Gemini CLI, Copilot CLI
- **Removed:** OpenCode. The converter's methods all returned empty, so
  `memory-installer --agent opencode` reported success and wrote nothing.
  Deleted: the converter, the `Runtime::OpenCode` variant, its tool mappings,
  its bats suite, and the archived `plugins/memory-opencode-plugin/` stub.
  A regression test asserts every runtime the installer offers actually
  converts something.
- `e2e-cli.yml` becomes the Tier 1 gate; new `e2e-cli-tier2.yml` runs Tier 2
  weekly and on dispatch. Tiering ≠ removal: Tier 2 converters keep their
  tests, they just do not block a PR.

## Exit criteria

1. GitHub landing page renders the new README
2. The README quickstart has been executed start-to-finish, verbatim, on a
   machine that did not previously have the toolchain — transcript committed
3. `task pr-precheck` green
