# Phase 45: Canonical Source Consolidation - Research

**Researched:** 2026-03-16
**Domain:** Claude plugin format, YAML frontmatter, plugin manifest, multi-source installer discovery
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Keep `memory-query-plugin/` and `memory-setup-plugin/` as separate directories
- Installer reads from both — no merge into single `plugins/memory-plugin/`
- Zero migration risk — existing plugins continue working as Claude plugins
- Installer parser must support reading from multiple plugin source directories
- Skip hook canonicalization in Phase 45 — that's Phase 49 work
- Phase 45 only consolidates commands, agents, and skills
- Existing hook implementations stay in the adapter directories
- Leave hand-written adapters (copilot, gemini, opencode) in place until Phase 50

### Claude's Discretion
- Whether to add a manifest file listing both plugin source directories for the installer
- Plugin.json format for the consolidated canonical reference
- Any cleanup of SKILL.md files or reference docs during consolidation

### Deferred Ideas (OUT OF SCOPE)
- Hook canonicalization — Phase 49
- Adapter archival — Phase 50
- Single merged plugin directory — not needed (installer handles multi-source)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CANON-01 | Canonical plugin source tree merges query+setup plugins into single `plugins/memory-plugin/` directory | USER DECISION OVERRIDES: keep both dirs separate. CANON-01 reinterpretation: the canonical source IS both existing plugin directories; they are together the canonical source. No file movement required. |
| CANON-02 | Canonical hook definitions in YAML format capture all event types across runtimes | DEFERRED to Phase 49 per locked decisions. Phase 45 does not implement hook YAML. |
| CANON-03 | All 6 commands, 2 agents, 13 skills consolidated with no content loss | Research confirms all 6 commands, 2 agents, 13 skills exist with consistent YAML frontmatter. Consolidation = audit, normalize, and add manifest — no content movement. |
</phase_requirements>

---

## Summary

Phase 45 prepares the two existing Claude plugin directories (`memory-query-plugin/` and `memory-setup-plugin/`) as the authoritative canonical source for the Phase 46 installer parser. The user decision to keep both directories separate reinterprets CANON-01: the "canonical source" is the pair of existing directories, not a new merged directory. This makes Phase 45 primarily an audit-and-canonicalize operation.

The good news: the existing source files are in excellent shape. All 13 SKILL.md files use identical `name`, `description`, `license`, `metadata.version`, `metadata.author` frontmatter fields with no outliers. All 6 command files use identical `name`, `description`, `parameters[]`, `skills[]` frontmatter. Both agent files use `name`, `description`, `triggers[]`, `skills[]` — no `allowed-tools` or `color` fields exist yet in the Claude plugin format (those are converter concerns for later phases). There are no YAML frontmatter inconsistencies to fix.

The main gaps are: (1) both plugin directories have `marketplace.json` but no `plugin.json` — the CONTEXT.md references a `plugin.json` that does not yet exist and that Phase 46 will need for discovery; (2) there is no manifest file telling the installer where the two plugin source directories are; (3) the REQUIREMENTS.md still says CANON-01 means "merge into single directory" which conflicts with the locked CONTEXT.md decision — the plan must resolve this discrepancy by documenting the reinterpretation.

**Primary recommendation:** Phase 45 delivers two tasks: (1) create a `plugins/installer-sources.json` manifest that lists both plugin directories for the Phase 46 parser, and (2) add `plugin.json` to each plugin's `.claude-plugin/` directory in the format Phase 46 will consume. No file content needs to move or be rewritten.

---

## Current State Audit

### File Inventory (verified by filesystem scan)

