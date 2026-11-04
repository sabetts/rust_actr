# Native Rust Environment Work

This document is the handoff and plan for the native `actr env` GUI in
`rust_actr7/src/env/` (feature `env`). The goal is functional parity with the
upstream Tcl/Tk Environment in `actr7.x/environment/gui/`: every control,
inspector, history, graph, stepper flow, experiment-window operation, and
option available there should be available here.

The Tcl bridge work is tracked separately in `ENVIRONMENT-WORK.md`. This file
now owns the native GUI program.

## Scope decisions

- Tools are **floating egui child windows** inside one application window.
  The Tcl Environment uses separate toplevels; floating windows are the
  chosen Rust equivalent.
- The built-in code editor (Open/New/Save, backups, paren matching) is a
  **later milestone**. Loading model/code files through the OS file dialog is
  in scope now.
- Python module import is **not in scope**. Upstream only exposes
  `Python-import-from-file` when a Python client registers it.
- `999-*` overlays (images, EMMA eye spot, touch fingers) are in scope in a
  later milestone.
- Both backends must expose the same event surface:
  - **in-process**: the GUI owns `Session`;
  - **client** (`--connect`): the GUI speaks the RPC protocol to `actr server`.

## Current status

The native GUI is an early subset of the Tcl Environment. It already has:

- in-process and RPC-client backends;
- load model, reset, run/run-full-time/step/stop (in-process), trace drainage;
- trace pane; declarative, procedural, buffers, and parameters viewers;
- a basic production transition graph;
- visible experiment windows with text/button/line/remove and
  attention/cursor ops, click forwarding;
- persistent main-window size, controls width, sort lists, real time.

Known gaps versus the Tcl Environment:

| Area | Tcl Environment | Native after M1 |
|---|---|---|
| Model lifecycle | model menu, load code, reload, creating/deleting monitors | model menu, load code/model, reload, monitors/events |
| Run control | run/reset/reload/stop, running indicator, event queue | run/reset/reload/stop, indicator, event queue; client runs are non-blocking |
| Inspectors | DM + why-not, chunk tree, procedural matches/why-not, buffers, visicon, audicon, parameters | basic DM/procedural/buffers/parameters as floating tools |
| Histories | trace (detail/time), retrieval, blending, buffer, visicon, audicon, production | trace only |
| Graphs/BOLD | production grid + 7 graph processors, 2 graphic traces, BOLD graph/2D/3D | one simple graph |
| Stepper | full stepper + tutoring | placeholder tool |
| Experiment windows | text/button/line/image, attention/cursor/eye/finger, keys+clicks, multiple titles | text/button/line/attention/cursor, clicks |
| Options/tools | 10 options, geometry/colors, event queue, connections, editor, about | all 10 options, tool geometry/colors, event queue, connections, about |
| Multi-model UI | client-side model menu | client-side model menu |

## Milestone program

### M1 — Shell, events, and run-control parity (complete)

Objective: match the Tcl control panel and event plumbing so later tools have
a stable foundation.

Workstreams:

1. **Backend event layer.**
   - `env/backend.rs`: add `BackendEvent` and `TraceStream`.
   - `env/inproc.rs`: synthesize run events, drain `Session::take_pending_signals`,
     expose tagged trace, route generic client commands through a real remote.
   - `env/client.rs`: register create/delete/run-start/run-stop monitors plus
     the four trace streams; generic callback routing.
2. **Run control and server pumping.**
   - Add `schedule-break` / `schedule-break-relative` to `Session` and
     `BUILTINS`; reuse `Event::Break`.
   - Generalize `pump_stepper` so ordinary requests (Stop, event queue) are
     serviced between run slices on the same connection.
   - Run client-mode `run` on a worker thread so the UI stays responsive.
3. **Shell.**
   - Current Model menu from `mp-models` + create/delete events.
   - Load code/model, reload, reset, run duration, run, stop, running
     indicator.
   - Model-scoped commands pass the selected model explicitly.
4. **Tools.**
   - Event Queue (`mp-queue-text`).
   - Connections & Commands (`list-connections`, `check`).
   - About (`act-r-version`), status bar, error log.
5. **Floating tool-window registry.**
   - Kind + model + instance id, `<title> (<model>)`, reuse/raise, model-delete
     cleanup, persisted geometry.
   - Migrate the existing four viewers into registry windows.
6. **Options and settings.**
   - All Tcl options: `update_when_stepped`, `use_env_window`,
     `show_copyrights`, `use_localhost`, `kill_model_windows`,
     `use_smart_load`, `save_before_reload`, `save_backups`, `sort_lists`,
     `edit_warn`; keep native `real_time`.
   - Per-tool and experiment-window geometry; production colors.

M1 exit criteria:

- load code/model, reload, reset, run, stop work from the native GUI in both
  in-process and client modes;
