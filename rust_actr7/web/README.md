# ACT-R in the browser (wasm MVP)

A single page that loads the Rust engine compiled to WebAssembly, runs the
model from a textarea, and writes the trace to another textarea.

## Build + run

```sh
web/build.sh                      # cargo build --target wasm32-unknown-unknown, copy the .wasm
cd web && python3 -m http.server 8000
# open http://localhost:8000/
```

`build.sh` needs a Rust toolchain whose sysroot ships the
`wasm32-unknown-unknown` std (the conda-forge `rust` package does — no `rustup`
and no `wasm-pack` needed). The page loads `./actr_engine.wasm`, so serve the
folder over HTTP; opening `index.html` as `file://` fails on the fetch.

## How it works

- `src/model.rs::load_models_from_str` — parse a model from a string (the
  filesystem-free half of `load_models`).
- `src/lib.rs::run_from_source` — build and run the engine from that source,
  returning the trace bytes.
- `src/wasm.rs` — a small hand-written C ABI (`alloc` / `dealloc` / `run_model`
  / `result_ptr` / `result_len`) over `run_from_source`; compiled only for
  `wasm32-unknown-unknown`. Deliberately no `wasm-bindgen`, so there is no extra
  tooling and no dependency added to the native crate.
- `index.html` — the UI (seed + run-time inputs, model textarea, Run button,
  trace textarea) and the JS glue that copies the model into wasm memory.

## Limits (by design, for the MVP)

- One self-contained model file. Models that need a task/setup file or a
  scripted device/window are not supported here yet.
- `run_model` is synchronous, so a pathological model will freeze the tab; the
  fix is to run it in a Web Worker (not done yet).
- The engine is deterministic given the seed, so equal inputs give equal traces.
