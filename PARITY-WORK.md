# Differential parity handoff

## Context

This repo contains a Rust port of ACT-R 7 plus the vendored `actr7.x` Lisp
tree. `actr7.x` is the oracle: do not edit it. Runtime commands may write
`actr7.x/environment/gui/init/05-current-net.tcl`; the differential harness
restores it automatically.

Working tree state:

- `actr7.x/support/load-mode.lisp` and `actr7.x/support/require-mode.lisp`
  were already modified before this work. Leave them alone.
- `actr7.x/environment/gui/init/05-current-net.tcl` is runtime-generated and
  untracked.
- `server_testing/` is an older untracked exploratory harness. The canonical
  harness now lives under `rust_actr7/fixtures/differential/`.

Commit at logical points. Do not commit `server_testing/` or `actr7.x`
changes.

## Current status

Rust changes are committed through `fbfb58c`. Priority 0 and Priority 1 are
complete. All three Priority 2 probes are implemented and currently `DIFF-OK`,
but Priority 2 is **reopened** for the residual gaps recorded below. Current
verified state: `cargo fmt --check` clean; `cargo test` passed (`113`
library/unit tests plus `560` model cases, `0` failed, `8` ignored model
cases); all 16 differential cases pass with `--isolate`.

Full Rust test suite:

```powershell
cd rust_actr7
cargo fmt --check
cargo test
```

Last result: `113` library/unit tests plus `560` model cases passed, `0`
failed, `8` ignored model cases.

Differential harness (requires SBCL, Python 3, and a built Rust binary):

```powershell
cargo build --bin actr
python rust_actr7/fixtures/differential/run_cases.py --server both --isolate
cargo test --test differential -- --ignored --nocapture
```

`--isolate` starts a fresh server per case; use it when adding probes so state
from one model cannot leak into the next.

Current differential cases all pass:

```text
DIFF-OK count-basic
DIFF-OK declarative-act
DIFF-OK declarative-long
DIFF-OK declarative-no-noise
DIFF-OK multi-model-talking
DIFF-OK probe-activation-combo
DIFF-OK probe-audio-stack
DIFF-OK probe-audio-trace
DIFF-OK probe-compile-utility
DIFF-OK probe-imaginal-indirect
DIFF-OK probe-retrieval-lifecycle
DIFF-OK probe-run-control
DIFF-OK probe-runtime-defs
DIFF-OK probe-motor-overlap
DIFF-OK probe-multi-model-params
DIFF-OK probe-vision-mutation
```

## Commits relevant to this work

- `fbfb58c style: format multi-model trace test`
- `7967f6c tests: add multi-model parameter differential probe`
- `4682da8 engine: match multi-model trace, seed, and reload semantics`
- `769e983 tests: add motor overlap differential probe`
- `7f9f48f engine: apply extended motor dual stage sgp`
- `e8697e4 engine: load wrapped extras and extended motor slots`
- `2308369 tests: add vision mutation differential probe`
- `525ce89 engine: match device slots in visual-location search`
- `b9616f1 docs: plan priority 2 parity probes`
- `6d8ee0f engine: keep audio end checks maintenance`
- `9013bba docs: record compile utility probe completion`
- `f10773e tests: add compile utility differential probe`
- `ed49e9c rpc: print f32 values as shortest single floats`
- `bf33fae docs: record imaginal indirect probe completion`
- `7ce4b8b tests: add imaginal indirect differential probe`
- `d6ef974 engine: apply same-fire imaginal mods before indirect requests`
- `3e405f6 docs: record activation combo probe completion`
- `dddb7f3 tests: add activation-combo differential probe`
- `02b6f7f engine: trace blending activation with lisp float formatting`
- `c0a4116 docs: record audio trace probe completion`
- `1dacda4 tests: add high-detail audio trace probe`
- `f26628d engine: mirror audio detection and end scheduling`
- `95fef3c docs: detail priority 1 handoff plan`
- `9d0df4a tests: add audio-stack differential probe`
- `28cf5ca engine: match audio, speech, and full-time boundaries`
- `2213f31 tests: add retrieval-lifecycle differential probe`
- `481a032 tests: add run-control differential probe`
- `d50a3cb engine: match run timing and event-limit reporting`
- `7e5aa22 tests: add isolated runtime-definition probe`
- `4ca31bf tests: add multi-model-talking differential case`
- `e3fd403 rpc: preserve multi-model state during callbacks`
- `d85282e engine: preserve speech sound names and self filtering`
- `ea13c0f tests: add declarative activation-order differential cases`
- `f4f8a1c engine: use dm-chunks order for retrieval candidates`
- `82f2b87 docs: add differential parity handoff`
- `1e0ad9d tests: add declarative retrieval differential case`
- `d678ea0 engine: match isa-only retrieval and imaginal conditions`
- `fdaa3f8 tests: add differential SBCL/Rust harness`
- `c065a46 rpc: relay speech output across models`

