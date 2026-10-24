#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

# --all también formatea dependencias que pertenecen a otros repositorios.
cargo fmt -- --check
cargo build --locked --offline
cargo clippy --locked --offline --all-targets --no-deps -- -D warnings
cargo test --locked --offline
