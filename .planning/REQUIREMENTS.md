# Requirements: Agent Memory v2.4

**Defined:** 2026-02-22
**Core Value:** Agent can answer "what were we talking about last week?" without scanning everything

## v2.4 Requirements

Requirements for v2.4 Headless CLI Testing milestone. Each maps to roadmap phases.

### Harness Framework

- [ ] **HARN-01**: Shell-based E2E harness using bats-core with isolated workspace per test file
- [ ] **HARN-02**: Daemon lifecycle management (start/stop/health check per workspace, OS-assigned port)
- [ ] **HARN-03**: CLI availability detection with graceful skip when binary not installed
- [ ] **HARN-04**: Common helper library (common.bash) with workspace, daemon, CLI wrapper functions
- [ ] **HARN-05**: JUnit XML reporting via bats native formatter
- [ ] **HARN-06**: CI integration with GitHub Actions matrix (CLI x category) and artifact retention on failure
- [ ] **HARN-07**: Fixture data directory with predefined JSON payloads and expected outputs

### Claude Code Tests

- [ ] **CLDE-01**: Claude Code headless smoke tests (binary detection, `-p` invocation, JSON output)
- [ ] **CLDE-02**: Claude Code hook capture tests (SessionStart, UserPrompt, PostToolUse, Stop payloads)
- [ ] **CLDE-03**: Claude Code E2E pipeline test (hook fire -> daemon ingest -> gRPC query verification)
- [ ] **CLDE-04**: Claude Code negative tests (daemon down, malformed input, timeout enforcement)

### Gemini CLI Tests

- [ ] **GEMI-01**: Gemini CLI headless smoke tests (binary detection, positional args, JSON output)
- [ ] **GEMI-02**: Gemini CLI hook capture tests (JSON stdin format, agent field verification)
- [ ] **GEMI-03**: Gemini CLI E2E pipeline test (hook -> ingest -> query)
- [ ] **GEMI-04**: Gemini CLI negative tests

### OpenCode Tests

- [ ] **OPEN-01**: OpenCode headless smoke tests (binary detection, `-p -q -f json` invocation)
- [ ] **OPEN-02**: OpenCode hook capture tests
- [ ] **OPEN-03**: OpenCode E2E pipeline test (hook -> ingest -> query)
- [ ] **OPEN-04**: OpenCode negative tests

### Copilot CLI Tests

- [ ] **CPLT-01**: Copilot CLI headless smoke tests (binary detection, `-p --yes --allow-all-tools`)
- [ ] **CPLT-02**: Copilot CLI hook capture tests (session ID synthesis verification)
- [ ] **CPLT-03**: Copilot CLI E2E pipeline test (hook -> ingest -> query)
- [ ] **CPLT-04**: Copilot CLI negative tests

### Codex CLI

- [ ] **CDEX-01**: Codex CLI adapter (commands + skills, no hooks, sandbox workaround docs)
- [ ] **CDEX-02**: Codex CLI headless smoke tests (binary detection, `codex exec -q --full-auto`)
- [ ] **CDEX-03**: Codex CLI command invocation tests (hooks skipped)
- [ ] **CDEX-04**: Codex CLI negative tests (hooks skipped)
- [ ] **CDEX-05**: Cross-CLI matrix report aggregation (CLI x scenario -> pass/fail/skipped)

## Future Requirements

### Post-v2.4

- Windows CLI testing support
- Performance regression tracking in shell tests
- GUI/dashboard for test results
- Cross-project shared harness (Agent RuleZ, Agent Cron, Agent CLOD)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Mock CLI simulators | Defeats E2E purpose; tests mock not CLI |
| Interactive/TUI testing | Brittle keystroke simulation; headless only |
| Full LLM round-trip tests | Slow, expensive, non-deterministic; test mechanical pipeline |
| API key management in tests | Use CI secrets; skip locally when absent |
| Custom test framework | Use bats-core; no maintenance burden |
| Windows support | macOS/Linux only for v2.4 |
| Shared state between tests | Each test file gets own workspace and daemon |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| HARN-01 | Phase 30 | Pending |
| HARN-02 | Phase 30 | Pending |
| HARN-03 | Phase 30 | Pending |
| HARN-04 | Phase 30 | Pending |
| HARN-05 | Phase 30 | Pending |
| HARN-06 | Phase 30 | Pending |
| HARN-07 | Phase 30 | Pending |
| CLDE-01 | Phase 30 | Pending |
| CLDE-02 | Phase 30 | Pending |
| CLDE-03 | Phase 30 | Pending |
| CLDE-04 | Phase 30 | Pending |
| GEMI-01 | Phase 31 | Pending |
| GEMI-02 | Phase 31 | Pending |
| GEMI-03 | Phase 31 | Pending |
| GEMI-04 | Phase 31 | Pending |
| OPEN-01 | Phase 32 | Pending |
| OPEN-02 | Phase 32 | Pending |
| OPEN-03 | Phase 32 | Pending |
| OPEN-04 | Phase 32 | Pending |
| CPLT-01 | Phase 33 | Pending |
| CPLT-02 | Phase 33 | Pending |
| CPLT-03 | Phase 33 | Pending |
| CPLT-04 | Phase 33 | Pending |
| CDEX-01 | Phase 34 | Pending |
| CDEX-02 | Phase 34 | Pending |
| CDEX-03 | Phase 34 | Pending |
| CDEX-04 | Phase 34 | Pending |
| CDEX-05 | Phase 34 | Pending |

**Coverage:**
- v2.4 requirements: 28 total
- Mapped to phases: 28
- Unmapped: 0 ✓

---
*Requirements defined: 2026-02-22*
*Last updated: 2026-02-22 after initial definition*
