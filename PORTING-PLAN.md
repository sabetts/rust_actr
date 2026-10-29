# ACT-R Rust Porting Plan

## Mission

Make the vendored ACT-R regression corpus pass from the Rust engine without
weakening the oracle:

- Never edit a `.ref`.
- Do not branch on a test/case name.
- Prefer an engine-backed fix. A task-driver port is acceptable when the
  behavior is genuinely task-side.
- Keep `cargo test` green after every wave.

## Current Baseline

- Repo: `C:\Users\sabetts\src4\actr7.x_rust`
- Crate: `rust_actr7`
- Latest commit: see `git log -1` (the pcomp/residual wave is committed)
- Full suite: **518 passed / 49 ignored / 0 failed**
- Separate environment-gated test: `rpc_conformance` (1 ignored; needs
  Python, bash, and a local ACT-R checkout).

Useful commands:

```powershell
cargo fmt --all
cargo test
cargo test --test test_models <test_name> -- --ignored --nocapture
python rust_actr7/tools/list-ignored.py --counts
python rust_actr7/tools/list-ignored.py --case pcomp --files
```

Oracle: `actr7.x/load-single-threaded-act-r.lisp` under SBCL. Use temporary
scripts outside the repo (for example under the OS temp directory) to
instrument ACT-R functions and compare a first divergence.

## Working Rules

1. Start a case by reading its `.rs`, `.ref`, upstream `.lisp`, and relevant
   `PORTING-NOTES.md` entries.
2. Find the first true divergence, not the last visible one.
3. For trace mismatches, compare normalized lines; for RNG/timing, compare
   draw counts/times as well as selected productions.
4. Add or update the smallest engine API needed; avoid test-specific logic.
5. Promote by removing `#[ignore]` only after the case passes in isolation.
6. Run the full suite, update `PORTING-NOTES.md`, and commit each coherent
   wave.
7. Re-run non-ignored tests that share changed code paths; fixture/device
   changes are data, not tests.

## Wave Status

### Wave 0 - Completed

Recovered deterministic engine/API cases:

- Event-hook effects, hook commands, retrieval-history traces.
- Module modification requests and hidden hook-line capture.
- Imaginal-action event ordering.
- Count/imaginal mod-request cases.
- BST imaginal-action trace.
- Zbrodoff single-trial setup.
- Pcomp task RNG at trial boundaries.
- Pcomp safe-eval, modifier invalidation, compiled request defaults,
  negative request slots, and utility updates while trace is suppressed.

### Wave 1 - Pcomp Residual (IN PROGRESS)

Goal: finish the remaining ignored pcomp cases.

Completed:

- [x] `trial-chunk` - one-trial device fixture with feedback.
- [x] `v10-one-game-result` - shared pcomp game driver.
- [x] `initial-run` - 50-game seeded summary.
- [x] `v3-initial-run` - 50-game seeded summary.
- [x] `v5-initial-run` - 10-game seeded summary.
- [x] `v7-find-seed` - task-command flag and preserved RNG across games.
- [x] Earlier pass: `v10-attend-num-1/2-productions`,
  `v10-one-game-late-productions`, `v10-production21`,
  `v10-production29`, `v4-response-chunks`.
- [x] Driver/API work: `Step::PcompGame`, block-wise score/RT summaries,
  fixed-seed and continue-stream modes, RNG `(seed, draws)` state,
  `reset_keep_rng`, registered-command capture, model-body `(when ...)`
  seeding, and `(sgp :seed)` getters.
- [x] Vendored pcomp models v11-v13.
- [x] `v10-one-game-result-unused` - compiled-production canonicalization now
  matches ACT-R's 111-production set.
- [x] `v11-test-runs` - negative retrieval-request variables stay symbolic;
  the composed `guess-other & respond` production matches ACT-R.
- [x] Full suite green at **518 passed / 49 ignored**.

Remaining:

