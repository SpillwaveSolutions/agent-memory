# Agent-Specific Setup Guides

Agent setup is intentionally separate from the core install flow. Pick the guide
that matches your tool and follow its steps to configure hooks or plugins.

## Claude Code

Use the Claude Code plugin guide to set up hooks and query commands.

- [Claude Code Plugin Guide](../../plugins/memory-query-plugin/README.md)

## OpenCode

The OpenCode adapter provides native commands and skills.

- [OpenCode Plugin Guide](../../plugins/memory-opencode-plugin/README.md)

## Gemini CLI

The Gemini adapter uses shell hooks and configuration files.

- [Gemini CLI Adapter Guide](../../plugins/memory-gemini-adapter/README.md)

## Copilot CLI

The Copilot adapter provides hook-based capture for CLI sessions.

- [Copilot CLI Adapter Guide](../../plugins/memory-copilot-adapter/README.md)

## Notes

- Complete core installation first (Quickstart or Full Guide)
- Keep agent setup steps out of the daemon install flow
- If you use multiple agents, configure each adapter separately

## Optional Verification

After completing your agent guide, you can verify ingestion with:

- `memory-daemon status`
- `memory-daemon query --endpoint http://[::1]:50051 root`

If events are not appearing, double-check the hooks file paths in the
agent-specific guide you followed.
