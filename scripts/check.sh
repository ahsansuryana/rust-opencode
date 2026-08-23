#!/usr/bin/env bash
# Check lokal tiap akhir sprint: fmt + clippy + test (aturan A.7 & deliverable
# #10 di SPRINT_00_bootstrap.md). Jalankan dari mana saja.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== cargo fmt --all --check =="
cargo fmt --all --check

echo "== cargo clippy --workspace --all-targets -- -D warnings =="
cargo clippy --workspace --all-targets -- -D warnings

echo "== cargo test --workspace =="
cargo test --workspace

echo "== semua check hijau =="