- [ ] **`v1-initial-run`** (1): rechecked after the compile fixes; still
  diverges from the first block with `:epl t` (`1.00 2.32 2.10 ...` vs
  `0.86 1.68 2.98 ...`).
- [ ] **`v12-50-runs`** (1): games 1-39 match. Game 40, trial 10 is the first
  real divergence: `retrieve-past-trial` returns the same
  `imaginal-chunk0-6`, but Rust latches it at 80.419 vs oracle 81.913. This
  is a DM activation/history-latency issue, not production compilation.
- [ ] **`v13-50-runs`** (1): rechecked after the compile fixes; still diverges
  (`2.56 5.44 6.38 ...` vs `3.04 4.38 5.58 ...`). Shares the v12 family.

Recommended next steps:

1. Compare the base-level activation history for `imaginal-chunk0-6` in game
   40 trial 10 and explain the 1.494 s latency gap.
2. Re-run v13 after that activation fix.
3. Isolate v1's EPL summary separately.

Pcomp-specific gotchas:

- The task draws two `act-r-random 4` values at every trial start. The first
  pair is covered by `Step::ConsumeRng(2)`; later pairs belong in the device
  sidecar as `(RNG-DRAW ms 2 4)` before the two trial `ADD-TEXT` ops.
- `(RNG-DRAW ...)` is consumed inside `proc_display`, before texts at the
  same millisecond.
- `!safe-eval!` is valid for compilation. First-parent variables stay
  symbolic in composed LHS conditions; second-parent variables are
  instantiated from the firing.
- Recreated compiled productions update `:u` even when `:v`/`:pct` output is
  suppressed.

### Wave 2 - Paired, Motor, and Timing

Goal: recover the paired and motor families.

Targets include:

- Six paired system-parameter/compiling/dynamic cases.
- Six paired-summary timing cases.
- `hand-move-test`, `keypad-test`.
- Any paired case still blocked by pcomp state after Wave 1.

Completed:

- [x] `tutorial-unit7-paired-learning-alpha-1` recovered with the same
  `permute-list`/`ConsumeRng` correction as the activation-trace case. Its
  real filter hook now breaks at `PRODUCTION7` on the expected state.
- [x] `tutorial-unit7-paired-learning-alpha-1-final` recovered. The generic
  mapping pass was applying `MAP-GOAL-BUFFER`'s variable-to-constant arm to
  retrieval slots, turning `ARG2 =VAL` into `ARG2 "9"` while leaving
  `KEY =VAL` on the RHS. Restricting that arm to `GOAL` fixes the composition
  without disturbing v11.
- [x] `tutorial-unit4-paired-act-trace` recovered. The test's
  `permute-list` command was permuting two strings instead of one pair, and
  the extra `ConsumeRng(1)` steps double-counted the task draw. Correcting
  both puts the activation-noise draw at ACT-R's `(200 10)` state.
- [x] `tutorial-unit7-paired-learning-results-of-compiling` recovered with
  its real filter hook and the corrected task RNG draws.
- [x] `tutorial-unit8-paired-dynamic-compiling-to-dynamic` and
  `...-compiling-to-static` recovered with their filter hooks
  (breaks at 400/600).
- [x] Six paired event-hook/compiling cases recovered in total; the shared
  fixes were the one-pair `permute-list` form, the real `filter`/`filter2`
  lifecycle, and confining the goal map's variable arm.
- [x] Five paired-summary cases recovered:
  `tutorial-unit4-paired-experiment`, `...-no-bll`, `...-with-rt`, and
  `tutorial-unit7-paired-learning-results-no-compiling`/`...-with-compiling`.
  The 4-5 ms offset was six adjacent same-finger key transitions; Rust now
  uses ACT-R's `distance=`/`direction=` peck tolerances (`motor.lisp:890`).
  The collector also reads `IMAGINAL.ARG1`/`WORD` and keeps only the probe half
  of each 10 s probe/feedback cycle.
