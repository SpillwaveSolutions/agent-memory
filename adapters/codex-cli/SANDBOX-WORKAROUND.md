# Codex CLI Sandbox Workaround

## The Problem

Codex CLI runs commands in a sandbox by default. This prevents `memory-ingest` and `memory-daemon` from making network connections (gRPC calls over TCP), which are required for the agent-memory system to function.

The sandbox behavior differs by platform:

| Platform | Sandbox Technology | Behavior |
|----------|-------------------|----------|
| Linux | Landlock | `network_access = true` in config works reliably |
| macOS | Seatbelt (Apple Sandbox) | Config may be silently ignored; workaround needed |

## Linux (Landlock)

On Linux, add network access to your Codex configuration:

```toml
# .codex/config.toml (project-level) or ~/.codex/config.toml (global)
[sandbox_workspace_write]
network_access = true
```

This reliably enables network access for commands run within Codex.

## macOS (Seatbelt)

On macOS, the Seatbelt sandbox may silently ignore the `network_access = true` configuration setting. There are two workarounds:

### Option 1: Disable Sandbox (Recommended for Development)

Run Codex with full access mode:

```bash
codex exec --sandbox danger-full-access "memory-daemon status"
```

Or set it in your configuration:

```toml
# .codex/config.toml
[sandbox]
mode = "danger-full-access"
```

**Warning:** This disables all sandbox protections. Only use in trusted development environments.

### Option 2: Network Access Configuration

Try the standard configuration first -- it may work on newer Codex CLI versions:

```toml
# .codex/config.toml
[sandbox_workspace_write]
network_access = true
```

If commands still fail with network errors, fall back to Option 1.

## Verification

After applying the workaround, verify network access works:

```bash
# Inside Codex
codex exec --full-auto "memory-daemon status"

# Expected: daemon status output (running/stopped)
# If sandbox blocks: connection refused or timeout error
```

## Related Issues

- [GitHub Issue #5041](https://github.com/openai/codex/issues/5041) -- macOS Seatbelt sandbox silently ignores network_access configuration
- The Codex team is aware of the macOS sandbox limitations and working on improvements

## Impact on Agent Memory

The sandbox affects these operations:
- `memory-daemon start` / `status` -- requires TCP port binding
- `memory-ingest` -- requires gRPC connection to daemon
- `memory-daemon query` / `retrieval` / `teleport` -- requires gRPC connection
- Any skill command that queries the daemon

If you see "connection refused" or timeout errors when running memory commands inside Codex, the sandbox is likely blocking network access. Apply one of the workarounds above.
