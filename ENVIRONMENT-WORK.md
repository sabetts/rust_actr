# ACT-R Environment Work

This document is the handoff for the Rust ACT-R Environment work. It records
the completed environment milestones and the remaining Environment limits.

The target of this document is the **upstream Tcl/Tk ACT-R Environment** in
`actr7.x/environment/`, connecting to `actr server` over the RPC socket. The
native `actr env` GUI is a separate program tracked in
[`NATIVE-ENVIRONMENT-WORK.md`](NATIVE-ENVIRONMENT-WORK.md).

## Current status

Milestones 1-5 are complete and committed:

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
- `bfcbf56` rpc: support multiple loaded models
- `73edb78` rpc: cover model-scoped histories and multi-model loading
- `865019a` tests: cover multi-model stepper callbacks
- `858ad0d` rpc: route multi-model windows and device actions
- `fc8fa75` history: preserve trace detail and expand blending history

Last verified results:

- Rust: **710 passed / 9 ignored**
- Python: **38 passed**
- `cargo test --features env --quiet --lib env::tests`: **3 passed**
- `cargo fmt --check`: clean

Only the pre-existing untracked
`actr7.x/environment/gui/init/05-current-net.tcl` is present.

## Confirmed scope decisions

- The target of this document is the Tcl/Tk Environment; the native `actr env`
  GUI has its own program in `NATIVE-ENVIRONMENT-WORK.md`.
- Multi-model is the Milestone 3 feature.
- `999-*` AGI overlays are deferred for the Tcl Environment; the native GUI
  tracks them in M6 of `NATIVE-ENVIRONMENT-WORK.md`.
- Python remains single-model; no multi-model Python API work.
- The stepper is included in Milestone 3.
- No `with-model-eval`-style host callbacks.
- Arbitrary Lisp/task execution remains unsupported.
- Advanced Lisp-task examples such as `multi-model-talking` may remain
  unrunnable.
- Structured history/BOLD records must match the Tcl parser; plain text
  formatting is functional parity, not byte-identical parity.
- Environment code changes stay Rust-only. Upstream Tcl and Lisp ACT-R are not
  modified; Tcl-side bugs are documented or worked around in Rust.
- One experiment window per model. Multiple simultaneous windows inside one
  model remain unsupported.

## Milestone 3 and 4 progress

Completed and committed:

- `bfcbf56` rpc: support multiple loaded models
- `73edb78` rpc: cover model-scoped histories and multi-model loading
- `865019a` tests: cover multi-model stepper callbacks
- `858ad0d` rpc: route multi-model windows and device actions

Implemented:

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
- Experiment windows are tracked per model; opening/clearing/closing one
  model's window does not disturb another's.
- Window display callbacks and mouse clicks resolve the owning model.
- Device actions are tagged with their model and dispatched during single- and
  multi-model runs, including stepper runs.
- Multi-model `run` uses the interleaved runner and emits one global run
  terminator with the `-` model column.
- Reload, clear-all, and model replacement close the affected experiment
  windows. Upstream Tcl does not register experiment windows for deletion, so
  the Rust session explicitly sends `close`.

No Milestone 3 or 4 work remains outstanding.

## Milestone 5 progress: viewer and history fidelity

Completed and committed:

- `fc8fa75` history: preserve trace detail and expand blending history

Implemented:

- Trace lines retain their output level and model alongside the rendered text.
- `trace-history` records now contain `[time, detail, model, text]` with real
  low/medium/high metadata.
- Trace-history availability, incremental transfer, and save honor the Tcl
  detail-level filter together with time bounds.
- Multi-model trace history carries each event's model name.
- The blending history trace now includes the slot-by-slot details the
  Environment viewer displays: matched values, magnitudes, method, numeric
  accumulation, chunk comparisons, and final results.
- Per-model retrieval histories are verified to remain independent.
- Unknown trace details fail safe by remaining visible rather than being
  silently dropped.

Remaining known limits are listed below.

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
- Standard viewers and histories, including audicon and a detailed blending
  history trace.
- Trace-history detail-level filtering and per-event model metadata.
- Lisp-compatible history save/load envelopes and comment-first incremental
  transfer, including time/detail filters.
- Multi-buffer `GOAL` access for the threads extra.
- Stepper core behavior and tutoring/conflict payload coverage.
- Per-model experiment-window metadata, item ids, and display callbacks.
- Model-qualified device actions dispatched from single- and multi-model runs.
- Lifecycle cleanup that closes replaced/reloaded/cleared model windows.

