# Orphan branch triage — SpillwaveSolutions/agent-memory

**Phase:** 59-02. Inventory vs `origin/main` @ `d6c8ac7`. Do not delete these
branches in this phase. Cherry-pick by feature in Phase 61.

**Maintainer decision (2026-09-01):** cherry-pick export/import and Claude
Code registration by feature. Skip the OpenCode converter. Keep the branches
until those cherry-picks land.

Trial merges were run in throwaway worktrees and discarded; the clone was
not modified.

## How to read this

Two independent "v3.1" numbering schemes collided:

| Line | When | What "v3.1" meant | What "phase 54–58" meant |
|---|---|---|---|
| **March GSD line (these orphans)** | 2026-03-21 → 03-25 | Memory Export/Import | 54 daily markdown, 55 JSONL backup, 56 import/bootstrap, then v3.2: 57 OpenCode converter, 58 Claude registration + plugin.json |
| **Current main** | 2026-05 → 08 | "Make It True" | 54 Integration Truth (#32), 54.5 truth leaks, 55 honest percentiles, 56 LOCOMO adapter v2, 57 shop window (deleted the OpenCode stub) |

Shared fork point for the March line: merge-base
`9be18d8d5e3ad4726f9f0a1cc892c34de0633f03` (`chore: release v2.7.0`, 2026-03-21).
All five primary branches (and leftover `phase-53`) are **20 commits behind**
main. Main later landed the same v3.0 work via PRs #28/#29/#30 (different SHAs),
then built a different v3.1 on top.

**Ancestry (nested, each tip is a descendant of the previous):**

```
9be18d8 (v2.7.0)
  └─ origin/gsd/phase-53-benchmark-suite          b1af44a  (leftover)
       └─ origin/gsd/phase-54-daily-markdown-export          2370a2f
            └─ origin/gsd/phase-55-structured-backup        5491f59
                 └─ origin/gsd/phase-56-import-bootstrap    88eb6da
                      └─ origin/gsd/phase-57-opencode-converter-registration  ee2ff82
                           └─ origin/gsd/phase-58-claude-registration-metadata  4f26f49
```

Triple-dot `git diff origin/main...tip` treats `memory-orchestrator`,
`memory-bench`, and `memory-cli` as **Added** because they did not exist at
`9be18d8`. They **do** exist on current main (independent history). That is
why a naïve merge is an add/add war on those crates — see Conflict map.

---

## PRIMARY 1 — `origin/gsd/phase-54-daily-markdown-export`

1. **Tip** `2370a2f1df1d16604448430e4e3c12bdbc387dc5` — 2026-03-23 18:03:04 -0500 —
   `docs(55): create phase plan for structured backup`.
   **63 ahead / 20 behind** main. Merge-base `9be18d8d5e3ad4726f9f0a1cc892c34de0633f03`.
   Unique vs previous tip (phase-53): **8 commits**.
2. **Log `merge-base..tip` (63 commits).** Newest 15:
   ```
   2370a2f docs(55): create phase plan for structured backup
   ccfaf16 docs(55): research phase domain
   39992db docs(phase-54): complete phase execution
   c27317e docs(54-02): complete daily CLI subcommand plan
   5be2c19 feat(54-02): add `memory daily` CLI subcommand with markdown rendering
   2ac7532 docs(54-01): complete ExportDaily RPC plan
   408003b feat(54-01): implement ExportDaily handler, trait dispatch, and client method
   1e29127 feat(54-01): add ExportDaily proto messages and RPC
   b1af44a docs(54): create phase plan for daily markdown export
   6efdb2d docs(54): research phase domain
   f4fd1a2 docs: populate CONTEXT.md files for phases 54-56 from spec
   f579d0b docs: create milestone v3.1 roadmap (3 phases)
   12aadf0 docs: define milestone v3.1 requirements
   2b20d7b docs: start milestone v3.1 Memory Export/Import
   cf163c0 chore: complete v3.0 Competitive Parity & Benchmarks milestone
   ```
   Oldest 10 (v3.0 scaffold already on main via PRs):
   ```
   e14625c feat(51-02): implement RRF fusion with deduplication and consensus boosting
   9151d49 docs(51-01): complete retrieval orchestrator scaffold plan
   7dc22c8 feat(51-01): implement heuristic query expansion with 6 tests
   7874baa feat(51-01): scaffold memory-orchestrator crate with core types
   1d25a22 fix(51): revise plan 03 for ORCH-04 mock LLM reranker integration test
   5b8aede docs(51): create phase plan for retrieval orchestrator
   bb0420e docs(51): generate context from PRD
   0fe32db docs: create milestone v3.0 roadmap (3 phases)
   087f251 docs: define milestone v3.0 requirements
   d84213a docs: start milestone v3.0 Competitive Parity & Benchmarks
   ```
   Feature-only increment vs phase-53 (8 commits): proto `ExportDaily`, handler in
   `memory-service/src/query.rs` + trait dispatch in `ingest.rs`, client method,
   `memory daily` CLI (`commands/daily.rs`, 371 lines).
3. **`git diff --stat origin/main...tip`:** 100 files, +15836/−215
   (non-planning: 52 files, +4995/−26). Highlight:
   - `crates/memory-orchestrator/` — shown as add (878-line March snapshot; **main is 1458 lines**)
   - `crates/memory-bench/` — shown as add (1319-line March snapshot; **main is 2649 lines + `judge.rs`**)
   - `crates/memory-service/src/query.rs` +160/−, `ingest.rs` +43
   - `proto/memory.proto` +37 (`rpc ExportDaily`)
   - `crates/memory-cli/src/commands/daily.rs` +371 (**new**)
   - `crates/memory-installer/` — no change
   - `plugins/` — no change
4. **Feature:** daily markdown export. Unary RPC `ExportDaily(ExportDailyRequest) returns (ExportDailyResponse)`
   with `DayExport` (TOC day node, segments, events, grips, `has_rollup`). CLI
   `memory daily` renders markdown. Design spec
   `docs/superpowers/specs/2026-03-23-memory-export-import-design.md`.
5. **On current main?** **Partial.** Orchestrator/bench/cli crates exist (evolved).
   `daily.rs`, `ExportDaily` RPC, `export_daily` handler/client: **no**.

---

## PRIMARY 2 — `origin/gsd/phase-55-structured-backup`

1. **Tip** `5491f59f55319367e44a258de13911f7ead5c748` — 2026-03-24 12:19:16 -0500 —
   `docs(56): add validation strategy and revise plans for IMPORT-02`.
   **72 ahead / 20 behind.** Merge-base `9be18d8`. Unique vs phase-54: **9 commits**.
2. **Increment vs phase-54:**
   ```
   5491f59 docs(56): add validation strategy and revise plans for IMPORT-02
   353ff76 docs(56): create phase plan for import/bootstrap
   08ab1c8 docs(56): research phase domain
   e500b22 docs(phase-55): complete phase execution
   bcda199 docs(55-02): complete backup CLI command plan
   bf51772 feat(55-02): add memory backup CLI command with streaming client
   f659356 docs(55-01): complete structured backup server-side plan
   2f2b148 feat(55-01): add streaming backup handler with service wiring
   858532b feat(55-01): add ExportBackup proto definitions, tokio-stream dep, and storage iteration methods
   ```
   Full `merge-base..tip` = 72 commits (phase-54's 63 + these 9).
3. **Triple-dot vs main:** 114 files, +18476/−219 (non-planning 59, +5837/−30).
   New vs phase-54:
   - `crates/memory-service/src/backup.rs` +308 (**new**; first streaming RPC)
   - `crates/memory-cli/src/commands/backup.rs` +308 (**new**)
   - `proto/memory.proto` +41 (`rpc ExportBackup(BackupOptions) returns (stream BackupChunk)`)
   - `crates/memory-storage/src/db.rs` +35 (list-all-grips for export)
   - `crates/memory-storage/src/episodes.rs` +41
   - `crates/memory-client` gains `tokio-stream` + `export_backup()`
   - orchestrator/bench still the March snapshot
4. **Feature:** structured JSONL backup. Server-streaming `ExportBackup`. Chunk
   types: events, TOC (segment/day/week/month/year), grips, episodes, manifest.
   Options: `events_only`, `since_ms`, `until_ms`. CLI `memory backup`.
5. **On current main?** **No** for backup.rs / ExportBackup / `memory backup`.
   `tokio-stream` already a workspace/client dep on main. Storage list helpers: **no**.

---

## PRIMARY 3 — `origin/gsd/phase-56-import-bootstrap`  ← export/import foundation tip

1. **Tip** `88eb6dae85a1363ddd89569a0137819d4bd53d8e` — 2026-03-25 15:04:29 -0500 —
   `docs(57): fix plan issues — add OREG-01, remove -x flag, add VALIDATION.md`.
   **88 ahead / 20 behind.** Merge-base `9be18d8`. Unique vs phase-55: **16 commits**.
2. **Increment vs phase-55 (newest 15 of the 16; last is proto):**
   ```
   88eb6da docs(57): fix plan issues — add OREG-01, remove -x flag, add VALIDATION.md
   19dce24 docs(57): create phase plan for OpenCode converter + registration
   15edbb0 docs(57): research phase domain
   98f83c6 docs(57): generate context from codebase-mentor reference
   dab420e docs: create milestone v3.2 roadmap (3 phases)
   1d7deae docs: define milestone v3.2 requirements
   3c61ec1 docs: start milestone v3.2 Plugin Installer & OpenCode Converter
   acc7294 chore: complete v3.1 Memory Export/Import milestone
   7fe632a docs(phase-56): complete phase execution
   1db4faa docs(56-02): complete import CLI + round-trip tests plan
   acbd6ae chore(56-02): apply cargo fmt and clippy fixes across workspace
   53fa9fc test(56-02): add round-trip integration tests for import handler
   ea9ee1d feat(56-02): add memory import CLI command with manifest validation
   0786e4a docs(56-01): complete import bootstrap server plan
   b6c7935 feat(56-01): add import handler, service wiring, and client method
   2c737a9 feat(56-01): add ImportBackup client-streaming RPC to proto
   ```
   Non-planning increment vs phase-55: **12 files, +688/−8**.
3. **Triple-dot vs main:** 126 files, +20953/−208 (non-planning 62, +6517/−30).
   Highlight crates (triple-dot, includes the v3.0 add/add illusion):
   - `crates/memory-service/src/import.rs` +280 (**new**)
   - `crates/memory-service/tests/import_round_trip.rs` +130 (**new**)
   - `crates/memory-cli/src/commands/import.rs` +164 (**new**)
   - `proto/memory.proto` +110 total vs main (`ImportBackup(stream ImportChunk) returns (ImportResult)` plus 54/55)
   - `crates/memory-client/src/client.rs` +110/− (`export_daily` / `export_backup` / `import_backup`)
   - `crates/memory-service/src/ingest.rs` +71/− (RPC trait dispatch for all three)
   - `crates/memory-orchestrator/` +878 lines shown as add — **do not take**
   - `crates/memory-bench/` +1319 lines shown as add — **do not take**
   - `crates/memory-installer/` — **no change at this tip**
   - `plugins/` — `plugins/memory-opencode-plugin/README.md` exists here, not on main
4. **Feature:** import/bootstrap. Client-streaming `ImportBackup`. `ImportChunk`
   reuses `BackupChunkType`, plus `dry_run` and `events_only`. `ImportResult`
   counts events/TOC/grips/episodes skipped+imported. CLI `memory import` with
   manifest validation. Post-import hint: `memory admin rebuild-toc` (see grep:
   that command is a documented stub on main). Completes March "v3.1 Memory
   Export/Import".
5. **On current main?** **No** for import.rs / ImportBackup / `memory import` /
   round-trip test. Service `lib.rs` on main has `federated` instead of
   `backup`/`import`.

---

## PRIMARY 4 — `origin/gsd/phase-57-opencode-converter-registration`  ← SKIP

1. **Tip** `ee2ff82401e84895573eeaa8762cfa5aafb086ab` — 2026-03-25 17:04:05 -0500 —
   `docs(58): create phase plan for Claude registration + plugin metadata`.
   **95 ahead / 20 behind.** Merge-base `9be18d8`. Unique vs phase-56: **7 commits**.
2. **Increment vs phase-56:**
   ```
   ee2ff82 docs(58): create phase plan for Claude registration + plugin metadata
   db9cb04 docs(58): research phase domain
   3d5d7e5 docs(58): generate context from codebase-mentor reference
   2737b3a docs(phase-57): complete phase execution
   94c654b docs(57-01): complete OpenCode converter plan
   578883d test(57-01): update E2E and integration tests for OpenCode converter
   f28793a feat(57-01): implement full OpenCode converter replacing stub
   ```
3. **Triple-dot vs main:** 134 files, +22981/−248 (non-planning 65, +7381/−64).
   Code increment vs phase-56 is **only installer**:
   - `crates/memory-installer/src/converters/opencode.rs` +779/− (stub → full converter)
   - `crates/memory-installer/src/converter.rs` +2/− (select OpenCode)
   - `crates/memory-installer/tests/e2e_converters.rs` +117/−
4. **Feature:** full OpenCode converter (OREG). Replaces the 49-line stub that
   already existed on the March line.
5. **On current main?** **No, and main deleted it on purpose.** `#36`
   (`feat(phase-57): shop window…`, 2026-08-30) removed
   `crates/memory-installer/src/converters/opencode.rs` ("delete the one
   converter that reported success while doing nothing"). Current main
   converters: claude, gemini, codex, copilot, skills — no OpenCode.

---

## PRIMARY 5 — `origin/gsd/phase-58-claude-registration-metadata`  ← CREG/META tip

1. **Tip** `4f26f49841f3f6a1041d82254d70dcf6827a069d` — 2026-03-25 17:17:07 -0500 —
   `docs(phase-58): complete phase execution`.
   **99 ahead / 20 behind.** Merge-base `9be18d8`. Unique vs phase-57: **4 commits**.
2. **Increment vs phase-57 (entire log of 4):**
   ```
   4f26f49 docs(phase-58): complete phase execution
   65b30fe docs(58-01): complete Claude registration metadata plan
   aedbfb9 feat(58-01): implement Claude Code registry registration in generate_guidance
   b806364 feat(58-01): create plugin.json and add chrono dependency
   ```
   Full `merge-base..tip` = 99 commits. Newest 15 = these 4 + phase-57's 7 +
   start of phase-56 docs. Oldest 10 = same v3.0 scaffold as phase-54.
3. **Triple-dot vs main:** 139 files, +23766/−272 (non-planning 68, +7955/−88).
   Highlight vs phase-57 / vs main:
   - `plugins/memory-query-plugin/.claude-plugin/plugin.json` +13 (**new**; name
     `memory-query`, version `1.0.0`)
   - `crates/memory-installer/src/converters/claude.rs` +584/− (`generate_guidance`
     writes `known_marketplaces.json`, `installed_plugins.json`,
     `settings.json` `enabledPlugins`; key `memory-query@agent-memory`; version
     from plugin.json; CREG-01..06 + META-03 tests)
   - `crates/memory-installer/Cargo.toml` +1 (`chrono`)
   - plus everything from 54–57, including **OpenCode converter**
4. **Feature:** Claude Code marketplace registration + plugin metadata (CREG/META).
   `ClaudeConverter::generate_guidance` (empty on main) emits registry files.
   `uninstall` is still only a comment in `writer.rs` ("Supports future
   `--uninstall`") — same as main. No `install-service` command on either side.
5. **On current main?** `claude.rs` **yes** (stub `generate_guidance` returns
   empty). `plugin.json` **no** (zero `plugin.json` files on main).
   `known_marketplaces` **no**. CREG/META tests **no**.

---

## Leftover branches (not the March nested line)

### `origin/gsd/phase-53-benchmark-suite`

- Tip `b1af44aba6692abc567a538da07db07a5f21a772` — 2026-03-23 14:36:14 -0500 —
  `docs(54): create phase plan for daily markdown export`.
- **55 ahead / 20 behind.** Merge-base `9be18d8`. **Ancestor of all five primaries.**
- Content: March v3.0 (orchestrator + CLI + bench + LOCOMO) plus v3.1 planning
  docs. **Already on main via PRs #28/#29/#30**, then superseded by Make It True
  (judge.rs, LOCOMO v2, honest percentiles). No unique feature vs current main.

### `origin/feature/phase-54-integration-truth`

- Tip `8d22cc136e7f2fe5e0aff6e3e9481db2fc05a187` — 2026-08-30 16:33:27 +0000 —
  `fix(clippy): use slice::fill in InFlightBuffer::clear`.
- Merge-base with main: `68ab122` (Phase 53 Benchmark Suite #30, 2026-05-14).
  **2 ahead / 8 behind.**
- Unique commits: `63b07a8 feat(phase-54): wire orchestrator…` and the clippy fix.
- Orchestrator patch-id of `63b07a8` **equals** `4e0e66a` (`feat(v3.1): Phase 54
  Integration Truth — wire orchestrator (#32)`). Work is on main under a
  different SHA. The clippy hunk is also already present on main (`git diff
  origin/main 8d22cc1 -- crates/memory-types/src/dedup.rs` is empty). Stale PR
  branch.

### `origin/claude/phase-54-toolchain-drift-3k4fer`

- Tip `2d205f151b1bfc38c7b181eef8e0a68a2208177e` — 2026-09-01 23:15:34 +0000 —
  `docs(plans): reconcile v3.2 plan with the March roadmap and v3 spec futures`.
- Merge-base = **current main** (`d6c8ac7`). **2 ahead / 0 behind.**
- Adds only `docs/plans/v3.2-prove-it-plan.md` (+367). Live unmerged docs branch
  sitting on today's main; **not** part of the March export/import line. Name is
  misleading (phase-54 here is Make It True, not daily-export).

### `origin/claude/spillwave-agent-memory-review-len4et`

- Tip `70699e10ebf70b3d0395d9951667402ba395bad6` — 2026-08-30 07:20:01 +0000 —
  `docs: v3.1 'Make It True' milestone design spec and phase plan`.
- Merge-base `68ab122`. **1 ahead / 8 behind.**
- Adds `docs/plans/v3.1-make-it-true-plan.md`. Landed on main as `#31`
  (`d937d1d`, same path). Duplicate.

---

## Conflict map (trial merge `--no-commit --no-ff` onto `origin/main`)

Throwaway worktrees at `/tmp/orphan-merge-{56,58}`, aborted and removed.

### phase-56-import-bootstrap vs main — **35 unmerged files**

**Content conflicts (UU):**
- `.github/workflows/ci.yml` (rustc 1.97 pin vs `stable`; bench-smoke `continue-on-error`)
- `.planning/{MILESTONES,PROJECT,REQUIREMENTS,ROADMAP,STATE}.md` (numbering collision)

**Add/add (AA) — independent crate creation after fork:**
- **Entire `crates/memory-bench/`** (10 files: Cargo.toml, baseline, cli, fixture,
  lib, locomo, main, report, runner, scorer). Main also has `judge.rs` (orphan does not).
- **Overlapping `crates/memory-cli/`:** Cargo.toml, cli.rs, main.rs,
  commands/{add,context,mod,search}.rs
- **Entire overlapping `crates/memory-orchestrator/`** except `expand.rs` (identical
  blob) and Cargo.toml (identical): context_builder, fusion, lib, orchestrator,
  rerank, types.
- `benchmarks/baselines.toml`, three fixture tomls, `download-locomo.sh`

**Content conflict (UU):**
- `crates/memory-client/src/client.rs` — main added `ingest`, `ingest_batch`,
  `route_query_ex`; orphan added `export_daily` / `export_backup` / `import_backup`.

**Auto-merged (clean) — this is the salvageable surface:**
- `proto/memory.proto` (append-only ExportDaily / ExportBackup / ImportBackup)
- `crates/memory-service/{Cargo.toml,ingest.rs,lib.rs,query.rs}`
- `crates/memory-service/src/backup.rs` (added)
- `crates/memory-service/src/import.rs` (added)
- `crates/memory-service/tests/import_round_trip.rs` (added)
- `crates/memory-cli/src/commands/{daily,backup,import}.rs` (added)
- `crates/memory-storage/src/{db.rs,episodes.rs}`
- `crates/memory-client/{Cargo.toml,src/lib.rs}`
- root `Cargo.toml`

### phase-58-claude-registration-metadata vs main — **38 unmerged files**

Everything in the phase-56 set, plus three installer conflicts:

| File | Kind | Why |
|---|---|---|
| `crates/memory-installer/src/converter.rs` | UU content | Orphan re-inserts `Runtime::OpenCode` into `select_converter` / tests; main's `#36` shop-window explicitly dropped OpenCode |
| `crates/memory-installer/src/converters/opencode.rs` | DU modify/delete | **Deleted on main**, modified on orphan (stub → 779-line converter) |
| `crates/memory-installer/tests/e2e_converters.rs` | UU content | OpenCode E2E |

**Auto-merged on phase-58 (keep):**
- `crates/memory-installer/src/converters/claude.rs` — CREG/META **applies cleanly**
  onto current main's claude converter
- `plugins/memory-query-plugin/.claude-plugin/plugin.json` — added, no conflict
- `crates/memory-installer/Cargo.toml` (`chrono`)

### memory-orchestrator / memory-bench divergence (do not take orphan copies)

| Crate | main @ d6c8ac7 | March line (p54–p58 identical) |
|---|---|---|
| `memory-orchestrator` | 8 files, **1458 lines**. Wired in v3.1 Integration Truth (#32): rerank 383, orchestrator 519, fusion 188. | 8 files, **878 lines**. expand.rs **identical** to main. types.rs trivial rename. Everything else is the pre-wiring snapshot. |
| `memory-bench` | 11 files, **2649 lines**, includes `judge.rs` (294) + LOCOMO v2 (`locomo.rs` 682, `runner.rs` 458). | 10 files, **1319 lines**, no judge, original LOCOMO adapter. |

Taking either crate from the orphan line would **regress** main.

---

## What main already has vs what only exists on the orphan line

Grep of `origin/main` and `origin/gsd/phase-58-claude-registration-metadata`
(excluding `*.md` / `.planning`).

| Topic | Main (`d6c8ac7`) | Orphan line (p58) |
|---|---|---|
| **Streaming RPCs** | None. No `rpc Export*` / `rpc Import*`. No `stream BackupChunk` / `stream ImportChunk`. | `ExportBackup` server-streaming, `ImportBackup` client-streaming, plus unary `ExportDaily`. Comment in proto: "first streaming RPC". |
| **export** | No `ExportDaily` / `export_daily` / `memory daily`. | `proto` messages + `query.rs:export_daily` + `commands/daily.rs` |
| **import / bootstrap** | No `ImportBackup` / `import.rs` / `memory import`. Word "bootstrap" does not appear in non-doc code. | `import.rs` + CLI + round-trip test. March v3.1 "import/bootstrap". |
| **JSONL backup** | No `backup.rs` / `ExportBackup` / `memory backup`. | `memory-service/src/backup.rs` (308) + CLI backup (308) |
| **known_marketplaces** | Absent. | `claude.rs:build_known_marketplaces` → `~/.claude/plugins/known_marketplaces.json` (CREG-01) |
| **plugin.json** | **Zero files.** Docs mention the path as a future layout (`docs/plans/v2.7-…`, authoring-guide). | `plugins/memory-query-plugin/.claude-plugin/plugin.json` only on **p58** |
| **uninstall** | Comment in `writer.rs` ("future `--uninstall`") + `install-helper.sh uninstall()`. | Same. No new uninstall implementation. |
| **install-service** | **Absent both sides.** | **Absent.** |
| **rebuild-toc** | `memory-daemon admin rebuild-toc` **exists and exits non-zero**: "offline TOC rebuild is not implemented; TOC nodes are produced by the daemon's scheduled rollup jobs". CHANGELOG + README status table match. Import CLI on the orphan tells the user to run it after restore. | Same stub (older daemon). |
| **rebuild-bm25** | Exists; CHANGELOG: "relabelled: it prunes documents below `--min-level` and re-indexes nothing". | Same command exists (pre-relabel). |
| **OpenCode converter** | **Deleted** in `#36` shop-window. | Full converter on p57/p58; stub on p54–p56. |
| **Claude generate_guidance** | Returns empty vec (test `generate_guidance_returns_empty`). | Emits known_marketplaces + installed_plugins + settings (CREG/META). |

File existence (yes/NO):

```
file                                               main p54 p55 p56 p57 p58
commands/daily.rs                                   NO  yes yes yes yes yes
commands/backup.rs                                  NO   NO yes yes yes yes
commands/import.rs                                  NO   NO  NO yes yes yes
service/backup.rs                                   NO   NO yes yes yes yes
service/import.rs                                   NO   NO  NO yes yes yes
tests/import_round_trip.rs                          NO   NO  NO yes yes yes
converters/opencode.rs                              NO  yes yes yes yes yes
converters/claude.rs                               yes  yes yes yes yes yes
plugin.json                                         NO   NO  NO  NO  NO yes
memory-opencode-plugin/README.md                    NO  yes yes yes yes yes
2026-03-23-memory-export-import-design.md           NO  yes yes yes yes yes
bench/judge.rs                                     yes   NO  NO  NO  NO  NO
service/federated.rs                               yes   NO  NO  NO  NO  NO
```

Main CLI `Commands` enum: Search, Add, Recall, Context, Timeline, Summary.
Orphan p56+ adds Daily, Backup, Import. recall.rs / summary.rs / timeline.rs
blobs are **identical** on main and p56.

---

## Recommendation (cherry-pick by feature; keep branches)

Two slices. Do **not** merge any `gsd/phase-*` branch wholesale (add/add on
orchestrator + bench would regress v3.1 Make It True). Skip all `.planning/`
(phase-number collision with current v3.1/v3.2).

### A. Export/import foundation (backfill) — take from phase-56, not later

Cherry-pick these **feature** commits (skip docs + skip `acbd6ae` workspace
fmt/clippy, which can retouch orchestrator/bench):

```
# phase 54 — daily markdown
1e29127 feat(54-01): add ExportDaily proto messages and RPC
408003b feat(54-01): implement ExportDaily handler, trait dispatch, and client method
5be2c19 feat(54-02): add `memory daily` CLI subcommand with markdown rendering

# phase 55 — JSONL backup / first streaming RPC
858532b feat(55-01): add ExportBackup proto definitions, tokio-stream dep, and storage iteration methods
2f2b148 feat(55-01): add streaming backup handler with service wiring
bf51772 feat(55-02): add memory backup CLI command with streaming client

# phase 56 — import/bootstrap
2c737a9 feat(56-01): add ImportBackup client-streaming RPC to proto
b6c7935 feat(56-01): add import handler, service wiring, and client method
ea9ee1d feat(56-02): add memory import CLI command with manifest validation
53fa9fc test(56-02): add round-trip integration tests for import handler
```

Expected conflict files while cherry-picking onto current main (the rest of
those commits should apply or add new files):

- `crates/memory-client/src/client.rs` — **will conflict**; keep main's
  `ingest` / `route_query_ex`, add the three export/import methods.
- `crates/memory-cli/src/{cli.rs,main.rs,commands/mod.rs}` — **will conflict**
  (independent CLI history); add Daily/Backup/Import variants next to existing
  Search/Add/Recall. New files `daily.rs` / `backup.rs` / `import.rs` should add clean.
- `proto/memory.proto` — likely **clean** (append after `GetSimilarEpisodes`).
- `memory-service/{ingest.rs,lib.rs,query.rs}` + new `backup.rs`/`import.rs` —
  likely **clean** (auto-merged in the trial merge).
- `memory-storage/{db.rs,episodes.rs}` — likely **clean**.

Do not take March `memory-orchestrator` or `memory-bench`. After import, the
CLI currently tells the user to run `memory admin rebuild-toc`; that command
is a documented non-implementation on main, so the backfill story needs a
follow-up (scheduled rollup, or actually implement rebuild-toc).

### B. Claude Code registration + plugin metadata (CREG/META) — take from phase-58, skip 57

```
b806364 feat(58-01): create plugin.json and add chrono dependency
aedbfb9 feat(58-01): implement Claude Code registry registration in generate_guidance
```

Trial merge showed **`claude.rs` auto-merges** and `plugin.json` is a clean add.
`chrono` on installer Cargo.toml also auto-merged.

### C. Skip entirely

- All of phase-57 (`f28793a`, `578883d`, and the OpenCode docs). Restoring
  OpenCode would fight `#36` (modify/delete on `opencode.rs`, content conflict
  on `converter.rs` + e2e).
- `gsd/phase-53-benchmark-suite` — already on main, older.
- `feature/phase-54-integration-truth` — already `#32`.
- `claude/spillwave-agent-memory-review-len4et` — already `#31`.
- `claude/phase-54-toolchain-drift-3k4fer` is a **current** docs PR for
  `docs/plans/v3.2-prove-it-plan.md`; not March-line salvage. Handle separately
  if that v3.2 plan is wanted.

Suggested order on a new branch off current main: **A (54→55→56 feature
commits), then B (58-01 plugin.json + generate_guidance).** Keep all listed
remote branches.

