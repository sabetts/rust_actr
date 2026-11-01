# Rust ACT-R Python Interface — Current State

Last updated: 2026-10-31  
Branch: `main`  
Latest relevant commit: `261ae69 python: coerce numeric chunk slots and cover bst-ppm`

This document records the Python-interface work added on top of the Rust
`actr-engine`: what exists, how it fits together, what is verified, and what
remains.

## 1. Goal

Make the Rust ACT-R engine usable directly from Python, including the ACT-R
tutorial models and their experiments, without an SBCL/Lisp runtime.

The interface is in-process: Python builds a model, the Rust engine executes it,
and Python can register task callbacks and inspect/mutate model state between
events.

## 2. Architecture

### 2.1 Native layers

`rust_actr7/python/src/lib.rs` exposes two PyO3 classes:

- `_native.Engine` — the original source-only engine wrapper:
  `run`, `run_until`, `reset`, trace, time.
- `_native.Session` — a wrapper around `actr_engine::rpc::session::Session`, the
  same command layer the RPC server uses. It supports:
  `evaluate(name, args_json)`, `run_begin`, `run_chunk`, `run_slice`,
  `step_once`, `take_trace`, `take_fired_commands`,
  `take_fired_task_commands`, `take_pending_device_actions`,
  `take_pending_host_action`, `complete_host_action`, `set_sim_hook`, command
  registration/monitors, and model time/event counts.

The native command protocol is JSON over `Session::evaluate`, matching the RPC
wire shapes (`[[args...]]` / result arrays). This avoids a large typed PyO3
surface while reusing the engine's existing command parsing.

### 2.2 Python layers

- `rust_actr.Model` (`python/rust_actr/_model.py`)
  - Builds a `define-model` source from Python calls.
  - Compiles lazily into a native `Session`.
  - Owns trace history, command/monitor registries, current window, and buffer
    type declarations.
- `RuntimeMixin` (`python/rust_actr/_runtime.py`)
  - Runtime commands: run/reset, buffers, DM, parameters, windows, devices,
    scheduling, audio, history/introspection, RNG, statistics.
  - Runs through the native session; no Lisp runtime.

### 2.3 Tutorial scripts

- `rust_actr.actr` (`python/rust_actr/actr.py`) is a module-level façade that
  matches the tutorial `actr.py` client. `use(model)` binds the current model
  and replays pending commands; `clear_state()` drops both.
- `rust_actr7/python/tutorial/unitN/` contains self-contained scripts.
  Model-only scripts build `m = actr.Model(...)` at module level and run it
  under `if __name__ == "__main__":`; task scripts expose `build(seed)` plus
  their task code (`MODEL = actr.use(build())`) and a small `main()`.
- `python/tools/port_tutorial.py` regenerates the tree from the vendored Lisp
  models and Python task scripts, lowercasing symbols and preserving each
  production's original multi-line layout.

## 3. What works

### 3.1 Model construction

`Model` supports:

- `chunk_type(name, *slots, defaults=..., include=...)`
- `add_chunk`, sparse `add_dm`, `define_chunks`
- named and inline `goal_focus`
- `add_production` (full Lisp production text)
- `sgp`, `spp`
- `set_similarities`, `set_base_levels`, `set_all_base_levels`
- `declare_buffer_usage`, `set_visloc_default`
- `call_act_r_command`
- initial `set_initial_buffer`
- model-body `install_device`
- raw `form(...)` escape hatch

Definitions freeze after the first runtime call. Runtime methods then apply to
the live session.

### 3.2 Runtime and task behavior

- Runs use a stepped path when callbacks/commands are registered, dispatching
  task monitors and `!eval!` commands between engine events. The stepped path
  is trace-identical to the fast path on the covered cases.
- Windows/buttons/lines use the tutorial client's `[[key, value], ...]` option
  shape, so text, position, size, visibility, and actions are honored.
