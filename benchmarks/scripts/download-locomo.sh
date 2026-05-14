#!/usr/bin/env bash
set -euo pipefail
DEST="${1:-locomo-data}"
mkdir -p "$DEST"
echo "Downloading LOCOMO dataset to $DEST ..."
curl -L "https://snap-research.github.io/locomo/data/locomo_v1.zip" -o "$DEST/locomo_v1.zip"
unzip -q "$DEST/locomo_v1.zip" -d "$DEST"
echo "Done. Dataset at: $DEST"
echo "NOTE: Verify license terms at https://snap-research.github.io/locomo/ before publishing scores."
