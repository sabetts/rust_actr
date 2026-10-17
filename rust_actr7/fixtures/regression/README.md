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
The rest have no spec and are ignored; each reason string says which kind:

- `tutorial-source scrape (not an engine trace)` — the stub re-reads a
  tutorial source file and echoes a fragment, so the ref validates the
  tutorial text, not the engine (202 refs; out of scope).
- `event-hook Lisp (not replayable)` — `run-it` installs an arbitrary-Lisp
  `add-pre/post-event-hook` the engine cannot run (37 refs; the documented
  "no arbitrary Lisp" divergence).
- `not an engine trace (command-layer output)` — real command/introspection
  output (`dm`, `sdp`, `spp`, `whynot`, `sgp`, `pp`, warnings) (104 refs;
  in scope as command-layer oracles).

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
`require-extra`; see `../../../PORTING-NOTES.md` #81–92.

## Conventions

- The `.ref` outputs are the oracle: match them, even where they contradict
  current ACT-R source.
- Keep `cargo test` green. A case is promoted by removing its `#[ignore]`
  (see Grow coverage); only then does it have to pass.
- Non-obvious behavior gets a `PORTING-NOTES.md` entry tied to a source line —
  not a changelog.

## Regenerate

```sh
../regen-specs.sh              # writes regression/specs/
../regen-specs.sh /tmp/specs   # write elsewhere
```

Requires SBCL. The script finds trace refs, runs those stubs under
instrumentation (capturing model chain, resolved seed, ordered steps, and the
`max-module-name-length` column width), and rewrites the specs. Seeds that do
not match a literal `(sgp :seed V ...)` in the sources are canonicalized to
`(100 0)` so specs are stable across regens. Regen is additive: hand-authored
`*.device.sexp` sidecars are read, never overwritten.

## Grow coverage

Promote a spec by removing its `#[ignore]` in
`../../tests/cases/test_models_cases.rs` and making it pass. To see what an
out-of-scope case currently does:

```sh
cargo test --test test_models -- --ignored <name> --nocapture
```

Current status and feature gaps are in `../../../PORTING-NOTES.md`.

## Command layer

The goal is a drop-in RPC replacement: a Python ACT-R client connects and
behaves like Lisp ACT-R, the only divergence being that arbitrary Lisp cannot
be executed. Both the client-facing command surface and whatever the refs
exercise are in scope; `../command-inventory.txt` (regenerate with
`../command-inventory.py`) is the work list.

## Out of scope

Refs tagged `tutorial-source scrape` re-read a tutorial source file instead of
exercising the engine, and no client command reaches them, so they stay
ignored. So do the `event-hook Lisp` refs. Everything else that is ignored is
either an in-scope command-layer oracle or a measured spec gap (see Grow
coverage).