- model menu tracks multi-model and model-scoped commands target the selection;
- running indicator, event queue, connections, about, options, and error log
  work;
- existing viewers are floating tool windows with persisted geometry;
- `cargo test` is green.

Completed and verified:

- Backend events cover tagged trace, model create/delete, run start/stop,
  window ops, status, and stepper callbacks in both backends.
- `schedule-break` / `schedule-break-relative` exist; the server services
  ordinary requests between capped run slices, so Stop works on the same
  connection; client-mode runs no longer block the UI.
- Control panel: model menu, load code/model, reload, reset, run duration,
  run/run-full-time/step, Stop, running indicator, options.
- Tools: Declarative, Buffers, Procedural, Parameters, Text Trace, Production
  Graph, Event Queue, Connections, About; floating windows with persisted
  geometry.
- Settings: all Tcl options, production colors, tool geometry.
- Tests: `cargo test` 559 passed / 8 ignored; env tests 8 passed;
  `environment_rpc` 10 passed; `stepper_rpc` 2 passed; `cargo fmt --check`
  clean.

Known M1 follow-ups (moved into M2):

- Viewer tools now have one instance per model and refresh on the global
  refresh tick.
- Reload still uses the server default compile behavior; Load honors
  `use_smart_load`.
- Connections shows command names but not the `check` documentation pane yet.
- Clear All has no control-panel button, matching the Tcl Environment.
### M2 — Inspectors parity (complete)

DM why-not, chunk tree, procedural match markers and why-not, buffers
contents/status and multi-buffer chunks, standalone buffer status, visicon,
audicon, complete parameter viewer; per-model windows and focus refresh.

Completed:

- Declarative viewer: slot filters, chunk list/details, and "Why not?".
- Chunk Tree: filters/list plus a recursive slot tree using
  `chunk-filled-slots-list`, `chunk-slot-value`, and `chunk-p`.
- Procedural viewer: Yes/No match markers from `no-output … whynot`, details,
  and "Why not?".
- Buffer viewer: Contents/Status toggle, empty buffers, multi-buffer chunks,
  and standalone Buffer Status.
- Visicon and Audicon viewers.
- One tool window per kind **per model**, titled `<title> (<model>)`; deleting
  a model closes its tools when `kill_model_windows` is enabled.
- Tool refresh is per-window and follows the model that opened it.

Verified: env tests 11 passed; `environment_rpc` 10 passed; `stepper_rpc`
2 passed; `cargo fmt --check` clean.

### M3 — Histories parity

Generic record/get/save/load/incremental framework with warning capture, time
filters, detail levels, and saved-file comments. Trace, retrieval, blending,
buffer, visicon, audicon histories. Production history raw stream.

### M4 — Graphs and BOLD parity

Production grid; the seven `production-graph-*` processors; horizontal and
vertical buffer graphic traces; buffer picker; BOLD graph, 2D brain, 3D brain;
EPS/PS/DOT export where practical.

### M5 — Stepper parity

Start/stop/step/run-until (production/time/module), Step All, Pause, detail
panes, item selection, tutor modes (production instantiation,
conflict-resolution, start-retrieval), reset/clear-all lifecycle.

### M6 — Experiment window and overlay parity

Image item, EMMA eye spot, touch fingers, keyboard forwarding, select/open/close
semantics, multiple windows per model (engine/session work), AGI extension
handlers.

### M7 — Editor and remaining utilities

Open/New/Save editor with backups and paren matching. Python import remains
out of scope.

## Architecture notes

- `EnvBackend` is the transport boundary (`env/backend.rs`). Both `InProc` and
  `Client` must implement events, run control, window ops, signals, and
  histories.
- `Session::evaluate` temporarily switches the current model for an explicit
  `model` argument. The native GUI should pass the selected model for
  model-scoped commands and `None` for global commands, matching the Tcl
  client's `nil`.
- The RPC server creates a separate `Session` per connection, so Stop must be
  serviced on the same connection as the running request. Do not use a second
  connection for control.
- Trace-time units are mixed upstream: history records use milliseconds; BOLD
  prediction arrays use seconds; trace-history start/end entries are seconds.
  Keep the conversions at the viewer boundary.

## Verification

From `rust_actr7/`:

```powershell
cargo fmt --check
cargo test
cargo test --features env --quiet --lib env::tests
```

Manual smoke (desktop session required):

```powershell
cargo run --features env -- env
cargo run --features env -- env --connect 127.0.0.1:2650
```

## Definition of done

- Every control-panel action in the Tcl Environment has a native equivalent.
- Every inspector, history, graph, BOLD view, stepper flow, and
  experiment-window operation is available.
- Options and window layout persist across launches.
- In-process and client modes behave equivalently.
- `cargo test` and the env-feature tests stay green.
- Remaining intentional exclusions are documented here with reasons.
