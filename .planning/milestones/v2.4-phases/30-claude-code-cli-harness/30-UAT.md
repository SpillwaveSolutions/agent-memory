---
status: testing
phase: 30-claude-code-cli-harness
source: 30-01-SUMMARY.md, 30-02-SUMMARY.md, 30-03-SUMMARY.md, 30-04-SUMMARY.md, 30-05-SUMMARY.md, 30-06-SUMMARY.md
started: 2026-02-23T20:45:00Z
updated: 2026-02-23T20:45:00Z
---

## Current Test

number: 1
name: Smoke tests pass (binary detection + fail-open ingest)
expected: |
  Run `bats tests/cli/claude-code/smoke.bats` after building binaries.
  Expected: 6+ tests pass (daemon binary exists, ingest binary exists, daemon healthy, valid/malformed/empty JSON returns continue:true). Tests 7-8 may skip if claude CLI is not installed.
awaiting: user response

## Tests

### 1. Smoke tests pass (binary detection + fail-open ingest)
expected: Run `bats tests/cli/claude-code/smoke.bats`. 6+ tests pass (binary detection, daemon health, fail-open ingest for valid/malformed/empty JSON). Tests 7-8 skip if claude not installed.
result: [pending]

### 2. Hook capture tests pass (all event types with gRPC verification)
expected: Run `bats tests/cli/claude-code/hooks.bats`. All 10 tests pass. Each test ingests a fixture via memory-ingest, then verifies the event appears in gRPC query results (hard Layer 2 assertions, no escape hatches).
result: [pending]

### 3. Pipeline tests pass (full E2E hook-to-query cycle)
expected: Run `bats tests/cli/claude-code/pipeline.bats`. Tests 1-3 and 5 pass (session lifecycle, TOC browse, cwd metadata, concurrent isolation). Test 4 skips if claude not installed. Uses random port (no hardcoded 50051).
result: [pending]

### 4. Negative tests pass (fail-open resilience)
expected: Run `bats tests/cli/claude-code/negative.bats`. All 7 tests pass. Each verifies memory-ingest returns exit 0 and `{"continue":true}` even under error conditions (daemon down, malformed JSON, empty stdin, unknown event, wrong port, large payload, timeout).
result: [pending]

### 5. CI workflow is valid and has correct matrix
expected: Run `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/e2e-cli.yml'))"` to validate YAML. Verify matrix includes all 5 CLIs (claude-code, gemini, opencode, copilot, codex) and 2 OSes (ubuntu-24.04, macos-latest). JUnit XML reporting configured.
result: [pending]

### 6. Fixture JSON files are valid
expected: Run `for f in tests/cli/fixtures/claude-code/*.json; do echo "$f:"; jq empty "$f" 2>&1; done`. All 9 event fixtures parse successfully. malformed.json intentionally fails (expected).
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0

## Gaps

[none yet]
