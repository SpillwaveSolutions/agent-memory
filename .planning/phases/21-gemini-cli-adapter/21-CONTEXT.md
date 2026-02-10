# Phase 21: Gemini CLI Adapter - Context

**Gathered:** 2026-02-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Create a Gemini CLI hook adapter that captures session events into Agent Memory with `agent:gemini` tagging, provides TOML-based query commands backed by shared skills, and includes an automated install skill. Achieves full Claude parity where Gemini's hook system allows.

</domain>

<decisions>
## Implementation Decisions

### Hook format & event mapping
- Gemini CLI has a hook system in its latest release — target the actual hook API, no wrapper scripts
- Research Gemini's hook format from scratch (user has no prior knowledge of specifics)
- Map Gemini hook events to existing Agent Memory event types (session_start, user_message, tool_result, assistant_stop, etc.) as closely as possible
- If 1:1 mapping isn't possible for some events, create Gemini-specific event types as fallback
- Hooks should call `memory-ingest` binary directly (same binary as Claude hooks); if Gemini's hook format makes that impractical, fall back to a Gemini-specific ingest binary

### Command & skill porting
- Create TOML command wrappers for query commands (memory-search, memory-recent, memory-context)
- TOML commands reference the same SKILL.md files — skills are the shared format across agents
- No separate navigator agent definition — embed navigator logic inside the skill, tell Gemini to invoke in parallel
- Skill file sharing strategy: Claude's discretion (separate copies vs shared references based on practical constraints)

### Installation & setup UX
- Hook handler calls the compiled `memory-ingest` Rust binary directly — no TypeScript/Bun runtime dependency
- Provide both: an `agent-memory-gemini-install-skill` for automated setup AND manual documentation
- Install skill auto-detects Gemini CLI presence and warns if not found
- Setup writes Gemini hook config files automatically

### Adapter boundary & parity
- Target maximum Claude parity — event capture + query commands + navigator equivalent + install skill
- Fail-open philosophy: hooks silently fail if memory daemon is unreachable (same as Claude/OpenCode)
- For missing hook events: Claude's discretion per event — document trivial gaps, work around important ones
- Automated E2E testing with real Gemini CLI sessions (not just unit tests + manual)

### Claude's Discretion
- Plugin directory structure (separate `plugins/memory-gemini-adapter/` vs shared structure)
- Whether to use separate skill copies or shared references
- Specific workaround strategies for missing Gemini hook events
- TOML command structure details (based on Gemini's actual format)

</decisions>

<specifics>
## Specific Ideas

- "Put the logic of the agent inside of the skill and then we just tell Gemini to invoke it in parallel" — navigator agent is a skill, not a separate agent definition
- "Make it part of an agent-memory-gemini-install-skill" — automated install follows the same pattern as the Claude Code setup installer
- "Attempt reuse of memory-ingest binary, fallback to Gemini-specific binary" — prefer reuse, pragmatic about fallbacks
- "Gemini has commands but they are TOML file based" — commands are TOML, skills use the agent skills standard
- "Agent skills and commands are basically the same — a skill with one file is almost identical to a command" — keep the boundary thin

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 21-gemini-cli-adapter*
*Context gathered: 2026-02-09*
