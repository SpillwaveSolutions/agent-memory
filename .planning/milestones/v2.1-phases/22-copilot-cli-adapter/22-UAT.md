---
status: complete
phase: 22-copilot-cli-adapter
source: 22-01-SUMMARY.md, 22-02-SUMMARY.md, 22-03-SUMMARY.md
started: 2026-02-10T19:00:00Z
updated: 2026-02-10T19:05:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Hook script handles all 5 event types with fail-open behavior
expected: Run simulated events through hook script (sessionStart, userPromptSubmitted, preToolUse with toolArgs, empty input, invalid JSON). All exit 0 with no stdout output. Fail-open on malformed input.
result: pass

### 2. Session ID synthesis via temp files
expected: Run sessionStart, check that a temp file is created at `/tmp/copilot-memory-session-*` keyed by CWD hash. Run sessionStart again with same CWD — should reuse the existing session file (Bug #991 handling). Run sessionEnd with `{"timestamp":1707580800000,"cwd":"/tmp","reason":"user_exit"}` — session temp file should be cleaned up.
result: pass

### 3. ANSI stripping handles OSC sequences
expected: CSI color codes and OSC hyperlinks stripped before JSON parsing. Both exit 0.
result: pass

### 4. Sensitive field redaction in payloads
expected: api_key in toolArgs is redacted from the payload sent to memory-ingest (walk filter with jq<1.6 fallback).
result: pass

### 5. memory-hooks.json is valid and registers 5 events
expected: Valid JSON with sessionStart, sessionEnd, userPromptSubmitted, preToolUse, postToolUse. Each references memory-capture.sh.
result: pass

### 6. Skills have YAML frontmatter and are in .github/skills/
expected: 5 skill directories with SKILL.md + references/command-reference.md.
result: pass

### 7. memory-query skill includes command-equivalent instructions
expected: Multiple memory-daemon references, search/recent/context operations, 474 lines.
result: pass

### 8. Navigator agent is a proper .agent.md file with infer:true
expected: YAML frontmatter with infer: true, tier routing, intent classification, 249 lines.
result: pass

### 9. plugin.json manifest for /plugin install
expected: Valid JSON with name, version (2.1.0), description.
result: pass

### 10. Install skill with prerequisites check and per-project setup
expected: 414 lines, prerequisites check, hook copying (NOT settings.json merge), verification, uninstall.
result: pass

### 11. README documents gaps, installation paths, and troubleshooting
expected: 448 lines, AssistantResponse gap, sessionStart bug, 3 install paths, adapter comparison, troubleshooting.
result: pass

### 12. No TOML command files exist (Copilot uses skills only)
expected: No .toml files — Copilot uses skills for all query operations.
result: pass

## Summary

total: 12
passed: 12
issues: 0
pending: 0
skipped: 0

## Gaps

[none]
