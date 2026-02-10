---
status: findings-captured
phase: 21-gemini-cli-adapter
source: user review (2026-02-10)
started: 2026-02-10
updated: 2026-02-10
---

## Review Findings (Post-Execution)

These issues were identified during post-execution review. They do not block Phase 21 completion but should be addressed in a gap closure pass.

### Finding 1: jq `walk` requires 1.6+ (silent fail-open on older jq)

**File:** `plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh:74`
**Severity:** Medium — silent data loss on older systems
**Description:** The capture script uses `jq walk(...)` for redaction without checking jq version. On systems with jq 1.5 (common on older macOS/Homebrew), the script will quietly fail-open and drop all events (still returns `{}`), giving the illusion of success.
**Fix:** Add a lightweight jq version check at script startup. If jq < 1.6, use a fallback redaction approach (e.g., simple `del()` on known keys instead of recursive `walk`).

### Finding 2: ANSI stripping is partial (misses OSC sequences)

**File:** `plugins/memory-gemini-adapter/.gemini/hooks/memory-capture.sh:52`
**Severity:** Medium — can corrupt JSON and trigger fail-open path
**Description:** The `sed` pattern only removes CSI sequences ending in a letter and will miss OSC hyperlinks (`ESC]...ST`) or other escape forms that Gemini CLI may emit, which can still corrupt JSON and trigger the fail-open path.
**Fix:** Use a broader strip pattern, e.g., `perl -pe 's/\e\[[0-9;]*[A-Za-z]|\e\].*?(\a|\e\\)//g'` or a JSON-safe parser approach.

### Finding 3: Per-project installs point to wrong hook path

**File:** `plugins/memory-gemini-adapter/.gemini/settings.json`
**Severity:** Low-Medium — silent failure on per-project installs
**Description:** Default `settings.json` hardcodes `$HOME/.gemini/hooks/memory-capture.sh`. For per-project installs, the README tells users to edit paths manually, but the install skill/automation doesn't rewrite them. This can leave project installs calling a non-existent hook and silently failing.
**Fix:** Auto-rewrite hook paths in the install skill when `--project` flag is used. Detect install mode and adjust `command` field in settings.json accordingly.

## Status

- [x] Findings captured
- [x] Gap closure planned (21-04-PLAN.md)
- [x] Fixes applied (21-04-SUMMARY.md, commits cc195f4 + 753bc2f)

## Notes

These findings were also fed forward into Phase 22 (Copilot CLI Adapter) planning as "lessons learned" — the Copilot adapter plans already include jq version checks, broader ANSI stripping, and per-project path handling.
