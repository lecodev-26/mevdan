#!/usr/bin/env bash
# Build optimized binary.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "▶ Building release binary..."
cargo build --release --bin mevdan

BIN="./target/release/mevdan"

if [ -f "$BIN" ]; then
    echo "✅ Binary: $BIN"
    "$BIN" --version
else
    echo "❌ Build failed."
    exit 1
fi
