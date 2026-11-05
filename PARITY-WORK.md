# Differential parity handoff

## Context

This repo contains a Rust port of ACT-R 7 plus the vendored `actr7.x` Lisp
tree. `actr7.x` is the oracle: do not edit it. Runtime commands may write
`actr7.x/environment/gui/init/05-current-net.tcl`; the differential harness
restores it automatically.

Working tree state:

- `actr7.x/support/load-mode.lisp` and `actr7.x/support/require-mode.lisp`
  were already modified before this work. Leave them alone.
- `actr7.x/environment/gui/init/05-current-net.tcl` is runtime-generated and
  untracked.
- `server_testing/` is an older untracked exploratory harness. The canonical
  harness now lives under `rust_actr7/fixtures/differential/`.

Commit at logical points. Do not commit `server_testing/` or `actr7.x`
changes.

## Current status

Rust changes are committed through `4ca31bf`.

Full Rust test suite:

```powershell
cd rust_actr7
cargo fmt --check
cargo test
```

Last result: `560 passed, 0 failed, 8 ignored`.

Differential harness (requires SBCL, Python 3, and a built Rust binary):

```powershell
cargo build --bin actr
python rust_actr7/fixtures/differential/run_cases.py --server both
cargo test --test differential -- --ignored --nocapture
```

Current differential cases all pass:

```text
DIFF-OK count-basic
DIFF-OK declarative-act
DIFF-OK declarative-long
DIFF-OK declarative-no-noise
DIFF-OK multi-model-talking
```

## Commits relevant to this work

- `4ca31bf tests: add multi-model-talking differential case`
- `e3fd403 rpc: preserve multi-model state during callbacks`
- `d85282e engine: preserve speech sound names and self filtering`
- `ea13c0f tests: add declarative activation-order differential cases`
- `f4f8a1c engine: use dm-chunks order for retrieval candidates`
- `82f2b87 docs: add differential parity handoff`
- `1e0ad9d tests: add declarative retrieval differential case`
- `d678ea0 engine: match isa-only retrieval and imaginal conditions`
- `fdaa3f8 tests: add differential SBCL/Rust harness`
- `c065a46 rpc: relay speech output across models`

## Harness layout

```text
rust_actr7/fixtures/differential/
  capture.py       # start SBCL or Rust, wait until ready, run one client, stop
  run_cases.py     # run every cases/*/client.py under both and compare
  cases/
    count-basic/client.py
    declarative-act/client.py
    declarative-long/client.py
    declarative-no-noise/client.py
    multi-model-talking/client.py
```

Each case is a directory with a `client.py` that imports the vendored
`actr7.x/tutorial/python/actr.py`, loads an absolute model path, and runs.
`run_cases.py` compares normalized non-empty lines plus the client exit code.

`tests/differential.rs` is the ignored wrapper:

```powershell
cargo test --test differential -- --ignored --nocapture
```

## Resolved: declarative retrieval candidate order

ACT-R's `chunk-list` comes from the ordered `dm-chunks` alist, not the merge
hash table:

- `declarative-memory.lisp:1125` builds `chunk-list` from `dm-chunks`.
- `declarative-memory.lisp:1929-1931` groups by filled-slot signature and
  pushes new signature groups and chunks to the front.
- `declarative-memory.lisp:2749` flattens the same structure for
  `all-dm-chunks`.
- `dm-chunk-hash-table` is only used for merge/identity lookup.

Rust standard retrieval originally walked raw reverse insertion order. That
happens to match a single signature group but not the interleaved groups in
`declarative-issues`. `f4f8a1c` switched retrieval, recent-retrieval tracing,
and `dm` printing to `dm_alist_order()`.

Regression coverage:

- `retrieval_candidate_order_uses_dm_signature_groups`
- `retrieval_candidate_order_follows_extended_signature_groups`
- `declarative-act`
- `declarative-long`
- `declarative-no-noise`

This also fixes the prior noisy activation mismatch; no SBCL hash-table
emulation is needed.

## Resolved: multi-model-talking

The intended Python task now matches SBCL. Root causes were:

1. Runtime `define-chunks` rebuilt the current engine, re-arming a reset
   event mid-run and replaying the trace. `define-chunks` now updates the live
   chunk table without a rebuild.
2. Engine rebuilds reset the global multi-model trace buffer and position.
   `rebuild_engine_in_place` now preserves them.
3. The multi-model run processed all events before draining client callbacks.
   The capped RPC path now yields one event at a time so `output-speech` relay
   callbacks run during the run.
4. `mp-time` used the current model's local clock. It now uses the shared
   meta-process clock.
5. `reset` reset the selected model first, changing shared FIFO ordering.
   It now resets models in `model-order`.
6. Speech self-sounds did not consume a `WORD{n}` name. They now create the
   same sound/event chunks as SBCL.
7. Automatic self-speech filtering incorrectly emitted
   `find-sound-failure`. `stuff-sound-buffer` is a stuffed request, so it
   leaves the buffer untouched instead.
8. Speech-device installation was lost on rebuild. It is now tracked per
   model and reapplied.

Regression coverage:

- `multi-model-talking`
- `runtime_define_chunks_does_not_replay_multi_model_trace`
- `hear_self`

## Out of scope

Do not spend time on:

- `define-model-fct`
- custom module SGP parameters (`:create-delay`, `:demo-param`)
- `style-test-model` / `extend-manual-requests`
- SBCL warning line wrapping
- Lisp compiler/source warnings

These account for the remaining known load-only probe diffs.
