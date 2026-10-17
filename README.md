# actr7.x_rust

A Rust port of the ACT-R 7 single-threaded kernel. It reproduces ACT-R's
execution traces and speaks ACT-R's RPC protocol, so an existing ACT-R client
(e.g. the Python `actr.py`) can connect unchanged — the one divergence being
that arbitrary Lisp cannot be executed.

## Layout

| path | what |
|---|---|
| `rust_actr7/` | the crate, `actr-engine` (`actr_engine` in code) |
| `rust_actr7/src/` | engine (`engine/`), s-expression reader, model loader, RNG, RPC server, wasm ABI, native GUI (`env/`, feature `env`) |
| `rust_actr7/src/bin/actr.rs` | CLI: `actr run <model>`, `actr server`, `actr env` |
| `rust_actr7/tests/` | integration suites (below) |
| `rust_actr7/fixtures/` | test oracles + regen scripts — see `fixtures/README.md` |
| `rust_actr7/web/` | browser (wasm) build — see `web/README.md` |
| `actr7.x/` | upstream Lisp ACT-R sources (the oracle) |
| `contrib/` | local unpacked GUI crate sources (untracked, not part of the engine) |
| `PORTING-NOTES.md` | non-obvious ACT-R/Lisp behaviors the port must match |

## Build and test

```sh
cd rust_actr7
cargo test                       # no SBCL needed; regen does
cargo run -- run <model.lisp>
cargo run -- server              # RPC dispatcher
cargo run --features env -- env  # native GUI
```

## Tests

Two static oracles, one `#[test]` per case:

- `tests/test_models.rs` — ACT-R's `test-models` corpus vs its `.ref` outputs.
  Out-of-scope cases are `#[ignore = "reason"]`; that ignored set *is* the
  remaining work.
- `tests/extra_traces.rs` — project-authored scenarios, byte-exact.
- `tests/errors.rs`, `tests/rpc_conformance.rs` — error paths and the RPC
  surface.

Fixtures and the comparison rules are documented in
`rust_actr7/fixtures/README.md` and `rust_actr7/fixtures/regression/README.md`.

## Conventions

- The `.ref`/`.trace` outputs are the oracle: match them, even where they
  contradict current ACT-R source.
- Promote a case by removing its `#[ignore]`; keep `cargo test` green.
- Log a non-obvious behavior in `PORTING-NOTES.md`, tied to a source line.