- [x] `tutorial-unit8-paired-dynamic-results-with-compiling` recovered. The
  missing compiled candidate was tied to p1's dynamic modify-request slot
  (`=which-slot`): resolving it against p1's firing lets composition drop the
  matching p2 imaginal condition, and instantiating p2's dynamic LHS slot name
  in the imaginal mapping supplies the `STIMULUS -> VAL` mapping.
- [x] `tutorial-unit7-paired-composed-production` recovered with a real
  `paired-task 1 2` driver, a four-prompt device sidecar, and `pp production0`.
  Composed query conditions now follow ACT-R's `buffer-index` order.

Remaining in this wave:

- [ ] `hand-move-test`: model load blocks on nested
  `offsets ((index -1 0))`, which needs a non-atomic slot-pattern
  representation.
- [ ] `keypad-test`: blocks on the unimplemented `HAND-TO-KEYPAD` manual
  request.

Approach:

- Compare per-trial `(start, key)` rows against a trained oracle subject; the
  aggregate summary hides which trials diverge.
- Address `hand-move-test`'s slot-value representation and `keypad-test`'s
  manual request.

### Wave 3 - Numerical RNG and Summaries

Goal: resolve exact numerical summaries.

Targets:

- Six Siegler response-distribution cases.
- Sperling experiment summary and Sperling visicon/RNG case.
- BST PPM learning summaries.

Approach:

- Compare per-trial RNG draw counts and order against a SBCL instrumented
  run.
- Separate activation noise from utility noise from task random draws.
- Fix shared draw-order issues before individual summary formatting.

### Wave 4 - Visicon, BST, PPM, and Hooks

Goal: finish visual lifecycle and hook-feature clusters.

Targets:

- `nearest-vis-loc`, `unstuff-visual`, `agi-modify-test`,
  `creating-an-image`, `bst-learn`.
- BST PPM copies/length, `test-ppm`, `adaptive-noise`.
- Revisit `fan-w-tree` with the full info-gain conflict-tree builder.

Approach:

- Implement the upstream conflict-tree info-gain builder if conflict-set
  order is still needed; it may also affect pcomp/utility draw order.
- Fix auto-stuff/location numbering/removal and visicon update lifecycle as
  one cluster.

### Wave 5 - Residual Engine Features

Targets:

- Multi-model trace columns (`unit-1-together-1-mp`).
- Audio sound-object replay (`audio-updates`).
- Base-level inhibition/safety traces.
- Runtime `:ncnar nil` vs v10 refs.
- `rpc_conformance` promotion or documented environment gate.
- Long-tail driver cases (`save-model`, `thorough-p-test`, Siegler original,
  etc.).

### External Blockers

These need the protected source/password or an upstream copy:

- Three protected Zbrodoff solution variants.
- `tutorial-unit6-choice-results`.
- Two protected categorize experiments.

Also determine whether `tutorial-unit3-subitize-results` should be treated as
source-missing or recoverable from the available case ref.

## First-Divergence Harness

Use this loop for any cluster:

1. Run the ignored case:
   `cargo test --test test_models <name> -- --ignored --nocapture`
2. Read the first `L n exp=... got=...` line. Ignore trailing differences
   until that line is understood.
3. If the ref is a filtered hook projection, replay the upstream `run-it`
   under SBCL and instrument the relevant ACT-R function
   (`act-r-noise`, `linear-update-utility`, `format-event`, etc.).
4. If timing/RNG is involved, log:
   - model time
   - selected/fired production
   - utility draw count and value
   - task random draw count
5. Fix the shared cause, then re-run the representative case and the full
   suite.

Recent examples:

- Pcomp utility mismatch was not a utility formula bug: task trial RNG draws
  were missing between trials.
- Pcomp `:u` mismatch was a trace-gated side effect: recreation utility was
  updated only when `:pct` output was visible.

## Definition of Done

- `cargo test` passes with zero failures.
- All recoverable ignored cases are promoted.
- Only externally blocked cases remain ignored, with a named reason.
- `PORTING-NOTES.md` records each root cause and the new corpus count.
- Working tree is clean after the final commit.
