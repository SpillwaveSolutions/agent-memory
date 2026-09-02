#!/usr/bin/env bash
# Local tests for scripts/release-guards.sh and scripts/changelog-section.sh.
# No Rust toolchain required — runs on every PR via the "Release Guard Scripts" job.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GUARDS="$ROOT/scripts/release-guards.sh"
CHANGELOG_SH="$ROOT/scripts/changelog-section.sh"
FAILS=0

assert_exit() {
  local want="$1"
  local label="$2"
  shift 2
  local got=0
  "$@" >/tmp/rg-out.txt 2>/tmp/rg-err.txt || got=$?
  if [[ "$got" -ne "$want" ]]; then
    echo "FAIL: $label (want exit $want, got $got)"
    echo "  stdout: $(cat /tmp/rg-out.txt)"
    echo "  stderr: $(cat /tmp/rg-err.txt)"
    FAILS=$((FAILS + 1))
  else
    echo "ok:   $label"
  fi
}

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

# --- changelog-section ---
cat > "$WORKDIR/CHANGELOG.md" <<'EOF'
# Changelog

## v3.1.0 — Make It True (2026-08-31)

Body of 3.1.0.

### Added

- a thing

## v2.7.0 — Older

Old body.
EOF

assert_exit 0 "changelog extracts v3.1.0" \
  bash "$CHANGELOG_SH" 3.1.0 "$WORKDIR/CHANGELOG.md"
if ! grep -q "Body of 3.1.0." /tmp/rg-out.txt; then
  echo "FAIL: changelog body missing"
  FAILS=$((FAILS + 1))
fi
if grep -q "v2.7.0" /tmp/rg-out.txt; then
  echo "FAIL: changelog leaked the next section"
  FAILS=$((FAILS + 1))
fi

assert_exit 0 "changelog accepts leading v" \
  bash "$CHANGELOG_SH" v3.1.0 "$WORKDIR/CHANGELOG.md"

assert_exit 1 "changelog missing section fails" \
  bash "$CHANGELOG_SH" 9.9.9 "$WORKDIR/CHANGELOG.md"

# --- git ancestor + version ---
REPO="$WORKDIR/repo"
mkdir -p "$REPO"
cd "$REPO"
git init -q
git config user.name "guard-test"
git config user.email "guard-test@example.com"
# Default branch name: main
git checkout -q -b main

cat > Cargo.toml <<'EOF'
[workspace]
members = ["crates/x"]

[workspace.package]
version = "3.1.0"
edition = "2021"
EOF
git add Cargo.toml
git commit -q -m "main: version 3.1.0"
MAIN_SHA="$(git rev-parse HEAD)"

# Feature-branch commit that is NOT on main
git checkout -q -b feature/stale
echo "stale" > extra.txt
git add extra.txt
git commit -q -m "stale local ref"
STALE_SHA="$(git rev-parse HEAD)"
git checkout -q main

assert_exit 0 "matching version on main succeeds" \
  bash "$GUARDS" --version 3.1.0 --sha "$MAIN_SHA" --main-ref main --cargo Cargo.toml

assert_exit 0 "leading v on version is stripped" \
  bash "$GUARDS" --version v3.1.0 --sha "$MAIN_SHA" --main-ref main --cargo Cargo.toml

assert_exit 1 "version mismatch fails (the v3.1.0-on-2.7.0 incident)" \
  bash "$GUARDS" --version 9.9.9 --sha "$MAIN_SHA" --main-ref main --cargo Cargo.toml

assert_exit 1 "commit not on main fails" \
  bash "$GUARDS" --version 3.1.0 --sha "$STALE_SHA" --main-ref main --cargo Cargo.toml

# An ancestor that is not HEAD still passes (old main commit).
echo "later" >> Cargo.toml
# keep version the same so only ancestry is under test
git add Cargo.toml
git commit -q -m "later commit still 3.1.0"
assert_exit 0 "older main commit is still an ancestor" \
  bash "$GUARDS" --version 3.1.0 --sha "$MAIN_SHA" --main-ref main --cargo Cargo.toml

if [[ "$FAILS" -ne 0 ]]; then
  echo "release-guards-test: $FAILS failure(s)"
  exit 1
fi
echo "release-guards-test: all passed"
