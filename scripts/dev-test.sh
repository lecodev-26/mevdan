#!/usr/bin/env bash
# Run the same checks CI runs. Use before pushing.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "▶ cargo fmt --check"
cargo fmt --all -- --check

echo "▶ cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "▶ cargo build"
cargo build --workspace --all-targets

echo "▶ cargo test"
cargo test --workspace --all-targets

echo "▶ cargo doc"
cargo doc --workspace --no-deps

echo "✅ All checks passed."