**memory-query-plugin/** (5 skills, 3 commands, 1 agent)
```
.claude-plugin/marketplace.json     ← exists; plugin.json does NOT exist
agents/memory-navigator.md
commands/memory-context.md
commands/memory-recent.md
commands/memory-search.md
skills/bm25-search/SKILL.md + references/command-reference.md
skills/memory-query/SKILL.md + references/command-reference.md
skills/retrieval-policy/SKILL.md + references/command-reference.md
skills/topic-graph/SKILL.md + references/command-reference.md
skills/vector-search/SKILL.md + references/command-reference.md
```

**memory-setup-plugin/** (8 skills, 3 commands, 1 agent)
```
.claude-plugin/marketplace.json     ← exists; plugin.json does NOT exist
agents/setup-troubleshooter.md
commands/memory-config.md
commands/memory-setup.md
commands/memory-status.md
skills/memory-agents/SKILL.md + references/(3 files)
skills/memory-configure/SKILL.md
skills/memory-install/SKILL.md
skills/memory-llm/SKILL.md + references/(5 files)
skills/memory-setup/SKILL.md + references/(6 files) + scripts/install-helper.sh
skills/memory-storage/SKILL.md + references/(4 files)
skills/memory-troubleshoot/SKILL.md
skills/memory-verify/SKILL.md
```

### Frontmatter Consistency Analysis

**SKILL.md frontmatter — all 13 files:**
| Field | Present in all 13? | Notes |
|-------|--------------------|-------|
| `name` | YES | Matches directory name |
| `description` | YES | Multi-line block scalar |
| `license` | YES | All `MIT` |
| `metadata.version` | YES | All `1.0.0` except memory-query (`2.0.0`) |
| `metadata.author` | YES | All `SpillwaveSolutions` |

**Result: FULLY CONSISTENT. No normalization needed.**

**Command frontmatter — all 6 files:**
| Field | Present in all 6? | Notes |
|-------|-------------------|-------|
| `name` | YES | Matches filename stem |
| `description` | YES | Single-line string |
| `parameters[]` | YES | All have at least one param |
| `parameters[].name` | YES | |
| `parameters[].description` | YES | |
| `parameters[].required` | YES | |
| `parameters[].default` | PARTIAL | memory-context has it; memory-setup/status use `type: flag` instead |
| `parameters[].type` | PARTIAL | Only flag-type params (memory-setup, memory-status) use this |
| `skills[]` | YES | All reference exactly 1 skill |

**Result: CONSISTENT for the fields that matter. The `default` and `type` field variations are correct domain usage (flags vs value params), not inconsistencies. No normalization needed.**

**Agent frontmatter — both files:**
| Field | Present in both? | Notes |
|-------|-----------------|-------|
| `name` | YES | |
| `description` | YES | |
| `triggers[]` | YES | Array of pattern/type objects |
| `triggers[].pattern` | YES | |
| `triggers[].type` | YES | Both use `message_pattern` |
| `skills[]` | YES | |
| `allowed-tools` | NO | Not present in canonical Claude format |
| `color` | NO | Not present in canonical Claude format |

**Important finding:** The CONTEXT.md code insights mention `allowed-tools` and `color` as established patterns for agents. These fields do NOT exist in the canonical Claude plugin agents. They exist in the OpenCode adapter (`tools` object) and Copilot adapter. This is not a problem — these are converter concerns for Phases 47-49. The canonical format is correct as-is.

**Result: FULLY CONSISTENT. No normalization needed.**

### Manifest Gap Analysis

Both plugin directories use `marketplace.json` in `.claude-plugin/`. Neither has a `plugin.json`. The CONTEXT.md references `plugin.json` as an "existing manifest format" — this is a forward reference to what needs to be created. The `marketplace.json` format is:

```json
{
  "name": "...",
  "owner": { "name": "...", "email": "..." },
  "metadata": { "description": "...", "version": "..." },
  "plugins": [{ "name": "...", "source": "./", "strict": false,
                "skills": [...], "commands": [...], "agents": [...] }]
}
```

This format lists all asset paths already. The Phase 46 parser can use `marketplace.json` directly for discovery — it already contains `skills`, `commands`, `agents` arrays with relative paths.

**Decision point (Claude's Discretion):** Should Phase 46 consume `marketplace.json` directly, or should we add a `plugin.json` with a simpler structure? Recommendation: use `marketplace.json` directly — it already has everything needed. Adding a separate `plugin.json` would be duplication.

### Installer Discovery Gap

There is no file that tells the installer "here are the two plugin source directories." Phase 46 needs a way to find both `memory-query-plugin/` and `memory-setup-plugin/`. Options:

1. **Manifest file** (`plugins/installer-sources.json`) — lists both dirs, simple JSON
2. **Convention** — installer scans `plugins/` dir for subdirs with `.claude-plugin/marketplace.json`
3. **Hardcoded** — installer knows both paths at compile time (fragile)

**Recommendation:** Convention-based discovery. The installer (Phase 46) walks `plugins/` and collects any subdirectory containing `.claude-plugin/marketplace.json`. This requires zero new files in Phase 45 and is robust to future additions. Phase 45's job is to ensure both existing plugin directories are properly discoverable via this convention — which they already are.

**Alternative:** If the Phase 46 parser team prefers an explicit manifest, create `plugins/installer-sources.json`:
```json
{
  "version": "1",
  "sources": [
    "./memory-query-plugin",
    "./memory-setup-plugin"
  ]
}
```
This is the discretionary manifest option from CONTEXT.md. Either approach is valid; the planner should pick one.

---

## Standard Stack

### Core (existing project dependencies)
| Tool/Format | Version | Purpose | Why Standard |
|-------------|---------|---------|--------------|
| YAML frontmatter | Claude plugin spec | Commands, agents, skills metadata | Established project format |
| `marketplace.json` | Project convention | Plugin manifest and asset listing | Already used in both plugins |
| `---` delimited YAML | CommonMark + YAML | Frontmatter in .md files | Used by all 6 commands, 2 agents, 13 skills |

### Supporting (for Phase 46 to consume)
| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| `gray_matter` (Rust crate) | 0.3.2 | Frontmatter parsing | Phase 46 parser reads these files |
| `walkdir` (Rust crate) | 2.5 | Directory traversal | Phase 46 parser discovers plugin dirs |

*(These are Phase 46 concerns noted here for context only — Phase 45 produces the static files.)*

---

## Architecture Patterns

### Recommended Source Layout (Confirmed Existing)
```
plugins/
├── memory-query-plugin/
│   ├── .claude-plugin/
│   │   └── marketplace.json    (plugin manifest — Phase 46 discovery anchor)
│   ├── agents/
│   │   └── memory-navigator.md
│   ├── commands/
│   │   ├── memory-context.md
│   │   ├── memory-recent.md
│   │   └── memory-search.md
│   └── skills/
│       ├── bm25-search/SKILL.md + references/
│       ├── memory-query/SKILL.md + references/
│       ├── retrieval-policy/SKILL.md + references/
│       ├── topic-graph/SKILL.md + references/
│       └── vector-search/SKILL.md + references/
└── memory-setup-plugin/
    ├── .claude-plugin/
    │   └── marketplace.json    (plugin manifest — Phase 46 discovery anchor)
    ├── agents/
    │   └── setup-troubleshooter.md
    ├── commands/
    │   ├── memory-config.md
    │   ├── memory-setup.md
    │   └── memory-status.md
    └── skills/
        ├── memory-agents/SKILL.md + references/
        ├── memory-configure/SKILL.md
        ├── memory-install/SKILL.md
        ├── memory-llm/SKILL.md + references/
        ├── memory-setup/SKILL.md + references/ + scripts/
        ├── memory-storage/SKILL.md + references/
        ├── memory-troubleshoot/SKILL.md
        └── memory-verify/SKILL.md
```

### Pattern: Command YAML Frontmatter (canonical)
**What:** All commands use this exact field set
**When to use:** Adding new commands to canonical source
```yaml
# Source: plugins/memory-query-plugin/commands/memory-search.md
---
name: memory-search
description: Search past conversations by topic or keyword
parameters:
  - name: topic
    description: Topic or keyword to search for
    required: true
  - name: period
    description: Time period to search (e.g., "last week", "january", "2026")
    required: false
skills:
  - memory-query
---
```

### Pattern: SKILL.md Frontmatter (canonical)
**What:** All 13 skills use this exact field set
**When to use:** Adding new skills to canonical source
```yaml
# Source: plugins/memory-query-plugin/skills/memory-query/SKILL.md
---
name: memory-query
description: |
  [multi-line description with "Use when..." trigger phrases]
license: MIT
metadata:
  version: 1.0.0
  author: SpillwaveSolutions
---
```

### Pattern: Agent Frontmatter (canonical)
**What:** Both agents use this exact field set; no `allowed-tools` or `color`
**When to use:** Adding new agents to canonical source
```yaml
# Source: plugins/memory-query-plugin/agents/memory-navigator.md
---
name: memory-navigator
description: [single-line description]
triggers:
  - pattern: "regex pattern"
    type: message_pattern
skills:
  - skill-name
---
```

### Pattern: marketplace.json (plugin discovery manifest)
**What:** Existing format used by both plugins; Phase 46 parser will walk this
```json
{
  "name": "memory-query-agentic-plugin",
  "owner": { "name": "SpillwaveSolutions", "email": "rick@spillwave.com" },
  "metadata": { "description": "...", "version": "2.0.0" },
  "plugins": [{
    "name": "memory-query",
    "source": "./",
    "strict": false,
    "skills": ["./skills/memory-query", "./skills/retrieval-policy", ...],
    "commands": ["./commands/memory-search.md", ...],
    "agents": ["./agents/memory-navigator.md"]
  }]
}
```

### Anti-Patterns to Avoid
- **Moving files for the sake of consolidation:** The user decision locks the directory structure. Phase 45 is audit + enrich, not migrate.
- **Creating a third "merged" directory:** Explicitly deferred — adds migration risk for zero benefit since the installer handles multi-source.
- **Touching hook files:** All hook implementations (Gemini settings.json, Copilot hooks.json, OpenCode plugin TS) are deferred to Phase 49. Do not modify them in Phase 45.
- **Modifying the adapter directories:** Copilot, Gemini, OpenCode adapters stay untouched until Phase 50.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Plugin asset discovery | Custom scanner | `marketplace.json` existing paths array | Already has `skills`, `commands`, `agents` arrays |
| Frontmatter validation | Custom YAML checker | Eye inspection is sufficient for Phase 45 | All 13+6+2 files verified consistent by this research |
| Manifest format design | New JSON schema | Reuse `marketplace.json` structure | Phase 46 parser already has a format to parse |

---

## Common Pitfalls

### Pitfall 1: CANON-01 Literal Interpretation
**What goes wrong:** Planner implements a file migration (merging both plugins into `plugins/memory-plugin/`) because REQUIREMENTS.md says "merges query+setup plugins into single directory."
**Why it happens:** REQUIREMENTS.md was written before the CONTEXT.md discussion that locked the "keep separate" decision.
**How to avoid:** CONTEXT.md decisions override REQUIREMENTS.md wording. Phase 45 reinterprets CANON-01 as "both existing directories together constitute the canonical source." Document this reinterpretation in the plan.
**Warning signs:** Any task that moves files from `memory-query-plugin/` or `memory-setup-plugin/` into a new location.

### Pitfall 2: CANON-02 Hook Work in Phase 45
**What goes wrong:** Implementing canonical hook YAML files as part of Phase 45.
**Why it happens:** REQUIREMENTS.md assigns CANON-02 to Phase 45, but CONTEXT.md explicitly defers hook canonicalization to Phase 49.
**How to avoid:** CONTEXT.md is authoritative. Mark CANON-02 as deferred. Phase 45 notes in its deliverable that hooks are Phase 49 work.
**Warning signs:** Any task creating `.yaml` or `.yml` hook definition files.

### Pitfall 3: plugin.json Confusion
**What goes wrong:** Treating `marketplace.json` and `plugin.json` as the same thing, or creating a redundant `plugin.json` alongside `marketplace.json`.
**Why it happens:** CONTEXT.md references `plugin.json` as "existing manifest format" but the filesystem only has `marketplace.json`.
**How to avoid:** Use `marketplace.json` as the discovery anchor. If Phase 46 needs a `plugin.json`, it should be created in Phase 45 as a new file with a distinct purpose (installer metadata, not Claude marketplace metadata). Recommendation: skip creating `plugin.json` and have Phase 46 consume `marketplace.json` directly.
**Warning signs:** Confusion about which manifest file Phase 46 reads.

### Pitfall 4: Missing `skills` Field Count
**What goes wrong:** Assuming CANON-03 ("13 skills consolidated") requires adding 4 setup skills to the query plugin or vice versa.
**Why it happens:** CANON-03 says "consolidated" but the split is intentional.
**How to avoid:** 13 skills = 5 in query-plugin + 8 in setup-plugin. They are already consolidated across both directories. No cross-pollination needed.
**Warning signs:** Any plan that moves skills between the two plugin directories.

### Pitfall 5: Agent Frontmatter Field Gap
**What goes wrong:** Adding `allowed-tools` or `color` fields to the canonical Claude agent files to "prepare" them for converter use.
**Why it happens:** CONTEXT.md mentions these as "established patterns" but they are actually converter OUTPUT fields, not canonical input fields.
**How to avoid:** The Claude plugin format does not use `allowed-tools` or `color`. These are converter-generated fields (OpenCode uses `tools: object`, Gemini strips `color`). Do not add them to canonical source.
**Warning signs:** Any task that edits agent frontmatter to add `allowed-tools` or `color`.

---

## Code Examples

### Verified: command frontmatter (all 6 consistent)
```yaml
# Source: plugins/memory-setup-plugin/commands/memory-status.md
---
name: memory-status
description: Check health and status of agent-memory installation
parameters:
  - name: verbose
    description: Show detailed diagnostics
    required: false
    type: flag
  - name: json
    description: Output in JSON format
    required: false
    type: flag
skills:
  - memory-setup
---
```

### Verified: SKILL.md frontmatter (all 13 consistent)
```yaml
# Source: plugins/memory-setup-plugin/skills/memory-install/SKILL.md
---
name: memory-install
description: |
  Wizard-style installation guide for agent-memory (macOS/Linux). Helps users
  choose an install path, check prerequisites, and set PATH. Always confirm
  before any file edits or copying binaries. Provide verification commands only.
license: MIT
metadata:
  version: 1.0.0
  author: SpillwaveSolutions
---
```

### Verified: agent frontmatter (both agents consistent)
```yaml
# Source: plugins/memory-setup-plugin/agents/setup-troubleshooter.md
---
name: setup-troubleshooter
description: Autonomous agent for diagnosing and fixing agent-memory issues
triggers:
  - pattern: "(memory|daemon).*(not working|broken|failing|error)"
    type: message_pattern
  [... 9 more triggers ...]
skills:
  - memory-setup
---
```

### Optional: installer-sources.json (if explicit manifest preferred)
```json
{
  "version": "1",
  "description": "Canonical plugin source directories for memory-installer",
  "sources": [
    "./memory-query-plugin",
    "./memory-setup-plugin"
  ]
}
```
*Location: `plugins/installer-sources.json` — only create if Phase 46 team prefers explicit over convention-based discovery.*

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single merged plugin directory (original CANON-01 intent) | Two separate directories as joint canonical source | Phase 45 CONTEXT.md decision | Zero migration risk; existing Claude plugins continue working |
| `plugin.json` manifest (referenced in CONTEXT.md) | `marketplace.json` (what actually exists) | Pre-existing | Phase 46 should parse `marketplace.json` directly |

---

## Open Questions

1. **Does Phase 46 parser prefer `marketplace.json` or a new `plugin.json`?**
   - What we know: `marketplace.json` already has all asset paths; it's sufficient for discovery
   - What's unclear: Phase 46 may have been designed expecting a simpler `plugin.json` format
   - Recommendation: Phase 45 plan notes that `marketplace.json` is the discovery anchor, but creates a minimal `plugin.json` if the CONTEXT.md discretion item is resolved in favor of an explicit manifest

2. **Does CANON-01 need a formal reinterpretation in writing?**
   - What we know: CONTEXT.md locks "keep both dirs separate" which conflicts with CANON-01 wording in REQUIREMENTS.md
   - What's unclear: Whether a formal written reinterpretation is needed or if the plan notes are sufficient
   - Recommendation: Plan notes section should document the reinterpretation explicitly so Phase 46+ teams understand the intent

3. **Are any reference docs missing or outdated?**
   - What we know: 13 SKILL.md files have correct content; reference files exist for the 7 skills that have them
   - What's unclear: Whether reference docs reflect v2.6 Cognitive Retrieval features (memory-query skill is v2.0.0)
   - Recommendation: Out of scope for Phase 45 unless a specific reference doc is factually incorrect; version numbers are advisory

---

## Validation Architecture

> `workflow.nyquist_validation` is `true` in `.planning/config.json` — include this section.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (Rust workspace) |
| Config file | Cargo.toml (workspace root) |
| Quick run command | `cargo test --workspace` |
| Full suite command | `task pr-precheck` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CANON-01 | Both plugin dirs exist with valid marketplace.json | manual | `ls plugins/memory-{query,setup}-plugin/.claude-plugin/marketplace.json` | ✅ |
| CANON-02 | DEFERRED to Phase 49 | — | — | — |
| CANON-03 | 6 commands + 2 agents + 13 skills present with YAML frontmatter | manual | `find plugins/memory-{query,setup}-plugin -name "*.md" -path "*/commands/*" \| wc -l && find plugins/memory-{query,setup}-plugin -name "SKILL.md" \| wc -l` | ✅ |

**Note:** Phase 45 produces static files (no new Rust code). Validation is manual inspection and file-count checks. No new test files needed.

### Sampling Rate
- **Per task commit:** `git status && ls plugins/memory-{query,setup}-plugin/.claude-plugin/`
- **Per wave merge:** `find plugins/memory-{query,setup}-plugin -name "*.md" | wc -l`
- **Phase gate:** Manual content audit of all files before `/gsd:verify-work`

### Wave 0 Gaps
None — existing test infrastructure covers all phase requirements. Phase 45 has no Rust code changes.

---

## Sources

### Primary (HIGH confidence)
- Filesystem scan of `plugins/memory-query-plugin/` — complete file inventory
- Filesystem scan of `plugins/memory-setup-plugin/` — complete file inventory
- Direct frontmatter extraction from all 6 command files, 2 agent files, 13 SKILL.md files
- `plugins/memory-query-plugin/.claude-plugin/marketplace.json` — manifest format
- `plugins/memory-setup-plugin/.claude-plugin/marketplace.json` — manifest format
- `.planning/phases/45-canonical-source-consolidation/45-CONTEXT.md` — locked decisions
- `.planning/REQUIREMENTS.md` — requirement IDs and descriptions
- `docs/plans/v2.7-multi-runtime-portability-plan.md` — implementation context

### Secondary (MEDIUM confidence)
- Cross-adapter comparison: copilot (`plugin.json`), gemini (`settings.json`), opencode (TypeScript plugin) — hook format reference for what Phase 49 must produce

---

## Metadata

**Confidence breakdown:**
- Current file state audit: HIGH — verified by direct filesystem scan and frontmatter extraction
- Frontmatter consistency: HIGH — all fields extracted from all 21 files (6+2+13)
- Manifest gap (plugin.json): HIGH — confirmed absent by find command
- Installer discovery recommendation: MEDIUM — based on analysis of existing formats; Phase 46 team may have different constraints
- CANON-01 reinterpretation: HIGH — locked by CONTEXT.md which overrides REQUIREMENTS.md

**Research date:** 2026-03-16
**Valid until:** Stable (file formats are locked; valid until next milestone)
