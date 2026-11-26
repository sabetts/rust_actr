# Tk Environment compatibility suite

Local-only harness that drives the **real Tk Environment Tcl client** against a
private Rust `actr server`.  It exists to find Rust/Tcl wire-compatibility bugs
(for example, history streams that never terminate); once a bug is found, add a
Rust regression test under `tests/` or `src/rpc/session.rs` — this suite is
exploratory, not the long-term gate.

## Requirements

- Python 3 with `tkinter`
- an `actr7.x` checkout next to `rust_actr7/` (for the Tcl Environment sources)
- Rust/cargo (unless `--binary` points at an existing `actr` build)

The suite never edits `actr7.x`.  It copies `environment/gui` into
`.tmp/tk-env/...` and sources `new-server.tcl` from that copy.

## Run

From `rust_actr7/`:

```sh
python fixtures/tk-env/run.py --list
python fixtures/tk-env/run.py                          # all scenarios
python fixtures/tk-env/run.py --scenario trace-history
python fixtures/tk-env/run.py --filter history --detail all
python fixtures/tk-env/run.py --addr 127.0.0.1:2650    # existing server
python fixtures/tk-env/run.py --audit                  # Tcl -> server commands
```

Each scenario runs in its own process with a fresh server on a free port
(`--no-port-file`), so port files and user settings are not touched.  Logs and
artifacts are written under `.tmp/tk-env/runs/<timestamp>/`.

`--audit` scans the Tcl dialogs for literal command names passed to
`call_act_r_command*` / `send_cmd` and compares them with the live server's
`list-commands` output; there is no Tk window in this mode.

## Layout

```
run.py            CLI: selection, per-scenario subprocesses, audit
harness.py        server lifecycle, copied env, hidden Tk client, helpers
shims.tcl         messageBox/file-dialog/control-panel/config stubs
scenarios/        one module per area; each exports SCENARIOS
```

Scenarios source the copied `new-server.tcl` plus only the dialog files they
need.  `shims.tcl` supplies a hidden control-panel frame, fonts, and stubs for
modal dialogs so dialog procs can run without the full GUI startup.

## Scenario API

A scenario is a `harness.Scenario` with a name, tags, timeout, and a
`run(ctx)` function.  `ctx` exposes:

- `source_dialog(name)`, `source_gui(name)`
- `call_act_r_command`, `call_with_errors`, `send`
- `eval`, `getvar`, `setvar`, `get_global`, `set_global`
- `text(widget)`, `list_items(widget)`, `entry(widget)`, `invoke(widget)`
- `find_window(prefix)`, `update`, `wait_until`
- `load_model`, `run_model`, `current_model`
- `message_boxes()`, `clear_message_boxes()`

Add a module under `scenarios/` exporting `SCENARIOS` and register it in
`scenarios/__init__.py`.

## Current scenarios

- `trace-history` — Text Trace viewer: empty history, load `addition.lisp`,
  run, low/ALL detail fetch, incremental termination.
- `control-panel` — Load button, creating-model menu update, Run/Reset/Reload/
  Stop via the real control-panel procs.
- `viewers` — Declarative, procedural, buffer, and parameter viewers.
- `histories` — Retrieval and buffer history viewers, selection/detail panes.
- `dispatcher` — Connections, per-connection commands, and documentation.

`--audit` checks Tcl command literals against the live server registry.

More areas (stepper, experiment windows/AGI, production histories/graphs,
BOLD, perceptual histories) are added incrementally.  Prefer semantic
assertions (non-empty, contains a known production/chunk, ordering) over brittle
full-window text.

## Limitations

- No pixel-level rendering checks.
- Only selected dialog procs are sourced; full `server.tcl`/settings init is
  not exercised.
- BOLD and graphical views are shape/smoke candidates only.
- Tk needs a display; this suite is intended for local runs.
