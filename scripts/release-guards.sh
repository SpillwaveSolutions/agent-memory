#!/usr/bin/env bash
# Release guards for agent-memory.
#
# Fail unless:
#   1. workspace.package.version equals --version (leading "v" stripped)
#   2. --sha is an ancestor of --main-ref (the tagged commit is on main)
#
# Intended to run in .github/workflows/release.yml *before* any platform
# build. Also exercised by scripts/release-guards-test.sh on every PR.
set -euo pipefail

usage() {
  cat <<'EOF' >&2
usage: release-guards.sh --version <X.Y.Z> --sha <commit> [--main-ref origin/main] [--cargo Cargo.toml]
EOF
  exit 2
}

VERSION=""
SHA=""
MAIN_REF="origin/main"
CARGO_TOML="Cargo.toml"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      VERSION="${2:-}"
      shift 2
      ;;
    --sha)
      SHA="${2:-}"
      shift 2
      ;;
    --main-ref)
      MAIN_REF="${2:-}"
      shift 2
      ;;
    --cargo)
      CARGO_TOML="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage
      ;;
  esac
done

if [[ -z "$VERSION" || -z "$SHA" ]]; then
  echo "error: --version and --sha are required" >&2
  usage
fi

VERSION="${VERSION#v}"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "error: version must be X.Y.Z (optional leading v), got '$VERSION'" >&2
  exit 1
fi

if [[ ! -f "$CARGO_TOML" ]]; then
  echo "error: Cargo.toml not found at $CARGO_TOML" >&2
  exit 1
fi

CARGO_VERSION="$(
  awk '
    $0 == "[workspace.package]" { in_pkg = 1; next }
    in_pkg && /^\[/ { in_pkg = 0 }
    in_pkg && $1 == "version" {
      val = $3
      gsub(/"/, "", val)
      print val
      exit
    }
  ' "$CARGO_TOML"
)"

if [[ -z "$CARGO_VERSION" ]]; then
  echo "error: could not parse workspace.package.version from $CARGO_TOML" >&2
  exit 1
fi

if [[ "$CARGO_VERSION" != "$VERSION" ]]; then
  echo "error: Cargo.toml version '$CARGO_VERSION' does not match release version '$VERSION'" >&2
  echo "       A tag that disagrees with the crate version cannot produce a release." >&2
  exit 1
fi

if ! git cat-file -e "${SHA}^{commit}" 2>/dev/null; then
  echo "error: '$SHA' is not a commit in this repository" >&2
  exit 1
fi

if ! git rev-parse --verify "$MAIN_REF" >/dev/null 2>&1; then
  echo "error: main ref '$MAIN_REF' not found (fetch origin/main first)" >&2
  exit 1
fi

if ! git merge-base --is-ancestor "$SHA" "$MAIN_REF"; then
  echo "error: $SHA is not an ancestor of $MAIN_REF" >&2
  echo "       Refusing to release a commit that is not on main." >&2
  exit 1
fi

echo "ok: version $VERSION matches $CARGO_TOML; $(git rev-parse --short "$SHA") is an ancestor of $MAIN_REF"
