# Rust ACT-R Python Interface — Current State

Last updated: 2026-11-22
Branch: `main`
Latest relevant commit: `0ab3f25 python: add categorize example memories from the task`

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
  `take_pending_host_action`, `complete_host_action`,
  `set_sim_hook`/`set_ppm_hook` (session-wide), command
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
- `sgp`, `spp`; parameter names accept `key` or `:key`, with `_` rendered as `-`
- `set_similarities`, `set_base_levels`, `set_all_base_levels`
- `declare_buffer_usage` (`all_slots=True` for `:all`), `set_visloc_default`
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
- Python `:ppm-hook` functions are supported. They receive
  `(production, [(buffer, slot, expected, actual, similarity), ...])` with
  typed values, can return `None`/`False` to fall through to the Lisp hook or
  default offset, and are auto-bound by `actr.use()`/`actr.add_command`.
- Python `imaginal-action` callbacks use a host-yield step: the engine pauses,
  Python runs the action with model access, then the engine places the result.
- Scheduled task commands (`schedule-event-relative` and friends) registered
  with `actr.add_command` are recorded as fired task commands and dispatched
  between events (`pm-issues` uses this for its prompt).
- `mod-chunk` and `copy-chunk` work; task-facing slot reads and copies use the
  live buffer chunk (including modifications made by the current production).
- Runtime model/task loading is filesystem-path-only. `ACT-R:` logical names
  and `ACTR_DIR` are fixture/regeneration-tooling concepts, not runtime API.
- Tutorial task modules bind their functions to their own model, so importing
  several modules in one process no longer makes later calls use the last
  model.
- `new-word-sound` schedules `WORD` audio events using the speech articulation
  time; tone and digit sounds remain supported.
- `:syllable-rate`, `:char-per-syllable`, `:use-syllable-table`, and
  `:split-vocal-strings` are runtime parameters. `register-articulation-time` /
  `get-articulation-time` and the syllable-count commands are available, with
  the CMU dictionary embedded in the engine.
- Installing a device makes speech notify the host via `output-speech`.

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

Every checked-in script for units 1-8 now has a smoke test in
`python/tests/test_tutorial_units.py`: the matrix runs each model and calls
the smallest trial/experiment entry point for each unit. This proves the
ported scripts execute; it does not compare every experiment summary against
its vendored `.ref`. Protected solution models (Zbrodoff, choice, categorize,
subitize) remain unavailable upstream.

The categorize starter creates its 10 example chunks from the task after load
because model-body `call-act-r-command` only resolves Lisp defuns.
`categorize.experiment` supports the tutorial's offset and alternate feature
slot names; changing slots rebuilds the model and regenerates the examples.
The checked-in module is copied from `tools/task_overrides/categorize.py`.

### 3.4 Tutorial notebooks

The tutorial notebooks are ported next to the unit scripts:
`tutorial/tutorial.ipynb` and `tutorial/unit{1..8}/unit{1..8}.ipynb`. They
import the ported Python modules instead of loading Lisp files. The facade now
tracks the module behind the current model, so `actr.use_module(module)` and
`actr.reload()` re-import the ported module and pick up edits. Unit 7/8
alternate paired models are supported by rebinding the task module:
`actr.bind_module(paired.__dict__, model)`.

Unit 5's history walkthrough is covered by new runtime helpers:
`get_history_data`, `saved_activation_history`, `print_activation_trace`,
`print_chunk_activation_trace`, `print_dm_finsts`, and `mp_show_queue`.
`saved-activation-history` is backed by the engine's existing `:sact` traces.

Cells that need a human participant, placeholder templates, and heavy
experiments are tagged `rust-actr-skip`. `broken-addition` remains Lisp-only.
`tools/port_notebooks.py` regenerates the notebooks and
`tools/run_notebook.py --all` executes them headlessly.

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

### 4.3 Similarity and PPM hooks

`Engine::set_sim_hook` receives an `Rc<dyn Fn(&HookValue, &HookValue) -> Option<f32>>`.
`HookValue` is a public typed value (`Sym`, `Int`, `Num`, `Str`, `Nil`).
`Engine::set_ppm_hook` receives an `Rc<dyn Fn(&str, &[PpmHookRow]) -> Option<f32>>`;
each `PpmHookRow` carries the buffer, slot, typed expected/actual values, and
similarity. `Session::set_sim_hook`/`set_ppm_hook` apply the hook to the
selected engine, inactive engines, and models loaded later.

The Python binding converts both hooks to native Python values. Python
`False`/`None` is treated as "no numeric result" rather than `0.0`; otherwise
procedural partial matching accepts every mismatch. The Lisp `:sim-hook`/
`:ppm-hook` task-function path remains the fallback when the callback returns
no value.

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
| `rust_actr7/python/tutorial/` | self-contained tutorial model/task scripts and notebooks |
| `rust_actr7/python/tools/port_tutorial.py` | model/task port generator |
| `rust_actr7/python/tools/task_overrides/categorize.py` | hand-maintained categorize task source copied by the generator |
| `rust_actr7/python/tools/port_notebooks.py` | notebook port generator |
| `rust_actr7/python/tools/run_notebook.py` | headless notebook executor |
| `rust_actr7/python/tests/test_model.py` | builder/runtime/device unit tests |
| `rust_actr7/python/tests/test_execution.py` | stepping, callbacks, sim/ppm hooks, bst-ppm, categorize examples, siegler |
| `rust_actr7/python/tests/test_tutorial_units.py` | per-unit smoke matrix for every ported unit 1-8 script |
| `rust_actr7/python/tests/test_notebook_support.py` | reload, rebind, history helpers |
| `rust_actr7/python/tests/test_notebooks.py` | notebook smoke tests |
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
python tutorial/unit8/categorize.py experiment 1
```

Direct Python:

```python
import rust_actr.actr as actr
from tutorial.unit5.grouped import build