## Harness layout

```text
rust_actr7/fixtures/differential/
  capture.py       # start SBCL or Rust, wait until ready, run one client, stop
  run_cases.py     # run every cases/*/client.py under both and compare
  cases/
    count-basic/client.py
    declarative-act/client.py
    declarative-long/client.py
    declarative-no-noise/client.py
    multi-model-talking/client.py
    probe-activation-combo/client.py
    probe-audio-stack/client.py
    probe-audio-trace/client.py
    probe-compile-utility/client.py
    probe-imaginal-indirect/client.py
    probe-motor-overlap/client.py
    probe-multi-model-params/client.py
    probe-retrieval-lifecycle/client.py
    probe-run-control/client.py
    probe-runtime-defs/client.py
    probe-vision-mutation/client.py
  models/
    probe-activation-combo.lisp
    probe-audio-stack.lisp
    probe-audio-trace.lisp
    probe-compile-utility.lisp
    probe-imaginal-indirect.lisp
    probe-motor-overlap.lisp
    probe-multi-model-params.lisp
    probe-retrieval-lifecycle.lisp
    probe-run-control.lisp
    probe-runtime-defs.lisp
    probe-vision-mutation.lisp
```

Project-authored probe models live in `differential/models/`; the vendored
`fixtures/models/` tree remains a mirror of `actr7.x`.

Each case is a directory with a `client.py` that imports the vendored
`actr7.x/tutorial/python/actr.py`, loads an absolute model path, and runs.
`run_cases.py` compares normalized non-empty lines plus the client exit code.

`tests/differential.rs` is the ignored wrapper:

```powershell
cargo test --test differential -- --ignored --nocapture
```

## Resolved: declarative retrieval candidate order

ACT-R's `chunk-list` comes from the ordered `dm-chunks` alist, not the merge
hash table:

- `declarative-memory.lisp:1125` builds `chunk-list` from `dm-chunks`.
- `declarative-memory.lisp:1929-1931` groups by filled-slot signature and
  pushes new signature groups and chunks to the front.
- `declarative-memory.lisp:2749` flattens the same structure for
  `all-dm-chunks`.
- `dm-chunk-hash-table` is only used for merge/identity lookup.

Rust standard retrieval originally walked raw reverse insertion order. That
happens to match a single signature group but not the interleaved groups in
`declarative-issues`. `f4f8a1c` switched retrieval, recent-retrieval tracing,
and `dm` printing to `dm_alist_order()`.

Regression coverage:

- `retrieval_candidate_order_uses_dm_signature_groups`
- `retrieval_candidate_order_follows_extended_signature_groups`
- `declarative-act`
- `declarative-long`
- `declarative-no-noise`

This also fixes the prior noisy activation mismatch; no SBCL hash-table
emulation is needed.

## Resolved: multi-model-talking

The intended Python task now matches SBCL. Root causes were:

1. Runtime `define-chunks` rebuilt the current engine, re-arming a reset
   event mid-run and replaying the trace. `define-chunks` now updates the live
   chunk table without a rebuild.
2. Engine rebuilds reset the global multi-model trace buffer and position.
   `rebuild_engine_in_place` now preserves them.
