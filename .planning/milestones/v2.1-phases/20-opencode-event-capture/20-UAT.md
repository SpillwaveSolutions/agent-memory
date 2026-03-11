---
status: testing
phase: 20-opencode-event-capture
source: 20-01-SUMMARY.md, 20-02-SUMMARY.md, 20-03-SUMMARY.md
started: 2026-02-09T22:30:00Z
updated: 2026-02-09T22:30:00Z
---

## Current Test

number: 1
name: Agent field backward compatibility in JSON ingest
expected: |
  memory-ingest binary accepts JSON without agent field (backward compat).
  Run: `echo '{"hook_event_name":"SessionStart","session_id":"test-1"}' | cargo run -p memory-ingest 2>&1`
  Expected: No parse error. The binary processes the event (may fail on gRPC connection, but JSON parsing succeeds).
awaiting: user response

## Tests

### 1. Agent field backward compatibility in JSON ingest
expected: memory-ingest binary accepts JSON without agent field (no parse error). Run: `echo '{"hook_event_name":"SessionStart","session_id":"test-1"}' | cargo run -p memory-ingest 2>&1` — JSON parsing should succeed (gRPC connection error is fine).
result: [pending]

### 2. Agent field accepted in JSON ingest
expected: memory-ingest binary accepts JSON with agent field. Run: `echo '{"hook_event_name":"SessionStart","session_id":"test-1","agent":"opencode"}' | cargo run -p memory-ingest 2>&1` — JSON parsing succeeds, agent field recognized.
result: [pending]

### 3. Agent pipeline unit tests pass
expected: All agent-related tests pass. Run: `cargo test -p memory-client --all-features && cargo test -p memory-ingest --all-features && cargo test -p memory-service --all-features` — All tests pass with zero failures.
result: [pending]

### 4. OpenCode event capture plugin exists with correct structure
expected: Plugin file at `plugins/memory-opencode-plugin/.opencode/plugin/memory-capture.ts` exports `MemoryCapturePlugin` with four lifecycle hooks. Run: `grep -c "session.created\|session.idle\|message.updated\|tool.execute.after" plugins/memory-opencode-plugin/.opencode/plugin/memory-capture.ts` — Should output 4.
result: [pending]

### 5. Plugin tags events with agent:opencode
expected: The plugin hardcodes `agent: "opencode"` in captured events. Run: `grep 'agent.*opencode' plugins/memory-opencode-plugin/.opencode/plugin/memory-capture.ts` — Should find the agent tagging line.
result: [pending]

### 6. Plugin uses fail-open pattern
expected: Every event capture is wrapped in try/catch so OpenCode is never blocked. Run: `grep -c 'catch' plugins/memory-opencode-plugin/.opencode/plugin/memory-capture.ts` — Should be at least 1 (the captureEvent helper catches all errors).
result: [pending]

### 7. CLI --agent filter wired to gRPC
expected: The `--agent` flag on retrieval route passes through to the gRPC request. Run: `grep "agent_filter" crates/memory-daemon/src/commands.rs | head -3` — Should show agent_filter parameter and mapping to RouteQueryRequest.
result: [pending]

### 8. CLI displays agent in retrieval results
expected: When a retrieval result has an agent field, CLI shows "Agent: <name>". Run: `grep -A1 'agent.*result' crates/memory-daemon/src/commands.rs | grep -i 'println\|agent'` — Should find conditional agent display code.
result: [pending]

### 9. Plugin README documents event capture
expected: The plugin README has an "Event Capture" section. Run: `grep "## Event Capture" plugins/memory-opencode-plugin/README.md` — Should find the section header.
result: [pending]

### 10. Zero clippy warnings across affected crates
expected: No clippy warnings. Run: `cargo clippy -p memory-client -p memory-ingest -p memory-service -p memory-daemon --all-targets --all-features -- -D warnings` — Should exit 0 with no warnings.
result: [pending]

## Summary

total: 10
passed: 0
issues: 0
pending: 10
skipped: 0

## Gaps

[none yet]
