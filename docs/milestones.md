# zz-extended-motor-actions — milestone plan

Ref: `actr7.x/test-models/zz-extended-motor-actions.ref` (3568 lines).
Driver: `extras/extended-motor-actions/test-model.lisp` `run-test` runs nine
functions (`test1`…`test9`), each printing a `########################` header
before its runs and, after all of them, `summarize-tests` prints a `#####`
summary block per sub-run (`PRESSED`/`RELEASED`/`CLICKED`/`UNCLICKED`/`Held`/
`Clicked`). Module: `extras/extended-motor-actions/motor-extension.lisp`
(2737 lines) — a redefined motor module plus ~18 new styles/commands.

## Status

**Complete.** `zz-extended-motor-actions` (3568 lines) is byte-exact and is in
`enabled.txt`; `cargo test` is green (43 enabled specs).

| test | lines |
|------|-------|
| test1–3 | 1–2017 |
| test4 | 2018–2310 |
| test5 | 2311–2893 |
| test6 | 2894–3034 |
| test7 | 3035–3226 |
| test8 | 3227–3393 |
| test9 | 3394–3568 |

## M0 — Loader + task-driver scaffolding  (mostly done)
- Loader: top-level `defmethod`; model-body `sgp-fct`/`p-fct` (backquoted,
  `,*var*` substitutions), `install-device`; quoted `:trace-detail`.
  **Done** (`40cc0cc`, `5f2660e`).
- Spec driver steps reproducing `run-test`/`testN`:
  per-test `(reset)` + parameter sets (`*dp*`/`*de*`/`*query*`/`*cdd*`/`*kct*`/
  `*krt*`/`*psd*`/`*dpd*`/`*fpd*`/`*spd*`), `(goal-focus …)`,
  `start-hand-at-mouse`, `pbreak`, `buffer-status`, `finger-loc-m` output, and
  the header line.
- Task summary output (`display-data`/`summarize-tests`): the `#####` blocks
  and the pressed/released/clicked tables.
- **Exit**: headers/summaries and reset/run skeleton line up (bodies still wrong).

## M1 — dual-processor / dual-execution motor module  (test1, 1751 lines)
The largest and foundational piece.
- `dual-execution-motor-module`: two processor stages + two execution stages,
  `compute-second-exec-time`, overrides of `prepare-movement`,
  `preparation-complete`, `initiation-complete`, `perform-movement`,
  `finish-movement-dual`, `change-state`, per-hand hand-tracker buffers.
- `keyboard-with-release` + `:key-release-time` (pressed/released events).
- `press-key` for same/diff hand, queries `state`/`processor`/`preparation`.
- **Exit**: test1 trace + its 24 summaries match (≈ half the ref).

## M2 — mouse: cursor-ply + drag delay  (test2, 249 lines)
- `mouse-with-release`, `cursor-ply`, `move-cursor`/`hold-mouse`/
  `release-mouse`/`click-mouse`, `:cursor-drag-delay`, `start-hand-at-mouse`.

## M3 — auto-release key & mouse on hand move  (test3, 284 lines)
- `release-key`/`release-key-to-home`/`release-mouse`, `:key-release-time`
  (release when the hand moves away).

## M4 — hold-peck + strike distance  (test4, 293 lines)  **Done**
- `hit-key`/`hold-key`/`hold-peck`/`move-to-key`, `point-finger`,
  `:peck-strike-distance`, `movement-strike-time`, `pbreak`/`buffer-status`.
- **Exit met**: test4 byte-exact through line 2310 (commits `bf309c4`,
  `d17dd7d`, `eaadeba`, `fdf2545`, `824b605`).

## M5 — finger / hand movement  (test5, 583 lines)  **Done**
- `move-finger-to-home`, `all-fingers-to-home`, `hand-to-home`,
  `hand-to-mouse` (M4c-1/M5a), then the two-phase `prepare`/`execute`,
  `point-hand-at-key`, and the `all-fingers-to-home hand <h>` restriction
  (M5b–M5d).
- **Exit met**: test5 byte-exact through line 2893 including its summary
  (commit `62fea19`; GOTCHA #140).

## M6 — type-key from a non-home position  (test6, 141 lines)  **Done**
- `type-key` (a `peck-recoil` that recoils to the finger's start), the
  deferred `output-key` scheduling, and `finish-movement-dual`'s finger
  freeing before the next movement.
- **Exit met**: test6 byte-exact through line 3034 including its summary
  (GOTCHA #141).

## M7 — delayed punch  (test7, 192 lines)  **Done**
- `delayed-punch` (a `punch` whose release is `delay` after the strike),
  `:default-punch-delay`/`:fast-punch-delay`/`:slow-punch-delay`.
- **Exit met**: test7 byte-exact through line 3226 including its summary
  (GOTCHA #142).

## M8 — all-fingers-to-home + release-all  (tests 8/9, 342 lines)  **Done**
- `all-fingers-to-home` with multiple fingers down (test8), `release-all-fingers`
  and its no-op warning (test9).
- **Exit met**: test9 byte-exact through line 3568 (GOTCHA #143).

## M9 — polish  **Done**
- Summary formatting, warnings/edge cases, and `enabled.txt` — the spec is
  enabled and the suite is green (43 specs).

## Suggested order
M0 → M1 (largest, unlocks the shared dual-execution plumbing) → M2/M3 (device
release semantics) → M4 → M5/M6 → M7/M8 → M9.  Each milestone should end with a
commit and the 43-spec suite still green.
