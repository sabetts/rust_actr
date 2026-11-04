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

## Program status

Milestones M1-M7 now have initial implementations:

- M1 shell/backends, M2 inspectors, M3 histories, M4 graphs/BOLD, M5 stepper,
  M6 window overlays/keys, and M7 editor.
- Remaining work is concentrated in the per-milestone follow-ups: multi-pane
  history layouts, layered production-graph processors, full tutor interaction
  UIs, engine-emitted image/eye/finger ops, multiple experiment windows per
  model, and editor paren matching.

Last full verification:

- `cargo test`: 559 passed / 8 ignored.
- `cargo test --features env --quiet --lib env::`: 20 passed.
- `environment_rpc`: 10 passed.
- `stepper_rpc`: 2 passed.
- `cargo fmt --check`: clean.

## Remaining polish backlog

These are the known gaps after the initial M1-M7 implementations. They are
ordered by impact rather than milestone number.

### P0 — Gaps users will hit

Completed quick-win batch:

- Production Graph now acquires/releases `production-history`, sharing
  recording with other viewers through reference counts.
- `update_when_stepped` refreshes open inspector tools after each stepper
  update.
- Connections & Commands has a command field and `check` documentation pane.
- Editors prompt before discarding dirty files and persist their geometry.
- The main trace pane filters model/command/warning/general streams.

Remaining P0 items:

1. **History pane splits.** Retrieval, blending, buffer, and perceptual
   histories render as one text pane instead of the Tcl
   Times/Chunks/Details/Request/Activation panes.
2. **Production grid/graph extras.** No EPS/PS export, zoom, per-cell
   production/why-not popups, or full hide-empty semantics.
### P1 — Parity depth

1. Seven layered `production-graph-*` processors (all, frequencies, cycles,
   unique cycles, runs, unique runs, utilities) and their layout/export.
2. Graphic-trace zoom, hide-text/hide-grid, hover Request/Chunk/Notes
   readouts, and EPS/PS export.
3. Real 3D BOLD voxel mesh; per-buffer limiting through the Buffer Picker; raw
   scaling/clamping.
4. Full stepper tutor interaction windows: production binding answers,
   conflict-resolution checklist, and start-retrieval chunk checklist.
5. Engine/session emission of image/eyeloc/finger display ops; real image
   loading and click hit-testing.
6. Multiple experiment windows per model.
7. Chunk-tree filled-slot filtering and the full recursive canvas tree.
8. Option wiring: `use_env_window`, `use_localhost`, client-mode real-time,
   reload smart-load, and `show_copyrights` beyond startup.

### P2 — Nice-to-haves

- Trace stream colors or separate warning/command panes.
- GUI error-log viewer and fuller About/copyright text.
- Editor syntax highlighting.
- Client-mode real-time pacing.
- Python module import (intentionally excluded).

## Current status

The native GUI now covers the initial M1-M7 surface. It has:

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

### M3 — Histories parity (complete)

Generic record/get/save/load/incremental framework with warning capture, time
filters, detail levels, and saved-file comments. Trace, retrieval, blending,
buffer, visicon, audicon histories. Production history raw stream.

Completed:

- Generic history tool framework: open starts `record-history`, close sends
  `stop-recording-history`, Refresh uses `history-data-available` and
  `start-incremental-history-data`, Save/Load use `save-history-data` and the
  file-load path, and saved comments are shown.
- Trace history with low/medium/high/ALL detail filtering and start/end time
  filters.
- Retrieval, blending, buffer, visicon, and audicon history viewers with
  Tcl-shaped record rendering and per-model targeting.
- History tools are one per kind per model and refresh from the tool's model.
- Unit tests cover every history record shape and recording-on-open.

Verified: env tests 13 passed; `environment_rpc` 10 passed; `stepper_rpc`
2 passed; `cargo fmt --check` clean.

Known follow-ups (M3 polish): the multi-pane split layouts (separate
Times/Chunks/Details/Request/Activation panes) are currently combined into a
single scrollable text pane; production history/grid/graph are M4.

### M4 — Graphs and BOLD parity (initial)

Production grid; the seven `production-graph-*` processors; horizontal and
vertical buffer graphic traces; buffer picker; BOLD graph, 2D brain, 3D brain;
EPS/PS/DOT export where practical.

Completed:

- Buffer Picker: per-buffer `record-history` / `stop-recording-history`,
  initialised from `used-production-buffers`; BOLD recording accumulates
  buffers instead of replacing them.
- Production Grid: production-history `production-grid` processor rendered as
  a colored decision grid, with unselected-column hiding and hover times.
