#!/usr/bin/env bash
# regen-surrogates.sh — regenerate project-authored surrogate traces.
# Requires SBCL and the ACT-R checkout.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$(cd "$HERE/../.." && pwd)"
ACTR_DIR="${ACTR_DIR:-$(cd "$CRATE/../actr7.x" && pwd)}"
export SURROGATES_DIR="${SURROGATES_DIR:-$CRATE/fixtures/models/extras/surrogates}"
export SURROGATES_OUT="${SURROGATES_OUT:-$HERE/traces}"
sbcl --non-interactive --disable-debugger \
  --load "$ACTR_DIR/load-single-threaded-act-r.lisp" \
  --load "$HERE/run-surrogates.lisp" \
  --eval '(surrogates-main)'
