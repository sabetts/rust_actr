#!/usr/bin/env bash
# rpc-conformance.sh — run the Python test-models through the Rust RPC server
# and compare against the shared `.ref` files.
#
# Requires: a built `actr-server`, Python 3, and the ACT-R checkout.
# Usage: rpc-conformance.sh [--expect|--all|<test>...]
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$(cd "$HERE/.." && pwd)"
ACTR_DIR="${ACTR_DIR:-$(cd "$CRATE/../actr7.x" && pwd)}"
export ACTR_DIR

MODE="${1:---expect}"
PORT="${RPC_PORT:-2699}"

cargo build --manifest-path "$CRATE/Cargo.toml" --bins >/dev/null

"$CRATE/target/debug/actr-server" --port "$PORT" --actr "$ACTR_DIR" \
  --no-port-file >/dev/null 2>&1 &
SERVER=$!
# Wait for the listener.
for _ in $(seq 1 50); do
  if python3 -c "import socket,sys; socket.create_connection(('127.0.0.1', $PORT), 0.2).close()" 2>/dev/null; then
    break
  fi
  sleep 0.1
done

cleanup() { kill "$SERVER" 2>/dev/null || true; }
trap cleanup EXIT

if [[ "$MODE" == "--expect" ]]; then
  python3 "$HERE/rpc-conformance.py" "$PORT" --expect "$HERE/rpc-enabled.txt"
elif [[ "$MODE" == "--all" ]]; then
  python3 "$HERE/rpc-conformance.py" "$PORT" --all
else
  python3 "$HERE/rpc-conformance.py" "$PORT" "$@"
fi
