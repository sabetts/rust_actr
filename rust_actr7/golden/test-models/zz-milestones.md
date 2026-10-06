# zz-extended-motor-actions — milestone plan

Ref: `actr7.x/test-models/zz-extended-motor-actions.ref` (3568 lines).
Driver: `extras/extended-motor-actions/test-model.lisp` `run-test` runs nine
functions (`test1`…`test9`), each printing a `########################` header
before its runs and, after all of them, `summarize-tests` prints a `#####`
summary block per sub-run (`PRESSED`/`RELEASED`/`CLICKED`/`UNCLICKED`/`Held`/
`Clicked`). Module: `extras/extended-motor-actions/motor-extension.lisp`
(2737 lines) — a redefined motor module plus ~18 new styles/commands.

Reference line ranges: test1 1–1751, test2 1752–2000, test3 2001–2284,
test4 2285–2871, test5 2872–3020, test6 3021–3185, test7 3186–3373,
test8 3374–3548, test9 3549–3568.

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

## M4 — hold-peck + strike distance  (test4, 587 lines)
- `hit-key`/`hold-key`/`hold-peck`/`move-to-key`/`point-hand-at-key`,
  the two-phase `prepare`/`execute`, `:peck-strike-distance`,
  `movement-strike-time`, `pbreak`/`buffer-status`.

## M5 — finger / hand movement  (test5, 149 lines)
- `move-finger-to-home`, `all-fingers-to-home`, `hand-to-home`, `hand-to-mouse`.

## M6 — type-key from a non-home position  (test6, 165 lines)
- `type-key`, `move-to-key` (finger relocation before the peck).

## M7 — delayed punch  (test7, 188 lines)
- `delayed-punch`, `:default-punch-delay`/`:fast-punch-delay`/`:slow-punch-delay`.

## M8 — all-fingers-to-home + release-all  (tests 8/9, 195 lines)
- `all-fingers-to-home`, `release-all-fingers`, and the
  `Release-all-fingers action called when no fingers need to be released.`
  warning.

## M9 — polish
- Exact summary formatting, remaining warnings/edge cases, add to `enabled.txt`.

## Suggested order
M0 → M1 (largest, unlocks the shared dual-execution plumbing) → M2/M3 (device
release semantics) → M4 → M5/M6 → M7/M8 → M9.  Each milestone should end with a
commit and the 42-spec suite still green.
