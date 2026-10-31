# Parity Audit (post-corpus)

The regression corpus is complete: **559 passed / 8 ignored**, with 14
project-authored compilation-unit tests and `pcomp` at 66/66. Green means the
printed traces match; it does not mean the engine is in full parity with the
Lisp implementation.

This document tracks verification work *beyond* what the per-case refs can
see. The oracle is line-oriented trace text, so anything a ref does not print,
any behavior supplied by a Rust driver instead of the engine, and any
parameter the engine silently accepts are invisible to the default suite.

## Confirmed blind spots

- **Silent `sgp` no-ops.** `apply_sgp` used to end in `_ => {}`. The registry
  in `engine/params.rs` now makes every accepted no-op explicit and rejects
  unknown keys. Remaining real gaps are the adaptive-noise hooks
  (`:UAN`/`:BL-HOOK`/`:NOISE-HOOK`, item 2) and the declarative task hooks
  (`:CHUNK-MERGE-HOOK`/`:RETRIEVAL-SET-HOOK`); cosmetic flags and
  driver-managed motor parameters carry their reason in the table.
- **Task-provided outputs.** `adaptive-noise` prints literal result rows, so
  the adaptive base-level/permanent-noise hooks are untested (PORTING-NOTES
  #340). `tutorial-unit1modeling-warnings`, `undefined-slots`, and one
  `act-touch` case emit literal warning strings instead of triggering the
  engine's warning path.
- **Source echoes.** 177 of 567 tests are `source_case!` source checks that
  never run the engine (about 31% of the green count).
- **Task logic outside the engine.** 67 cases replay task RNG with
  `ConsumeRng`, 9 use recovered `.spec` drivers, and 14 use device sidecars.
  This is intentional for external task code, but it means task-RNG and device
  ordering are not what those cases exercise.
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
2. **Engine-generated adaptive-noise.** Port `:uan`, `:bl-hook`, and
   `:noise-hook`; derive the result rows from engine state and remove the
   literals.
3. **Full-detail differential harness.** For one representative model per
   subsystem, run SBCL and Rust with maximal trace
   (`:v t :act high :pct t :sact t :cst t :blt t`) and store the SBCL output
   as a project-authored trace. This is the main way to expose numeric and
   event-ordering drift the refs suppress.
4. **State/RNG instrumentation.** For a small sample, compare RNG draw
   sequences, reference merges, utility updates, and queue ordering between
   instrumented SBCL and Rust. Trace text alone can hide draws that never
   affect a printed line.
5. **Warning and placeholder oracles.** Make warning cases trigger the real
   conditions; add SBCL-generated refs for `print-visicon` and custom
   sounds, or record them as accepted limitations.
6. **RPC conformance.** Promote `rpc_conformance` or keep the documented
   environment gate explicit.

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