- `output-key` and `output-speech` monitors call Python callbacks mid-run.
- Button `:action`s are surfaced to Python and dispatched.
- Python `:sim-hook` functions are supported. They receive typed Python values
  (numbers as numbers, strings as strings), can return `None`/`False` for
  "no similarity", and are auto-bound by `actr.use()`.
- Python `imaginal-action` callbacks use a host-yield step: the engine pauses,
  Python runs the action with model access, then the engine places the result.
- Scheduled task commands (`schedule-event-relative` and friends) registered
  with `actr.add_command` are recorded as fired task commands and dispatched
  between events (`pm-issues` uses this for its prompt).
- `mod-chunk` and `copy-chunk` work; task-facing slot reads and copies use the
  live buffer chunk (including modifications made by the current production).
- `new-digit-sound` schedules digit audio events, and installing a device makes
  speech notify the host via `output-speech`.

### 3.3 Tutorial status

Working end-to-end (verified by tests or direct runs):

- `count`, `addition`, `semantic`
- `demo2`
- `fan-no-pm`
- `grouped`
- `onehit` (hands/blocks/learning)
- `zbrodoff` (set/block/experiment)
- `sperling` (trial; experiment code path)
- `pm-issues` (scheduled prompt command)
- `paired`, `bst`, `past-tense`, `pcomp-issues` (one-trial/one-game runs)
- `bst-ppm` (live multi-trial `bst_set`; `experiment(1)` produces choices and
  utility values)
- `siegler` (digit sounds + `output-speech`)
- dynamic models that do not need task-side hooks

Starting models (no or partial productions by design):

- unit2 assignment
- `subitize`
- `choice`
- `categorize`

`unit1/broken-addition.lisp` is intentionally malformed upstream and is not
ported.

## 4. Important implementation notes

### 4.1 Stepped runs

`Model._run_stepped_result` uses:

1. `Session.run_begin(name, seconds)`
2. repeated `Session.run_chunk(limit, full, 1, None)`
3. after each chunk: absorb trace, complete pending host actions, dispatch
   window-device actions, dispatch fired task commands
4. continue when `run_chunk` says the run is still capped or a host/device
   action was handled

`run_chunk` emits the normal `Stopped because ...` terminator when it returns
`false`.

### 4.2 Callback timing

- `output-key`/`output-speech` callbacks run between engine events.
- `!eval!` and scheduled task commands carry evaluated arguments.
- Host actions (`imaginal-action`) and device actions can schedule new events;
  the stepping loop re-enters the engine after handling them.
- Nested `run` from a callback is rejected; other commands are allowed.

### 4.3 Similarity hooks

`Engine::set_sim_hook` receives an `Rc<dyn Fn(&HookValue, &HookValue) -> Option<f32>>`.
`HookValue` is a public typed value (`Sym`, `Int`, `Num`, `Str`, `Nil`).
The Python binding converts to native Python values. Python `False`/`None` is
treated as "no numeric result" rather than `0.0`; otherwise procedural partial
matching accepts every mismatch.

### 4.4 Live buffer reads

`Engine::chunk_slot_live` (used by RPC `chunk-slot-value`) prefers a buffer-
resident chunk over its named source. `Engine::copy_chunk_live_rpc` does the
same for task `copy-chunk`. This lets an `imaginal-action` callback see a slot
just written by the firing production without changing compilation/mapping
semantics (which continue to use `chunk_name_slot`).

## 5. Files

| Path | Role |
|---|---|
| `rust_actr7/python/src/lib.rs` | PyO3 `Engine` and `Session` bindings |
| `rust_actr7/python/python/rust_actr/_model.py` | builder + runtime host |
| `rust_actr7/python/python/rust_actr/_runtime.py` | runtime command surface |
| `rust_actr7/python/python/rust_actr/actr.py` | tutorial `actr` façade (`use`, `clear_state`) |
| `rust_actr7/python/tutorial/` | self-contained tutorial model/task scripts |
| `rust_actr7/python/tools/port_tutorial.py` | model/task port generator |
| `rust_actr7/python/tests/test_model.py` | builder/runtime/device unit tests |
| `rust_actr7/python/tests/test_execution.py` | stepping, callbacks, sim-hook, bst-ppm, siegler |
| `rust_actr7/python/tests/test_tutorial.py` | model compile, CLI smoke tests |
| `rust_actr7/src/engine/*` | engine changes for hooks, host actions, live reads, speech |

