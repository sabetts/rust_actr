# ACT-R test-model corpus

`actr7.x/test-models/` is ACT-R's own regression suite: `.lisp` driver stubs
paired with `.ref` reference outputs. This directory integrates the
**trace-reference** subset of those refs as the primary Rust oracle, compared
the way ACT-R's own `compare-test-model-files` does (right-trim,
case-insensitive).

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

The case list itself is static: `tests/cases/test_models_cases.rs` has one
`#[test]` per ref, generated once by `../../tools/gen_test_cases.py` and then
hand-maintained. There is no `enabled.txt`: a case that must pass is a plain
test; an out-of-scope case is `#[ignore = "reason"]`.

## Scope

Only `.ref` files with timestamped trace lines become specs (219 of 563).
The rest test the command layer (`dm`, `sdp`, `spp`, `whynot`, parsed-source
dumps) or warnings, and are not engine traces; they appear as ignored tests.

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
and type-free `isa` matching, chunk-valued references + merge aliasing, and
`!stop!` are handled. The next gaps are `!bind!`/`!eval!` and
`require-extra`; see `../../../GOTCHAS.md` #81–92.

## Regenerate

```sh
../regen-specs.sh              # writes regression/specs/
../regen-specs.sh /tmp/specs   # write elsewhere
```

Requires SBCL. The script finds trace refs, runs those stubs under
instrumentation (capturing model chain, resolved seed, ordered steps, and the
`max-module-name-length` column width), and rewrites the specs. Seeds that do
not match a literal `(sgp :seed V ...)` in the sources are canonicalized to
`(100 0)` so specs are stable across regens.

## Grow coverage

Promote a spec by removing its `#[ignore]` in
`../../tests/cases/test_models_cases.rs` and making it pass. To see what an
out-of-scope case currently does:

```sh
cargo test --test test_models -- --ignored <name> --nocapture
```

Current status and feature gaps are in `../../../GOTCHAS.md`.