- Horizontal and vertical buffer graphic traces rendered from the existing
  `horizontal-graphic-trace` / `vertical-graphic-trace` processors.
- BOLD graph (raw / scale-each / scale-all), BOLD 2D module view, and a
  simplified BOLD 3D voxel-grid view with a scan slider.
- Production colors come from persisted settings.

Verified: env tests 14 passed; `cargo test --lib` 86 passed; `environment_rpc`
10 passed; `stepper_rpc` 2 passed; `cargo fmt --check` clean.

Follow-ups (M4 polish):

- The seven layered `production-graph-*` processors are not ported yet; the
  Production Graph tool still uses the existing simplified `production-graph`
  transition graph.
- Production grid lacks the Tcl EPS/PS export, zoom, empty-column hiding on
  selected flags, and per-cell production/why-not popups.
- Graphic traces lack zoom, hide-text/hide-grid, hover readouts, and EPS/PS
  export.
- BOLD 3D is a scan-driven module grid, not the Tcl voxel mesh with rotation.
- Multi-pane history splits remain combined text.

### M5 — Stepper parity (initial)

Start/stop/step/run-until (production/time/module), Step All, Pause, detail
panes, item selection, tutor modes (production instantiation,
conflict-resolution, start-retrieval), reset/clear-all lifecycle.

Completed:

- Stepper tool starts with step-all enabled, sends the client-owned
  `next-event` / `update-event` callbacks, and calls `set-stepper-ready`.
- Step, Stop, Run Until (production/time/module), Step All, and Tutor Mode
  controls.
- Last-stepped / next-step text, possible-item list, and parameter / binding /
  production panes from `update-stepper(item)`.
- Both backends route stepper callbacks: in-process drains the shared
  pause/update state; client mode registers the callback commands and receives
  them through the reader thread.
- Closing the Stepper tool stops the stepper; reset/clear-all lifecycle is
  handled by the session.
- Tests: in-process GUI-state stepper pause/step and client callback routing.

Verified: env tests 16 passed; `cargo test --lib` 86 passed;
`environment_rpc` 10 passed; `stepper_rpc` 2 passed; `cargo fmt --check` clean.

Follow-ups (M5 polish):

- Tutor mode has the setter and tutorable marker but not the full interaction
  UIs (production binding answers, conflict-resolution checklist, start-retrieval
  chunk checklist).
- `update-when-stepped` does not yet re-focus every inspector window; the
  global refresh tick covers it functionally.
- Run-until time/module are wired but only production selection is covered by
  the stepper RPC tests.

### M6 — Experiment window and overlay parity (initial)

Image item, EMMA eye spot, touch fingers, keyboard forwarding, select/open/close
semantics, multiple windows per model (engine/session work), AGI extension
handlers.

Completed:

- Native window ops now cover text, button, line, image, remove, attention,
  cursor, EMMA eye spot (`eyeloc`/`cleareyeloc`), and touch finger markers
  (`showfinger`/`removefinger`).
- Image items render as a labelled placeholder until an image loader is added.
- Clicking a visible window focuses it; printable keys and special keys are
  translated and forwarded through `output-key` (Enter/Escape/arrows/F-keys,
  space, etc.).
- `output-key`/`release-key` are accepted by the session instead of erroring.

Verified: env tests 17 passed; `cargo test --lib` 86 passed; `cargo fmt --check`
clean.

Follow-ups (M6 polish):

- Engine/session do not yet emit image/eyeloc/finger display ops from device
  state; the native renderer is ready but needs the producer side.
- Multiple experiment windows per model are still not supported; the engine
  keeps one window record per model.
- Window titles remain global keys, matching the Tcl limitation.
- The image item is a placeholder; real image loading and click hit-testing are
  not wired.

### M7 — Editor and remaining utilities (initial)

Completed:

- Open File / New File / Save All control-panel actions.
- Floating editor windows with a multiline text pane, dirty marker, and Save.
- Backup files (`file-backup-N`) honor the `save_backups` option.
- Reload saves all dirty editor files first when `save_before_reload` is set.
- The editor-limitations warning is shown when `edit_warn` is set.
- Unit test covers open/edit/save/backup.

Verified: env tests 18 passed; `cargo test --lib` 86 passed; `cargo fmt --check`
clean.

Follow-ups:

- Parenthesis matching/highlighting from the Tcl editor is not implemented.
- Python module import remains out of scope.
- Multi-pane history layouts, layered production-graph processors, full tutor
  UIs, engine-emitted image/eye/finger ops, and multiple experiment windows
  per model remain tracked in their milestone follow-ups.

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