## 6. Build and run

```powershell
cd rust_actr7/python
python -m maturin build --release
python -m pip install --force-reinstall --no-deps target/wheels/rust_actr-0.1.0-cp39-abi3-win_amd64.whl
```

Run tutorial scripts:

```powershell
python tutorial/unit1/count.py
python tutorial/unit2/demo2.py
python tutorial/unit5/grouped.py
python tutorial/unit5/onehit.py hands 2
python tutorial/unit8/bst_ppm.py experiment 1
```

Direct Python:

```python
import rust_actr.actr as actr
from tutorial.unit5.grouped import build

actr.use(build())
print(actr.run(20))
```

The scripts no longer need an ACT-R checkout; they import only `rust_actr`.

## 7. Tests

```powershell
cd rust_actr7/python
python -m unittest discover -s tests -q
```

Current Python suite: **33 tests passing**. It covers:

- builder source and runtime values
- runtime DM/buffers, callbacks, devices
- stepped-vs-fast trace equality
- mid-run callback timing
- Python sim-hooks
- `mod-chunk`
- imaginal-action host yield
- `siegler` digit/speech
- `zbrodoff` 24-trial set
- `bst-ppm` live task
- tutorial script compilation and CLI smoke tests

Rust suite (engine):

```powershell
cd rust_actr7
cargo test
```

Current result: **559 passed / 8 ignored**. The 8 ignored cases are the
protected/blocked upstream solution sources and the environment-gated RPC test.

## 8. Known limitations and next steps

1. **Cross-task reuse in one long-lived process.** Loading several tasks into
   the same process can leave model/task state behind. Use a fresh
   `load_act_r_model`/process when switching tasks.
2. **Full `.ref` parity for every experiment summary.** The runtime paths now
   exist; expanding tests to compare every tutorial experiment against its
   vendored reference is the next verification step.
3. **Task packaging.** The wheel contains the Python model ports but not the
   vendored `actr7.x/tutorial/python` task scripts; `runner` currently requires
   an `ACTR_DIR` checkout.
4. **Arbitrary Lisp.** The engine supports the tutorial's bounded Lisp
   evaluator, not arbitrary Lisp. Python callbacks cover the gaps.
5. **Multiple models.** The in-process façade targets one current model.
6. **Voice synthesis.** `new-word-sound` remains a no-op; `new-digit-sound` and
   `new-tone-sound` are implemented.
7. **Protected/starting models.** `choice` and the full `categorize` solution
   are not available upstream; `subitize`/unit2 assignment are starting models.
8. **Performance.** The JSON command bridge and one-event stepping are chosen
   for correctness; a typed bridge or batched stepping would be useful for
   large experiments.

## 9. Milestone commits

- `698be56` python: extend Model builder for tutorial model bodies
- `e273fc3` python: add session-backed runtime, devices, and callbacks
- `103d5f2` python: add tutorial model ports and task runner
- `7cce7f3` engine: add similarity hooks and chunk copy/modify commands
- `0f7eb99` python: dispatch callbacks mid-run and wire Python sim-hooks
- `12577ce` engine: yield Python imaginal-action callbacks to the host
- `73198c8` python: complete host-yield imaginal actions
- `bd6fcc7` engine: wire digit sounds and speech device notifications
- `6c42abc` python: cover siegler speech output
- `5b987d2` engine: pass typed values to similarity hooks
- `d4aec2d` python: dispatch window-button actions and typed similarity hooks
- `3123a85` python: send window options in the client's pair-list shape
- `3212fc0` python: treat non-numeric hook results as no similarity and continue after host actions
- `cdb4834` engine: read live buffer chunks for task slot reads and copies
- `261ae69` python: coerce numeric chunk slots and cover bst-ppm
