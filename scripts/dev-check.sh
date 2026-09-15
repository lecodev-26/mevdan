#!/usr/bin/env bash
# Quick check before committing. Faster than full CI.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "▶ cargo fmt"
cargo fmt --all

echo "▶ cargo check"
cargo check --workspace --all-targets

echo "▶ cargo test"
cargo test --workspace --quiet

echo "✅ Ready to commit."
