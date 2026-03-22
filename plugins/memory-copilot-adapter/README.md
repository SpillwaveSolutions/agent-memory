# Memory Copilot Adapter (Archived)

This adapter has been replaced by `memory-installer`, which generates runtime-specific
plugin files from the canonical source.

## Migration

Install plugins for this runtime using the installer:

```bash
memory-installer --agent copilot --project
```

See `crates/memory-installer/` for details.

## Note

This directory is retained for one release cycle and will be removed in a future version.
The `.github/hooks/scripts/memory-capture.sh` file is preserved as a compile-time dependency of `CopilotConverter`.
