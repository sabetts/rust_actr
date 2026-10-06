# `zz` M5b handoff — `prepare` / `execute` (test5-8/5-9)

## Baseline

- Repo: `/data/sabetts/actr_transpile`
- Baseline commit: **`03497a6`** (`zz M5a: test5 run loop, direct styles, release/release-recoil — byte-exact through 2519`)
- Tree clean, `cargo test` green (42 enabled specs + unit tests).
- `zz-extended-motor-actions` first diff at **line 2520**; prefix **2519 byte-exact**
  (test1–test4 fully, test5 through test5-7 including both breakpoint dumps).
- `zz-extended-motor-actions` is **not** in `enabled.txt`; leave it out until tests 5–9 pass.

## Goal

Advance the byte-exact prefix past test5-8 (`prepare`) and test5-9 (`execute`), toward
test5-12. Then M5c = `point-hand-at-key` (test5-10), M5d = `hold-key k` (test5-11) +
`all-fingers-to-home hand right` (test5-12) + final dump/summary.

## Oracle (`actr7.x/test-models/zz-extended-motor-actions.ref`)

```
2.450   MODULE-REQUEST MANUAL
2.450   CLEAR-BUFFER MANUAL
2.450   MOTOR      PREPARE HAND-PLY HAND RIGHT R 1 THETA 3.14
2.450   CONFLICT-RESOLUTION
2.650   MOTOR      PREPARATION-COMPLETE 2.45 style HAND-PLY hand RIGHT
2.650   CONFLICT-RESOLUTION
2.650   PRODUCTION-SELECTED TEST5-9
(P TEST5-9
   =GOAL>
       TEST TEST5
       STATE 9
   ?MANUAL>
       STATE FREE
 ==>
   +MANUAL>
       CMD EXECUTE
   =GOAL>
       STATE 10
)
2.650   ------     BREAK-EVENT PRODUCTION TEST5-9
<buffer-status manual-left/manual-right + finger-loc dump>
2.650   BUFFER-READ-ACTION GOAL
...
2.700   MODULE-REQUEST MANUAL
2.700   CLEAR-BUFFER MANUAL
2.700   MOTOR      EXECUTE
2.700   CONFLICT-RESOLUTION
2.750   MOTOR      INITIATION-COMPLETE 2.45 style HAND-PLY hand RIGHT
2.750   CONFLICT-RESOLUTION
2.850   MOTOR      MOVE-A-HAND RIGHT 1 3.14
2.850   CONFLICT-RESOLUTION
2.900   MOTOR      FINISH-MOVEMENT 2.45 style HAND-PLY hand RIGHT
2.900   CONFLICT-RESOLUTION
```

Key timing: prep `2.450 → 2.650` = 0.200 (4 feats). `execute` at 2.700 re-bases exec:
INIT `2.750` (+0.050), MOVE-A-HAND `2.850` (+0.150), FINISH `2.900` (+0.200).
So exec/finish are offsets **from the execute event**, not from prep-complete.

## Source facts

- `prepare` (`general-pm.lisp:559-566`): `make-instance` the style from the spec,
  set `exec-immediate-p` nil, `prepare-movement`. Prep completes but the movement is
  **not** put in the exec queue.
- `execute` (`general-pm.lisp:577-592`): takes `last-prep`, appends it to the exec
  queue, `update-prepared`, `maybe-execute-movement`.
- Routing (`motor.lisp:1405-1428` `pm-module-request`): `cmd = prepare` reads the
  `style` slot plus `(hand finger r theta)` and schedules `'prepare` with
  `(style . params)`. Trace details = `prepare <style> HAND <h> R <r> THETA <theta>`.
- `execute` routing (`motor.lisp:1478`): `medium-output-command-request` →
  `handle-simple-command-request` → details `"EXECUTE"`.
- Spec in model (`test-model.lisp:656-683`):
  - test5-8: `isa prepare style hand-ply hand right r 1 theta 3.14`
  - test5-9: `isa execute`
- `hand-ply` (`motor.lisp:1096`, `ply` at `:1055`): feature-slots `(hand finger r theta)`
  with `finger = :dummy`, so `num-possible-feats` = 5 but `num-to-prepare` = 4
  (hand, r, theta). Hence prep 0.200 when the previous style-name differs
  (`PUNCH` vs `PLY`).
