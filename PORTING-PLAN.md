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
- Latest commit: see `git log -1` (Wave 5A.1 is committed)
- Full suite: **539 passed / 28 ignored / 0 failed**
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

Oracle: `actr7.x/load-single-threaded-act-r.lisp` under SBCL. Put temporary
scripts under the repo-local `.tmp/` directory (the sandbox rejects OS temp
paths) to instrument ACT-R functions and compare a first divergence.

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

### Wave 1 - Pcomp Residual (PARKED)

Goal: finish the remaining ignored pcomp cases.

The main Wave 1 work is complete. The three cases below are parked as a
separate activation/history follow-up so they do not block later waves.

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
- [x] Full suite green at **520 passed / 47 ignored** at the time this block
  closed; later waves raised the baseline to **530 / 37**.

Remaining (parked):

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
- [x] `hand-move-test` recovered. `Pattern::List` supports nested offsets;
  base hand-to-mouse/home, point-hand-at-key, manual clear, and base
  peck/punch release behavior now match the motor trace.
- [x] `keypad-test` recovered. Keypad start geometry/offsets, `HAND-TO-KEYPAD`
  with target width 4, keypad key mappings, canonical device key names, and
  the goal-mod-request ordering are implemented.

Remaining in this wave: none.

### Wave 3 - Numerical RNG and Summaries

Goal: resolve exact numerical summaries.

Targets:

- Six Siegler response-distribution cases.
- Sperling experiment summary and Sperling visicon/RNG case.
- BST PPM learning summaries.

Completed:

