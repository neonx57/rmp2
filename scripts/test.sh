#!/usr/bin/env sh
set -e

cd "$(dirname "$0")/.."

echo "== cargo test"
cargo test

if command -v mpv >/dev/null 2>&1; then
    echo "== cargo test (mpv integration enabled)"
    cargo test --test mpv_integration -- --nocapture
else
    echo "== mpv not found: mpv integration self-skipped"
fi

echo "== cargo build --release"
cargo build --release

echo "== test: all green"
