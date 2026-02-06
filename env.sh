#!/usr/bin/env bash
# SDK environment setup for C++ compilation on macOS
# Source this before cargo build/test when CLT headers are incomplete
set -euo pipefail

# Prefer xcrun SDK; fallback to CLT path
sdk_path=$(xcrun --sdk macosx --show-sdk-path 2>/dev/null || echo "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk")

export CXXFLAGS="-isystem ${sdk_path}/usr/include/c++/v1"
export CFLAGS="-isystem ${sdk_path}/usr/include/c++/v1"

# Surface for logging
echo "Using SDK: ${sdk_path}"