- `ply.compute-exec-time` (`motor.lisp:1084`) = `init + randomize(fitts(coeff, r, width))`;
  `default-target-width` initform = **1.0** (`motor.lisp:481`), so
  `fitts(0.1, 1, 1.0) = max(min-fitts 0.100, 0.1·log2(1.5)) = 0.100`
  → exec 0.150. `hand-ply` finish = `exec + burst` = 0.200. (Same shape as the
  existing `ZzHandPrep`/`ZzKind::Hand` path, but with an explicit polar vector
  instead of a mouse/home target.)

## Suggested implementation

The existing machinery is close: `ZzKind::Hand` already does a hand-ply with
`exec_ms`/`finish_ms` offsets, auto-release at `init + krt`, a `MOVE-A-HAND` emit,
and a hand-loc update — all scheduled relative to the exec-start time via
`zz_maybe_execute`. What's missing is the ability to **defer** the exec-start until
an `execute` request arrives.

1. Add `deferred: bool` to the `ZzKeyPrep`-style prep event (or reuse `ZzHandPrep`
   with a `deferred` flag), and a field `zz_prepared: Option<ZzPending>`.
   In the prep-complete handler: if `deferred`, store the `ZzPending` in
   `zz_prepared` and **do not** call `zz_maybe_execute`; else current behaviour.
2. Add `zz_prepare(slots)`:
   - read `style` (→ `HAND-PLY`), `hand`, `r`, `theta` from the spec;
   - emit `PREPARE <STYLE> HAND <h> R <r> THETA <theta>` at request time
     (`Out::Low`, module `MOTOR`). Use the **raw slot values** so `r 1` prints `1`
     not `1.0`;
   - run the normal prep (feats via `zz_last_move`/`zz_style_info`), but mark the
     pending deferred.
   - Target for the eventual move: `hand.loc + polar(r, theta)` (moves the hand,
     not a finger). Add a `ZzKind` arm or reuse `Hand` with an explicit target.
3. Add `zz_execute()`: emit `EXECUTE` (module `MOTOR`, `Out::Medium`), then take
   `zz_prepared`, push it to `man_exec_queue`, clear the slot, and call
   `zz_maybe_execute`. The pending's `exec_ms`/`finish_ms` are offsets, so
   `zz_maybe_execute` will re-base them to the current time — exactly what the
   oracle shows (2.700 → 2.850/2.900).
4. Route `PREPARE` / `EXECUTE` in `manual_request` (they are already in the
   cmd-detection list; currently they hit a no-op arm with a `TODO(M5b/M5c)`
   comment — replace that).
5. `zz_style_info` needs `"HAND-PLY" => ("PLY", 4, true)` (fresh = 4) so the
   prepare's prep is 0.200. (Currently `_ => ("PLY", 3, false)`.)

### Gotchas to respect

- `ZzPending` already carries `both`; keep it `false` for hand-ply.
- The `ZzKeyPrep` event now has a `loc: Option<(i64,i64)>` field used for the
  finger-offset update; a hand-ply has no finger offset, so pass `None`.
- `zz_last_cmd` seeds to `"NONE"` and the buffer-status prints it verbatim.
- The `pbreak` printer prints a `+buf>` request's slots **before** its `CMD <type>`.

## Verification

```
# prefix probe (workdir rust_actr7)
cargo test --test test_models probe_runnable -- --ignored --nocapture 2>&1 \
  | grep -E "first diff|common prefix" | head
# expect first diff to move from 2520 to the point-hand-at-key line (~2570+)

# full regression
cargo test --manifest-path rust_actr7/Cargo.toml 2>&1 | grep -E "test result|FAILED"
```

Commit per checkpoint; add a GOTCHA entry (source-tied, non-obvious only).

## Remaining after M5b

- **M5c** point-hand-at-key (test5-10): `r/theta = xy-to-polar(hand.loc, key_loc)`,
  then set `hand.loc = key_loc` and offsets to standard; style `HAND-PLY`, prep 1
  feat when the previous style-name is also `PLY` (only theta differs).
  Trace: `POINT-HAND-AT-KEY HAND RIGHT TO-KEY y`.
- **M5d** hold-key `k` (test5-11) + `all-fingers-to-home hand right` (test5-12) +
  final dump + summary.
- **M6** test6 `type-key` from non-home (~2872–3020).
- **M7** test7 delayed punch (~3021–3185).
- **M8** test8/9 all-fingers-to-home with multiple fingers down + release-all-fingers
  (~3186–3568).
