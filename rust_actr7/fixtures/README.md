# Test fixtures

Everything the integration tests compare against lives here. There are two
independent oracles plus unit tables and error fixtures. The default
`cargo test` is self-contained: it needs neither an `actr7.x` checkout nor
SBCL.

```
fixtures/
  models/          vendored model/task/tutorial sources (manifest.txt, 183);
                   mirrored from $ACTR_DIR by sync-fixtures.py
  devices/         hand-authored <name>.device.sexp sidecars (105)
  regression/      ACT-R's own test-models corpus (the authority)
    specs/         only specs still read at test time: data-heavy experiments
    README.md
  extra-traces/    project-authored scenarios not covered upstream
  units/           module latency / base-level tables (unit tests)
  errors/          err-*.lisp fixtures for the error-path tests
  manifest.lisp    model list for the extra traces (regen input)
  sync-fixtures.py refresh tests/cases/*.ref + models/ from $ACTR_DIR
  regen.sh         regenerate extra-traces/ (needs sbcl)
  regen-specs.sh   regenerate regression/specs/ (needs sbcl)
  run-traces.lisp  extra-trace SBCL driver
  extract-specs.lisp  spec extractor for the regression corpus
  rpc-conformance.{sh,py}, rpc-enabled.txt
  command-inventory.{py,txt}
```

The refs themselves live beside their tests: `tests/cases/<name>.ref`, one
per case (563). `sync-fixtures.py` copies/checks both the refs and
`fixtures/models/` against `$ACTR_DIR` (default `<crate>/../actr7.x`).

## Two oracles

| | `regression/` | `extra-traces/` |
|---|---|---|
| source | ACT-R's `actr7.x/test-models/*.ref` (vendored in `tests/cases/`) | project-authored, no upstream ref |
| how run | one Rust driver per case in `tests/cases/<name>.rs` (a few data-heavy experiments keep `specs/<name>.spec`) | `<name>.meta` (model/seed/run) |
| compared | ACT-R's `compare-test-model-files` (right-trim, case-insensitive) | **byte-exact** (CRLF → LF only) |
| consumed by | `tests/test_models.rs` + `tests/cases/` | `tests/extra_traces.rs` |

Both suites list their cases **statically** — one `#[test]` per case.
Out-of-scope cases are `#[ignore = "reason"]`.

### regression/

ACT-R ships no single test suite; `actr7.x/test-models/` is its regression
corpus of `.lisp` driver stubs and `.ref` reference outputs. Each ref has one
Rust file (`tests/cases/<ident>.rs`) whose `Case` builder owns the model
chain, seed, and ordered steps. A handful of data-heavy experiments (tens of
thousands of runs) keep their normalized `.spec` as the driver instead; all
other cases, including the event-hook cases, own their driver in Rust.
`extract-specs.lisp` + `regen-specs.sh` are only needed when those specs are
regenerated. See `regression/README.md`.

The 189 `SCRAPE` refs are replayed against the vendored sources: five use the
engine's Lisp reader to reproduce the original `(read f)` selection, 173 use
`SourceCase` line presence/order checks, and 11 are small custom transforms.
The four refs that scrape `core-modules/audio.lisp`/`vision.lisp` stay ignored
pending engine-level chunk-type assertions; no core-module Lisp is vendored.

### extra-traces/

Scenarios the upstream corpus does not cover (activation noise, base-level
learning, device/window scaffolding). Ground truth is `<name>.trace`,
captured from the Lisp system by `regen.sh` (+ `run-traces.lisp`,
`manifest.lisp`) and compared byte-exact. The list is **frozen**: it is not
expected to grow. The models are vendored under `fixtures/models/user-loads/`.
See `tests/extra_traces.rs`.

### units/

`units/tables/*.sexp` are module-level latency / base-level tables checked
row-by-row by `src/engine/tests.rs`. `regen-units.sh` + `run-units.lisp`
regenerate them.

## Regenerate

```sh
./sync-fixtures.py         # refresh ../tests/cases/*.ref + models/
./sync-fixtures.py --check
./regen.sh                 # extra-traces/ from manifest.lisp
./regen.sh /tmp/g2         # elsewhere; diff -r proves determinism
./regen-specs.sh           # regression/specs/ (data-heavy experiments)
./units/regen-units.sh     # units/tables/
```

The regen scripts require an `actr7.x` checkout (`ACTR_DIR`, default
`<crate>/../actr7.x`) and SBCL (`sbcl --version` >= 2.4 tested);
`sync-fixtures.py` needs only the checkout. `regen-specs.sh` refreshes only the
committed specs; pass `SPEC_LIST` to import a new set from upstream.

## Why this is trustworthy

- Capture uses ACT-R's own trace sink (`:v` file redirect), not stdout
  scraping.
- Seeds are explicit per model. Without them the RNG is time-seeded
  (`actr7.x/framework/random.lisp`) and runs differ.
- The vendored copies are byte-identical to upstream while `actr7.x` exists;
  `sync-fixtures.py --check` enforces it.
- Lisp-side quirks that affect these fixtures are logged in
  `../../PORTING-NOTES.md`.
