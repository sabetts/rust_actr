#!/usr/bin/env bash
# regen-specs.sh — Phase 1 regenerate `test-models/specs/*.spec`.
#
# Runs the ACT-R regression stubs (trace-reference tests only) under
# `extract-specs.lisp` and rewrites the committed spec corpus. Requires SBCL.
#
# Usage: ./regen-specs.sh [output-dir]
# Env overrides: ACTR_DIR, SPEC_LIST (explicit name list), SPEC_OUTDIR.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ACTR_DIR="${ACTR_DIR:-$(cd "$HERE/../../actr7.x" && pwd)}"
TM_DIR="$ACTR_DIR/test-models"
OUT_DIR="${SPEC_OUTDIR:-${1:-$HERE/test-models/specs}}"

if [[ ! -d "$TM_DIR" ]]; then
  echo "test-models dir not found: $TM_DIR" >&2
  exit 1
fi

export SPEC_TMDIR="$TM_DIR"
export SPEC_OUTDIR="$OUT_DIR"
export SPEC_ACTRDIR="$ACTR_DIR"
mkdir -p "$OUT_DIR"

if [[ -z "${SPEC_LIST:-}" ]]; then
  # Trace refs only: those with a timestamped line, mirroring the classic
  # ` 0.000   GOAL ...` trace shape.
  SPEC_LIST="$(mktemp "${TMPDIR:-/tmp/opencode}/spec-list.XXXXXX")"
  ( cd "$TM_DIR" && grep -lE '^[[:space:]]*[0-9]+\.[0-9]{3}' *.ref | sed 's/\.ref$//' | sort ) > "$SPEC_LIST"
  CLEANUP_LIST=1
else
  CLEANUP_LIST=0
fi
export SPEC_LIST

trap '[[ "${CLEANUP_LIST:-0}" == 1 ]] && rm -f "$SPEC_LIST"' EXIT

echo "spec regen: $(wc -l < "$SPEC_LIST") trace tests -> $OUT_DIR"
sbcl --non-interactive --disable-debugger \
  --load "$ACTR_DIR/load-single-threaded-act-r.lisp" \
  --load "$TM_DIR/model-tester.cl" \
  --load "$HERE/extract-specs.lisp" \
  --eval '(spec-main)'