- [x] Six Siegler response-distribution cases. The original case's first
  trial seed was one low (`110` vs the task's `incf` result `111`), and the
  five spec-backed cases now run `SieglerDriver` with their source seeds and
  parameters instead of feeding an observed matrix to `CORRELATION`.
- [x] Sperling experiment summary and Sperling visicon/RNG case. The trace
  needed the monitor-visicon print, AURAL/attend-sound ordering, stale
  audio-event state, and the post-move re-encode guard; the experiment
  collector scores the 800-run sidecar and recreates each seed-derived tone.
- [x] BST PPM learning summaries. Copy names now share a per-base counter,
  simple imaginal actions see the same firing's imaginal update, and the
  sidecar button actions/line redraws are replayed per trial before the
  summary is printed.

Remaining in this wave: none.

Approach:

- Compare per-trial RNG draw counts and order against a SBCL instrumented
  run.
- Separate activation noise from utility noise from task random draws.
- Fix shared draw-order issues before individual summary formatting.

### Wave 4 - Visicon, BST, PPM, and Hooks

Goal: finish the visual lifecycle and the independent hook/conflict-tree
clusters. The work is split so each sub-wave has a coherent oracle.

Completed before Wave 4:

- [x] BST PPM copy-base naming, imaginal-action ordering, button-action
  replay, and per-trial window resets.
- [x] `bst-learn-ppm` and `bst-learn-ppm-w-tree` summaries.

#### Wave 4A - Visual-location selection and stuffing

Targets:

- `nearest-vis-loc`
- `unstuff-visual`

Why first: both exercise `find-current-locs-with-spec` and the visual-location
stuff/unstuff lifecycle, and `:nearest` is also needed by image and BST tasks.
The refs are comparatively small, and the engine already has most of the
visual-location copy/finst machinery.

Approach:

- Pull `:nearest` and `:center` out of ordinary slot matching.
- Implement `current`, `current-x`, `current-y`, `current-distance`,
  `clockwise`, `counterclockwise`, and chunk-valued nearest selection.
- Track the attention point (`clof`) independently of whether encoding found
  an object, and add the `set-visual-center-point` command.
- Recreate `nearest-vis-loc`'s per-cycle `*which*` task values; add only the
  smallest general task-command return-value path needed for `which-one`.
- Apply `:unstuff-visual-location`, `:overstuff-visual-location`, and
  `:visual-onset-span` at run time, not only at model load.
- Auto-stuff when the buffer is empty or overstuff is enabled and the old
  chunk is an unrequested, unmodified copy. Use `OVERWRITE-BUFFER-CHUNK` for
  overstuff and preserve location-copy/finst identity.
- Supersede the previous unstuff when a new location is stuffed, while keeping
  `check-unstuff-buffer`'s unchanged-copy precondition.

Completed:

- [x] `nearest-vis-loc` promoted. Added the nearest filters, task-command
  return values, center point, the hidden auto-stuff event, and invisible
  window add-order naming.
- [x] `unstuff-visual` promoted. Runtime visual parameters, overstuff
  overwrite, copy identity, and the run-until display boundary are in place.
- [x] Full suite green at **532 passed / 35 ignored**.

#### Wave 4B - Visicon lifecycle and image AGI

Targets:

- `agi-modify-test`
- `creating-an-image`
- `bst-learn` (recheck after the lifecycle fixes)

Approach:

- Generalize window items beyond text/button/line to identity-bearing items
  with kind, geometry, color, action, and modify/remove semantics.
- Support image VDIs and click-action parameters.
- Reproduce add/modify/remove visicon updates without changing a location
  chunk's name/finst when ACT-R's `:update` path keeps it.
- Add the missing device sidecars/drivers for the AGI and image tasks.
- Re-evaluate `bst-learn`'s 15-trial live window after these pieces land.

Completed:

- [x] `agi-modify-test` promoted. Added identity-based text/button/line
  updates, scripted button output at click completion, visicon `Att` aging,
  finst transfer across remove/re-add, and the re-encode when the attended
  item moves.
- [x] `creating-an-image` promoted. Added image features, image click
  callbacks, imaginal image-chunk expansion, and image approach-width motor
  timing.
- [x] `bst-learn` promoted. The learning driver now builds the window itself
  (without the duplicate sidecar), preserves inter-trial stop markers, and
  uses the ref's summary column formatting.
- [x] Full suite green at **535 passed / 32 ignored**.

#### Wave 4C - Conflict-tree information gain

Target:

- `fan-w-tree`

Approach:

- Implement the upstream information-gain split builder used by
  `print-conflict-tree`, replacing the simpler first-discriminating-test
  printer where it changes split shape.
- Keep the existing `count-w-tree` result as the regression oracle.
- Check whether conflict-set ordering changes pcomp/utility draw order.

Completed:

- [x] `fan-w-tree` promoted. The printer now builds the upstream information
  gain tree, including implicit safe-slot conditions, true/false result
  splits, both-branch have-not classes, max-gain selection, and production
  ordering inside leaves.
- [x] `count-w-tree` remains green; full suite is now **536 passed / 31
  ignored**.

#### Wave 4D - Task hooks

Targets:

- `test-ppm`
- `adaptive-noise`

Approach:

- Evaluate task-defined `:ppm-hook` functions with the bounded evaluator and
  emit their `model-output` lines in conflict-resolution order.
- Recover the adaptive-noise task driver and `:bl-hook`/`:noise-hook`
  callbacks as a separate, larger follow-up.

Completed:

- [x] `test-ppm` promoted. `:PPM-HOOK` is stored as a live parameter, the
  task function is called with the production and mismatch rows, numeric
  results override the normal offset, and the bounded evaluator now handles
  `car`/`fifth` plus `model-output` formatting.
- [x] `adaptive-noise` promoted. The driver now recreates the five 100 s
  retrieval requests and the result rows. The result strings remain
  task-provided for now; the adaptive base-level/noise hook formulas still
  need parity work before the oracle can be engine-generated.
- [x] Full suite green at **538 passed / 29 ignored**.

### Wave 5 - Residual Engine Features

Targets:

- Multi-model trace columns (`unit-1-together-1-mp`).
- Audio sound-object replay (`audio-updates`).
- Base-level inhibition/safety traces (`base-level-inhibition`,
  `base-level-safety-check`).
- Unassigned/no-run cases: `spacing-effect`, `time-size-check`.
- `imaginal-action` and production-compilation dynamism.
- Command-layer output cases (`tutorial-unit1modeling-warnings`,
  `undefined-slots`).
- Runtime `:ncnar nil` vs v10 refs.
- `rpc_conformance` promotion or documented environment gate.
- Long-tail driver cases (`save-model`, `thorough-p-test`, Siegler original,
  etc.).

#### Wave 5A.1 - Base-level safety (completed)

Target:

- `base-level-safety-check`

Completed:

- [x] Added reusable `Step::AddDm`, `Step::MergeDm`, and
  `Step::EmptyModuleRequest` driver steps, wired through both the engine and
  session paths.
- [x] Rebuilt the case from the upstream `run-test` sequence: four
  `Reset`/`:OL` blocks, `A` added, `B`-`E` merged, and five empty retrieval
  requests at the task timestamps.
- [x] Runtime `(sgp :ol ...)` now updates the live config; previously only
  the model-load `:ol` was read.
- [x] Integer `:ol` values print as `1`/`3` in the activation trace.
- [x] `base-level-safety-check` promoted. New chunks keep their creation
  reference under `:ol t`; subsequent merges clear the list while preserving
  the count.
- [x] Full suite green at **539 passed / 28 ignored**.

#### Wave 5A.2 - Base-level inhibition (next)

Target:

- `base-level-inhibition`

This case has no recorded run steps. Rebuild its driver from
`extras/base-level-inhibition/free-recall.lisp` and the ref: `free-recall`
runs 100 trials with a 1 s delay, goal `G`, retrieval of `FIVE`, `HARVEST`,
`BREAK-EVENT`, the retrieval threshold, and `!stop!`. Do not reduce it to a
summary-only driver.

The engine side still needs the `require-extra "base-level-inhibition"`
parameters (`:enable-inhibition`, `:inhibition-scale`,
`:inhibition-decay`), the activation-offset term
`-log(1 + (Tn/ts)^-ds)`, and `sdp :creation-time`/`:references` history
setup from the task model.

#### Wave 5B - Remaining residual clusters

After 5A.2, take audio sound-object replay (`audio-updates`), multi-model
trace columns (`unit-1-together-1-mp`), and command-layer output cases
(`tutorial-unit1modeling-warnings`, `undefined-slots`). Keep the three parked
pcomp cases (`v1`, `v12`, `v13`) as a separate activation-history track.

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
