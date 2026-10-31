#!/usr/bin/env bash
# regen-draws.sh — regenerate the draw-sequence fixtures. Requires: sbcl.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ACTR_DIR="${ACTR_DIR:-$(cd "$HERE/../../../actr7.x" && pwd)}"
export DRAWS_MANIFEST="${DRAWS_MANIFEST:-$HERE/manifest.lisp}"
export DRAWS_OUTDIR="${DRAWS_OUTDIR:-${1:-$HERE}}"
mkdir -p "$DRAWS_OUTDIR"
sbcl --non-interactive --disable-debugger \
  --load "$ACTR_DIR/load-single-threaded-act-r.lisp" \
  --load "$HERE/run-draws.lisp" \
  --eval '(draws-main)'
