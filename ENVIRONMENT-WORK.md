# ACT-R Environment Work

This document is the handoff for the Rust ACT-R Environment work. It records
what has been completed and the agreed implementation plan for Milestone 3
(multi-model support).

The target is the **upstream Tcl/Tk ACT-R Environment** in
`actr7.x/environment/`, connecting to `actr server` over the RPC socket. The
native `actr env` GUI is a separate, deferred project.

## Current status

Milestones 1 and 2 are complete and committed:

- `9d923d8` docs: document filesystem-only runtime paths
- `c392fee` engine: make model loading filesystem-path-only
- `091ef11` rpc: emit environment run and reset signals
- `0d85d73` rpc: reload models from disk and clear all
- `fbff262` rpc: expose environment module names to the stepper
- `13e3d43` rpc: complete environment viewers and standard histories
- `c94b3c8` tests: add a fake-tcl environment rpc harness
- `b85c0db` rpc: route print-warning through warning-trace
- `dcaa8f4` docs: record the single-model environment milestone
- `508b637` rpc: tag trace output by stream
- `802212d` rpc: filter histories by start and end time
- `bd311e4` rpc: expose command docs and connection counters
- `37b69a6` rpc: expose multi-buffer goal chunks
- `d1e5b85` tests: cover stepper tutoring conflict payload
- `cc2d36b` docs: record environment parity hardening

Last verified results:

- Rust: **702 passed / 9 ignored**
- Python: **38 passed**
- `cargo test --features env --quiet --lib env::tests`: **3 passed**
- `cargo fmt --check`: clean

Only the pre-existing untracked
`actr7.x/environment/gui/init/05-current-net.tcl` is present.

## Confirmed scope decisions

- The target is the Tcl/Tk Environment, not the native `actr env` GUI.
- Multi-model is the Milestone 3 feature.
- `999-*` AGI overlays are deferred.
- The native `actr env` GUI is deferred.
- Python remains single-model; no multi-model Python API work.
- The stepper is included in Milestone 3.
- No `with-model-eval`-style host callbacks.
- Arbitrary Lisp/task execution remains unsupported.
- Advanced Lisp-task examples such as `multi-model-talking` may remain
  unrunnable.
- Structured history/BOLD records must match the Tcl parser; plain text
  formatting is functional parity, not byte-identical parity.

## Milestone 3 progress

Completed in the working tree and committed:

- `bfcbf56` rpc: support multiple loaded models
- `73edb78` rpc: cover model-scoped histories and multi-model loading
- `865019a` tests: cover multi-model stepper callbacks

Implemented so far:

- Ordered model set with a selected engine and inactive engines.
- Multiple models from separate files and from one file.
- `mp-models` returns load order.
- `current-model` and `set-current-model` switch the selected engine.
- Explicit RPC `model` arguments switch command context and restore it after
  model-scoped commands.
- Multi-model `run` interleaves all engines on the shared clock and emits the
  model-name trace column.
- Multi-model `reset` resets every engine.
- `clear-all` removes every engine and model.
- Reload replaces the last model file's group while retaining unrelated
  models.
- Per-model viewer and history commands work through the selected model.
- Stepper hooks know which model paused/stepped, and pause/update callbacks
  carry the model name.
- Server-level fake-Tcl stepper callbacks are tested across two models.
- Multi-model command tests and multi-model Tcl trace/clear-all tests pass.
- Regression harnesses retain their historical last-model loading semantics
  via `set_load_replaces_models`.

Still to finish:

- Multi-model device/window association and callback routing.
- Broader per-model device/window tests.

## What currently exists

### RPC and Environment

`rust_actr7/src/rpc/server.rs` and `rust_actr7/src/rpc/session.rs` implement:

- `\x04`-framed JSON RPC with command registry, monitors, callbacks, check,
  set-name, list-commands, list-connections, and close.
- Multi-model RPC sessions; Python remains single-model.
- `run-start` / `run-stop`, `reset-start` / `reset-step1`,
  `clear-all-start`, creating/deleting model signals.
- Stream-tagged trace output:
  - model
  - command
  - warning
  - general
- Warning routing, `mp-queue-text`, and event queue support.
- Disk-backed reload and clear-all.
- Time filters for histories.
- Command documentation and connection sent/received counters.
- Standard viewers and histories, including audicon and a simplified blending
  history.
- Lisp-compatible history save/load envelopes and comment-first incremental
  transfer.
- Multi-buffer `GOAL` access for the threads extra.
- Stepper core behavior and tutoring/conflict payload coverage.

### Tests and fixtures

