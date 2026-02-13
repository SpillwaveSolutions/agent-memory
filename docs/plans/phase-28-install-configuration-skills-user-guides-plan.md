---
phase: 28-install-configuration-skills-user-guides
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - docs/setup/quickstart.md
  - docs/setup/full-guide.md
  - docs/setup/agent-setup.md
  - docs/README.md
  - plugins/memory-setup-plugin/skills/memory-install/SKILL.md
  - plugins/memory-setup-plugin/skills/memory-configure/SKILL.md
  - plugins/memory-setup-plugin/skills/memory-verify/SKILL.md
  - plugins/memory-setup-plugin/skills/memory-troubleshoot/SKILL.md
  - plugins/memory-setup-plugin/README.md
  - .planning/ROADMAP.md
autonomous: true

must_haves:
  truths:
    - "Users can follow a Quickstart checklist for macOS or Linux using either source build or prebuilt binaries."
    - "Users can follow a Full Guide to install, configure a single-agent setup, and run an optional dry-run/config check step."
    - "Verification steps are presented as optional 'verify now' callouts, not mandatory gates."
    - "Agent-specific setup is available in a separate guide and not part of the core install flow."
    - "Install/config/verify/troubleshoot skills provide wizard-style prompts with confirmation before edits and only provide verification commands."
  artifacts:
    - path: "docs/setup/quickstart.md"
      provides: "Checklist-style Quickstart with macOS/Linux install paths and verify-now callouts"
      min_lines: 80
    - path: "docs/setup/full-guide.md"
      provides: "Narrative step-by-step install/config guide with dry-run step and full sample config"
      min_lines: 160
    - path: "docs/setup/agent-setup.md"
      provides: "Separate agent-specific setup links for adapters"
      min_lines: 40
    - path: "plugins/memory-setup-plugin/skills/memory-install/SKILL.md"
      provides: "Wizard-style install skill with confirmation gates"
      min_lines: 120
    - path: "plugins/memory-setup-plugin/skills/memory-configure/SKILL.md"
      provides: "Wizard-style configuration skill scoped to single-agent defaults"
      min_lines: 120
    - path: "plugins/memory-setup-plugin/skills/memory-verify/SKILL.md"
      provides: "Verification skill listing commands only (no auto-run)"
      min_lines: 80
    - path: "plugins/memory-setup-plugin/skills/memory-troubleshoot/SKILL.md"
      provides: "Troubleshooting skill with safe automation and confirmation"
      min_lines: 120
  key_links:
    - from: "docs/setup/quickstart.md"
      to: "docs/setup/agent-setup.md"
      via: "Agent setup link section"
      pattern: "agent-setup.md"
    - from: "docs/setup/full-guide.md"
      to: "docs/references/configuration-reference.md"
      via: "Advanced options reference link"
      pattern: "configuration-reference.md"
    - from: "plugins/memory-setup-plugin/README.md"
      to: "plugins/memory-setup-plugin/skills/memory-install/SKILL.md"
      via: "Skills list"
      pattern: "memory-install"
---

<objective>
Deliver Quickstart and Full Guide install/config documentation plus four dedicated setup skills (install, configure, verify, troubleshoot) aligned to the v2.3 setup experience.

Purpose: Give users clear, step-by-step setup guidance and safe wizard-style skills without adding new product capabilities.
Output: New setup docs, new setup skills, and updated documentation links.
</objective>

<execution_context>
@/Users/richardhightower/.config/opencode/get-shit-done/workflows/execute-plan.md
@/Users/richardhightower/.config/opencode/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@docs/design/09-getting-started.md
@docs/references/configuration-reference.md
@plugins/memory-setup-plugin/README.md
@plugins/memory-setup-plugin/skills/memory-setup/SKILL.md
@plugins/memory-setup-plugin/commands/memory-setup.md
@plugins/memory-setup-plugin/commands/memory-status.md
@plugins/memory-setup-plugin/commands/memory-config.md
</context>

<tasks>

<task type="auto">
  <name>Task 1: Author Quickstart and Full Guide setup docs</name>
  <files>docs/setup/quickstart.md, docs/setup/full-guide.md, docs/setup/agent-setup.md, docs/README.md</files>
  <action>