3. The multi-model run processed all events before draining client callbacks.
   The capped RPC path now yields one event at a time so `output-speech` relay
   callbacks run during the run.
4. `mp-time` used the current model's local clock. It now uses the shared
   meta-process clock.
5. `reset` reset the selected model first, changing shared FIFO ordering.
   It now resets models in `model-order`.
6. Speech self-sounds did not consume a `WORD{n}` name. They now create the
   same sound/event chunks as SBCL.
7. Automatic self-speech filtering incorrectly emitted
   `find-sound-failure`. `stuff-sound-buffer` is a stuffed request, so it
   leaves the buffer untouched instead.
8. Speech-device installation was lost on rebuild. It is now tracked per
   model and reapplied.

Regression coverage:

- `multi-model-talking`
- `runtime_define_chunks_does_not_replay_multi_model_trace`
- `hear_self`

## Priority 0 probe corpus

Four project-authored probes now run under both servers and pass:

- `probe-runtime-defs`: a `model-output` monitor calls `add-dm` and
  `define-chunks` while the model is running, then retrieves the new chunk.
  This covers the runtime-definition path without re-arming reset trace.
- `probe-run-control`: exercises `run-n-events`, `run-until-time`,
  `run-full-time`, repeated `run` after a break, and `schedule-break-relative`.
- `probe-retrieval-lifecycle`: aborts a pending retrieval, uses
  `:recently-retrieved nil`, and checks the retrieval-failure path.
- `probe-audio-stack`: speaks twice, adds word/digit/tone sounds, and compares
  `print-audicon` snapshots at four times. The probe turns `:v` off because its
  target is the audicon/sound state, not same-time audio trace ordering.

Findings fixed by this pass:

- `run`/`run-n-events` return elapsed time (not the absolute clock), the
  processed event count, and the break flag; `get-time` returns milliseconds.
- A break with no details prints `BREAK-EVENT`, not
  `BREAK-EVENT Stopped by Environment`.
- `run-full-time` processes ordinary events at the limit, but leaves a
  `ProcDisplay` display wave exactly on the boundary for the next task run.
- Generic `print-audicon` no longer appends `ONLY-SOUND`/`ONLY-EVT`/`BOTH`
  columns; those columns remain for models that actually define audio
  features.
- A digit sound's offset matches Lisp's `sound-event` `:after` ordering quirk
  (offset stays at onset).
- With a speech device installed, the speech path emits
  `PREPARATION-COMPLETE`, `INITIATION-COMPLETE`, `FINISH-MOVEMENT`, and
  `MICROPHONE output-speech` at high detail.

The high-detail ordering lead was resolved by `probe-audio-trace` in
Priority 1: detection is now queued at sound creation, fills the real
`AURAL-LOCATION` buffer, checks the live buffer for the end precondition, and
skips end checks for sounds already finished at detection.

## Priority 1 — combined mechanisms

Work these in order. Each should get a project-authored model under
`rust_actr7/fixtures/differential/models/` and a case under
`rust_actr7/fixtures/differential/cases/`.

### 1. `probe-audio-trace` (do this first)

The known lead from the Priority 0 audio work. The goal is full high-detail
audio trace parity, not just audicon state.

- Model: two self-spoken words plus external word, digit, and tone sounds,
  with onsets chosen so an external sound and a self sound overlap around
  0.45–0.55 s. Set `:hear-self t`, `:trace-detail high`, `:v t`, and
  `:unstuff-aural-location .5`.
- Client: install `["speech", "microphone"]`, schedule the sounds, run in
  short slices, and leave the trace on.
- Expected observation to reproduce: Rust emits `AUDIO-EVENT-ENDED` and
  `MOD-BUFFER-CHUNK AURAL-LOCATION` where SBCL does not, orders `new-sound`
  and `SET-BUFFER-CHUNK AURAL-LOCATION` differently at 0.50 s, and may add an
  extra aural-location set at 0.55 s.
