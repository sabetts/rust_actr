#!/usr/bin/env bash
# regen-full.sh — regenerate the full-detail differential traces.
# Usage: ./regen-full.sh [output-dir]
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES="$(cd "$HERE/.." && pwd)"
export TRACE_MANIFEST="${TRACE_MANIFEST:-$HERE/manifest.lisp}"
export TRACE_OUTDIR="${TRACE_OUTDIR:-${1:-$HERE/traces}}"
exec "$FIXTURES/regen.sh"