Create two setup guides plus a separate agent setup guide:
- docs/setup/quickstart.md: checklist-style, concise; include prerequisites (required + optional labeled), both install paths (source build + prebuilt binaries) for macOS/Linux only, optional "Verify now" callouts after key steps, dry-run/config check step, and a troubleshooting section at the end.
- docs/setup/full-guide.md: narrative step-by-step with same install paths; include defaults inline, a full sample config file, single-agent-only guidance, an explicit dry-run/config check step before starting the daemon, minimal advanced options with a link to docs/references/configuration-reference.md, and troubleshooting section at the end.
- docs/setup/agent-setup.md: list agent-specific setup guides (Claude Code, OpenCode, Gemini CLI, Copilot CLI) with links to existing plugin READMEs; keep out of main install flow and reference from both guides.
- Update docs/README.md to add a "Setup Guides" section linking to the Quickstart, Full Guide, and Agent Setup guide.
Honor decisions: macOS/Linux only, prerequisites with optional labeling, separate agent setup guide, optional verification callouts, and troubleshooting at the end of each guide.
  </action>
  <verify>Check that both guides include both install paths, macOS/Linux only, verify-now callouts, dry-run step, and a troubleshooting section at the end.</verify>
  <done>Quickstart + Full Guide exist with required structure, agent setup guide is separate, and docs/README.md links to all three.</done>
</task>

<task type="auto">
  <name>Task 2: Add dedicated install/config/verify/troubleshoot skills</name>
  <files>plugins/memory-setup-plugin/skills/memory-install/SKILL.md, plugins/memory-setup-plugin/skills/memory-configure/SKILL.md, plugins/memory-setup-plugin/skills/memory-verify/SKILL.md, plugins/memory-setup-plugin/skills/memory-troubleshoot/SKILL.md, plugins/memory-setup-plugin/README.md</files>
  <action>
Create four new skills in plugins/memory-setup-plugin/skills/ with wizard-style prompts:
- memory-install: guides install path selection (source build or prebuilt), prerequisite checks, and PATH setup; include safe automation with explicit confirmation before edits (writing files, copying binaries). No auto-run verification; provide verification commands only.
- memory-configure: single-agent-only configuration guidance; shows defaults inline and includes a full sample config; any config file creation or edits require confirmation; include dry-run/config check step.
- memory-verify: lists verification commands for install/config/daemon/ingest; do not run commands automatically.
- memory-troubleshoot: diagnostic flow with safe automation; confirmation required before any edits or restarts; provide fix steps for common install/config/daemon/capture issues.
Update plugins/memory-setup-plugin/README.md to list these four skills and explain when to use each, keeping agent-specific setup separate.
  </action>
  <verify>Each SKILL.md uses wizard-style prompts, includes confirmation before edits, and provides verification commands without auto-running.</verify>
  <done>Four skills exist with required behavior and plugin README references them.</done>
</task>

<task type="auto">
  <name>Task 3: Update roadmap plan listing for Phase 28</name>
  <files>.planning/ROADMAP.md</files>
  <action>Update Phase 28 plan count and list to reflect this plan’s objective (Quickstart + Full Guide + setup skills). Keep goal intact; update only the plans section.</action>
  <verify>Phase 28 shows 1 plan with updated description.</verify>
  <done>ROADMAP.md plan list matches the new Phase 28 plan file.</done>
</task>

</tasks>

<verification>
- Quickstart and Full Guide contain macOS/Linux-only instructions with both install paths, optional verify callouts, and troubleshooting sections at the end.
- Configuration guidance is single-agent only with defaults inline, full sample config, and dry-run step.
- Four setup skills exist and follow wizard + confirmation rules.
</verification>

<success_criteria>
- Users can set up agent-memory from either install path using Quickstart or Full Guide without encountering agent-specific setup steps in the core flow.
- Dedicated install/config/verify/troubleshoot skills are available and documented.
</success_criteria>

<output>
After completion, create `.planning/phases/28-install-configuration-skills-user-guides/28-01-SUMMARY.md`
</output>
