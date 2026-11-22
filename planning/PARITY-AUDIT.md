# Parity Audit (post-corpus)

The regression corpus is complete: **559 passed / 8 ignored**, with 14
project-authored compilation-unit tests and `pcomp` at 66/66. Green means the
printed traces match; it does not mean the engine is in full parity with the
Lisp implementation.

Status: all six audit items are complete. The remaining accepted limitations
are listed under item 5; the default `cargo test` remains self-contained.

This document tracks verification work *beyond* what the per-case refs can
see. The oracle is line-oriented trace text, so anything a ref does not print,
any behavior supplied by a Rust driver instead of the engine, and any
parameter the engine silently accepts are invisible to the default suite.

## Confirmed blind spots

- **Silent `sgp` no-ops.** `apply_sgp` used to end in `_ => {}`. The registry
  in `engine/params.rs` now makes every accepted no-op explicit and rejects
  unknown keys. `:UAN` is implemented (item 2); `:BL-HOOK`/`:NOISE-HOOK` are
  accepted as the names `:UAN` drives. The declarative task hooks
  (`:CHUNK-MERGE-HOOK`/`:RETRIEVAL-SET-HOOK`) remain no-ops; cosmetic flags
  and driver-managed motor parameters carry their reason in the table.
- **Task-provided outputs.** `adaptive-noise` used to print literal result
  rows, so the adaptive base-level/permanent-noise hooks were untested
  (PORTING-NOTES #340). Item 2 now generates those rows from engine state.
  `tutorial-unit1modeling-warnings` is engine-driven (item 5);
  `undefined-slots` and one `act-touch` case still emit literal warning
  strings.
- **Source echoes.** 177 of 567 tests are `source_case!` source checks that
  never run the engine (about 31% of the green count).
- **Task logic outside the engine.** 67 cases replay task RNG with
  `ConsumeRng`, 9 use recovered `.spec` drivers, and 14 use device sidecars.
  This is intentional for external task code, but it means task-RNG and device
  ordering are not what those cases exercise. The tutorial experiment drivers
  embedded in `src/engine` are tracked separately in
  `planning/TUTORIAL-TEST-DRIVERS.md`; they are test oracles and should be
  feature-gated out of production builds.
- **Trace-limited observation.** 86 case files use filtered hooks (148
  explicit `trace(false)` calls), 22 model fixtures set `:v nil`, and only
  about 179 engine case files touch `:ACT` at all. `sdp` prints three
  decimals. RNG draw order, reference lists, utilities, and activations are
  checked only where a ref happens to print them.
- **Known placeholders.** `print-visicon` is an approximate line-per-feature
  form with no oracle (#2982); `new-word-sound`/`new-digit-sound` are no-ops
  (#2984); `GOAL-CHUNK0` handling is an approximation (#2103); chunkhood is
  DM membership (#528).
- **Loudly unimplemented.** `:pas`, `:randomize-time`, `:auto-attend`,
  `:vidt`, and `:optimize-visual nil` are refused rather than silently
  ignored. That is the right failure mode, but the features are still absent.
- **No coverage.** The protected Zbrodoff/choice/categorize solution variants
  and the subitize-results scrape source remain ignored with named reasons;
  `rpc_conformance` is environment-gated.

Not a defect list: the upstream refs are independent oracles compared
line-by-line, 366 case files do drive the engine, 72 unit tests include
SBCL-derived RNG and base-level/latency tables, and the 14 project-authored
compilation-unit tests compare directly against Lisp traces.

## Ordered work

1. [x] **Parameter registry and strict validation.** `engine/params.rs`
   classifies every known `sgp` key as `Both`, `ModelOnly`, `RuntimeOnly`, or
   `Ignored(reason)`. Unknown model-body and runtime keys now fail;
   `tests/params.rs` scans every `(sgp …)`/`(ssp …)` in `fixtures/models` and
   every driver `Sgp` key. Also fixed model-body `:audio-ms-times` loading;
   model-body `:v` stays runtime-only because the harness controls traces.
2. [x] **Engine-generated adaptive-noise.** `:uan` now computes the
   adaptive base-level formulas and suppresses the separate noise draw. The
   driver schedules the task's rehearsals and captures the five
   `(winner C1 C2 latency)` rows after each request; the literal rows are
   gone.
3. [x] **Full-detail differential harness.** `tests/full_detail.rs` compares
   byte-exact SBCL traces captured with `:trace-detail high :act high :pct t`
   for five small models (`fixtures/full-detail/`): noisy-count (activation
   noise), bll-count (base level), mp-test (partial matching), ul-test
   (utility), and attend (vision). The first run found two things:
   - `Engine::new` skipped seeding when the seed was `(100 0)`, treating it
     as the extractor placeholder. The API/driver paths now always apply the
     seed; only `Step::RunFullTime`'s mid-run placeholder is skipped.
   - `:cst` writes to the trace stream in Rust but to command output in Lisp
     (`print-instantiation` → `command-output`), so `:cst t` is excluded from
     these trace comparisons and belongs to item 5.
4. [x] **State/RNG instrumentation.** `Mt` records every public
   `act-r-random`/`act-r-noise` call when enabled. `fixtures/draws/` captures
   SBCL sequences for `noisy-count` with seeds `(100 0)` and `(7 0)`, and
   `tests/draws.rs` compares kind/argument/value/order entry-for-entry. The
   reference history, utility updates, and event ordering are covered by the
   full-detail traces (`bll-count`, `ul-test`, `attend`).
5. [x] **Warning and placeholder oracles.** `tutorial-unit1modeling-warnings`
   now triggers the real paths: `define-chunks` no-slots, a second queued
   retrieval, and runtime `(p ...)` redefinition. `undefined-slots` and one
   `act-touch` case still emit literals; `:cst` writes to command output in
   Lisp but the trace stream in Rust; custom `new-word-sound`/`new-digit-sound`
   remain no-ops. Those are recorded as accepted limitations.
   `print-visicon` is already exercised by several refs (the old #2982 note is
   historical).
6. [x] **RPC conformance.** The gate is explicit
   (`#[ignore = "requires Python 3 and an ACT-R checkout"]`). The gated run
   passes all 20 expected Python test-models here; it stays out of the
   default suite so `cargo test` remains self-contained.

## Blocked solution-source cases

Seven upstream cases remain `#[ignore]` because their model/task source lives
in a password-protected archive. The refs alone are aggregate outputs
(correlation, mean deviation, RT/choice tables), so they do not reveal the
production/event paths. The available task sources and the unit text do:

| blocked case(s) | missing asset | paths exercised | surrogate |
|---|---|---|---|
| `tutorial-unit4-zbrodoff-with-*` | Zbrodoff solution model | task loop, keypress/RT, retrieval-before-counting, `:rt/:bll/:ans/:lf`, `set-all-base-levels` | `tests/surrogates.rs::zbrodoff_retrieval_{hit,miss}` |
| `tutorial-unit8-categorize-*` | categorize solution model | procedural partial matching + `:sim-hook`, dynamic imaginal slots, indirect retrieval, declarative partial matching | `tests/surrogates.rs::categorize_surrogate` |
| `tutorial-unit6-choice-results` | choice task + model | visual choice, keypress, reward-driven utility learning over 12-trial blocks | `tests/surrogates.rs::choice_surrogate` |
| `tutorial-unit3-subitize-results` | `unit-3-subitize-solution.ref` scrape input only | the stub runs an empty model and copies lines; no engine path | none (data-only blocker) |

The surrogates are project-authored models under
`fixtures/models/extras/surrogates/`; their SBCL traces live in
`fixtures/surrogates/traces/` and are compared byte-exact. They do not replace
the upstream refs or match their protected models.

The categorize surrogate exposed and fixed four engine gaps: `if` in the
bounded user-function evaluator, live-buffer chunk lookup for indirect
requests (a stale `named` copy previously won), adding the partial-matching
component once (Lisp's float result), and the missing `trigger-reward` RPC
command used by the choice surrogate.

## Exit criteria

- Unknown parameters fail loudly; every corpus `sgp` key is either handled or
  explicitly classified.
- No result rows are supplied by test code where the engine can compute them.
- At least one full-detail differential trace exists per subsystem
  (declarative, procedural, vision, audio, motor, imaginal, blending,
  utilities, compilation).
- Placeholder features are oracle-backed or listed as known limitations in
  `PORTING-NOTES.md`.
- `PORTING-PLAN.md` and `PORTING-NOTES.md` reflect the new status; ignored
  cases still carry named reasons.
