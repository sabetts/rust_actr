# ACT-R test-model corpus (Phase 1/2)

`actr7.x/test-models/` is ACT-R's own regression suite: 554 `.lisp` driver
stubs paired with 563 `.ref` reference outputs. This directory integrates the
**trace-reference** subset of those refs as a second Rust oracle alongside the
hand-built `golden/traces/` corpus.

`cargo test` never needs SBCL: the Lisp side runs only during regen.

## Files

- `specs/<name>.spec` — committed runner spec per test, produced by
  `../regen-specs.sh` (`../extract-specs.lisp`). One S-expression:
  ```
  (:NAME "count" :REF "count.ref"
   :MODELS ("tutorial;unit1;count.lisp")
   :SEED-SET NIL
   :RUNS ((10 (100 0) T 22))     ; (secs (seed advance) trace-on module-width)
   :GOAL-FOCUS 0 :RESETS 0 :SGP NIL
   :SCOPE :PURE :FLAGS NIL :ERROR NIL)
  ```
- `enabled.txt` — checked-in allowlist of specs that must pass under
  `cargo test` (`tests/test_models.rs::enabled_test_models_match`). Everything
  else is out of scope for now; the gap is visible, not silently skipped.

## Scope

Only `.ref` files with timestamped trace lines become specs (219 of 563).
The rest test the command layer (`dm`, `sdp`, `spp`, `whynot`, parsed-source
dumps) or warnings, and are not engine traces.

`extract-specs.lisp` classifies each trace test without executing the ones that
only make sense in the command/device harness:

| scope | meaning | runnable now |
|---|---|---|
| `PURE` | one run, one model, trace only | yes (if feature-supported) |
| `MULTI` | repeated `run` with `goal-focus`/`reset` | needs scripted engine (Phase 3) |
| `DEVICE` | window/device scaffolding not in the spec | later |
| `COMMAND` | ref contains command-layer output | later |
| `DRIVER` | unsupported `run-it` op (`pdisable`, `spp`, …) | out of scope |
| `ERROR` | stub needed the harness (`*out-file*`, helpers) | out of scope |

## Regenerate

```sh
./regen-specs.sh              # writes test-models/specs/
./regen-specs.sh /tmp/specs   # write elsewhere
```

Requires SBCL. The script finds trace refs, runs those stubs under
instrumentation (capturing model chain, resolved seed, run steps, and the
`max-module-name-length` column width), and rewrites the specs. Seeds that were
never set explicitly are canonicalized to `(100 0)` so specs are stable.

## Grow coverage

```sh
cargo test --test test_models probe_all_pure -- --ignored --nocapture
```

reports every `PURE` spec that already matches its ref. Promote passing names
into `enabled.txt`. Current status and feature gaps are in `../../GOTCHAS.md`
under "M10 findings".
