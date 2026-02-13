---
name: memory-install
description: |
  Wizard-style installation guide for agent-memory (macOS/Linux). Helps users
  choose an install path, check prerequisites, and set PATH. Always confirm
  before any file edits or copying binaries. Provide verification commands only.
license: MIT
metadata:
  version: 1.0.0
  author: SpillwaveSolutions
---

# Memory Install Skill

Install agent-memory with a guided, step-by-step wizard. Supports two install
paths: source build or prebuilt binaries. This skill never runs verification
commands automatically; it only provides them for the user to run if desired.

## When to Use

- User asks to install agent-memory
- User wants to choose between source build and prebuilt binaries
- User needs help adding binaries to PATH

## When Not to Use

- Configuration changes (use `memory-configure`)
- Verifying an existing install (use `memory-verify`)
- Troubleshooting failures (use `memory-troubleshoot`)

## Wizard Principles

- Ask one question at a time
- Confirm before any write/copy/edit action
- Provide verification commands but do not run them
- macOS and Linux only

## Wizard Flow

### Step 1: Confirm platform

```
Agent Memory Install
====================

Which platform are you on?

1. macOS
2. Linux

Enter selection [1-2]:
```

If not macOS/Linux, stop and explain platform support.

### Step 2: Prerequisites check

Ask whether these are installed:

- `protoc`
- Rust (only required for source builds)

Provide verification commands only:

```
Optional verify commands:
  protoc --version
  rustc --version
```

### Step 3: Choose install path

```
How would you like to install agent-memory?

1. Build from source (requires Rust)
2. Prebuilt binaries (fastest)

Default: 1 if Rust is available, else 2

Enter selection [1-2]:
```

### Step 4: Confirm install location

```
Where should the binaries live?

1. ~/.local/bin (recommended)
2. ~/.cargo/bin
3. /usr/local/bin (requires sudo)
4. Other path

Default: 1

Enter selection [1-4]:
```

### Step 5: Execute installation (with confirmation)

Before making changes, ask for confirmation:

```
I will:
  - Create the install directory (if needed)
  - Copy or unpack memory-daemon and memory-ingest
  - Set executable permissions

Proceed? (yes/no)
```

Provide commands based on choice:

**Source build**

```bash
git clone https://github.com/SpillwaveSolutions/agent-memory.git
cd agent-memory
cargo build --release
mkdir -p ~/.local/bin
cp target/release/memory-daemon ~/.local/bin/
cp target/release/memory-ingest ~/.local/bin/
```

**Prebuilt binaries**

```bash
mkdir -p ~/.local/bin
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
curl -L "https://github.com/SpillwaveSolutions/agent-memory/releases/latest/download/memory-daemon-${PLATFORM}-${ARCH}.tar.gz" | tar xz -C ~/.local/bin
curl -L "https://github.com/SpillwaveSolutions/agent-memory/releases/latest/download/memory-ingest-${PLATFORM}-${ARCH}.tar.gz" | tar xz -C ~/.local/bin
chmod +x ~/.local/bin/memory-daemon ~/.local/bin/memory-ingest
```

### Step 6: PATH guidance

Ask before editing shell profiles. If the user wants automation, confirm the
specific file before any edit.

```
I can add ~/.local/bin to your PATH in your shell profile.
Which file should I edit?

1. ~/.zshrc
2. ~/.bashrc
3. ~/.bash_profile
4. Skip (show command only)

Enter selection [1-4]:
```

If user chooses to edit, confirm again:

```
I will add:
  export PATH="$HOME/.local/bin:$PATH"

Proceed? (yes/no)
```

### Step 7: Verification commands (optional)

Provide commands only:

```
Optional verify commands:
  memory-daemon --version
  memory-ingest --version
```

### Step 8: Next steps

Guide users to configuration and agent setup:

- Use `memory-configure` for config
- Use `memory-verify` for health checks
- Use the agent-specific setup guide for hooks