### Tests and fixtures

- `rust_actr7/tests/environment_rpc.rs` is the fake-Tcl conformance harness,
  including per-model windows, clicks, device actions, and lifecycle closes.
- Session tests cover trace detail/model metadata, detail-filtered
  availability/save, full blending trace text, and per-model histories.
- `rust_actr7/tests/stepper_rpc.rs` drives the stepper over a real socket.
- `rust_actr7/fixtures/stepper/stepper-model.lisp` is the stepper fixture.
- `rust_actr7/tests/test_models.rs` and `tests/hooks/mod.rs` opt into
  multi-model loading for the existing engine regression tests.
- The same-file multi-model regression is
  `examples;multiple-models;unit-1-together-1-mp.lisp`, exercised by
  `rust_actr7/tests/cases/unit_1_together_1_mp.rs`.

## Remaining limitations

- One experiment window per model; multiple windows within one model are not
  supported by the current engine/session model.
- Window titles are still Environment-global identifiers because Tcl keys
  windows by title. Duplicate visible titles across models are resolved
  deterministically, with the selected model winning, but cannot be made fully
  independent without upstream Tcl changes.
- Upstream Tcl does not register experiment windows in its per-model window
  map, so Rust closes those windows explicitly on model replacement, reload,
  and clear-all. This is documented rather than patched upstream.
- Viewer text remains functionally compatible rather than byte-identical for
  every pane.
- Blending history exposes the complete viewer-facing trace and chunk data,
  but not the raw Lisp SBLT structure.
- BOLD records are shape-verified; real Tk graph/brain rendering is still a
  manual check.
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
- native `actr env` (tracked separately in `NATIVE-ENVIRONMENT-WORK.md`)
- full threads task-language work beyond existing engine support
- byte-identical text formatting for all viewer panes

## Milestone 4 objective (completed)

Make experiment windows and device actions model-owned without changing
upstream Tcl or Lisp:

- one live experiment window per model
- model-scoped window metadata and display callbacks
- clicks routed to the model that owns the window title
- device actions tagged with the model and dispatched during runs
- multi-model runs use the interleaved runner and stop for device actions
- reload/clear-all/replacement close only the affected windows
- Rust-side workaround for Tcl's missing experiment-window registration

## Target architecture

### Ordered model store

The session keeps one ordered model set rather than a separate `ModelStore`
type:

- `engine` is the selected live engine
- `inactive` holds the remaining engines in load order
- `model_order` supplies the model-menu order
- `windows` maps model names to their experiment-window metadata
- clock, event sequence, and trace are session-global

Each model's definition, live engine, histories, and window contents stay
together on that model's runtime state. `MultiEngine` remains the regression
runner over independent engines.

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

## Known risks and limits

1. One model has one live experiment window; multi-window models still need
   engine work.
2. Tcl uses window titles as global identifiers, so duplicate visible titles
   across models cannot be fully isolated without upstream Tcl changes.
3. Upstream Tcl does not register experiment windows for deletion; Rust sends
   explicit `close` operations instead.
4. Advanced Lisp-task examples remain blocked even after multi-model runtime
   support.
5. Running all models must preserve the exact trace ordering used by
   `unit-1-together-1-mp`, including the global `-` terminator.
6. Real Tk manual smoke testing is still separate from the fake-Tcl harness.

## Completed multi-model commit sequence

1. `bfcbf56` rpc: support multiple loaded models
2. `73edb78` rpc: cover model-scoped histories and multi-model loading
3. `865019a` tests: cover multi-model stepper callbacks
4. `858ad0d` rpc: route multi-model windows and device actions

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
- Trace-history records carry real detail/model metadata and honor the Tcl
  low/medium/high filter.
- Blending history includes the viewer's full slot-detail trace.
- Each model has an independent experiment-window record, and window
  commands/clicks follow the owning model.
- Device actions are dispatched with the owning model during multi-model and
  stepper runs.
- Reload, clear-all, and replacement close only the affected model windows.
- The stepper pauses and steps across models with correct model callbacks.
- `unit-1-together-1-mp` remains green.
- Single-model Tcl behavior remains green.
- Python remains green and single-model.
- `999-*`, `with-model-eval`, and arbitrary Lisp remain documented as
  unsupported or deferred. The native `actr env` GUI is tracked in
  `NATIVE-ENVIRONMENT-WORK.md`.