- Inspect `actr7.x/core-modules/audio.lisp:1045` (`find-sound` scheduling
  `audio-event-ended`), `:1073` (`aural-location-update-check`), and `:1077`
  (`audio-event-ended`). The Rust paths are in `events.rs`
  (`Event::AudioEventEnded`, `Event::StuffSound`, `Event::NewSound`).
- Do not “fix” this by hiding trace lines. Capture the exact first divergence
  with `--verbose`, then add a minimal engine test before changing ordering or
  preconditions.
- Acceptance: `DIFF-OK probe-audio-trace` under both servers; full suite
  green; any changed behavior has a default-suite regression.

**Status: complete (`1dacda4`, fix `f26628d`).** The reproduced diff had three
parts: an extra `AUDIO-EVENT-ENDED` at 0.45, `new-sound` before
`SET-BUFFER-CHUNK` at 0.50, and an extra aural-location set at 0.55. Root
causes fixed:

- `StuffSound`/speech detection was queued when the sound's onset event ran,
  not when the sound was created; same-time order now follows Lisp's
  `new-sound-event :around`.
- The speech path only emitted a set trace and never filled the real
  `AURAL-LOCATION` buffer, so later stuffing saw an empty buffer.
- The end precondition used cached `aural_loc_evt` instead of the live
  buffer, and the end check was scheduled for sounds already finished at
  detection.
- `unstuff-buffer` now erases inside the maintenance event so a same-time
  conflict resolution follows the erase.
- `audio-event-ended` is queued as maintenance (`6d8ee0f`), matching
  `audio.lisp:1045`; a no-op end check no longer wakes `M2`/`M3` conflict
  resolution in `multi-model-talking`.

Regression tests: `aural_detection_installs_location_and_skips_finished_end`,
`aural_detection_schedules_end_for_unfinished_sound`, plus existing
`hear-self`, `audio-updates`, `unstuff-aural`, and Sperling cases.

Candidate root causes checked:

- Rust schedules `AudioEventEnded` for self-speech sounds that already have
  `offset`/`duration`; Lisp's `find-sound` skips the end check for those.
- `aural_loc_evt` is a cached event id; Lisp's precondition reads the live
  `aural-location` buffer, so a stale cache can emit a mod after the buffer has
  moved to a later sound.
- Same-time priority/queue order differs between `new-sound`,
  `SET-BUFFER-CHUNK`, `ModBuffer`, and `AUDIO-EVENT-ENDED`.
- `:output high` is applied at schedule time in Lisp but event visibility is
  checked at emit time in Rust, changing whether the internal end event runs.

Useful commands:

```powershell
cargo build --bin actr
python rust_actr7/fixtures/differential/run_cases.py --server both --isolate --case probe-audio-trace --verbose
python rust_actr7/fixtures/differential/capture.py --server both --test rust_actr7/fixtures/differential/cases/probe-audio-trace/client.py --full-server-output
```

### 2. `probe-activation-combo`

One model combining base-level learning, spreading activation, partial
matching, activation noise, and blending. The current full-detail suite covers
these mostly in isolation; this probe targets rounding, candidate ordering, and
trace narration when they interact.

- Use explicit seeds and `:trace-detail high :act high :pct t`.
- Include at least two competing chunks with explicit similarities and a
  blended request, plus one noisy retrieval.
- Compare full traces and, if numeric drift appears, add a draw-sequence test
  in the style of `tests/draws.rs`.
- `:blt`, `:tmp`, `:min-bl`, and `:blend-all-slots` are **model-only** in Rust;
  put them in the model body, not in client `set_parameter_value` calls.
- Highest-risk areas: one noise draw per candidate in DM order; partial
  matching reversing activation order; blending drawing in reversed match
  order; `:min-bl` drawing before suppression; `lisp_f` vs `lisp_s_f32`
  formatting.
- Avoid `:cst t`; Rust and Lisp route it to different streams.
- Acceptance: `DIFF-OK probe-activation-combo`; any numerical fix backed by a
  focused unit test.

**Status: complete (`dddb7f3`, fix `02b6f7f`).** The probe combines a
partial-matching retrieval (base-level + spreading + similarities + noise) with
a blending request. Findings fixed:

