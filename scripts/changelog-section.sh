#!/usr/bin/env bash
# Print the CHANGELOG.md section for a version (heading through the next ##).
# Exits 1 if the section is missing so a release cannot ship empty notes.
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: changelog-section.sh <X.Y.Z> [CHANGELOG.md]" >&2
  exit 2
fi

VERSION="${1#v}"
FILE="${2:-CHANGELOG.md}"

if [[ ! -f "$FILE" ]]; then
  echo "error: changelog not found at $FILE" >&2
  exit 1
fi

awk -v ver="$VERSION" '
  BEGIN { found = 0 }
  $0 ~ ("^## v" ver "([[:space:]].*)?$") {
    found = 1
    print
    next
  }
  found && /^## / { exit }
  found { print }
  END {
    if (!found) {
      printf("error: no CHANGELOG.md section for v%s\n", ver) > "/dev/stderr"
      exit 1
    }
  }
' "$FILE"
