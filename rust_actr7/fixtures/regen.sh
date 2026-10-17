#!/usr/bin/env bash
# regen.sh — regenerate the project extra traces. Requires: sbcl.
# Usage: ./regen.sh [output-dir]
# Env overrides: ACTR_DIR, TRACE_MANIFEST, TRACE_OUTDIR.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ACTR_DIR="${ACTR_DIR:-$(cd "$HERE/../../actr7.x" && pwd)}"
export TRACE_MANIFEST="${TRACE_MANIFEST:-$HERE/manifest.lisp}"
export TRACE_OUTDIR="${TRACE_OUTDIR:-${1:-$HERE/extra-traces}}"
mkdir -p "$TRACE_OUTDIR"
sbcl --non-interactive --disable-debugger \
  --load "$ACTR_DIR/load-single-threaded-act-r.lisp" \
  --load "$HERE/run-traces.lisp" \
  --eval '(trace-main)'