- Blending candidate activations bypassed the standard activation narration;
  they now print base-level, spreading, partial-matching, noise, and final
  activation lines when `:act` is medium/high.
- Blending trace values used `~S`-style scientific formatting
  (`3.7122225e-6`); SBCL uses `~f` (`0.0000037122225`). All blending trace
  numbers now use `lisp_f`. `tests/cases/blending.ref` was corrected to the
  SBCL spelling.

The probe uses numeric goal states deliberately: Lisp auto-creates chunks for
symbol slot values, while Rust only treats names already in the chunk table as
sources. That separate widening is not exercised here.

### 3. `probe-imaginal-indirect`

Dynamic imaginal slots, indirect retrieval from `=imaginal>`, copy/merge into
imaginal, clear, and a failed indirect request.

- Keep the model small enough that the first divergence is attributable.
- Cover both `:recently-retrieved` and the `:ncnar` update path if practical.
- Inspect `engine/productions.rs` indirect-request resolution and
  `engine/events.rs` imaginal actions.
- Known likely gaps to target:
  - Rust concretizes `+retrieval> =imaginal` before the same firing's queued
    `=imaginal>` MOD; Lisp expands the indirect request after the MOD.
  - An indirect request from an empty/failed imaginal buffer errors in Rust
    instead of warning and requesting an empty spec.
  - `ImaginalError` does not set failure on both `imaginal` and
    `imaginal-action`.
  - `?imaginal> state error` is not implemented in production matching.
  - Whole-chunk copies into imaginal can keep the source name where Lisp
    copies to `CHUNK{n}` when `buffer-requires-copies` is in force.
- Use `buffer_read`, `buffer_chunk`, `sdm`, and `whynot` for deterministic
  output. Avoid `buffer-status`; Rust hardcodes its `buffer full` line.
- Acceptance: `DIFF-OK probe-imaginal-indirect`; default-suite regression for
  any engine change.

**Status: complete (`7ce4b8b`, fix `d6ef974`).** The probe covers dynamic
imaginal creation/modification, a same-fire `=imaginal>` MOD plus
`+retrieval> =imaginal`, copy into imaginal, and clear. It found the expected
ordering gap: Rust expanded the indirect retrieval before the MOD, so it
retrieved `R1` where SBCL correctly failed. Fixed by applying a queued same-fire
imaginal MOD to the live buffer before resolving a request, while leaving the
visible `MOD-BUFFER-CHUNK` event queued. Regression:
`indirect_request_sees_same_fire_imaginal_mod`.

The probe intentionally prints slot values rather than buffer/chunk names, so
the known `GOAL` reuse-name difference (`START` vs `GOAL-CHUNK0`) does not
obscure the targeted behavior.

### 4. `probe-compile-utility`

A tiny two-production chain that compiles on the second trial, with a reward
and utility update.

- Exercise dynamic slots in the composed production and at least one reward
  that changes selection.
- Compare compiled-production trace text, utility values, and action timing.
- Inspect `engine/compilation.rs` and `engine/productions.rs`.
- Base the model on `actr7.x/user-loads/compilation-units/goal-dynamic-roundtrip.lisp`;
  its expected compiled block already exists in
  `rust_actr7/fixtures/compilation-traces/goal-dynamic-roundtrip.trace`.
- Likely risk areas: generated `PRODUCTION{n}` collision handling, hardcoded
  new-production utility vs `:nu`, duplicate-utility updates gated by `:esc`,
  compile-time rendering vs later `production-details`, and reward timing.
- The differential harness compares client stdout, not server trace. Prefer
  `all_productions`, `production-details`, `spp` getters, and `get_time`; add a
  companion trace oracle only if exact server trace text is required.
- Acceptance: `DIFF-OK probe-compile-utility`; full suite green.

