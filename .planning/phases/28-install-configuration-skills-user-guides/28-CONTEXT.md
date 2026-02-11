# Phase 28: Install & Configuration Skills + User Guides - Context

**Gathered:** 2026-02-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Deliver step-by-step install/config user guides and agent skills that help users set up and validate the system. Focus is on installation, configuration, and setup guidance (not new product capabilities).

</domain>

<decisions>
## Implementation Decisions

### Install paths and prerequisites
- Document both source build and prebuilt binaries as first-class install paths.
- First-class platforms: macOS and Linux only.
- Prerequisites should include required tools plus optional recommended tools (clearly labeled optional).
- Agent-specific setup is NOT part of the core install flow; keep it in separate guides.

### Doc structure and depth
- Split documentation into a Quickstart and a Full Guide.
- Quickstart is concise, checklist-style; Full Guide is narrative step-by-step.
- Use optional “verify now” callouts rather than mandatory verification after each step.
- Troubleshooting lives in a dedicated section at the end of the docs.

### Configuration guidance
- Scope configuration docs to single-agent setups only.
- Show defaults inline and provide a full sample config file.
- Advanced options should be minimal with a link to a reference doc.
- Include a dry-run/config check step in the guide.

### Setup skills behavior
- Skills should be wizard-style prompts.
- Use safe automation with confirmation (edits/config creation only with user confirmation).
- Provide separate skills for install, config, verify, and troubleshoot.
- Verification handled by providing verification commands (not auto-running).

### Claude's Discretion
- Exact copy and wording for checklists and callouts.
- Ordering of steps within each install path, as long as prerequisites and verification are preserved.

</decisions>

<specifics>
## Specific Ideas

- Keep agent-specific setup in separate guides, not in the main install flow.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 28-install-configuration-skills-user-guides*
*Context gathered: 2026-02-11*