actr.use(build())
print(actr.run(20))
```

The scripts no longer need an ACT-R checkout; they import only `rust_actr`.

Run tutorial notebooks headlessly:

```powershell
python tools/run_notebook.py --all
```

Launch Jupyter from `rust_actr7/python` and open
`tutorial/tutorial.ipynb` for the interactive versions. Do not run tagged
human-mode cells without an ACT-R Environment.

## 7. Tests

```powershell
cd rust_actr7/python
python -m unittest discover -s tests -q
```

Current Python suite: **91 tests passing**. It covers:

- builder source and runtime values
- runtime DM/buffers, callbacks, devices
- stepped-vs-fast trace equality
- mid-run callback timing
- Python sim-hooks and ppm-hooks (explicit and auto-bound)
- `mod-chunk`
- imaginal-action host yield
- `siegler` digit/speech
- `zbrodoff` set/block
- `bst-ppm` live task
- categorize example-memory creation
- categorize offset and custom-slot regeneration
- per-unit smoke coverage for every ported unit 1-8 model/task script
  (`tests/test_tutorial_units.py`)
- notebook reload/rebind and history helpers
- headless execution of every tutorial notebook
- tutorial script compilation and CLI smoke tests
- multiple tutorial modules in one process
- word sounds and articulation/syllable commands

Rust suite (engine):

```powershell
cd rust_actr7
cargo test
```

Current result: **765 passed / 10 ignored** across the full `cargo test`
run: 129 library/unit tests, 560 model cases, and the other integration
binaries. The ignored cases are the protected/blocked upstream solution
sources and the environment-gated RPC test.

## 8. Known limitations and next steps

1. **Full `.ref` parity for every experiment summary.** The runtime paths and
   per-unit smoke tests now exist; expanding tests to compare every tutorial
   experiment against its vendored reference is the next verification step.
2. **Arbitrary Lisp.** The engine supports the tutorial's bounded Lisp
   evaluator, not arbitrary Lisp. Python callbacks cover the gaps. Model-body
   `call-act-r-command` forms still resolve only Lisp defuns; categorize
   therefore adds its examples from the task.
3. **Multiple models.** The Python façade binds tutorial modules to their own
   models, but it still targets one current model at a time; multi-model files
   remain out of scope.
4. **Audio rendering.** Word/tone/digit sound events and speech articulation
   are modeled, but no waveform/voice playback is produced.
5. **Protected/starting models.** `choice` and the full `categorize` solution
   are not available upstream; `subitize`/unit2 assignment are starting models.
   The categorize starter creates its 10 example memories from the task and
   supports offset/custom feature slots; only the protected solution productions
   are missing.
6. **Performance.** The JSON command bridge and one-event stepping are chosen
   for correctness; a typed bridge or batched stepping would be useful for
   large experiments.
7. **Tutorial packaging (by design).** The tutorial tree is a checkout-only
   learning resource, not wheel content. Its scripts and notebooks build
   models from in-memory source and do not require `ACTR_DIR`.
8. **Notebook editing uses the Python ports.** The notebooks edit the
   `tutorial/unitN/*.py` modules rather than the original `.lisp` files.
   `broken-addition` stays Lisp-only.
9. **Notebook human-mode cells.** Interactive `human=True` cells need the
   pending Python-to-Environment bridge; they are tagged `rust-actr-skip` and
   documented instead of run headlessly.
10. **Model-body host commands.** `call-act-r-command` forms in a model body
    resolve only through Lisp defuns; Python task commands are not dispatched.
    The categorize port works around this from the task. A generic bridge would
    queue host-owned model-body commands and run them after `_ensure_session`
    and reset, and it would also need a reset-cleared runtime DM add (or
    removal) to avoid duplicate examples; see `PARITY-AUDIT.md`.

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
- `68cf2d9` python: bind tutorial task functions to their own models
- `d97b58d` engine: add word sounds and syllable-count speech support
- `86a2b18` engine: let remove-syllable-count hide dictionary entries
- `c392fee` engine: make model loading filesystem-path-only
- `c95627d` rpc: enforce single-model environment sessions
- `091ef11` rpc: emit environment run and reset signals
- `0d85d73` rpc: reload models from disk and clear all
- `fbff262` rpc: expose environment module names to the stepper
- `13e3d43` rpc: complete environment viewers and standard histories
- `c94b3c8` tests: add a fake-tcl environment rpc harness
- `74502b6` tests: opt regression harnesses into multi-model loading
- `508b637` rpc: tag trace output by stream
- `802212d` rpc: filter histories by start and end time
- `bd311e4` rpc: expose command docs and connection counters
- `37b69a6` rpc: expose multi-buffer goal chunks
- `d1e5b85` tests: cover stepper tutoring conflict payload
- `bfcbf56` rpc: support multiple loaded models
- `73edb78` rpc: cover model-scoped histories and multi-model loading
- `865019a` tests: cover multi-model stepper callbacks
- `858ad0d` rpc: route multi-model windows and device actions
- `fc8fa75` history: preserve trace detail and expand blending history
- `27cf483` engine: add embedder ppm-hook callback and session hooks
- `0a7cec8` python: expose ppm-hook callback and bind pending hooks
- `0ab3f25` python: add categorize example memories from the task
- `6c52ee5` test: cover embedder hooks and host actions
- `fb5d3d3` python: support categorize offsets and custom slots