**Status: complete (`f10773e`, fix `ed49e9c`).** The probe clones
`goal-dynamic-roundtrip`, adds rewards/utility learning, and compares the
compiled `PRODUCTION0` block, `:u` values, and action timing before/after the
parent production is disabled. The compiled text and timing already matched.
The remaining diff was RPC float spelling: Lisp returns shortest single floats
(`0.19`, `0.59000003`, `0`), while Rust sent exact f64 expansions
(`0.1899999976158142`, `0.5900000333786011`, `0.0`). `Json::write_float` now
prints the shortest single-float form when the value is exactly an f32, and
`:u` zero is returned as integer `0`. Regression:
`f32_values_print_shortest_single_float`.

## Priority 2 — device and meta-process probes

These are more expensive. Start them only after the Priority 1 queue is green
or after an explicit reassessment.

### Priority 2 implementation plan (reopened)

Priority 1 is green as of `d8da1a3`; begin Priority 2 in the document order:
vision mutation, motor overlap, then multi-model parameters.

Planning findings:

- Existing tests already cover nearest visual locations (`nearest-vis-loc`) and
  basic remove/restore (`removed-and-restored-visual-item`). The new vision
  probe must combine window mutation with nearest selection and a two-slot
  visual search rather than duplicating those cases.
- Rust `install-device` only marks a speech device
  (`src/rpc/session.rs:3362`). Keyboard/mouse behavior must not depend on a
  Rust-side device install; the motor probe should use the standard manual
  module plus `extended-motor-actions`.
- Extended motor parameters (`:dual-processor-stages`,
  `:dual-execution-stages`, and related timing flags) are currently wired only
  through the scripted `ZzDriver`, not the normal SGP path
  (`src/engine/debug.rs`). The second-hand overlap probe will likely expose
  this.
- `run_multi_secs` forces `set_trace(true)` on every engine
  (`src/rpc/session.rs:1115-1129`), so per-model `:v nil` is a likely gap.
- Replacing a model removes it and appends the replacement
  (`add_model_definition`, `src/rpc/session.rs:652`), so model order during a
  single-model reload needs checking.
- The harness-layout list below is stale; update it when each probe lands.

Execution plan:

1. `probe-vision-mutation`: add a small goal-state model and client that
   opens a headless window, sets an explicit visual center point, adds colored
   text, requests `:nearest current` and a two-slot search, then removes,
   re-adds at a new position, and clears the window. Compare `printed-visicon`,
   selected location slots, `get_time`, and high-detail vision trace. Fix
   confirmed mutation/search defects with focused engine regressions.
2. `probe-motor-overlap`: require `extended-motor-actions`, enable dual
   processor/execution stages in the model body, and issue left/right hand
   keypresses plus a release while the other hand is pending. Compare
   high-detail motor/device trace, `output-key`/`release-key` monitor timing,
   and manual buffer state. Add SGP support for confirmed missing extended
   motor flags and regress it.
3. `probe-multi-model-params`: define two models with different `:seed`,
   `:ans`, `:rt`, and `:v`, print `mp-models` and per-model parameters, run
   interleaved, mutate a parameter from a client callback, then reload and
   check model order. Validate per-model trace suppression, independent RNG,
   global clock behavior, and no trace replay. Fix confirmed lifecycle/trace
   defects with session regressions.
4. After each probe: run targeted differential plus relevant default tests,
   commit probe and fix separately where practical, and update this document.
   Finish with the full suite, all differential cases under `--isolate`, and a
   final Priority 2 status/commit update.

### 1. `probe-vision-mutation`

Virtual-window add/remove/restore/clear, nearest-location selection, and a
two-feature search. This extends the existing `attend` and `unstuff-visual`
coverage into mutation ordering.

- Use `open_exp_window` / `add_text_to_exp_window` / `remove_items_from_exp_window`
  from the Python client, or a scripted device sidecar if the timing is easier.
- Keep the window small and seed the model explicitly.
- Acceptance: `DIFF-OK probe-vision-mutation`.

**Status: complete (`2308369`, fixes `525ce89`).** The probe requests
`nearest current` before mutation, after removing the alpha item, and after a
clear, then performs a two-slot `value "beta"`/`color black` search. Findings
fixed:

