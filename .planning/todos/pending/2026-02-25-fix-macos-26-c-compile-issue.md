---
created: 2026-02-25T18:05:23.941Z
title: Fix macOS 26 C++ compile issue with .cargo/config.toml
area: tooling
files:
  - .cargo/config.toml
---

## Problem

macOS 26 (Tahoe) beta Command Line Tools have a bug where C++ standard library headers (`<algorithm>`, etc.) exist in the SDK directory (`MacOSX.sdk/usr/include/c++/v1/` — 189 files) but the toolchain directory (`usr/include/c++/v1/`) only has 4 legacy stubs. Clang searches the toolchain path first and finds nothing.

This blocks all `cargo build`, `cargo clippy`, and `cargo test` for any crate depending on `librocksdb-sys` or `cxx` (which includes most of the workspace via `memory-daemon` -> `memory-storage` -> `librocksdb-sys`).

Not a Rust/cc-rs/RocksDB issue — even bare `echo '#include <algorithm>' | c++ -c -` fails.

Verified workaround: two environment variables fix the issue completely:
- `CPLUS_INCLUDE_PATH=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/c++/v1`
- `MACOSX_DEPLOYMENT_TARGET=14.0`

## Solution

**Option 1 (recommended): `.cargo/config.toml` with `[env]`**
- Persists per-project, checked into git
- Harmless on other platforms (just adds an additional search path)
- CI is on Ubuntu/macOS-latest (macOS 15), not affected

```toml
# .cargo/config.toml
# Workaround for macOS 26 (Tahoe) beta: C++ stdlib headers missing from
# toolchain include path. Safe to remove once Apple fixes Command Line Tools.
[env]
CPLUS_INCLUDE_PATH = "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/c++/v1"
MACOSX_DEPLOYMENT_TARGET = "14.0"
```

**Option 2: Shell profile export**
- Persists per-machine, doesn't affect the repo
- Add to `~/.zshrc`: `export CPLUS_INCLUDE_PATH=... MACOSX_DEPLOYMENT_TARGET=14.0`

**Removal:** Safe to remove once Apple fixes Command Line Tools for macOS 26.
