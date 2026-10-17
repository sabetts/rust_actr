# Test fixtures

Everything the integration tests compare against lives here. There are two
independent oracles plus unit tables and error fixtures.

```
fixtures/
  regression/      ACT-R's own test-models corpus (the authority)
    specs/         machine-readable driver specs (generated; see below)
    README.md
  extra-traces/    project-authored scenarios not covered upstream
  units/           module latency / base-level tables (unit tests)
  errors/          err-*.lisp fixtures for the error-path tests
  manifest.lisp    model list for the extra traces
  regen.sh         regenerate extra-traces/ (needs sbcl)
  regen-specs.sh   regenerate regression/specs/ (needs sbcl)
  run-golden.lisp  extra-trace SBCL driver
  extract-specs.lisp  spec extractor for the regression corpus
  rpc-conformance.{sh,py}, rpc-enabled.txt
  command-inventory.{py,txt}
```

## Two oracles

| | `regression/` | `extra-traces/` |
|---|---|---|
| source | ACT-R's `actr7.x/test-models/*.ref` | project-authored, no upstream ref |
| how run | `specs/<name>.spec` (extracted from the Lisp stubs) | `<name>.meta` (model/seed/run) |
| compared | ACT-R's `compare-test-model-files` (right-trim, case-insensitive) | **byte-exact** (CRLF → LF only) |
| consumed by | `tests/test_models.rs` | `tests/extra_traces.rs` |

Both suites list their cases **statically** — one `#[test]` per case.
Out-of-scope cases are `#[ignore = "reason"]`.

### regression/

ACT-R ships no single test suite; `actr7.x/test-models/` is its regression
corpus of `.lisp` driver stubs and `.ref` reference outputs. The `.ref` is
the ground truth but does not say *how* to drive the engine, so
`extract-specs.lisp` records each stub's model chain, seed and ordered steps
into `specs/<name>.spec` (run by `regen-specs.sh`). `cargo test` never needs
SBCL — only regeneration does. See `regression/README.md`.

### extra-traces/

Scenarios the upstream corpus does not cover (activation noise, base-level
learning, device/window scaffolding). Ground truth is `<name>.trace`,
captured from the Lisp system by `regen.sh` (+ `run-golden.lisp`,
`manifest.lisp`) and compared byte-exact. The list is **frozen**: it is not
expected to grow. See `tests/extra_traces.rs`.

### units/

`units/tables/*.sexp` are module-level latency / base-level tables checked
row-by-row by `src/engine/tests.rs`. `regen-units.sh` + `run-units.lisp`
regenerate them.

## Regenerate

```sh
./regen.sh                 # extra-traces/ from manifest.lisp
./regen.sh /tmp/g2         # elsewhere; diff -r proves determinism
./regen-specs.sh           # regression/specs/ from actr7.x/test-models/
./units/regen-units.sh     # units/tables/
```

All three require SBCL (`sbcl --version` >= 2.4 tested) and an `actr7.x`
checkout (`ACTR_DIR`, default `<crate>/../actr7.x`).

## Why this is trustworthy

- Capture uses ACT-R's own trace sink (`:v` file redirect), not stdout
  scraping.
- Seeds are explicit per model. Without them the RNG is time-seeded
  (`actr7.x/framework/random.lisp`) and runs differ.
- Lisp-side quirks that affect these fixtures are logged in
  `../../PORTING-NOTES.md`.
