#!/usr/bin/env bash
# regen-units.sh — regenerate unit tables. Requires: sbcl.
# Usage: ./regen-units.sh [output-dir]
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ACTR_DIR="${ACTR_DIR:-$(cd "$HERE/../../../actr7.x" && pwd)}"
export UNITS_OUTDIR="${UNITS_OUTDIR:-${1:-$HERE/tables}}"
mkdir -p "$UNITS_OUTDIR"
sbcl --non-interactive --disable-debugger \
  --load "$ACTR_DIR/load-single-threaded-act-r.lisp" \
  --load "$HERE/run-units.lisp" \
  --eval '(units-main)'
