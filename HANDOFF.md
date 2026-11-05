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

Full Rust test suite:

```powershell
cd rust_actr7
cargo test
```

Last result: `559 passed, 0 failed, 8 ignored`.

Differential harness (requires SBCL, Python 3, and a built Rust binary):

```powershell
cargo build --bin actr
python rust_actr7/fixtures/differential/run_cases.py --server both
cargo test --test differential -- --ignored --nocapture
```

Current ignored differential result:

```text
DIFF-OK count-basic
DIFF-OK declarative-no-noise
```

## Commits relevant to this work

- `fdaa3f8 tests: add differential SBCL/Rust harness`
- `d678ea0 engine: match isa-only retrieval and imaginal conditions`
- `1e0ad9d tests: add declarative retrieval differential case`
- `c065a46 rpc: relay speech output across models`
- `9a04221 motor: warn on invalid cursor locations`
- `3e67294 model: reject malformed model files`
- `18a1292 model: support model-body model-output`
- `d46f04b engine: drop invalid scheduled command events`
- `00a8503 rpc: align multi-model traces and load warnings`
- `0addd3b engine: honor model :v on reset`

## Harness layout

```text
rust_actr7/fixtures/differential/
  capture.py       # start SBCL or Rust, wait until ready, run one client, stop
  run_cases.py     # run every cases/*/client.py under both and compare
  cases/
    count-basic/client.py
    declarative-no-noise/client.py
```

Each case is a directory with a `client.py` that imports the vendored
`actr7.x/tutorial/python/actr.py`, loads an absolute model path, and runs.
`run_cases.py` compares normalized non-empty lines plus the client exit code.

`tests/differential.rs` is the ignored wrapper:

```powershell
cargo test --test differential -- --ignored --nocapture
```

## What is fixed

- Trace monitor callbacks now include the trailing newline SBCL sends.
- Load-time reset warnings are replayed when trace monitors are attached.
- `:v nil` is honored on reset.
- Multi-model warnings include the model name; trace columns use dynamic width.
- Invalid scheduled commands are dropped instead of emitting `NONE` trace lines.
- Model-body `(model-output "…")` is supported.
- Malformed model files are rejected.
- Invalid `move-cursor` locations emit SBCL's warning; valid visual-location
  chunks are still accepted.
- `set-audloc-default` supports `- slot value`; self-speech can produce
  `find-sound-failure` instead of a self aural-location.
- `output-speech` is a registered command, and fired commands are drained from
  inactive models as well as the current model.

## Open issue 1: declarative-issues activation noise

Short run is fixed. The remaining mismatch is the longer run with activation
noise (`:ans .25`), at the second retrieval:

```text
SBCL: 0.243 RETRIEVED-CHUNK 2-1=1
Rust: 0.245 RETRIEVED-CHUNK ZERO
```

With `:ans nil` the full trace matches exactly. The committed Rust regression is
`tutorial_unit5modeling_long_no_noise` (`tests/cases/`), and the differential
case is `declarative-no-noise`.

The activation trace (`set :act t`) shows the root difference is candidate
order, which shifts every noise draw:

```text
SBCL order starts: V3, 1-1=0, 2-1=1, 3-2=1, 1+1=2, 1+2=3, V1, V2, ...
Rust order starts: 1-1=0, 2-1=1, 3-2=1, 1+1=2, 1+2=3, V1, V2, V3, ...
```

SBCL then picks `2-1=1` with activation `2.8939683`; Rust picks `ZERO` with
`2.8666682`. This looks like SBCL hash-table iteration order, not a simple
request/type-filter bug. Reproduce with a temporary case whose `client.py` is:

```python
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "actr7.x" / "tutorial" / "python"))

import actr

actr.load_act_r_model(str(ROOT / "actr7.x" / "tutorial" / "unit5" / "declarative-issues.lisp"))
actr.set_parameter_value(":act", True)
actr.run(1.0)
```

Options:

1. Try to emulate SBCL's hash iteration order. Likely high effort and
   implementation-specific.
2. Treat this as a documented stochastic-order limitation and add
   expected-failure metadata to the differential harness.
3. Investigate whether `set-base-levels` or DM insertion can be matched more
   narrowly, but preserve all existing refs.

Recommendation: option 2 unless exact noisy-trace parity is a requirement.

## Open issue 2: multi-model-talking full task

The bare model now produces `find-sound-failure` for self speech, and the
committed RPC fixes allow `output-speech` callbacks and cross-model sounds.
The intended Python task still does not match. Temporary probe:
`C:\Users\sabetts\AppData\Local\Temp\opencode\probe_talking.py` (if still
present). It loads the absolute multi-model file, adds the `relay-speech-output`
command, monitors `output-speech`, installs `["speech","microphone"]` on every
model, and runs 10 seconds.

Observed after the partial fixes:

- M1/M3 relay sounds are still missing/misordered in the first run.
- The Rust client trace is emitted multiple times during nested
  `set-current-model` callbacks from the relay handler.
- SBCL produces one continuous multi-model trace ending around `1.775`.

Likely areas:

- `Session::take_fired_commands` and multi-model fired-command draining.
- `set-current-model` / `activate_model` during a run.
- `global_trace` / `global_trace_pos` handling when models are switched inside
  a callback.
- Whether `rebuild_global_trace` is invoked during the relay callback.

Recommended next step: build a minimal two-model differential case:

1. Listener model with a `hear` production.
2. Speaker model that produces `SPEAK TEXT hello`.
3. Python installs speech devices, adds an `output-speech` monitor that calls
   `new_word_sound` on the listener.
4. Run both servers, compare callback order and trace.
5. Add nested `set_current_model` inside the relay handler and isolate the trace
   duplication.

Add the resulting regression to `tests/environment_rpc.rs` and a small
differential case once it passes.

## Out of scope

Do not spend time on:

- `define-model-fct`
- custom module SGP parameters (`:create-delay`, `:demo-param`)
- `style-test-model` / `extend-manual-requests`
- SBCL warning line wrapping
- Lisp compiler/source warnings

These account for the remaining known load-only probe diffs.
