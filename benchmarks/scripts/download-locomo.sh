#!/usr/bin/env bash
# Fetch the real LoCoMo dataset (locomo10.json) from snap-research/locomo.
# Prints LICENSE.txt before the data file so a human can confirm CC BY-NC 4.0.
set -euo pipefail

DEST="${1:-locomo-data}"
REPO="https://raw.githubusercontent.com/snap-research/locomo/main"
mkdir -p "$DEST"

echo "Fetching LICENSE.txt from snap-research/locomo ..."
curl -fsSL "$REPO/LICENSE.txt" -o "$DEST/LICENSE.txt"
echo
echo "========== LICENSE.txt (verify before publishing any score) =========="
sed -n '1,40p' "$DEST/LICENSE.txt"
echo "======================================================================"
echo
echo "LoCoMo is licensed Creative Commons BY-NC 4.0."
echo "Non-commercial research use only. Do not publish a score for commercial marketing"
echo "without reading the full license at $DEST/LICENSE.txt"
echo

echo "Downloading data/locomo10.json ..."
curl -fL "$REPO/data/locomo10.json" -o "$DEST/locomo10.json"

python3 - "$DEST/locomo10.json" <<'PY' || {
  echo "python3 sanity check failed; file is at $DEST/locomo10.json — parse it with memory-bench instead."
  exit 0
}
import json, sys
path = sys.argv[1]
with open(path) as f:
    data = json.load(f)
assert isinstance(data, list), f"expected top-level array, got {type(data)}"
assert data, "empty dataset"
sample = data[0]
assert "qa" in sample and "conversation" in sample, sample.keys()
print(f"OK: {len(data)} conversations, sample keys={list(sample.keys())}")
PY

echo "Done. Dataset at: $DEST/locomo10.json"
echo "Run: cargo run -p memory-bench -- locomo --dataset $DEST --scorer mock"
echo "LLM-judge (requires OPENAI_API_KEY or ANTHROPIC_API_KEY):"
echo "  cargo run -p memory-bench -- locomo --dataset $DEST --scorer llm-judge"
