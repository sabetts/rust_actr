# ACT-R in the browser

A static browser page that runs the Rust ACT-R engine as WebAssembly. It loads
canonical tutorial model source, runs Unit 1 models directly, and drives Unit 2
tasks through a small browser AGI runtime.

## Build + run

From the repository root:

```sh
python rust_actr7/web/build.py
cd rust_actr7/web
python -m http.server 8000
# open http://localhost:8000/
```

`build.py` stages the model files and manifest under `web/models/`, builds the
wasm, and copies it next to `index.html`:

```text
cargo build --manifest-path rust_actr7/Cargo.toml --release --target wasm32-unknown-unknown
```

It needs a Rust toolchain with the `wasm32-unknown-unknown` target:

```sh
rustup target add wasm32-unknown-unknown
```

No `wasm-pack` is needed. To stage only the model menu:

```sh
python rust_actr7/web/build.py --models-only
```

The page loads `./actr_engine.wasm` and the generated `./models/` files, so
serve the `web/` folder over HTTP. Opening `index.html` as `file://` fails on
the fetches.

## Models

The menu is generated from `web/models/manifest.json`, which `build.py` creates
from:

### Unit 1

- `actr7.x/tutorial/unit1/count.lisp` - Count
- `actr7.x/tutorial/unit1/addition.lisp` - Addition
- `actr7.x/tutorial/unit1/semantic.lisp` - Semantic
- `actr7.x/tutorial/unit1/tutor-model.lisp` - Tutor Model (starter)
- `actr7.x/tutorial/unit1/tutor-model-solution.lisp` - Tutor Model (solution)
- `actr7.x/tutorial/unit1/broken-addition.lisp` - Broken Addition (intentionally broken)

### Unit 2

- `actr7.x/tutorial/unit2/demo2-model.lisp` - Demo2
- `actr7.x/tutorial/unit2/unit2-assignment-model.lisp` - Unit 2 Assignment (starter)

The canonical model sources stay in `actr7.x/tutorial/`; the `web/models/` copies
are generated and ignored by git. Selecting a model replaces the textarea
contents; it does not run automatically.

## Task runtime

The wasm engine exposes the Rust RPC session's AGI command layer and
visible-window events. The browser side mirrors the in-process Python interface
in `rust_actr7/python`:

- `src/web_session.rs` / `src/wasm.rs` - session bridge, JSON commands, fired
  commands, trace lines, and visible-window events.
- `web/js/actr.js` - JS ACT-R facade: commands, monitors, windows, devices,
  model runs, and ACT-R RNG helpers.
- `web/js/agi-window.js` - HTML visible-window renderer for AGI text, buttons,
  lines, images, attention, and cursor events.
- `web/js/tasks/unit2.js` - JS ports of the Unit 2 experiment code.
- `web/js/main.js` - model menu, editor, task panel, and run wiring.

Unit 2's task Lisp is translated to JS; it is not evaluated in wasm. The task
code still calls the same AGI/RPC commands as the Lisp/Python tasks, including
`open-exp-window`, `add-text-to-exp-window`, `install-device`, `run`,
`add-act-r-command`, and `monitor-command`. Letter selection uses the engine's
`permute-list` and `act-r-random`, so the seeded trial behavior matches ACT-R.

Demo2 uses the tutorial's fixed seed and displays the canonical `V`. The Unit 2
Assignment starter is intentionally incomplete and has no solution model here,
so running it faithfully produces no response unless the editor is changed to
add productions.

## Limits

- Task Lisp/Python is translated to JS; arbitrary Lisp task evaluation is not
  supported.
- Human-participant interaction is out of scope; tasks run models only.
- The AGI surface currently covers the item and device features needed by the
  tutorial; more item/device behavior can be added as later units need it.
- Runs are synchronous, so a pathological model can block the tab.
- One self-contained model file at a time. Models that need an external
  task/setup file are not supported yet.
- The Tutor starter model is intentionally incomplete, and Broken Addition is
  intentionally broken.
- The engine is deterministic given the seed, so equal inputs give equal traces.
