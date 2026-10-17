#!/usr/bin/env bash
# regen-units.sh â€” regenerate unit tables. Requires: sbcl.
# Usage: ./regen-units.sh [output-dir]
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ACTR_DIR="${ACTR_DIR:-$(cd "$HERE/../../../actr7.x" && pwd)}"
export GOLDEN_UNITS_DIR="${GOLDEN_UNITS_DIR:-${1:-$HERE/tables}}"
mkdir -p "$GOLDEN_UNITS_DIR"
sbcl --non-interactive --disable-debugger \
  --load "$ACTR_DIR/load-single-threaded-act-r.lisp" \
  --load "$HERE/run-units.lisp" \
  --eval '(units-main)'
