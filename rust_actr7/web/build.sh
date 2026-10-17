#!/usr/bin/env bash
# Build the wasm and stage it next to index.html.
#
# Needs a Rust toolchain whose sysroot has the wasm32-unknown-unknown std
# (e.g. the conda-forge `rust` package). No rustup / wasm-pack required.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$(cd "$HERE/.." && pwd)"

cargo build --manifest-path "$CRATE/Cargo.toml" --release --target wasm32-unknown-unknown
cp "$CRATE/target/wasm32-unknown-unknown/release/actr_engine.wasm" "$HERE/actr_engine.wasm"

echo "staged $HERE/actr_engine.wasm"
echo "now serve it (ES modules + wasm need HTTP, not file://):"
echo "  cd \"$HERE\" && python3 -m http.server 8000"
echo "  open http://localhost:8000/"