- Device-feature slots (`color`, `width`, `height`, `size`, `distance`,
  `text`/`line`/`oval`/`image`, and line endpoints) were not valid
  visual-location request slots; any such constraint errored
  (`not implemented: visual-location slot`). `loc_matches` now reads them from
  the feature construction spec, case-insensitively for string/symbol values.
- A `nearest current`/`current-x`/`current-y`/`current-distance` or
  clockwise/counterclockwise request with no prior attention now emits the
  same model warning as SBCL.

Regression: `visual_location_request_matches_device_feature_slots`; existing
`nearest-vis-loc`, `removed-and-restored-visual-item`, and vision tutorial
cases remain green.

Same-time clear-then-add is now covered: the probe re-adds `delta` immediately
after clearing, and after a one-event display run re-adds `zeta` at the same
clock. `proc_display` now clears `display_queued_ms` at the end of a wave while
keeping any wave still queued for the current time. Regression:
`same_time_window_ops_after_display_queue_a_new_wave`.

### 2. `probe-motor-overlap`

Overlapping manual movement and keypress actions: preparation/initiation/
execution overlap, a second hand request while the first executes, and a
keypress that releases while a hand movement is pending.

- Use `install_device(["keyboard", "mouse"])` or the relevant virtual device.
- Compare high-detail MOTOR timing and buffer state.
- Acceptance: `DIFF-OK probe-motor-overlap`.

**Status: complete (`769e983`, fixes `e8697e4`, `7f9f48f`).** The probe uses
`install-device '("motor" "keyboard")`, holds a left-hand key, types a
right-hand key while the left hand is executing, then releases the left key
while the right hand is executing. Findings fixed:

- Model files failed to load when `require-extra` was wrapped in
  `suppress-warnings`/`let` output redirection; top-level `require-extra`
  detection now scans those wrappers. The wrapped extra also suppresses SBCL
  compiler/redefinition chatter so the differential output stays comparable.
- Extended manual query slots (`execution`, `processor`, `last-command`, and
  the five fingers) were reported as previously undefined. They are now known
  when `extended-motor-actions` is required, and `required_extras` is attached
  before production validation.
- `:dual-processor-stages` / `:dual-execution-stages` were only applied by the
  scripted `ZzDriver`. They now load from the model body, work through
  `sgp`/`get-parameter-value`, and survive reset.

Regression: `required_extra_wrapped_in_let_enables_extended_motor_slots`,
`extended_motor_dual_stage_params_survive_reset`; existing
`zz_extended_motor_actions` and manual-keyboard cases remain green.

The probe now gates the overlap with real per-hand queries:
`?manual-left> execution busy`, `?manual-right> processor free/state free`,
`?manual-right> processor busy`, and `?manual-left> index down`. Rust now
implements per-hand `state`, `processor`, and `execution`, plus `last-command`
and finger `up` queries. Regression:
`extended_manual_hand_stage_queries_follow_per_hand_state`; the
`zz-extended-motor-actions` reference remains green.

### 3. `probe-multi-model-params`

Two models with different seeds/parameters, interleaved runs, one model
deleted/reloaded, and a reset in the middle.

- Extend `multi-model-talking` rather than duplicating it.
- Cover per-model `:seed`, `:ans`, `:rt`, and `:v`, plus trace column ordering.
- Include one client callback that mutates model order or parameters.
- Acceptance: `DIFF-OK probe-multi-model-params`; no trace replay or global
  clock drift.

**Status: complete (`7967f6c`, fix `4682da8`).** The probe loads one file
containing `MP-A` (`:v t`, seed 101, `.1` activation noise, `:rt -100`) and
`MP-B` (`:v nil`, seed 202, `.5` noise, `:rt 100`). It prints `mp-models`,
per-model parameters before/after the run, then reloads the file and checks
that time, seeds, and parameters reset. A `run-start` monitor mutates `MP-B`'s
`:rt`/`:ans` mid-run. Findings fixed:

