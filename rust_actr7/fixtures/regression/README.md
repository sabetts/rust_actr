# ACT-R test-model corpus

`actr7.x/test-models/` is ACT-R's own regression suite: `.lisp` driver stubs
paired with `.ref` reference outputs. This crate owns a vendored copy so the
default `cargo test` never needs the checkout or SBCL:

| data | vendored at | count |
|---|---|---|
| refs | `tests/cases/*.ref` | 563 |
| model/task/tutorial sources | `fixtures/models/` (see `models/manifest.txt`) | 183 |
| device sidecars | `fixtures/devices/*.device.sexp` | 105 |

`../sync-fixtures.py` copies/verifies the refs and model sources against
`$ACTR_DIR` (default `<repo>/actr7.x`). While the checkout exists, CI can run
`python ../sync-fixtures.py --check`.

## One test per ref

Every ref has one Rust file next to it:

```
tests/
  test_models.rs          # harness (`Case`), spec reader for data-heavy experiments
  cases/
    mod.rs                # flat index: `mod count; mod semantic; ...` (563)
    count.rs
    count.ref
    tutorial_unit1_count_first_goal_chunk.rs
    tutorial-unit1-count-first-goal-chunk.ref
```

A case file starts with canonical metadata:

```rust
//! case: semantic
//! ref:  semantic.ref
//! lisp: actr7.x/test-models/semantic.lisp
```

and owns its driver:

```rust
use crate::Case;
use actr_engine::Step;

#[test]
fn semantic() -> Result<(), String> {
    Case::new("semantic")
        .models(&["tutorial;unit1;semantic.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Run { secs: 10.0, trace: true, width: 22 },
            Step::GoalFocus(crate::chunk_desc(r#"(ISA IS-MEMBER OBJECT CANARY CATEGORY ANIMAL)"#)),
            Step::Run { secs: 10.0, trace: true, width: 22 },
            // ...
        ])
        .check()
}
```

`Case::check` reads the sibling `.ref`, runs the driver against
`fixtures/models/`, and compares the way ACT-R's own
`compare-test-model-files` does: each line is right-trimmed and compared
case-insensitively (`string-equal`), line by line.

`case_files_cover_every_ref` (in `tests/test_models.rs`) enforces the layout:
refs ↔ case-file headers ↔ `mod` lines in `cases/mod.rs`, bijectively. A
forgotten `mod` line fails CI instead of silently skipping a test.

## Driver kinds

| kind | where | notes |
|---|---|---|
| inline driver | `tests/cases/*.rs` | models/seed/steps written in the file |
| spec-backed | `tests/cases/*.rs` with `//! spec:` | data-heavy experiments (tens of thousands of runs) keep their normalized `.spec` |
| hook | `tests/cases/*.rs` via `tests/hooks/` | direct `EventHook` filter; driver is inline |
| source echo | `tests/cases/*.rs` via `source_case!`, the Lisp reader, or a small custom test | checks the vendored source/selector still produces the `.ref` fragment |
| stub | `tests/cases/*.rs` via `ignored_case!` | no engine driver: arbitrary/out-of-scope or a measured gap |

`fixtures/regression/specs/` now holds only the data-heavy experiment specs.
The rest of the old corpus, including every event-hook driver, was translated
into the per-case files.

### Source echo

The 189 runnable `SCRAPE` refs do not exercise the engine: their upstream
`run-it` re-reads a tutorial/model source and echoes a fragment. Five replay
the original `(read f)` logic with the engine's Lisp reader: parse the source,
find the first `(define-model ...)`, select child forms, and render them like
`~(~s~)` (`tutorial_unit1_count_add_dm` keeps the original `add-dm` layout).
The other 173 use `SourceCase` to verify that each nonblank ref line still
appears in the vendored source, in order; eight truncated refs use
`allow_prefix`. Eleven `format`/multi-file special cases are small custom
tests. The four refs that scrape `core-modules/audio.lisp`/`vision.lisp`
stay ignored pending engine-level chunk-type assertions, because core-module
Lisp is not vendored.

## Promote a case

Remove the `#[ignore = "…"]` from its own file and make it pass:

```sh
cargo test --test test_models <name> -- --nocapture
```

To inspect an ignored case:

```sh
cargo test --test test_models -- --ignored <name> --nocapture
```

Stubs panic with their reason when forced, so `--ignored` never reports a
fake pass.

## Regenerate

