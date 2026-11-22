# ACT-R in the browser (wasm MVP)

A single page that loads the Rust engine compiled to WebAssembly, runs a
selected Unit 1 model from a textarea, and writes the trace to another textarea.

## Build + run

From the repository root:

```sh
python rust_actr7/web/build.py
cd rust_actr7/web
python -m http.server 8000
# open http://localhost:8000/
```

`build.py` stages the Unit 1 model files and manifest under `web/models/`, then
builds the wasm and copies it next to `index.html`:

```text
cargo build --manifest-path rust_actr7/Cargo.toml --release --target wasm32-unknown-unknown
```

It needs a Rust toolchain with the `wasm32-unknown-unknown` target. Install it
with:

```sh
rustup target add wasm32-unknown-unknown
```

No `wasm-pack` is needed. To stage only the model menu, for example before the
wasm target is installed:

```sh
python rust_actr7/web/build.py --models-only
```

The page loads `./actr_engine.wasm` and the generated `./models/` files, so
serve the `web/` folder over HTTP. Opening `index.html` as `file://` fails on
the fetches.

## Models

The menu is generated from `web/models/manifest.json`, which `build.py` creates
from:

- `actr7.x/tutorial/unit1/count.lisp` — Count
- `actr7.x/tutorial/unit1/addition.lisp` — Addition
- `actr7.x/tutorial/unit1/semantic.lisp` — Semantic
- `actr7.x/tutorial/unit1/tutor-model.lisp` — Tutor Model (starter)
- `actr7.x/tutorial/unit1/tutor-model-solution.lisp` — Tutor Model (solution)
- `actr7.x/tutorial/unit1/broken-addition.lisp` — Broken Addition (intentionally broken)

The canonical model sources stay in `actr7.x/tutorial/unit1/`; the `web/models/`
copies are generated and ignored by git. Selecting a model replaces the
textarea contents; it does not run automatically.

## How it works

- `src/model.rs::load_models_from_str` — parse a model from a string (the
  filesystem-free half of `load_models`).
- `src/lib.rs::run_from_source` — build and run the engine from that source,
  returning the trace bytes.
- `src/wasm.rs` — a small hand-written C ABI (`alloc` / `dealloc` / `run_model`
  / `result_ptr` / `result_len`) over `run_from_source`; compiled only for
  `wasm32-unknown-unknown`. Deliberately no `wasm-bindgen`, so there is no extra
  tooling and no dependency added to the native crate.
- `build.py` — stages the canonical Unit 1 models and manifest, builds the wasm,
  and copies it into `web/`.
- `index.html` — the UI (model menu, seed + run-time inputs, model textarea, Run
  button, trace textarea) and the JS glue that loads models and copies them into
  wasm memory. The seed is ACT-R's `(S C)` pair: `S` initializes the generator,
  `C` discards that many draws before the first real one (0 = fresh start).

## Limits (by design, for the MVP)

- One self-contained model file at a time. Models that need a task/setup file or
  a scripted device/window are not supported here yet.
- The Tutor starter model is intentionally incomplete, and Broken Addition is
  intentionally broken.
- `run_model` is synchronous, so a pathological model will freeze the tab; the
  fix is to run it in a Web Worker (not done yet).
- The engine is deterministic given the seed, so equal inputs give equal traces.