- Multi-model runs forced `set_trace(true)` on every engine, so a model with
  `:v nil` leaked into the global trace. Runs now honor each model's own trace
  flag; `MultiEngine` also no longer forces trace on construction.
- `get-parameter-value` returned `:seed` as the string `"(S C)"` and `:v` as
  the strings `"T"/"NIL"`. It now returns a two-element array for `:seed` and
  JSON booleans/null for `:v`, matching the Lisp RPC types and the live RNG
  draw count.
- `reload` did not execute the model file's top-level `(clear-all)`, so model
  time and state survived. Reload now clears when the source has a top-level
  `(clear-all)`, matching SBCL.
- A second `load-act-r-model` of a different file intentionally keeps the old
  Rust "append models" behavior: existing tests rely on it, while the
  canonical multi-model file contains all models. `reload` handles the
  clear-all case separately.

Regression: `multi_model_honors_per_model_trace_and_parameter_types`,
`source_clear_all_is_detected_at_top_level_only`; existing multi-model session,
RPC, and `multi-model-talking` cases remain green.

Single-model deletion is covered through a code sidecar:
`probe-multi-model-extra.lisp` adds `MP-C` without a top-level `clear-all`,
`probe-multi-model-delete.lisp` runs `(delete-model mp-c)`, and the probe then
loads `MP-C` again while `MP-A`/`MP-B` remain untouched. Rust now recognizes
top-level `(delete-model name)` in code files. Regression:
`delete_model_forms_are_read_from_code_files`.

Coverage gap still open:

- The probe does not perform a `reset` in the middle of a multi-model run.

Residual note: with `MP-B` at `:v nil`, SBCL's `run` reports 33 processed
events and Rust reports 37, although both produce the same trace, parameter
state, and clock. The probe does not print the `run` event count; this count
semantics remains uninvestigated.

### Open Priority 2 gaps

Priority 2 remains open until these are reproduced and either fixed or
explicitly scoped out:

1. Multi-model: the `run` event count differs under per-model `:v nil`
   (`33` SBCL vs `37` Rust) even though trace, parameter state, and clock
   match. The probe currently omits the event count from its output.
2. Multi-model: add a mid-run `reset` scenario and verify trace replay, model
   order, parameters, seeds, and the global clock.

Execution decisions:

- "Mid-run reset" means between separate client `run` calls, because ACT-R
  rejects `reset` while the meta-process is running.
- Single-model deletion uses an existing Lisp `(delete-model name)` form
  loaded from a code sidecar. Rust will recognize that top-level form in the
  code-file loader; no new remote command is exposed.
- Event-count acceptance is exact parity of the `run` return tuple, including
  the processed-event count. The count will not be normalized or hidden.

**Priority 2 status: reopened.** The probes and fixes above are valid, but the
gaps listed here remain in scope. Current run is still green:
`cargo fmt --check` clean, `cargo test` green (`113` library/unit tests plus
`560` model cases, `8` ignored model cases), and all 16 differential cases
`DIFF-OK` under `--isolate`.

## Probe workflow

For each new probe:

1. Add `models/probe-<name>.lisp` and `cases/probe-<name>/client.py`.
2. Use `ROOT = Path(__file__).resolve().parents[5]` in the client and load an
   absolute model path; do not edit `actr7.x`.
3. Run with `--isolate`. Capture the first diff with `--verbose`; use
   `capture.py --full-server-output` when the client output is interleaved.
4. Minimize the model before fixing. Add a failing Rust unit/session test when
   the behavior is testable without SBCL.
5. Fix the engine, re-run the probe under both servers, then run
   `cargo fmt --check` and `cargo test`.
6. Commit the probe and fix separately where practical, and update the current
   case list and findings in this file.
7. If a mismatch is genuinely out of scope, record it here with the exact
   command and first-diff line rather than silently dropping the probe.

## Out of scope

Do not spend time on:

- `define-model-fct`
- custom module SGP parameters (`:create-delay`, `:demo-param`)
- `style-test-model` / `extend-manual-requests`
- SBCL warning line wrapping
- Lisp compiler/source warnings

These account for the remaining known load-only probe diffs.
