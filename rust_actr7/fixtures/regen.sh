#!/usr/bin/env bash
# regen.sh â€” regenerate the project extra traces. Requires: sbcl.
# Usage: ./regen.sh [output-dir]
# Env overrides: ACTR_DIR, GOLDEN_MANIFEST, GOLDEN_OUTDIR.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ACTR_DIR="${ACTR_DIR:-$(cd "$HERE/../../actr7.x" && pwd)}"
export GOLDEN_MANIFEST="${GOLDEN_MANIFEST:-$HERE/manifest.lisp}"
export GOLDEN_OUTDIR="${GOLDEN_OUTDIR:-${1:-$HERE/extra-traces}}"
mkdir -p "$GOLDEN_OUTDIR"
sbcl --non-interactive --disable-debugger \
  --load "$ACTR_DIR/load-single-threaded-act-r.lisp" \
  --load "$HERE/run-golden.lisp" \
  --eval '(golden-main)'
