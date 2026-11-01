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
| `rust_actr7/python/` | in-process Python interface (`import rust_actr`) — see `python/README.md` |
| `actr7.x/` | upstream Lisp ACT-R sources (reference/regen; tests use vendored copies) |
| `contrib/` | local unpacked GUI crate sources (untracked, not part of the engine) |
| `PORTING-NOTES.md` | non-obvious ACT-R/Lisp behaviors the port must match |

## Build and test

```sh
cd rust_actr7
cargo test                       # no SBCL or actr7.x checkout needed; regen does
cargo run -- run <model.lisp>
cargo run -- server              # RPC dispatcher
cargo run --features env -- env  # native GUI
```

## Python interface

`rust_actr7/python` provides an in-process PyO3 binding:

```python
import rust_actr as actr

m = actr.Model("count")
m.chunk_type("goal", "state")
start = m.add_chunk("start", "goal", state="start")
m.add_production("""
(p finish
   =goal> state start
   ==>
   =goal> state done)
""")
m.goal_focus(start)
print(m.run(1.0))
```

Build and install it with maturin; see `rust_actr7/python/README.md`.

## Tests

Two static oracles, one `#[test]` per case and one Rust file per ACT-R ref:

- `tests/cases/<name>.rs` — the driver for one `test-models` ref;
  `tests/cases/<name>.ref` is the expected output. `tests/cases/mod.rs` is the
  flat index; `case_files_cover_every_ref` keeps refs, files, and index in
  sync. Out-of-scope cases are `#[ignore = "reason"]`; that ignored set *is*
  the remaining work.
- Source-echo cases verify the vendored tutorial sources: five replay the
  original `(read f)` logic with the engine's Lisp reader, 173 use
  `source_case!` / `SourceCase` line presence, and 11 use small custom
  transforms. Four refs that scrape `core-modules/audio.lisp`/`vision.lisp`
  stay ignored pending engine-level chunk-type assertions.
- `tests/extra_traces.rs` — project-authored scenarios, byte-exact.
- `tests/full_detail.rs` — SBCL-captured maximal-detail traces, byte-exact.
- `tests/draws.rs` — SBCL-captured `act-r-random`/`act-r-noise` sequences.
- `tests/surrogates.rs` — project-authored traces for the blocked
  solution-source cases (categorize, Zbrodoff retrieval, choice).
- `tests/params.rs` — every corpus `sgp` key is classified.
- `tests/errors.rs`, `tests/rpc_conformance.rs` — error paths and the RPC
  surface.

`python tools/list-ignored.py` prints the current ignored-case list (both
`ignored_case!` stubs and direct `#[ignore]` annotations); `--counts` groups by
reason and `--reason TEXT` / `--case TEXT` filter it.

Refs and model/task/tutorial sources are vendored (`tests/cases/*.ref`,
`fixtures/models/`), so the default `cargo test` needs no `actr7.x` checkout.
Fixtures and comparison rules are documented in `rust_actr7/fixtures/README.md`
and `rust_actr7/fixtures/regression/README.md`.

## Conventions

- The `.ref`/`.trace` outputs are the oracle: match them, even where they
  contradict current ACT-R source.
- Promote a case by removing its `#[ignore]`; keep `cargo test` green.
- Log a non-obvious behavior in `PORTING-NOTES.md`, tied to a source line.