```sh
../regen-specs.sh          # rewrites specs from actr7.x/test-models/
../sync-fixtures.py        # refresh refs + models from $ACTR_DIR
../sync-fixtures.py --check
```

The regen step needs the checkout and SBCL; the sync steps need only the
checkout. `cargo test` itself is self-contained. `regen-specs.sh` refreshes
only the specs already committed (data-heavy experiments); pass `SPEC_LIST`
to import a different set from upstream. Seeds that do not
match a literal
`(sgp :seed V ...)` in the sources are canonicalized to `(100 0)` so specs are
stable across regens.

## Scope

`extract-specs.lisp` classifies stubs it cannot replay from source. The
per-case files preserve that classification as their `#[ignore]` reason:

| scope | meaning | state |
|---|---|---|
| `PURE` | one run, one model, trace only | runnable if feature-supported |
| `MULTI` | repeated `run`/`goal-focus` | runnable unless it needs an unsupported feature |
| `COMMAND` | command-only dump (`pp`/`dm`/`sdm`/`buffer-chunk`/…) | replayed through the command layer |
| `DEVICE` | window/device scaffolding not in the driver | later |
| `HOOK` | `add-pre/post-event-hook` arbitrary Lisp | projection, `pcomp` control, and trace-suppress/window filters are direct Rust `EventHook`s with inline drivers (`tests/hooks/`) |
| `SCRAPE` | re-reads a tutorial source and echoes a fragment | 189 run via `SourceCase`; 4 core-module refs pending engine-level chunk-type assertions |
| `ARBITRARY` | output computed in arbitrary Lisp (`format`, experiments) | later |
| `DRIVER` / `ERROR` / `OTHER` | unsupported `run-it` op / extraction failed / no run | ignored |

The remaining `#[ignore]` reason strings say which kind a case is and why it is
not yet runnable. `PURE`/`MULTI`/`COMMAND` ignored cases are measured gaps
awaiting promotion; the four remaining `SCRAPE` cases need engine-level
chunk-type assertions.

The scripted engine supports multiple `run` steps (each bounded relative to
the current model time), `goal-focus`, mid-driver `:v`/`:trace-detail`,
`:seed`, and `(reset)` (clock to 0, DM/buffers rebuilt, RNG preserved).
`define-chunks` named chunks, model-body
`declare-buffer-usage`/`spp`/`set-base-levels`/`set-similarities`,
`?buffer> buffer empty|full`, `+goal>` requests, `-` request slots, spreading
(`:mas`), signature-based candidacy, request-spec `=buffer` chunk copies,
`:act`/`:crt` traces, unnamed `add-dm` auto-naming, LHS variable slot names,
type-free `isa` matching, chunk-valued references + merge aliasing, and
`!stop!` are handled. The next gaps are `!bind!`/`!eval!` and
`require-extra`; see `../../../PORTING-NOTES.md` #81–92.

## Conventions

- The `.ref` outputs are the oracle: match them, even where they contradict
  current ACT-R source.
- Keep `cargo test` green. A case is promoted by removing its `#[ignore]`.
- Non-obvious behavior gets a `PORTING-NOTES.md` entry tied to a source line —
  not a changelog.

## Command layer

The goal is a drop-in RPC replacement: a Python ACT-R client connects and
behaves like Lisp ACT-R, the only divergence being that arbitrary Lisp cannot
be executed. Both the client-facing command surface and whatever the refs
exercise are in scope; `../command-inventory.txt` (regenerate with
`../command-inventory.py`) is the work list.

A `COMMAND`-scope case is a stub whose `run-it` only issues commands (a pure
`pp`/`dm`/`sdm`/`buffer-chunk`/`sdp` dump). Its per-case driver replays them
through `Session::evaluate` (the same dispatch a Python client hits), and the
printing commands append their output to the engine buffer, which the harness
compares to the `.ref`.

## Out of scope

The 189 source-echo refs run through `SourceCase`; the four that read
`core-modules/audio.lisp`/`vision.lisp` stay ignored until they are replaced by
engine-level chunk-type assertions. The `ARBITRARY` (experiment/`format`) refs
stay ignored too. The converted `HOOK` cases run directly through
`tests/hooks/`; the 16 that remain ignored there expose measured engine gaps.
The 67 not-yet-converted `:HOOK` refs carry a reason naming their next blocker:
driver recovery, hook-side system-parameter/production effects, buffer
effects/output, command effects, or hook-side scheduling. Everything else that
is ignored is either an in-scope command-layer oracle or a measured
`PURE`/`MULTI`/`COMMAND` gap.
