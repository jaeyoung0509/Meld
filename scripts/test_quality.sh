#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

require_tool() {
  local subcommand="$1"
  if ! cargo "$subcommand" --version >/dev/null 2>&1; then
    echo "[FAIL] cargo-$subcommand is not installed"
    echo "Install: cargo install cargo-$subcommand --locked"
    exit 1
  fi
}

require_tool "nextest"
require_tool "llvm-cov"

mkdir -p target/coverage

echo "[1/3] cargo nextest run --workspace --all-targets"
cargo nextest run --workspace --all-targets

echo "[2/3] cargo llvm-cov --workspace --summary-only"
cargo llvm-cov --workspace --summary-only | tee target/coverage/summary.txt

echo "[3/3] cargo llvm-cov --workspace --lcov --output-path target/coverage/lcov.info"
cargo llvm-cov --workspace --lcov --output-path target/coverage/lcov.info

echo "Quality checks completed."
echo "Coverage summary: target/coverage/summary.txt"
echo "Coverage lcov: target/coverage/lcov.info"
