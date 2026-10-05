# ACT-R test-model corpus (Phases 1–3)

`actr7.x/test-models/` is ACT-R's own regression suite: 554 `.lisp` driver
stubs paired with 563 `.ref` reference outputs. This directory integrates the
**trace-reference** subset of those refs as a second Rust oracle alongside the
hand-built `golden/traces/` corpus.

`cargo test` never needs SBCL: the Lisp side runs only during regen.

## Files

- `specs/<name>.spec` — committed runner spec per test, produced by
  `../regen-specs.sh` (`../extract-specs.lisp`). One S-expression:
  ```
  (:NAME "semantic" :REF "semantic.ref"
   :MODELS ("tutorial;unit1;semantic.lisp")
   :SEED-SET NIL
   :RUNS ((10 (100 0) T 22) ...) ; (secs (seed advance) trace-on module-width)
   :STEPS ((:RUN 10 (100 0) T 22)
           (:GOAL-FOCUS (ISA IS-MEMBER OBJECT CANARY CATEGORY ANIMAL))
           (:RUN 10 (100 0) T 22) ...)   ; ordered driver ops
   :SCOPE :MULTI :FLAGS NIL :ERROR NIL)
  ```
  `:STEPS` is the ordered driver (`:RUN`, `:GOAL-FOCUS`, `:RESET`, `:SGP`) and
  is what `tests/test_models.rs` executes; `:RUNS` is a convenience view.
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
| `MULTI` | repeated `run`/`goal-focus` | yes, unless it also needs `(reset)` or an unsupported model feature |
| `DEVICE` | window/device scaffolding not in the spec | later |
| `COMMAND` | ref contains command-layer output | later |
| `DRIVER` | unsupported `run-it` op (`pdisable`, `spp`, …) | out of scope |
| `ERROR` | stub needed the harness (`*out-file*`, helpers) | out of scope |

The scripted engine supports multiple `run` steps (each bounded relative to
the current model time), `goal-focus`, mid-driver `:v`/`:trace-detail`,
`:seed`, and `(reset)` (clock to 0, DM/buffers rebuilt, RNG preserved).
`define-chunks` named chunks, model-body
`declare-buffer-usage`/`spp`/`set-base-levels`/`set-similarities`,
`?buffer> buffer empty|full`, `+goal>` requests, `-` request slots, spreading
(`:mas`), signature-based candidacy, request-spec `=buffer` chunk copies,
`:act`/`:crt` traces, unnamed `add-dm` auto-naming, LHS variable slot names,
and type-free `isa` matching are handled. The next gaps are
`!bind!`/`!eval!` and `require-extra`; see `../../GOTCHAS.md` #81–88.

## Regenerate

```sh
./regen-specs.sh              # writes test-models/specs/
./regen-specs.sh /tmp/specs   # write elsewhere
```

Requires SBCL. The script finds trace refs, runs those stubs under
instrumentation (capturing model chain, resolved seed, ordered steps, and the
`max-module-name-length` column width), and rewrites the specs. Seeds that do
not match a literal `(sgp :seed V ...)` in the sources are canonicalized to
`(100 0)` so specs are stable across regens.

## Grow coverage

```sh
cargo test --test test_models probe_runnable -- --ignored --nocapture
```

reports every runnable (`PURE`/`MULTI`) spec that already matches its ref.
Promote passing names into `enabled.txt`. Current status and feature gaps are
in `../../GOTCHAS.md` under "M10 findings".
