#!/usr/bin/env sh
set -e

cd "$(dirname "$0")/.."

echo "== cargo fmt --check"
cargo fmt --all -- --check

echo "== cargo check --all-targets"
cargo check --all-targets

echo "== cargo clippy -D warnings"
cargo clippy --all-targets -- -D warnings

echo "== cargo test"
cargo test

echo "== check: all green"