- `rust_actr7/tests/environment_rpc.rs` is the fake-Tcl conformance harness.
- `rust_actr7/tests/stepper_rpc.rs` drives the stepper over a real socket.
- `rust_actr7/fixtures/stepper/stepper-model.lisp` is the stepper fixture.
- `rust_actr7/tests/test_models.rs` and `tests/hooks/mod.rs` opt into
  multi-model loading for the existing engine regression tests.
- The same-file multi-model regression is
  `examples;multiple-models;unit-1-together-1-mp.lisp`, exercised by
  `rust_actr7/tests/cases/unit_1_together_1_mp.rs`.

## Remaining limitations

- Device and visible-window state is still session-global; per-model device
  and window routing remains to finish.
- Per-model viewer/history commands work by switching command context; those
  state stores are not yet independently modeled for every device family.
- Python remains intentionally single-model.
- Advanced Lisp-task examples remain blocked by the lack of `with-model-eval`
  and arbitrary Lisp execution.

## Milestone 3 objective

Support multiple models in the Rust RPC server and Tcl Environment:

- load multiple models from separate files
- load multiple models from one file
- list and switch the current model
- run/reset all models on the shared ACT-R clock
- target commands, viewers, and histories at a model
- support the stepper across models
- preserve existing single-model behavior
- preserve `unit-1-together-1-mp`

## Non-goals for Milestone 3

- `with-model-eval` and arbitrary Lisp task execution
- `multi-model-talking` and other Lisp-task examples
- multi-model Python API
- `999-*` overlays
- native `actr env`
- full threads task-language work beyond existing engine support
- byte-identical text formatting for all viewer panes

## Target architecture

### Ordered model store

Replace the `engine` + `multi` split with one ordered model store:

```text
ModelStore
  models: Vec<ModelRuntime>       // order = model menu order
  current: Option<ModelName>
  shared clock
  shared event sequence
  global trace with optional model column
```

Each `ModelRuntime` owns:

- model definition
- live `Engine`
- source file/group
- per-model histories
- per-model device/window state
- task functions/variables

`MultiEngine` should become the runner over this store, or be refactored into
a thin regression adapter over it. Keep a single source of truth for the live
engines.

### Command routing

`evaluate(name, model, args)` resolves a model in this order:

1. explicit model name
2. current model

Global commands bypass model selection:

- `mp-models`
- `current-model`
- `set-current-model`
- `run`, `run-full-time`, `run-until-time`
- `reset`
- `clear-all`
- `reload`
- `creating-model`, `deleting-model`

All other model-scoped commands use the resolved model's engine.

### Execution semantics

Match Lisp ACT-R:

- `run` advances **all models** on the shared clock.
- Events interleave by time, priority, and global insertion order.
- One global `run-start` / `run-stop`.
- One global trace; the model-name column appears only when more than one
  model exists.
- `reset` resets all models.
- `clear-all` removes all models.
- `reload` reloads the last loaded file group.
- `creating-model` fires per added model.
- `deleting-model` fires per removed model.

### Histories

Move history ownership from the session singleton to each model:

- retrieval
- buffer
- visicon
- audicon
- blending
- production
- buffer-trace/graphic
- BOLD

`trace-history` remains the global combined trace. History commands take a
model argument and default to the current model.

### Stepper

The stepper must operate on the model store:

- install a hook on every model engine
- record which model produced the paused/stepped event
- pass that model to `next-event` and `update-event` callbacks
- `update-stepper` uses the stepped model's snapshot
- `step-all` pauses on every model event
- run-until production/module/time applies across the global event stream
- reset/clear-all resets or closes the stepper correctly
- preserve single-model behavior

## Implementation phases

### Phase 1 — ModelStore and lifecycle

- Introduce `ModelRuntime` and `ModelStore`.
- Remove the RPC single-model guard and test-only `allow_multi_model` path.
- Support:
  - one file with several models
  - several files with one model each
- Implement order-preserving `mp-models`.
- Implement `current-model` and `set-current-model`.
- Implement delete/reload by file group.
- Keep `unit-1-together-1-mp` green.

Exit criteria:

- load two files, list both, switch current, clear-all removes both.

### Phase 2 — Global execution

- Move run/reset/step operations to the model store.
- Interleave events across models.
- Preserve model-column trace formatting.
- Aggregate run time/event count correctly.
- Fire global run/reset signals once.
- Associate device actions with their model.
- Keep one-model behavior unchanged.

Exit criteria:

- same-file and separate-file multi-model traces both have correct ordering
  and model columns.

### Phase 3 — Per-model commands and viewers

Refactor model-scoped handlers to use the resolved engine:

- `goal-focus`, `mod-focus`
- `add-dm`, `dm`, `sdm`, `sdp`, `whynot-dm`
- buffers and productions
- `sgp`, `spp`
- DM/procedural/buffer/visicon/audicon/parameter viewers
- experiment-window/device commands where model state exists

Exit criteria:

- selecting model A vs B changes command results and viewer data.

### Phase 4 — Per-model histories

- Move history recorders/tables into `ModelRuntime`.
- `record-history <model> ...` targets that model.
- Trace history remains global.
- History start/end filters, save/load, and incremental transfers use the
  target model.
- Add per-model history tests.

### Phase 5 — Multi-model stepper

- Install hooks on every model engine.
- Return the model name with pause/update callbacks.
- Select the snapshot from the stepped model.
- Make `step-all` work across models.
- Make run-until work across models.
- Ensure reset/clear-all closes or resets the stepper correctly.
- Preserve single-model stepper tests.

Exit criteria:

- fake-Tcl stepper pauses, steps, and updates across two models with the
  correct `model` argument.

### Phase 6 — Tcl model menu and fake-Tcl matrix

Extend `rust_actr7/tests/environment_rpc.rs`:

- two models from separate files
- two models from one file
- creating/deleting callbacks
- current model selection
- per-model viewer data
- per-model history data
- run all with model column
- reset all
- reload last model group
- clear-all
- multi-model stepper
- per-model history save/load

## Files likely to change

- `rust_actr7/src/rpc/session.rs`
- `rust_actr7/src/rpc/server.rs`
- `rust_actr7/src/engine.rs`
- `rust_actr7/src/engine/multi.rs`
- `rust_actr7/src/engine/debug.rs`
- `rust_actr7/src/engine/events.rs`
- `rust_actr7/src/engine/driver.rs`
- `rust_actr7/tests/environment_rpc.rs`
- `rust_actr7/tests/stepper_rpc.rs`
- `rust_actr7/tests/test_models.rs`
- `rust_actr7/tests/hooks/mod.rs`
- `PORTING-NOTES.md`
- `PYTHON-INTERFACE-WORK.md`

Reference Tcl files:

- `actr7.x/environment/gui/dialogs/06-ctrl-panel-show-loaded.tcl`
- `actr7.x/environment/gui/dialogs/21-stepper.tcl`
- `actr7.x/environment/gui/dialogs/91-dispatcher-status.tcl`
- `actr7.x/environment/gui/dialogs/99-exp-window.tcl`

Reference Lisp files:

- `actr7.x/framework/top-level.lisp`
- `actr7.x/framework/meta-process.lisp`
- `actr7.x/framework/model.lisp`
- `actr7.x/framework/dispatcher.lisp`
- `actr7.x/examples/multiple-models/unit-1-together-1-mp.lisp`
- `actr7.x/examples/multiple-models/multi-model-talking-models.lisp`

## Known risks

1. Most command handlers currently use `self.engine`; this is the largest
   refactor.
2. Histories currently live in one session-level table.
3. Device and visible-window state currently assumes one active model.
4. The stepper assumes one event stream and one snapshot.
5. Advanced Lisp-task examples remain blocked even after multi-model runtime
   support.
6. Running all models must preserve the current exact trace ordering used by
   `unit-1-together-1-mp`.

## Recommended commit points

1. `rpc: introduce the model store`
2. `rpc: load and switch multiple models`
3. `rpc: run and reset all models`
4. `rpc: route commands and viewers by model`
5. `rpc: make histories model-scoped`
6. `stepper: step across the model store`
7. `tests: cover the multi-model environment`
8. `docs: record multi-model semantics and limits`

Each commit should keep `cargo test` green.

## Verification commands

From `rust_actr7/`:

```powershell
cargo fmt --check
cargo test
cargo test --features env --quiet --lib env::tests
```

Python (after a maturin build/install when Rust APIs change):

```powershell
cd python
python -m maturin build --release
python -m pip install --force-reinstall --no-deps target/wheels/rust_actr-0.1.0-cp39-abi3-win_amd64.whl
$env:ACTR_DIR=$null
python -m unittest discover -s tests -q
```

The RPC conformance test is still environment-gated:

```powershell
cargo test --test rpc_conformance -- --ignored --nocapture
```

## Definition of done

- Two models can be loaded from separate files or one file.
- The Tcl model menu lists both and switches the selected model.
- Global run/reset/clear-all behave like Lisp.
- Viewers and histories follow the selected model.
- The stepper pauses and steps across models with correct model callbacks.
- `unit-1-together-1-mp` remains green.
- Single-model Tcl behavior remains green.
- Python remains green and single-model.
- `999-*`, native `actr env`, `with-model-eval`, and arbitrary Lisp remain
  documented as unsupported or deferred.
